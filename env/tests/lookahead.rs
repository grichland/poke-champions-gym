//! The `lookahead` Player: names and options, legal choices with every check on, determinism, rule
//! counts, depth 2, the fallback when no State can be built, and that only its Battle decides.

mod common;

use battle::{legal, Battle, ReqKind};
use env::batch::preview_action;
use env::builtin::{by_name, NAMES};
use env::from_battle::state_from_battle;
use env::lookahead::{opponent_three, Config, LookaheadPlayer};
use env::teams::{TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind, Rule};
use players::{Player, Rng};
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn names_and_options() {
    assert!(NAMES.contains(&"lookahead") && NAMES.contains(&"lookahead2"));
    for n in NAMES {
        assert!(by_name(n).is_some(), "{}", n);
    }
    // the ten scripted players keep their place and names
    assert_eq!(&NAMES[..10], &players::NAMES[..]);
    assert_eq!(Config::parse("lookahead"), Some(Config::default()));
    assert_eq!(Config::parse("lookahead2").unwrap().depth, 2);
    let c = Config::parse("lookahead@depth=2,samples=3,replies=1,seed=9,fill=1").unwrap();
    assert_eq!((c.depth, c.samples, c.replies, c.seed, c.fill), (2, 3, 1, 9, 1));
    assert_eq!(Config::parse("lookahead2@samples=1").unwrap().depth, 2);
    for bad in ["lookahead@depth=3", "lookahead@samples=0", "lookahead@what=1", "lookahead@samples", "lookahead3", "lookahead@fill=x", "lookahead@fill=0"] {
        assert!(Config::parse(bad).is_none() && by_name(bad).is_none(), "{}", bad);
    }
    assert_eq!(by_name("lookahead@samples=2").unwrap().name(), "lookahead");
    assert_eq!(by_name("lookahead2").unwrap().name(), "lookahead2");
}

struct Run {
    actions: Vec<u16>,
    outside: u64,
    winners: Vec<Option<u8>>,
    fired: BTreeMap<&'static str, u64>,
    refusals: u64,
    anomalies: u64,
    errors: u64,
    rebuilt: (u64, u64),
    kinds: [u64; 3],
}

/// `games` games of `name` against `other` (alternating seats), every check on.
fn play(name: &str, other: &str, games: u64, seed: u64, replace_before_rows: bool) -> Run {
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: true, keep_records: true, auto_locked: true, replace_before_rows };
    let mut env = BatchEnv::with_rule(Arc::new(common::pool()), cfg, Rule::Battle);
    let mut ps = [by_name(name).unwrap(), by_name(other).unwrap()];
    let mut lg = Vec::new();
    let mut r = Run { actions: vec![], outside: 0, winners: vec![], fired: BTreeMap::new(), refusals: 0, anomalies: 0, errors: 0, rebuilt: (0, 0), kinds: [0; 3] };
    while env.games_done < games {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let who = (seat + env.games_done as usize) % 2;
        let b = *env.battle(0, seat);
        let mut rng = Rng::new(seed ^ (env.decisions << 1) ^ who as u64);
        let act = match p.kind {
            RequestKind::Preview => {
                if who == 0 {
                    r.kinds[0] += 1;
                }
                preview_action(ps[who].teampreview(&b, &mut rng)).expect("three distinct picks")
            }
            k => {
                let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                let a = ps[who].choose_move(&b, rk, &mut rng);
                if who == 0 {
                    r.kinds[if rk == ReqKind::Turn { 1 } else { 2 }] += 1;
                    if !legal(&b, rk).mask[a] {
                        r.outside += 1;
                    }
                }
                a as u16
            }
        };
        env.legal_actions(0, &mut lg);
        if who == 0 {
            r.actions.push(act);
        }
        let act = if lg.contains(&act) { act } else { lg[0] };
        env.act(&[act]).unwrap();
        let res = env.results()[0];
        if res.done {
            r.winners.push(res.winner);
        }
    }
    r.fired = ps[0].fired().into_iter().collect();
    r.refusals = env.counters.refusals;
    r.anomalies = env.counters.anomalies.counts.values().sum();
    r.errors = env.counters.errors;
    r.rebuilt = (env.counters.rebuilt, env.counters.rebuild_equal);
    r
}

