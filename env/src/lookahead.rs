//! `lookahead`: a built-in Player that plays each of its legal actions against each action it thinks the
//! opponent may take, on an engine State built from its own Battle (`from_battle`), samples the turn's
//! outcomes with the engine's probabilities, scores the positions (`evaluate`) and keeps the action whose
//! worst replies are best. It reads only its Battle: unknown opposing sets follow `from_battle`'s fixed
//! rule (revealed moves, item and ability; Serious, no points) with one fixed fill for unrevealed moves
//! (`placeholder_moves`, from public species data).
//!
//! - Opponent: every choice its State offers (moves, Mega, switches), except switches to Pokemon it has
//!   not shown. Unseen brought Pokemon are the lowest unseen preview indices.
//! - Ours: every legal action; with Mega Evolution offered, the Mega variants of the moves only.
//! - Score of an action: the mean over its `replies` worst opponent actions of the mean over `samples`
//!   draws of `evaluate` (1 reply: maximin). Ties go to the lower action index. Draws share seeds across
//!   our actions (common random numbers).
//! - Replace: each replacement is scored by a one-turn search (one draw per pair) from where it leads.
//! - Depth 2: the best `WIDTH2` actions of depth 1, each against its `WIDTH2` worst replies, are re-scored
//!   by a one-turn search from each drawn position.
//! - Team preview: the three with the best worst-case matchup (`players::matchup`) over the six shown,
//!   the one with the best worst case leading.
//! - No State (a replace request before its rows with nobody left, ...): max damage decides.

use crate::batch::{battle_options, choice_of, each_turn_action};
use crate::from_battle::{state_from_battle, OppGuess};
use crate::teams::mix;
use battle::{legal, Battle, ReqKind, Side, Who};
use cg_engine::sim::data::*;
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::{State, Winner, TEAM};
use cg_engine::sim::{Choice, Kind};
use players::estimate::Estimator;
use players::fired::Fired;
use players::matchup::{matchup, HeuristicParams};
use players::max_damage::single;
use players::{MaxDamagePlayer, Player, Rng};
use std::sync::OnceLock;

/// Options, from the name: `lookahead` (depth 1), `lookahead2` (depth 2), each optionally followed by
/// `@key=value,...` with keys `depth` (1, 2), `samples` (1..16), `replies` (1..11), `seed`, `fill` (1, 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// 1 or 2 turns
    pub depth: u8,
    /// draws per (our action, opponent action) pair
    pub samples: u8,
    /// opponent actions averaged per action, worst first (1: maximin)
    pub replies: u8,
    /// mixed into every draw's seed
    pub seed: u64,
    /// unrevealed opposing moves: 1 a STAB placeholder per type, 2 also one coverage placeholder (without
    /// them a Pokemon that has shown no move has none, no State can be built and max damage decides)
    pub fill: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config { depth: 1, samples: 4, replies: 2, seed: 0, fill: 2 }
    }
}

impl Config {
    /// The options a built-in name stands for; None when it is not a lookahead name or an option is bad.
    pub fn parse(name: &str) -> Option<Config> {
        let (head, opts) = match name.split_once('@') {
            Some((h, o)) => (h, Some(o)),
            None => (name, None),
        };
        let mut c = Config::default();
        match head {
            "lookahead" => {}
            "lookahead2" => c.depth = 2,
            _ => return None,
        }
        for kv in opts.unwrap_or("").split(',').filter(|s| !s.is_empty()) {
            let (k, v) = kv.split_once('=')?;
            let n: u64 = v.trim().parse().ok()?;
            match k.trim() {
                "depth" if (1..=2).contains(&n) => c.depth = n as u8,
                "samples" if (1..=16).contains(&n) => c.samples = n as u8,
                "replies" if (1..=11).contains(&n) => c.replies = n as u8,
                "seed" => c.seed = n,
                "fill" if (1..=2).contains(&n) => c.fill = n as u8,
                _ => return None,
            }
        }
        Some(c)
    }
}

