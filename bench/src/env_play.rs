//! Speed of `env::BatchEnv` with uniformly random legal actions (strict turn-taking: one decision per
//! game per call), one thread and then one BatchEnv per thread on every core; then the cost of
//! enumerating every outcome of a step on states sampled from random games.
//!
//! Usage: env_play TEAMS_JSON [--games N] [--seed S] [--secs T] [--enum-secs E] [--out FILE]
//! Each phase runs for its time; one JSON line per phase goes to stdout and to FILE (appended).

use env::teams::{distinct, mix, TeamMode, TeamPool};
use env::{BatchEnv, EnumOptions, EnvConfig, Game};
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::State;
use cg_engine::sim::Kind;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

struct Run {
    decisions: u64,
    games: u64,
    calls: u64,
    wall: f64,
}

fn drive(pool: Arc<TeamPool>, games: usize, seed: u64, secs: f64) -> Run {
    let mut env = BatchEnv::new(pool, EnvConfig { games, seed, team_mode: team_mode(), max_steps: 1000, ..Default::default() });
    let mut rng = Seeded::new(mix(seed, 99));
    let mut acts = vec![0u16; games];
    let mut legal = Vec::with_capacity(64);
    let t0 = Instant::now();
    let mut calls = 0u64;
    while t0.elapsed().as_secs_f64() < secs {
        for g in 0..games {
            env.legal_actions(g, &mut legal);
            acts[g] = legal[rng.below(legal.len() as u32) as usize];
        }
        env.act(&acts).expect("act");
        calls += 1;
    }
    Run { decisions: env.decisions, games: env.games_done, calls, wall: t0.elapsed().as_secs_f64() }
}

fn speed_line(phase: &str, threads: usize, games: usize, r: &Run) -> String {
    format!(
        "{{\"bench\": \"env_play\", \"phase\": \"{}\", \"pool\": \"stage3\", \"threads\": {}, \"games_per_env\": {}, \"wall_s\": {:.3}, \
         \"act_calls\": {}, \"decisions\": {}, \"battles\": {}, \"decisions_per_s\": {:.1}, \"battles_per_s\": {:.1}, \"decisions_per_battle\": {:.2}}}",
        phase,
        threads,
        games,
        r.wall,
        r.calls,
        r.decisions,
        r.games,
        r.decisions as f64 / r.wall,
        r.games as f64 / r.wall,
        r.decisions as f64 / r.games.max(1) as f64
    )
}

