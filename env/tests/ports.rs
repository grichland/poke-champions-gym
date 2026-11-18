//! The ported players in real games against tower: every rule fires (the pilots on their own team pools),
//! no choice falls outside the legal set, and the counts are reported. Checks are off for speed;
//! `players_run` runs the same players with every check on.

mod common;

use battle::ReqKind;
use env::batch::preview_action;
use env::teams::{TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind};
use players::{by_name, Rng};
use std::collections::BTreeMap;
use std::sync::Arc;

const GAMES: u64 = 3000;

/// Plays `name` against tower; returns its rule firings and the number of choices outside the legal set.
fn play(name: &str, pool: TeamPool, seed: u64) -> (BTreeMap<&'static str, u64>, u64) {
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: false, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(Arc::new(pool), cfg);
    let mut ps = [by_name(name).unwrap(), by_name("tower").unwrap()];
    let mut legal = Vec::new();
    let mut outside = 0;
    while env.games_done < GAMES {
        let p = env.pending(0);
        // the player under test alternates seats by game
        let seat = p.seat as usize;
        let who = (seat + env.games_done as usize) % 2;
        let bt = *env.battle(0, seat);
        let mut rng = Rng::new(seed ^ (env.decisions << 1) ^ who as u64);
        let act = match p.kind {
            RequestKind::Preview => preview_action(ps[who].teampreview(&bt, &mut rng)).unwrap(),
            k => ps[who].choose_move(&bt, if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn }, &mut rng) as u16,
        };
        env.legal_actions(0, &mut legal);
        let act = if legal.contains(&act) {
            act
        } else {
            outside += 1;
            legal[0]
        };
        env.act(&[act]).unwrap();
    }
    (ps[0].fired().into_iter().collect(), outside)
}

fn shipped() -> TeamPool {
    common::pool()
}

fn style(kind: &str) -> TeamPool {
    let path = format!("{}/../bench/data/pools/pilot_{}.json", env!("CARGO_MANIFEST_DIR"), kind);
    let p = TeamPool::from_file(&path).expect("pilot pool");
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    p
}

/// Every rule fired at least once, except the guards in `rare`.
fn check(name: &str, pool: TeamPool, rare: &[&str]) {
    let (f, outside) = play(name, pool, 19);
    println!("{} over {} games: {:?}", name, GAMES, f);
    assert_eq!(outside, 0, "{} chose outside the legal set", name);
    assert!(f["decisions"] > GAMES * 4, "{:?}", f);
    for (k, v) in &f {
        if !rare.contains(k) {
            assert!(*v > 0, "{}: rule {:?} never fired: {:?}", name, k, f);
        }
    }
}

#[test]
fn best_matchup_rules_fire() {
    check("best_matchup_max_damage", shipped(), &["switch_blocked"]);
}

#[test]
fn disciplined_rules_fire() {
    check("disciplined", shipped(), &[]);
}

#[test]
fn endgame_rules_fire() {
    check("endgame", shipped(), &["endgame:fallback_unseen"]);
}

#[test]
fn pilot_rules_fire_on_their_pools() {
    for k in ["encore", "weather", "hazard", "priority"] {
        check(&format!("pilot_{}", k), style(k), &[]);
    }
}