/// Weights of `evaluate`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    /// per Pokemon not fainted, times its HP fraction to the power `alive_pow`
    pub alive: f64,
    pub alive_pow: f64,
    /// per Pokemon, times its HP fraction
    pub hp: f64,
    pub slp: f64,
    pub frz: f64,
    pub par: f64,
    /// burn on a special attacker; a physical one (Atk above SpA) takes `brn_physical`
    pub brn: f64,
    pub brn_physical: f64,
    pub psn: f64,
    pub tox: f64,
    /// the active Pokemon moves first (the other's loses it)
    pub speed: f64,
    /// per stage of the active's better attacking stat and of speed; half for each defence
    pub boost: f64,
    /// per benched Pokemon not fainted: Stealth Rock, a Spikes layer, a Toxic Spikes layer, Sticky Web
    pub rock: f64,
    pub spikes: f64,
    pub tspikes: f64,
    pub web: f64,
    /// a won or lost game
    pub win: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            alive: 1.0,
            alive_pow: 0.5,
            hp: 1.0,
            slp: 0.4,
            frz: 0.5,
            par: 0.25,
            brn: 0.1,
            brn_physical: 0.3,
            psn: 0.1,
            tox: 0.2,
            speed: 0.15,
            boost: 0.1,
            rock: 0.12,
            spikes: 0.06,
            tspikes: 0.05,
            web: 0.05,
            win: 2.0,
        }
    }
}

fn stage(x: u32, s: i8) -> f64 {
    let s = s.clamp(-6, 6) as f64;
    x as f64 * if s >= 0.0 { (2.0 + s) / 2.0 } else { 2.0 / (2.0 - s) }
}

fn has_cond(st: &State, side: usize, c: Cond) -> Option<u8> {
    st.sides[side].conds().iter().find(|x| x.id == c).map(|x| x.layers.max(1))
}

/// The speed the active Pokemon of `side` acts at, from the State's numbers (no ability effects).
fn speed(st: &State, side: usize) -> f64 {
    let m = st.sides[side].active_mon();
    let mut s = stage(m.stats[4] as u32, m.boosts[4]);
    if m.status == Status::Par {
        s *= 0.5;
    }
    if m.item.id() == "choicescarf" {
        s *= 1.5;
    }
    if has_cond(st, side, conds::TAILWIND).is_some() {
        s *= 2.0;
    }
    s
}

fn side_value(st: &State, side: usize, w: &Weights) -> f64 {
    let sd = &st.sides[side];
    let mut v = 0.0;
    let mut bench = 0.0;
    for (i, m) in sd.mons.iter().enumerate() {
        if m.fainted || m.hp == 0 {
            continue;
        }
        let h = m.hp as f64 / m.maxhp.max(1) as f64;
        v += w.alive * h.powf(w.alive_pow) + w.hp * h;
        v -= match m.status {
            Status::None => 0.0,
            Status::Slp => w.slp,
            Status::Frz => w.frz,
            Status::Par => w.par,
            Status::Brn if m.stats[0] > m.stats[2] => w.brn_physical,
            Status::Brn => w.brn,
            Status::Psn => w.psn,
            Status::Tox => w.tox,
        };
        if !(sd.has_active && sd.active as usize == i) {
            bench += 1.0;
        }
    }
    let mut hz = 0.0;
    if has_cond(st, side, conds::STEALTHROCK).is_some() {
        hz += w.rock;
    }
    if let Some(n) = has_cond(st, side, conds::SPIKES) {
        hz += w.spikes * n as f64;
    }
    if let Some(n) = has_cond(st, side, conds::TOXICSPIKES) {
        hz += w.tspikes * n as f64;
    }
    if has_cond(st, side, conds::STICKYWEB).is_some() {
        hz += w.web;
    }
    v -= hz * bench;
    if sd.has_active {
        let m = sd.active_mon();
        if !m.fainted {
            let b = m.boosts;
            let atk = if m.stats[0] >= m.stats[2] { b[0] } else { b[2] };
            v += w.boost * (atk as f64 + b[4] as f64 + 0.5 * (b[1] as f64 + b[3] as f64));
        }
    }
    v
}

/// The position for side `us`: its Pokemon, HP, statuses, boosts and hazards less the other side's, and
/// who moves first; a finished game adds +-`win`.
pub fn evaluate(st: &State, us: usize, w: &Weights) -> f64 {
    let them = 1 - us;
    let mut v = side_value(st, us, w) - side_value(st, them, w);
    if st.ended {
        return v + match st.winner {
            Winner::Side(s) if s as usize == us => w.win,
            Winner::Side(_) => -w.win,
            _ => 0.0,
        };
    }
    let (a, b) = (&st.sides[us], &st.sides[them]);
    if a.has_active && b.has_active && !a.active_mon().fainted && !b.active_mon().fainted {
        let (x, y) = (speed(st, us), speed(st, them));
        let tr = st.field.pseudo[..st.field.npseudo as usize].iter().any(|c| c.id == conds::TRICKROOM);
        let first = if tr { x < y } else { x > y };
        if x != y {
            v += if first { w.speed } else { -w.speed };
        }
    }
    v
}

