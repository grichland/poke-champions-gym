//! Observations for the whole batch: for every game, the tensors of the seat that must act now, read from
//! that seat's Battle (`obs::observe_into`), written into caller-owned arrays of shape [N, ...].
//!
//! | array | shape | dtype |
//! |---|---|---|
//! | rows | [N, 12, 300] | f32 |
//! | row_ids | [N, 12, 10] | i32 |
//! | field | [N, 66] | f32 |
//! | last | [N, 17] | f32 |
//! | last_ids | [N, 4] | i32 |
//! | mask | [N, 11] | u8: `battle::legal` of the seat's Battle (the observation's M1-1) |
//! | actions | [N, 71] | u8: what `act` accepts, in the one action space (0..11 turn, 11..71 preview) |
//! | seat | [N] | u8 |
//! | kind | [N] | u8: 0 preview, 1 turn, 2 replace |
//! | opp_prior | [N, 13] | f32: how available each opponent action is (`obs::opp_prior`), an input |
//! | decision | [N] | i64: the pending decision's id, the key of its label (`labels`) |
//!
//! At team preview the seat's Battle holds its own six (in roster order, none brought yet) and the
//! opponent's six preview species, and nothing else: rows 0..6 are our six in team order with their full
//! builds and moves, rows 6..12 the opponent's six from the preview species alone (full HP, types, base
//! stats, species id, possible abilities); no row is active or brought. The field vector is 0 except the two alive counts (3 of 3),
//! the last-turn vector 0 except L1-1 and L1-4 "none" and L3-2 (no turn played yet), the last-turn ids 0,
//! and `mask` is all 0 (no turn action exists; the preview actions are in `actions`). The Battle is
//! observed as for a turn request.
//!
//! Under the Battle rule `mask` and the turn part of `actions` differ only where the environment fell
//! back to the State's options (the Battle allowed nothing, or kept offering an action the State refused);
//! under the State rule `actions` is the State's.

use crate::batch::{each_turn_action, BatchEnv, LegalRule, RequestKind, N_PREVIEW_ACTIONS, N_TURN_ACTIONS};
use crate::pool::{Chunks, Shards};
use battle::ReqKind;
use obs::{ObsOut, FIELD, LAST, LAST_IDS, MASK, N_ROWS, OPP_ACTIONS, ROW, ROW_IDS};

/// The one action space: 11 turn and replace actions, then the 60 preview actions.
pub const N_ALL_ACTIONS: usize = N_TURN_ACTIONS + N_PREVIEW_ACTIONS;
/// First preview action in the one action space.
pub const PREVIEW0: usize = N_TURN_ACTIONS;

pub const KIND_PREVIEW: u8 = 0;
pub const KIND_TURN: u8 = 1;
pub const KIND_REPLACE: u8 = 2;

/// Per-game lengths of the batch arrays.
pub const ROWS_LEN: usize = N_ROWS * ROW;
pub const ROW_IDS_LEN: usize = N_ROWS * ROW_IDS;

pub fn kind_code(k: RequestKind) -> u8 {
    match k {
        RequestKind::Preview => KIND_PREVIEW,
        RequestKind::Turn => KIND_TURN,
        RequestKind::Replace => KIND_REPLACE,
    }
}

/// Caller-owned batch arrays, each flat in C order with N games.
pub struct ObsBatch<'a> {
    pub rows: &'a mut [f32],
    pub row_ids: &'a mut [i32],
    pub field: &'a mut [f32],
    pub last: &'a mut [f32],
    pub last_ids: &'a mut [i32],
    pub mask: &'a mut [u8],
    pub actions: &'a mut [u8],
    pub seat: &'a mut [u8],
    pub kind: &'a mut [u8],
    pub decision: &'a mut [i64],
    pub opp_prior: &'a mut [f32],
}

impl<'a> ObsBatch<'a> {
    /// Games held, or an error naming the first array whose length does not fit.
    pub fn games(&self) -> Result<usize, String> {
        let n = self.seat.len();
        let lens = [
            ("rows", self.rows.len(), ROWS_LEN),
            ("row_ids", self.row_ids.len(), ROW_IDS_LEN),
            ("field", self.field.len(), FIELD),
            ("last", self.last.len(), LAST),
            ("last_ids", self.last_ids.len(), LAST_IDS),
            ("mask", self.mask.len(), MASK),
            ("actions", self.actions.len(), N_ALL_ACTIONS),
            ("kind", self.kind.len(), 1),
            ("decision", self.decision.len(), 1),
            ("opp_prior", self.opp_prior.len(), OPP_ACTIONS),
        ];
        for (name, len, per) in lens {
            if len != n * per {
                return Err(format!("observe: {} has {} elements, need {} x {}", name, len, n, per));
            }
        }
        Ok(n)
    }

