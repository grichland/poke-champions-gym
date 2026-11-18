//! A live battle session (contracts/LIVE.md sections 2 and 3): the controller's messages in order, CG's
//! answers out. The Battle is built by the same `Builder` as in training; this adds the protocol: which
//! request is open, answers kept by `seq` (a retry or a restart gets the same answer), refusals, stale
//! requests, and an answer for every request even when the Battle allows nothing.
//!
//! The Player is outside: `handle` returns `Step::Ask` when one must choose, and `answer` /
//! `answer_picks` turn its choice into the reply. Nothing here panics or fails on input.
//!
//! Known answers are never asked for: picks given in `battle_start.picks`, and a replayed request's
//! `played` order (S8-1, S8-2).

use crate::builder::{Builder, Input};
use crate::event::*;
use crate::json::Json;
use crate::legal::{legal, order_for, Legal};
use crate::state::*;
use std::collections::HashMap;

/// What the open request asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskKind {
    Preview,
    Turn,
    Replace,
}

impl AskKind {
    pub fn req(self) -> ReqKind {
        if self == AskKind::Replace {
            ReqKind::Replace
        } else {
            ReqKind::Turn
        }
    }
}

/// The request waiting for an answer. `seq` is -1 for team preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Open {
    pub seq: i64,
    pub kind: AskKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// nothing to send back
    None,
    /// send this now
    Reply(Json),
    /// the Player must choose for the open request; then call `answer` or `answer_picks`
    Ask(Open),
}

/// A request answered without asking the Player (replayed, `played`, given picks, or repeated from the
/// session's record): the Battle as it was at that request, before the answer was applied, and what was
/// played. A Player with memory uses it to take the step it would have taken when asked.
#[derive(Clone, Debug, PartialEq)]
pub struct Answered {
    pub battle: Battle,
    pub seq: i64,
    pub kind: AskKind,
    /// the action 0..10 played (None: unknown, or not in the legal set)
    pub action: Option<usize>,
    /// at preview: the three brought, lead first (None: unknown)
    pub picks: Option<Vec<u8>>,
    /// "played", "repeated", "given" or "unknown"
    pub how: &'static str,
}

/// A protocol problem (the Builder's own anomalies are in `builder.anomalies`).
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    /// messages handled before it
    pub at: usize,
    pub what: String,
}

#[derive(Clone, Debug, Default)]
pub struct Session {
    pub builder: Builder,
    /// the battle tag of `battle_start`
    pub tag: Option<String>,
    pub open: Option<Open>,
    /// the last answer per `seq`, kept across a restart of the same battle
    answers: HashMap<i64, (Order, String)>,
    picks: Option<Vec<u8>>,
    /// the three brought, from `battle_start.picks`
    given: Option<Vec<u8>>,
    /// the last order sent (a retry with a new `seq` repeats it)
    last_order: Option<(Order, String)>,
    /// highest `seq` seen since the last `battle_start`
    high_seq: Option<i64>,
    pub notes: Vec<Note>,
    /// messages handled
    pub messages: usize,
    /// `battle_start` of the same battle again
    pub restarts: u32,
    pub ended: bool,
    /// the request the last message answered without asking the Player, if any
    pub answered: Option<Answered>,
}

/// A request sent again after a restart (`replay`), with the answer the game took when known (`played`).
#[derive(Clone, Debug, Default)]
struct Replay {
    replay: bool,
    played: Option<Order>,
}

fn error(msg: &str, what: impl Into<String>) -> Step {
    let mut o = Json::obj();
    o.set("error", what.into());
    if !msg.is_empty() {
        o.set("type", msg);
    }
    Step::Reply(o)
}

/// The legal actions (0..10) of a mask.
pub fn actions(l: &Legal) -> Vec<usize> {
    (0..N_ACTIONS).filter(|&a| l.mask[a]).collect()
}

impl Session {
    pub fn new() -> Session {
        Session::default()
    }

