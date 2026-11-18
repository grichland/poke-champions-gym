//! Disciplined (`pcsa/players/disciplined.py`, `config/players/disciplined.yaml`): max damage's preview,
//! attacks and forced picks, plus rules from the ladder research. Mega variants only, when Mega is offered.
//! The first rule that gives an action decides:
//!
//! - D-0: no move legal or an active missing: max damage ("forced").
//! - D-1: the foe is boosted (atk/def/spa/spd/spe) or has revealed a setup move: the denier steps, in order
//!   (a) a switch-in that takes under `denier_switch_in_max` of its HP from the foe's best revealed hit and
//!   can KO it (least damage taken first); (b) we can KO it, and move first or it cannot KO us: the move of
//!   our best known hit; (c) the first legal anti-setup move that can work; (d) our strongest priority move
//!   that KOs; then max damage ("hardest hit"). The rules below are skipped.
//! - D-2 futile: our best legal hit under `futile_hit`: switch to the member with the best hit on the foe
//!   (ignores the back-to-back rule).
//! - D-3 bad trade: best legal hit under `trade_hit_min` and threatened: the member taking the least,
//!   if under `switch_in_relaxed` of its HP.
//! - D-4 threatened: threatened and we cannot KO: the member taking under `switch_in_max` of its HP and
//!   hitting for `switch_in_threatens` (least taken first).
//! - D-5 recover: a recovery move below `recover_below` HP, if the heal outlasts their best hit, we cannot
//!   KO, and we are not ahead in members.
//! - Otherwise max damage; ahead in members, status moves are dropped while an attack is legal ("phase").
//!
//! Switch rules D-1a, D-3 and D-4 obey the back-to-back rule (`best_matchup::blocked`).

use crate::best_matchup::{blocked, switches};
use crate::endgame::{self, EndgameParams};
use crate::estimate::Estimator;
use crate::fired::Fired;
use crate::lists::{self, class};
use crate::max_damage::{self, action_move, move_actions, preview_picks, single};
use crate::threat::{best_hit, can_ko, faster, hit_fraction, hp, left, priority_ko, threatened, type_mult};
use crate::view::has_vol;
use crate::{Options, Player, Rng};
use battle::{legal, Battle, Legal, ReqKind, Side, Who};
use cg_engine::sim::data::*;

/// `config/players/disciplined.yaml`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisciplinedParams {
    /// vs a boosted foe: a switch-in must take less than this share of its HP (and KO it)
    pub denier_switch_in_max: f32,
    /// their best hit, as a share of our max HP, that makes our active threatened
    pub threat_threshold: f32,
    /// a voluntary switch-in must take less than this share of its HP...
    pub switch_in_max: f32,
    /// ...and hit them for at least this share of their max HP
    pub switch_in_threatens: f32,
    /// a recovery move only below this HP fraction
    pub recover_below: f32,
    /// our best legal hit under this share of their max HP: switch anyway
    pub futile_hit: f32,
    /// threatened and our best legal hit under this: a bad trade, switch if...
    pub trade_hit_min: f32,
    /// ...a bench member takes less than this share of its HP
    pub switch_in_relaxed: f32,
    /// voluntary switches in a row (best_matchup's MAX_STREAK)
    pub max_streak: u8,
}

impl Default for DisciplinedParams {
    fn default() -> Self {
        DisciplinedParams {
            denier_switch_in_max: 0.25,
            threat_threshold: 0.4,
            switch_in_max: 0.25,
            switch_in_threatens: 0.3,
            recover_below: 0.6,
            futile_hit: 0.05,
            trade_hit_min: 0.15,
            switch_in_relaxed: 0.6,
            max_streak: 2,
        }
    }
}

/// Recovery moves (Rest is not one: it puts the user to sleep).
pub const RECOVERY: [&str; 10] = ["recover", "roost", "slackoff", "softboiled", "milkdrink", "shoreup", "wish", "synthesis", "moonlight", "morningsun"];
const WEATHER_HEALS: [&str; 3] = ["synthesis", "moonlight", "morningsun"];
const HALF: f32 = 0.5;
const SUN_HEAL: f32 = 2.0 / 3.0;
const WEATHER_HEAL: f32 = 0.25;
const MIN_HP: f32 = 1e-3;
/// A phaze needs an opposing member left besides the active.
const PHAZE_TARGETS: i32 = 2;
const ANTI_SETUP: [&str; 8] = ["haze", "clearsmog", "encore", "taunt", "roar", "whirlwind", "dragontail", "perishsong"];
const CLEARS_BOOSTS: [&str; 5] = ["haze", "clearsmog", "roar", "whirlwind", "dragontail"];
const PHAZES: [&str; 3] = ["roar", "whirlwind", "dragontail"];

