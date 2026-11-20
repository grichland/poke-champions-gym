//! Time per decision of a built-in Player on recorded Battles: plays `--games` games of it against max
//! damage (alternating seats), keeps the Battle of each of its turn and replace decisions, then has
//! `--threads` threads (each with its own Player) choose on all of them at once and reports microseconds
//! per decision: over every decision, and over those with more than one legal action (the searched ones).
//!
//! Usage: lookahead_speed --name lookahead --games 300 --seed 1 --threads 8 [--rounds 3]

use battle::{legal, Battle, ReqKind};
use env::batch::preview_action;
use env::builtin::by_name;
use env::teams::{TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind};
use players::Rng;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let v: Vec<String> = std::env::args().collect();
    let get = |k: &str| v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned();
    let name = get("--name").unwrap_or_else(|| "lookahead".into());
    let games: u64 = get("--games").and_then(|x| x.parse().ok()).unwrap_or(300);
    let seed: u64 = get("--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let threads: usize = get("--threads").and_then(|x| x.parse().ok()).unwrap_or(1).max(1);
    let rounds: usize = get("--rounds").and_then(|x| x.parse().ok()).unwrap_or(3).max(1);
    let teams = get("--teams").unwrap_or_else(|| "tools/rig/data/teams.json".into());
    if by_name(&name).is_none() {
        eprintln!("unknown player {}", name);
        std::process::exit(2);
    }
    let pool = Arc::new(TeamPool::open(&teams).expect("teams"));
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: false, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(pool, cfg);
    let mut ps = [by_name(&name).unwrap(), by_name("max_damage").unwrap()];
    let mut rec: Vec<(Battle, ReqKind)> = Vec::new();
    let mut legal_acts = Vec::new();
    while env.games_done < games {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let who = (seat + env.games_done as usize) % 2;
        let b = *env.battle(0, seat);
        let mut rng = Rng::new(seed ^ (env.decisions << 1) ^ who as u64);
        let act = match p.kind {
            RequestKind::Preview => preview_action(ps[who].teampreview(&b, &mut rng)).unwrap_or(0),
            k => {
                let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                if who == 0 {
                    rec.push((b, rk));
                }
                ps[who].choose_move(&b, rk, &mut rng) as u16
            }
        };
        env.legal_actions(0, &mut legal_acts);
        let act = if legal_acts.contains(&act) { act } else { legal_acts[0] };
        env.act(&[act]).expect("act");
    }
    let searched: Vec<bool> = rec.iter().map(|(b, k)| legal(b, *k).count() > 1).collect();
    let n_search = searched.iter().filter(|x| **x).count();
    let rec = Arc::new(rec);
    let searched = Arc::new(searched);
    let mut best: Option<(f64, f64, f64)> = None;
    for _ in 0..rounds {
        let t0 = Instant::now();
        let outs: Vec<(f64, f64, usize)> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads)
                .map(|t| {
                    let rec = rec.clone();
                    let searched = searched.clone();
                    let name = name.clone();
                    sc.spawn(move || {
                        let mut p = by_name(&name).unwrap();
                        let (mut all, mut srch) = (0.0f64, 0.0f64);
                        let mut sum = 0usize;
                        for (i, (b, k)) in rec.iter().enumerate() {
                            let mut rng = Rng::new(seed ^ (i as u64) << 8 ^ t as u64);
                            let s = Instant::now();
                            sum += p.choose_move(b, *k, &mut rng);
                            let d = s.elapsed().as_secs_f64();
                            all += d;
                            if searched[i] {
                                srch += d;
                            }
                        }
                        (all, srch, sum)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().expect("thread")).collect()
        });
        let wall = t0.elapsed().as_secs_f64();
        let n = rec.len().max(1) as f64;
        let all = outs.iter().map(|o| o.0).sum::<f64>() / threads as f64 / n * 1e6;
        let srch = outs.iter().map(|o| o.1).sum::<f64>() / threads as f64 / n_search.max(1) as f64 * 1e6;
        let r = (all, srch, wall);
        if best.map_or(true, |b| r.0 < b.0) {
            best = Some(r);
        }
        println!("{} threads {} decisions {} (searched {}): {:.1} us per decision, {:.1} us per searched decision, wall {:.2} s", name, threads, rec.len(), n_search, all, srch, wall);
    }
    let (all, srch, _) = best.unwrap();
    println!("best of {}: {} threads {}: {:.1} us per decision, {:.1} us per searched decision", rounds, name, threads, all, srch);
}
