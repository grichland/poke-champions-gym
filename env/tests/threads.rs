//! Stepping on the worker pool gives exactly what one thread gives for the same seed: every game's State,
//! both Battles, pending decision, result, labels and observation, and the counters, whatever the thread
//! count, with and without the checks.

mod common;

use env::observe::{ObsBatch, ROWS_LEN, ROW_IDS_LEN};
use env::{BatchEnv, Counters, EnvConfig, TeamMode, N_ALL_ACTIONS};
use obs::{FIELD, LAST, LAST_IDS, MASK, OPP_ACTIONS};
use cg_engine::sim::outcome::Seeded;
use std::sync::Arc;

#[derive(Clone, PartialEq, Debug)]
struct Obs {
    rows: Vec<f32>,
    row_ids: Vec<i32>,
    field: Vec<f32>,
    last: Vec<f32>,
    last_ids: Vec<i32>,
    mask: Vec<u8>,
    actions: Vec<u8>,
    seat: Vec<u8>,
    kind: Vec<u8>,
    decision: Vec<i64>,
    prior: Vec<f32>,
}

/// The observation of every game through `observe_batch` (the pool when the env has one).
fn observe_all(env: &BatchEnv) -> Obs {
    let n = env.len();
    let mut o = Obs {
        rows: vec![0.0; n * ROWS_LEN],
        row_ids: vec![0; n * ROW_IDS_LEN],
        field: vec![0.0; n * FIELD],
        last: vec![0.0; n * LAST],
        last_ids: vec![0; n * LAST_IDS],
        mask: vec![0; n * MASK],
        actions: vec![0; n * N_ALL_ACTIONS],
        seat: vec![0; n],
        kind: vec![0; n],
        decision: vec![0; n],
        prior: vec![0.0; n * OPP_ACTIONS],
    };
    env.observe_batch_with(
        ObsBatch {
            rows: &mut o.rows,
            row_ids: &mut o.row_ids,
            field: &mut o.field,
            last: &mut o.last,
            last_ids: &mut o.last_ids,
            mask: &mut o.mask,
            actions: &mut o.actions,
            seat: &mut o.seat,
            kind: &mut o.kind,
            decision: &mut o.decision,
            opp_prior: &mut o.prior,
        },
        env::ObsOptions { threads: env.threads(), damage: true },
    )
    .unwrap();
    o
}

fn counts(c: &Counters) -> String {
    format!(
        "{} {} {} {} {} {} {} {:?} {:?} {:?} {:?} {} {} {} {} {} {} {} {:?} {:?} {} {} {:?}",
        c.asked,
        c.auto,
        c.refusals,
        c.battle_empty,
        c.refusal_fallbacks,
        c.checked,
        c.legal_equal,
        c.agree.counts,
        c.legal.counts,
        c.refused.counts,
        c.anomalies.counts,
        c.rebuilt,
        c.rebuild_equal,
        c.steps,
        c.rows,
        c.refused_trapped,
        c.errors,
        c.labels_checked,
        c.prior0.counts,
        c.truth.counts,
        c.hidden_games,
        c.hidden_steps,
        c.hidden.counts
    )
}

/// Plays `calls` act calls of random legal actions on `threads` workers; returns everything seen.
fn play(threads: usize, checks: bool, calls: usize) -> (Vec<String>, BatchEnv) {
    let pool = Arc::new(common::pool());
    let cfg = EnvConfig { games: 300, seed: 77, team_mode: TeamMode::AsGiven, max_steps: 1000, checks, keep_records: checks, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(pool, cfg);
    env.set_threads(threads);
    assert_eq!(env.threads(), threads);
    let mut rng = Seeded::new(5);
    let mut acts = vec![0u16; env.len()];
    let mut out = Vec::new();
    for call in 0..calls {
        let o = observe_all(&env);
        for g in 0..env.len() {
            let legal: Vec<u16> = (0..N_ALL_ACTIONS).filter(|&a| o.actions[g * N_ALL_ACTIONS + a] == 1).map(|a| a as u16).collect();
            let a = legal[rng.below(legal.len() as u32) as usize];
            acts[g] = if a as usize >= env::PREVIEW0 { a - env::PREVIEW0 as u16 } else { a };
        }
        if call % 50 == 0 {
            out.push(format!("{:?}", o));
        }
        env.act(&acts).unwrap();
        out.push(format!("{:?} {:?}", env.results(), env.labels()));
    }
    for g in 0..env.len() {
        out.push(format!("{:?} {:?} {:?} {:?}", env.slot(g).game.state, env.battle(g, 0), env.battle(g, 1), env.pending(g)));
    }
    out.push(counts(&env.counters));
    out.push(format!("{} {}", env.games_done, env.decisions));
    (out, env)
}

#[test]
fn threads_give_the_same_games() {
    let (one, e1) = play(1, false, 400);
    assert!(e1.games_done > 300, "games {}", e1.games_done);
    for t in [2, 8, 24] {
        let (many, _) = play(t, false, 400);
        assert_eq!(one.len(), many.len());
        for (i, (a, b)) in one.iter().zip(&many).enumerate() {
            assert!(a == b, "{} threads differ from one at entry {}", t, i);
        }
    }
}

#[test]
fn threads_give_the_same_checks() {
    let (one, e1) = play(1, true, 250);
    assert!(e1.counters.checked > 10_000 && e1.counters.hidden_games > 10, "{} {}", e1.counters.checked, e1.counters.hidden_games);
    let (many, _) = play(8, true, 250);
    for (i, (a, b)) in one.iter().zip(&many).enumerate() {
        assert!(a == b, "8 threads differ from one at entry {}", i);
    }
}
