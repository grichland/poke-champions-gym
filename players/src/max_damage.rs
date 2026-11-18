//! Max damage (spec section 1, MD-1..MD-10) over the shared estimate (PS-Q1).
//!
//! - MD-2: the legal move with the highest expected fraction against the foe.
//! - MD-3 / PS-Q3: with Mega offered, only the Mega variants are candidates (scored as the Mega form).
//! - MD-4 / PS-Q6: ties and all-zero scores go to the lowest action index.
//! - MD-5: never switches while a move is legal.
//! - MD-6: forced switch to the bench member with the best attack on the foe (HP and damage taken ignored).
//! - MD-7: otherwise the lowest legal action.
//! - MD-9: preview by summed best attack over the opponent's preview species, stable, lead first.

use crate::estimate::{estimate, Estimator};
use crate::view::{Fighter, FieldView};
use crate::{first_legal, Options, Player, Rng};
use battle::{legal, Battle, Forced, Legal, ReqKind, Side, Who, N_ACTIONS};
use cg_engine::sim::data::*;

#[derive(Default)]
pub struct MaxDamagePlayer {
    pub opts: Options,
    est: Estimator,
}

impl MaxDamagePlayer {
    pub fn new(opts: Options) -> Self {
        MaxDamagePlayer { opts, est: Estimator::default() }
    }
}

/// Own moves of the active Pokémon (its copied moves while transformed), by slot.
pub fn own_moves(b: &Battle, w: Who) -> [Option<MoveId>; 4] {
    let a = &b.side(Side::Own).active;
    if a.slot == Some(w.slot) && a.transformed.is_some() {
        a.t_moves
    } else {
        b.mon(w).moves
    }
}

/// The move an action stands for: Struggle or the forced move for action 0 when forced, else the slot.
pub fn action_move(b: &Battle, l: &Legal, me: Who, a: usize) -> Option<MoveId> {
    if a == 0 {
        match l.forced {
            Some(Forced::Struggle) => return Some(moves::STRUGGLE),
            Some(Forced::Move(mv)) => return Some(mv),
            Some(Forced::Recharge) => return None,
            None => {}
        }
    }
    if a >= 8 {
        return None;
    }
    own_moves(b, me)[a % 4]
}

/// The only legal action, when there is exactly one.
pub fn single(l: &Legal) -> Option<usize> {
    if l.count() == 1 {
        l.mask.iter().position(|x| *x)
    } else {
        None
    }
}

/// The move candidates: plain slots, or their Mega variants when Mega is offered and taken.
pub fn move_actions(l: &Legal, opts: &Options) -> ([usize; 4], usize) {
    let mega = l.can_mega && opts.mega_when_offered && l.mask[4..8].iter().any(|x| *x);
    let base = if mega { 4 } else { 0 };
    let mut out = [0; 4];
    let mut n = 0;
    for i in base..base + 4 {
        if l.mask[i] {
            out[n] = i;
            n += 1;
        }
    }
    (out, n)
}

/// Best expected fraction of one Pokémon's damaging moves against a defender (0 when none).
pub fn best_attack(field: &FieldView, atk: &Fighter, moves: &[Option<MoveId>; 4], def: &Fighter, b: Option<&Battle>) -> f32 {
    moves.iter().flatten().map(|&mv| estimate(field, atk, mv, def, b).expected()).fold(0.0, f32::max)
}

/// MD-9: picks by summed best attack over the opponent's preview, ties keep the lower index.
pub fn preview_picks(b: &Battle) -> [u8; 3] {
    let field = FieldView::default();
    let n = (b.n_own as usize).clamp(1, 6);
    let foes: Vec<Fighter> =
        b.side(Side::Opp).team.iter().filter_map(|m| m.preview.or(m.species)).map(|sp| Fighter::species(sp, Side::Opp)).collect();
    let mut scored: Vec<(f32, usize)> = (0..n)
        .map(|i| {
            let mut me = Fighter::own(b, i, false);
            me.form = b.sets[i].species;
            let moves = b.sets[i].moves;
            let s: f32 = foes.iter().map(|f| best_attack(&field, &me, &moves, f, None)).sum();
            (s, i)
        })
        .collect();
    scored.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal).then(x.1.cmp(&y.1)));
    let at = |k: usize| scored[k.min(scored.len() - 1)].1 as u8;
    [at(0), at(1), at(2)]
}

/// MD-2..MD-7 over a precomputed legal set.
pub fn choose(b: &Battle, l: &Legal, opts: &Options, est: &mut Estimator) -> usize {
    if l.count() == 0 {
        return 0;
    }
    if let Some(a) = single(l) {
        return a;
    }
    let me = b.active(Side::Own);
    let foe = b.active(Side::Opp);
    let (cands, n) = move_actions(l, opts);
    if n > 0 {
        let (Some(me), Some(foe)) = (me, foe) else { return cands[0] };
        let mut best = cands[0];
        let mut best_s = f32::NEG_INFINITY;
        for &a in &cands[..n] {
            let Some(mv) = action_move(b, l, me, a) else { continue };
            let s = est.hit(b, me, a >= 4, mv, foe).expected();
            if s > best_s {
                best_s = s;
                best = a;
            }
        }
        return best;
    }
    if let (Some(foe), true) = (foe, l.mask[8..].iter().any(|x| *x)) {
        return best_switch(b, l, foe, est);
    }
    first_legal(&l.mask, 0..N_ACTIONS).unwrap_or(0)
}

/// MD-6: the switch whose Pokémon has the best attack on the foe; ties to the lower action.
pub fn best_switch(b: &Battle, l: &Legal, foe: Who, est: &mut Estimator) -> usize {
    let mut best = None;
    let mut best_s = f32::NEG_INFINITY;
    for a in 8..N_ACTIONS {
        if !l.mask[a] {
            continue;
        }
        let Some(slot) = b.picks[a - 8] else { continue };
        let me = Who { side: Side::Own, slot };
        let moves = b.mon(me).moves;
        let s = moves.iter().flatten().map(|&mv| est.hit(b, me, false, mv, foe).expected()).fold(0.0, f32::max);
        if s > best_s {
            best_s = s;
            best = Some(a);
        }
    }
    best.or_else(|| first_legal(&l.mask, 8..N_ACTIONS)).unwrap_or(0)
}

impl Player for MaxDamagePlayer {
    fn name(&self) -> &'static str {
        "max_damage"
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        preview_picks(b)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, _rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        self.est.reset(b);
        choose(b, &l, &self.opts, &mut self.est)
    }
}