    fn note(&mut self, what: impl Into<String>) {
        self.notes.push(Note { at: self.messages, what: what.into() });
    }

    pub fn battle(&self) -> &Battle {
        &self.builder.battle
    }

    /// The legal set of the open turn or replace request (empty at preview or with none open).
    pub fn legal(&self) -> Option<Legal> {
        match self.open {
            Some(o) if o.kind != AskKind::Preview => Some(legal(&self.builder.battle, o.kind.req())),
            _ => None,
        }
    }

    /// One message, as JSON text.
    pub fn handle_text(&mut self, text: &str) -> Step {
        match Json::parse(text) {
            Ok(j) => self.handle(&j),
            Err(e) => {
                self.messages += 1;
                self.note(format!("bad JSON: {}", e));
                error("", format!("bad JSON: {}", e))
            }
        }
    }

    pub fn handle(&mut self, j: &Json) -> Step {
        self.answered = None;
        let step = self.handle_inner(j);
        self.messages += 1;
        step
    }

    fn handle_inner(&mut self, j: &Json) -> Step {
        let Some(ty) = j.get("type").and_then(|t| t.as_str()).map(str::to_string) else {
            self.note("message with no type");
            return error("", "message with no 'type'");
        };
        let tag = j.get("battle").and_then(|t| t.as_str()).map(str::to_string);
        if ty == "battle_start" {
            return self.start(j, tag);
        }
        if tag.is_some() && tag != self.tag {
            self.note(format!("{} for battle {:?} while the session holds {:?}", ty, tag, self.tag));
            return error(&ty, format!("message for battle {:?}; this session holds {:?} (send battle_start first)", tag.unwrap_or_default(), self.tag));
        }
        if self.tag.is_none() {
            self.note(format!("{} before battle_start", ty));
            return error(&ty, "no battle_start yet");
        }
        let msg = match Message::from_json(j) {
            Ok(m) => m,
            Err(e) if ty == "events" => return self.events_lenient(j, &e),
            Err(e) => {
                self.note(format!("{}: {}", ty, e));
                return error(&ty, e);
            }
        };
        match &msg.body {
            Body::PreviewRequest { .. } => self.preview(&msg, j),
            Body::Events { events, .. } => {
                if events.iter().any(|e| matches!(e, Event::End { .. })) {
                    self.ended = true;
                }
                self.builder.apply(&Input::Msg(msg.clone()));
                Step::None
            }
            Body::BattleEnd { .. } => {
                self.builder.apply(&Input::Msg(msg.clone()));
                self.ended = true;
                self.open = None;
                Step::None
            }
            Body::DecisionRequest { seq, kind, refused, retry, screen_offer, .. } => {
                let (replay, played) = Session::replay_of(j);
                let played = match played.map(|p| Order::from_json(&p)) {
                    Some(Ok(o)) => Some(o),
                    Some(Err(e)) => {
                        self.note(format!("seq {}: played: {}", seq, e));
                        None
                    }
                    None => None,
                };
                let r = Replay { replay, played };
                self.request(&msg, *seq, *kind, refused.is_some(), *retry, screen_offer.as_ref(), r)
            }
            Body::BattleStart { .. } => Step::None,
        }
    }

