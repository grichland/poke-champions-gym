//! The battle state: one plain value (`Copy`), fixed-size arrays and ids only, so a battle forks for the
//! cost of a memory copy. Holds everything the rig snapshot needs and what the resolver needs mid-turn.

use super::data::*;

pub const SIDES: usize = 2;
pub const TEAM: usize = 3;
/// volatile conditions the active Pokémon of a side can hold at once
pub const MAX_VOLATILES: usize = 16;
pub const MAX_SIDE_CONDS: usize = 12;
pub const MAX_SLOT_CONDS: usize = 4;
pub const MAX_PSEUDO: usize = 5;
pub const MAX_QUEUE: usize = 12;
pub const MAX_FAINTS: usize = 6;
pub const MAX_UNIMPL: usize = 4;
pub const MAX_ATTACKS: usize = 4;
/// a Pokémon reference (side * 4 + slot) that is none
pub const NO_MON: u8 = 0xFF;
pub const LEVEL: u8 = 50;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveSlot {
    pub id: MoveId,
    pub pp: u8,
    pub max_pp: u8,
    /// set by DisableMove effects at the start of a decision (Showdown's moveSlot.disabled)
    pub disabled: bool,
    /// disabled as 'hidden' (Imprison): the request still offers the move
    pub disabled_hidden: bool,
    pub used: bool,
}

/// A volatile condition on a side's active Pokémon. Fields beyond `id` and `duration` are the condition's
/// state as Showdown keeps it; each condition uses the ones it needs (see `conds.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Volatile {
    pub id: Cond,
    /// Showdown's effectState.duration; 0 = none
    pub duration: i8,
    /// stall and Ally Switch counter
    pub counter: i16,
    /// Substitute HP; Counter and Mirror Coat damage
    pub hp: i16,
    /// the condition's move (Encore, Disable, locked and two-turn moves, partial trap source move)
    pub mv: MoveId,
    /// side * 4 + slot of the Pokémon that caused it, or NO_MON
    pub source: u8,
    /// lockedmove trueDuration, confusion time, Stockpile layers, bound divisor, Counter slot set
    pub data: i8,
    /// Stockpile Def gained; Focus Punch lost focus; Dragon Cheer Dragon type
    pub data2: i8,
    /// Stockpile SpD gained
    pub data3: i8,
    /// linked to the condition's counterpart on `source` (Showdown's linkedPokemon: trapped and trapper)
    pub linked: bool,
}

/// One entry of Showdown's pokemon.attackedBy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attack {
    /// side * 4 + slot of the attacker
    pub source: u8,
    pub damage: u16,
    /// damageValue was a number (a hit that dealt damage, 0 included)
    pub numeric: bool,
    pub this_turn: bool,
}

/// pokemon.moveThisTurnResult: undefined, null, false or true.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveResult {
    #[default]
    Undefined,
    Null,
    False,
    True,
}

