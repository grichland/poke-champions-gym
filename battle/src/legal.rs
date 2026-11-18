//! What our side may choose, by one rule from the Battle only (PROJECT.md "Observation features").
//!
//! Action layout (N_ACTIONS = 11):
//! - 0..=3: move slot 0..=3 of the active Pokémon (the set's order; its copied moves while transformed)
//! - 4..=7: the same move with Mega Evolution
//! - 8..=10: switch to `picks[0..=2]` (team-select picks, lead first); the active one is never legal
//!
//! When the active Pokémon must Struggle, recharge, or continue a locked or charged move that is not in
//! its slots, action 0 is the only legal action and `forced` names the move it stands for.

use crate::event::{Order, ReqKind, Side};
use crate::items;
use crate::state::*;
use cg_engine::sim::data::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forced {
    Struggle,
    Recharge,
    Move(MoveId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Legal {
    pub mask: [bool; N_ACTIONS],
    pub forced: Option<Forced>,
    /// switching is blocked by a trap the Battle knows of
    pub trapped: bool,
    pub can_mega: bool,
}

impl Legal {
    pub fn count(&self) -> usize {
        self.mask.iter().filter(|x| **x).count()
    }
}

fn has_vol(a: &Active, id: &str) -> Option<Vol> {
    a.vols.iter().flatten().find(|v| v.cond.id() == id).copied()
}

/// Our active Pokémon is on the ground (Arena Trap).
fn grounded(b: &Battle, w: Who) -> bool {
    let a = &b.side(w.side).active;
    let mon = b.mon(w);
    let held = |id: &str| matches!(mon.item, ItemState::Held(i) if i.id() == id) && !ignores_item(b, w);
    if b.field.rooms[R_GRAVITY].is_some() || held("ironball") || has_vol(a, "smackdown").is_some() || has_vol(a, "ingrain").is_some() {
        return true;
    }
    let flying = b.types(w).map_or(false, |t| t.contains(&Type::Flying));
    let levitate = mon.ability.map(|x| x.id()) == Some("levitate");
    !(flying || levitate || held("airballoon") || has_vol(a, "magnetrise").is_some())
}

fn ignores_item(b: &Battle, w: Who) -> bool {
    b.field.rooms[R_MAGICROOM].is_some() || b.mon(w).ability.map(|x| x.id()) == Some("klutz")
}

/// Switching is blocked by something shown: a trapping condition, Fairy Lock, or a shown trapping ability.
pub fn trapped(b: &Battle) -> bool {
    let Some(w) = b.active(Side::Own) else { return false };
    let a = &b.side(Side::Own).active;
    let mon = b.mon(w);
    if b.types(w).map_or(false, |t| t.contains(&Type::Ghost)) {
        return false;
    }
    if matches!(mon.item, ItemState::Held(i) if i.id() == "shedshell") && !ignores_item(b, w) {
        return false;
    }
    // Champions: Run Away escapes every trap
    if mon.ability.map(|x| x.id()) == Some("runaway") {
        return false;
    }
    if ["trapped", "noretreat", "ingrain"].iter().any(|id| has_vol(a, id).is_some()) {
        return true;
    }
    let foe = b.active(Side::Opp);
    if a.trapped_by.is_some() && a.trapped_by == foe {
        return true;
    }
    for id in ["partiallytrapped", "octolock"] {
        if let Some(v) = has_vol(a, id) {
            if v.by.is_none() || v.by == foe {
                return true;
            }
        }
    }
    if b.field.rooms[R_FAIRYLOCK].map_or(false, |f| b.turn <= f.start.saturating_add(1)) {
        return true;
    }
    if let Some(f) = foe {
        match b.mon(f).ability.map(|x| x.id()) {
            Some("shadowtag") => return mon.ability.map(|x| x.id()) != Some("shadowtag"),
            Some("arenatrap") => return grounded(b, w),
            Some("magnetpull") => return b.types(w).map_or(false, |t| t.contains(&Type::Steel)),
            _ => {}
        }
    }
    false
}

/// Moves of our active Pokémon and PP left per slot.
fn own_moves(b: &Battle, w: Who) -> [Option<MoveId>; 4] {
    let a = &b.side(Side::Own).active;
    if a.transformed.is_some() {
        a.t_moves
    } else {
        b.mon(w).moves
    }
}

/// Why a move slot cannot be chosen, if it cannot (`None`: usable).
pub fn move_blocked(b: &Battle, slot: usize) -> Option<&'static str> {
    let w = b.active(Side::Own)?;
    let a = &b.side(Side::Own).active;
    let mon = b.mon(w);
    let Some(mv) = own_moves(b, w)[slot] else { return Some("no move") };
    let d = mv.data();
    if b.pp_left(w)[slot] == Some(0) {
        return Some("no PP");
    }
    if b.refused[slot] {
        return Some("refused");
    }
    if let Some(v) = has_vol(a, "disable") {
        if v.detail == VolDetail::Move(mv) {
            return Some("Disable");
        }
    }
    if let Some(v) = has_vol(a, "encore") {
        if let VolDetail::Move(e) = v.detail {
            if e != mv && own_moves(b, w).contains(&Some(e)) {
                return Some("Encore");
            }
        }
    }
    if has_vol(a, "taunt").is_some() && d.category == Category::Status {
        return Some("Taunt");
    }
    if has_vol(a, "torment").is_some() && a.last_move == Some(mv) && mv != moves::STRUGGLE {
        return Some("Torment");
    }
    if has_vol(a, "healblock").is_some() && d.has_flag(flag::HEAL) {
        return Some("Heal Block");
    }
    if has_vol(a, "throatchop").is_some() && d.has_flag(flag::SOUND) {
        return Some("Throat Chop");
    }
    if b.field.rooms[R_GRAVITY].is_some() && d.has_flag(flag::GRAVITY) {
        return Some("Gravity");
    }
    if let Some(f) = b.active(Side::Opp) {
        let oa = &b.side(Side::Opp).active;
        let theirs = if oa.transformed.is_some() { oa.t_moves } else { b.mon(f).moves };
        // transformed into the imprisoner, every move we have is one of its
        let copied = a.transformed.is_some() && oa.transformed.is_none() && a.transformed == b.mon(f).form.or(b.mon(f).species);
        if has_vol(oa, "imprison").is_some() && (theirs.contains(&Some(mv)) || copied) {
            return Some("Imprison");
        }
    }
    if let Some(lock) = a.choice_lock {
        let choice = matches!(mon.item, ItemState::Held(i) if i.data().is_choice) && !ignores_item(b, w);
        let gorilla = mon.ability.map(|x| x.id()) == Some("gorillatactics");
        if (choice || gorilla) && lock != mv && own_moves(b, w).contains(&Some(lock)) {
            return Some("Choice lock");
        }
    }
    if d.has_flag(flag::CANTUSETWICE) && a.last_move == Some(mv) {
        return Some("used last turn");
    }
    // Champions: Fake Out and First Impression only on the first move action after entering
    if (mv.id() == "fakeout" || mv.id() == "firstimpression") && a.move_actions > 0 {
        return Some("not the first action");
    }
    None
}

/// The legal actions for a request, from the Battle only.
pub fn legal(b: &Battle, kind: ReqKind) -> Legal {
    let mut out = Legal::default();
    let own = b.side(Side::Own);
    let mut active_slot = own.active.slot;
    if kind == ReqKind::Replace {
        // a replacement asked before the rows of its turn (LV5Q-7): the Pokémon our switch this turn named
        // is the one on the field, though no row has shown it yet
        if let Some(x) = b.ctx.own_switch.filter(|&x| own.now.act == Act::None && active_slot != Some(x)) {
            active_slot = Some(x);
        }
    }
    let can_switch_to = |i: usize| -> bool {
        match b.picks[i] {
            Some(p) => Some(p) != active_slot && !own.team[p as usize].fainted && !b.refused[8 + i],
            None => false,
        }
    };
    if kind == ReqKind::Replace && b.ctx.unread {
        return stale_replace(b);
    }
    if kind == ReqKind::Replace {
        // after our Revival Blessing the choice is which fainted Pokémon comes back
        let reviving = own.active.slot.is_some() && !own.active.out && own.now.mv.map_or(false, |m| m.id() == "revivalblessing");
        for i in 0..3 {
            out.mask[8 + i] = if reviving {
                b.picks[i].map_or(false, |p| own.team[p as usize].fainted && !b.refused[8 + i])
            } else {
                can_switch_to(i)
            };
        }
        return out;
    }
    let Some(w) = b.active(Side::Own) else {
        // no Pokémon on the field: only a replacement can be chosen
        for i in 0..3 {
            out.mask[8 + i] = can_switch_to(i);
        }
        return out;
    };
    let a = &own.active;
    let moves = own_moves(b, w);
    let prev_turn = b.turn.saturating_sub(1);
    let locked = match a.lock {
        Lock::Recharge { turn } if turn == prev_turn => Some(Forced::Recharge),
        Lock::Charging { mv: Some(mv), turn, .. } if turn == prev_turn => Some(Forced::Move(mv)),
        // a charge row that named no move: our only charging move
        Lock::Charging { mv: None, turn, .. } if turn == prev_turn => {
            let mut c = moves.iter().flatten().filter(|m| m.data().has_flag(flag::CHARGE));
            match (c.next(), c.next()) {
                (Some(&m), None) => Some(Forced::Move(m)),
                _ => None,
            }
        }
        Lock::Locked { mv, .. } => Some(Forced::Move(mv)),
        _ => None,
    };
    if let Some(f) = locked {
        out.trapped = true;
        match f {
            Forced::Move(mv) if moves.contains(&Some(mv)) => {
                let i = moves.iter().position(|m| *m == Some(mv)).unwrap();
                out.mask[i] = true;
            }
            f => {
                out.mask[0] = true;
                out.forced = Some(f);
            }
        }
        return out;
    }
    for i in 0..4 {
        out.mask[i] = move_blocked(b, i).is_none();
    }
    if !out.mask[..4].iter().any(|x| *x) {
        out.mask[0] = true;
        out.forced = Some(Forced::Struggle);
    } else {
        let mon = b.mon(w);
        out.can_mega = !own.mega_used
            && !mon.mega
            && match (mon.item, mon.form) {
                (ItemState::Held(it), Some(f)) => items::mega_form(it, f).is_some(),
                _ => false,
            };
        if out.can_mega {
            for i in 0..4 {
                out.mask[4 + i] = out.mask[i];
            }
        }
    }
    out.trapped = trapped(b);
    if !out.trapped {
        for i in 0..3 {
            out.mask[8 + i] = can_switch_to(i);
        }
    }
    out
}

/// The team slot of the Pokémon a `replace` request asked before any row of its turn replaces: the one our
/// last replacement brought in, else the one our switch this turn named, else the active one.
pub fn on_field(b: &Battle) -> Option<u8> {
    let own = b.side(Side::Own);
    b.ctx.sent_in.or(b.ctx.own_switch.filter(|_| own.now.act == Act::None)).or(own.active.slot)
}

/// A `replace` request before the rows of its turn (LV5Q-7): every brought Pokémon not known fainted except
/// the one on the field. A Pokémon that fainted in the unread rows (a second replacement in the same turn)
/// is offered and the game refuses it. After our Revival Blessing the fainted ones are offered too: the
/// request may be the revival's choice or a replacement before we moved, and the game refuses the wrong one.
/// With no one else to offer, the one on the field is offered; with none of those either (all refused at
/// this screen), every one not known fainted. A refusal holds for its own replace screen (and a `retry` of
/// it), not for a new one.
fn stale_replace(b: &Battle) -> Legal {
    let mut out = Legal::default();
    let own = b.side(Side::Own);
    let field = on_field(b);
    let revival = b.ctx.own_choice == Some(moves::REVIVALBLESSING);
    let open = |i: usize| b.picks[i].map_or(false, |p| !b.refused[8 + i] && (revival || !own.team[p as usize].fainted));
    for i in 0..3 {
        out.mask[8 + i] = open(i) && b.picks[i] != field;
    }
    // the game asks, so someone can come in: with no one else, the one taken to be on the field was dragged
    // out in the unread rows (Red Card, Roar, Dragon Tail)
    if !out.mask[8..].iter().any(|x| *x) {
        for i in 0..3 {
            out.mask[8 + i] = open(i);
        }
    }
    // never nothing: every brought Pokémon not known fainted (any, after our Revival Blessing)
    if !out.mask[8..].iter().any(|x| *x) {
        for i in 0..3 {
            out.mask[8 + i] = b.picks[i].map_or(false, |p| revival || !own.team[p as usize].fainted);
        }
    }
    out
}

/// A `replace` request asked before the rows of its turn (no move is legal and no row has been read since
/// our turn's answer): the Pokémon the Battle has on the field may have fainted or be leaving.
pub fn replace_unread(b: &Battle, l: &Legal) -> bool {
    b.ctx.unread && !l.mask[..8].iter().any(|x| *x)
}

/// The Order an action stands for.
pub fn order_for(b: &Battle, l: &Legal, action: usize) -> Option<Order> {
    if action >= N_ACTIONS || !l.mask[action] {
        return None;
    }
    if action >= 8 {
        let p = b.picks[action - 8]?;
        return Some(Order::Switch { mon: b.side(Side::Own).team[p as usize].species?.id().to_string() });
    }
    if action == 0 {
        match l.forced {
            Some(Forced::Struggle) => return Some(Order::Move { mv: "struggle".into(), mega: false }),
            Some(Forced::Recharge) => return Some(Order::Move { mv: "recharge".into(), mega: false }),
            Some(Forced::Move(mv)) => return Some(Order::Move { mv: mv.id().into(), mega: false }),
            None => {}
        }
    }
    let w = b.active(Side::Own)?;
    let mv = own_moves(b, w)[action % 4]?;
    Some(Order::Move { mv: mv.id().into(), mega: action >= 4 })
}
