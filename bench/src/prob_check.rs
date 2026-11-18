//! Probability check of the resolver's random events (R11).
//!
//! For each class of random event a prepared situation is played many times with the seeded source (one
//! seed per trial); the outcome counts are tested against Showdown's distribution with a chi-square test.
//! The same situation is enumerated with an enumerating source (every branch of every draw, exact
//! rational probabilities) and the exact distribution is compared with the reference. The seeded generator
//! is checked on its own, and whole random games give coarse aggregates.
//!
//! Usage: prob_check [--out FILE] [--trials N] [--only a,b] [--tables FILE] [--sd FILE]
//!                   [--games N] [--games-out FILE] [--teams FILE] [--no-gen]
//! `--sd` reads Showdown's measured counts (tools/prob/sd_prob.js) and adds a two-sample test per situation.

use cg_engine::sim::data::{abilities, conds, items, moves, AbilityId, ItemId, MoveId, Ohko, SpeciesId, Status, BOOST_IDS, MOVES};
use cg_engine::sim::log::{Fx, ItemWhat, Row};
use cg_engine::sim::outcome::{EffectRef, Event, Key, Outcomes, Recording, Seeded, Site, Value};
use cg_engine::sim::state::{State, Winner};
use cg_engine::sim::{Choice, Kind, MonSpec};
use rig::json::Json;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Instant;

