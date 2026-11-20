//! Emission: what the resolver did, as the contract events (contracts/LIVE.md) each seat's battle log
//! shows, for CG's Battle builder. A step is played once with the resolver's row record (`step_logged`);
//! `view` turns its rows into each seat's events; `check` compares a seat's Battle with the State.

pub mod check;
pub mod fmt;
pub mod view;

use battle::{Body, Builder, Event, Input, Message, Picks, ReqKind, CONTRACT_VERSION};
use cg_engine::sim::outcome::{Outcomes, Seeded};
use cg_engine::sim::state::{State, Winner, SIDES};
use cg_engine::sim::{Choice, MonSpec, Row};
pub use view::{Emitter, Names};

/// Both seats' Builders of one game, fed each step's events each from its own side, with the full record
/// kept when asked (for the rebuild check and for saving).
#[derive(Clone, Debug, Default)]
pub struct Seats {
    pub builders: [Builder; SIDES],
    pub records: [Vec<Input>; SIDES],
    pub keep: bool,
    pub tag: String,
    pub seq: i64,
    pub emitter: Emitter,
    /// the events of the last step, per seat
    pub last: [Vec<Event>; SIDES],
    /// time sections when profiling: nanoseconds in the engine, the emitter and the Builders
    pub profile: bool,
    pub prof: [u64; 3],
    /// the live order (LIVE.md section 2, rule 18): a seat's log rows reach its Builder only at its next
    /// `turn` request or at the end of the game, so a `replace` request comes before them
    pub replace_before_rows: bool,
    /// per seat, the rows emitted and not yet delivered (`replace_before_rows`)
    pub held: [Vec<Input>; SIDES],
    /// per seat, events in `held`
    pub held_events: [usize; SIDES],
}

fn msg(tag: &str, body: Body) -> Message {
    Message { v: CONTRACT_VERSION.into(), battle: tag.into(), body }
}

impl Seats {
    pub fn new(tag: String, keep: bool) -> Seats {
        Seats { tag, keep, ..Seats::default() }
    }

    /// Empty for a new game `g-e{episode}`, keeping what it had allocated.
    pub fn reset(&mut self, g: usize, episode: u64, keep: bool) {
        use std::fmt::Write;
        self.builders = Default::default();
        for r in self.records.iter_mut() {
            r.clear();
        }
        self.keep = keep;
        self.tag.clear();
        let _ = write!(self.tag, "g{}-e{}", g, episode);
        self.seq = 0;
        self.emitter = Emitter::default();
        for l in self.last.iter_mut() {
            l.clear();
        }
        for h in self.held.iter_mut() {
            h.clear();
        }
        self.held_events = [0; SIDES];
    }

    /// A copy without the records (for a replay that compares Battles only).
    pub fn view_copy(&self) -> Seats {
        Seats {
            builders: self.builders.clone(),
            records: Default::default(),
            keep: false,
            tag: self.tag.clone(),
            seq: self.seq,
            emitter: self.emitter.clone(),
            last: Default::default(),
            profile: false,
            prof: [0; 3],
            replace_before_rows: self.replace_before_rows,
            held: self.held.clone(),
            held_events: self.held_events,
        }
    }

    pub fn input(&mut self, seat: usize, i: Input) {
        self.builders[seat].apply(&i);
        if self.keep {
            self.records[seat].push(i);
        }
    }

    fn message(&mut self, seat: usize, body: Body) {
        let m = msg(&self.tag, body);
        self.input(seat, Input::Msg(m));
    }

    /// `battle_start` and `preview_request` for both seats (rosters in team-select order).
    pub fn start(&mut self, rosters: [&[MonSpec]; SIDES]) {
        for seat in 0..SIDES {
            self.message(seat, view::battle_start(rosters[seat], rosters[1 - seat]));
            self.message(seat, Body::PreviewRequest { deadline_ms: None });
        }
    }

    pub fn picks(&mut self, seat: usize, picks: [u8; 3]) {
        self.input(seat, Input::Picks(Picks { picks: picks.to_vec() }));
    }

    /// The rows of the leads' entry (`State::start_logged`).
    pub fn leads(&mut self, rows: &[Row], s: &State) {
        self.emitter = Emitter::new(s);
        let n = Names::of(s);
        let mut s0 = *s;
        for side in 0..SIDES {
            s0.sides[side].has_active = false;
        }
        let ev = self.emitter.events(rows, &s0, s, &n, true);
        self.send(ev);
    }

    /// One step played for real with its rows recorded, then emitted to both seats. Returns the rows.
    pub fn step(&mut self, state: &mut State, rng: &mut Seeded, choices: [Option<Choice>; SIDES]) -> Result<Vec<Row>, String> {
        let mut rows = Vec::with_capacity(32);
        self.step_into(state, rng, choices, &mut rows)?;
        Ok(rows)
    }