/// For each species, learnable damaging moves standing in for unrevealed attacks: one per type and one of
/// another type (coverage), each the strongest plain one (power 100 or less, 90% accuracy or more, no
/// recoil, charge, recharge, self-KO, fixed damage, multiple hits or priority) in the category of its
/// higher attacking base stat, else in the other.
fn placeholder_table() -> &'static Vec<[MoveId; 3]> {
    static T: OnceLock<Vec<[MoveId; 3]>> = OnceLock::new();
    T.get_or_init(|| {
        (0..SPECIES.len())
            .map(|s| {
                let sp = SpeciesId(s as u16);
                let d = sp.data();
                let phys = d.base_stats[1] >= d.base_stats[3];
                let mut out = [MoveId::NONE; 3];
                // slots 0 and 1: each type; slot 2: the strongest of any other type (coverage)
                for k in 0..3 {
                    let ty = if k < 2 { d.types[k] } else { Type::Typeless };
                    if k < 2 && (!ty.is_real() || (k == 1 && ty == d.types[0])) {
                        continue;
                    }
                    let mut best: Option<(bool, u16, MoveId)> = None;
                    for i in 1..MOVES.len() {
                        let mv = MoveId(i as u16);
                        let m = mv.data();
                        let type_ok = if k < 2 { m.ty == ty } else { !d.types.contains(&m.ty) };
                        if !type_ok || m.category == Category::Status || m.base_power == 0 || m.base_power > 100 {
                            continue;
                        }
                        let acc_ok = m.accuracy == ACC_ALWAYS || m.accuracy >= 90;
                        let plain = m.recoil.0 == 0
                            && m.flags & (flag::CHARGE | flag::RECHARGE) == 0
                            && m.selfdestruct == SelfDestruct::No
                            && m.damage == FixedDamage::None
                            && m.ohko == Ohko::No
                            && m.multihit == (0, 0)
                            && m.priority == 0;
                        if !acc_ok || !plain || !obs::can_learn(sp, mv) {
                            continue;
                        }
                        let pref = (m.category == Category::Physical) == phys;
                        let key = (pref, m.base_power, mv);
                        if best.map_or(true, |b| (key.0, key.1) > (b.0, b.1)) {
                            best = Some(key);
                        }
                    }
                    if let Some((_, _, mv)) = best {
                        out[k] = mv;
                    }
                }
                out
            })
            .collect()
    })
}

/// The fixed fill for an opposing Pokemon's moves: its revealed moves, then a placeholder for each of its
/// types no revealed damaging move has, then (`coverage`) the coverage one unless a revealed damaging move
/// is of another type; at most four.
pub fn placeholder_moves(sp: SpeciesId, revealed: &[MoveId], coverage: bool) -> Vec<MoveId> {
    let mut v: Vec<MoveId> = revealed.to_vec();
    let t = placeholder_table();
    let types = sp.data().types;
    let attacks = |pred: &dyn Fn(Type) -> bool| revealed.iter().any(|r| r.data().category != Category::Status && pred(r.data().ty));
    for (k, &mv) in t.get(sp.0 as usize).map_or(&[MoveId::NONE; 3][..], |x| &x[..]).iter().enumerate() {
        if mv.is_none() || v.len() >= 4 || v.contains(&mv) || (k == 2 && !coverage) {
            continue;
        }
        let ty = mv.data().ty;
        let has = if k < 2 { attacks(&|t| t == ty) } else { attacks(&|t| !types.contains(&t)) };
        if !has {
            v.push(mv);
        }
    }
    v
}