    fn start(&mut self, j: &Json, tag: Option<String>) -> Step {
        let msg = match Message::from_json(j) {
            Ok(m) => m,
            Err(e) => {
                self.note(format!("battle_start: {}", e));
                return error("battle_start", e);
            }
        };
        let given = match Picks::field(j, "picks") {
            Ok(p) => p.map(|p| p.picks),
            Err(e) => {
                self.note(format!("battle_start picks: {}", e));
                None
            }
        };
        if tag.is_some() && tag == self.tag {
            // a restart: the controller sends the whole battle again; answers already given stay
            self.restarts += 1;
        } else {
            if self.tag.is_some() && !self.ended {
                self.note(format!("battle {:?} replaced before its end", self.tag));
            }
            self.answers.clear();
            self.picks = None;
            self.given = None;
            self.last_order = None;
            self.restarts = 0;
        }
        self.tag = tag;
        self.builder = Builder::new();
        self.open = None;
        self.high_seq = None;
        self.ended = false;
        self.builder.apply(&Input::Msg(msg));
        self.given = None;
        if let Some(p) = given {
            let n = self.builder.battle.n_own;
            if p.len() == 3 && p.iter().all(|&x| x < n) && p[0] != p[1] && p[0] != p[2] && p[1] != p[2] {
                self.builder.apply(&Input::Picks(Picks { picks: p.clone() }));
                self.given = Some(p);
            } else {
                self.note(format!("battle_start picks {:?}: not three team indices; ignored", p));
            }
        }
        Step::None
    }

    /// `replay` and `played` (the answer the game took) of a request sent again after a restart.
    fn replay_of(j: &Json) -> (bool, Option<Json>) {
        let played = j.get("played").filter(|x| !x.is_null()).cloned();
        let replay = played.is_some() || j.get("replay").and_then(|x| x.as_bool()).unwrap_or(false);
        (replay, played)
    }

    fn preview(&mut self, msg: &Message, j: &Json) -> Step {
        self.builder.apply(&Input::Msg(msg.clone()));
        self.open = None;
        let (replay, played) = Session::replay_of(j);
        let played = match played.as_ref().map(|p| Picks::field(p, "picks")) {
            Some(Ok(Some(p))) if p.picks.len() == 3 => Some(p.picks),
            Some(_) => {
                self.note("preview_request played: not three picks; ignored");
                None
            }
            None => None,
        };
        let mut notes = Json::obj();
        let played_known = self.given.is_none() && played.is_some();
        // what is known of the three brought: given at battle_start, played, or our earlier answer
        let known = if let Some(p) = self.given.clone() {
            notes.set("given", true);
            Some(p)
        } else if let Some(p) = played {
            self.builder.apply(&Input::Picks(Picks { picks: p.clone() }));
            Some(p)
        } else if let Some(p) = self.picks.clone() {
            self.builder.apply(&Input::Picks(Picks { picks: p.clone() }));
            Some(p)
        } else {
            None
        };
        if replay {
            notes.set("replayed", true);
        }
        // the preview Battle as it was asked: before any picks
        let mut at = self.builder.battle;
        at.picks = [None; 3];
        let how = if self.given.is_some() { "given" } else if played_known { "played" } else if known.is_some() { "repeated" } else { "unknown" };
        if known.is_some() || replay {
            self.answered = Some(Answered { battle: at, seq: -1, kind: AskKind::Preview, action: None, picks: known.clone(), how });
        }
        match known {
            Some(p) => {
                let mut o = Picks { picks: p }.to_json();
                if notes.as_obj().map_or(false, |x| !x.is_empty()) {
                    o.set("notes", notes);
                }
                Step::Reply(o)
            }
            // replayed with the picks unknown: no Player is asked; they come from the entries (rule 32)
            None if replay => {
                let mut o = Json::obj();
                o.set("replayed", true);
                Step::Reply(o)
            }
            None => {
                self.open = Some(Open { seq: -1, kind: AskKind::Preview });
                Step::Ask(Open { seq: -1, kind: AskKind::Preview })
            }
        }
    }