    /// `step`, writing the rows into `rows` (cleared first), with any outcome source.
    pub fn step_into<O: Outcomes>(&mut self, state: &mut State, rng: &mut O, choices: [Option<Choice>; SIDES], rows: &mut Vec<Row>) -> Result<(), String> {
        let t0 = self.profile.then(std::time::Instant::now);
        let s0 = *state;
        rows.clear();
        state.step_logged(choices, rng, rows)?;
        let t1 = self.profile.then(std::time::Instant::now);
        let n = Names::of(state);
        let ev = self.emitter.events(rows, &s0, state, &n, false);
        let t2 = self.profile.then(std::time::Instant::now);
        self.send(ev);
        if let (Some(t0), Some(t1), Some(t2)) = (t0, t1, t2) {
            self.prof[0] += (t1 - t0).as_nanos() as u64;
            self.prof[1] += (t2 - t1).as_nanos() as u64;
            self.prof[2] += t2.elapsed().as_nanos() as u64;
        }
        if state.ended {
            // the final rows are applied before the result (live, the phone usually cannot read them)
            self.flush_all();
            for seat in 0..SIDES {
                let r = match state.winner {
                    Winner::Side(w) if w as usize == seat => "win",
                    Winner::Side(_) => "loss",
                    _ => "tie",
                };
                self.message(seat, Body::BattleEnd { result: r.into() });
            }
        }
        Ok(())
    }

    fn send(&mut self, ev: [Vec<Event>; SIDES]) {
        for (seat, e) in ev.into_iter().enumerate() {
            if e.is_empty() {
                self.last[seat] = e;
                continue;
            }
            let i = Input::Msg(msg(&self.tag, Body::Events { events: e, fallback: None }));
            if self.replace_before_rows {
                if let Input::Msg(Message { body: Body::Events { events, .. }, .. }) = &i {
                    self.last[seat] = events.clone();
                    self.held_events[seat] += events.len();
                }
                self.held[seat].push(i);
                continue;
            }
            self.builders[seat].apply(&i);
            if self.keep {
                self.records[seat].push(i.clone());
            }
            if let Input::Msg(Message { body: Body::Events { events, .. }, .. }) = i {
                self.last[seat] = events;
            }
        }
    }

    /// Delivers a seat's held rows, in order.
    pub fn flush(&mut self, seat: usize) {
        let held = std::mem::take(&mut self.held[seat]);
        for i in &held {
            self.builders[seat].apply(i);
        }
        if self.keep {
            self.records[seat].extend(held);
        } else {
            let mut held = held;
            held.clear();
            self.held[seat] = held;
        }
        self.held_events[seat] = 0;
    }

    pub fn flush_all(&mut self) {
        for seat in 0..SIDES {
            self.flush(seat);
        }
    }

    /// A seat is asked for a decision. `screen_offer`: the move rows the action screen shows, when the
    /// Battle cannot know them (our own transformed Pokémon's copied moves). The log is read on a `turn`
    /// action screen only: there the held rows are delivered first.
    pub fn request(&mut self, seat: usize, kind: ReqKind, refused: Option<battle::Order>, screen_offer: Option<battle::json::Json>) {
        if kind == ReqKind::Turn && !self.held[seat].is_empty() {
            self.flush(seat);
        }
        self.seq += 1;
        let seq = self.seq;
        self.message(seat, Body::DecisionRequest { seq, kind, deadline_ms: None, refused, retry: false, screen_offer });
    }

    /// The order the seat gave for its open request (CG's own answer, recorded like the picks).
    pub fn order(&mut self, seat: usize, order: battle::Order) {
        self.input(seat, Input::Order(order));
    }

    /// The State refused an order the Battle allowed: the game's `refused` event.
    pub fn refused(&mut self, seat: usize, order: battle::Order) {
        self.message(seat, Body::Events { events: vec![Event::Refused { order }], fallback: None });
    }
}

/// The move rows our transformed Pokémon's action screen shows (its copied moves) and the ability it
/// copied (extension: the summary screen), as `screen_offer`.
pub fn screen_offer(s: &State, side: usize) -> Option<battle::json::Json> {
    let sd = &s.sides[side];
    if !sd.has_active || !sd.active_mon().transformed {
        return None;
    }
    let m = sd.active_mon();
    let mut o = battle::json::Json::obj();
    o.set("moves", m.moves[..m.nmoves as usize].iter().map(|x| battle::json::Json::from(x.id.id())).collect::<Vec<_>>());
    o.set("ability", m.ability.id());
    Some(o)
}
