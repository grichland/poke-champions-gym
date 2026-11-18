//! Game data for the new resolver: tables generated from Showdown's Champions mod
//! (`tools/data/gen_sim_data.js`), with lookups by id string both ways.

pub mod generated;
pub mod types;

pub use generated::{
    abilities, conds, flag, items, moves, species, ABILITIES, CONDITIONS, ITEMS, MOVES, NATURES, PLAIN_MOVES, STAGE2_MOVES, STAGE3_MOVES,
    SHOWDOWN_COMMIT, SPECIES, TYPE_CHART,
};
pub use types::*;

/// Showdown's id: lower case letters and digits.
pub fn to_id(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn find<T>(table: &'static [T], id: &str, key: fn(&T) -> &'static str) -> Option<usize> {
    table.binary_search_by(|x| key(x).cmp(id)).ok()
}

impl SpeciesId {
    pub fn from_id(id: &str) -> Option<SpeciesId> {
        find(&SPECIES, id, |s| s.id).map(|i| SpeciesId(i as u16))
    }
    pub fn data(self) -> &'static SpeciesData {
        &SPECIES[self.0 as usize]
    }
    pub fn id(self) -> &'static str {
        self.data().id
    }
}

impl MoveId {
    pub fn from_id(id: &str) -> Option<MoveId> {
        find(&MOVES, id, |m| m.id).map(|i| MoveId(i as u16))
    }
    #[inline]
    pub fn data(self) -> &'static MoveData {
        &MOVES[self.0 as usize]
    }
    pub fn id(self) -> &'static str {
        self.data().id
    }
}

impl ItemId {
    pub fn from_id(id: &str) -> Option<ItemId> {
        find(&ITEMS, id, |x| x.id).map(|i| ItemId(i as u16))
    }
    pub fn data(self) -> &'static ItemData {
        &ITEMS[self.0 as usize]
    }
    pub fn id(self) -> &'static str {
        self.data().id
    }
}

impl AbilityId {
    pub fn from_id(id: &str) -> Option<AbilityId> {
        find(&ABILITIES, id, |x| x.id).map(|i| AbilityId(i as u16))
    }
    pub fn data(self) -> &'static AbilityData {
        &ABILITIES[self.0 as usize]
    }
    pub fn id(self) -> &'static str {
        self.data().id
    }
}

impl Cond {
    pub fn from_id(id: &str) -> Option<Cond> {
        find(&CONDITIONS, id, |x| x.id).map(|i| Cond(i as u16))
    }
    pub fn data(self) -> &'static ConditionData {
        &CONDITIONS[self.0 as usize]
    }
    pub fn id(self) -> &'static str {
        self.data().id
    }
}

pub fn nature_from_id(id: &str) -> Option<u8> {
    find(&NATURES, id, |n| n.id).map(|i| i as u8)
}

pub fn type_from_id(id: &str) -> Option<Type> {
    TYPE_IDS.iter().position(|t| *t == id).map(|i| ALL_TYPES[i])
}

pub const ALL_TYPES: [Type; 20] = [
    Type::Bug,
    Type::Dark,
    Type::Dragon,
    Type::Electric,
    Type::Fairy,
    Type::Fighting,
    Type::Fire,
    Type::Flying,
    Type::Ghost,
    Type::Grass,
    Type::Ground,
    Type::Ice,
    Type::Normal,
    Type::Poison,
    Type::Psychic,
    Type::Rock,
    Type::Steel,
    Type::Water,
    Type::Stellar,
    Type::Typeless,
];

/// Showdown's getEffectiveness for one defending type: 1, 0 or -1 (immunity is separate).
#[inline]
pub fn effectiveness(atk: Type, def: Type) -> i32 {
    if !atk.is_real() || !def.is_real() {
        return 0;
    }
    match TYPE_CHART[atk as usize][def as usize] {
        IMMUNE => 0,
        v => v as i32,
    }
}

/// Showdown's getImmunity for one defending type: false when the type does not affect it.
#[inline]
pub fn type_affects(atk: Type, def: Type) -> bool {
    !(atk.is_real() && def.is_real() && TYPE_CHART[atk as usize][def as usize] == IMMUNE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_both_ways() {
        for (i, m) in MOVES.iter().enumerate().skip(1) {
            assert_eq!(MoveId::from_id(m.id), Some(MoveId(i as u16)));
        }
        for (i, s) in SPECIES.iter().enumerate() {
            assert_eq!(SpeciesId::from_id(s.id).map(|x| x.0 as usize), Some(i));
        }
        assert_eq!(MoveId::from_id("earthquake").unwrap().id(), "earthquake");
        assert_eq!(moves::STRUGGLE.id(), "struggle");
        assert_eq!(ItemId::from_id("leftovers").unwrap().data().name, "Leftovers");
        assert_eq!(AbilityId::from_id("noability").unwrap().data().name, "No Ability");
        assert!(Cond::from_id("confusion").is_some());
        assert!(MoveId::from_id("notamove").is_none());
    }

    #[test]
    fn type_chart() {
        assert_eq!(effectiveness(Type::Water, Type::Fire), 1);
        assert_eq!(effectiveness(Type::Fire, Type::Water), -1);
        assert!(!type_affects(Type::Normal, Type::Ghost));
        assert!(!type_affects(Type::Ground, Type::Flying));
        assert!(type_affects(Type::Typeless, Type::Ghost));
    }

    #[test]
    fn move_data() {
        let m = MoveId::from_id("bulletseed").unwrap().data();
        assert_eq!(m.multihit, (2, 5));
        let m = MoveId::from_id("aerialace").unwrap().data();
        assert!(m.never_misses());
        let m = MoveId::from_id("bravebird").unwrap().data();
        assert_eq!(m.recoil, (33, 100));
        assert_eq!(moves::STRUGGLE.data().max_pp, 1);
        let sp = SpeciesId::from_id("garchompmegaz").unwrap().data();
        assert!(sp.is_mega);
    }
}
