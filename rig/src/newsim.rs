//! The new resolver (`cg_engine::sim`) behind the contract: `new` and `step`, with and without `force`,
//! exporting the contract's snapshot, options and events.

use crate::json::Json;
use cg_engine::sim::data::{self as d, to_id, AbilityId, ItemId, MoveId, SpeciesId, BOOST_IDS};
use cg_engine::sim::outcome::{self as oc, Event, Forced, Key, Recording, Seeded, TapeEvent, TapeValue, Value};
use cg_engine::sim::state::{State, Winner, SIDES, TEAM};
use cg_engine::sim::log::{Detail, Fx, How, ItemWhat, Reset, SideWhat, Who};
use cg_engine::sim::{Choice, Kind, MonSpec, Row};

pub const SIDE_IDS: [&str; 2] = ["p1", "p2"];

pub struct SimBattle {
    pub state: State,
    pub rng: Seeded,
    /// what the last step met that is not implemented, also when it failed
    pub attempt_unimpl: Vec<String>,
    /// the events the last step drew, also when it failed
    pub attempt_events: Vec<Event>,
    /// the choices the last step had refused (side, reason); the battle did not move
    pub refused: Vec<(usize, &'static str)>,
    /// record the battle log's rows (`rig --rows`)
    pub log_rows: bool,
    /// the rows of the last `new` or `step`
    pub rows: Vec<Row>,
}

fn str_field(set: &Json, key: &str) -> String {
    set.get(key).and_then(Json::as_str).map(to_id).unwrap_or_default()
}

/// A Showdown set object to a MonSpec. Unknown ids are errors: the tables cover the whole legal universe
/// (and No Ability).
pub fn mon_spec(set: &Json) -> Result<MonSpec, String> {
    let sid = str_field(set, "species");
    let species = SpeciesId::from_id(&sid).ok_or_else(|| format!("unknown species {}", sid))?;
    let iid = str_field(set, "item");
    let item = if iid.is_empty() { ItemId::NONE } else { ItemId::from_id(&iid).ok_or_else(|| format!("unknown item {}", iid))? };
    let aid = str_field(set, "ability");
    let ability = if aid.is_empty() { AbilityId::NONE } else { AbilityId::from_id(&aid).ok_or_else(|| format!("unknown ability {}", aid))? };
    let mut moves = [MoveId::NONE; 4];
    let mut n = 0;
    for m in set.get("moves").and_then(Json::as_arr).cloned().unwrap_or_default() {
        let id = m.as_str().map(to_id).unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let mv = MoveId::from_id(&id).ok_or_else(|| format!("unknown move {}", id))?;
        if n >= 4 {
            return Err("more than four moves".into());
        }
        moves[n] = mv;
        n += 1;
    }
    if n == 0 {
        return Err(format!("{} has no moves", sid));
    }
    let nid = str_field(set, "nature");
    let nature = d::nature_from_id(if nid.is_empty() { "serious" } else { &nid }).ok_or_else(|| format!("unknown nature {}", nid))?;
    let mut evs = [0u8; 6];
    if let Some(e) = set.get("evs") {
        for (i, k) in ["hp", "atk", "def", "spa", "spd", "spe"].iter().enumerate() {
            evs[i] = e.get(k).and_then(Json::as_i64).unwrap_or(0).clamp(0, 255) as u8;
        }
    }
    let g = set.get("gender").and_then(Json::as_str).unwrap_or("");
    let fixed = species.data().gender;
    let gender = match (g, fixed) {
        (_, "M") | ("M", _) => 1,
        (_, "F") | ("F", _) => 2,
        (_, "N") | ("N", _) => 0,
        _ => return Err(format!("{}: gender required (Showdown would draw it)", sid)),
    };
    Ok(MonSpec { species, ability, item, moves, nmoves: n as u8, nature, evs, gender })
}

fn picks_of(picks: &Json, side: &str) -> Result<[u8; 3], String> {
    let a = picks.get(side).and_then(Json::as_arr).ok_or_else(|| format!("no picks for {}", side))?;
    if a.len() != 3 {
        return Err(format!("bad picks for {}", side));
    }
    let mut p = [0u8; 3];
    for (i, x) in a.iter().enumerate() {
        p[i] = x.as_i64().filter(|v| (0..6).contains(v)).ok_or_else(|| format!("bad picks for {}", side))? as u8;
    }
    Ok(p)
}

fn seed_of(req: &Json) -> u64 {
    let mut s = 0x5EED_u64;
    if let Some(a) = req.get("seed").and_then(Json::as_arr) {
        for x in a {
            s = s.wrapping_mul(0x1_0000_01B3).wrapping_add(x.as_i64().unwrap_or(0) as u64);
        }
    }
    s
}

pub fn tape(force: &Json) -> Result<Vec<TapeEvent>, String> {
    let mut out = Vec::new();
    for ev in force.as_arr().ok_or("force: not a list")? {
        let key_s = ev.get("key").and_then(Json::as_str).unwrap_or("");
        let Some(key) = Key::from_name(key_s) else {
            return Err(format!("tape: event key {:?} not known to the engine", key_s));
        };
        let side = match ev.get("side").and_then(Json::as_str) {
            Some("p1") => Some(0),
            Some("p2") => Some(1),
            _ => None,
        };
        let value = match ev.get("value") {
            Some(Json::Bool(b)) => TapeValue::Bool(*b),
            Some(Json::Num(n)) => TapeValue::Int(*n as i64),
            Some(Json::Str(s)) => TapeValue::Str(s.clone()),
            v => return Err(format!("tape: bad value {:?}", v)),
        };
        let index = match ev.get("index") {
            Some(Json::Num(n)) => Some((*n as i64).to_string()),
            Some(Json::Str(s)) => Some(s.clone()),
            _ => None,
        };
        out.push(TapeEvent {
            key,
            side,
            mv: ev.get("move").and_then(Json::as_str).map(String::from),
            hit: ev.get("hit").and_then(Json::as_i64),
            index,
            effect: ev.get("effect").and_then(Json::as_str).map(String::from),
            value,
        });
    }
    Ok(out)
}

pub fn event_json(e: &Event) -> Json {
    let mut o = Json::obj();
    o.set("key", e.site.key.name());
    o.set("side", match e.site.side {
        Some(s) => Json::from(SIDE_IDS[s as usize]),
        None => Json::Null,
    });
    if let Some(m) = e.site.mv {
        o.set("move", m.id());
    } else if let Some(m) = e.site.mv_name {
        o.set("move", m);
    }
    if let Some(h) = e.site.hit {
        o.set("hit", h as i64);
    }
    match e.site.index {
        Some(oc::SecIndex::At(i)) => o.set("index", i as i64),
        Some(oc::SecIndex::SelfEffect) => o.set("index", "self"),
        None => {}
    }
    if let Some(s) = oc::effect_string(&e.site.effect) {
        o.set("effect", s);
    }
    if let Some((lo, hi)) = e.site.args {
        o.set("kind", "shuffle");
        o.set("args", Json::Arr(vec![Json::from(lo as i64), Json::from(hi as i64)]));
    }
    match e.value {
        Value::Bool(b) => o.set("value", b),
        Value::Int(i) => o.set("value", i as i64),
        Value::Side(s) => o.set("value", SIDE_IDS[s as usize]),
        Value::Effect(e) => o.set("value", e.id()),
        Value::Str(x) => o.set("value", x),
    }
    o.set("p", e.p);
    o
}

/// A failed `new`: the message and what the attempt met that is not implemented.
pub struct NewError {
    pub msg: String,
    pub unimplemented: Vec<String>,
}

impl From<String> for NewError {
    fn from(msg: String) -> NewError {
        NewError { msg, unimplemented: vec![] }
    }
}

impl From<&str> for NewError {
    fn from(msg: &str) -> NewError {
        NewError { msg: msg.into(), unimplemented: vec![] }
    }
}

fn tape_error(e: String) -> String {
    format!("tape:{}", e)
}

impl SimBattle {
    pub fn new(req: &Json, log_rows: bool) -> Result<(SimBattle, Vec<Event>), NewError> {
        let teams = req.get("teams").ok_or("new: no teams")?;
        let picks = req.get("picks").ok_or("new: no picks")?;
        let mut specs: [Vec<MonSpec>; 2] = [vec![], vec![]];
        let mut pk = [[0u8; 3]; 2];
        for (i, sid) in SIDE_IDS.iter().enumerate() {
            let t = teams.get(sid).and_then(Json::as_arr).ok_or_else(|| format!("{} needs 6 sets", sid))?;
            if t.len() != 6 {
                return Err(format!("{} needs 6 sets", sid).into());
            }
            for set in t {
                specs[i].push(mon_spec(set)?);
            }
            pk[i] = picks_of(picks, sid)?;
        }
        let mut rng = Seeded::new(seed_of(req));
        let mut rows = Vec::new();
        let (state, events) = match req.get("force") {
            Some(fj) => {
                let mut src = Recording::new(Forced::new(tape(fj)?));
                let st = if log_rows {
                    State::start_logged([&specs[0], &specs[1]], pk, &mut src, &mut rows)?
                } else {
                    State::start([&specs[0], &specs[1]], pk, &mut src)?
                };
                if let Err(e) = src.inner.finish() {
                    return Err(NewError { msg: tape_error(e), unimplemented: st.unimpl.describe() });
                }
                (st, src.events)
            }
            None => {
                let mut src = Recording::new(rng.clone());
                let st = if log_rows {
                    State::start_logged([&specs[0], &specs[1]], pk, &mut src, &mut rows)?
                } else {
                    State::start([&specs[0], &specs[1]], pk, &mut src)?
                };
                rng = src.inner;
                (st, src.events)
            }
        };
        let attempt_unimpl = state.unimpl.describe();
        Ok((SimBattle { state, rng, attempt_unimpl, attempt_events: vec![], refused: vec![], log_rows, rows }, events))
    }