    /// An `events` message with a bad event: the good ones are applied, the reply names the others.
    fn events_lenient(&mut self, j: &Json, e: &str) -> Step {
        let evs = j.get("events").and_then(|x| x.as_arr()).cloned();
        let Some(evs) = evs else {
            self.note(format!("events: {}", e));
            return error("events", e);
        };
        let mut good = vec![];
        let mut bad = vec![];
        for (i, ev) in evs.iter().enumerate() {
            match Event::from_json(ev) {
                Ok(x) => good.push(x),
                Err(e2) => {
                    self.note(format!("event {}: {}", i, e2));
                    bad.push((i, e2));
                }
            }
        }
        // the fallback, if it reads
        let fallback = match Fallback::list(j.get("fallback").filter(|f| !f.is_null())) {
            Ok(x) => x,
            Err(e3) => {
                bad.push((usize::MAX, format!("fallback: {}", e3)));
                None
            }
        };
        if good.iter().any(|e| matches!(e, Event::End { .. })) {
            self.ended = true;
        }
        let v = j.get("v").and_then(|x| x.as_str()).unwrap_or(CONTRACT_VERSION).to_string();
        let m = Message { v, battle: self.tag.clone().unwrap_or_default(), body: Body::Events { events: good, fallback } };
        self.builder.apply(&Input::Msg(m));
        let mut o = Json::obj();
        o.set("error", if bad.is_empty() { e.to_string() } else { format!("{} event(s) could not be read; the others were applied", bad.len()) });
        o.set("type", "events");
        o.set(
            "dropped",
            bad.iter()
                .map(|(i, w)| {
                    let mut x = Json::obj();
                    if *i != usize::MAX {
                        x.set("index", *i as i64);
                    }
                    x.set("why", w.as_str());
                    x
                })
                .collect::<Vec<_>>(),
        );
        Step::Reply(o)
    }

    #[allow(clippy::too_many_arguments)]
    fn request(&mut self, msg: &Message, seq: i64, kind: ReqKind, refused: bool, retry: bool, offer: Option<&Json>, r: Replay) -> Step {
        if let Some(h) = self.high_seq {
            if seq < h {
                // an older request: the controller ignores answers to it; nothing is applied
                self.note(format!("stale seq {} after {}", seq, h));
                let mut o = Json::obj();
                o.set("seq", seq);
                o.set("error", format!("stale seq {} (latest {})", seq, h));
                if let Some((a, _)) = self.answers.get(&seq) {
                    o.set("order", a.to_json());
                }
                return Step::Reply(o);
            }
        }
        let same = self.open.map_or(false, |o| o.seq == seq);
        self.high_seq = Some(seq);
        let ask = if kind == ReqKind::Replace { AskKind::Replace } else { AskKind::Turn };
        self.builder.apply(&Input::Msg(msg.clone()));
        self.open = Some(Open { seq, kind: ask });
        if let Some(so) = offer {
            self.check_offer(so);
        }
        if r.replay {
            // a request sent again after a restart: the Player is not asked. What the game took, else
            // what this session answered before, else unknown.
            let held = self.answers.get(&seq).cloned();
            let known = match (r.played, held) {
                (Some(o), held) => {
                    if let Some((a, _)) = held.filter(|(a, _)| *a != o) {
                        self.note(format!("seq {}: played {:?}, CG had answered {:?}", seq, o, a));
                    }
                    Some((o, "played".to_string(), false))
                }
                (None, Some((o, player))) => Some((o, player, true)),
                (None, None) => None,
            };
            if let Some((o, player, repeated)) = known {
                let l = legal(&self.builder.battle, kind);
                let at = (0..N_ACTIONS).find(|&a| order_for(&self.builder.battle, &l, a).as_ref() == Some(&o));
                let how = if repeated { "repeated" } else { "played" };
                self.answered = Some(Answered { battle: self.builder.battle, seq, kind: ask, action: at, picks: None, how });
                if at.is_none() {
                    self.note(format!("seq {}: replayed {:?} is outside the legal set", seq, o));
                }
                let mut extra = vec![("replayed", Json::from(true))];
                if repeated {
                    extra.push(("repeated", Json::from(true)));
                }
                let reply = self.send(seq, o, at, &l, &player, extra);
                self.open = None;
                return Step::Reply(reply);
            }
            // the answer is unknown: like a fallback without an order (rule 31)
            self.answered = Some(Answered { battle: self.builder.battle, seq, kind: ask, action: None, picks: None, how: "unknown" });
            let f = Fallback { seq: Some(seq), order: None, reason: Some("replayed".into()), picks: None };
            let m = Message { v: CONTRACT_VERSION.into(), battle: msg.battle.clone(), body: Body::Events { events: vec![], fallback: Some(vec![f]) } };
            self.builder.apply(&Input::Msg(m));
            self.last_order = None;
            self.open = None;
            let mut o = Json::obj();
            o.set("seq", seq);
            o.set("replayed", true);
            return Step::Reply(o);
        }
        if !refused {
            // the same request again (a repeated send, a retry, or the replay after a restart)
            let again = self.answers.get(&seq).cloned().or_else(|| if retry || same { self.last_order.clone() } else { None });
            if let Some((o, player)) = again {
                let l = legal(&self.builder.battle, kind);
                let at = (0..N_ACTIONS).find(|&a| order_for(&self.builder.battle, &l, a).as_ref() == Some(&o));
                if at.is_some() {
                    self.answered = Some(Answered { battle: self.builder.battle, seq, kind: ask, action: at, picks: None, how: "repeated" });
                    return Step::Reply(self.send(seq, o, at, &l, &player, vec![("repeated", Json::from(true))]));
                }
                self.note(format!("seq {}: the earlier order {:?} is outside the legal set: asked again", seq, o));
            }
        }
        Step::Ask(Open { seq, kind: ask })
    }

