//! Every outcome of a step with its probability. The step is replayed over the tree of its random events:
//! each replay answers a forced prefix of alternatives and takes the first alternative at every event past
//! it, then the next prefix is the deepest event on that path with an untried alternative (depth first).
//! A leaf's probability is the product of its events' probabilities.

use cg_engine::sim::outcome::{EffectRef, Event, Outcomes, Site, Value};
use cg_engine::sim::state::{State, SIDES, TEAM};
use cg_engine::sim::Choice;
use std::collections::HashMap;

/// One event on a replay's path: the alternative taken and how many there were.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub pick: u16,
    pub arity: u16,
}

/// The outcome source of one replay. Alternatives per call, in order:
/// `chance`: true, false; `range`: lo..hi ascending; `sample` / `sample_str`: the distinct values in order of
/// first appearance; `tie`: a, b; `tie_effects`: a first, b first.
#[derive(Clone, Debug, Default)]
pub struct Replay {
    prefix: Vec<u16>,
    pos: usize,
    pub path: Vec<Node>,
    /// the product of the events' probabilities so far
    pub p: f64,
    pub keep_events: bool,
    pub events: Vec<Event>,
    /// a prefix entry was out of range for its event
    pub bad: Option<String>,
}

impl Replay {
    pub fn new(prefix: Vec<u16>, keep_events: bool) -> Replay {
        Replay { prefix, pos: 0, path: Vec::new(), p: 1.0, keep_events, events: Vec::new(), bad: None }
    }

    /// The prefix this replay was forced with; on success `path` starts with it.
    pub fn prefix(&self) -> &[u16] {
        &self.prefix
    }

    /// Whether every forced entry was used.
    pub fn consumed(&self) -> bool {
        self.pos >= self.prefix.len()
    }

    fn pick(&mut self, arity: u16) -> u16 {
        let k = if self.pos < self.prefix.len() {
            let k = self.prefix[self.pos];
            if k >= arity && self.bad.is_none() {
                self.bad = Some(format!("forced alternative {} at event {} of {}", k, self.pos, arity));
            }
            k.min(arity - 1)
        } else {
            0
        };
        self.pos += 1;
        self.path.push(Node { pick: k, arity });
        k
    }

    fn note(&mut self, site: &Site, value: Value, p: f64) {
        self.p *= p;
        if self.keep_events {
            self.events.push(Event { site: *site, value, p });
        }
    }
}

/// the k-th distinct entry of `values` (first-appearance order), its first index and its count
fn distinct_at<T: PartialEq + Copy>(values: &[T], k: usize) -> (usize, usize) {
    let mut seen = 0;
    for i in 0..values.len() {
        if values[..i].contains(&values[i]) {
            continue;
        }
        if seen == k {
            let n = values.iter().filter(|x| **x == values[i]).count();
            return (i, n);
        }
        seen += 1;
    }
    unreachable!("distinct index out of range")
}

fn distinct_count<T: PartialEq>(values: &[T]) -> usize {
    (0..values.len()).filter(|&i| !values[..i].contains(&values[i])).count()
}

