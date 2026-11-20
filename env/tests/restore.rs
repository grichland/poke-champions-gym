//! `BatchEnv::restore`: a saved game put back into a slot plays as any other game of the environment. With
//! every check on, restored games pass the checks (Battle against State at every request, labels, the
//! rebuild from the records); the same save and seed give the same game in any slot for any thread count;
//! the slots around a restored one are untouched; what restore refuses.

mod common;

use env::batch::Slot;
use env::teams::{mix, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind, SlotSave, TeamMode};
use cg_engine::sim::outcome::Seeded;
use std::sync::Arc;

fn pool_file(name: &str) -> TeamPool {
    TeamPool::from_file(&format!("{}/../bench/data/pools/{}.json", env!("CARGO_MANIFEST_DIR"), name)).expect("pool")
}

fn cfg(games: usize, seed: u64, checks: bool, rbr: bool) -> EnvConfig {
    EnvConfig { games, seed, team_mode: TeamMode::AsGiven, max_steps: 400, checks, keep_records: checks, auto_locked: true, replace_before_rows: rbr }
}

/// A random legal action per game, each game drawing from its own source (`call`, game).
fn random_acts(env: &BatchEnv, call: u64, out: &mut Vec<u16>) {
    let mut legal = Vec::new();
    out.clear();
    for g in 0..env.len() {
        env.legal_actions(g, &mut legal);
        let mut r = Seeded::new(mix(call, g as u64));
        out.push(legal[r.below(legal.len() as u32) as usize]);
    }
}

/// Everything a slot shows: State, both Battles, pending decision, its ids and options.
fn slot_text(s: &Slot) -> String {
    format!("{:?} {:?} {:?} {:?} {} {} {:?} {:?}", s.game.state, s.seats.builders[0].battle, s.seats.builders[1].battle, s.pending, s.decision, s.steps, s.team_ids, s.opts)
}

/// Saves of `games` games played `calls` acts with random actions, taken at random decisions.
fn saves(pool: &Arc<TeamPool>, seed: u64, rbr: bool, checks: bool, want: usize) -> Vec<SlotSave> {
    let mut src = BatchEnv::new(pool.clone(), cfg(64, seed, checks, rbr));
    let mut rng = Seeded::new(seed ^ 0x55);
    let mut acts = Vec::new();
    let mut out = Vec::new();
    let mut call = seed << 20;
    while out.len() < want {
        call += 1;
        random_acts(&src, call, &mut acts);
        src.act(&acts).unwrap();
        for g in 0..src.len() {
            if out.len() < want && rng.below(16) == 0 {
                if let Some(s) = src.save(g) {
                    out.push(s);
                }
            }
        }
    }
    out
}

#[test]
fn restored_games_pass_every_check() {
    for (k, (pool, rbr)) in [(common::pool(), true), (common::pool(), false), (pool_file("hidden_trap"), true), (pool_file("traps"), true), (pool_file("imprison"), false)].into_iter().enumerate() {
        let pool = Arc::new(pool);
        let sv = saves(&pool, 31 + k as u64, rbr, true, 120);
        let mut env = BatchEnv::new(pool.clone(), cfg(48, 900 + k as u64, true, rbr));
        env.set_threads(4);
        let mut rng = Seeded::new(17 + k as u64);
        let mut acts = Vec::new();
        let (mut restored, mut ended) = (0u64, 0u64);
        let mut live = vec![false; env.len()];
        let mut kinds = [0u64; 2];
        // fresh slots take saves too
        for g in 0..8 {
            env.restore(g, &sv[g], 1000 + g as u64).unwrap();
            live[g] = true;
            restored += 1;
        }
        let mut call = 7u64 << 32;
        while ended < 150 {
            call += 1;
            random_acts(&env, call, &mut acts);
            env.act(&acts).unwrap();
            for g in 0..env.len() {
                if !env.results()[g].done {
                    continue;
                }
                if live[g] {
                    ended += 1;
                    live[g] = false;
                }
                if rng.below(2) == 0 {
                    let s = &sv[rng.below(sv.len() as u32) as usize];
                    kinds[(s.pending().kind == RequestKind::Replace) as usize] += 1;
                    env.restore(g, s, rng.next_u64()).unwrap();
                    assert_ne!(env.pending(g).kind, RequestKind::Preview);
                    live[g] = true;
                    restored += 1;
                }
            }
        }
        let c = &env.counters;
        eprintln!("pool {}: restored {} (turn, replace saves {:?}), ended {}, checked {}, stale replace {} checked {}, refusals {}", k, restored, kinds, ended, c.checked, c.stale_replace, c.stale_checked, c.refusals);
        assert!(restored >= 150 && kinds[0] > 0 && kinds[1] > 0, "{} {:?}", restored, kinds);
        assert_eq!(c.agree.unexplained(), 0, "{:?}", c.agree.examples);
        assert_eq!(c.rebuild_equal, c.rebuilt);
        assert_eq!(c.anomalies.unexplained(), 0, "{:?}", c.anomalies.examples);
        assert_eq!(c.battle_forbids, 0, "{:?}", c.legal.examples);
        assert_eq!(c.refused.unexplained(), 0, "{:?}", c.refused.examples);
        assert_eq!(c.truth.total(), 0, "{:?}", c.truth.examples);
        assert_eq!(c.prior0.unexplained(), 0, "{:?}", c.prior0.examples);
        assert_eq!(c.hidden.total(), 0, "{:?}", c.hidden.examples);
        assert_eq!(c.errors, 0);
        assert!(c.labels_checked > 1000);
    }
}