/// A side condition or a slot condition (Showdown's EffectState on side.sideConditions or slotConditions).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SideCond {
    pub id: Cond,
    pub layers: u8,
    pub duration: i8,
    /// order of creation (Showdown's effectOrder), for hazards that tie
    pub order: u16,
    /// Wish: the HP it heals; Future Sight: the move
    pub data: i16,
    /// side * 4 + slot of the Pokémon that set it, or NO_MON
    pub source: u8,
    /// Wish: startingTurn; Future Sight: endingTurn
    pub turn: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldCond {
    pub id: Cond,
    /// 0 = none or permanent
    pub duration: i8,
    /// a pseudo-weather's state has had its target set by one of its event handlers (Showdown sets
    /// effectState.target when a handler runs; until then the handler sorts with a condition's sub-order)
    pub targeted: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mon {
    /// the form now
    pub species: SpeciesId,
    /// Showdown's baseSpecies: the form it returns to on switching (a Mega stays Mega)
    pub base_species: SpeciesId,
    /// as brought
    pub set_species: SpeciesId,
    /// the first `ntypes` entries are the types; Typeless is Showdown's "???"
    pub types: [Type; 2],
    pub ntypes: u8,
    /// Forest's Curse and Trick-or-Treat; Typeless when none
    pub added_type: Type,
    pub ability: AbilityId,
    pub base_ability: AbilityId,
    pub item: ItemId,
    pub hp: u16,
    pub maxhp: u16,
    pub base_maxhp: u16,
    /// atk, def, spa, spd, spe before stages (storedStats)
    pub stats: [u16; 5],
    /// stat points by stat (hp first) and nature, to recompute stats on a form change
    pub evs: [u8; 6],
    pub nature: u8,
    pub level: u8,
    pub status: Status,
    /// sleep or freeze counter (Showdown's statusState.time)
    pub status_time: i8,
    pub tox_stage: u8,
    pub moves: [MoveSlot; 4],
    /// the own move slots while `moves` holds borrowed ones (Transform, Mimic); PP lives in `moves`
    pub base_moves: [MoveSlot; 4],
    pub transformed: bool,
    pub nmoves: u8,
    /// atk, def, spa, spd, spe, accuracy, evasion
    pub boosts: [i8; 7],
    pub fainted: bool,
    pub faint_queued: bool,
    pub is_active: bool,
    pub is_started: bool,
    pub last_move: MoveId,
    pub last_move_used: MoveId,
    pub move_this_turn: MoveId,
    pub active_turns: u16,
    pub active_move_actions: u16,
    /// Showdown's pokemon.speed: the action speed as last updated
    pub speed: i32,
    pub weight_hg: u32,
    /// the form this Pokémon can Mega Evolve into now (Showdown's canMegaEvo)
    pub can_mega: Option<SpeciesId>,
    pub switch_flag: bool,
    /// the move that set switch_flag (pivot moves), NONE for a faint replacement
    pub switch_flag_move: MoveId,
    pub force_switch_flag: bool,
    pub move_this_turn_result: MoveResult,
    pub move_last_turn_result: MoveResult,
    pub stats_raised_this_turn: bool,
    pub stats_lowered_this_turn: bool,
    /// Showdown's skipPursuit: the switch-out step already ran (pivot, Baton Pass, Shed Tail)
    pub skip_pursuit: bool,
    pub attacked: [Attack; MAX_ATTACKS],
    pub nattacked: u8,
    pub last_damage: u16,
    pub hurt_this_turn: Option<u16>,
    pub times_attacked: u8,
    pub newly_switched: bool,
    pub trapped: bool,
    /// the trap is an ability's (Shadow Tag, Arena Trap, Magnet Pull): the request still offers switches
    pub trap_hidden: bool,
    pub gender: u8,
    /// abilityState.effectOrder
    pub ab_order: u16,
    /// abilityState flags (abilities.rs `abf`)
    pub ab_flags: u8,
    /// abilityState data of the ability (Supreme Overlord's fallen, Gorilla Tactics' move)
    pub ab_data: i16,
    /// once-per-battle ability flags kept on the Pokémon (abilities.rs `abb`)
    pub ab_battle: u8,
    /// Illusion's disguise is up (pokemon.illusion)
    pub illusion: bool,
    /// pokemon.beingCalledBack: switching out
    pub being_called_back: bool,
    /// pokemon.lastItem: the item last used or eaten (Recycle, Harvest, Pickup)
    pub last_item: ItemId,
    pub ate_berry: bool,
    pub used_item_this_turn: bool,
    /// itemState.effectOrder
    pub item_order: u16,
    /// lastMoveTargetLoc: 1 the foe's position, -1 its own, 0 none
    pub last_move_tloc: i8,
    /// a second abilityState counter (Cud Chew's)
    pub ab_count: i8,
}

impl Mon {
    pub fn boost(&self, i: usize) -> i8 {
        self.boosts[i]
    }
    /// the base types (no added type, no Roost)
    pub fn has_type(&self, t: Type) -> bool {
        self.types[..self.ntypes as usize].contains(&t)
    }
    pub fn set_types(&mut self, t: &[Type]) {
        self.ntypes = t.len().min(2) as u8;
        self.types = [Type::Typeless; 2];
        self.types[..t.len().min(2)].copy_from_slice(&t[..t.len().min(2)]);
        self.added_type = Type::Typeless;
    }
    pub fn attacks(&self) -> &[Attack] {
        &self.attacked[..self.nattacked as usize]
    }
    pub fn moves(&self) -> &[MoveSlot] {
        &self.moves[..self.nmoves as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Request {
    /// no decision (Showdown's wait)
    #[default]
    Wait,
    TeamPreview,
    /// a normal turn
    Move,
    /// must switch (faint replacement, pivot)
    Switch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Side {
    pub mons: [Mon; TEAM],
    /// slot of the active Pokémon
    pub active: u8,
    pub has_active: bool,
    pub pokemon_left: u8,
    pub total_fainted: u8,
    pub fainted_this_turn: bool,
    pub fainted_last_turn: bool,
    pub volatiles: [Volatile; MAX_VOLATILES],
    pub nvolatiles: u8,
    pub conds: [SideCond; MAX_SIDE_CONDS],
    pub nconds: u8,
    /// slot conditions of the active position (Wish, Future Sight, Healing Wish)
    pub slot_conds: [SideCond; MAX_SLOT_CONDS],
    pub nslot_conds: u8,
    pub request: Request,
    /// a Pokémon of this side has Mega Evolved (Showdown sets canMegaEvo false on the whole side)
    pub mega_spent: bool,
    /// what refused choices revealed for this request: the hidden trap (bit 0), a hidden disabled move
    /// (bit 1 + its position)
    pub revealed: u8,
    /// slots by position in Showdown's side.pokemon (the active one first; a switch swaps two)
    pub order: [u8; TEAM],
    /// Opportunist's abilityState.boosts on the active Pokémon (None when it has no table)
    pub opp_boosts: Option<super::events::Boosts>,
}

impl Side {
    #[inline]
    pub fn active_mon(&self) -> &Mon {
        &self.mons[self.active as usize]
    }
    #[inline]
    pub fn active_mon_mut(&mut self) -> &mut Mon {
        &mut self.mons[self.active as usize]
    }
    pub fn volatiles(&self) -> &[Volatile] {
        &self.volatiles[..self.nvolatiles as usize]
    }
    #[inline]
    pub fn vol_index(&self, c: Cond) -> Option<usize> {
        self.volatiles[..self.nvolatiles as usize].iter().position(|v| v.id == c)
    }
    #[inline]
    pub fn has_vol(&self, c: Cond) -> bool {
        self.vol_index(c).is_some()
    }
    pub fn vol(&self, c: Cond) -> Option<&Volatile> {
        self.vol_index(c).map(|i| &self.volatiles[i])
    }
    pub fn vol_mut(&mut self, c: Cond) -> Option<&mut Volatile> {
        self.vol_index(c).map(move |i| &mut self.volatiles[i])
    }
    /// deletes a volatile without its End handler (Showdown's `delete pokemon.volatiles[id]`)
    pub fn delete_vol(&mut self, c: Cond) -> bool {
        let Some(i) = self.vol_index(c) else { return false };
        let n = self.nvolatiles as usize;
        self.volatiles.copy_within(i + 1..n, i);
        self.volatiles[n - 1] = Volatile::default();
        self.nvolatiles -= 1;
        true
    }
    pub fn conds(&self) -> &[SideCond] {
        &self.conds[..self.nconds as usize]
    }
    pub fn slot_conds(&self) -> &[SideCond] {
        &self.slot_conds[..self.nslot_conds as usize]
    }
    pub fn mega_used(&self) -> bool {
        self.mons.iter().any(|m| m.species.data().is_mega) || self.mega_spent
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub weather: FieldCond,
    pub terrain: FieldCond,
    pub pseudo: [FieldCond; MAX_PSEUDO],
    pub npseudo: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ActionKind {
    #[default]
    Start,
    InstaSwitch,
    BeforeTurn,
    RunSwitch,
    Switch,
    MegaEvo,
    Move,
    Residual,
    /// Counter, Mirror Coat: beforeTurnCallback (order 5)
    BeforeTurnMove,
    /// Focus Punch, Beak Blast, Chilly Reception: priorityChargeCallback (order 107)
    PriorityChargeMove,
    /// the choice after Revival Blessing (order 6): `target` is the fainted slot revived
    RevivalBlessing,
}

/// A queued action (Showdown's Action), sorted by order, priority, speed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Action {
    pub kind: ActionKind,
    pub order: u16,
    /// move priority in tenths (fractional priority is 0.1)
    pub priority: i16,
    pub speed: i32,
    pub sub_order: i16,
    /// the acting Pokémon: side and slot; side 0xFF = none
    pub side: u8,
    pub slot: u8,
    /// switch target slot
    pub target: u8,
    /// position in the move list, or 0xFF for Struggle
    pub move_index: u8,
    pub mv: MoveId,
    pub mega: bool,
    /// the action's sourceEffect: the pivot move of an instaswitch, Round when prioritized
    pub source_effect: MoveId,
    /// fractionalPriority in tenths (Quick Draw, Mycelium Might, Stall)
    pub frac: i8,
    /// the action move's pranksterBoosted
    pub prankster: bool,
    /// targetLoc: 1 the foe's position, -1 the user's own
    pub tloc: i8,
    /// originalTarget: the Pokémon at `tloc` when the action was resolved (side * 4 + slot), or NO_MON
    pub orig_target: u8,
    /// Champions resolveAction: Curse by a user that was not a Ghost then targets the user
    pub curse_self: bool,
}

impl Action {
    pub fn has_pokemon(&self) -> bool {
        self.side != 0xFF
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Queue {
    pub list: [Action; MAX_QUEUE],
    pub len: u8,
}

impl Queue {
    pub fn as_slice(&self) -> &[Action] {
        &self.list[..self.len as usize]
    }
    pub fn as_mut_slice(&mut self) -> &mut [Action] {
        &mut self.list[..self.len as usize]
    }
    pub fn push(&mut self, a: Action) {
        assert!((self.len as usize) < MAX_QUEUE, "action queue full");
        self.list[self.len as usize] = a;
        self.len += 1;
    }
    pub fn insert(&mut self, i: usize, a: Action) {
        assert!((self.len as usize) < MAX_QUEUE, "action queue full");
        let n = self.len as usize;
        self.list.copy_within(i..n, i + 1);
        self.list[i] = a;
        self.len += 1;
    }
    pub fn shift(&mut self) -> Option<Action> {
        if self.len == 0 {
            return None;
        }
        let a = self.list[0];
        let n = self.len as usize;
        self.list.copy_within(1..n, 0);
        self.len -= 1;
        Some(a)
    }
    pub fn peek(&self) -> Option<&Action> {
        self.as_slice().first()
    }
    pub fn remove(&mut self, i: usize) {
        let n = self.len as usize;
        self.list.copy_within(i + 1..n, i);
        self.len -= 1;
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

/// What the battle met that stage 1 does not play: an effect with a handler at a hook point, or move
/// data the resolver does not apply yet. A battle with any of these is not exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unimplemented {
    pub count: u32,
    /// the first distinct entries: (effect kind 0 move, 1 ability, 2 item, 3 condition; id; hook)
    pub first: [(u8, u16, u8); MAX_UNIMPL],
    pub nfirst: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Winner {
    #[default]
    None,
    Side(u8),
    Tie,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaintEntry {
    pub side: u8,
    pub slot: u8,
    /// the faint's source Pokémon (side * 4 + slot) or NO_MON, and whether its effect was a move
    pub source: u8,
    pub by_move: bool,
    /// the move was a future attack (Destiny Bond does not take its user)
    pub future: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub sides: [Side; SIDES],
    pub field: Field,
    pub turn: u16,
    pub ended: bool,
    pub winner: Winner,
    pub mid_turn: bool,
    /// the battle-wide request state (Showdown's requestState)
    pub request: Request,
    pub queue: Queue,
    pub faint_queue: [FaintEntry; MAX_FAINTS],
    pub nfaints: u8,
    /// creation counter for conditions (effectOrder)
    pub effect_order: u16,
    pub unimpl: Unimplemented,
    /// no Pokémon of the battle has had an ability or item with handlers: lets the hook points skip
    /// their scan while no condition is in play either (false, the safe value, by default)
    pub no_holder_fx: bool,
    /// the events the abilities that can be in this battle have handlers for (each ability a Pokémon is
    /// given is added), so an event no ability can handle skips the ability checks
    pub ab_union: u128,
    /// the same for the items that can be in this battle
    pub it_union: u128,
    /// the move being executed (Showdown's activeMove), for hooks
    pub active_move: MoveId,
    /// battle.activePokemon (side * 4 + slot) or NO_MON
    pub active_pokemon: u8,
    /// battle.activeTarget (side * 4 + slot) or NO_MON
    pub active_target: u8,
    /// battle.lastMove: the last move that finished as an action (Copycat)
    pub last_move: MoveId,
    /// counter of ActiveMove uses
    pub active_uid: u32,
}

impl State {
    pub fn mon(&self, side: usize, slot: usize) -> &Mon {
        &self.sides[side].mons[slot]
    }
    pub fn mon_mut(&mut self, side: usize, slot: usize) -> &mut Mon {
        &mut self.sides[side].mons[slot]
    }
    pub fn active(&self, side: usize) -> &Mon {
        self.sides[side].active_mon()
    }
    pub fn active_mut(&mut self, side: usize) -> &mut Mon {
        self.sides[side].active_mon_mut()
    }
}