// --------------------------------------------------------------------------------------------- rationals

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rat {
    n: u128,
    d: u128,
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Rat {
    fn new(n: u128, d: u128) -> Rat {
        let g = gcd(n, d).max(1);
        Rat { n: n / g, d: d / g }
    }
    fn one() -> Rat {
        Rat { n: 1, d: 1 }
    }
    fn zero() -> Rat {
        Rat { n: 0, d: 1 }
    }
    fn mul(self, o: Rat) -> Option<Rat> {
        let g1 = gcd(self.n, o.d).max(1);
        let g2 = gcd(o.n, self.d).max(1);
        let n = (self.n / g1).checked_mul(o.n / g2)?;
        let d = (self.d / g2).checked_mul(o.d / g1)?;
        Some(Rat::new(n, d))
    }
    fn add(self, o: Rat) -> Option<Rat> {
        let g = gcd(self.d, o.d).max(1);
        let l = (self.d / g).checked_mul(o.d)?;
        let n = self.n.checked_mul(l / self.d)?.checked_add(o.n.checked_mul(l / o.d)?)?;
        Some(Rat::new(n, l))
    }
    fn f(self) -> f64 {
        self.n as f64 / self.d as f64
    }
    fn s(self) -> String {
        format!("{}/{}", self.n, self.d)
    }
}

// --------------------------------------------------------------------------------------------- statistics

fn ln_gamma(x: f64) -> f64 {
    let g = [76.18009172947146, -86.50532032941677, 24.01409824083091, -1.231739572450155, 0.1208650973866179e-2, -0.5395239384953e-5];
    let mut y = x;
    let tmp = x + 5.5;
    let tmp = tmp - (x + 0.5) * tmp.ln();
    let mut ser = 1.000000000190015;
    for c in g {
        y += 1.0;
        ser += c / y;
    }
    -tmp + (2.5066282746310005 * ser / x).ln()
}

/// upper regularized incomplete gamma Q(a, x)
fn gamma_q(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        let (mut sum, mut del, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..10000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        1.0 - sum * (-x + a * x.ln() - ln_gamma(a)).exp()
    } else {
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / 1e-300;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..10000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < 1e-300 {
                d = 1e-300;
            }
            c = b + an / c;
            if c.abs() < 1e-300 {
                c = 1e-300;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-15 {
                break;
            }
        }
        (-x + a * x.ln() - ln_gamma(a)).exp() * h
    }
}

fn chi2_p(chi2: f64, df: usize) -> f64 {
    if df == 0 {
        return 1.0;
    }
    gamma_q(df as f64 / 2.0, chi2 / 2.0)
}

/// goodness of fit: counts against probabilities. A count on a label of probability 0 fails outright (p 0).
fn gof(counts: &BTreeMap<String, u64>, reference: &[(String, Rat)]) -> (f64, usize, f64) {
    let n: u64 = counts.values().sum();
    let mut chi2 = 0.0;
    let mut df = 0usize;
    for (k, c) in counts {
        let p = reference.iter().find(|(l, _)| l == k).map_or(0.0, |(_, r)| r.f());
        if p == 0.0 && *c > 0 {
            return (f64::INFINITY, 0, 0.0);
        }
    }
    for (l, r) in reference {
        let e = r.f() * n as f64;
        if e <= 0.0 {
            continue;
        }
        let o = *counts.get(l).unwrap_or(&0) as f64;
        chi2 += (o - e) * (o - e) / e;
        df += 1;
    }
    let df = df.saturating_sub(1);
    (chi2, df, chi2_p(chi2, df))
}

/// two-sample homogeneity test (2 x k contingency)
fn two_sample(a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>) -> (f64, usize, f64) {
    let na: u64 = a.values().sum();
    let nb: u64 = b.values().sum();
    let n = (na + nb) as f64;
    let mut labels: Vec<&String> = a.keys().chain(b.keys()).collect();
    labels.sort();
    labels.dedup();
    let mut chi2 = 0.0;
    let mut k = 0usize;
    for l in labels {
        let (oa, ob) = (*a.get(l).unwrap_or(&0) as f64, *b.get(l).unwrap_or(&0) as f64);
        let col = oa + ob;
        if col == 0.0 {
            continue;
        }
        k += 1;
        let (ea, eb) = (col * na as f64 / n, col * nb as f64 / n);
        chi2 += (oa - ea).powi(2) / ea + (ob - eb).powi(2) / eb;
    }
    let df = (k as usize).saturating_sub(1);
    (chi2, df, chi2_p(chi2, df))
}

// --------------------------------------------------------------------------------------------- enumerating source

/// Answers each draw by a path of alternative indices and extends the path with the first alternative;
/// a depth-first walk over paths visits every outcome once, each with its exact probability.
struct Enum {
    path: Vec<usize>,
    arity: Vec<usize>,
    pos: usize,
    p: Rat,
    /// keys answered with a fixed value at probability 1 (draws the situation's label does not depend on)
    fix: Vec<Key>,
    overflow: bool,
}

impl Enum {
    fn new(path: Vec<usize>, fix: Vec<Key>) -> Enum {
        Enum { path, arity: Vec::new(), pos: 0, p: Rat::one(), fix, overflow: false }
    }
    fn pick(&mut self, key: Key, w: &[Rat]) -> Option<usize> {
        if self.fix.contains(&key) {
            return None;
        }
        let i = if self.pos < self.path.len() {
            self.path[self.pos]
        } else {
            self.path.push(0);
            0
        };
        self.arity.push(w.len());
        let i = i.min(w.len() - 1);
        match self.p.mul(w[i]) {
            Some(p) => self.p = p,
            None => self.overflow = true,
        }
        self.pos += 1;
        Some(i)
    }
}

fn distinct_i32(values: &[i32]) -> (Vec<i32>, Vec<Rat>) {
    let mut v: Vec<i32> = Vec::new();
    let mut c: Vec<u128> = Vec::new();
    for x in values {
        match v.iter().position(|y| y == x) {
            Some(i) => c[i] += 1,
            None => {
                v.push(*x);
                c.push(1);
            }
        }
    }
    let n = values.len() as u128;
    (v, c.into_iter().map(|k| Rat::new(k, n)).collect())
}

impl Outcomes for Enum {
    fn chance(&mut self, site: &Site, num: u32, den: u32) -> bool {
        let w = [Rat::new(num as u128, den as u128), Rat::new((den - num) as u128, den as u128)];
        match self.pick(site.key, &w) {
            None => false,
            Some(i) => i == 0,
        }
    }
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32 {
        let n = (hi - lo) as u128;
        let w = vec![Rat::new(1, n); n as usize];
        match self.pick(site.key, &w) {
            None => lo,
            Some(i) => lo + i as i32,
        }
    }
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32 {
        let (v, w) = distinct_i32(values);
        match self.pick(site.key, &w) {
            None => values[0],
            Some(i) => v[i],
        }
    }
    fn tie(&mut self, site: &Site, a: u8, b: u8) -> u8 {
        let w = [Rat::new(1, 2), Rat::new(1, 2)];
        match self.pick(site.key, &w) {
            Some(1) => b,
            _ => a,
        }
    }
    fn tie_effects(&mut self, site: &Site, _: EffectRef, _: EffectRef) -> bool {
        let w = [Rat::new(1, 2), Rat::new(1, 2)];
        !matches!(self.pick(site.key, &w), Some(1))
    }
    fn sample_str(&mut self, site: &Site, values: &[&'static str]) -> usize {
        let mut v: Vec<&str> = Vec::new();
        let mut first: Vec<usize> = Vec::new();
        let mut c: Vec<u128> = Vec::new();
        for (i, x) in values.iter().enumerate() {
            match v.iter().position(|y| y == x) {
                Some(j) => c[j] += 1,
                None => {
                    v.push(x);
                    first.push(i);
                    c.push(1);
                }
            }
        }
        let n = values.len() as u128;
        let w: Vec<Rat> = c.into_iter().map(|k| Rat::new(k, n)).collect();
        match self.pick(site.key, &w) {
            None => 0,
            Some(i) => first[i],
        }
    }
}

// --------------------------------------------------------------------------------------------- situations

fn set(species: &str, ability: &str, item: &str, mv: &[&str]) -> MonSpec {
    let sp = SpeciesId::from_id(species).unwrap_or_else(|| panic!("species {}", species));
    let ab = AbilityId::from_id(ability).unwrap_or_else(|| panic!("ability {}", ability));
    let it = if item.is_empty() { ItemId::NONE } else { ItemId::from_id(item).unwrap_or_else(|| panic!("item {}", item)) };
    let mut m = [MoveId::NONE; 4];
    for (i, x) in mv.iter().enumerate() {
        m[i] = MoveId::from_id(x).unwrap_or_else(|| panic!("move {}", x));
    }
    let gender = match sp.data().gender {
        "F" => 2,
        "N" => 0,
        _ => 1,
    };
    MonSpec { species: sp, ability: ab, item: it, moves: m, nmoves: mv.len() as u8, nature: 0, evs: [0; 6], gender }
}

fn fem(mut m: MonSpec) -> MonSpec {
    m.gender = 2;
    m
}

/// a brought team: lead, two benched, three never brought
fn team(lead: MonSpec, side: usize) -> Vec<MonSpec> {
    let sd = ["swordsdance"];
    let (b1, b2) = if side == 0 {
        (set("skarmory", "sturdy", "", &sd), set("dragonite", "innerfocus", "", &sd))
    } else {
        (set("corviknight", "pressure", "", &sd), set("umbreon", "synchronize", "", &sd))
    };
    vec![lead, b1, b2, set("slowbro", "owntempo", "", &sd), set("toxapex", "regenerator", "", &sd), set("clefable", "magicguard", "", &sd)]
}

struct Ctx<'a> {
    st: &'a State,
    steps: &'a [Vec<Row>],
    ev: &'a [Event],
}

impl<'a> Ctx<'a> {
    fn rows(&self) -> impl Iterator<Item = &Row> {
        self.steps.iter().flatten()
    }
    fn ev(&self, key: Key, side: u8) -> Option<&Event> {
        self.ev.iter().find(|e| e.site.key == key && e.site.side == Some(side))
    }
    fn missed(&self, by: u8) -> bool {
        self.rows().any(|r| matches!(r, Row::Miss { by: b, .. } if b.0 == by))
    }
    fn hits(&self) -> Option<u8> {
        self.rows().find_map(|r| if let Row::Hits { n, .. } = r { Some(*n) } else { None })
    }
    fn cant(&self, side: u8, why: Fx) -> usize {
        self.rows().filter(|r| matches!(r, Row::Cant { p, why: w, .. } if p.0 == side && *w == why)).count()
    }
    fn status(&self, side: u8) -> Option<Status> {
        self.rows().find_map(|r| if let Row::Status { p, st, .. } = r { if p.0 == side { Some(*st) } else { None } } else { None })
    }
    fn vol(&self, side: u8, c: cg_engine::sim::data::Cond) -> bool {
        self.rows().any(|r| matches!(r, Row::Volatile { p, c: x, .. } if p.0 == side && *x == c))
    }
}

fn yes(b: bool, a: &str, n: &str) -> String {
    if b { a.into() } else { n.into() }
}

fn st_name(s: Option<Status>) -> String {
    match s {
        None | Some(Status::None) => "none",
        Some(Status::Brn) => "brn",
        Some(Status::Par) => "par",
        Some(Status::Slp) => "slp",
        Some(Status::Frz) => "frz",
        Some(Status::Psn) => "psn",
        Some(Status::Tox) => "tox",
    }
    .into()
}

type Classify = fn(&Ctx) -> String;
type Setup = fn(&mut State);

struct Sc {
    name: &'static str,
    class: &'static str,
    teams: [Vec<MonSpec>; 2],
    setup: Setup,
    /// move index per side per step
    turns: Vec<[u8; 2]>,
    classify: Classify,
    reference: Vec<(String, Rat)>,
    fix: Vec<Key>,
    /// the trial starts at team preview (the leads' entry is part of it)
    from_start: bool,
    eng: &'static str,
    sd: &'static str,
}

fn sc(name: &'static str, class: &'static str) -> Sc {
    Sc {
        name,
        class,
        teams: [
            team(set("garchomp", "roughskin", "", &["dragonclaw"]), 0),
            team(set("snorlax", "thickfat", "", &["swordsdance"]), 1),
        ],
        setup: |_| {},
        turns: vec![[0, 0]],
        classify: |_| "?".into(),
        reference: Vec::new(),
        fix: vec![Key::Roll, Key::Crit],
        from_start: false,
        eng: "",
        sd: "",
    }
}

impl Sc {
    fn p1(mut self, m: MonSpec) -> Sc {
        self.teams[0][0] = m;
        self
    }
    fn p2(mut self, m: MonSpec) -> Sc {
        self.teams[1][0] = m;
        self
    }
    fn turns(mut self, t: &[[u8; 2]]) -> Sc {
        self.turns = t.to_vec();
        self
    }
    fn n(mut self, k: usize) -> Sc {
        self.turns = vec![[0, 0]; k];
        self
    }
    fn setup(mut self, f: Setup) -> Sc {
        self.setup = f;
        self
    }
    fn cls(mut self, f: Classify) -> Sc {
        self.classify = f;
        self
    }
    fn r(mut self, refs: &[(&str, u128, u128)]) -> Sc {
        self.reference = refs.iter().map(|(l, a, b)| (l.to_string(), Rat::new(*a, *b))).collect();
        self
    }
    fn rr(mut self, refs: Vec<(String, Rat)>) -> Sc {
        self.reference = refs;
        self
    }
    fn fix(mut self, k: &[Key]) -> Sc {
        self.fix = k.to_vec();
        self
    }
    fn refs(mut self, eng: &'static str, sd: &'static str) -> Sc {
        self.eng = eng;
        self.sd = sd;
        self
    }
    fn start(mut self) -> Sc {
        self.from_start = true;
        self
    }
}

const PICKS: [[u8; 3]; 2] = [[0, 1, 2], [0, 1, 2]];

fn prepare(sc: &Sc) -> State {
    let mut st = State::start([&sc.teams[0], &sc.teams[1]], PICKS, &mut Seeded::new(0x5eed)).expect("start");
    (sc.setup)(&mut st);
    st
}

fn run<O: Outcomes>(sc: &Sc, start: &State, o: O) -> (String, O) {
    let mut rec = Recording::new(o);
    let mut steps: Vec<Vec<Row>> = Vec::new();
    let mut st = if sc.from_start {
        let mut rows = Vec::new();
        let st = State::start_logged([&sc.teams[0], &sc.teams[1]], PICKS, &mut rec, &mut rows).expect("start");
        steps.push(rows);
        st
    } else {
        *start
    };
    let mut err = None;
    for t in &sc.turns {
        if st.ended {
            break;
        }
        let opts = st.options();
        let mut ch = [None, None];
        for s in 0..2 {
            if matches!(opts[s].kind, Kind::Turn | Kind::Replace) {
                let want = Choice::Move { index: t[s], mega: false };
                ch[s] = Some(if opts[s].allows(want) { want } else { opts[s].choices()[0] });
            }
        }
        let mut rows = Vec::new();
        if let Err(e) = st.step_logged(ch, &mut rec, &mut rows) {
            err = Some(e);
            break;
        }
        steps.push(rows);
    }
    let label = match err {
        Some(e) => format!("error:{}", e),
        None => (sc.classify)(&Ctx { st: &st, steps: &steps, ev: &rec.events }),
    };
    if DUMP.load(std::sync::atomic::Ordering::Relaxed) {
        for (i, rows) in steps.iter().enumerate() {
            eprintln!("-- step {}", i);
            for r in rows {
                eprintln!("   {:?}", r);
            }
        }
        for e in &rec.events {
            eprintln!("   event {:?} = {:?} p={}", e.site.key, e.value, e.p);
        }
        eprintln!("   label {}", label);
    }
    (label, rec.inner)
}

const ENUM_CAP: usize = 200_000;
static DUMP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn enumerate(sc: &Sc, start: &State) -> Result<(usize, Vec<(String, Rat)>), String> {
    let mut path: Vec<usize> = Vec::new();
    let mut dist: Vec<(String, Rat)> = Vec::new();
    let mut leaves = 0usize;
    loop {
        let (label, e) = run(sc, start, Enum::new(path.clone(), sc.fix.clone()));
        if e.overflow {
            return Err("rational overflow".into());
        }
        if e.pos < path.len() {
            return Err("a replayed path used fewer draws than before".into());
        }
        leaves += 1;
        if leaves > ENUM_CAP {
            return Err(format!("more than {} outcomes", ENUM_CAP));
        }
        match dist.iter_mut().find(|(l, _)| *l == label) {
            Some((_, p)) => *p = p.add(e.p).ok_or("rational overflow")?,
            None => dist.push((label, e.p)),
        }
        path = e.path;
        path.truncate(e.pos);
        let arity = e.arity;
        loop {
            let Some(j) = path.len().checked_sub(1) else {
                dist.sort_by(|a, b| a.0.cmp(&b.0));
                return Ok((leaves, dist));
            };
            if path[j] + 1 < arity[j] {
                path[j] += 1;
                break;
            }
            path.pop();
        }
    }
}

fn mon_boost(st: &mut State, side: usize, i: usize, v: i8) {
    st.sides[side].active_mon_mut().boosts[i] = v;
}

fn all_hits(n: usize, acc: Rat, per: Rat) -> Vec<(String, Rat)> {
    // first hit through hitStepAccuracy, later hits each `per` (multiaccuracy)
    let miss = Rat::new(acc.d - acc.n, acc.d);
    let fail = Rat::new(per.d - per.n, per.d);
    let mut out = vec![("0".to_string(), miss)];
    let mut p = acc;
    for k in 1..n {
        out.push((k.to_string(), p.mul(fail).unwrap()));
        p = p.mul(per).unwrap();
    }
    out.push((n.to_string(), p));
    out
}

fn situations() -> Vec<Sc> {
    let mut v = Vec::new();
    let fb = |ab: &str, it: &str| set("garchomp", ab, it, &["focusblast"]);
    let acc_cls: Classify = |c| yes(c.missed(0), "miss", "hit");
    let accs = "moves.rs:960 (hitStepAccuracy), calc.rs:60 (stages)";
    let sda = "sim/battle-actions.ts:774-800";
    // accuracy
    v.push(sc("acc70", "accuracy").p1(fb("roughskin", "")).cls(acc_cls).r(&[("hit", 70, 100), ("miss", 30, 100)]).refs(accs, sda));
    v.push(sc("acc70_accm1", "accuracy").p1(fb("roughskin", "")).setup(|s| mon_boost(s, 0, 5, -1)).cls(acc_cls).r(&[("hit", 52, 100), ("miss", 48, 100)]).refs(accs, "sim/battle-actions.ts:788 trunc(70*3/4)=52"));
    v.push(sc("acc70_accp1", "accuracy").p1(fb("roughskin", "")).setup(|s| mon_boost(s, 0, 5, 1)).cls(acc_cls).r(&[("hit", 93, 100), ("miss", 7, 100)]).refs(accs, "sim/battle-actions.ts:786 trunc(70*4/3)=93"));
    v.push(sc("acc70_evap6", "accuracy").p1(fb("roughskin", "")).setup(|s| mon_boost(s, 1, 6, 6)).cls(acc_cls).r(&[("hit", 23, 100), ("miss", 77, 100)]).refs(accs, "sim/battle-actions.ts:788 trunc(70*3/9)=23"));
    v.push(sc("acc70_accp1_evap2", "accuracy").p1(fb("roughskin", "")).setup(|s| {
        mon_boost(s, 0, 5, 1);
        mon_boost(s, 1, 6, 2)
    }).cls(acc_cls).r(&[("hit", 52, 100), ("miss", 48, 100)]).refs(accs, "sim/battle-actions.ts:782 net stage -1"));
    v.push(sc("acc_widelens", "accuracy").p1(fb("roughskin", "widelens")).cls(acc_cls).r(&[("hit", 77, 100), ("miss", 23, 100)]).refs("items.rs (ModifyAccuracy) + moves.rs:960", "data/items.ts widelens chainModify([4505,4096]): 70 -> 77"));
    v.push(sc("acc_brightpowder", "accuracy").p1(fb("roughskin", "")).p2(set("snorlax", "thickfat", "brightpowder", &["swordsdance"])).cls(acc_cls).r(&[("hit", 63, 100), ("miss", 37, 100)]).refs("items.rs (ModifyAccuracy) + moves.rs:960", "data/items.ts brightpowder chainModify([3686,4096]): 70 -> 63"));
    v.push(sc("acc_compoundeyes", "accuracy").p1(fb("compoundeyes", "")).cls(acc_cls).r(&[("hit", 91, 100), ("miss", 9, 100)]).refs("abilities.rs (ModifyAccuracy) + moves.rs:960", "data/abilities.ts compoundeyes chainModify([5325,4096]): 70 -> 91"));
    v.push(sc("acc_hustle", "accuracy").p1(set("garchomp", "hustle", "", &["stoneedge"])).cls(acc_cls).r(&[("hit", 64, 100), ("miss", 36, 100)]).refs("abilities.rs (ModifyAccuracy) + moves.rs:960", "data/abilities.ts hustle chainModify([3277,4096]) physical: 80 -> 64"));
    v.push(sc("acc_sandveil", "accuracy").p1(fb("sandstream", "")).p2(set("snorlax", "sandveil", "", &["swordsdance"])).cls(acc_cls).r(&[("hit", 56, 100), ("miss", 44, 100)]).refs("abilities.rs (ModifyAccuracy) + moves.rs:960", "data/abilities.ts sandveil chainModify([3277,4096]) in sand: 70 -> 56"));
    v.push(sc("acc_sheercold", "accuracy (OHKO)").p1(set("garchomp", "roughskin", "", &["sheercold"])).cls(acc_cls).r(&[("hit", 20, 100), ("miss", 80, 100)]).refs("moves.rs:903 (20 for a non-Ice user)", "sim/battle-actions.ts:760-766"));
    v.push(sc("acc_fissure", "accuracy (OHKO)").p1(set("garchomp", "roughskin", "", &["fissure"])).cls(acc_cls).r(&[("hit", 30, 100), ("miss", 70, 100)]).refs("moves.rs:902", "sim/battle-actions.ts:760"));
    v.push(sc("acc_thunder_sun", "accuracy (weather)").p1(set("garchomp", "drought", "", &["thunder"])).cls(acc_cls).r(&[("hit", 50, 100), ("miss", 50, 100)]).refs("movefx.rs (Thunder ModifyMove) + moves.rs:960", "data/moves.ts thunder onModifyMove: sun -> 50"));
    v.push(sc("acc_thunder_rain", "accuracy (weather)").p1(set("garchomp", "drizzle", "", &["thunder"])).cls(acc_cls).r(&[("hit", 1, 1)]).refs("movefx.rs (Thunder ModifyMove)", "data/moves.ts thunder onModifyMove: rain -> true (no draw)"));
    v.push(sc("acc_hurricane_sun", "accuracy (weather)").p1(set("garchomp", "drought", "", &["hurricane"])).cls(acc_cls).r(&[("hit", 50, 100), ("miss", 50, 100)]).refs("movefx.rs (Hurricane ModifyMove) + moves.rs:960", "data/moves.ts hurricane onModifyMove: sun -> 50"));
    let hits_cls: Classify = |c| if c.missed(0) { "0".into() } else { c.hits().map_or("1?".into(), |n| n.to_string()) };
    v.push(sc("tripleaxel", "multi-hit accuracy").p1(set("garchomp", "roughskin", "", &["tripleaxel"])).p2(set("skarmory", "sturdy", "", &["swordsdance"]))
        .cls(hits_cls).rr(all_hits(3, Rat::new(9, 10), Rat::new(9, 10))).refs("moves.rs:968-1010 (multi_accuracy)", "data/mods/champions/scripts.ts:493-507"));
    v.push(sc("popbomb", "multi-hit accuracy").p1(set("garchomp", "roughskin", "", &["populationbomb"])).p2(set("skarmory", "sturdy", "", &["swordsdance"]))
        .cls(hits_cls).rr(all_hits(10, Rat::new(9, 10), Rat::new(9, 10))).refs("moves.rs:968-1010", "data/mods/champions/scripts.ts:493-507"));
    v.push(sc("popbomb_skilllink", "multi-hit accuracy").p1(set("garchomp", "skilllink", "", &["populationbomb"])).p2(set("skarmory", "sturdy", "", &["swordsdance"]))
        .cls(hits_cls).r(&[("0", 1, 10), ("10", 9, 10)]).refs("abilities.rs:1449-1453", "data/abilities.ts skilllink onModifyMove (multiaccuracy deleted)"));
    // critical hits
    let crit_cls: Classify = |c| match c.ev(Key::Crit, 0) {
        Some(Event { value: Value::Bool(b), .. }) => yes(*b, "crit", "no"),
        _ => "noevent".into(),
    };
    let cr = "moves.rs:1597-1602 [0,24,8,2,1]";
    v.push(sc("crit_r1", "critical hit").cls(crit_cls).fix(&[Key::Roll]).r(&[("crit", 1, 24), ("no", 23, 24)]).refs(cr, "sim/battle-actions.ts:1678-1695 critMult [0,24,8,2,1]"));
    v.push(sc("crit_r2", "critical hit").p1(set("garchomp", "roughskin", "", &["nightslash"])).cls(crit_cls).fix(&[Key::Roll]).r(&[("crit", 1, 8), ("no", 7, 8)]).refs(cr, "sim/battle-actions.ts:1687 (critRatio 2)"));
    v.push(sc("crit_r3", "critical hit").p1(set("garchomp", "roughskin", "scopelens", &["nightslash"])).cls(crit_cls).fix(&[Key::Roll]).r(&[("crit", 1, 2), ("no", 1, 2)]).refs(cr, "sim/battle-actions.ts:1687 (critRatio 3: Scope Lens +1)"));
    v.push(sc("crit_r4", "critical hit").p1(set("garchomp", "superluck", "scopelens", &["nightslash"])).cls(crit_cls).fix(&[Key::Roll]).r(&[("noevent", 1, 1)]).refs(cr, "sim/battle-actions.ts:1687 (critRatio 4: 1/1, no draw)"));
    // damage roll
    let roll_cls: Classify = |c| match c.ev(Key::Roll, 0) {
        Some(Event { value: Value::Int(x), .. }) => x.to_string(),
        _ => "noevent".into(),
    };
    v.push(sc("roll", "damage roll").cls(roll_cls).fix(&[Key::Crit]).rr((0..16).map(|i| (i.to_string(), Rat::new(1, 16))).collect()).refs("moves.rs:1640 range(0,16)", "sim/battle.ts:2407-2409 random(16); champions scripts.ts:225"));
    // multi-hit counts
    let h_cls: Classify = |c| c.hits().map_or("none".into(), |n| n.to_string());
    v.push(sc("hits_2to5", "multi-hit count").p1(set("garchomp", "roughskin", "", &["bulletseed"])).p2(set("skarmory", "sturdy", "", &["swordsdance"]))
        .cls(h_cls).r(&[("2", 7, 20), ("3", 7, 20), ("4", 3, 20), ("5", 3, 20)]).refs("moves.rs:1024", "data/mods/champions/scripts.ts:439"));
    v.push(sc("hits_skilllink", "multi-hit count").p1(set("garchomp", "skilllink", "", &["bulletseed"])).p2(set("skarmory", "sturdy", "", &["swordsdance"]))
        .cls(h_cls).r(&[("5", 1, 1)]).refs("abilities.rs:1449", "data/abilities.ts skilllink onModifyMove multihit = max"));
    // secondaries
    let par2: Classify = |c| yes(c.status(1) == Some(Status::Par), "par", "none");
    let flinch2: Classify = |c| yes(c.cant(1, Fx::Cond(conds::FLINCH)) > 0, "flinch", "none");
    let sec = "moves.rs:1484-1520 chance(c,100), c doubled by sec_double";
    v.push(sc("sec_bodyslam", "secondary").p1(set("garchomp", "roughskin", "", &["bodyslam"])).cls(par2).r(&[("par", 30, 100), ("none", 70, 100)]).refs(sec, "sim/battle-actions.ts:1393-1403"));
    v.push(sc("sec_bodyslam_sg", "secondary (Serene Grace)").p1(set("garchomp", "serenegrace", "", &["bodyslam"])).cls(par2).r(&[("par", 60, 100), ("none", 40, 100)]).refs(sec, "data/abilities.ts serenegrace onModifyMove chance *= 2"));
    // Air Slash and Fire Fang have accuracy 95: the reference includes the miss
    v.push(sc("sec_airslash", "secondary (flinch)").p1(set("garchomp", "roughskin", "", &["airslash"])).cls(flinch2).r(&[("flinch", 285, 1000), ("none", 715, 1000)]).refs(sec, "data/moves.ts airslash secondary 30 (x acc 95)"));
    v.push(sc("sec_airslash_sg", "secondary (Serene Grace)").p1(set("garchomp", "serenegrace", "", &["airslash"])).cls(flinch2).r(&[("flinch", 570, 1000), ("none", 430, 1000)]).refs(sec, "data/abilities.ts serenegrace (60 x acc 95)"));
    v.push(sc("sec_kingsrock", "secondary (King's Rock)").p1(set("garchomp", "roughskin", "kingsrock", &["earthquake"])).cls(flinch2).r(&[("flinch", 10, 100), ("none", 90, 100)]).refs("items.rs (ModifyMove adds 10% flinch) + moves.rs:1516", "data/items.ts kingsrock onModifyMove push {chance 10, flinch}"));
    v.push(sc("sec_kingsrock_sg", "secondary (King's Rock + Serene Grace)").p1(set("garchomp", "serenegrace", "kingsrock", &["earthquake"])).cls(flinch2).r(&[("flinch", 20, 100), ("none", 80, 100)]).refs("moves.rs:1508", "items.ts kingsrock onModifyMovePriority -1 runs before serenegrace -2: 20"));
    v.push(sc("sec_stench", "secondary (Stench)").p1(set("garchomp", "stench", "", &["earthquake"])).cls(flinch2).r(&[("flinch", 10, 100), ("none", 90, 100)]).refs("abilities.rs (Stench ModifyMove) + moves.rs:1516", "data/abilities.ts stench onModifyMove push {chance 10, flinch}"));
    v.push(sc("sec_firefang", "secondary (two)").p1(set("garchomp", "roughskin", "", &["firefang"])).cls(|c| {
        format!("{}+{}", st_name(c.status(1)), yes(c.cant(1, Fx::Cond(conds::FLINCH)) > 0, "flinch", "none"))
    }).r(&[("brn+flinch", 95, 10000), ("brn+none", 855, 10000), ("none+flinch", 855, 10000), ("none+none", 8195, 10000)]).refs(sec, "data/moves.ts firefang secondaries [10 brn, 10 flinch]"));
    let st2: Classify = |c| st_name(c.status(1));
    v.push(sc("triattack", "secondary + pick").p1(set("garchomp", "roughskin", "", &["triattack"])).cls(st2)
        .r(&[("none", 4, 5), ("brn", 1, 15), ("par", 1, 15), ("frz", 1, 15)]).refs("moves.rs:1516 + movefx.rs:715-718", "data/moves.ts triattack chance 20, sample [brn,par,frz]"));
    v.push(sc("direclaw", "secondary + pick").p1(set("garchomp", "roughskin", "", &["direclaw"])).cls(st2)
        .r(&[("none", 7, 10), ("psn", 1, 10), ("par", 1, 10), ("slp", 1, 10)]).refs("moves.rs:1516 + movefx.rs:715-718", "data/mods/champions/moves.ts:218-225 chance 30, sample [psn,par,slp]"));
    // speed ties
    let first_mover: Classify = |c| {
        c.steps.last().and_then(|rows| rows.iter().find_map(|r| if let Row::Move { p, .. } = r { Some(format!("p{}", p.0 + 1)) } else { None })).unwrap_or("none".into())
    };
    let gsd = set("garchomp", "roughskin", "", &["swordsdance"]);
    v.push(sc("speedtie_move", "speed tie").p1(gsd).p2(gsd).cls(first_mover).r(&[("p1", 1, 2), ("p2", 1, 2)]).refs("battle.rs:246-250 tie()", "sim/battle.ts speedSort -> prng.shuffle (sim/prng.ts:148-156)"));
    v.push(sc("speedtie_entry", "speed tie").p1(set("garchomp", "intimidate", "", &["swordsdance"])).p2(set("garchomp", "intimidate", "", &["swordsdance"])).start().n(0)
        .cls(|c| c.rows().find_map(|r| if let Row::Ability { p, ab, .. } = r { if *ab == abilities::INTIMIDATE { Some(format!("p{}", p.0 + 1)) } else { None } } else { None }).unwrap_or("none".into()))
        .r(&[("p1", 1, 2), ("p2", 1, 2)]).refs("battle.rs:268-269 / events.rs:706-712", "sim/battle.ts speedSort of switch-in handlers"));
    v.push(sc("speedtie_residual", "speed tie (residual handlers)").p1(set("garchomp", "roughskin", "leftovers", &["swordsdance"])).p2(set("garchomp", "roughskin", "leftovers", &["swordsdance"]))
        .setup(|s| {
            s.sides[0].active_mon_mut().hp -= 50;
            s.sides[1].active_mon_mut().hp -= 50;
        })
        .cls(|c| c.rows().find_map(|r| if let Row::Hp { p, cause: Fx::Item(i), .. } = r { if *i == items::LEFTOVERS { Some(format!("p{}", p.0 + 1)) } else { None } } else { None }).unwrap_or("none".into()))
        .fix(&[]).r(&[("p1", 1, 2), ("p2", 1, 2)]).refs("residual.rs:176-183", "sim/battle.ts fieldEvent Residual speedSort"));
    // sleep, confusion, paralysis, freeze
    let after = |k: usize| -> Vec<[u8; 2]> { std::iter::once([0, 0]).chain(std::iter::repeat([1, 0]).take(k - 1)).collect() };
    v.push(sc("sleep_turns", "sleep duration").p1(set("garchomp", "roughskin", "", &["spore", "swordsdance"])).turns(&after(5))
        .cls(|c| c.cant(1, Fx::Status(Status::Slp)).to_string()).r(&[("1", 1, 3), ("2", 2, 3)]).refs("status.rs:248-251 sample [2,3,3]", "data/mods/champions/conditions.ts:23"));
    v.push(sc("confusion_turns", "confusion duration").p1(set("garchomp", "roughskin", "", &["confuseray", "swordsdance"])).turns(&after(7))
        .cls(|c| c.rows().filter(|r| matches!(r, Row::VolatileActed { p, c: x, of: None, .. } if p.0 == 1 && *x == conds::CONFUSION)).count().to_string())
        .r(&[("1", 1, 4), ("2", 1, 4), ("3", 1, 4), ("4", 1, 4)]).refs("conds.rs:109-112 range(2,6)", "data/conditions.ts:174 random(min,6)"));
    v.push(sc("confusion_turns_axekick", "confusion duration").p1(set("garchomp", "roughskin", "", &["axekick", "swordsdance"])).p2(set("skarmory", "sturdy", "", &["swordsdance"])).turns(&after(7))
        .cls(|c| if c.missed(0) { "miss".into() } else { c.rows().filter(|r| matches!(r, Row::VolatileActed { p, c: x, of: None, .. } if p.0 == 1 && *x == conds::CONFUSION)).count().to_string() })
        .r(&[("miss", 1, 10), ("0", 63, 100), ("2", 9, 100), ("3", 9, 100), ("4", 9, 100)]).refs("conds.rs:110-112 range(3,6) for Axe Kick", "data/conditions.ts:173-174 min 3 for axekick"));
    v.push(sc("confusion_hit", "confusion self-hit").p1(set("garchomp", "roughskin", "", &["confuseray"]))
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Hp { p, cause: Fx::Confusion, .. } if p.0 == 1)), "hit", "none"))
        .r(&[("hit", 33, 100), ("none", 67, 100)]).refs("conds.rs:990-991 chance(33,100)", "data/conditions.ts:187"));
    v.push(sc("roll_confusion", "damage roll (confusion self-hit)").p1(set("garchomp", "roughskin", "", &["confuseray"]))
        .cls(|c| c.ev.iter().find(|e| e.site.key == Key::Roll && e.site.mv_name == Some("confusion")).map_or("none".into(), |e| match e.value { Value::Int(x) => x.to_string(), _ => "?".into() }))
        .fix(&[]).rr(std::iter::once(("none".to_string(), Rat::new(67, 100))).chain((0..16).map(|i| (i.to_string(), Rat::new(33, 1600)))).collect())
        .refs("moves.rs:1841-1842 range(0,16)", "data/conditions.ts confusion onBeforeMove getConfusionDamage -> randomizer"));
    v.push(sc("par_full", "full paralysis").setup(|s| s.sides[1].active_mon_mut().status = Status::Par)
        .cls(|c| yes(c.cant(1, Fx::Status(Status::Par)) > 0, "cant", "moves")).r(&[("cant", 1, 8), ("moves", 7, 8)]).refs("conds.rs:1009-1011 chance(1,8)", "data/mods/champions/conditions.ts:5 randomChance(1,8)"));
    v.push(sc("frz_thaw", "freeze thaw").p1(set("garchomp", "roughskin", "", &["swordsdance"])).n(4).setup(|s| {
        let m = s.sides[1].active_mon_mut();
        m.status = Status::Frz;
        m.status_time = 3;
    })
    .cls(|c| c.cant(1, Fx::Status(Status::Frz)).to_string()).r(&[("0", 1, 4), ("1", 3, 16), ("2", 9, 16)]).refs("conds.rs:919-927 time 3, chance(1,4)", "data/mods/champions/conditions.ts:46-49"));
    // Protect
    v.push(sc("protect_chain", "Protect consecutive").p1(set("snorlax", "thickfat", "", &["swordsdance"])).p2(set("garchomp", "roughskin", "", &["protect"])).n(3)
        .cls(|c| {
            (1..3).map(|i| if c.steps[i].iter().any(|r| matches!(r, Row::Fail { p: Some(p), .. } if p.0 == 1)) { 'F' } else { 'S' }).collect()
        })
        .r(&[("SS", 1, 27), ("SF", 8, 27), ("FS", 18, 27)]).refs("conds.rs:822-825 chance(1,counter)", "data/conditions.ts:440-466 stall counter 3, x3, max 729"));
    // the counter up to its cap (729): the number of successes in a row after the first use
    v.push(sc("protect_chain7", "Protect consecutive (Protect has 8 PP)").p1(set("snorlax", "thickfat", "", &["swordsdance"])).p2(set("garchomp", "roughskin", "", &["protect"])).n(8)
        .cls(|c| (1..8).take_while(|&i| !c.steps[i].iter().any(|r| matches!(r, Row::Fail { p: Some(p), .. } if p.0 == 1))).count().to_string())
        .rr({
            let mut r = Vec::new();
            let mut p = Rat::one();
            for k in 0..7u32 {
                let c = Rat::new(1, 3u128.pow(k + 1).min(729));
                r.push((k.to_string(), p.mul(Rat::new(c.d - c.n, c.d)).unwrap()));
                p = p.mul(c).unwrap();
            }
            r.push(("7".to_string(), p));
            r
        }).refs("conds.rs:822-825 chance(1,counter)", "data/conditions.ts:443-466 counter 3, x3 to counterMax 729"));
    // locks and traps
    v.push(sc("lock_turns", "lock duration").p1(set("umbreon", "synchronize", "", &["outrage"])).p2(set("skarmory", "sturdy", "", &["swordsdance"])).n(4)
        .cls(|c| (0..c.steps.len()).find(|&i| c.steps[i].iter().any(|r| matches!(r, Row::Volatile { p, c: x, .. } if p.0 == 0 && *x == conds::CONFUSION))).map_or("none".into(), |i| (i + 1).to_string()))
        .r(&[("2", 1, 2), ("3", 1, 2)]).refs("conds.rs:124-126 range(2,4)", "data/conditions.ts:265 random(2,4)"));
    v.push(sc("trap_turns", "partial trap duration").p1(set("garchomp", "roughskin", "", &["infestation", "calmmind"])).turns(&[[0, 0], [1, 0], [1, 0], [1, 0], [1, 0], [1, 0], [1, 0], [1, 0]])
        .cls(|c| (c.rows().filter(|r| matches!(r, Row::Hp { p, before, after, .. } if p.0 == 1 && after < before)).count().saturating_sub(1)).to_string())
        .r(&[("4", 1, 2), ("5", 1, 2)]).refs("conds.rs:87-91 range(5,7)", "data/conditions.ts:227 random(5,7)"));
    // forced switches
    let drag2: Classify = |c| c.rows().find_map(|r| if let Row::Enter { p, forced: true } = r { if p.0 == 1 { Some(p.1.to_string()) } else { None } } else { None }).unwrap_or("none".into());
    v.push(sc("drag_roar", "forced switch pick").p1(set("garchomp", "roughskin", "", &["roar"])).cls(drag2).r(&[("1", 1, 2), ("2", 1, 2)]).refs("battle.rs:937-965 sample(bench)", "sim/battle.ts:1586 sample(canSwitchIn)"));
    v.push(sc("drag_dragontail", "forced switch pick").p1(set("garchomp", "roughskin", "", &["dragontail"])).cls(|c| if c.missed(0) { "miss".into() } else { c.rows().find_map(|r| if let Row::Enter { p, forced: true } = r { Some(p.1.to_string()) } else { None }).unwrap_or("none".into()) })
        .r(&[("miss", 1, 10), ("1", 9, 20), ("2", 9, 20)]).refs("battle.rs:937-965", "sim/battle.ts:1586"));
    v.push(sc("drag_redcard", "forced switch pick").p2(set("snorlax", "thickfat", "redcard", &["swordsdance"]))
        .cls(|c| c.rows().find_map(|r| if let Row::Enter { p, forced: true } = r { if p.0 == 0 { Some(p.1.to_string()) } else { None } } else { None }).unwrap_or("none".into()))
        .r(&[("1", 1, 2), ("2", 1, 2)]).refs("battle.rs:937-965 (effect redcard)", "data/items.ts redcard -> dragIn -> sim/battle.ts:1586"));
    // ability procs
    let st1: Classify = |c| st_name(c.status(0));
    let ab = "abilities.rs:1638-1660 chance(3,10)";
    v.push(sc("ab_static", "ability proc").p2(set("snorlax", "static", "", &["swordsdance"])).cls(st1).r(&[("par", 3, 10), ("none", 7, 10)]).refs(ab, "data/abilities.ts:4540 randomChance(3,10)"));
    v.push(sc("ab_flamebody", "ability proc").p2(set("snorlax", "flamebody", "", &["swordsdance"])).cls(st1).r(&[("brn", 3, 10), ("none", 7, 10)]).refs(ab, "data/abilities.ts:1320"));
    v.push(sc("ab_poisonpoint", "ability proc").p2(set("snorlax", "poisonpoint", "", &["swordsdance"])).cls(st1).r(&[("psn", 3, 10), ("none", 7, 10)]).refs(ab, "data/abilities.ts:3348"));
    v.push(sc("ab_cutecharm", "ability proc").p2(fem(set("snorlax", "cutecharm", "", &["swordsdance"]))).cls(|c| yes(c.vol(0, conds::ATTRACT), "attract", "none")).r(&[("attract", 3, 10), ("none", 7, 10)]).refs("abilities.rs:1652-1656", "data/abilities.ts:801"));
    v.push(sc("ab_effectspore", "ability proc + pick").p2(set("snorlax", "effectspore", "", &["swordsdance"])).cls(st1)
        .r(&[("slp", 11, 100), ("par", 10, 100), ("psn", 9, 100), ("none", 70, 100)]).refs("abilities.rs:124-133, 1663-1668", "data/abilities.ts:1164-1172 random(100) <11 slp, <21 par, <30 psn"));
    v.push(sc("ab_cursedbody", "ability proc").p2(set("snorlax", "cursedbody", "", &["swordsdance"])).cls(|c| yes(c.vol(0, conds::DISABLE), "disable", "none")).r(&[("disable", 3, 10), ("none", 7, 10)]).refs("abilities.rs:1717-1723", "data/abilities.ts:788"));
    v.push(sc("ab_poisontouch", "ability proc").p1(set("garchomp", "poisontouch", "", &["dragonclaw"])).cls(st2).r(&[("psn", 3, 10), ("none", 7, 10)]).refs("abilities.rs:1611-1619", "data/abilities.ts:3376"));
    v.push(sc("ab_toxicchain", "ability proc").p1(set("garchomp", "toxicchain", "", &["earthquake"])).cls(st2).r(&[("tox", 3, 10), ("none", 7, 10)]).refs("abilities.rs:1621-1629", "data/abilities.ts:5097"));
    v.push(sc("ab_quickdraw", "ability proc (order)").p1(set("snorlax", "quickdraw", "", &["bodyslam"])).p2(set("garchomp", "roughskin", "", &["swordsdance"]))
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Ability { ab, .. } if *ab == abilities::QUICKDRAW)), "proc", "none")).r(&[("proc", 3, 10), ("none", 7, 10)]).refs("abilities.rs:398-401 chance(3,10)", "data/abilities.ts:3739"));
    v.push(sc("ab_shedskin", "ability proc (residual)").p2(set("snorlax", "shedskin", "", &["swordsdance"])).p1(set("garchomp", "roughskin", "", &["swordsdance"])).setup(|s| s.sides[1].active_mon_mut().status = Status::Brn)
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Cure { p, .. } if p.0 == 1)), "cure", "none")).r(&[("cure", 33, 100), ("none", 67, 100)]).refs("abilities.rs:2735-2738 chance(33,100)", "data/abilities.ts:4192"));
    v.push(sc("ab_harvest", "ability proc (residual)").p2(set("snorlax", "harvest", "", &["swordsdance"])).p1(set("garchomp", "roughskin", "", &["swordsdance"])).setup(|s| s.sides[1].active_mon_mut().last_item = items::SITRUSBERRY)
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Item { p, what: ItemWhat::Received, .. } if p.0 == 1)), "harvest", "none")).r(&[("harvest", 1, 2), ("none", 1, 2)]).refs("abilities.rs:2685-2688 chance(1,2)", "data/abilities.ts:1805"));
    v.push(sc("ab_moody", "ability pick (residual)").p2(set("snorlax", "moody", "", &["swordsdance"])).p1(set("garchomp", "roughskin", "", &["swordsdance"])).turns(&[[0, 0]])
        .cls(|c| {
            let mut sum = [0i8; 7];
            for r in c.rows() {
                if let Row::Boost { p, by_stat, by: Fx::Ability(_) } = r {
                    if p.0 == 1 {
                        for i in 0..7 { sum[i] += by_stat[i]; }
                    }
                }
            }
            let up = (0..5).find(|&i| sum[i] == 2).map_or("-", |i| BOOST_IDS[i]);
            let dn = (0..5).find(|&i| sum[i] == -1).map_or("-", |i| BOOST_IDS[i]);
            format!("{}+{}-", up, dn)
        })
        .rr({
            let s = ["atk", "def", "spa", "spd", "spe"];
            let mut r = Vec::new();
            for a in s { for b in s { if a != b { r.push((format!("{}+{}-", a, b), Rat::new(1, 20))); } } }
            r
        }).refs("abilities.rs:2653-2675 sample_str x2", "data/abilities.ts:2715-2727 sample(stats) x2"));
    v.push(sc("ab_forewarn", "ability pick (entry)").p1(set("garchomp", "roughskin", "", &["closecombat", "outrage"])).p2(set("snorlax", "forewarn", "", &["swordsdance"])).start().n(0)
        .cls(|c| c.ev.iter().find(|e| e.site.key == Key::Pick).map_or("noevent".into(), |e| match e.value { Value::Int(x) => x.to_string(), _ => "?".into() }))
        .r(&[("0", 1, 2), ("1", 1, 2)]).refs("abilities.rs:2440-2445 sample(index)", "data/abilities.ts:1526 sample(warnMoves)"));
    // condition proc
    v.push(sc("attract_cant", "infatuation").p1(set("garchomp", "roughskin", "", &["attract"])).p2(fem(set("snorlax", "thickfat", "", &["swordsdance"])))
        .cls(|c| yes(c.cant(1, Fx::Cond(conds::ATTRACT)) > 0, "cant", "moves")).r(&[("cant", 1, 2), ("moves", 1, 2)]).refs("conds.rs:999-1003 chance(1,2)", "data/conditions.ts attract onBeforeMove randomChance(1,2)"));
    // items
    v.push(sc("item_quickclaw", "item proc").p1(set("snorlax", "thickfat", "quickclaw", &["bodyslam"])).p2(set("garchomp", "roughskin", "", &["swordsdance"]))
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Item { item, .. } if *item == items::QUICKCLAW)), "proc", "none")).r(&[("proc", 1, 5), ("none", 4, 5)]).refs("items.rs:632-634 chance(1,5)", "data/items.ts:5017"));
    v.push(sc("item_focusband", "item proc").p2(set("snorlax", "thickfat", "focusband", &["swordsdance"])).setup(|s| s.sides[1].active_mon_mut().hp = 1)
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Item { item, .. } if *item == items::FOCUSBAND)), "proc", "none")).r(&[("proc", 1, 10), ("none", 9, 10)]).refs("items.rs:665-667 chance(1,10)", "data/items.ts:2273"));
    // moves with their own draws
    v.push(sc("move_ficklebeam", "move proc").p1(set("garchomp", "roughskin", "", &["ficklebeam"]))
        .cls(|c| yes(c.rows().any(|r| matches!(r, Row::Once { fx: Fx::Move(m), .. } if *m == moves::FICKLEBEAM)), "double", "none")).r(&[("double", 3, 10), ("none", 7, 10)]).refs("movefx.rs:1611-1613 chance(3,10)", "data/moves.ts ficklebeam onBasePower randomChance(3,10)"));
    v.push(sc("move_acupressure", "move pick").p1(set("garchomp", "roughskin", "", &["acupressure"]))
        .cls(|c| c.rows().find_map(|r| if let Row::Boost { p, by_stat, .. } = r { if p.0 == 0 { (0..7).find(|&i| by_stat[i] == 2).map(|i| BOOST_IDS[i].to_string()) } else { None } } else { None }).unwrap_or("none".into()))
        .rr(BOOST_IDS.iter().map(|s| (s.to_string(), Rat::new(1, 7))).collect()).refs("movefx.rs:784-802 sample_str", "data/moves.ts acupressure sample(stats)"));
    v.push(sc("move_sleeptalk", "move pick").p2(set("snorlax", "thickfat", "", &["sleeptalk", "bodyslam", "swordsdance", "curse"])).p1(set("garchomp", "roughskin", "", &["swordsdance"])).setup(|s| {
        let m = s.sides[1].active_mon_mut();
        m.status = Status::Slp;
        m.status_time = 3;
    })
    .cls(|c| c.rows().find_map(|r| if let Row::Move { p, mv, called_by: Fx::Move(by), .. } = r { if p.0 == 1 && *by == moves::SLEEPTALK { Some(mv.id().to_string()) } else { None } } else { None }).unwrap_or("none".into()))
    .r(&[("bodyslam", 1, 3), ("swordsdance", 1, 3), ("curse", 1, 3)]).refs("movefx.rs:1041-1060 sample_str", "data/moves.ts sleeptalk sample(moves)"));
    v.push(sc("move_shellsidearm", "move tie (category)").p1(set("garchomp", "roughskin", "", &["shellsidearm"])).setup(|s| {
        let a = s.sides[0].active_mon_mut();
        a.stats[2] = a.stats[0];
        let t = s.sides[1].active_mon_mut();
        t.stats[3] = t.stats[1];
    })
    .cls(|c| c.ev.iter().find(|e| e.site.key == Key::Proc && e.site.side == Some(0)).map_or("noevent".into(), |e| match e.value { Value::Bool(b) => yes(b, "physical", "special"), _ => "?".into() }))
    .r(&[("physical", 1, 2), ("special", 1, 2)]).refs("movefx.rs:601-605 chance(1,2)", "data/moves.ts shellsidearm random(2) on a tie"));
    v
}

