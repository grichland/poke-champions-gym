//! Decisions per second for each Player on a fixed set of random Battles (seeded), capped at 60 s per
//! Player. Prints one JSON line per Player; `--out FILE` appends them to FILE.

use battle::{legal, ReqKind};
use players::gen::random_battle;
use players::{by_name, Rng, NAMES};
use std::io::Write;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let n: usize = arg("--battles").and_then(|x| x.parse().ok()).unwrap_or(2000);
    let rounds: usize = arg("--rounds").and_then(|x| x.parse().ok()).unwrap_or(200);
    let seed: u64 = arg("--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let cap_s: f64 = arg("--cap").and_then(|x| x.parse().ok()).unwrap_or(60.0);
    let out = arg("--out");
    let mut rng = Rng::new(seed);
    let t0 = Instant::now();
    let set: Vec<_> = (0..n).map(|_| random_battle(&mut rng, false)).collect();
    let previews: Vec<_> = (0..n / 10).map(|_| random_battle(&mut rng, true).0).collect();
    let build_s = t0.elapsed().as_secs_f64();
    let turn_share = set.iter().filter(|(_, k)| *k == ReqKind::Turn).count();
    let mut lines = vec![];
    let only = arg("--players");
    for name in NAMES.iter().copied().filter(|n| only.as_ref().map_or(true, |o| o.split(',').any(|x| x == *n))) {
        let mut p = by_name(name).unwrap();
        let mut prng = Rng::new(seed ^ 0xABCD);
        // warm-up and legality check
        let mut illegal = 0usize;
        for (b, k) in &set {
            let a = p.choose_move(b, *k, &mut prng);
            let l = legal(b, *k);
            if l.count() > 0 && !l.mask[a] {
                illegal += 1;
            }
        }
        let t = Instant::now();
        let mut decisions = 0u64;
        let mut sink = 0usize;
        let mut capped = false;
        for _ in 0..rounds {
            for (b, k) in &set {
                sink = sink.wrapping_add(p.choose_move(b, *k, &mut prng));
            }
            decisions += set.len() as u64;
            if t.elapsed().as_secs_f64() > cap_s {
                capped = true;
                break;
            }
        }
        let dt = t.elapsed().as_secs_f64();
        let tp = Instant::now();
        let mut previews_done = 0u64;
        for _ in 0..rounds.min(20) {
            for b in &previews {
                let picks = p.teampreview(b, &mut prng);
                sink = sink.wrapping_add(picks[0] as usize);
            }
            previews_done += previews.len() as u64;
        }
        let dtp = tp.elapsed().as_secs_f64();
        let line = format!(
            "{{\"player\": \"{}\", \"battles\": {}, \"turn_requests\": {}, \"replace_requests\": {}, \"seed\": {}, \"decisions\": {}, \"wall_s\": {:.3}, \"decisions_per_s\": {:.1}, \"us_per_decision\": {:.3}, \"previews\": {}, \"previews_per_s\": {:.1}, \"illegal\": {}, \"build_s\": {:.3}, \"time_capped\": {}, \"sink\": {}}}",
            name,
            n,
            turn_share,
            n - turn_share,
            seed,
            decisions,
            dt,
            decisions as f64 / dt,
            dt * 1e6 / decisions as f64,
            previews_done,
            previews_done as f64 / dtp,
            illegal,
            build_s,
            capped,
            sink % 1000
        );
        println!("{}", line);
        lines.push(line);
    }
    if let Some(path) = out {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("open out");
        for l in &lines {
            writeln!(f, "{}", l).unwrap();
        }
    }
}