    /// Logs where the screen's own list disagrees with the legal set (it is never used to choose).
    fn check_offer(&mut self, so: &Json) {
        let Some(o) = self.open else { return };
        let b = &self.builder.battle;
        let l = legal(b, o.kind.req());
        let legal_moves: Vec<String> = (0..4).filter_map(|a| order_for(b, &l, a)).filter_map(|x| if let Order::Move { mv, .. } = x { Some(mv) } else { None }).collect();
        let legal_mons: Vec<String> = (8..N_ACTIONS).filter_map(|a| order_for(b, &l, a)).filter_map(|x| if let Order::Switch { mon } = x { Some(mon) } else { None }).collect();
        let mut out = vec![];
        if let Some(ms) = so.get("moves").and_then(|x| x.as_arr()) {
            for m in ms.iter().filter_map(|x| x.as_str()) {
                if !legal_moves.iter().any(|x| x == m) {
                    out.push(format!("screen_offer move {} not in the legal set", m));
                }
            }
        }
        for k in ["switches", "mons"] {
            if let Some(ms) = so.get(k).and_then(|x| x.as_arr()) {
                for m in ms.iter().filter_map(|x| x.as_str()) {
                    if !legal_mons.iter().any(|x| x == m) {
                        out.push(format!("screen_offer switch {} not in the legal set", m));
                    }
                }
            }
        }
        for w in out {
            self.note(w);
        }
    }

    fn send(&mut self, seq: i64, order: Order, action: Option<usize>, l: &Legal, player: &str, extra: Vec<(&str, Json)>) -> Json {
        self.builder.apply(&Input::Order(order.clone()));
        self.answers.insert(seq, (order.clone(), player.to_string()));
        self.last_order = Some((order.clone(), player.to_string()));
        let mut notes = Json::obj();
        match action {
            Some(a) => notes.set("action", a as i64),
            None => notes.set("action", Json::Null),
        }
        notes.set("legal", actions(l).into_iter().map(|a| Json::from(a as i64)).collect::<Vec<_>>());
        notes.set("player", player);
        notes.set("kind", self.open.map_or("turn", |o| o.kind.req().id()));
        notes.set("turn", self.builder.battle.turn as i64);
        for (k, v) in extra {
            notes.set(k, v);
        }
        Answer { seq, order, notes: Some(notes) }.to_json()
    }