    /// One step of the State with or without rows.
    fn play<O: cg_engine::sim::Outcomes>(&mut self, st: &mut State, choices: [Option<Choice>; SIDES], src: &mut O) -> Result<(), String> {
        self.rows.clear();
        if self.log_rows {
            st.step_logged(choices, src, &mut self.rows)
        } else {
            st.step(choices, src)
        }
    }

    /// One step; on any error the battle is left unchanged.
    pub fn step(&mut self, req: &Json) -> Result<Vec<Event>, String> {
        let ch = req.get("choices").cloned().unwrap_or_else(Json::obj);
        let mut choices = [None; SIDES];
        for (i, sid) in SIDE_IDS.iter().enumerate() {
            if let Some(c) = ch.get(sid).and_then(Json::as_str) {
                choices[i] = Some(parse_choice(c)?);
            }
        }
        self.refused.clear();
        self.rows.clear();
        // a choice the game offers and refuses: nothing is played, the side learns why
        if let Some(r) = self.state.refuse(choices) {
            for (i, x) in r.iter().enumerate() {
                if let Some(why) = x {
                    self.refused.push((i, why));
                }
            }
            self.attempt_events.clear();
            return Ok(vec![]);
        }
        let mut st = self.state;
        let r = match req.get("force") {
            Some(fj) => {
                let mut src = Recording::new(Forced::new(tape(fj)?));
                let r = self.play(&mut st, choices, &mut src);
                self.attempt_events = src.events.clone();
                r.and_then(|_| src.inner.finish().map_err(tape_error)).map(|_| src.events)
            }
            None => {
                let mut src = Recording::new(self.rng.clone());
                let r = self.play(&mut st, choices, &mut src).map(|_| src.events);
                if r.is_ok() {
                    self.rng = src.inner;
                }
                r
            }
        };
        self.attempt_unimpl = st.unimpl.describe();
        let events = r?;
        self.state = st;
        Ok(events)
    }