/// The opponent's three for `state_from_battle`: the lead, the others it has shown, then the lowest unseen
/// preview indices; and which of the three it has shown.
pub fn opponent_three(b: &Battle, fill: u8) -> (Vec<OppGuess>, [bool; TEAM]) {
    let opp = &b.sides[1];
    let mut slots: Vec<u8> = Vec::with_capacity(TEAM);
    if let Some(l) = (0..6u8).find(|&i| opp.team[i as usize].was_lead) {
        slots.push(l);
    }
    for i in 0..6u8 {
        if opp.team[i as usize].seen && !slots.contains(&i) {
            slots.push(i);
        }
    }
    let mut shown = [false; TEAM];
    for k in 0..slots.len().min(TEAM) {
        shown[k] = true;
    }
    for i in 0..6u8 {
        if slots.len() >= TEAM {
            break;
        }
        let m = &opp.team[i as usize];
        if !slots.contains(&i) && (m.preview.is_some() || m.species.is_some()) {
            slots.push(i);
        }
    }
    let guesses = slots
        .iter()
        .take(TEAM)
        .map(|&i| {
            let m = &opp.team[i as usize];
            let revealed: Vec<MoveId> = m.moves.iter().flatten().copied().filter(|x| *x != moves::STRUGGLE).collect();
            let moves = match (fill, m.species.or(m.preview)) {
                (1..=2, Some(sp)) => placeholder_moves(sp, &revealed, fill == 2),
                _ => revealed,
            };
            OppGuess { slot: i, moves, ..OppGuess::default() }
        })
        .collect();
    (guesses, shown)
}

/// Depth 2 re-scores this many of the best actions, each against this many of its worst replies.
const WIDTH2: usize = 3;

pub const RULES: &[&str] = &["decisions", "single", "search", "search_replace", "depth2", "fallback", "preview"];
const R_DECISIONS: usize = 0;
const R_SINGLE: usize = 1;
const R_SEARCH: usize = 2;
const R_REPLACE: usize = 3;
const R_DEPTH2: usize = 4;
const R_FALLBACK: usize = 5;
const R_PREVIEW: usize = 6;

/// The `lookahead` Player. Keeps nothing between decisions but its rule counts (one may serve many games).
pub struct LookaheadPlayer {
    pub cfg: Config,
    pub w: Weights,
    fallback: MaxDamagePlayer,
    est: Estimator,
    pub fired: Fired,
}

impl LookaheadPlayer {
    pub fn new(cfg: Config) -> LookaheadPlayer {
        let w = Weights::default();
        LookaheadPlayer { cfg, w, fallback: MaxDamagePlayer::default(), est: Estimator::default(), fired: Fired::new(RULES) }
    }
    pub fn by_name(name: &str) -> Option<LookaheadPlayer> {
        Config::parse(name).map(LookaheadPlayer::new)
    }
}

/// One side's choices at a State: `None` alone when it does not choose.
fn side_choices(st: &State, side: usize, shown: Option<&[bool; TEAM]>) -> Vec<Option<Choice>> {
    let o = st.options()[side];
    if !matches!(o.kind, Kind::Turn | Kind::Replace) {
        return vec![None];
    }
    let mut v = Vec::with_capacity(11);
    each_turn_action(&o, |a| {
        let ok = match (a, shown) {
            (8..=10, Some(s)) => s[(a - 8) as usize],
            _ => true,
        };
        if ok {
            if let Some(c) = choice_of(&o, a) {
                v.push(Some(c));
            }
        }
    });
    if v.is_empty() {
        // only unshown Pokemon to send in: they stand in
        each_turn_action(&o, |a| v.extend(choice_of(&o, a).map(Some)));
    }
    if v.is_empty() {
        v.push(None);
    }
    v
}

/// Mean of the `k` smallest of `row` (NaN entries, failed steps, are left out). None when all failed.
fn worst_mean(row: &[f64], k: usize) -> Option<f64> {
    let mut v: Vec<f64> = row.iter().copied().filter(|x| !x.is_nan()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = k.clamp(1, v.len());
    Some(v[..k].iter().sum::<f64>() / k as f64)
}

struct Search<'a> {
    w: &'a Weights,
    us: usize,
    shown: [bool; TEAM],
    replies: usize,
}

