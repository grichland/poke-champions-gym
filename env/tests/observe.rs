//! `observe_batch`: each game's entry equals `obs::observe` of its pending seat's Battle, threads change
//! nothing, the action mask equals `legal_actions`, and preview entries hold what the module doc says.

mod common;

use battle::{legal, ReqKind};
use env::observe::{kind_code, ObsBatch, ObsOptions, ROWS_LEN, ROW_IDS_LEN};
use env::{BatchEnv, EnvConfig, RequestKind, TeamMode, N_ALL_ACTIONS, PREVIEW0};
use obs::layout::*;
use obs::{ObsBuffers, FIELD, LAST, LAST_IDS, MASK};
use cg_engine::sim::outcome::Seeded;
use std::sync::Arc;

#[derive(Clone, PartialEq, Debug)]
struct Arrays {
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
    opp_prior: Vec<f32>,
}

impl Arrays {
    fn new(n: usize) -> Arrays {
        Arrays {
            rows: vec![7.0; n * ROWS_LEN],
            row_ids: vec![7; n * ROW_IDS_LEN],
            field: vec![7.0; n * FIELD],
            last: vec![7.0; n * LAST],
            last_ids: vec![7; n * LAST_IDS],
            mask: vec![7; n * MASK],
            actions: vec![7; n * N_ALL_ACTIONS],
            seat: vec![7; n],
            kind: vec![7; n],
            decision: vec![7; n],
            opp_prior: vec![7.0; n * 13],
        }
    }
    fn out(&mut self) -> ObsBatch<'_> {
        ObsBatch {
            rows: &mut self.rows,
            row_ids: &mut self.row_ids,
            field: &mut self.field,
            last: &mut self.last,
            last_ids: &mut self.last_ids,
            mask: &mut self.mask,
            actions: &mut self.actions,
            seat: &mut self.seat,
            kind: &mut self.kind,
            decision: &mut self.decision,
            opp_prior: &mut self.opp_prior,
        }
    }
}

#[test]
fn batch_entries_equal_single_observations() {
    let pool = Arc::new(common::pool());
    let n = 24;
    let cfg = EnvConfig { games: n, seed: 5, team_mode: TeamMode::Stage3, max_steps: 1000, ..Default::default() };
    let mut env = BatchEnv::new(pool, cfg);
    let mut rng = Seeded::new(1);
    let mut legal_v = Vec::new();
    let mut one = Box::new(ObsBuffers::default());
    let (mut previews, mut turns) = (0, 0);
    for step in 0..600 {
        let mut a = Arrays::new(n);
        env.observe_batch(a.out()).unwrap();
        if step % 50 == 0 {
            let mut t = Arrays::new(n);
            env.observe_batch_with(t.out(), ObsOptions { threads: 5, damage: true }).unwrap();
            assert_eq!(a, t, "threads changed the tensors");
        }
        for g in 0..n {
            let p = env.pending(g);
            assert_eq!((a.seat[g], a.kind[g]), (p.seat, kind_code(p.kind)));
            let b = env.battle(g, p.seat as usize);
            let rk = if p.kind == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
            obs::observe(b, rk, &mut one);
            assert_eq!(&a.rows[g * ROWS_LEN..(g + 1) * ROWS_LEN], one.rows.as_flattened());
            assert_eq!(&a.row_ids[g * ROW_IDS_LEN..(g + 1) * ROW_IDS_LEN], one.row_ids.as_flattened());
            assert_eq!(&a.field[g * FIELD..(g + 1) * FIELD], &one.field[..]);
            assert_eq!(&a.last[g * LAST..(g + 1) * LAST], &one.last[..]);
            assert_eq!(&a.last_ids[g * LAST_IDS..(g + 1) * LAST_IDS], &one.last_ids[..]);
            assert_eq!(&a.opp_prior[g * 13..(g + 1) * 13], &one.opp_prior[..]);
            if p.kind == RequestKind::Preview {
                assert!(one.opp_prior.iter().all(|&x| x == 0.0));
            }
            let m = &a.mask[g * MASK..(g + 1) * MASK];
            let acts = &a.actions[g * N_ALL_ACTIONS..(g + 1) * N_ALL_ACTIONS];
            env.legal_actions(g, &mut legal_v);
            let off = if p.kind == RequestKind::Preview { PREVIEW0 } else { 0 };
            for x in 0..N_ALL_ACTIONS {
                assert_eq!(acts[x] == 1, x >= off && legal_v.contains(&((x - off) as u16)), "game {} action {}", g, x);
            }
            if p.kind == RequestKind::Preview {
                previews += 1;
                assert!(m.iter().all(|&x| x == 0));
                assert!(!legal(b, ReqKind::Turn).mask.iter().any(|&x| x));
                let r = &a.rows[g * ROWS_LEN..(g + 1) * ROWS_LEN];
                let ids = &a.row_ids[g * ROW_IDS_LEN..(g + 1) * ROW_IDS_LEN];
                for row in 0..12 {
                    assert!(ids[row * ROWIDS_LEN + F1_3] > 0, "preview row {} has no species", row);
                    assert_eq!(r[row * ROW_LEN + F2_5], (row < 6) as u8 as f32);
                    assert_eq!(r[row * ROW_LEN + F4_1], 0.0);
                    assert_eq!(r[row * ROW_LEN + F3_9], 0.0);
                    let moves_known = (0..4).filter(|j| ids[row * ROWIDS_LEN + F7_13 + j] > 0).count();
                    assert_eq!(moves_known > 0, row < 6);
                }
                let f = &a.field[g * FIELD..(g + 1) * FIELD];
                assert!(f.iter().enumerate().all(|(k, &x)| x == if k == G1_4_1 || k == G1_4_2 { 1.0 } else { 0.0 }), "{:?}", f);
                let l = &a.last[g * LAST..(g + 1) * LAST];
                assert!(l.iter().enumerate().all(|(k, &x)| x == if k == L1_1 + 2 || k == L1_4 + 2 || k == L3_2 { 1.0 } else { 0.0 }), "{:?}", l);
                assert!(a.last_ids[g * LAST_IDS..(g + 1) * LAST_IDS].iter().all(|&x| x == 0));
            } else {
                turns += 1;
                assert_eq!(m.iter().map(|&x| x == 1).collect::<Vec<_>>(), legal(b, rk).mask.to_vec());
            }
        }
        let acts: Vec<u16> = (0..n)
            .map(|g| {
                env.legal_actions(g, &mut legal_v);
                legal_v[rng.below(legal_v.len() as u32) as usize]
            })
            .collect();
        env.act(&acts).unwrap();
    }
    assert!(previews > 100 && turns > 5000, "previews {} turns {}", previews, turns);
}

#[test]
fn wrong_lengths_are_refused() {
    let pool = Arc::new(common::pool());
    let cfg = EnvConfig { games: 4, ..Default::default() };
    let env = BatchEnv::new(pool, cfg);
    let mut a = Arrays::new(3);
    assert!(env.observe_batch(a.out()).is_err());
    let mut a = Arrays::new(4);
    a.field.pop();
    assert!(env.observe_batch(a.out()).is_err());
}