    /// The first `g` games and the rest.
    fn split_at(self, g: usize) -> (ObsBatch<'a>, ObsBatch<'a>) {
        let (r0, r1) = self.rows.split_at_mut(g * ROWS_LEN);
        let (i0, i1) = self.row_ids.split_at_mut(g * ROW_IDS_LEN);
        let (f0, f1) = self.field.split_at_mut(g * FIELD);
        let (l0, l1) = self.last.split_at_mut(g * LAST);
        let (d0, d1) = self.last_ids.split_at_mut(g * LAST_IDS);
        let (m0, m1) = self.mask.split_at_mut(g * MASK);
        let (a0, a1) = self.actions.split_at_mut(g * N_ALL_ACTIONS);
        let (s0, s1) = self.seat.split_at_mut(g);
        let (k0, k1) = self.kind.split_at_mut(g);
        let (e0, e1) = self.decision.split_at_mut(g);
        let (q0, q1) = self.opp_prior.split_at_mut(g * OPP_ACTIONS);
        (
            ObsBatch { rows: r0, row_ids: i0, field: f0, last: l0, last_ids: d0, mask: m0, actions: a0, seat: s0, kind: k0, decision: e0, opp_prior: q0 },
            ObsBatch { rows: r1, row_ids: i1, field: f1, last: l1, last_ids: d1, mask: m1, actions: a1, seat: s1, kind: k1, decision: e1, opp_prior: q1 },
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ObsOptions {
    /// worker threads for the batch (1: the calling thread only)
    pub threads: usize,
    /// false leaves the damage features at 0
    pub damage: bool,
}

impl Default for ObsOptions {
    fn default() -> Self {
        ObsOptions { threads: 1, damage: true }
    }
}

impl<R: LegalRule + Sync> BatchEnv<R> {
    /// Every game's tensors for its pending seat (see the module doc), with all features.
    pub fn observe_batch(&self, out: ObsBatch<'_>) -> Result<(), String> {
        self.observe_batch_with(out, ObsOptions::default())
    }

    pub fn observe_batch_with(&self, out: ObsBatch<'_>, opt: ObsOptions) -> Result<(), String> {
        let n = out.games()?;
        if n != self.len() {
            return Err(format!("observe: arrays for {} games, the environment has {}", n, self.len()));
        }
        let threads = opt.threads.clamp(1, n.max(1));
        if threads == 1 {
            self.observe_range(0, out, opt.damage);
            return Ok(());
        }
        if let Some(pool) = self.workers() {
            // the environment's worker pool, in chunks of games
            let ch = (n / (pool.threads() * 8)).clamp(8, 256);
            let chunks = Chunks::new(n.div_ceil(ch));
            let sh = (
                Shards::new(out.rows),
                Shards::new(out.row_ids),
                Shards::new(out.field),
                Shards::new(out.last),
                Shards::new(out.last_ids),
                Shards::new(out.mask),
                Shards::new(out.actions),
                Shards::new(out.seat),
                Shards::new(out.kind),
                Shards::new(out.decision),
                Shards::new(out.opp_prior),
            );
            let job = |_: usize| {
                while let Some(c) = chunks.take() {
                    let (a, b) = (c * ch, ((c + 1) * ch).min(n));
                    // each chunk is handed out once: these parts are this worker's alone
                    let part = unsafe {
                        ObsBatch {
                            rows: sh.0.part(a * ROWS_LEN, b * ROWS_LEN),
                            row_ids: sh.1.part(a * ROW_IDS_LEN, b * ROW_IDS_LEN),
                            field: sh.2.part(a * FIELD, b * FIELD),
                            last: sh.3.part(a * LAST, b * LAST),
                            last_ids: sh.4.part(a * LAST_IDS, b * LAST_IDS),
                            mask: sh.5.part(a * MASK, b * MASK),
                            actions: sh.6.part(a * N_ALL_ACTIONS, b * N_ALL_ACTIONS),
                            seat: sh.7.part(a, b),
                            kind: sh.8.part(a, b),
                            decision: sh.9.part(a, b),
                            opp_prior: sh.10.part(a * OPP_ACTIONS, b * OPP_ACTIONS),
                        }
                    };
                    self.observe_range(a, part, opt.damage);
                }
            };
            pool.run(&job);
            return Ok(());
        }
        let per = n.div_ceil(threads);
        std::thread::scope(|sc| {
            let mut rest = out;
            let mut g0 = 0;
            while g0 < n {
                let k = per.min(n - g0);
                let (part, tail) = rest.split_at(k);
                rest = tail;
                let start = g0;
                sc.spawn(move || self.observe_range(start, part, opt.damage));
                g0 += k;
            }
        });
        Ok(())
    }

    /// Fills `out` (games g0..g0 + len) in place.
    fn observe_range(&self, g0: usize, out: ObsBatch<'_>, damage: bool) {
        for (i, s) in out.seat.iter_mut().enumerate() {
            let g = g0 + i;
            let p = self.pending(g);
            *s = p.seat;
            out.kind[i] = kind_code(p.kind);
            out.decision[i] = self.slot(g).decision as i64;
            let b = self.battle(g, p.seat as usize);
            let rk = if p.kind == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
            let mask = &mut out.mask[i * MASK..(i + 1) * MASK];
            obs::observe_into(
                b,
                rk,
                ObsOut {
                    rows: &mut out.rows[i * ROWS_LEN..(i + 1) * ROWS_LEN],
                    row_ids: &mut out.row_ids[i * ROW_IDS_LEN..(i + 1) * ROW_IDS_LEN],
                    field: &mut out.field[i * FIELD..(i + 1) * FIELD],
                    last: &mut out.last[i * LAST..(i + 1) * LAST],
                    last_ids: &mut out.last_ids[i * LAST_IDS..(i + 1) * LAST_IDS],
                    mask: &mut *mask,
                    opp_prior: &mut out.opp_prior[i * OPP_ACTIONS..(i + 1) * OPP_ACTIONS],
                },
                damage,
            );
            if p.kind == RequestKind::Preview {
                mask.fill(0);
            }
            self.action_mask(g, &mut out.actions[i * N_ALL_ACTIONS..(i + 1) * N_ALL_ACTIONS]);
        }
    }

    /// Game `g`'s legal actions in the one action space (71 entries, 1 = legal), without allocating.
    pub fn action_mask(&self, g: usize, out: &mut [u8]) {
        out.fill(0);
        let s = self.slot(g);
        match s.pending.kind {
            RequestKind::Preview => out[PREVIEW0..PREVIEW0 + N_PREVIEW_ACTIONS].fill(1),
            _ => each_turn_action(&s.opts[s.pending.seat as usize], |a| out[a as usize] = 1),
        }
    }
}