impl Search<'_> {
    fn step(&self, st: &State, ours: Option<Choice>, theirs: Option<Choice>, seed: u64) -> Option<State> {
        let mut s = *st;
        let mut ch = [None, None];
        ch[self.us] = ours;
        ch[1 - self.us] = theirs;
        let mut r = Seeded::new(seed);
        s.step(ch, &mut r).ok().map(|_| s)
    }

    /// matrix[our i][their j]: the mean of `n` draws' evaluations (NaN when the step failed).
    fn matrix(&self, st: &State, ours: &[Option<Choice>], theirs: &[Option<Choice>], n: usize, seed: u64, leaf: &mut dyn FnMut(&State) -> f64) -> Vec<Vec<f64>> {
        let mut m = vec![vec![0.0; theirs.len()]; ours.len()];
        for (i, &a) in ours.iter().enumerate() {
            for (j, &o) in theirs.iter().enumerate() {
                let mut sum = 0.0;
                for k in 0..n {
                    match self.step(st, a, o, mix(seed, (j * 64 + k) as u64)) {
                        Some(s) => sum += leaf(&s),
                        None => {
                            sum = f64::NAN;
                            break;
                        }
                    }
                }
                m[i][j] = sum / n as f64;
            }
        }
        m
    }

    /// The value of a position by a one-turn search with one draw per pair, when both sides (or only we)
    /// choose next; otherwise its evaluation.
    fn one_turn(&self, st: &State, seed: u64) -> f64 {
        if st.ended {
            return evaluate(st, self.us, self.w);
        }
        let o = st.options();
        if !matches!(o[self.us].kind, Kind::Turn) {
            return evaluate(st, self.us, self.w);
        }
        let ours = side_choices(st, self.us, None);
        let theirs = side_choices(st, 1 - self.us, Some(&self.shown));
        let m = self.matrix(st, &ours, &theirs, 1, seed, &mut |s| evaluate(s, self.us, self.w));
        m.iter().filter_map(|r| worst_mean(r, self.replies)).fold(None, |b: Option<f64>, x| Some(b.map_or(x, |b| b.max(x)))).unwrap_or_else(|| evaluate(st, self.us, self.w))
    }
}

/// What `LookaheadPlayer::analyse` saw: our actions, the opponent's choices, the depth-1 matrix of mean
/// evaluations (NaN: the step failed), each action's final score and the State searched from.
#[derive(Clone, Debug)]
pub struct Analysis {
    pub actions: Vec<usize>,
    pub theirs: Vec<Option<Choice>>,
    pub matrix: Vec<Vec<f64>>,
    pub scores: Vec<Option<f64>>,
    pub state: State,
}

/// The index of the best score; ties to the first (the lower action).
fn best_of(scores: &[Option<f64>]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, s) in scores.iter().enumerate() {
        if let Some(x) = *s {
            if best.map_or(true, |(_, b)| x > b) {
                best = Some((i, x));
            }
        }
    }
    best.map(|b| b.0)
}

impl LookaheadPlayer {
    /// The search's choice, or None when no State can be built or every step failed.
    fn search(&mut self, b: &Battle, kind: ReqKind, seed: u64) -> Option<usize> {
        let a = self.analyse(b, kind, seed).ok()?;
        best_of(&a.scores).map(|i| a.actions[i])
    }

    /// The search in full: our actions, the opponent's choices, the depth-1 matrix and the final scores.
    pub fn analyse(&mut self, b: &Battle, kind: ReqKind, seed: u64) -> Result<Analysis, String> {
        let (guesses, shown) = opponent_three(b, self.cfg.fill);
        let (st, _) = state_from_battle(b, kind, 0, &guesses, &[None; 6], seed)?;
        let us = 0;
        let l = legal(b, kind);
        let bo = battle_options(b, if kind == ReqKind::Replace { Kind::Replace } else { Kind::Turn });
        let mut acts: Vec<usize> = Vec::with_capacity(11);
        let mut ours: Vec<Option<Choice>> = Vec::with_capacity(11);
        let mega_only = (4..8).any(|i| l.mask[i]);
        each_turn_action(&bo, |a| {
            if l.mask[a as usize] && !(mega_only && a < 4) {
                if let Some(c) = choice_of(&bo, a) {
                    acts.push(a as usize);
                    ours.push(Some(c));
                }
            }
        });
        if ours.is_empty() {
            return Err("no legal action maps to a choice".into());
        }
        let w = self.w;
        let sr = Search { w: &w, us, shown, replies: self.cfg.replies as usize };
        let theirs = side_choices(&st, 1 - us, Some(&shown));
        let n = self.cfg.samples as usize;
        let replace = kind == ReqKind::Replace;
        let m = if replace {
            self.fired.hit(R_REPLACE);
            sr.matrix(&st, &ours, &theirs, n, seed, &mut |s| sr.one_turn(s, mix(seed, 7)))
        } else {
            self.fired.hit(R_SEARCH);
            sr.matrix(&st, &ours, &theirs, n, seed, &mut |s| evaluate(s, us, &w))
        };
        let mut scores: Vec<Option<f64>> = m.iter().map(|r| worst_mean(r, sr.replies)).collect();
        if self.cfg.depth >= 2 && !replace && ours.len() > 1 {
            self.fired.hit(R_DEPTH2);
            // the best few of depth 1, each against its worst few replies, one turn deeper
            let mut order: Vec<usize> = (0..ours.len()).filter(|&i| scores[i].is_some()).collect();
            order.sort_by(|&x, &y| scores[y].partial_cmp(&scores[x]).unwrap().then(x.cmp(&y)));
            order.truncate(WIDTH2);
            let mut deep: Vec<Option<f64>> = vec![None; ours.len()];
            for &i in &order {
                let mut js: Vec<usize> = (0..theirs.len()).filter(|&j| !m[i][j].is_nan()).collect();
                js.sort_by(|&x, &y| m[i][x].partial_cmp(&m[i][y]).unwrap().then(x.cmp(&y)));
                js.truncate(WIDTH2);
                let mut row = Vec::with_capacity(js.len());
                for &j in &js {
                    let mut sum = 0.0;
                    let mut ok = true;
                    for k in 0..n {
                        let s0 = mix(seed, (j * 64 + k) as u64);
                        match sr.step(&st, ours[i], theirs[j], s0) {
                            Some(s) => sum += sr.one_turn(&s, mix(s0, 11)),
                            None => ok = false,
                        }
                    }
                    row.push(if ok { sum / n as f64 } else { f64::NAN });
                }
                deep[i] = worst_mean(&row, sr.replies);
            }
            scores = deep;
        }
        Ok(Analysis { actions: acts, theirs, matrix: m, scores, state: st })
    }

