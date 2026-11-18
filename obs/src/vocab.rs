//! CG's own compact id numbering for the embedding inputs. Every vocabulary reserves 0.
//!
//! - species: 0 = pad / unknown, `SpeciesId(i)` -> i + 1 (the generated table has no "none" entry)
//! - moves: 0 = empty / none / unknown, `MoveId(i)` -> i (index 0 of the move table is already "none")
//! - items: 0 = unknown, 1 = known to hold nothing (none brought, or used up / removed), `ItemId(i)` -> i + 1
//! - abilities: 0 = unknown / pad, `AbilityId(i)` -> i (index 0 of the ability table is "none")

use battle::ItemState;
use cg_engine::sim::data::*;

pub const SPECIES_VOCAB: usize = SPECIES.len() + 1;
pub const MOVE_VOCAB: usize = MOVES.len();
pub const ITEM_VOCAB: usize = ITEMS.len() + 1;
pub const ABILITY_VOCAB: usize = ABILITIES.len();

pub const ITEM_UNKNOWN: i32 = 0;
pub const ITEM_NONE: i32 = 1;

#[inline]
pub fn species(s: Option<SpeciesId>) -> i32 {
    s.map_or(0, |s| s.0 as i32 + 1)
}

#[inline]
pub fn mv(m: Option<MoveId>) -> i32 {
    m.map_or(0, |m| m.0 as i32)
}

#[inline]
pub fn item(st: ItemState) -> i32 {
    match st {
        ItemState::Unknown => ITEM_UNKNOWN,
        ItemState::Gone(_) => ITEM_NONE,
        ItemState::Held(i) if i.0 == 0 => ITEM_NONE,
        ItemState::Held(i) => i.0 as i32 + 1,
    }
}

#[inline]
pub fn ability(a: Option<AbilityId>) -> i32 {
    a.map_or(0, |a| a.0 as i32)
}

/// The species an id stands for (None for 0).
pub fn species_of(id: i32) -> Option<SpeciesId> {
    (id >= 1 && (id as usize) < SPECIES_VOCAB).then(|| SpeciesId(id as u16 - 1))
}