/// Restores save k into slots 2k and 2k+1 with the same seed and plays both with the same actions: the
/// pair stays identical until its game ends. Returns the trace of every slot.
fn pairs(pool: &Arc<TeamPool>, sv: &[SlotSave], threads: usize, restore: bool) -> Vec<Vec<String>> {
    let mut env = BatchEnv::new(pool.clone(), cfg(64, 4242, false, true));
    env.set_threads(threads);
    if restore {
        for k in 0..32 {
            env.restore(2 * k, &sv[k], 70 + k as u64).unwrap();
            env.restore(2 * k + 1, &sv[k], 70 + k as u64).unwrap();
        }
    }
    let mut acts = Vec::new();
    let mut live = vec![restore; 32];
    let mut trace = vec![Vec::new(); 64];
    for call in 0..300 {
        random_acts(&env, call, &mut acts);
        for k in 0..32 {
            if live[k] {
                assert_eq!(slot_text(env.slot(2 * k)), slot_text(env.slot(2 * k + 1)), "pair {}", k);
                acts[2 * k + 1] = acts[2 * k];
            }
        }
        env.act(&acts).unwrap();
        for g in 0..64 {
            trace[g].push(format!("{} {:?} {:?}", slot_text(env.slot(g)), env.results()[g], env.labels()[g]));
        }
        for k in 0..32 {
            if live[k] {
                let (a, b) = (env.results()[2 * k], env.results()[2 * k + 1]);
                assert_eq!(a, b, "pair {}", k);
                assert_eq!(format!("{:?}", env.labels()[2 * k]), format!("{:?}", env.labels()[2 * k + 1]));
                if a.done {
                    live[k] = false;
                }
            }
        }
    }
    assert!(!restore || live.iter().all(|x| !x), "restored games still running");
    trace
}

#[test]
fn same_save_and_seed_give_the_same_game_in_any_slot_and_thread_count() {
    let pool = Arc::new(common::pool());
    let sv = saves(&pool, 5, true, false, 32);
    let one = pairs(&pool, &sv, 1, true);
    let eight = pairs(&pool, &sv, 8, true);
    assert!(one == eight, "8 threads differ from one");
}

#[test]
fn restoring_leaves_the_other_slots_alone() {
    let pool = Arc::new(common::pool());
    let sv = saves(&pool, 6, true, false, 4);
    let run = |restore_into: &[usize]| {
        let mut env = BatchEnv::new(pool.clone(), cfg(40, 99, false, true));
        env.set_threads(8);
        let mut acts = Vec::new();
        let mut trace = vec![Vec::new(); 40];
        let mut left = 6;
        for call in 0..400 {
            random_acts(&env, call, &mut acts);
            env.act(&acts).unwrap();
            for &g in restore_into {
                if env.results()[g].done && left > 0 {
                    env.restore(g, &sv[left % sv.len()], left as u64).unwrap();
                    left -= 1;
                }
            }
            for g in 0..40 {
                trace[g].push(format!("{} {:?} {:?}", slot_text(env.slot(g)), env.results()[g], env.labels()[g]));
            }
        }
        (trace, env.games_done)
    };
    let (plain, _) = run(&[]);
    let (with, _) = run(&[3, 17, 33]);
    for g in 0..40 {
        if [3, 17, 33].contains(&g) {
            assert!(plain[g] != with[g]);
        } else {
            assert!(plain[g] == with[g], "slot {} changed by restores elsewhere", g);
        }
    }
}

#[test]
fn restore_refuses_what_it_cannot_do() {
    let pool = Arc::new(common::pool());
    let sv = saves(&pool, 9, true, false, 2);
    let mut env = BatchEnv::new(pool.clone(), cfg(4, 1, false, true));
    assert!(env.restore(9, &sv[0], 1).is_err());
    // a running slot
    env.restore(0, &sv[0], 1).unwrap();
    assert!(env.restore(0, &sv[1], 1).unwrap_err().contains("being played"));
    // preview with seat 0's picks given
    let mut acts = Vec::new();
    random_acts(&env, 1, &mut acts);
    env.act(&acts).unwrap();
    assert!(env.slot(1).in_preview && env.slot(1).picks[0].is_some());
    assert!(env.restore(1, &sv[1], 1).unwrap_err().contains("being played"));
    // other settings, other teams
    for c in [cfg(4, 1, false, false), cfg(4, 1, true, true), EnvConfig { team_mode: TeamMode::Stage3, ..cfg(4, 1, false, true) }] {
        let mut other = BatchEnv::new(pool.clone(), c);
        assert!(other.restore(0, &sv[0], 1).unwrap_err().contains("other settings"));
    }
    let mut other = BatchEnv::new(Arc::new(pool_file("traps")), cfg(4, 1, false, true));
    assert!(other.restore(0, &sv[0], 1).unwrap_err().contains("other teams"));
    // the same teams read again: accepted
    let mut same = BatchEnv::new(Arc::new(common::pool()), cfg(4, 1, false, true));
    same.restore(0, &sv[0], 1).unwrap();
}
