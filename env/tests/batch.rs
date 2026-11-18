mod common;

use env::batch::{preview_action, preview_picks, N_PREVIEW_ACTIONS};
use env::{BatchEnv, EnvConfig, RequestKind, TeamMode};
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::Kind;
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn preview_layout_round_trips() {
    let mut seen = HashSet::new();
    for a in 0..N_PREVIEW_ACTIONS as u16 {
        let p = preview_picks(a).unwrap();
        assert!(p.iter().all(|&x| x < 6) && p[0] != p[1] && p[0] != p[2] && p[1] < p[2]);
        assert_eq!(preview_action(p), Some(a));
        assert_eq!(preview_action([p[0], p[2], p[1]]), Some(a));
        seen.insert(p);
    }
    assert_eq!(seen.len(), 60);
    assert!(preview_picks(60).is_none());
}

fn run(env: &mut BatchEnv, rng: &mut Seeded, calls: usize, log: &mut Vec<(usize, u8, u16)>) {
    let mut acts = vec![0u16; env.len()];
    let mut legal = Vec::new();
    for _ in 0..calls {
        for g in 0..env.len() {
            env.legal_actions(g, &mut legal);
            acts[g] = legal[rng.below(legal.len() as u32) as usize];
        }
        env.act(&acts).unwrap();
        for (g, r) in env.results().iter().enumerate() {
            if r.done {
                log.push((g, r.winner.unwrap_or(9), r.turns));
            }
        }
    }
}

#[test]
fn same_master_seed_same_games() {
    let pool = Arc::new(common::pool());
    let cfg = EnvConfig { games: 16, seed: 42, team_mode: TeamMode::Stage3, max_steps: 1000, ..Default::default() };
    let (mut a, mut b) = (BatchEnv::new(pool.clone(), cfg), BatchEnv::new(pool.clone(), cfg));
    let (mut la, mut lb) = (vec![], vec![]);
    run(&mut a, &mut Seeded::new(3), 3000, &mut la);
    run(&mut b, &mut Seeded::new(3), 3000, &mut lb);
    assert!(la.len() > 50, "{}", la.len());
    assert_eq!(la, lb);
    for g in 0..a.len() {
        assert!(a.slot(g).game.state == b.slot(g).game.state);
        assert_eq!(a.pending(g), b.pending(g));
    }
    let mut c = BatchEnv::new(pool, EnvConfig { seed: 43, ..cfg });
    let mut lc = vec![];
    run(&mut c, &mut Seeded::new(3), 3000, &mut lc);
    assert_ne!(la, lc);
    // results make sense
    assert!(la.iter().all(|&(_, w, t)| t >= 1 && (w == 0 || w == 1 || w == 9)));
}

#[test]
fn strict_turn_taking() {
    let pool = Arc::new(common::pool());
    let mut env = BatchEnv::new(pool, EnvConfig { games: 8, seed: 7, ..Default::default() });
    let mut rng = Seeded::new(11);
    let mut legal = Vec::new();
    let (mut both, mut single, mut previews) = (0, 0, 0);
    for _ in 0..4000 {
        let before: Vec<_> = (0..env.len()).map(|g| (env.pending(g), env.slot(g).game.state, env.slot(g).episode)).collect();
        let mut acts = vec![0u16; env.len()];
        for g in 0..env.len() {
            env.legal_actions(g, &mut legal);
            assert!(!legal.is_empty());
            acts[g] = legal[rng.below(legal.len() as u32) as usize];
        }
        env.act(&acts).unwrap();
        for g in 0..env.len() {
            let (p, st, ep) = before[g];
            let now = env.pending(g);
            let s = env.slot(g);
            match p.kind {
                RequestKind::Preview if p.seat == 0 => {
                    assert_eq!(now, env::Pending { seat: 1, kind: RequestKind::Preview });
                    previews += 1;
                }
                RequestKind::Preview => assert!(now.kind != RequestKind::Preview || s.episode != ep),
                _ => {
                    let o = st.options();
                    // seats the game asks: deciding and not locked into a move
                    let deciders: Vec<usize> =
                        (0..2).filter(|&x| matches!(o[x].kind, Kind::Turn | Kind::Replace) && !(o[x].kind == Kind::Turn && st.locked_move(x).is_some())).collect();
                    if s.game.state == st && now == p && s.episode == ep {
                        // the State refused the order: the same seat is asked again
                        continue;
                    }
                    if p.seat == 0 && deciders.len() == 2 {
                        // seat 0 chose: nothing resolved, seat 1 asked next
                        assert!(s.game.state == st);
                        assert_eq!(now.seat, 1);
                        both += 1;
                    } else {
                        // the last seat in: the step resolved
                        assert!(s.game.state != st || s.episode != ep);
                        single += (deciders.len() == 1) as usize;
                    }
                }
            }
        }
    }
    assert!(both > 1000 && previews > 50 && single > 10, "{} {} {}", both, previews, single);
}

#[test]
fn illegal_action_is_refused_without_change() {
    let pool = Arc::new(common::pool());
    let mut env = BatchEnv::new(pool, EnvConfig { games: 3, seed: 1, ..Default::default() });
    assert!(env.act(&[0, 0]).is_err());
    assert!(env.act(&[0, 0, 60]).is_err());
    assert_eq!(env.decisions, 0);
    env.act(&[0, 1, 2]).unwrap();
    env.act(&[0, 1, 2]).unwrap();
    for g in 0..3 {
        assert_eq!(env.pending(g).kind, RequestKind::Turn);
        assert_eq!(env.pending(g).seat, 0);
    }
    // a switch to the active slot is never legal
    let active = env.slot(0).game.state.sides[0].active as u16;
    let before = env.slot(0).game.state;
    assert!(env.act(&[8 + active, 0, 0]).is_err());
    assert!(env.slot(0).game.state == before);
    assert_eq!(env.pending(0).seat, 0);
}
