//! Observations per second on a fixed set of random Battles (seeded), single thread, capped at 60 s per
//! mode. Modes: full, without the damage features, and the parts alone. Prints one JSON line per mode;
//! `--out FILE` appends them to FILE.

use battle::{legal, ReqKind};
use obs::{observe_into, ObsBuffers};
use players::gen::random_battle;
use players::Rng;
use std::io::Write;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let n: usize = arg("--battles").and_then(|x| x.parse().ok()).unwrap_or(5000);
    let rounds: usize = arg("--rounds").and_then(|x| x.parse().ok()).unwrap_or(200);
    let seed: u64 = arg("--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let cap_s: f64 = arg("--cap").and_then(|x| x.parse().ok()).unwrap_or(60.0);
    let out = arg("--out");
    let mut rng = Rng::new(seed);
    let set: Vec<_> = (0..n).map(|_| random_battle(&mut rng, false)).collect();
    let mut buf = Box::new(ObsBuffers::default());
    let mut lines = vec![];
    let modes: [(&str, u8); 4] = [("full", 0), ("no_damage", 1), ("legal_only", 2), ("fill_only", 3)];
    for (name, mode) in modes {
        let mut sink = 0f64;
        let t = Instant::now();
        let mut calls = 0u64;
        let mut capped = false;
        for _ in 0..rounds {
            for (b, k) in &set {
                match mode {
                    0 | 1 => {
                        observe_into(b, *k, buf.out(), mode == 0);
                        sink += buf.rows[0][34] as f64 + buf.field[0] as f64;
                    }
                    2 => sink += legal(b, *k).count() as f64,
                    _ => {
                        let o = buf.out();
                        o.rows.fill(0.0);
                        o.row_ids.fill(0);
                        sink += o.rows[7] as f64 + (*k == ReqKind::Turn) as u8 as f64;
                    }
                }
            }
            calls += set.len() as u64;
            if t.elapsed().as_secs_f64() > cap_s {
                capped = true;
                break;
            }
        }
        let dt = t.elapsed().as_secs_f64();
        let line = format!(
            "{{\"bench\":\"obs\",\"mode\":\"{}\",\"battles\":{},\"calls\":{},\"seconds\":{:.3},\"calls_per_s\":{:.0},\"us_per_call\":{:.3},\"capped\":{},\"seed\":{},\"sink\":{:.1}}}",
            name,
            set.len(),
            calls,
            dt,
            calls as f64 / dt,
            dt * 1e6 / calls as f64,
            capped,
            seed,
            sink
        );
        println!("{}", line);
        lines.push(line);
    }
    if let Some(p) = out {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).expect("open --out");
        for l in lines {
            writeln!(f, "{}", l).unwrap();
        }
    }
}
