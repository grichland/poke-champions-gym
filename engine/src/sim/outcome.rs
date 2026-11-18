//! The outcome source: every random event of a battle passes through one `Outcomes` value, keyed by the rig
//! contract's event keys (tools/rig/CONTRACT.md section 5). Each call states its alternatives and their
//! probabilities, so an enumerating source can be written later. Calls are made only for events with more
//! than one possible outcome, as the contract reports them.
//!
//! Sources: `Seeded` (own PRNG), `Forced` (a tape of contract events), `Recording` (wraps another and keeps
//! the events with their probabilities). The resolver takes the source as a generic parameter.

use super::data::{AbilityId, Cond, ItemId, MoveId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    SpeedTie,
    Accuracy,
    Crit,
    Roll,
    Hits,
    Secondary,
    FullPara,
    Thaw,
    SleepTurns,
    ConfusionTurns,
    ConfusionHit,
    LockTurns,
    TrapTurns,
    Protect,
    Proc,
    Pick,
    /// a draw the oracle reports unclassified: here only the shuffle of three or more tied handlers
    Raw,
}

pub const KEY_NAMES: [&str; 17] = [
    "speedtie",
    "accuracy",
    "crit",
    "roll",
    "hits",
    "secondary",
    "fullpara",
    "thaw",
    "sleepturns",
    "confusionturns",
    "confusionhit",
    "lockturns",
    "trapturns",
    "protect",
    "proc",
    "pick",
    "raw",
];

impl Key {
    pub fn name(self) -> &'static str {
        KEY_NAMES[self as usize]
    }
    pub fn from_name(s: &str) -> Option<Key> {
        KEY_NAMES.iter().position(|k| *k == s).map(|i| ALL_KEYS[i])
    }
}

const ALL_KEYS: [Key; 17] = [
    Key::SpeedTie,
    Key::Accuracy,
    Key::Crit,
    Key::Roll,
    Key::Hits,
    Key::Secondary,
    Key::FullPara,
    Key::Thaw,
    Key::SleepTurns,
    Key::ConfusionTurns,
    Key::ConfusionHit,
    Key::LockTurns,
    Key::TrapTurns,
    Key::Protect,
    Key::Proc,
    Key::Pick,
    Key::Raw,
];

/// The effect an event belongs to (the contract's `effect` field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectRef {
    Move(MoveId),
    Ability(AbilityId),
    Item(ItemId),
    Cond(Cond),
}

impl EffectRef {
    pub fn id(self) -> &'static str {
        match self {
            EffectRef::Move(m) => m.id(),
            EffectRef::Ability(a) => a.id(),
            EffectRef::Item(i) => i.id(),
            EffectRef::Cond(c) => c.id(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    None,
    /// a tie between two Pokémon in an ordered list (`speedtie` with effect "pokemon")
    Pokemon,
    /// an action inserted among equals (`speedtie` with effect "insert")
    Insert,
    /// a drag with no move running (a Red Card used by a residual hit): Showdown's "dragin"
    DragIn,
    /// a tie between two handlers: Showdown's "a+b" (ids sorted)
    Pair(EffectRef, EffectRef),
    One(EffectRef),
}

/// `index` of a `secondary` event: a position in the move's list, or the move's own `self` effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SecIndex {
    At(u8),
    SelfEffect,
}

/// Where an event happens: everything but its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Site {
    pub key: Key,
    /// 0 = p1, 1 = p2, None = null
    pub side: Option<u8>,
    pub mv: Option<MoveId>,
    /// the `move` field when it is not a move of the tables (the confusion self-hit's "confusion")
    pub mv_name: Option<&'static str>,
    /// a raw shuffle draw's range (`args` of the oracle's raw event)
    pub args: Option<(i32, i32)>,
    pub hit: Option<u8>,
    pub index: Option<SecIndex>,
    pub effect: Effect,
}

