//! Item facts the builder needs beyond the generated tables.

use cg_engine::sim::data::*;

/// Legal items, other than Berries, that are used up when they act: those whose Showdown handlers call
/// `useItem` or that are Gems (data/items.ts at the pinned commit).
pub const SPENT_NON_BERRIES: [&str; 10] =
    ["ejectbutton", "electricseed", "focussash", "grassyseed", "mentalherb", "mistyseed", "normalgem", "psychicseed", "redcard", "whiteherb"];

/// LIVE.md 5.7: whether an item is gone after a `Held Item` row (`item` with `what: activated`).
pub fn spent_when_activated(item: ItemId) -> bool {
    let d = item.data();
    d.is_berry || SPENT_NON_BERRIES.contains(&d.id)
}

/// The item that Mega Evolves `base` into `mega`.
pub fn mega_stone_for(base: SpeciesId, mega: SpeciesId) -> Option<ItemId> {
    ITEMS.iter().position(|it| it.mega_stone.iter().any(|&(b, m)| b == base && m == mega)).map(|i| ItemId(i as u16))
}

/// The Mega form a held stone gives `species`, if any.
pub fn mega_form(item: ItemId, species: SpeciesId) -> Option<SpeciesId> {
    item.data().mega_stone.iter().find(|&&(b, _)| b == species).map(|&(_, m)| m)
}