/// Rule counters; the endgame player extends the list.
pub const RULES: &[&str] = &[
    "decisions",
    "forced",
    "denier:switch",
    "denier:ko",
    "denier:anti_setup",
    "denier:priority_ko",
    "denier:hardest_hit",
    "futile_switch",
    "trade_switch",
    "threatened_switch",
    "recover",
    "attack",
    "switch_blocked",
    "recover_blocked",
    "phase",
];
pub const R_DECISIONS: usize = 0;
const R_FORCED: usize = 1;
const R_DENY_SWITCH: usize = 2;
const R_DENY_KO: usize = 3;
const R_DENY_ANTI: usize = 4;
const R_DENY_PRIO: usize = 5;
const R_DENY_HARD: usize = 6;
const R_FUTILE: usize = 7;
const R_TRADE: usize = 8;
const R_THREAT: usize = 9;
const R_RECOVER: usize = 10;
const R_ATTACK: usize = 11;
const R_SW_BLOCKED: usize = 12;
const R_REC_BLOCKED: usize = 13;
const R_PHASE: usize = 14;

pub fn is_recovery(mv: MoveId) -> bool {
    RECOVERY.contains(&mv.id())
}

/// Fraction a recovery move heals: half, or the weather share for Synthesis and the like.
pub fn heal_fraction(b: &Battle, mv: MoveId) -> f32 {
    let Some(w) = b.field.weather.filter(|_| WEATHER_HEALS.contains(&mv.id())) else { return HALF };
    if matches!(w.cond.id(), "sunnyday" | "desolateland") {
        SUN_HEAL
    } else {
        WEATHER_HEAL
    }
}

/// A positive stage on atk, def, spa, spd or spe.
pub fn boosted(b: &Battle, side: Side) -> bool {
    b.side(side).active.boosts[..5].iter().any(|&x| x > 0)
}

/// One decision's inputs.
pub struct Ctx<'a> {
    pub b: &'a Battle,
    pub l: &'a Legal,
    pub me: Who,
    pub foe: Who,
    pub moves: [usize; 4],
    pub n_moves: usize,
    pub sw: [(usize, Who); 3],
    pub n_sw: usize,
}

impl<'a> Ctx<'a> {
    pub fn moves(&self) -> &[usize] {
        &self.moves[..self.n_moves]
    }
    pub fn switches(&self) -> &[(usize, Who)] {
        &self.sw[..self.n_sw]
    }
    pub fn mv(&self, a: usize) -> Option<MoveId> {
        action_move(self.b, self.l, self.me, a)
    }
    /// The first legal move action using `mv`.
    pub fn by_id(&self, mv: Option<MoveId>) -> Option<usize> {
        let mv = mv?;
        self.moves().iter().copied().find(|&a| self.mv(a) == Some(mv))
    }
}

/// Share of `member`'s current HP that the foe's best revealed hit takes.
fn taken(c: &Ctx, est: &mut Estimator, member: Who) -> f32 {
    best_hit(c.b, est, c.foe, member).0 / hp(c.b, member).max(MIN_HP)
}

/// Our best hit among the legal moves.
fn legal_hit(c: &Ctx, est: &mut Estimator) -> f32 {
    c.moves().iter().filter_map(|&a| c.mv(a)).map(|mv| hit_fraction(c.b, est, c.me, mv, c.foe)).fold(0.0, f32::max)
}

/// Lowest `taken` among the members that pass `keep`; first on ties.
fn least_taken(c: &Ctx, est: &mut Estimator, mut keep: impl FnMut(&mut Estimator, Who, f32) -> bool) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for &(a, w) in c.switches() {
        let t = taken(c, est, w);
        if keep(est, w, t) && best.map_or(true, |(x, _)| t < x) {
            best = Some((t, a));
        }
    }
    best.map(|x| x.1)
}

pub struct Rules<'p> {
    pub p: &'p DisciplinedParams,
    pub fired: &'p mut Fired,
}

impl<'p> Rules<'p> {
    fn take_switch(&mut self, c: &Ctx, a: Option<usize>, force: bool) -> Option<usize> {
        let a = a?;
        if !force && blocked(c.b, self.p.max_streak).is_some() {
            self.fired.hit(R_SW_BLOCKED);
            return None;
        }
        Some(a)
    }

