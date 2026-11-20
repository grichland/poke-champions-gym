//! What each seat of a Game knows, kept as the Game is stepped: both seats' Builders fed with the rows
//! each would see, in the delivery order of the environment the Game came from (`replace_before_rows`),
//! with the requests the environment would send. A seat's tensors are then the ones `BatchEnv` shows it.
//!
//! The environment asks seat 0 first, then seat 1, and steps when both are in; a Game takes both actions at
//! once. A seat's Builder only ever sees its own rows, requests and orders, so asking both seats when a
//! step resolves gives each the Battle the environment gives it when it is asked. Decision ids follow the
//! environment's: seat 0's request first, then seat 1's.
//!
//! `Tracked` carries the views lazily: a Game step only records its choices and outcome source; the rows
//! are emitted when a view is asked for, by replaying the recorded steps from the last built point.

use crate::batch::{battle_options, each_turn_action, own_order, RequestKind, Rule, Slot};
use crate::enumerate::Replay;
use crate::labels::Took;
use crate::observe::{ObsBatch, KIND_REPLACE, KIND_TURN, ROWS_LEN, ROW_IDS_LEN};
use crate::N_ALL_ACTIONS;
use battle::{Battle, ReqKind};
use cg_engine::sim::outcome::{Outcomes, Seeded};
use cg_engine::sim::state::{State, SIDES};
use cg_engine::sim::{Choice, Kind, Row, SideOptions};
use emit::Seats;
use obs::{ObsOut, FIELD, LAST, LAST_IDS, MASK, OPP_ACTIONS};
use std::sync::Arc;

/// `kind` of a seat with no open request (not deciding, locked, already answered, or the game ended).
pub const KIND_NONE: u8 = 3;

#[derive(Clone, Debug)]
pub struct Views {
    pub seats: Seats,
    pub rule: Rule,
    /// a seat locked into a move is not asked (the environment's `auto_locked`)
    pub auto_locked: bool,
    /// per seat, the id of its open request (None: no request open)
    pub open: [Option<u64>; SIDES],
    /// per seat, what its open request allows (the rule's options)
    pub opts: [SideOptions; SIDES],
    /// per seat, the kind of its open request
    pub kind: [Kind; SIDES],
    /// per seat, an order already given for the step under way (a fork between the two seats' answers)
    pub answered: [bool; SIDES],
    /// the last decision id given
    pub decision: u64,
    /// per seat, whether its Battle is known (a Game built from a live Battle knows ours only)
    pub known: [bool; SIDES],
}

impl Views {
    /// The views of a game of the environment, as its pending seat sees it now. The other seat: if it
    /// answered already, its Battle holds its order (it shows kind none until the step); if the
    /// environment asks it after the pending seat, it is asked now with the next decision id.
    pub fn of_slot(s: &Slot, rule: Rule, auto_locked: bool) -> Views {
        let p = s.pending.seat as usize;
        let mut v = Views {
            seats: s.seats.view_copy(),
            rule,
            auto_locked,
            open: [None; SIDES],
            opts: [SideOptions::default(); SIDES],
            kind: [Kind::Wait; SIDES],
            answered: [false; SIDES],
            decision: s.decision,
            known: [true; SIDES],
        };
        v.open[p] = Some(s.decision);
        v.opts[p] = s.opts[p];
        v.kind[p] = if s.pending.kind == RequestKind::Replace { Kind::Replace } else { Kind::Turn };
        let q = 1 - p;
        if q < p {
            v.answered[q] = matches!(s.took[q], Took::Asked(_));
        } else if deciding(&s.state_opts[q]) && s.chosen[q].is_none() {
            v.ask(&s.game.state, q, &s.state_opts);
        }
        v
    }

    /// The views of a Game built from our live Battle: `seats` holds our Builder at its open request
    /// (`seq`, `kind`); the other seat's Battle is not known.
    pub fn live(seats: Seats, seat: usize, kind: ReqKind, seq: i64) -> Views {
        let mut v = Views {
            seats,
            rule: Rule::Battle,
            auto_locked: true,
            open: [None; SIDES],
            opts: [SideOptions::default(); SIDES],
            kind: [Kind::Wait; SIDES],
            answered: [false; SIDES],
            decision: seq.max(0) as u64,
            known: [false; SIDES],
        };
        v.known[seat] = true;
        v.open[seat] = Some(v.decision);
        v.kind[seat] = if kind == ReqKind::Replace { Kind::Replace } else { Kind::Turn };
        v
    }

    fn ask(&mut self, st: &State, seat: usize, state_opts: &[SideOptions; SIDES]) {
        let so = state_opts[seat];
        let rk = if so.kind == Kind::Replace { ReqKind::Replace } else { ReqKind::Turn };
        let offer = if rk == ReqKind::Turn { emit::screen_offer(st, seat) } else { None };
        self.seats.request(seat, rk, None, offer);
        let o = match self.rule {
            Rule::Battle => battle_options(&self.seats.builders[seat].battle, so.kind),
            Rule::State => so,
        };
        let mut any = false;
        each_turn_action(&o, |_| any = true);
        self.opts[seat] = if any { o } else { so };
        self.kind[seat] = so.kind;
        self.decision += 1;
        self.open[seat] = Some(self.decision);
    }