impl Outcomes for Replay {
    fn chance(&mut self, site: &Site, num: u32, den: u32) -> bool {
        let t = num as f64 / den as f64;
        if num == 0 || num >= den {
            return num > 0;
        }
        let v = self.pick(2) == 0;
        self.note(site, Value::Bool(v), if v { t } else { 1.0 - t });
        v
    }
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32 {
        if hi - lo <= 1 {
            return lo;
        }
        let v = lo + self.pick((hi - lo) as u16) as i32;
        self.note(site, Value::Int(v), 1.0 / (hi - lo) as f64);
        v
    }
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32 {
        let d = distinct_count(values);
        if d <= 1 {
            return values[0];
        }
        let k = self.pick(d as u16) as usize;
        let (i, n) = distinct_at(values, k);
        self.note(site, Value::Int(values[i]), n as f64 / values.len() as f64);
        values[i]
    }
    fn tie(&mut self, site: &Site, a: u8, b: u8) -> u8 {
        let v = if self.pick(2) == 0 { a } else { b };
        self.note(site, Value::Side(v), 0.5);
        v
    }
    fn tie_effects(&mut self, site: &Site, a: EffectRef, b: EffectRef) -> bool {
        let v = self.pick(2) == 0;
        self.note(site, Value::Effect(if v { a } else { b }), 0.5);
        v
    }
    fn sample_str(&mut self, site: &Site, values: &[&'static str]) -> usize {
        let d = distinct_count(values);
        if d <= 1 {
            return 0;
        }
        let k = self.pick(d as u16) as usize;
        let (i, n) = distinct_at(values, k);
        self.note(site, Value::Str(values[i]), n as f64 / values.len() as f64);
        i
    }
    fn error(&self) -> Option<&str> {
        self.bad.as_deref()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EnumOptions {
    /// merge outcomes whose resulting States are equal (probabilities add; the first path is kept)
    pub merge: bool,
    /// stop after this many replays (leaves); the result says whether it stopped early
    pub max_branches: usize,
    /// keep each outcome's events (site, value, probability), for a Forced tape
    pub keep_events: bool,
}

impl Default for EnumOptions {
    fn default() -> Self {
        EnumOptions { merge: true, max_branches: 100_000, keep_events: false }
    }
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub p: f64,
    pub state: State,
    /// alternatives taken, one per random event: force with `Replay::new(path, _)`
    pub path: Vec<u16>,
    pub events: Vec<Event>,
    /// how many leaves were merged into this outcome
    pub leaves: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Enumeration {
    pub outcomes: Vec<Outcome>,
    /// replays run
    pub leaves: usize,
    /// the branch cap stopped the walk before every leaf was seen
    pub capped: bool,
    /// the probability of the outcomes listed (1 when not capped)
    pub covered: f64,
    /// the longest path (events in one replay)
    pub max_depth: usize,
}

impl Enumeration {
    /// The outcome whose State equals `s`, if any (meaningful when merged).
    pub fn find(&self, s: &State) -> Option<usize> {
        self.outcomes.iter().position(|o| o.state == *s)
    }
}

/// A cheap hash of the parts of a State that differ between outcomes, to bucket equality checks.
pub fn fingerprint(s: &State) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut put = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    };
    put(s.turn as u64 | (s.ended as u64) << 16 | (s.mid_turn as u64) << 17 | (s.queue.len as u64) << 18);
    put(s.field.weather.id.0 as u64 | (s.field.terrain.id.0 as u64) << 16 | (s.field.npseudo as u64) << 32);
    for sd in &s.sides {
        put(sd.active as u64 | (sd.nvolatiles as u64) << 8 | (sd.nconds as u64) << 16 | (sd.nslot_conds as u64) << 24 | (sd.pokemon_left as u64) << 32);
        for v in sd.volatiles() {
            put(v.id.0 as u64 | (v.duration as u8 as u64) << 16 | (v.data as u8 as u64) << 24 | (v.hp as u16 as u64) << 32);
        }
        for m in &sd.mons {
            put(m.hp as u64 | (m.status as u64) << 16 | (m.status_time as u8 as u64) << 24 | (m.species.0 as u64) << 32);
            let mut b = 0u64;
            for x in m.boosts {
                b = b << 8 | x as u8 as u64;
            }
            put(b);
            put(m.moves[0].pp as u64 | (m.moves[1].pp as u64) << 8 | (m.moves[2].pp as u64) << 16 | (m.moves[3].pp as u64) << 24);
        }
    }
    h
}

/// Enumerates the outcomes of `run`, a function that plays something from a fixed start with the given
/// source and returns the resulting State. `run` must be deterministic given the source's answers.
pub fn enumerate_with<F>(opts: &EnumOptions, mut run: F) -> Result<Enumeration, String>
where
    F: FnMut(&mut Replay) -> Result<State, String>,
{
    let mut out = Enumeration::default();
    let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut prefix: Vec<u16> = Vec::new();
    let mut total = 0.0f64;
    loop {
        let mut r = Replay::new(std::mem::take(&mut prefix), opts.keep_events);
        let st = run(&mut r)?;
        if let Some(e) = r.bad.take() {
            return Err(format!("enumerate: {}", e));
        }
        out.leaves += 1;
        out.max_depth = out.max_depth.max(r.path.len());
        total += r.p;
        let merged = if opts.merge {
            let fp = fingerprint(&st);
            let b = buckets.entry(fp).or_default();
            match b.iter().copied().find(|&i| out.outcomes[i].state == st) {
                Some(i) => {
                    out.outcomes[i].p += r.p;
                    out.outcomes[i].leaves += 1;
                    true
                }
                None => {
                    b.push(out.outcomes.len());
                    false
                }
            }
        } else {
            false
        };
        if !merged {
            out.outcomes.push(Outcome {
                p: r.p,
                state: st,
                path: r.path.iter().map(|n| n.pick).collect(),
                events: std::mem::take(&mut r.events),
                leaves: 1,
            });
        }
        // the next prefix: the deepest event with an untried alternative
        let mut i = r.path.len();
        let next = loop {
            if i == 0 {
                break None;
            }
            i -= 1;
            if r.path[i].pick + 1 < r.path[i].arity {
                break Some(i);
            }
        };
        let Some(i) = next else { break };
        if out.leaves >= opts.max_branches {
            out.capped = true;
            break;
        }
        prefix = r.path[..i].iter().map(|n| n.pick).collect();
        prefix.push(r.path[i].pick + 1);
    }
    out.covered = total;
    Ok(out)
}

/// Every outcome of one step of `state` with both sides' choices.
pub fn enumerate_step(state: &State, choices: [Option<Choice>; SIDES], opts: &EnumOptions) -> Result<Enumeration, String> {
    enumerate_with(opts, |r| {
        let mut s = *state;
        s.step(choices, r)?;
        Ok(s)
    })
}

/// Every outcome of a battle start (team preview resolved, leads in).
pub fn enumerate_start(teams: [&[cg_engine::sim::MonSpec]; SIDES], picks: [[u8; TEAM]; SIDES], opts: &EnumOptions) -> Result<Enumeration, String> {
    enumerate_with(opts, |r| State::start(teams, picks, r))
}

/// A seeded source that also records the path `Replay` would need to reproduce each draw, so a sampled
/// outcome can be forced again (`Game::step_forced`). Draws have the engine's probabilities.
#[derive(Clone, Debug)]
pub struct Sampler {
    rng: cg_engine::sim::outcome::Seeded,
    pub path: Vec<u16>,
}

impl Sampler {
    pub fn new(seed: u64) -> Sampler {
        Sampler { rng: cg_engine::sim::outcome::Seeded::new(seed), path: Vec::new() }
    }

    fn below(&mut self, site: &Site, n: usize) -> usize {
        self.rng.range(site, 0, n as i32) as usize
    }
}

impl Outcomes for Sampler {
    fn chance(&mut self, site: &Site, num: u32, den: u32) -> bool {
        if num == 0 || num >= den {
            return num > 0;
        }
        let v = self.rng.chance(site, num, den);
        self.path.push(if v { 0 } else { 1 });
        v
    }
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32 {
        if hi - lo <= 1 {
            return lo;
        }
        let v = self.rng.range(site, lo, hi);
        self.path.push((v - lo) as u16);
        v
    }
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32 {
        let d = distinct_count(values);
        if d <= 1 {
            return values[0];
        }
        let i = self.below(site, values.len());
        let k = distinct_index(values, values[i]).expect("value present");
        self.path.push(k as u16);
        values[distinct_at(values, k).0]
    }
    fn tie(&mut self, site: &Site, a: u8, b: u8) -> u8 {
        // the first alternative is `a`, as in `Replay`, even when `a == b`
        let first = self.below(site, 2) == 0;
        self.path.push(if first { 0 } else { 1 });
        if first {
            a
        } else {
            b
        }
    }
    fn tie_effects(&mut self, site: &Site, a: EffectRef, b: EffectRef) -> bool {
        let v = self.rng.tie_effects(site, a, b);
        self.path.push(if v { 0 } else { 1 });
        v
    }
    fn sample_str(&mut self, site: &Site, values: &[&'static str]) -> usize {
        let d = distinct_count(values);
        if d <= 1 {
            return 0;
        }
        let i = self.below(site, values.len());
        let k = distinct_index(values, values[i]).expect("value present");
        self.path.push(k as u16);
        distinct_at(values, k).0
    }
}

/// The position of `v` among the distinct entries of `values` (first-appearance order).
fn distinct_index<T: PartialEq + Copy>(values: &[T], v: T) -> Option<usize> {
    let mut k = 0;
    for i in 0..values.len() {
        if values[..i].contains(&values[i]) {
            continue;
        }
        if values[i] == v {
            return Some(k);
        }
        k += 1;
    }
    None
}

/// `n` independent outcomes of one step of `state`, drawn with the engine's probabilities from seeds
/// `mix(seed, k)`. Each outcome has weight 1/n; with `merge`, draws with equal States are one outcome
/// whose weight adds up and whose `leaves` counts the draws (the first draw's path is kept).
pub fn sample_step(state: &State, choices: [Option<Choice>; SIDES], n: usize, seed: u64, merge: bool) -> Result<Enumeration, String> {
    let mut out = Enumeration { covered: 1.0, ..Enumeration::default() };
    let mut buckets: HashMap<u64, Vec<usize>> = HashMap::new();
    let w = 1.0 / n.max(1) as f64;
    for k in 0..n {
        let mut r = Sampler::new(crate::teams::mix(seed, k as u64));
        let mut s = *state;
        s.step(choices, &mut r)?;
        out.leaves += 1;
        out.max_depth = out.max_depth.max(r.path.len());
        if merge {
            let b = buckets.entry(fingerprint(&s)).or_default();
            if let Some(&i) = b.iter().find(|&&i| out.outcomes[i].state == s) {
                out.outcomes[i].p += w;
                out.outcomes[i].leaves += 1;
                continue;
            }
            b.push(out.outcomes.len());
        }
        out.outcomes.push(Outcome { p: w, state: s, path: r.path, events: Vec::new(), leaves: 1 });
    }
    Ok(out)
}