    /// D-1c: an anti-setup move that can work on the foe now.
    fn denies(&self, c: &Ctx, mv: MoveId) -> bool {
        let id = mv.id();
        if !ANTI_SETUP.contains(&id) {
            return false;
        }
        if CLEARS_BOOSTS.contains(&id) && !boosted(c.b, Side::Opp) {
            return false;
        }
        if PHAZES.contains(&id) && left(c.b, Side::Opp) < PHAZE_TARGETS {
            return false;
        }
        let on = |cond| has_vol(c.b, Side::Opp, cond).is_some();
        if (id == "taunt" && on(conds::TAUNT)) || (id == "encore" && on(conds::ENCORE)) || (id == "perishsong" && on(conds::PERISHSONG)) {
            return false;
        }
        let d = mv.data();
        d.category == Category::Status || c.b.types(c.foe).map_or(true, |t| type_mult(d.ty, &t) > 0.0)
    }

    fn deny(&mut self, c: &Ctx, est: &mut Estimator) -> Option<usize> {
        let p = *self.p;
        // (a) a switch-in that takes little and KOs
        let pick = least_taken(c, est, |est, w, t| t < p.denier_switch_in_max && can_ko(c.b, est, w, c.foe));
        if let Some(a) = self.take_switch(c, pick, false) {
            self.fired.hit(R_DENY_SWITCH);
            return Some(a);
        }
        // (b) KO now
        if can_ko(c.b, est, c.me, c.foe) && (faster(c.b, est, c.me, c.foe) || !can_ko(c.b, est, c.foe, c.me)) {
            if let Some(a) = c.by_id(best_hit(c.b, est, c.me, c.foe).1) {
                self.fired.hit(R_DENY_KO);
                return Some(a);
            }
        }
        // (c) anti-setup
        if let Some(a) = c.moves().iter().copied().find(|&a| c.mv(a).map_or(false, |mv| self.denies(c, mv))) {
            self.fired.hit(R_DENY_ANTI);
            return Some(a);
        }
        // (d) priority KO
        if let Some(a) = c.by_id(priority_ko(c.b, est, c.me, c.foe)) {
            self.fired.hit(R_DENY_PRIO);
            return Some(a);
        }
        self.fired.hit(R_DENY_HARD);
        None
    }

    fn futile(&mut self, c: &Ctx, est: &mut Estimator) -> Option<usize> {
        if c.n_sw == 0 || legal_hit(c, est) >= self.p.futile_hit {
            return None;
        }
        let mut best: Option<(f32, usize)> = None;
        for &(a, w) in c.switches() {
            let s = best_hit(c.b, est, w, c.foe).0;
            if best.map_or(true, |(x, _)| s > x) {
                best = Some((s, a));
            }
        }
        self.take_switch(c, best.map(|x| x.1), true)
    }

    fn trade(&mut self, c: &Ctx, est: &mut Estimator) -> Option<usize> {
        if c.n_sw == 0 || legal_hit(c, est) >= self.p.trade_hit_min {
            return None;
        }
        if !threatened(c.b, est, c.me, c.foe, self.p.threat_threshold) {
            return None;
        }
        let relaxed = self.p.switch_in_relaxed;
        let pick = least_taken(c, est, |_, _, t| t < relaxed);
        self.take_switch(c, pick, false)
    }

    fn threatened_switch(&mut self, c: &Ctx, est: &mut Estimator) -> Option<usize> {
        if c.n_sw == 0 || !threatened(c.b, est, c.me, c.foe, self.p.threat_threshold) || can_ko(c.b, est, c.me, c.foe) {
            return None;
        }
        let p = *self.p;
        let pick = least_taken(c, est, |est, w, t| t < p.switch_in_max && best_hit(c.b, est, w, c.foe).0 >= p.switch_in_threatens);
        self.take_switch(c, pick, false)
    }

    fn recover(&mut self, c: &Ctx, est: &mut Estimator) -> Option<usize> {
        let a = c.moves().iter().copied().find(|&a| c.mv(a).map_or(false, is_recovery))?;
        let mine = hp(c.b, c.me);
        if mine >= self.p.recover_below {
            return None;
        }
        let healed = (mine + heal_fraction(c.b, c.mv(a)?)).min(1.0);
        if best_hit(c.b, est, c.foe, c.me).0 >= healed || can_ko(c.b, est, c.me, c.foe) {
            return None;
        }
        if left(c.b, Side::Own) > left(c.b, Side::Opp) {
            self.fired.hit(R_REC_BLOCKED);
            return None;
        }
        Some(a)
    }