    /// The reply for the open turn or replace request with the Player's action (0..10). An action outside
    /// the legal set is replaced by the first legal one; with none legal, a last-resort order is sent.
    pub fn answer(&mut self, action: i64, player: &str) -> Json {
        let Some(open) = self.open.filter(|o| o.kind != AskKind::Preview) else {
            self.note("answer with no open request");
            let mut o = Json::obj();
            o.set("error", "no open decision request");
            return o;
        };
        let b = &self.builder.battle;
        let l = legal(b, open.kind.req());
        let ok = action >= 0 && (action as usize) < N_ACTIONS && l.mask[action as usize];
        let a = if ok {
            Some(action as usize)
        } else {
            let first = (0..N_ACTIONS).find(|&a| l.mask[a]);
            if first.is_some() {
                self.note(format!("seq {}: {} chose {} outside the legal set", open.seq, player, action));
            }
            first
        };
        match a.and_then(|a| order_for(&self.builder.battle, &l, a).map(|o| (a, o))) {
            Some((a, o)) => {
                let extra = if ok { vec![] } else { vec![("replaced", Json::from(format!("{} chose {}", player, action)))] };
                self.send(open.seq, o, Some(a), &l, player, extra)
            }
            None => {
                let (o, why) = self.last_resort(open.kind);
                self.note(format!("seq {}: no legal action ({})", open.seq, why));
                self.send(open.seq, o, None, &l, player, vec![("last_resort", Json::from(true)), ("why", Json::from(why))])
            }
        }
    }

    /// An order when the Battle allows nothing: a switch to a picked Pokémon not known to have fainted,
    /// else the active Pokémon's first move, else Struggle.
    fn last_resort(&self, kind: AskKind) -> (Order, String) {
        let b = &self.builder.battle;
        let own = b.side(Side::Own);
        let active = own.active.slot.filter(|_| !own.active.out);
        let mon = |p: u8| own.team[p as usize].species.map(|s| Order::Switch { mon: s.id().to_string() });
        let alive: Vec<u8> = b.picks.iter().flatten().copied().filter(|&p| !own.team[p as usize].fainted).collect();
        if kind == AskKind::Replace || active.is_none() {
            if let Some(o) = alive.iter().copied().filter(|&p| Some(p) != own.active.slot).find_map(mon) {
                return (o, "switch to a Pokémon the Battle shows refused".into());
            }
            if let Some(o) = alive.iter().copied().find_map(mon) {
                return (o, "switch to the Pokémon the Battle holds as active".into());
            }
            if let Some(o) = b.picks.iter().flatten().copied().find_map(mon) {
                return (o, "switch to a Pokémon the Battle holds as fainted".into());
            }
        }
        if let Some(w) = b.active(Side::Own) {
            if let Some(m) = b.mon(w).moves.iter().flatten().next() {
                return (Order::Move { mv: m.id().to_string(), mega: false }, "the active Pokémon's first move".into());
            }
        }
        (Order::Move { mv: "struggle".into(), mega: false }, "nothing known: struggle".into())
    }

    /// The reply to the open team preview with three indices into `own_team`, lead first.
    pub fn answer_picks(&mut self, picks: &[i64], player: &str) -> Json {
        if self.open.map(|o| o.kind) != Some(AskKind::Preview) {
            self.note("picks with no open preview request");
        }
        let n = self.builder.battle.n_own.max(1) as i64;
        let mut p: Vec<u8> = picks.iter().filter(|&&x| (0..n).contains(&x)).map(|&x| x as u8).collect();
        let distinct = p.len() == 3 && p[0] != p[1] && p[0] != p[2] && p[1] != p[2];
        if !distinct || picks.len() != 3 {
            self.note(format!("{} gave picks {:?}: replaced by the first three", player, picks));
            p = (0..n.min(3) as u8).collect();
        }
        self.builder.apply(&Input::Picks(Picks { picks: p.clone() }));
        self.picks = Some(p.clone());
        self.open = None;
        Picks { picks: p }.to_json()
    }
}
