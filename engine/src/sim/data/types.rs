//! Types of the generated tables (`generated.rs`). Ids are dense indices into the tables; index 0 of the
//! move, item, ability and condition tables is "none".

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SpeciesId(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MoveId(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ItemId(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct AbilityId(pub u16);
/// A condition: status, volatile, side, slot or field condition, weather or terrain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Cond(pub u16);

impl MoveId {
    pub const NONE: MoveId = MoveId(0);
    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}
impl ItemId {
    pub const NONE: ItemId = ItemId(0);
    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}
impl AbilityId {
    pub const NONE: AbilityId = AbilityId(0);
}
impl Cond {
    pub const NONE: Cond = Cond(0);
    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum Type {
    Bug,
    Dark,
    Dragon,
    Electric,
    Fairy,
    Fighting,
    Fire,
    Flying,
    Ghost,
    Grass,
    Ground,
    Ice,
    Normal,
    Poison,
    Psychic,
    Rock,
    Steel,
    Water,
    Stellar,
    /// Showdown's "???": no type (Struggle, a Pokémon with no second type)
    #[default]
    Typeless,
}

pub const TYPE_IDS: [&str; 20] = [
    "bug", "dark", "dragon", "electric", "fairy", "fighting", "fire", "flying", "ghost", "grass", "ground", "ice",
    "normal", "poison", "psychic", "rock", "steel", "water", "stellar", "???",
];

impl Type {
    pub fn id(self) -> &'static str {
        TYPE_IDS[self as usize]
    }
    pub fn is_real(self) -> bool {
        (self as u8) < 18
    }
}

/// TYPE_CHART value for "does not affect".
pub const IMMUNE: i8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Physical,
    Special,
    Status,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Normal,
    Self_,
    AllAdjacent,
    AdjacentAlly,
    Allies,
    AllAdjacentFoes,
    AdjacentAllyOrSelf,
    All,
    Any,
    AllySide,
    Scripted,
    AllyTeam,
    RandomNormal,
    FoeSide,
    AdjacentFoe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Status {
    #[default]
    None,
    Brn,
    Par,
    Slp,
    Frz,
    Psn,
    Tox,
}

impl Status {
    pub fn id(self) -> &'static str {
        ["", "brn", "par", "slp", "frz", "psn", "tox"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Stat {
    Hp,
    Atk,
    Def,
    Spa,
    Spd,
    Spe,
}

pub const STAT_IDS: [&str; 6] = ["hp", "atk", "def", "spa", "spd", "spe"];
/// Boost order: atk, def, spa, spd, spe, accuracy, evasion.
pub const BOOST_IDS: [&str; 7] = ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion"];
pub const B_ACC: usize = 5;
pub const B_EVA: usize = 6;

impl Stat {
    pub fn id(self) -> &'static str {
        STAT_IDS[self as usize]
    }
    /// index into a boost table (Hp has none)
    pub fn boost_index(self) -> usize {
        self as usize - 1
    }
}

/// Never misses (Showdown's `accuracy: true`).
pub const ACC_ALWAYS: u8 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedDamage {
    None,
    Level,
    Value(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ohko {
    No,
    Yes,
    Type(Type),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfSwitch {
    No,
    Yes,
    CopyVolatile,
    ShedTail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfDestruct {
    No,
    Always,
    IfHit,
}

/// A secondary effect or a move's `self` effect.
#[derive(Clone, Copy, Debug)]
pub struct HitEffect {
    /// None: always happens
    pub chance: Option<u8>,
    pub boosts: Option<[i8; 7]>,
    pub status: Status,
    pub volatile_status: Cond,
    pub side_condition: Cond,
    pub pseudo_weather: Cond,
    /// a secondary's own `self` effect
    pub self_boosts: Option<[i8; 7]>,
    pub self_volatile: Cond,
    /// has an `onHit` function in Showdown
    pub has_handler: bool,
}

#[derive(Debug)]
pub struct MoveData {
    pub id: &'static str,
    pub name: &'static str,
    pub num: i16,
    pub ty: Type,
    pub category: Category,
    pub base_power: u16,
    /// 1..=100, or ACC_ALWAYS
    pub accuracy: u8,
    pub priority: i8,
    pub pp: u8,
    pub max_pp: u8,
    pub no_pp_boosts: bool,
    pub target: Target,
    pub flags: u64,
    pub crit_ratio: u8,
    pub will_crit: Option<bool>,
    /// (min, max) hits; (0, 0) when not multi-hit
    pub multihit: (u8, u8),
    pub multihit_parental_bond: bool,
    pub drain: (u8, u8),
    pub recoil: (u8, u8),
    pub heal: (u8, u8),
    pub damage: FixedDamage,
    pub ohko: Ohko,
    pub secondaries: &'static [HitEffect],
    pub self_effect: Option<HitEffect>,
    pub self_boost: Option<[i8; 7]>,
    pub boosts: Option<[i8; 7]>,
    pub status: Status,
    pub volatile_status: Cond,
    pub side_condition: Cond,
    pub slot_condition: Cond,
    pub weather: Cond,
    pub terrain: Cond,
    pub pseudo_weather: Cond,
    /// the move's own condition (Protect's volatile, Reflect's side condition, ...)
    pub condition: Cond,
    pub self_switch: SelfSwitch,
    pub force_switch: bool,
    pub selfdestruct: SelfDestruct,
    pub ignore_immunity: bool,
    pub ignore_ability: bool,
    pub ignore_accuracy: bool,
    pub ignore_evasion: bool,
    pub ignore_offensive: bool,
    pub ignore_defensive: bool,
    pub ignore_negative_offensive: bool,
    pub ignore_positive_defensive: bool,
    pub override_offensive_stat: Option<Stat>,
    pub override_defensive_stat: Option<Stat>,
    /// Foul Play: the target's attack
    pub offensive_pokemon_target: bool,
    pub defensive_pokemon_source: bool,
    pub breaks_protect: bool,
    pub steals_boosts: bool,
    pub thaws_target: bool,
    pub multiaccuracy: bool,
    pub sleep_usable: bool,
    pub tracks_target: bool,
    pub smart_target: bool,
    pub force_stab: bool,
    pub has_crash_damage: bool,
    pub stalling_move: bool,
    pub calls_move: bool,
    pub struggle_recoil: bool,
    pub mind_blown_recoil: bool,
    pub non_ghost_target: Option<Target>,
    /// in 4096ths; 0 = default
    pub crit_modifier: u16,
    /// Showdown handler names (functions or constants) on the move itself
    pub handlers: &'static [&'static str],
    /// the move or one of its secondary/self effects has a function in Showdown
    pub has_handlers: bool,
}

impl MoveData {
    pub fn has_flag(&self, f: u64) -> bool {
        self.flags & f != 0
    }
    pub fn never_misses(&self) -> bool {
        self.accuracy == ACC_ALWAYS
    }
}

#[derive(Debug)]
pub struct SpeciesData {
    pub id: &'static str,
    pub name: &'static str,
    pub base_species: SpeciesId,
    pub forme: &'static str,
    /// second type Typeless when single-typed
    pub types: [Type; 2],
    /// hp, atk, def, spa, spd, spe
    pub base_stats: [u16; 6],
    /// slots 0, 1, H, S
    pub abilities: [AbilityId; 4],
    pub weight_hg: u32,
    /// "M", "F", "N" or "" (random)
    pub gender: &'static str,
    pub is_mega: bool,
    pub mega_from: Option<SpeciesId>,
    pub mega_stone: Option<ItemId>,
    pub max_hp: u16,
}

#[derive(Debug)]
pub struct ItemData {
    pub id: &'static str,
    pub name: &'static str,
    pub num: i16,
    /// (species, Mega form) pairs this stone Mega Evolves
    pub mega_stone: &'static [(SpeciesId, SpeciesId)],
    pub is_berry: bool,
    pub is_choice: bool,
    pub ignore_klutz: bool,
    pub fling_power: u16,
    pub handlers: &'static [&'static str],
    /// ordering numbers of the handlers (onResidualOrder, ...)
    pub nums: &'static [(&'static str, f32)],
}

pub mod aflag {
    pub const BREAKABLE: u16 = 1 << 0;
    pub const CANTSUPPRESS: u16 = 1 << 1;
    pub const FAILROLEPLAY: u16 = 1 << 2;
    pub const FAILSKILLSWAP: u16 = 1 << 3;
    pub const NOENTRAIN: u16 = 1 << 4;
    pub const NORECEIVER: u16 = 1 << 5;
    pub const NOTRACE: u16 = 1 << 6;
    pub const NOTRANSFORM: u16 = 1 << 7;
}

#[derive(Debug)]
pub struct AbilityData {
    pub id: &'static str,
    pub name: &'static str,
    pub num: i16,
    pub flags: u16,
    pub suppress_weather: bool,
    pub handlers: &'static [&'static str],
    pub nums: &'static [(&'static str, f32)],
}

/// How the legal data uses a condition (bits of ConditionData::kind).
pub mod ckind {
    pub const STATUS: u8 = 1 << 0;
    pub const VOLATILE: u8 = 1 << 1;
    pub const SIDE: u8 = 1 << 2;
    pub const SLOT: u8 = 1 << 3;
    pub const FIELD: u8 = 1 << 4;
    pub const WEATHER: u8 = 1 << 5;
    pub const TERRAIN: u8 = 1 << 6;
}

#[derive(Debug)]
pub struct ConditionData {
    pub id: &'static str,
    pub kind: u8,
    pub effect_type: &'static str,
    pub duration: u8,
    /// not passed by Baton Pass (Showdown's noCopy)
    pub no_copy: bool,
    pub handlers: &'static [&'static str],
    pub nums: &'static [(&'static str, f32)],
}

#[derive(Debug)]
pub struct NatureData {
    pub id: &'static str,
    pub name: &'static str,
    pub plus: Option<Stat>,
    pub minus: Option<Stat>,
}