    /// D-0..D-5 (None: max damage decides).
    pub fn decide(&mut self, c: &Ctx, est: &mut Estimator) -> Option<usize> {
        let theirs_setup = c.b.mon(c.foe).moves.iter().flatten().any(|&m| lists::is(m, class::SETUP));
        if boosted(c.b, Side::Opp) || theirs_setup {
            return self.deny(c, est);
        }
        if let Some(a) = self.futile(c, est) {
            self.fired.hit(R_FUTILE);
            return Some(a);
        }
        if let Some(a) = self.trade(c, est) {
            self.fired.hit(R_TRADE);
            return Some(a);
        }
        if let Some(a) = self.threatened_switch(c, est) {
            self.fired.hit(R_THREAT);
            return Some(a);
        }
        if let Some(a) = self.recover(c, est) {
            self.fired.hit(R_RECOVER);
            return Some(a);
        }
        self.fired.hit(R_ATTACK);
        None
    }
}

/// The decision's context, or None for D-0 (no legal move, or an active missing).
pub fn context<'a>(b: &'a Battle, l: &'a Legal, opts: &Options) -> Option<Ctx<'a>> {
    let (me, foe) = (b.active(Side::Own)?, b.active(Side::Opp)?);
    let (moves, n_moves) = move_actions(l, opts);
    if n_moves == 0 {
        return None;
    }
    let (sw, n_sw) = switches(b, l);
    Some(Ctx { b, l, me, foe, moves, n_moves, sw, n_sw })
}

/// Ahead in members: status moves dropped while an attack is legal.
fn phase_filtered(b: &Battle, l: &Legal, fired: &mut Fired) -> Legal {
    if left(b, Side::Own) <= left(b, Side::Opp) {
        return *l;
    }
    let Some(me) = b.active(Side::Own) else { return *l };
    let status = |a: usize| action_move(b, l, me, a).map_or(false, |m| m.data().category == Category::Status);
    let attacks = (0..8).any(|a| l.mask[a] && !status(a));
    if !attacks {
        return *l;
    }
    let mut out = *l;
    let mut dropped = false;
    for a in 0..8 {
        if out.mask[a] && status(a) {
            out.mask[a] = false;
            dropped = true;
        }
    }
    if dropped {
        fired.hit(R_PHASE);
    }
    out
}

/// One decision of disciplined, or of endgame when `endgame` is given.
pub fn choose(b: &Battle, l: &Legal, opts: &Options, p: &DisciplinedParams, eg: Option<&EndgameParams>, est: &mut Estimator, fired: &mut Fired) -> usize {
    fired.hit(R_DECISIONS);
    if l.count() == 0 {
        return 0;
    }
    if let Some(a) = single(l) {
        return a;
    }
    if let Some(e) = eg {
        if let Some(a) = endgame::pick(b, l, opts, p, e, est, fired) {
            return a;
        }
    }
    let chosen = match context(b, l, opts) {
        Some(c) => Rules { p, fired: &mut *fired }.decide(&c, est),
        None => {
            fired.hit(R_FORCED);
            None
        }
    };
    if let Some(a) = chosen {
        return a;
    }
    let f = phase_filtered(b, l, fired);
    max_damage::choose(b, &f, opts, est)
}

pub struct DisciplinedPlayer {
    pub opts: Options,
    pub params: DisciplinedParams,
    est: Estimator,
    pub fired: Fired,
}

impl Default for DisciplinedPlayer {
    fn default() -> Self {
        DisciplinedPlayer::new(Options::default(), DisciplinedParams::default())
    }
}

impl DisciplinedPlayer {
    pub fn new(opts: Options, params: DisciplinedParams) -> Self {
        DisciplinedPlayer { opts, params, est: Estimator::default(), fired: Fired::new(RULES) }
    }
}

impl Player for DisciplinedPlayer {
    fn name(&self) -> &'static str {
        "disciplined"
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        preview_picks(b)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, _rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        self.est.reset(b);
        choose(b, &l, &self.opts, &self.params, None, &mut self.est, &mut self.fired)
    }
    fn fired(&self) -> Vec<(&'static str, u64)> {
        self.fired.list()
    }
}
