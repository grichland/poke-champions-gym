//! Tower (spec section 2, T-0..T-13) over the shared estimate. First rule that fires wins:
//!
//! - T-0: preview as max damage.
//! - T-1: no foe on the field: max damage. No own active (a replacement): straight to T-10.
//! - T-2: Perish count 1 and a switch is legal: a uniform random switch.
//! - T-4: usable moves (Taunt, Dream Eater, screens up, healing at full HP, first-turn moves, hazard caps,
//!   status on a statused foe, setup at +6; plus PS-Q4 "cannot work" and PS-B6 repeats).
//! - T-5/T-6: sort into `ko`, `high`, `status` and `best`.
//! - T-7: a KO, priority ones first, uniform. T-8: uniform over `high` + `best`. T-9: uniform over
//!   `status` + `best`. T-10: switch weighted by super-effective moves. T-11: max damage.
//!
//! The pivot-switch rule (T-3) is not built.

use crate::estimate::{priority, Estimator};
use crate::lists::{self, class};
use crate::max_damage::{self, action_move, move_actions, single};
use crate::usable::cannot_work;
use crate::view::{has_vol, outspeeds, Fighter};
use crate::{Options, Player, Rng};
use battle::{legal, Battle, Legal, ReqKind, Side, VolDetail, Who, N_ACTIONS, SIDE_CONDS};
use cg_engine::sim::data::*;

/// T-4: healing moves are dropped at this HP fraction or above.
pub const FULL_HP: f32 = 0.99;
/// T-2: the Perish count that sends the active out.
pub const PERISH_ESCAPE: i16 = 1;

#[derive(Default)]
pub struct TowerPlayer {
    pub opts: Options,
    est: Estimator,
}

impl TowerPlayer {
    pub fn new(opts: Options) -> Self {
        TowerPlayer { opts, est: Estimator::default() }
    }
}

/// The lists one decision sorts the usable moves into (action indices).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sorted {
    pub usable: Vec<usize>,
    pub ko: Vec<usize>,
    pub high: Vec<usize>,
    pub status: Vec<usize>,
    pub best: Option<usize>,
}

fn side_cond(b: &Battle, side: Side, c: Cond) -> Option<u8> {
    let i = SIDE_CONDS.iter().position(|s| *s == c.id())?;
    b.side(side).conds[i].map(|x| x.layers)
}

/// T-12: the foe cannot KO us with a revealed move (highest roll x most hits, no accuracy).
pub fn survives(b: &Battle, est: &mut Estimator, me: Who, foe: Who, opts: &Options) -> bool {
    let our_hp = b.mon(me).hp / 100.0;
    let mut any_damaging = false;
    for mv in b.mon(foe).moves.iter().flatten() {
        if mv.data().category == Category::Status {
            continue;
        }
        any_damaging = true;
        let h = est.hit(b, foe, false, *mv, me);
        if h.max_frac() > 0.0 && h.max_frac() >= our_hp {
            return false;
        }
    }
    any_damaging || opts.setup_safe_without_revealed_attacks
}