// --------------------------------------------------------------------------------------------- tables

fn tables_json() -> String {
    let mut s = String::from("{");
    for (i, m) in MOVES.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let secs: Vec<String> = m.secondaries.iter().map(|x| x.chance.map_or("null".into(), |c| c.to_string())).collect();
        let selfc = m.self_effect.as_ref().map_or("null".into(), |x| x.chance.map_or("\"always\"".into(), |c| c.to_string()));
        let ohko = match m.ohko {
            Ohko::No => "false".to_string(),
            Ohko::Yes => "true".to_string(),
            Ohko::Type(t) => format!("\"{:?}\"", t).to_lowercase(),
        };
        if s.len() > 1 {
            s.push(',');
        }
        let _ = write!(
            s,
            "\n\"{}\":{{\"accuracy\":{},\"critRatio\":{},\"willCrit\":{},\"multihit\":[{},{}],\"multiaccuracy\":{},\"ohko\":{},\"secondaries\":[{}],\"self\":{}}}",
            m.id,
            if m.accuracy == 255 { "true".into() } else { m.accuracy.to_string() },
            m.crit_ratio,
            m.will_crit.map_or("null".into(), |b| b.to_string()),
            m.multihit.0,
            m.multihit.1,
            m.multiaccuracy,
            ohko,
            secs.join(","),
            selfc
        );
    }
    s.push_str("\n}\n");
    s
}