/// A stage-3 game some random steps in, with random choices for its next step.
fn sample(pool: &TeamPool, seed: u64) -> Option<(State, [Option<cg_engine::sim::Choice>; 2])> {
    let mut rng = Seeded::new(seed);
    let (_, teams) = pool.draw(&mut rng, team_mode());
    let p0 = distinct(&mut rng, 6, 3);
    let p1 = distinct(&mut rng, 6, 3);
    let picks = [[p0[0] as u8, p0[1] as u8, p0[2] as u8], [p1[0] as u8, p1[1] as u8, p1[2] as u8]];
    let mut g = Game::start([&teams[0], &teams[1]], picks, mix(seed, 1)).ok()?;
    let choose = |g: &Game, rng: &mut Seeded| {
        let o = g.state.options();
        let mut ch = [None; 2];
        for s in 0..2 {
            if matches!(o[s].kind, Kind::Turn | Kind::Replace) {
                let cs = o[s].choices();
                ch[s] = Some(cs[rng.below(cs.len() as u32) as usize]);
            }
        }
        ch
    };
    for _ in 0..rng.below(40) {
        if g.state.ended {
            return None;
        }
        let ch = choose(&g, &mut rng);
        g.step(ch).ok()?;
    }
    if g.state.ended {
        return None;
    }
    let ch = choose(&g, &mut rng);
    Some((g.state, ch))
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn enum_line(pool: &TeamPool, seed: u64, secs: f64, cap: usize) -> String {
    let opts_raw = EnumOptions { merge: false, max_branches: cap, keep_events: false };
    let opts_m = EnumOptions { merge: true, max_branches: cap, keep_events: false };
    let (mut leaves, mut merged, mut t_raw, mut t_m, mut depth) = (vec![], vec![], vec![], vec![], vec![]);
    let mut capped = 0;
    let t0 = Instant::now();
    let mut k = 0u64;
    while t0.elapsed().as_secs_f64() < secs {
        k += 1;
        let Some((s, ch)) = sample(pool, mix(seed, k)) else { continue };
        let a = Instant::now();
        let r = env::enumerate_step(&s, ch, &opts_raw).expect("enumerate");
        t_raw.push(a.elapsed().as_secs_f64() * 1e6);
        let a = Instant::now();
        let m = env::enumerate_step(&s, ch, &opts_m).expect("enumerate");
        t_m.push(a.elapsed().as_secs_f64() * 1e6);
        capped += r.capped as usize;
        leaves.push(r.leaves as f64);
        merged.push(m.outcomes.len() as f64);
        depth.push(r.max_depth as f64);
    }
    let n = leaves.len();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let (ml, mm, mtr, mtm) = (mean(&leaves), mean(&merged), mean(&t_raw), mean(&t_m));
    format!(
        "{{\"bench\": \"env_enumerate\", \"pool\": \"stage3\", \"states\": {}, \"cap\": {}, \"capped\": {}, \
         \"leaves_mean\": {:.1}, \"leaves_p50\": {}, \"leaves_p90\": {}, \"leaves_p99\": {}, \"leaves_max\": {}, \
         \"merged_mean\": {:.1}, \"merged_p50\": {}, \"merged_p90\": {}, \"merged_p99\": {}, \"merged_max\": {}, \
         \"events_depth_max\": {}, \"us_raw_mean\": {:.1}, \"us_raw_p50\": {:.1}, \"us_raw_p99\": {:.1}, \"us_raw_max\": {:.1}, \
         \"us_merged_mean\": {:.1}, \"us_merged_p50\": {:.1}, \"us_merged_max\": {:.1}, \"us_per_leaf\": {:.3}}}",
        n,
        cap,
        capped,
        ml,
        pct(&mut leaves, 0.5),
        pct(&mut leaves, 0.9),
        pct(&mut leaves, 0.99),
        pct(&mut leaves, 1.0),
        mm,
        pct(&mut merged, 0.5),
        pct(&mut merged, 0.9),
        pct(&mut merged, 0.99),
        pct(&mut merged, 1.0),
        pct(&mut depth, 1.0),
        mtr,
        pct(&mut t_raw, 0.5),
        pct(&mut t_raw, 0.99),
        pct(&mut t_raw, 1.0),
        mtm,
        pct(&mut t_m, 0.5),
        pct(&mut t_m, 1.0),
        mtr / ml.max(1e-9)
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: env_play TEAMS_JSON [--games N] [--seed S] [--secs T] [--enum-secs E] [--out FILE]");
        std::process::exit(2);
    }
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let games: usize = flag("--games").map_or(256, |s| s.parse().expect("--games"));
    let seed: u64 = flag("--seed").map_or(1, |s| s.parse().expect("--seed"));
    let secs: f64 = flag("--secs").map_or(20.0, |s| s.parse().expect("--secs"));
    let enum_secs: f64 = flag("--enum-secs").map_or(10.0, |s| s.parse().expect("--enum-secs"));
    let out = flag("--out");
    let pool = Arc::new(TeamPool::open(&args[1]).expect("teams"));
    let mut lines = Vec::new();
    let mut emit = |l: String| {
        println!("{}", l);
        lines.push(l);
    };

    let r = drive(pool.clone(), games, seed, secs);
    emit(speed_line("single_thread", 1, games, &r));

    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let t0 = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let p = pool.clone();
            std::thread::spawn(move || drive(p, games, mix(seed, t as u64 + 1), secs))
        })
        .collect();
    let runs: Vec<Run> = handles.into_iter().map(|h| h.join().expect("thread")).collect();
    let all = Run {
        decisions: runs.iter().map(|r| r.decisions).sum(),
        games: runs.iter().map(|r| r.games).sum(),
        calls: runs.iter().map(|r| r.calls).sum(),
        wall: t0.elapsed().as_secs_f64(),
    };
    emit(speed_line("all_cores", threads, games, &all));

    if enum_secs > 0.0 {
        emit(enum_line(&pool, seed, enum_secs, 100_000));
    }

    if let Some(path) = out {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("open --out");
        for l in &lines {
            writeln!(f, "{}", l).expect("write");
        }
    }
}

/// The real teams as written; `--stage3` for No Ability, no item and stage-3 moves.
fn team_mode() -> TeamMode {
    if std::env::args().any(|a| a == "--stage3") {
        TeamMode::Stage3
    } else {
        TeamMode::AsGiven
    }
}