/// T-4: whether a legal move passes tower's filter.
pub fn usable(b: &Battle, est: &mut Estimator, opts: &Options, me: Who, foe: Who, mv: MoveId, user: &Fighter, target: &Fighter) -> bool {
    let d = mv.data();
    let our_hp = b.mon(me).hp / 100.0;
    let a = &b.side(Side::Own).active;
    if d.category == Category::Status && has_vol(b, Side::Own, conds::TAUNT).is_some() {
        return false;
    }
    if d.id == "dreameater" && b.mon(foe).status != Status::Slp && target.ab() != "comatose" {
        return false;
    }
    if lists::is(mv, class::SCREEN) && side_cond(b, Side::Own, d.side_condition).is_some() {
        return false;
    }
    if lists::is(mv, class::RECOVERY) && our_hp >= FULL_HP {
        return false;
    }
    if lists::is(mv, class::FIRST_TURN) && a.move_actions > 0 {
        return false;
    }
    if let Some((c, cap)) = lists::hazard_cap(mv) {
        if side_cond(b, Side::Opp, c).map_or(false, |n| n >= cap) {
            return false;
        }
    }
    // the status check of `reasonable` applies to status moves only (PS-B5: Nuzzle keeps its damage)
    if d.category == Category::Status && lists::is(mv, class::INFLICT) && d.volatile_status != conds::CONFUSION && d.volatile_status != conds::ATTRACT {
        if b.mon(foe).status != Status::None || (d.volatile_status == conds::YAWN && has_vol(b, Side::Opp, conds::YAWN).is_some()) {
            return false;
        }
    }
    if lists::is(mv, class::SETUP) {
        let r = lists::raised(mv);
        if (0..7).any(|i| r[i]) && (0..7).filter(|&i| r[i]).all(|i| a.boosts[i] >= 6) {
            return false;
        }
    }
    if opts.skip_status_that_cannot_work || opts.skip_repeat_volatile {
        let field = est.field;
        if let Some(why) = cannot_work(b, &field, user, mv, target, opts.skip_repeat_volatile) {
            let repeat_only = why == "already has it";
            if (repeat_only && opts.skip_repeat_volatile) || (!repeat_only && opts.skip_status_that_cannot_work) {
                return false;
            }
        }
    }
    true
}

/// T-6 for a status move.
fn high_status(b: &Battle, est: &mut Estimator, opts: &Options, me: Who, foe: Who, mv: MoveId, faster: bool) -> bool {
    if lists::is(mv, class::SPEED_DOWN) && !faster {
        return true;
    }
    if lists::is(mv, class::SETUP) {
        return survives(b, est, me, foe, opts);
    }
    if lists::is(mv, class::INFLICT) {
        return b.mon(foe).status == Status::None || lists::is(mv, class::ALWAYS_HIGH);
    }
    lists::is(mv, class::SCREEN) || lists::is(mv, class::FIRST_TURN)
}

/// T-4 to T-6: the usable moves sorted into tower's lists.
pub fn sort_moves(b: &Battle, l: &Legal, opts: &Options, est: &mut Estimator, me: Who, foe: Who) -> Sorted {
    let mut s = Sorted::default();
    let (cands, n) = move_actions(l, opts);
    let target = est.fighter(b, foe, false);
    let foe_hp = b.mon(foe).hp / 100.0;
    let mut best_dmg = 0.0f32;
    for &a in &cands[..n] {
        let Some(mv) = action_move(b, l, me, a) else { continue };
        let mega = a >= 4;
        let user = est.fighter(b, me, mega);
        if !usable(b, est, opts, me, foe, mv, &user, &target) {
            continue;
        }
        s.usable.push(a);
        let field = est.field;
        let faster = outspeeds(&user, &target, &field);
        if mv.data().category == Category::Status {
            if high_status(b, est, opts, me, foe, mv, faster) {
                s.high.push(a);
            } else {
                s.status.push(a);
            }
            continue;
        }
        let dmg = est.hit(b, me, mega, mv, foe).expected();
        if dmg > 0.0 && dmg >= foe_hp {
            s.ko.push(a);
        }
        if lists::is(mv, class::SPEED_DOWN) && !faster {
            s.high.push(a);
        }
        if lists::is(mv, class::FIRST_TURN) {
            s.high.push(a);
        }
        if dmg > best_dmg {
            best_dmg = dmg;
            s.best = Some(a);
        }
    }
    s
}