#[test]
fn legal_choices_with_checks_on() {
    for (other, rbr) in [("tower", true), ("max_damage", false), ("disciplined", true)] {
        let r = play("lookahead", other, 150, 41, rbr);
        println!("lookahead vs {} (replace before rows {}): kinds {:?} refusals {} fired {:?}", other, rbr, r.kinds, r.refusals, r.fired);
        assert_eq!(r.outside, 0, "a choice outside the Battle's legal set");
        assert_eq!(r.anomalies, 0);
        assert_eq!(r.errors, 0);
        assert_eq!(r.rebuilt.0, r.rebuilt.1, "Battles rebuilt from the record");
        assert!(r.kinds.iter().all(|&k| k > 0), "preview, turn and replace decisions all made: {:?}", r.kinds);
        assert!(r.fired["search"] > 0 && r.fired["search_replace"] > 0 && r.fired["preview"] > 0, "{:?}", r.fired);
        assert!(r.fired["fallback"] * 20 <= r.fired["decisions"], "{:?}", r.fired);
        // refusals come only from what no Battle can know (a hidden trap, a stale replace screen)
        assert!(r.refusals * 50 <= r.kinds[1] + r.kinds[2], "refusals {}", r.refusals);
    }
}

#[test]
fn depth_two_and_options_play() {
    for name in ["lookahead2", "lookahead@samples=1,replies=1,fill=1", "lookahead@depth=2,samples=2,replies=3,fill=1"] {
        let r = play(name, "tower", 25, 43, true);
        println!("{}: {:?}", name, r.fired);
        assert_eq!(r.outside, 0, "{}", name);
        assert_eq!(r.anomalies + r.errors, 0, "{}", name);
        assert!(r.fired["fallback"] * 20 <= r.fired["decisions"], "{}: {:?}", name, r.fired);
        if name.contains("2") && !name.contains("samples=1") {
            assert!(r.fired["depth2"] > 0, "{}: {:?}", name, r.fired);
        }
    }
}

#[test]
fn same_seed_same_games() {
    let a = play("lookahead", "tower", 40, 47, true);
    let b = play("lookahead", "tower", 40, 47, true);
    assert_eq!(a.actions, b.actions);
    assert_eq!(a.winners, b.winners);
    // the option seed changes the draws, not the legality
    let c = play("lookahead@seed=5", "tower", 40, 47, true);
    assert_eq!(c.outside, 0);
}

/// Every turn and replace Battle of `games` games, with the request kind.
fn battles(games: u64, seed: u64) -> Vec<(Battle, ReqKind)> {
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: false, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(Arc::new(common::pool()), cfg);
    let mut ps = [by_name("tower").unwrap(), by_name("max_damage").unwrap()];
    let mut out = vec![];
    let mut rng = Rng::new(seed);
    let mut lg = Vec::new();
    while env.games_done < games {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let b = *env.battle(0, seat);
        let act = match p.kind {
            RequestKind::Preview => preview_action(ps[seat].teampreview(&b, &mut rng)).unwrap(),
            k => {
                let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                out.push((b, rk));
                ps[seat].choose_move(&b, rk, &mut rng) as u16
            }
        };
        env.legal_actions(0, &mut lg);
        env.act(&[if lg.contains(&act) { act } else { lg[0] }]).unwrap();
    }
    out
}

#[test]
fn a_battle_and_a_seed_decide() {
    let bs = battles(30, 53);
    let mut fresh_vs_used = 0;
    let mut used = LookaheadPlayer::new(Config::default());
    for (i, (b, k)) in bs.iter().enumerate() {
        let mut p = LookaheadPlayer::new(Config::default());
        let x = p.choose_move(b, *k, &mut Rng::new(i as u64));
        // the same Player after other decisions, and a copy of the Battle, choose the same
        let copy = *b;
        let y = used.choose_move(&copy, *k, &mut Rng::new(i as u64));
        assert_eq!(x, y, "decision {}", i);
        assert!(legal(b, *k).mask[x] || legal(b, *k).count() == 0);
        fresh_vs_used += 1;
    }
    assert!(fresh_vs_used > 100);
}

#[test]
fn works_from_threads_in_any_order() {
    let bs = Arc::new(battles(20, 59));
    let serial: Vec<usize> = bs.iter().enumerate().map(|(i, (b, k))| LookaheadPlayer::new(Config::default()).choose_move(b, *k, &mut Rng::new(i as u64 * 7))).collect();
    let par: Vec<usize> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..4)
            .map(|t| {
                let bs = bs.clone();
                sc.spawn(move || {
                    let mut p = by_name("lookahead").unwrap();
                    // each thread takes every fourth decision, last first
                    let mut v: Vec<(usize, usize)> = (0..bs.len()).filter(|i| i % 4 == t).rev().map(|i| (i, p.choose_move(&bs[i].0, bs[i].1, &mut Rng::new(i as u64 * 7)))).collect();
                    v.reverse();
                    v
                })
            })
            .collect();
        let mut all: Vec<(usize, usize)> = hs.into_iter().flat_map(|h| h.join().unwrap()).collect();
        all.sort();
        all.into_iter().map(|x| x.1).collect()
    });
    assert_eq!(serial, par);
}