    /// Team preview: the three whose best answer to each of the six shown is best in the worst case
    /// (then on average), lead first: the member with the best worst case.
    fn preview(&mut self, b: &Battle) -> [u8; 3] {
        let n = (b.n_own as usize).clamp(1, 6);
        let opp: Vec<u8> = (0..6u8).filter(|&j| b.sides[1].team[j as usize].preview.or(b.sides[1].team[j as usize].species).is_some()).collect();
        if n < 3 || opp.is_empty() {
            return players::max_damage::preview_picks(b);
        }
        self.est.reset(b);
        let p = HeuristicParams::default();
        let mut mu = [[0f32; 6]; 6];
        for i in 0..n {
            for &j in &opp {
                mu[i][j as usize] = matchup(b, &mut self.est, Who { side: Side::Own, slot: i as u8 }, Who { side: Side::Opp, slot: j }, &p);
            }
        }
        let mut best: Option<((f32, f32), [u8; 3])> = None;
        for x in 0..n {
            for y in x + 1..n {
                for z in y + 1..n {
                    let set = [x, y, z];
                    let ans: Vec<f32> = opp.iter().map(|&j| set.iter().map(|&i| mu[i][j as usize]).fold(f32::NEG_INFINITY, f32::max)).collect();
                    let worst = ans.iter().copied().fold(f32::INFINITY, f32::min);
                    let mean = ans.iter().sum::<f32>() / ans.len() as f32;
                    let key = (worst, mean);
                    if best.map_or(true, |(k, _)| key > k) {
                        best = Some((key, [x as u8, y as u8, z as u8]));
                    }
                }
            }
        }
        let (_, mut set) = best.expect("three of at least three");
        let lead_key = |i: u8| {
            let r: Vec<f32> = opp.iter().map(|&j| mu[i as usize][j as usize]).collect();
            (r.iter().copied().fold(f32::INFINITY, f32::min), r.iter().sum::<f32>())
        };
        let mut li = 0;
        for k in 1..3 {
            if lead_key(set[k]) > lead_key(set[li]) {
                li = k;
            }
        }
        set.swap(0, li);
        set
    }
}

impl Player for LookaheadPlayer {
    fn name(&self) -> &'static str {
        if self.cfg.depth >= 2 {
            "lookahead2"
        } else {
            "lookahead"
        }
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        self.fired.hit(R_PREVIEW);
        self.preview(b)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, rng: &mut Rng) -> usize {
        self.fired.hit(R_DECISIONS);
        let l = legal(b, kind);
        if l.count() == 0 {
            return 0;
        }
        if let Some(a) = single(&l) {
            self.fired.hit(R_SINGLE);
            return a;
        }
        let seed = mix(rng.next_u64(), self.cfg.seed);
        let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.search(b, kind, seed))).ok().flatten();
        match found {
            Some(a) if l.mask[a] => a,
            _ => {
                self.fired.hit(R_FALLBACK);
                self.fallback.choose_move(b, kind, rng)
            }
        }
    }
    fn fired(&self) -> Vec<(&'static str, u64)> {
        self.fired.list()
    }
}
