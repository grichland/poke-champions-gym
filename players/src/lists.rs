//! Move classes tower reads, generated once from CG's Champions move table (not from gen9 lists).

use cg_engine::sim::data::*;
use std::sync::OnceLock;

/// Class bits per move.
pub mod class {
    /// raises the user's stats with a status move (Belly Drum included)
    pub const SETUP: u16 = 1 << 0;
    /// heals the user with a status move (Recover, Roost, Wish, Rest, ...)
    pub const RECOVERY: u16 = 1 << 1;
    /// surely lowers the target's speed or paralyses it
    pub const SPEED_DOWN: u16 = 1 << 2;
    /// status move that inflicts a major status, confusion, attraction or drowsiness on the target
    pub const INFLICT: u16 = 1 << 3;
    /// sets Reflect, Light Screen or Aurora Veil
    pub const SCREEN: u16 = 1 << 4;
    /// works only on the first move action after entering
    pub const FIRST_TURN: u16 = 1 << 5;
    /// sets an entry hazard on the foe's side
    pub const HAZARD: u16 = 1 << 6;
    /// tower's `_high` keeps these even when the foe has a major status
    pub const ALWAYS_HIGH: u16 = 1 << 7;
}

/// Tower's always-high inflicters (Python list, damaging Infestation dropped).
const ALWAYS_HIGH_IDS: [&str; 3] = ["confuseray", "swagger", "attract"];
/// Showdown's first-turn moves (onTry on activeMoveActions); no flag marks them.
pub const FIRST_TURN_IDS: [&str; 3] = ["fakeout", "firstimpression", "matblock"];
/// Hazards with their layer caps.
pub const HAZARD_CAPS: [(&str, u8); 4] = [("stealthrock", 1), ("spikes", 3), ("toxicspikes", 2), ("stickyweb", 1)];

pub struct Lists {
    pub bits: Vec<u16>,
}

fn targets_foe(t: Target) -> bool {
    matches!(t, Target::Normal | Target::Any | Target::AdjacentFoe | Target::AllAdjacentFoes | Target::AllAdjacent | Target::RandomNormal)
}

fn classify(d: &MoveData) -> u16 {
    use class::*;
    let mut c = 0;
    let status = d.category == Category::Status;
    let to_self = matches!(d.target, Target::Self_ | Target::AdjacentAllyOrSelf | Target::AllySide | Target::Allies);
    if status && to_self {
        if d.id == "bellydrum" || d.boosts.map_or(false, |b| b.iter().map(|&x| x as i32).sum::<i32>() > 0) {
            c |= SETUP;
        }
        // Revival Blessing heals a fainted ally, not the user
        if d.has_flag(flag::HEAL) && d.selfdestruct == SelfDestruct::No && d.id != "revivalblessing" {
            c |= RECOVERY;
        }
    }
    let foe = targets_foe(d.target);
    if foe {
        let lowers = |b: &Option<[i8; 7]>| b.map_or(false, |b| b[4] < 0);
        let sure = |e: &HitEffect| e.chance.map_or(true, |x| x >= 100);
        if status && (lowers(&d.boosts) || d.status == Status::Par) {
            c |= SPEED_DOWN;
        }
        if !status && d.secondaries.iter().any(|e| sure(e) && (lowers(&e.boosts) || e.status == Status::Par)) {
            c |= SPEED_DOWN;
        }
        if status
            && (d.status != Status::None || [conds::CONFUSION, conds::ATTRACT, conds::YAWN].contains(&d.volatile_status))
        {
            c |= INFLICT;
        }
    }
    if [conds::REFLECT, conds::LIGHTSCREEN, conds::AURORAVEIL].contains(&d.side_condition) {
        c |= SCREEN;
    }
    if FIRST_TURN_IDS.contains(&d.id) {
        c |= FIRST_TURN;
    }
    if HAZARD_CAPS.iter().any(|(h, _)| *h == d.id) {
        c |= HAZARD;
    }
    if ALWAYS_HIGH_IDS.contains(&d.id) {
        c |= ALWAYS_HIGH;
    }
    c
}

pub fn lists() -> &'static Lists {
    static L: OnceLock<Lists> = OnceLock::new();
    L.get_or_init(|| Lists { bits: MOVES.iter().map(classify).collect() })
}

#[inline]
pub fn is(mv: MoveId, c: u16) -> bool {
    lists().bits.get(mv.0 as usize).map_or(false, |b| b & c != 0)
}

/// Every legal move in a class, by id.
pub fn members(c: u16) -> Vec<&'static str> {
    MOVES.iter().enumerate().filter(|(i, _)| *i > 0 && lists().bits[*i] & c != 0).map(|(_, m)| m.id).collect()
}

/// Stats a setup move raises (boost index), for the "all already +6" test.
pub fn raised(mv: MoveId) -> [bool; 7] {
    let d = mv.data();
    if d.id == "bellydrum" {
        let mut r = [false; 7];
        r[0] = true;
        return r;
    }
    d.boosts.map_or([false; 7], |b| b.map(|x| x > 0))
}

/// The layer cap of a hazard move.
pub fn hazard_cap(mv: MoveId) -> Option<(Cond, u8)> {
    let d = mv.data();
    HAZARD_CAPS.iter().find(|(h, _)| *h == d.id).map(|&(_, cap)| (d.side_condition, cap))
}