// --------------------------------------------------------------------------------------------- generator

fn generator_checks(out: &mut Vec<String>) {
    let site = Site::new(Key::Roll, Some(0));
    // uniformity of range(0,16)
    let n = 16_000_000u64;
    let mut c = [0u64; 16];
    let mut g = Seeded::new(1);
    for _ in 0..n {
        c[g.range(&site, 0, 16) as usize] += 1;
    }
    let e = n as f64 / 16.0;
    let chi2: f64 = c.iter().map(|&o| (o as f64 - e).powi(2) / e).sum();
    out.push(format!("{{\"kind\":\"generator\",\"test\":\"roll uniformity range(0,16)\",\"draws\":{},\"counts\":{:?},\"chi2\":{:.3},\"df\":15,\"p\":{:.6}}}", n, c, chi2, chi2_p(chi2, 15)));
    // pairs of consecutive rolls (serial dependence)
    let n2 = 8_000_000u64;
    let mut pc = vec![0u64; 256];
    let mut g = Seeded::new(2);
    let mut prev = g.range(&site, 0, 16) as usize;
    for _ in 0..n2 {
        let x = g.range(&site, 0, 16) as usize;
        pc[prev * 16 + x] += 1;
        prev = x;
    }
    let e = n2 as f64 / 256.0;
    let chi2: f64 = pc.iter().map(|&o| (o as f64 - e).powi(2) / e).sum();
    out.push(format!("{{\"kind\":\"generator\",\"test\":\"consecutive roll pairs (16x16)\",\"draws\":{},\"chi2\":{:.3},\"df\":255,\"p\":{:.6}}}", n2, chi2, chi2_p(chi2, 255)));
    // lag-1 to lag-3 autocorrelation of uniform floats
    let n3 = 10_000_000usize;
    let mut g = Seeded::new(3);
    let xs: Vec<f64> = (0..n3).map(|_| (g.next_u64() >> 11) as f64 / (1u64 << 53) as f64).collect();
    let mean = xs.iter().sum::<f64>() / n3 as f64;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n3 as f64;
    for lag in 1..=3 {
        let cov = (0..n3 - lag).map(|i| (xs[i] - mean) * (xs[i + lag] - mean)).sum::<f64>() / (n3 - lag) as f64;
        let r = cov / var;
        let z = r * ((n3 - lag) as f64).sqrt();
        out.push(format!("{{\"kind\":\"generator\",\"test\":\"autocorrelation lag {}\",\"draws\":{},\"r\":{:.3e},\"z\":{:.3},\"mean\":{:.6},\"var\":{:.6}}}", lag, n3, r, z, mean, var));
    }
    // chance(1,24) rate with many seeds: one draw per seed, consecutive seeds
    let seeds = 2_400_000u64;
    let mut k = 0u64;
    for s in 0..seeds {
        k += Seeded::new(s).chance(&site, 1, 24) as u64;
    }
    let p = 1.0 / 24.0;
    let z = (k as f64 - seeds as f64 * p) / (seeds as f64 * p * (1.0 - p)).sqrt();
    out.push(format!("{{\"kind\":\"generator\",\"test\":\"first draw chance(1,24) over consecutive seeds\",\"seeds\":{},\"hits\":{},\"z\":{:.3}}}", seeds, k, z));
    // first rolls of consecutive seeds: independence of seed i and seed i+1
    let ns = 4_000_000u64;
    let mut pc = vec![0u64; 256];
    let first = |s: u64| Seeded::new(s).range(&site, 0, 16) as usize;
    let mut prev = first(0);
    for s in 1..=ns {
        let x = first(s);
        pc[prev * 16 + x] += 1;
        prev = x;
    }
    let e = ns as f64 / 256.0;
    let chi2: f64 = pc.iter().map(|&o| (o as f64 - e).powi(2) / e).sum();
    out.push(format!("{{\"kind\":\"generator\",\"test\":\"first roll of seed s vs seed s+1 (16x16)\",\"seeds\":{},\"chi2\":{:.3},\"df\":255,\"p\":{:.6}}}", ns, chi2, chi2_p(chi2, 255)));
    // distinct streams: first 4 outputs of 1,000,000 seeds, no repeat
    let mut firsts: Vec<[u64; 4]> = (0..1_000_000u64).map(|s| {
        let mut g = Seeded::new(s);
        [g.next_u64(), g.next_u64(), g.next_u64(), g.next_u64()]
    }).collect();
    firsts.sort();
    let dup = firsts.windows(2).filter(|w| w[0] == w[1]).count();
    let mut a = Seeded::new(1);
    let mut b = Seeded::new(2);
    let same = (0..1_000_000).filter(|_| a.next_u64() == b.next_u64()).count();
    out.push(format!("{{\"kind\":\"generator\",\"test\":\"distinct seeds give distinct streams\",\"seeds\":1000000,\"duplicate_prefixes\":{},\"equal_outputs_seed1_vs_seed2_in_1e6\":{}}}", dup, same));
}

