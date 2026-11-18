//! HP-fraction threat questions over the shared estimate (`pcsa/players/threat.py`): strongest known hit,
//! can it KO, priority KO, threatened. Known moves are our own four and the opponent's revealed ones (no
//! assumed sets). Every estimate goes through the decision's `Estimator` cache.

use crate::estimate::{priority, Estimator};
use crate::max_damage::own_moves;
use crate::view::outspeeds;
use battle::{Battle, Side, Who};
use cg_engine::sim::data::*;

/// Our member's moves (the copied ones while our active is transformed), or the opponent's revealed moves.
pub fn known_moves(b: &Battle, w: Who) -> [Option<MoveId>; 4] {
    let mut m = if w.side == Side::Own && b.side(Side::Own).active.slot == Some(w.slot) { own_moves(b, w) } else { b.mon(w).moves };
    for x in m.iter_mut() {
        if x.map_or(false, |mv| mv == moves::STRUGGLE) {
            *x = None;
        }
    }
    m
}

/// HP fraction of a Pokémon, 0..=1.
pub fn hp(b: &Battle, w: Who) -> f32 {
    if b.mon(w).fainted {
        0.0
    } else {
        b.mon(w).hp / 100.0
    }
}

/// Expected fraction of the defender's max HP (fixed roll x accuracy x hits); 0 when immune.
pub fn hit_fraction(b: &Battle, est: &mut Estimator, atk: Who, mv: MoveId, def: Who) -> f32 {
    est.hit(b, atk, false, mv, def).expected()
}

/// (expected fraction, move) of the attacker's strongest known move; (0, None) when none does damage.
pub fn best_hit(b: &Battle, est: &mut Estimator, atk: Who, def: Who) -> (f32, Option<MoveId>) {
    let mut best = (0.0, None);
    for mv in known_moves(b, atk).iter().flatten() {
        let s = hit_fraction(b, est, atk, *mv, def);
        if s > best.0 {
            best = (s, Some(*mv));
        }
    }
    best
}

/// Damage for a KO check: highest roll x most hits against us, fixed roll x expected hits on them; no accuracy.
fn ko_share(b: &Battle, est: &mut Estimator, atk: Who, mv: MoveId, def: Who) -> f32 {
    let h = est.hit(b, atk, false, mv, def);
    if def.side == Side::Own {
        h.max_frac()
    } else {
        h.frac() * h.hits
    }
}

/// Each known move that KOs the defender from its current HP, with its KO-check damage.
fn ko_moves(b: &Battle, est: &mut Estimator, atk: Who, def: Who) -> ([(f32, MoveId); 4], usize) {
    let mut out = [(0.0, MoveId::NONE); 4];
    let mut n = 0;
    let target = hp(b, def);
    if target <= 0.0 {
        return (out, 0);
    }
    for mv in known_moves(b, atk).iter().flatten() {
        let s = ko_share(b, est, atk, *mv, def);
        if s > 0.0 && s >= target {
            out[n] = (s, *mv);
            n += 1;
        }
    }
    (out, n)
}

pub fn can_ko(b: &Battle, est: &mut Estimator, atk: Who, def: Who) -> bool {
    ko_moves(b, est, atk, def).1 > 0
}

/// The attacker's strongest positive-priority known move that KOs the defender.
pub fn priority_ko(b: &Battle, est: &mut Estimator, atk: Who, def: Who) -> Option<MoveId> {
    let (k, n) = ko_moves(b, est, atk, def);
    let user = est.fighter(b, atk, false);
    let field = est.field;
    let mut best: Option<(f32, MoveId)> = None;
    for &(s, mv) in &k[..n] {
        if priority(&user, mv, &field) > 0 && best.map_or(true, |(x, _)| s > x) {
            best = Some((s, mv));
        }
    }
    best.map(|x| x.1)
}

/// `a` moves before `b` at equal priority (strictly; Trick Room reverses).
pub fn faster(b: &Battle, est: &mut Estimator, a: Who, d: Who) -> bool {
    let fa = est.fighter(b, a, false);
    let fd = est.fighter(b, d, false);
    outspeeds(&fa, &fd, &est.field)
}

/// Their best expected hit takes `threshold` of my max HP, or they can KO me.
pub fn threatened(b: &Battle, est: &mut Estimator, mine: Who, theirs: Who, threshold: f32) -> bool {
    best_hit(b, est, theirs, mine).0 >= threshold || can_ko(b, est, theirs, mine)
}

/// Type-chart multiplier of an attacking type on a defender's types (0 when a type is immune).
pub fn type_mult(ty: Type, def: &[Type; 2]) -> f32 {
    let mut m = 1.0f32;
    for &t in def.iter() {
        if !t.is_real() {
            continue;
        }
        if !type_affects(ty, t) {
            return 0.0;
        }
        m *= 2f32.powi(effectiveness(ty, t));
    }
    m
}

/// Members a side has left: three brought less its fainted ones.
pub fn left(b: &Battle, side: Side) -> i32 {
    let n = if side == Side::Own { (b.n_own as usize).min(6) } else { 6 };
    3 - b.side(side).team[..n].iter().filter(|m| m.fainted).count() as i32
}