/// T-10: switches weighted by the number of the member's damaging moves that are super effective on the
/// foe's types (type chart only); uniform when every weight is 0.
pub fn smart_switch(b: &Battle, l: &Legal, foe: Option<Who>, rng: &mut Rng) -> Option<usize> {
    let mut sw = [0usize; 3];
    let mut weights = [0u32; 3];
    let mut n = 0;
    let foe_types = foe.and_then(|f| b.types(f));
    for a in 8..N_ACTIONS {
        if !l.mask[a] {
            continue;
        }
        let Some(slot) = b.picks[a - 8] else { continue };
        let mut w = 0;
        if let Some(ft) = foe_types {
            for mv in b.mon(Who { side: Side::Own, slot }).moves.iter().flatten() {
                let d = mv.data();
                if d.category == Category::Status {
                    continue;
                }
                let real: Vec<Type> = ft.iter().copied().filter(|t| t.is_real()).collect();
                let affects = real.iter().all(|&t| type_affects(d.ty, t));
                let steps: i32 = real.iter().map(|&t| effectiveness(d.ty, t)).sum();
                if affects && steps > 0 {
                    w += 1;
                }
            }
        }
        sw[n] = a;
        weights[n] = w;
        n += 1;
    }
    if n == 0 {
        return None;
    }
    let total: u32 = weights[..n].iter().sum();
    if total == 0 {
        return Some(sw[rng.below(n)]);
    }
    let mut r = rng.below(total as usize) as u32;
    for i in 0..n {
        if r < weights[i] {
            return Some(sw[i]);
        }
        r -= weights[i];
    }
    Some(sw[n - 1])
}

/// T-1..T-11 over a precomputed legal set.
pub fn choose(b: &Battle, l: &Legal, opts: &Options, est: &mut Estimator, rng: &mut Rng) -> usize {
    if l.count() == 0 {
        return 0;
    }
    if let Some(a) = single(l) {
        return a;
    }
    let foe = b.active(Side::Opp);
    // at a replacement asked before its rows our Pokémon on the field is the one leaving
    let me = b.active(Side::Own).filter(|_| !battle::legal::replace_unread(b, l));
    let Some(foe) = foe else { return max_damage::choose(b, l, opts, est) };
    let Some(me) = me else {
        return smart_switch(b, l, Some(foe), rng).unwrap_or_else(|| max_damage::choose(b, l, opts, est));
    };
    let switches: Vec<usize> = (8..N_ACTIONS).filter(|&i| l.mask[i]).collect();
    // T-2
    if has_vol(b, Side::Own, conds::PERISHSONG) == Some(VolDetail::Num(PERISH_ESCAPE)) && !switches.is_empty() {
        return rng.pick(&switches);
    }
    let s = sort_moves(b, l, opts, est, me, foe);
    // T-7
    if !s.ko.is_empty() {
        let user = est.fighter(b, me, false);
        let field = est.field;
        let fast: Vec<usize> =
            s.ko.iter().copied().filter(|&a| action_move(b, l, me, a).map_or(false, |mv| priority(&user, mv, &field) > 0)).collect();
        return rng.pick(if fast.is_empty() { &s.ko } else { &fast });
    }
    // T-8
    if !s.high.is_empty() {
        let mut pool = s.high.clone();
        pool.extend(s.best);
        return rng.pick(&pool);
    }
    // T-9
    let mut pool = s.status.clone();
    pool.extend(s.best);
    if !pool.is_empty() {
        return rng.pick(&pool);
    }
    // T-10
    if let Some(a) = smart_switch(b, l, Some(foe), rng) {
        return a;
    }
    // T-11
    max_damage::choose(b, l, opts, est)
}

impl Player for TowerPlayer {
    fn name(&self) -> &'static str {
        "tower"
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        max_damage::preview_picks(b)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        self.est.reset(b);
        choose(b, &l, &self.opts, &mut self.est, rng)
    }
}

/// The lists for the current request, for tests and inspection.
pub fn inspect(b: &Battle, kind: ReqKind, opts: &Options) -> Option<Sorted> {
    let l = legal(b, kind);
    let mut est = Estimator::new(b);
    Some(sort_moves(b, &l, opts, &mut est, b.active(Side::Own)?, b.active(Side::Opp)?))
}