impl Site {
    pub fn new(key: Key, side: Option<u8>) -> Site {
        Site { key, side, mv: None, mv_name: None, args: None, hit: None, index: None, effect: Effect::None }
    }
    pub fn of_move(key: Key, side: u8, mv: MoveId, hit: u8) -> Site {
        Site { key, side: Some(side), mv: Some(mv), mv_name: None, args: None, hit: Some(hit), index: None, effect: Effect::None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i32),
    /// a side id (speed ties)
    Side(u8),
    /// the effect that goes first (a tie between two handlers of one side)
    Effect(EffectRef),
    /// a named choice (a stat, a status, a move id)
    Str(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub site: Site,
    pub value: Value,
    pub p: f64,
}

/// The one door for randomness. Implementations decide each event; `Forced` may fail, which it records
/// (see `error`) while still answering, so the resolver needs no error paths.
pub trait Outcomes {
    /// Showdown's randomChance(num, den), called only when 0 < num < den. True with probability num/den.
    fn chance(&mut self, site: &Site, num: u32, den: u32) -> bool;
    /// A uniform integer in lo..hi (hi exclusive, hi - lo >= 2).
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32;
    /// One of `values`, each entry equally likely (repeats weight a value). Returns the chosen value.
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32;
    /// A tie between sides `a` and `b`, each first with probability 1/2. Returns the side that goes first.
    fn tie(&mut self, site: &Site, a: u8, b: u8) -> u8;
    /// A tie between two handlers of one side: true when `a` goes first (probability 1/2).
    fn tie_effects(&mut self, site: &Site, a: EffectRef, b: EffectRef) -> bool;
    /// One of `values` (named), each equally likely. Returns its index.
    fn sample_str(&mut self, site: &Site, values: &[&'static str]) -> usize;
    /// The first tape error, if this source can fail.
    fn error(&self) -> Option<&str> {
        None
    }
}

// --------------------------------------------------------------------------------------------- seeded

/// xoshiro256** seeded through SplitMix64.
#[derive(Clone, Debug)]
pub struct Seeded {
    s: [u64; 4],
}

impl Seeded {
    pub fn new(seed: u64) -> Seeded {
        let mut z = seed;
        let mut s = [0u64; 4];
        for x in s.iter_mut() {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut v = z;
            v = (v ^ (v >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            v = (v ^ (v >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *x = v ^ (v >> 31);
        }
        Seeded { s }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// uniform in 0..n (Lemire's multiply-shift with rejection)
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        let n64 = n as u64;
        let mut m = (self.next_u64() >> 32) * n64;
        if (m as u32) < n {
            let t = n.wrapping_neg() % n;
            while (m as u32) < t {
                m = (self.next_u64() >> 32) * n64;
            }
        }
        (m >> 32) as u32
    }
}

impl Outcomes for Seeded {
    #[inline]
    fn chance(&mut self, _: &Site, num: u32, den: u32) -> bool {
        self.below(den) < num
    }
    #[inline]
    fn range(&mut self, _: &Site, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo) as u32) as i32
    }
    #[inline]
    fn sample(&mut self, _: &Site, values: &[i32]) -> i32 {
        values[self.below(values.len() as u32) as usize]
    }
    #[inline]
    fn tie(&mut self, _: &Site, a: u8, b: u8) -> u8 {
        if self.below(2) == 0 {
            a
        } else {
            b
        }
    }
    #[inline]
    fn tie_effects(&mut self, _: &Site, _: EffectRef, _: EffectRef) -> bool {
        self.below(2) == 0
    }
    #[inline]
    fn sample_str(&mut self, _: &Site, values: &[&'static str]) -> usize {
        self.below(values.len() as u32) as usize
    }
}

// --------------------------------------------------------------------------------------------- forced

/// A tape event: the contract fields that a forced program must match, and the value.
#[derive(Clone, Debug, PartialEq)]
pub struct TapeEvent {
    pub key: Key,
    pub side: Option<u8>,
    /// fields present on the tape; each must match the event the battle needs
    pub mv: Option<String>,
    pub hit: Option<i64>,
    pub index: Option<String>,
    pub effect: Option<String>,
    pub value: TapeValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TapeValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

/// Answers every event from a tape, in order per (key, side). Records the first mismatch, missing or
/// leftover event as a tape error.
#[derive(Clone, Debug, Default)]
pub struct Forced {
    queues: Vec<((Key, Option<u8>), std::collections::VecDeque<TapeEvent>)>,
    err: Option<String>,
}

pub fn side_name(s: Option<u8>) -> &'static str {
    match s {
        Some(0) => "p1",
        Some(1) => "p2",
        _ => "null",
    }
}

pub fn effect_string(e: &Effect) -> Option<String> {
    match e {
        Effect::None => None,
        Effect::Pokemon => Some("pokemon".into()),
        Effect::Insert => Some("insert".into()),
        Effect::DragIn => Some("dragin".into()),
        Effect::One(x) => Some(x.id().to_string()),
        Effect::Pair(a, b) => {
            let (x, y) = (a.id(), b.id());
            Some(if x <= y { format!("{}+{}", x, y) } else { format!("{}+{}", y, x) })
        }
    }
}

pub fn index_string(i: &Option<SecIndex>) -> Option<String> {
    match i {
        None => None,
        Some(SecIndex::At(n)) => Some(n.to_string()),
        Some(SecIndex::SelfEffect) => Some("self".into()),
    }
}

impl Forced {
    pub fn new(tape: Vec<TapeEvent>) -> Forced {
        let mut f = Forced::default();
        for ev in tape {
            let k = (ev.key, ev.side);
            match f.queues.iter_mut().find(|(q, _)| *q == k) {
                Some((_, q)) => q.push_back(ev),
                None => f.queues.push((k, std::collections::VecDeque::from(vec![ev]))),
            }
        }
        f
    }

    /// Call after the step: an unused event is an error.
    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        for (_, q) in &self.queues {
            if let Some(ev) = q.front() {
                return Err(format!("unused event {:?}", ev));
            }
        }
        Ok(())
    }

    fn fail(&mut self, msg: String) {
        if self.err.is_none() {
            self.err = Some(msg);
        }
    }

    fn take(&mut self, site: &Site) -> Option<TapeValue> {
        let k = (site.key, site.side);
        let ev = self.queues.iter_mut().find(|(q, _)| *q == k).and_then(|(_, q)| q.pop_front());
        let Some(ev) = ev else {
            self.fail(format!("missing event {}", describe(site)));
            return None;
        };
        let mv = site.mv.map(|m| m.id().to_string()).or(site.mv_name.map(String::from));
        let ok = (ev.mv.is_none() || ev.mv == mv)
            && (ev.hit.is_none() || ev.hit == site.hit.map(|h| h as i64))
            && (ev.index.is_none() || ev.index == index_string(&site.index))
            && (ev.effect.is_none() || ev.effect == effect_string(&site.effect));
        if !ok {
            self.fail(format!("event {:?} does not match needed {}", ev, describe(site)));
            return None;
        }
        Some(ev.value)
    }
}

pub fn describe(site: &Site) -> String {
    let mut s = format!("{{key:{}, side:{}", site.key.name(), side_name(site.side));
    if let Some(m) = site.mv {
        s += &format!(", move:{}", m.id());
    } else if let Some(m) = site.mv_name {
        s += &format!(", move:{}", m);
    }
    if let Some(h) = site.hit {
        s += &format!(", hit:{}", h);
    }
    if let Some(i) = index_string(&site.index) {
        s += &format!(", index:{}", i);
    }
    if let Some(e) = effect_string(&site.effect) {
        s += &format!(", effect:{}", e);
    }
    s + "}"
}

impl Outcomes for Forced {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        match self.take(site) {
            Some(TapeValue::Bool(b)) => b,
            Some(v) => {
                self.fail(format!("impossible value {:?} for {}", v, describe(site)));
                false
            }
            None => false,
        }
    }
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32 {
        match self.take(site) {
            Some(TapeValue::Int(v)) if v >= lo as i64 && v < hi as i64 => v as i32,
            Some(v) => {
                self.fail(format!("impossible value {:?} for {}", v, describe(site)));
                lo
            }
            None => lo,
        }
    }
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32 {
        match self.take(site) {
            Some(TapeValue::Int(v)) if values.iter().any(|x| *x as i64 == v) => v as i32,
            Some(v) => {
                self.fail(format!("impossible value {:?} for {}", v, describe(site)));
                values[0]
            }
            None => values[0],
        }
    }
    fn tie(&mut self, site: &Site, a: u8, b: u8) -> u8 {
        match self.take(site) {
            Some(TapeValue::Str(s)) if s == side_name(Some(a)) => a,
            Some(TapeValue::Str(s)) if s == side_name(Some(b)) => b,
            Some(v) => {
                self.fail(format!("impossible value {:?} for {}", v, describe(site)));
                a
            }
            None => a,
        }
    }
    fn tie_effects(&mut self, site: &Site, a: EffectRef, b: EffectRef) -> bool {
        match self.take(site) {
            Some(TapeValue::Str(s)) if s == a.id() => true,
            Some(TapeValue::Str(s)) if s == b.id() => false,
            Some(v) => {
                self.fail(format!("impossible value {:?} for {}", v, describe(site)));
                true
            }
            None => true,
        }
    }
    fn sample_str(&mut self, site: &Site, values: &[&'static str]) -> usize {
        match self.take(site) {
            Some(TapeValue::Str(s)) if values.iter().any(|v| *v == s) => values.iter().position(|v| *v == s).unwrap(),
            Some(v) => {
                self.fail(format!("impossible value {:?} for {}", v, describe(site)));
                0
            }
            None => 0,
        }
    }
    fn error(&self) -> Option<&str> {
        self.err.as_deref()
    }
}

// --------------------------------------------------------------------------------------------- recording

/// Wraps a source and keeps every event with its value and probability.
#[derive(Clone, Debug)]
pub struct Recording<O> {
    pub inner: O,
    pub events: Vec<Event>,
}

impl<O: Outcomes> Recording<O> {
    pub fn new(inner: O) -> Recording<O> {
        Recording { inner, events: Vec::new() }
    }
}

impl<O: Outcomes> Outcomes for Recording<O> {
    fn chance(&mut self, site: &Site, num: u32, den: u32) -> bool {
        let v = self.inner.chance(site, num, den);
        let t = num as f64 / den as f64;
        self.events.push(Event { site: *site, value: Value::Bool(v), p: if v { t } else { 1.0 - t } });
        v
    }
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32 {
        let v = self.inner.range(site, lo, hi);
        self.events.push(Event { site: *site, value: Value::Int(v), p: 1.0 / (hi - lo) as f64 });
        v
    }
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32 {
        let v = self.inner.sample(site, values);
        let n = values.iter().filter(|x| **x == v).count();
        self.events.push(Event { site: *site, value: Value::Int(v), p: n as f64 / values.len() as f64 });
        v
    }
    fn tie(&mut self, site: &Site, a: u8, b: u8) -> u8 {
        let v = self.inner.tie(site, a, b);
        self.events.push(Event { site: *site, value: Value::Side(v), p: 0.5 });
        v
    }
    fn tie_effects(&mut self, site: &Site, a: EffectRef, b: EffectRef) -> bool {
        let v = self.inner.tie_effects(site, a, b);
        self.events.push(Event { site: *site, value: Value::Effect(if v { a } else { b }), p: 0.5 });
        v
    }
    fn sample_str(&mut self, site: &Site, values: &[&'static str]) -> usize {
        let i = self.inner.sample_str(site, values);
        let n = values.iter().filter(|x| **x == values[i]).count();
        self.events.push(Event { site: *site, value: Value::Str(values[i]), p: n as f64 / values.len() as f64 });
        i
    }
    fn error(&self) -> Option<&str> {
        self.inner.error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_is_deterministic_and_in_range() {
        let mut a = Seeded::new(7);
        let mut b = Seeded::new(7);
        let site = Site::new(Key::Roll, Some(0));
        for _ in 0..1000 {
            let x = a.range(&site, 0, 16);
            assert_eq!(x, b.range(&site, 0, 16));
            assert!((0..16).contains(&x));
        }
        let mut n = 0;
        for _ in 0..24000 {
            n += a.chance(&site, 1, 24) as u32;
        }
        assert!((800..1200).contains(&n), "{}", n);
    }

    #[test]
    fn forced_matches_and_reports_leftovers() {
        let tape = vec![
            TapeEvent { key: Key::Roll, side: Some(0), mv: None, hit: Some(0), index: None, effect: None, value: TapeValue::Int(3) },
            TapeEvent { key: Key::Crit, side: Some(1), mv: None, hit: None, index: None, effect: None, value: TapeValue::Bool(true) },
        ];
        let roll = Site { hit: Some(0), ..Site::new(Key::Roll, Some(0)) };
        let mut f = Forced::new(tape.clone());
        assert_eq!(f.range(&roll, 0, 16), 3);
        assert!(f.finish().is_err());
        let mut f = Forced::new(tape.clone());
        assert!(f.chance(&Site::new(Key::Crit, Some(1)), 1, 24));
        assert_eq!(f.range(&roll, 0, 16), 3);
        assert!(f.finish().is_ok());
        let mut f = Forced::new(tape);
        f.range(&Site::new(Key::Roll, Some(0)), 0, 16);
        assert!(f.error().is_some(), "hit missing on the needed event");
        let mut f = Forced::new(vec![]);
        f.chance(&Site::new(Key::Crit, Some(1)), 1, 24);
        assert!(f.finish().is_err());
    }
}