// --------------------------------------------------------------------------------------------- whole games

#[derive(Default, Clone, Copy)]
struct GameStat {
    turns: f64,
    acc_n: f64,
    acc_miss: f64,
    crit_n: f64,
    crit_y: f64,
    para_n: f64,
    para_y: f64,
    hits_n: f64,
    hits_sum: f64,
    moves: f64,
    miss_rows: f64,
}

/// ratio of sums with a cluster (per game) standard error
fn ratio(gs: &[GameStat], x: fn(&GameStat) -> f64, n: fn(&GameStat) -> f64) -> (f64, f64) {
    let sx: f64 = gs.iter().map(x).sum();
    let sn: f64 = gs.iter().map(n).sum();
    let r = sx / sn;
    let k = gs.len() as f64;
    let nbar = sn / k;
    let v: f64 = gs.iter().map(|g| (x(g) - r * n(g)).powi(2)).sum::<f64>() / (k - 1.0);
    (r, (v / k).sqrt() / nbar)
}

fn games(path: &str, n: usize, out_games: Option<&str>, out: &mut Vec<String>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
    let tj = Json::parse(&text).expect("teams json");
    let teams: Vec<Vec<MonSpec>> = tj.as_arr().unwrap().iter().filter_map(|t| t.as_arr().unwrap().iter().map(rig::newsim::mon_spec).collect::<Result<Vec<_>, _>>().ok()).filter(|v| v.len() == 6).collect();
    let mut choose = Seeded::new(0xC401CE);
    let mut gs = Vec::with_capacity(n);
    let mut lines = String::new();
    let (mut unfinished, mut errors) = (0, 0);
    let t0 = Instant::now();
    for gi in 0..n {
        let a = choose.below(teams.len() as u32) as usize;
        let mut b = choose.below(teams.len() as u32 - 1) as usize;
        if b >= a {
            b += 1;
        }
        let mut picks = [[0u8; 3]; 2];
        for p in picks.iter_mut() {
            let mut v: Vec<u8> = (0..6).collect();
            for i in 0..3 {
                let j = i + choose.below((6 - i) as u32) as usize;
                v.swap(i, j);
            }
            *p = [v[0], v[1], v[2]];
        }
        let mut rec = Recording::new(Seeded::new(1_000_000 + gi as u64));
        let mut rows = Vec::new();
        let mut st = match State::start_logged([&teams[a], &teams[b]], picks, &mut rec, &mut rows) {
            Ok(s) => s,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let mut k = 0;
        while !st.ended && k < 1000 {
            let opts = st.options();
            let mut ch = [None, None];
            for s in 0..2 {
                if matches!(opts[s].kind, Kind::Turn | Kind::Replace) {
                    let cs = opts[s].choices();
                    ch[s] = Some(cs[choose.below(cs.len() as u32) as usize]);
                }
            }
            if st.step_logged(ch, &mut rec, &mut rows).is_err() {
                errors += 1;
                break;
            }
            k += 1;
        }
        if st.winner == Winner::None {
            unfinished += 1;
        }
        let mut g = GameStat { turns: st.turn as f64, ..Default::default() };
        for e in &rec.events {
            match (e.site.key, e.value) {
                (Key::Accuracy, Value::Bool(v)) => {
                    g.acc_n += 1.0;
                    g.acc_miss += (!v) as u8 as f64;
                }
                (Key::Crit, Value::Bool(v)) => {
                    g.crit_n += 1.0;
                    g.crit_y += v as u8 as f64;
                }
                (Key::FullPara, Value::Bool(v)) => {
                    g.para_n += 1.0;
                    g.para_y += v as u8 as f64;
                }
                (Key::Hits, Value::Int(v)) => {
                    g.hits_n += 1.0;
                    g.hits_sum += v as f64;
                }
                _ => {}
            }
        }
        for r in &rows {
            match r {
                Row::Move { .. } => g.moves += 1.0,
                Row::Miss { .. } => g.miss_rows += 1.0,
                _ => {}
            }
        }
        if out_games.is_some() {
            let _ = writeln!(lines, "{{\"g\":{},\"turns\":{},\"acc\":[{},{}],\"crit\":[{},{}],\"para\":[{},{}],\"hits\":[{},{}],\"moves\":{},\"miss_rows\":{}}}", gi, g.turns, g.acc_miss, g.acc_n, g.crit_y, g.crit_n, g.para_y, g.para_n, g.hits_sum, g.hits_n, g.moves, g.miss_rows);
        }
        gs.push(g);
    }
    if let Some(p) = out_games {
        std::fs::write(p, lines).expect("write games");
    }
    let k = gs.len() as f64;
    let mt = gs.iter().map(|g| g.turns).sum::<f64>() / k;
    let sdt = (gs.iter().map(|g| (g.turns - mt).powi(2)).sum::<f64>() / (k - 1.0)).sqrt();
    let (miss, miss_se) = ratio(&gs, |g| g.acc_miss, |g| g.acc_n);
    let (crit, crit_se) = ratio(&gs, |g| g.crit_y, |g| g.crit_n);
    let (para, para_se) = ratio(&gs, |g| g.para_y, |g| g.para_n);
    let (hits, hits_se) = ratio(&gs, |g| g.hits_sum, |g| g.hits_n);
    let (mrow, mrow_se) = ratio(&gs, |g| g.miss_rows, |g| g.moves);
    out.push(format!(
        "{{\"kind\":\"games\",\"engine\":\"cg\",\"games\":{},\"unfinished\":{},\"errors\":{},\"wall_s\":{:.2},\
         \"turns_mean\":{:.4},\"turns_se\":{:.4},\"miss_share_of_accuracy_draws\":{:.5},\"miss_se\":{:.5},\"crit_share_of_crit_draws\":{:.5},\"crit_se\":{:.5},\
         \"fullpara_share\":{:.5},\"fullpara_se\":{:.5},\"mean_hits_variable_multihit\":{:.5},\"hits_se\":{:.5},\"miss_rows_per_move_row\":{:.5},\"miss_rows_se\":{:.5},\
         \"accuracy_draws\":{},\"crit_draws\":{},\"fullpara_draws\":{},\"hits_draws\":{}}}",
        gs.len(), unfinished, errors, t0.elapsed().as_secs_f64(), mt, sdt / k.sqrt(), miss, miss_se, crit, crit_se, para, para_se, hits, hits_se, mrow, mrow_se,
        gs.iter().map(|g| g.acc_n).sum::<f64>(), gs.iter().map(|g| g.crit_n).sum::<f64>(), gs.iter().map(|g| g.para_n).sum::<f64>(), gs.iter().map(|g| g.hits_n).sum::<f64>()
    ));
}

// --------------------------------------------------------------------------------------------- main

fn counts_json(c: &BTreeMap<String, u64>) -> String {
    let v: Vec<String> = c.iter().map(|(k, n)| format!("\"{}\":{}", k, n)).collect();
    format!("{{{}}}", v.join(","))
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let out_path = arg("--out");
    let trials: usize = arg("--trials").map_or(200_000, |s| s.parse().unwrap());
    let only: Option<Vec<String>> = arg("--only").map(|s| s.split(',').map(String::from).collect());
    let no_gen = args.iter().any(|a| a == "--no-gen");
    let mut sd: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    if let Some(p) = arg("--sd") {
        for line in std::fs::read_to_string(&p).expect("sd file").lines() {
            let Ok(j) = Json::parse(line) else { continue };
            let (Some(name), Some(c)) = (j.get("name").and_then(Json::as_str), j.get("counts").and_then(Json::as_obj)) else { continue };
            let m: BTreeMap<String, u64> = c.iter().map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0) as u64)).collect();
            sd.insert(name.to_string(), m);
        }
    }
    if let Some(p) = arg("--tables") {
        std::fs::write(&p, tables_json()).expect("write tables");
        eprintln!("tables written to {}", p);
    }
    let mut out: Vec<String> = Vec::new();
    // the result file is rewritten after every row, so a stopped run keeps what finished
    let save = |out: &Vec<String>| {
        if let Some(p) = &out_path {
            std::fs::write(p, out.join("
") + "
").expect("write out");
        }
    };
    let all = situations();
    if let Some(name) = arg("--dump") {
        let s = all.iter().find(|s| s.name == name).expect("no such situation");
        let start = prepare(s);
        DUMP.store(true, std::sync::atomic::Ordering::Relaxed);
        for i in 0..3u64 {
            run(s, &start, Seeded::new(i));
        }
        if let Ok((leaves, dist)) = {
            DUMP.store(false, std::sync::atomic::Ordering::Relaxed);
            enumerate(s, &start)
        } {
            eprintln!("enumeration: {} leaves {:?}", leaves, dist.iter().map(|(l, r)| format!("{}={}", l, r.s())).collect::<Vec<_>>());
        }
        return;
    }
    let list: Vec<&Sc> = all.iter().filter(|s| only.as_ref().map_or(true, |o| o.iter().any(|x| x == s.name))).collect();
    let tests = list.len() * 2 + if sd.is_empty() { 0 } else { sd.len() * 2 };
    let alpha = 0.001 / tests.max(1) as f64;
    eprintln!("{} situations, {} trials each, per-test alpha {:.2e} (family 0.001, Bonferroni over {} tests)", list.len(), trials, alpha, tests);
    for s in &list {
        let t0 = Instant::now();
        let start = prepare(s);
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        // one seed per trial; a different seed range per situation
        let base = s.name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
        for i in 0..trials {
            let (label, _) = run(s, &start, Seeded::new(base.wrapping_add(i as u64)));
            *counts.entry(label).or_insert(0) += 1;
        }
        let (chi2, df, p) = gof(&counts, &s.reference);
        let en = enumerate(s, &start);
        let (enum_json, enum_ok) = match &en {
            Ok((leaves, dist)) => {
                let mut sum = Rat::zero();
                for (_, r) in dist {
                    sum = sum.add(*r).unwrap_or(Rat::zero());
                }
                let exact = dist.len() == s.reference.iter().filter(|(_, r)| r.n > 0).count()
                    && dist.iter().all(|(l, r)| s.reference.iter().any(|(m, q)| m == l && q == r));
                let d: Vec<String> = dist.iter().map(|(l, r)| format!("\"{}\":\"{}\"", esc(l), r.s())).collect();
                (format!("{{\"leaves\":{},\"sum\":\"{}\",\"exact_match\":{},\"dist\":{{{}}},\"fixed\":\"{:?}\"}}", leaves, sum.s(), exact, d.join(","), s.fix), exact && sum == Rat::one())
            }
            Err(e) => (format!("{{\"error\":\"{}\"}}", esc(e)), false),
        };
        let refs: Vec<String> = s.reference.iter().map(|(l, r)| format!("\"{}\":\"{}\"", l, r.s())).collect();
        let mut line = format!(
            "{{\"kind\":\"situation\",\"name\":\"{}\",\"class\":\"{}\",\"trials\":{},\"counts\":{},\"reference\":{{{}}},\"chi2\":{:.3},\"df\":{},\"p\":{:.3e},\"pass\":{},\"enum\":{},\"enum_ok\":{},\"engine_ref\":\"{}\",\"showdown_ref\":\"{}\"",
            s.name, esc(s.class), trials, counts_json(&counts), refs.join(","), chi2, df, p, p >= alpha, enum_json, enum_ok, esc(s.eng), esc(s.sd)
        );
        if let Some(c) = sd.get(s.name) {
            let (c2, df2, p2) = two_sample(&counts, c);
            let (c3, df3, p3) = gof(c, &s.reference);
            let _ = write!(line, ",\"sd_counts\":{},\"sd_trials\":{},\"cg_vs_sd_chi2\":{:.3},\"cg_vs_sd_df\":{},\"cg_vs_sd_p\":{:.3e},\"sd_vs_ref_chi2\":{:.3},\"sd_vs_ref_p\":{:.3e}", counts_json(c), c.values().sum::<u64>(), c2, df2, p2, c3, p3);
            let _ = df3;
        }
        let _ = write!(line, ",\"wall_s\":{:.2}}}", t0.elapsed().as_secs_f64());
        eprintln!("{:<22} p={:.3e} pass={} enum_ok={} {}", s.name, p, p >= alpha, enum_ok, counts_json(&counts));
        out.push(line);
        save(&out);
    }
    if !no_gen && only.is_none() {
        generator_checks(&mut out);
        save(&out);
    }
    if let Some(n) = arg("--games").map(|s| s.parse::<usize>().unwrap()) {
        let teams = arg("--teams").unwrap_or("tools/rig/data/teams.json".into());
        games(&teams, n, arg("--games-out").as_deref(), &mut out);
        save(&out);
    }
    if out_path.is_none() {
        print!("{}", out.join("\n") + "\n");
    }
}
