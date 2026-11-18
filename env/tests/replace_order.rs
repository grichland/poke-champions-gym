//! The live order (`replace_before_rows`): the same games played with the replace request before and after
//! the rows of its turn. Every Battle at a turn request is the same in both orders; at a replace request the
//! live-order Battle holds nothing of the turn's rows, offers everything the State allows (except a
//! Pokémon dragged out in the unread rows), and observes the same whatever happened in those rows.

mod common;

use battle::{legal, ReqKind};
use env::batch::{each_turn_action, Slot};
use env::teams::TeamPool;
use env::{BatchEnv, EnvConfig, RequestKind};
use cg_engine::sim::outcome::Seeded;
use std::sync::Arc;

fn pool_file(name: &str) -> TeamPool {
    TeamPool::from_file(&format!("{}/../bench/data/pools/{}.json", env!("CARGO_MANIFEST_DIR"), name)).expect("pool")
}

fn env_of(pool: &Arc<TeamPool>, seed: u64, on: bool, checks: bool) -> BatchEnv {
    let cfg = EnvConfig { games: 1, seed, max_steps: 400, checks, keep_records: checks, replace_before_rows: on, ..EnvConfig::default() };
    BatchEnv::new(pool.clone(), cfg)
}

fn mask_of(s: &Slot) -> [bool; 11] {
    let mut m = [false; 11];
    each_turn_action(&s.opts[s.pending.seat as usize], |a| m[a as usize] = true);
    m
}

fn state_mask(s: &Slot) -> [bool; 11] {
    let mut m = [false; 11];
    each_turn_action(&s.state_opts[s.pending.seat as usize], |a| m[a as usize] = true);
    m
}

#[derive(Default, Debug)]
struct Seen {
    games: u64,
    turn_requests: u64,
    replace_requests: u64,
    stale: u64,
    /// live-order replace requests whose mask is not a superset of the State's (dragged out unseen)
    not_superset: u64,
    /// games stopped because no action was legal in both orders
    stopped: u64,
}

/// Plays `games` games in both orders with the same actions (drawn among those legal in the State and in
/// both orders' masks) and compares.
fn both_orders(pool: TeamPool, seed: u64, games: u64) -> Seen {
    let pool = Arc::new(pool);
    let mut env_seed = seed;
    let mut on = env_of(&pool, env_seed, true, false);
    let mut off = env_of(&pool, env_seed, false, false);
    let mut rng = Seeded::new(seed ^ 0x5eed);
    let mut seen = Seen::default();
    while seen.games < games {
        let (po, pf) = (on.pending(0), off.pending(0));
        assert_eq!(po, pf, "pending differs");
        let (so, sf) = (on.slot(0), off.slot(0));
        let seat = po.seat as usize;
        let act = if po.kind == RequestKind::Preview {
            rng.below(60) as u16
        } else {
            let (mo, mf, ms) = (mask_of(so), mask_of(sf), state_mask(so));
            assert_eq!(ms, state_mask(sf), "State options differ");
            match po.kind {
                RequestKind::Turn => {
                    seen.turn_requests += 1;
                    let (bo, bf) = (on.battle(0, seat), off.battle(0, seat));
                    assert!(bo == bf, "Battles differ at a turn request: ep {} turn {} seat {}", so.episode, so.game.state.turn, seat);
                    assert_eq!(so.seats.held_events[seat], 0);
                }
                _ => {
                    seen.replace_requests += 1;
                    if so.seats.held_events[seat] > 0 {
                        seen.stale += 1;
                        assert!(on.battle(0, seat).ctx.unread, "stale replace without unread");
                    }
                    // the old order's Battle allows exactly the State's set here
                    assert_eq!(mf, ms, "old order at a replace: ep {} turn {}", sf.episode, sf.game.state.turn);
                    if (0..11).any(|a| ms[a] && !mo[a]) {
                        seen.not_superset += 1;
                    }
                    assert_eq!(legal(on.battle(0, seat), ReqKind::Replace).mask, mo, "mask is the Battle's legal set");
                }
            }
            let both: Vec<u16> = (0..11u16).filter(|&a| mo[a as usize] && mf[a as usize] && ms[a as usize]).collect();
            if both.is_empty() {
                // only a Pokémon dragged out unseen was left: this game cannot go on in both orders
                seen.stopped += 1;
                env_seed += 1000;
                on = env_of(&pool, env_seed, true, false);
                off = env_of(&pool, env_seed, false, false);
                continue;
            }
            both[rng.below(both.len() as u32) as usize]
        };
        on.act(&[act]).unwrap();
        off.act(&[act]).unwrap();
        let (ro, rf) = (on.results()[0], off.results()[0]);
        assert_eq!(ro, rf, "results differ");
        if ro.done {
            seen.games += 1;
            assert_eq!(on.counters.refusals, 0);
        }
    }
    seen
}

#[test]
fn both_orders_give_the_same_battles_on_the_shipped_pool() {
    let s = both_orders(common::pool(), 11, 400);
    assert!(s.stopped <= s.not_superset, "{:?}", s);
    assert!(s.replace_requests > 300 && s.stale == s.replace_requests, "{:?}", s);
    assert!(s.not_superset * 200 <= s.replace_requests, "{:?}", s);
}

#[test]
fn both_orders_give_the_same_battles_on_the_mechanics_pools() {
    for (i, name) in ["field", "traps", "calls", "stages", "exchanges", "pp", "abilities", "charges", "turn_end", "items", "imprison", "hidden_trap"].iter().enumerate() {
        let s = both_orders(pool_file(name), 100 + i as u64, 60);
        assert!(s.stopped <= s.not_superset, "{} {:?}", name, s);
        assert!(s.replace_requests > 0, "{} {:?}", name, s);
    }
}

/// With every check on in the live order: check a at a replace request runs against the State at the
/// seat's last turn request, the hidden-information replay holds, and replaying the step before a replace
/// request with other outcomes never changes what the seat is shown at it.
#[test]
fn live_order_checks_hold() {
    let pool = Arc::new(common::pool());
    let mut env = env_of(&pool, 21, true, true);
    let mut rng = Seeded::new(5);
    let mut legal_now = Vec::new();
    while env.games_done < 200 {
        env.legal_actions(0, &mut legal_now);
        let a = legal_now[rng.below(legal_now.len() as u32) as usize];
        env.act(&[a]).unwrap();
    }
    let c = &env.counters;
    assert_eq!(c.agree.unexplained(), 0, "{:?}", c.agree.examples);
    assert_eq!(c.hidden.total(), 0, "{:?}", c.hidden.examples);
    assert_eq!(c.rebuild_equal, c.rebuilt);
    assert_eq!(c.anomalies.unexplained(), 0, "{:?}", c.anomalies.examples);
    assert!(c.stale_replace > 100 && c.stale_checked >= c.stale_replace, "{} {}", c.stale_replace, c.stale_checked);
    assert!(c.replace_twins > 20 && c.replace_twins_varied > 0, "{} {}", c.replace_twins, c.replace_twins_varied);
    assert_eq!(c.battle_forbids, 0, "{:?}", c.legal.examples);
    assert_eq!(c.refused.unexplained(), 0, "{:?}", c.refused.examples);
}
