//! Training labels (model contract section 5): what each seat chose, the other seat's choice of the same
//! step as one of 13 actions seen from this seat's side, and the truth of the opponent's six. Never inputs.
//!
//! Matching: every request of a game has a decision id (`Slot::decision`, per game, from 1, never reset
//! across episodes). `observe_batch` returns the pending decision's id with its tensors. A decision's
//! label is complete once its step has resolved (both seats of a turn have chosen, a replace is in, or at
//! preview both seats have picked), and appears in `BatchEnv::labels()` right after the `act` call that
//! completed it, with the same decision id. One `act` can complete two decisions of a game (seat 0 and
//! seat 1 of the same turn), so labels are per game and seat. A decision whose action the State refused is
//! labelled at once with `refused` set and nothing else; the seat is asked again under a new id. A
//! replace request has no opposing choice (`opp_action` -1), as the contract says, even when both seats
//! replace at once.
//!
//! Opponent actions: 0..4 their active (row 6) used the move in its slot 0..4; 4..8 the same with Mega;
//! 8..13 they switched to the Pokémon of row 7..12. Their slots fill in reveal order, so a move not yet
//! revealed at the decision gets the first empty slot of row 6. Struggle and Recharge have no slot:
//! `opp_action` -1 with `opp_move` Struggle's id (or 0 for Recharge).
//!
//! `opp_prior` is the observation's input of the same name at that decision, repeated here.
//! Truth (`opp_true_*`) is indexed by the decision's opponent rows 6..12 (index 0 = row 6).

use crate::observe::PREVIEW0;
use battle::*;
use obs::vocab;
use cg_engine::sim::battle::STRUGGLE;
use cg_engine::sim::data::*;
use cg_engine::sim::state::{State, SIDES};
use cg_engine::sim::{Choice, MonSpec};

pub const N_OPP_ACTIONS: usize = obs::OPP_ACTIONS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label {
    /// a decision of this seat was completed in the last `act`
    pub valid: bool,
    pub decision: u64,
    /// the State refused this action; the seat was asked again
    pub refused: bool,
    /// 0..71 in the one action space
    pub own_action: i16,
    /// 0..13 as above; -1 no opposing choice (or Struggle, Recharge)
    pub opp_action: i8,
    /// the opponent had nothing to choose (locked, charging, recharging)
    pub opp_forced: u8,
    /// obs move id of the move chosen or forced; 0 for a switch, Recharge or no choice
    pub opp_move: i32,
    /// the events this seat was shown during the step revealed the opposing choice
    pub opp_shown: u8,
    /// the observation's opp_prior at the decision
    pub opp_prior: [f32; N_OPP_ACTIONS],
    /// at preview: the other seat's picks, indices into its preview order, lead first
    pub opp_picks: [i8; 3],
    /// by opponent row 6..12: brought, four move ids, item id, ability id (obs vocabularies)
    pub opp_true_brought: [u8; 6],
    pub opp_true_moves: [[i32; 4]; 6],
    pub opp_true_item: [i32; 6],
    pub opp_true_ability: [i32; 6],
}

impl Default for Label {
    fn default() -> Self {
        Label {
            valid: false,
            decision: 0,
            refused: false,
            own_action: -1,
            opp_action: -1,
            opp_forced: 0,
            opp_move: 0,
            opp_shown: 0,
            opp_prior: [0.0; N_OPP_ACTIONS],
            opp_picks: [-1; 3],
            opp_true_brought: [0; 6],
            opp_true_moves: [[0; 4]; 6],
            opp_true_item: [0; 6],
            opp_true_ability: [0; 6],
        }
    }
}

/// A seat's decision as asked, kept until its step resolves.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asked {
    pub decision: u64,
    /// in the one action space
    pub action: u16,
    /// a replace request (no opposing choice)
    pub replace: bool,
    /// row of each opposing team slot (preview index) in this decision's observation
    pub opp_rows: [i8; 6],
    /// opposing team slot of each row 6..12
    pub row_slots: [i8; 6],
    /// row 6's move slots at the decision (as observed: the copied moves while transformed)
    pub opp_moves: [Option<MoveId>; 4],
    /// each opposing Pokémon's own revealed moves by team slot
    pub revealed: [[Option<MoveId>; 4]; 6],
    pub opp_prior: [f32; N_OPP_ACTIONS],
}

/// What a seat did at the step under way.
#[derive(Clone, Copy, Debug, Default)]
pub enum Took {
    #[default]
    Nothing,
    Asked(Asked),
    /// not asked: locked, charging or recharging
    Locked,
}

/// The move slots of a Pokémon as the observation shows them (copied ones while transformed on the field).
fn slots(b: &Battle, w: Who) -> [Option<MoveId>; 4] {
    let a = &b.side(w.side).active;
    if a.slot == Some(w.slot) && !a.out && a.transformed.is_some() {
        a.t_moves
    } else {
        b.mon(w).moves
    }
}