    /// The requests after a step, as the environment sends them.
    fn advance(&mut self, st: &State) {
        self.open = [None; SIDES];
        self.answered = [false; SIDES];
        self.kind = [Kind::Wait; SIDES];
        if st.ended {
            return;
        }
        let so = st.options();
        for side in 0..SIDES {
            if !deciding(&so[side]) || (self.auto_locked && so[side].kind == Kind::Turn && st.locked_move(side).is_some()) {
                continue;
            }
            self.ask(st, side, &so);
        }
    }

    /// One step with these choices from `o`: each asked seat's order, the rows, the next requests.
    pub fn step<O: Outcomes>(&mut self, st: &mut State, o: &mut O, ch: [Option<Choice>; SIDES], rows: &mut Vec<Row>) -> Result<(), String> {
        for seat in 0..SIDES {
            if let Some(c) = ch[seat] {
                if self.open[seat].is_some() || self.answered[seat] {
                    self.seats.order(seat, own_order(st, seat, c));
                }
            }
        }
        self.seats.step_into(st, o, ch, rows)?;
        self.advance(st);
        Ok(())
    }

    pub fn battle(&self, seat: usize) -> &Battle {
        &self.seats.builders[seat].battle
    }

    /// The open request of a seat: (decision id, kind code 1 turn or 2 replace).
    pub fn request(&self, seat: usize) -> Option<(u64, u8)> {
        self.open[seat].map(|d| (d, if self.kind[seat] == Kind::Replace { KIND_REPLACE } else { KIND_TURN }))
    }

    /// The seat's legal actions in the one action space (71 entries, 1 = legal); all 0 with no request.
    pub fn action_mask(&self, seat: usize, out: &mut [u8]) {
        out.fill(0);
        if self.open[seat].is_some() {
            each_turn_action(&self.opts[seat], |a| out[a as usize] = 1);
        }
    }

    /// Entry `i` of `out`: the seat's tensors as `BatchEnv::observe_batch` writes them for an asked seat;
    /// a seat with no open request is observed as for a turn, with `mask` and `actions` 0, kind
    /// `KIND_NONE` and decision -1.
    pub fn observe(&self, seat: usize, out: &mut ObsBatch<'_>, i: usize, damage: bool) {
        let req = self.request(seat);
        let rk = if req.map(|r| r.1) == Some(KIND_REPLACE) { ReqKind::Replace } else { ReqKind::Turn };
        let mask = &mut out.mask[i * MASK..(i + 1) * MASK];
        obs::observe_into(
            self.battle(seat),
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
        out.seat[i] = seat as u8;
        match req {
            Some((d, k)) => {
                out.kind[i] = k;
                out.decision[i] = d as i64;
            }
            None => {
                mask.fill(0);
                out.kind[i] = KIND_NONE;
                out.decision[i] = -1;
            }
        }
        self.action_mask(seat, &mut out.actions[i * N_ALL_ACTIONS..(i + 1) * N_ALL_ACTIONS]);
    }
}

#[inline]
fn deciding(o: &SideOptions) -> bool {
    matches!(o.kind, Kind::Turn | Kind::Replace)
}

/// How a recorded step drew its random events.
#[derive(Clone, Debug)]
pub enum Source {
    /// the Game's seeded source as it was before the step
    Seeded(Seeded),
    /// an enumerated outcome's path
    Path(Arc<Vec<u16>>),
}

#[derive(Clone, Debug)]
struct Logged {
    ch: [Option<Choice>; SIDES],
    src: Source,
}

/// Views carried lazily: the last built point (State and Views) and the steps taken since.
#[derive(Clone, Debug)]
pub struct Tracked {
    base: Arc<(State, Views)>,
    steps: Vec<Logged>,
}

impl Tracked {
    pub fn new(state: State, views: Views) -> Tracked {
        Tracked { base: Arc::new((state, views)), steps: Vec::new() }
    }

    /// Records a step drawn from `src`.
    pub fn record(&mut self, ch: [Option<Choice>; SIDES], src: Source) {
        self.steps.push(Logged { ch, src });
    }

    /// Steps recorded since the last built point.
    pub fn pending_steps(&self) -> usize {
        self.steps.len()
    }

    /// The views at `now` (the Game's State): the recorded steps are replayed from the last built point,
    /// which then moves here. Errors when the replay does not reach `now`.
    pub fn views(&mut self, now: &State) -> Result<&Views, String> {
        if !self.steps.is_empty() {
            let (mut st, mut v) = (*self.base).clone();
            let mut rows = Vec::with_capacity(64);
            for l in &self.steps {
                match &l.src {
                    Source::Seeded(r) => {
                        let mut r = r.clone();
                        v.step(&mut st, &mut r, l.ch, &mut rows)?;
                    }
                    Source::Path(p) => {
                        let mut r = Replay::new(p.to_vec(), false);
                        v.step(&mut st, &mut r, l.ch, &mut rows)?;
                        if let Some(e) = r.bad {
                            return Err(e);
                        }
                    }
                }
            }
            if st != *now {
                return Err("views: the replayed steps do not reach the game's State".into());
            }
            self.base = Arc::new((st, v));
            self.steps.clear();
        }
        Ok(&self.base.1)
    }

    /// The views if no step is waiting to be replayed.
    pub fn built(&self) -> Option<&Views> {
        self.steps.is_empty().then(|| &self.base.1)
    }
}
