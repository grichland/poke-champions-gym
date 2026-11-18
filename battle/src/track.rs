//! Running counters kept as events arrive, for the observation (PROJECT.md "Observation features",
//! running counters). Read from the events only, the same in training and live.

use crate::event::Event;
use crate::state::*;
use cg_engine::sim::data::*;

/// A move that raises the user's own stat stages as its effect (status moves only), or Belly Drum.
pub fn is_setup(mv: MoveId) -> bool {
    let d = mv.data();
    if d.category != Category::Status {
        return false;
    }
    let up = |b: Option<[i8; 7]>| b.map_or(false, |b| b.iter().any(|&x| x > 0));
    let on_self = matches!(d.target, Target::Self_ | Target::AdjacentAllyOrSelf | Target::AllySide | Target::Allies);
    (on_self && up(d.boosts)) || up(d.self_boost) || d.self_effect.map_or(false, |e| up(e.boosts)) || d.id == "bellydrum"
}

/// Protect and its variants (stalling moves other than Endure).
pub fn is_protect(mv: MoveId) -> bool {
    let d = mv.data();
    d.stalling_move && d.id != "endure"
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SideTrack {
    /// completed turns counted, from turn 1
    pub turns: u16,
    pub switches: u16,
    pub protects: u16,
    pub setups: u16,
    pub attacks: u16,
    /// per team index: turns its move came before / after the opposing move
    pub moved: [[u8; 2]; 6],
    /// consecutive turns the active used a stalling move (Endure included) that did not fail
    pub stall_streak: u8,
    /// consecutive completed turns in which this side switched by choice (not a pivot move or a drag)
    pub switch_run: u8,
    /// turn this side made a Wish that did not fail; cleared when it heals
    pub wish_turn: Option<u16>,
    /// turn this side used Future Sight; cleared when it lands
    pub future_turn: Option<u16>,
    /// faints and item / ability rows about this side, in the turn under way and in the last one
    pub faints_now: u8,
    pub faints_last: u8,
    pub item_now: bool,
    pub item_last: bool,
    pub ability_now: bool,
    pub ability_last: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Track {
    /// index 0 own, 1 opponent
    pub sides: [SideTrack; 2],
}

fn moving(r: &TurnRecord) -> bool {
    matches!(r.act, Act::Move | Act::Pivot) && r.order > 0
}

/// Updates the counters after the builder applied `e`; `turn_before` is the Battle's turn before it.
pub fn after_event(b: &mut Battle, e: &Event, turn_before: u16) {
    match e {
        Event::Turn { .. } => {
            if b.turn != turn_before {
                rollover(b, turn_before);
            }
        }
        Event::Enter { who, .. } => b.track.sides[who.side.idx()].stall_streak = 0,
        Event::Faint { who } => {
            let t = &mut b.track.sides[who.side.idx()];
            t.faints_now = t.faints_now.saturating_add(1);
        }
        Event::Item { who, .. } => {
            let side = who.as_ref().map(|m| m.side).or(b.ctx.last_move.map(|(u, ..)| u.side));
            if let Some(s) = side {
                b.track.sides[s.idx()].item_now = true;
            }
        }
        Event::MegaStone { who, .. } => b.track.sides[who.side.idx()].item_now = true,
        Event::Cure { who, item: Some(_), .. } => b.track.sides[who.side.idx()].item_now = true,
        Event::Ability { who, .. } | Event::AbilityChange { who, .. } => b.track.sides[who.side.idx()].ability_now = true,
        Event::Blocked { effect, who: Some(m), .. } => {
            if AbilityId::from_id(effect).map_or(false, |a| a.0 != 0) {
                b.track.sides[m.side.idx()].ability_now = true;
            }
        }
        Event::Cause { effect, who, of } => {
            if let Some(m) = of.as_ref().or(who.as_ref()) {
                let t = &mut b.track.sides[m.side.idx()];
                if ItemId::from_id(effect).map_or(false, |i| i.0 != 0) {
                    t.item_now = true;
                } else if AbilityId::from_id(effect).map_or(false, |a| a.0 != 0) {
                    t.ability_now = true;
                }
            }
            if let Some(m) = who {
                match effect.as_str() {
                    "wish" => b.track.sides[m.side.idx()].wish_turn = None,
                    "futuresight" => b.track.sides[m.side.other().idx()].future_turn = None,
                    _ => {}
                }
            }
        }
        Event::Once { effect, who: Some(m) } | Event::VolatileHit { effect, who: Some(m) } if effect == "futuresight" => {
            b.track.sides[m.side.other().idx()].future_turn = None;
        }
        _ => {}
    }
}

/// A turn ended: `done` is its number and `sides[s].last` its record.
fn rollover(b: &mut Battle, done: u16) {
    for s in 0..2 {
        let r = b.sides[s].last;
        let o = b.sides[1 - s].last;
        let active = b.sides[s].active.slot;
        let t = &mut b.track.sides[s];
        if done >= 1 {
            t.turns = t.turns.saturating_add(1);
            match (r.act, r.mv) {
                (Act::Switch, _) => t.switches = t.switches.saturating_add(1),
                (Act::Move | Act::Pivot, Some(mv)) => {
                    if is_protect(mv) {
                        t.protects = t.protects.saturating_add(1);
                    }
                    if is_setup(mv) {
                        t.setups = t.setups.saturating_add(1);
                    }
                    if mv.data().category != Category::Status {
                        t.attacks = t.attacks.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
        if let (true, true, Some(m)) = (moving(&r), moving(&o), r.mon) {
            let k = if r.order < o.order { 0 } else { 1 };
            let c = &mut t.moved[m as usize % 6][k];
            *c = c.saturating_add(1);
        }
        let stalled = moving(&r) && r.mon == active && r.mv.map_or(false, |m| m.data().stalling_move) && !r.failed;
        t.stall_streak = if stalled { t.stall_streak.saturating_add(1) } else { 0 };
        t.switch_run = if done >= 1 && r.act == Act::Switch { t.switch_run.saturating_add(1) } else { 0 };
        if moving(&r) && !r.failed {
            match r.mv.map(|m| m.id()) {
                Some("wish") => t.wish_turn = Some(done),
                Some("futuresight") => t.future_turn = Some(done),
                _ => {}
            }
        }
        t.faints_last = t.faints_now;
        t.faints_now = 0;
        t.item_last = t.item_now;
        t.item_now = false;
        t.ability_last = t.ability_now;
        t.ability_now = false;
    }
}