/// A seat's decision, from its Battle at the request.
pub fn asked(b: &Battle, decision: u64, replace: bool) -> Asked {
    let rows = obs::row_order(b);
    let mut a = Asked { decision, replace, opp_rows: [-1; 6], row_slots: [-1; 6], ..Asked::default() };
    for (r, w) in rows.iter().enumerate().skip(6) {
        if let Some(w) = w {
            a.opp_rows[w.slot as usize % 6] = r as i8;
            a.row_slots[r - 6] = w.slot as i8;
        }
    }
    a.opp_moves = b.active(Side::Opp).map_or([None; 4], |w| slots(b, w));
    for t in 0..6 {
        a.revealed[t] = b.sides[1].team[t].moves;
    }
    let mut p = [0f32; N_OPP_ACTIONS];
    obs::opp_prior(b, &mut p);
    a.opp_prior = p;
    a
}

/// The opponent's truth by row of the decision: revealed moves of the set in their slots, the rest of the
/// set in set order in the empty slots.
fn truth(a: &Asked, roster: &[MonSpec], picks: Option<[u8; 3]>, l: &mut Label) {
    for i in 0..6 {
        let Ok(t) = usize::try_from(a.row_slots[i]) else { continue };
        let spec = &roster[t];
        l.opp_true_brought[i] = picks.map_or(false, |p| p.contains(&(t as u8))) as u8;
        let set = &spec.moves[..spec.nmoves as usize];
        // a revealed move outside the set (a called move the Battle kept, e.g. Uproar via Copycat) is not
        // the Pokémon's: its slot takes a set move
        let mut mv = a.revealed[t].map(|m| m.filter(|x| set.contains(x)));
        for &m in set {
            if !mv.contains(&Some(m)) {
                if let Some(e) = mv.iter_mut().find(|x| x.is_none()) {
                    *e = Some(m);
                }
            }
        }
        l.opp_true_moves[i] = mv.map(|m| vocab::mv(m));
        l.opp_true_item[i] = vocab::item(ItemState::Held(spec.item));
        l.opp_true_ability[i] = vocab::ability(Some(spec.ability));
    }
}

fn base(a: &Asked) -> Label {
    Label { valid: true, decision: a.decision, own_action: a.action as i16, opp_prior: a.opp_prior, ..Label::default() }
}

/// Labels of the step just resolved. `s0` is the State before it; `shown[seat]` the events emitted to
/// that seat during the step.
#[allow(clippy::too_many_arguments)]
pub fn step_labels(
    s0: &State,
    took: &[Took; SIDES],
    chosen: &[Option<Choice>; SIDES],
    picks: &[Option<[u8; 3]>; SIDES],
    rosters: [&[MonSpec]; SIDES],
    shown: [&[Event]; SIDES],
    out: &mut [Label; SIDES],
) {
    for seat in 0..SIDES {
        let Took::Asked(a) = took[seat] else { continue };
        let o = 1 - seat;
        let mut l = base(&a);
        truth(&a, rosters[o], picks[o], &mut l);
        let c = if a.replace { None } else { chosen[o] };
        let Some(c) = c else {
            out[seat] = l;
            continue;
        };
        l.opp_forced = matches!(took[o], Took::Locked) as u8;
        match c {
            Choice::Move { index, mega } => {
                let mv = if index == STRUGGLE { s0.locked_move(o).unwrap_or(moves::STRUGGLE) } else { s0.active(o).moves[index as usize].id };
                let recharge = mv.is_none();
                l.opp_move = if recharge { 0 } else { vocab::mv(Some(mv)) };
                if !recharge && mv != moves::STRUGGLE {
                    let slot = a.opp_moves.iter().position(|x| *x == Some(mv)).or_else(|| a.opp_moves.iter().position(|x| x.is_none()));
                    if let Some(j) = slot {
                        l.opp_action = j as i8 + if mega { 4 } else { 0 };
                    }
                }
                l.opp_shown = shown[seat].iter().any(|e| match e {
                    Event::Move { who, mv: m, called_by, .. } => !recharge && who.side == Side::Opp && called_by.is_none() && *m == mv.id(),
                    Event::Cant { who, reason } => recharge && who.as_ref().map_or(false, |w| w.side == Side::Opp) && reason == "recharge",
                    _ => false,
                }) as u8;
            }
            Choice::Switch(p) => {
                if let Some(pk) = picks[o] {
                    let r = a.opp_rows[pk[p as usize % 3] as usize % 6];
                    if (7..12).contains(&r) {
                        l.opp_action = 8 + (r - 7);
                    }
                }
                l.opp_shown = shown[seat].iter().any(|e| match e {
                    Event::Withdraw { who, .. } => who.side == Side::Opp,
                    Event::Enter { who, forced } => who.side == Side::Opp && !forced,
                    _ => false,
                }) as u8;
            }
        }
        out[seat] = l;
    }
}

/// Labels once both seats have picked.
pub fn preview_labels(took: &[Took; SIDES], picks: &[Option<[u8; 3]>; SIDES], rosters: [&[MonSpec]; SIDES], out: &mut [Label; SIDES]) {
    for seat in 0..SIDES {
        let Took::Asked(a) = took[seat] else { continue };
        let mut l = base(&a);
        if let Some(p) = picks[1 - seat] {
            l.opp_picks = [p[0] as i8, p[1] as i8, p[2] as i8];
        }
        truth(&a, rosters[1 - seat], picks[1 - seat], &mut l);
        out[seat] = l;
    }
}

pub fn refused_label(decision: u64, action: u16) -> Label {
    Label { valid: true, decision, refused: true, own_action: action as i16, ..Label::default() }
}

pub fn preview_action_code(a: u16) -> u16 {
    a + PREVIEW0 as u16
}