#[test]
fn no_state_falls_back_to_a_legal_choice() {
    // replace requests before their rows: some have nobody to send in and no State can be built
    let bs = battles(400, 61);
    let mut no_state = 0;
    for (i, (b, k)) in bs.iter().enumerate() {
        let (g, _) = opponent_three(b, 2);
        if state_from_battle(b, *k, 0, &g, &[None; 6], 1).is_ok() {
            continue;
        }
        let l = legal(b, *k);
        if l.count() < 2 {
            continue;
        }
        no_state += 1;
        let mut p = LookaheadPlayer::new(Config::default());
        let a = p.choose_move(b, *k, &mut Rng::new(i as u64));
        assert!(l.mask[a], "fallback outside the legal set");
        assert_eq!(p.fired.get("fallback"), 1);
    }
    println!("decisions with no State and a choice to make: {}", no_state);
    assert!(no_state > 0, "no such decision found");
    // a Battle with no battle_start: no State, still an answer
    let empty = Battle::default();
    let mut p = LookaheadPlayer::new(Config::default());
    let _ = p.choose_move(&empty, ReqKind::Turn, &mut Rng::new(1));
    let _ = p.teampreview(&empty, &mut Rng::new(1));
}

/// Team pool of two teams: the first for seat 0 only, the second for seat 1 only.
fn pair_pool(t0: &str, t1: &str) -> TeamPool {
    let mut p = TeamPool::from_json(&format!("[{},{}]", t1, t0)).expect("teams");
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    p.set_weights(&[0.0, 1.0], &[1.0, 0.0], false).expect("weights");
    p
}

/// Seat 0 (lookahead) against random on fixed teams; its Battles and choices until the game ends.
fn seat0_record(pool: TeamPool, seed: u64) -> Vec<(Battle, usize)> {
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: false, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::with_agent(Arc::new(pool), cfg, Rule::Battle, vec![0]);
    let mut ps = [by_name("lookahead").unwrap(), by_name("random").unwrap()];
    let mut out = vec![];
    let mut lg = Vec::new();
    while env.games_done < 1 {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let b = *env.battle(0, seat);
        let mut rng = Rng::new(seed ^ env.decisions << 1 ^ seat as u64);
        let act = match p.kind {
            RequestKind::Preview => preview_action(ps[seat].teampreview(&b, &mut rng)).unwrap(),
            k => {
                let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                ps[seat].choose_move(&b, rk, &mut rng) as u16
            }
        };
        if seat == 0 {
            out.push((b, act as usize));
        }
        env.legal_actions(0, &mut lg);
        env.act(&[if lg.contains(&act) { act } else { lg[0] }]).unwrap();
    }
    out
}

#[test]
fn hidden_information_does_not_decide() {
    // the opponent's sets differ in what seat 0 is not shown before they act: natures, stat points, items
    // and the last move of every member, and everything of the members it leaves behind
    let file = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../python/champions_gym/data/teams.json")).unwrap();
    let teams = match battle::json::Json::parse(&file).unwrap() {
        battle::json::Json::Arr(v) => v,
        _ => panic!("teams"),
    };
    let mut compared = 0;
    let mut games = 0;
    for k in 0..12 {
        let mine = teams[2 * k].to_string();
        let theirs = teams[2 * k + 1].to_string();
        let changed = theirs
            .replace("\"Adamant\"", "\"Hardy\"")
            .replace("\"Modest\"", "\"Hardy\"")
            .replace("\"Jolly\"", "\"Hardy\"")
            .replace("\"Timid\"", "\"Hardy\"")
            .replace("\"Leftovers\"", "\"Shell Bell\"")
            .replace("\"Sitrus Berry\"", "\"Shell Bell\"");
        if changed == theirs {
            continue;
        }
        let a = seat0_record(pair_pool(&mine, &theirs), 100 + k as u64);
        let b = seat0_record(pair_pool(&mine, &changed), 100 + k as u64);
        games += 1;
        for ((ba, xa), (bb, xb)) in a.iter().zip(b.iter()) {
            if ba != bb {
                break;
            }
            assert_eq!(xa, xb, "same Battle, different choice (pair {})", k);
            compared += 1;
        }
    }
    println!("{} games, {} decisions with equal Battles compared", games, compared);
    assert!(games >= 5 && compared >= 2 * games);
}