    pub fn options(&self) -> Json {
        options_json(&self.state)
    }

    pub fn snapshot(&self) -> Json {
        snapshot(&self.state)
    }

    pub fn unimplemented(&self) -> Json {
        Json::Arr(self.state.unimpl.describe().into_iter().map(Json::Str).collect())
    }

    /// The last step's rows.
    pub fn rows_json(&self) -> Json {
        Json::Arr(self.rows.iter().map(row_json).collect())
    }
}

fn who_json(w: Who) -> Json {
    Json::Arr(vec![Json::from(w.0 as i64), Json::from(w.1 as i64)])
}

fn fx_str(f: Fx) -> String {
    match f {
        Fx::None => String::new(),
        Fx::Move(m) => format!("move:{}", m.id()),
        Fx::Ability(a) => format!("ability:{}", a.id()),
        Fx::Item(i) => format!("item:{}", i.id()),
        Fx::Cond(c) => format!("cond:{}", c.id()),
        Fx::Status(s) => format!("status:{}", s.id()),
        Fx::Recoil(m) => format!("recoil:{}", m.id()),
        Fx::Drain(m) => format!("drain:{}", m.id()),
        Fx::Crash(m) => format!("crash:{}", m.id()),
        Fx::Cost(m) => format!("cost:{}", m.id()),
        Fx::Confusion => "confusion".into(),
        Fx::HpFull => "hpfull".into(),
    }
}

fn detail_json(d: Detail) -> Json {
    match d {
        Detail::None => Json::Null,
        Detail::Move(m) => Json::from(m.id()),
        Detail::Num(n) => Json::from(n as i64),
        Detail::Type(t) => Json::from(t.id()),
    }
}

fn opt_who(o: &mut Json, key: &str, w: Option<Who>) {
    if let Some(w) = w {
        o.set(key, who_json(w));
    }
}

/// A row as JSON: `k` its kind, Pokémon as [side, slot], effects as "kind:id".
pub fn row_json(r: &Row) -> Json {
    let mut o = Json::obj();
    o.set("k", r.kind());
    match *r {
        Row::Turn(n) => o.set("n", n as i64),
        Row::Enter { p, forced } => {
            o.set("p", who_json(p));
            o.set("forced", forced);
        }
        Row::Withdraw { p, pivot } => {
            o.set("p", who_json(p));
            if let Some(m) = pivot {
                o.set("pivot", m.id());
            }
        }
        Row::Move { p, mv, target, called_by, priority } => {
            o.set("p", who_json(p));
            o.set("move", mv.id());
            opt_who(&mut o, "target", target);
            o.set("called_by", fx_str(called_by));
            o.set("priority", priority as i64);
        }
        Row::Prepare { p, mv } => {
            o.set("p", who_json(p));
            o.set("move", mv.id());
        }
        Row::Cant { p, why, mv } => {
            o.set("p", who_json(p));
            o.set("why", fx_str(why));
            if !mv.is_none() {
                o.set("move", mv.id());
            }
        }
        Row::Protect { p, by } => {
            o.set("p", who_json(p));
            o.set("by", by.id());
        }
        Row::Miss { p, by } => {
            o.set("p", who_json(p));
            o.set("by", who_json(by));
        }
        Row::Fail { p, why } => {
            opt_who(&mut o, "p", p);
            o.set("why", fx_str(why));
        }
        Row::Result { how, p } => {
            let h = match how {
                How::Super => "super",
                How::Extreme => "extreme",
                How::Resisted => "resisted",
                How::Mostly => "mostly_ineffective",
                How::Immune => "immune",
                How::Ohko => "ohko",
            };
            o.set("how", h);
            opt_who(&mut o, "p", p);
        }
        Row::Hits { n, p } => {
            o.set("n", n as i64);
            o.set("p", who_json(p));
        }
        Row::Hp { p, before, after, cause, of } => {
            o.set("p", who_json(p));
            o.set("before", before as i64);
            o.set("after", after as i64);
            o.set("cause", fx_str(cause));
            opt_who(&mut o, "of", of);
        }
        Row::Faint { p } | Row::Revive { p } => o.set("p", who_json(p)),
        Row::Status { p, st, by, of, rest } => {
            o.set("p", who_json(p));
            o.set("status", st.id());
            o.set("by", fx_str(by));
            opt_who(&mut o, "of", of);
            if rest {
                o.set("rest", true);
            }
        }
        Row::Cure { p, st, by } => {
            o.set("p", who_json(p));
            o.set("status", st.id());
            o.set("by", fx_str(by));
        }
        Row::Volatile { p, c, detail, by, of } => {
            o.set("p", who_json(p));
            o.set("effect", c.id());
            o.set("detail", detail_json(detail));
            o.set("by", fx_str(by));
            opt_who(&mut o, "of", of);
        }
        Row::VolatileEnd { p, c, by } => {
            o.set("p", who_json(p));
            o.set("effect", c.id());
            o.set("by", fx_str(by));
        }
        Row::VolatileActed { p, c, detail, of } => {
            o.set("p", who_json(p));
            o.set("effect", c.id());
            o.set("detail", detail_json(detail));
            opt_who(&mut o, "of", of);
        }
        Row::Boost { p, by_stat, by } => {
            o.set("p", who_json(p));
            let mut b = Json::obj();
            for (i, &v) in by_stat.iter().enumerate() {
                if v != 0 {
                    b.set(BOOST_IDS[i], v as i64);
                }
            }
            o.set("stats", b);
            o.set("by", fx_str(by));
        }
        Row::BoostLimit { p, stats, up, by } => {
            o.set("p", who_json(p));
            let names: Vec<Json> = (0..7).filter(|&i| stats[i]).map(|i| Json::from(BOOST_IDS[i])).collect();
            o.set("stats", Json::Arr(names));
            o.set("dir", if up { "up" } else { "down" });
            o.set("by", fx_str(by));
        }
        Row::BoostReset { p, what, of, by } => {
            opt_who(&mut o, "p", p);
            let w = match what {
                Reset::Field => "field",
                Reset::All => "all",
                Reset::Negative => "negative",
                Reset::Positive => "positive",
                Reset::Copy => "copy",
                Reset::Swap => "swap",
                Reset::Invert => "invert",
            };
            o.set("what", w);
            opt_who(&mut o, "of", of);
            o.set("by", fx_str(by));
        }
        Row::Blocked { p, by, what } => {
            opt_who(&mut o, "p", p);
            o.set("by", fx_str(by));
            o.set("what", fx_str(what));
        }
        Row::Ability { p, ab, of } => {
            o.set("p", who_json(p));
            o.set("ability", ab.id());
            opt_who(&mut o, "of", of);
        }
        Row::AbilityChange { p, from, to, by } => {
            o.set("p", who_json(p));
            o.set("from", from.id());
            o.set("to", to.id());
            o.set("by", fx_str(by));
        }
        Row::Item { p, item, what, of, by } => {
            o.set("p", who_json(p));
            o.set("item", item.id());
            let w = match what {
                ItemWhat::Activated => "activated",
                ItemWhat::Used => "used",
                ItemWhat::Removed => "removed",
                ItemWhat::Received => "received",
                ItemWhat::Found => "found",
            };
            o.set("what", w);
            opt_who(&mut o, "of", of);
            o.set("by", fx_str(by));
        }
        Row::MegaStone { p, stone } => {
            o.set("p", who_json(p));
            o.set("stone", stone.id());
        }
        Row::Mega { p, form } => {
            o.set("p", who_json(p));
            o.set("form", form.id());
        }
        Row::Form { p, form, by } => {
            o.set("p", who_json(p));
            o.set("form", form.id());
            o.set("by", fx_str(by));
        }
        Row::Transform { p, into } => {
            o.set("p", who_json(p));
            o.set("into", who_json(into));
        }
        Row::Types { p, types, lost, added, by } => {
            o.set("p", who_json(p));
            let t: Vec<Json> = types.iter().filter(|t| t.is_real()).map(|t| Json::from(t.id())).collect();
            o.set("types", Json::Arr(t));
            if lost.is_real() {
                o.set("lost", lost.id());
            }
            if added.is_real() {
                o.set("added", added.id());
            }
            o.set("by", fx_str(by));
        }
        Row::Weather { w, by, of } => {
            o.set("weather", if w.is_none() { "" } else { w.id() });
            o.set("by", fx_str(by));
            opt_who(&mut o, "of", of);
        }
        Row::Field { c, start, by, of } => {
            o.set("effect", c.id());
            o.set("what", if start { "start" } else { "end" });
            o.set("by", fx_str(by));
            opt_who(&mut o, "of", of);
        }
        Row::Side { side, c, what, p, by } => {
            o.set("side", side as i64);
            o.set("effect", c.id());
            o.set(
                "what",
                match what {
                    SideWhat::Start => "start",
                    SideWhat::End => "end",
                    SideWhat::Acted => "acted",
                },
            );
            opt_who(&mut o, "p", p);
            o.set("by", fx_str(by));
        }
        Row::Once { p, fx, of } => {
            opt_who(&mut o, "p", p);
            o.set("effect", fx_str(fx));
            opt_who(&mut o, "of", of);
        }
        Row::Pp { p, mv, lost, by } => {
            o.set("p", who_json(p));
            o.set("move", mv.id());
            o.set("lost", lost as i64);
            o.set("by", fx_str(by));
        }
        Row::End { winner } => match winner {
            Winner::Side(s) => o.set("winner", s as i64),
            Winner::Tie => o.set("winner", "tie"),
            Winner::None => {}
        },
    }
    o
}

pub fn parse_choice(c: &str) -> Result<Choice, String> {
    let parts: Vec<&str> = c.split_whitespace().collect();
    match parts.as_slice() {
        ["move", n] if n.parse::<u8>().is_err() => Ok(Choice::Move { index: cg_engine::sim::battle::STRUGGLE, mega: false }),
        ["move", n] => Ok(Choice::Move { index: n.parse().map_err(|_| format!("bad choice {}", c))?, mega: false }),
        ["move", n, "mega"] => Ok(Choice::Move { index: n.parse().map_err(|_| format!("bad choice {}", c))?, mega: true }),
        ["switch", n] => Ok(Choice::Switch(n.parse().map_err(|_| format!("bad choice {}", c))?)),
        _ => Err(format!("bad choice {}", c)),
    }
}

pub fn options_json(s: &State) -> Json {
    let mut out = Json::obj();
    for (i, o) in s.options().iter().enumerate() {
        let mut j = Json::obj();
        match o.kind {
            Kind::Wait => j.set("kind", "wait"),
            Kind::Done => j.set("kind", "done"),
            Kind::Replace => {
                j.set("kind", "replace");
                j.set("switches", ints(&o.switch_list()));
            }
            Kind::Turn => {
                j.set("kind", "turn");
                if let Some(f) = o.forced {
                    j.set("moves", Json::Arr(vec![Json::from(f)]));
                } else if o.struggle {
                    j.set("moves", Json::Arr(vec![Json::from("struggle")]));
                } else {
                    j.set("moves", ints(&o.move_list()));
                }
                j.set("mega", o.mega);
                j.set("switches", ints(&o.switch_list()));
                if o.hidden_switches.iter().any(|x| *x) || o.hidden_moves.iter().any(|x| *x) {
                    let mut h = Json::obj();
                    h.set("switches", ints(&o.hidden_switch_list()));
                    h.set("moves", ints(&o.hidden_move_list()));
                    j.set("hidden", h);
                }
            }
        }
        out.set(SIDE_IDS[i], j);
    }
    out
}

/// A volatile as the oracle exports it: `left` (Showdown's duration; confusion's time) and the state fields
/// that affect play.
fn volatile_json(v: &cg_engine::sim::state::Volatile) -> Json {
    use cg_engine::sim::data::conds as c;
    let mut x = Json::obj();
    if v.id == c::CONFUSION {
        if v.data != 0 {
            x.set("left", v.data as i64);
        }
    } else if v.duration != 0 {
        x.set("left", v.duration as i64);
    }
    match v.id {
        id if id == c::SUBSTITUTE => {
            // kept in half points
            if v.hp % 2 == 0 {
                x.set("hp", (v.hp / 2) as i64)
            } else {
                x.set("hp", Json::Num(v.hp as f64 / 2.0))
            }
        }
        id if id == c::LOCKEDMOVE => {
            x.set("move", v.mv.id());
            x.set("trueDuration", v.data as i64);
        }
        id if id == c::TWOTURNMOVE || id == c::ENCORE || id == c::DISABLE => {
            if !v.mv.is_none() {
                x.set("move", v.mv.id());
            }
        }
        id if id == c::STALL || id == c::ALLYSWITCH => x.set("counter", v.counter as i64),
        id if id == c::PROTOSYNTHESIS || id == c::QUARKDRIVE => {
            x.set("bestStat", ["atk", "def", "spa", "spd", "spe"][(v.data as usize).min(4)]);
        }
        id if id == c::STOCKPILE => x.set("layers", v.data as i64),
        id if id == c::PARTIALLYTRAPPED => {
            x.set("boundDivisor", v.data as i64);
            x.set("move", v.mv.id());
        }
        id if id == c::CHOICELOCK => x.set("move", v.mv.id()),
        id if id == c::METRONOME => {
            x.set("lastMove", v.mv.id());
            x.set("numConsecutive", v.data as i64);
        }
        _ => {}
    }
    x
}

fn ints(v: &[u8]) -> Json {
    Json::Arr(v.iter().map(|x| Json::from(*x as i64)).collect())
}

fn field_cond(c: &cg_engine::sim::state::FieldCond) -> Json {
    let mut o = Json::obj();
    if c.id.is_none() {
        o.set("id", "");
        o.set("left", 0);
    } else {
        o.set("id", c.id.id());
        o.set("left", if c.duration > 0 { Json::from(c.duration as i64) } else { Json::Null });
    }
    o
}

pub fn snapshot(s: &State) -> Json {
    let mut o = Json::obj();
    o.set("turn", s.turn as i64);
    o.set("ended", s.ended);
    o.set("winner", match s.winner {
        Winner::None => Json::Null,
        Winner::Side(x) => Json::from(SIDE_IDS[x as usize]),
        Winner::Tie => Json::from("tie"),
    });
    o.set("weather", field_cond(&s.field.weather));
    o.set("terrain", field_cond(&s.field.terrain));
    let mut rooms = Json::obj();
    for c in &s.field.pseudo[..s.field.npseudo as usize] {
        rooms.set(c.id.id(), if c.duration > 0 { Json::from(c.duration as i64) } else { Json::Null });
    }
    o.set("rooms", rooms);
    let mut sides = Json::obj();
    for side in 0..SIDES {
        sides.set(SIDE_IDS[side], side_json(s, side));
    }
    o.set("sides", sides);
    o
}

fn side_json(s: &State, side: usize) -> Json {
    let sd = &s.sides[side];
    let mut o = Json::obj();
    o.set("active", if sd.has_active { Json::from(sd.active as i64) } else { Json::Null });
    o.set("megaUsed", sd.mega_used());
    let mut conds = Json::obj();
    for c in sd.conds() {
        let hazard = ["stealthrock", "spikes", "toxicspikes", "stickyweb"].contains(&c.id.id());
        let v = if hazard { c.layers.max(1) as i64 } else if c.duration > 0 { c.duration as i64 } else { c.layers.max(1) as i64 };
        conds.set(c.id.id(), v);
    }
    let mut wish = None;
    let mut future = None;
    for c in sd.slot_conds() {
        if c.id == d::conds::WISH {
            // the oracle's wishLeft: startingTurn + 3 - the turn whose end-of-turn step runs next
            let pending = s.queue.as_slice().iter().any(|a| a.kind == cg_engine::sim::state::ActionKind::Residual);
            let next = s.turn as i64 + if s.mid_turn && !pending { 1 } else { 0 };
            let mut w = Json::obj();
            w.set("left", c.turn as i64 + 3 - next);
            w.set("hp", c.data.max(1) as i64);
            wish = Some(w);
        } else if c.id == d::conds::FUTUREMOVE {
            // Showdown keeps the ending turn, not a duration
            let mut f = Json::obj();
            f.set("move", MoveId(c.data as u16).id());
            future = Some(f);
        } else {
            conds.set(c.id.id(), if c.duration > 0 { c.duration as i64 } else { 1 });
        }
    }
    o.set("conditions", conds);
    if let Some(w) = wish {
        o.set("wish", w);
    }
    if let Some(f) = future {
        o.set("futuresight", f);
    }
    let mons: Vec<Json> = (0..TEAM).map(|i| mon_json(s, side, i)).collect();
    o.set("pokemon", Json::Arr(mons));
    o
}

fn mon_json(s: &State, side: usize, slot: usize) -> Json {
    let sd = &s.sides[side];
    let m = &sd.mons[slot];
    let active = m.is_active;
    let mut o = Json::obj();
    o.set("species", m.species.id());
    o.set("baseSpecies", m.set_species.id());
    let mut tl: Vec<d::Type> = m.types[..m.ntypes as usize].to_vec();
    if active && sd.has_vol(d::conds::ROOST) {
        tl.retain(|t| *t != d::Type::Flying);
    }
    if tl.is_empty() {
        tl.push(d::Type::Normal);
    }
    if m.added_type != d::Type::Typeless {
        tl.push(m.added_type);
    }
    let types: Vec<Json> = tl.iter().filter(|t| t.is_real() || **t == d::Type::Stellar).map(|t| Json::from(t.id())).collect();
    o.set("types", Json::Arr(types));
    o.set("ability", m.ability.id());
    o.set("baseAbility", m.base_ability.id());
    o.set("item", m.item.id());
    o.set("hp", m.hp as i64);
    o.set("maxhp", m.maxhp as i64);
    o.set("fainted", m.fainted);
    let mut st = Json::obj();
    for (i, k) in ["atk", "def", "spa", "spd", "spe"].iter().enumerate() {
        st.set(k, m.stats[i] as i64);
    }
    o.set("stats", st);
    let status = if m.fainted { "" } else { m.status.id() };
    o.set("status", status);
    let mut sdj = Json::obj();
    match status {
        "slp" | "frz" => sdj.set("turns", m.status_time as i64),
        "tox" => sdj.set("stage", if active { m.tox_stage as i64 } else { 0 }),
        _ => {}
    }
    o.set("statusData", sdj);
    let dis = s.disabled_flags(side, slot);
    let mv: Vec<Json> = m
        .moves()
        .iter()
        .enumerate()
        .map(|(i, ms)| {
            let mut x = Json::obj();
            x.set("id", ms.id.id());
            x.set("pp", ms.pp as i64);
            x.set("maxpp", ms.max_pp as i64);
            x.set("disabled", dis[i]);
            x
        })
        .collect();
    o.set("moves", Json::Arr(mv));
    let mut b = Json::obj();
    for (i, k) in BOOST_IDS.iter().enumerate() {
        b.set(k, if active { m.boosts[i] as i64 } else { 0 });
    }
    o.set("boosts", b);
    let mut vols = Json::obj();
    if active {
        for v in sd.volatiles() {
            // a volatile that only prints a message (the oracle's BOOKKEEPING)
            if v.id == d::conds::CHILLYRECEPTION {
                continue;
            }
            vols.set(v.id.id(), volatile_json(v));
        }
    }
    o.set("volatiles", vols);
    o.set("lastMove", if active && !m.last_move.is_none() { m.last_move.id() } else { "" });
    o.set("activeTurns", if active { m.active_turns as i64 } else { 0 });
    o
}
