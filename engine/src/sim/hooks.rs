//! Hook points: the places where effects of moves, abilities, items and conditions attach, named after
//! Showdown's events. Each effect's Showdown handlers (from the generated tables) give a mask of the hooks
//! it acts at. At a hook point the resolver asks every effect in play that has a handler there to act;
//! an effect whose handler has no Rust implementation is recorded in `State::unimpl`, so a battle in
//! which it could matter is detectable instead of silently played wrong.

use super::data::*;
use super::outcome::EffectRef;
use super::state::*;
use std::sync::OnceLock;

macro_rules! hooks {
    ($($h:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Hook { $($h),* }
        pub const HOOK_NAMES: &[&str] = &[$(stringify!($h)),*];
        const ALL_HOOKS: &[Hook] = &[$(Hook::$h),*];
    };
}

hooks!(
    // turn and action order
    BeforeTurn, ModifyPriority, FractionalPriority, OverrideAction, ModifySpe,
    // the move: can it move, use, targets
    BeforeMove, MoveAborted, LockMove, DisableMove, DeductPP, ModifyTarget, ModifyType, ModifyMove, TryMove,
    UseMoveMessage, Try, PrepareHit, Invulnerability, TryHit, TryImmunity, NegateImmunity, Immunity, Type,
    ModifyAccuracy, Accuracy, ModifyBoost,
    // damage
    ModifyCritRatio, CriticalHit, BasePower, ModifyAtk, ModifyDef, ModifySpA, ModifySpD, WeatherModifyDamage,
    ModifySTAB, Effectiveness, ModifyDamage, TryPrimaryHit, Damage, TryHeal, Heal,
    // after the hit
    Hit, DamagingHit, AfterHit, ModifySecondaries, AfterMoveSecondary, AfterMoveSecondarySelf, AfterMove,
    MoveFail, EmergencyExit, DragOut, Update,
    // switching and fainting
    SwitchIn, SwitchOut, BeforeSwitchIn, BeforeSwitchOut, Start, End, TrapPokemon, MaybeTrapPokemon,
    BeforeFaint, Faint, AfterFaint, AfterMega, ModifySpecies, ModifyWeight,
    // end of turn
    Residual,
    // everything else Showdown names
    Other,
);

impl Hook {
    pub fn name(self) -> &'static str {
        HOOK_NAMES[self as usize]
    }
    #[inline]
    pub fn bit(self) -> u128 {
        1u128 << (self as u8)
    }

    /// The hook of a Showdown handler name (`onSourceModifyDamage` -> ModifyDamage). Callbacks that are
    /// not `on*` events map to the step they replace.
    pub fn of_handler(name: &str) -> Hook {
        match name {
            "basePowerCallback" => return Hook::BasePower,
            "damageCallback" => return Hook::Damage,
            "beforeMoveCallback" => return Hook::BeforeMove,
            "beforeTurnCallback" => return Hook::BeforeTurn,
            "priorityChargeCallback" => return Hook::BeforeTurn,
            "durationCallback" => return Hook::Start,
            _ => {}
        }
        let Some(mut rest) = name.strip_prefix("on") else { return Hook::Other };
        for p in ["Source", "Foe", "Ally", "Any", "Side", "Field"] {
            if let Some(r) = rest.strip_prefix(p) {
                if r.starts_with(|c: char| c.is_ascii_uppercase()) && Hook::by_name(rest).is_none() {
                    rest = r;
                }
            }
        }
        // a few Showdown events share a hook
        match rest {
            "TryPrimaryHit" => Hook::TryPrimaryHit,
            "AfterSubDamage" => Hook::Hit,
            "AfterEachBoost" | "AfterBoost" | "TryBoost" | "ChangeBoost" => Hook::ModifyBoost,
            "SetStatus" | "AfterSetStatus" | "TryAddVolatile" => Hook::Immunity,
            "Weather" | "WeatherChange" | "TerrainChange" | "SetWeather" => Hook::Residual,
            "Restart" | "Copy" | "Swap" => Hook::Start,
            "HitField" | "HitSide" => Hook::Hit,
            "EntryHazard" | "AfterEntryHazard" => Hook::SwitchIn,
            _ => Hook::by_name(rest).unwrap_or(Hook::Other),
        }
    }

    fn by_name(s: &str) -> Option<Hook> {
        HOOK_NAMES.iter().position(|h| *h == s).map(|i| ALL_HOOKS[i])
    }
}

/// Hook masks of every effect, built once from the generated handler lists. `*_unimpl` keep only the
/// handlers without a Rust implementation.
pub struct Masks {
    pub moves: Vec<u128>,
    pub abilities: Vec<u128>,
    pub items: Vec<u128>,
    pub conds: Vec<u128>,
    pub moves_unimpl: Vec<u128>,
    pub conds_unimpl: Vec<u128>,
    pub abilities_unimpl: Vec<u128>,
    pub items_unimpl: Vec<u128>,
}

/// The conditions whose handlers are implemented (conds.rs, status.rs, residual.rs). Conditions that only
/// act in doubles (Follow Me, Rage Powder, Helping Hand redirections) are listed: in singles they never act.
pub const IMPLEMENTED_CONDS: &[&str] = &[
    "brn", "par", "slp", "frz", "psn", "tox", "confusion", "flinch", "trapped", "trapper", "partiallytrapped",
    "lockedmove", "twoturnmove", "mustrecharge", "stall", "banefulbunker", "beakblast", "curse", "disable",
    "dragoncheer", "electrify", "encore", "kingsshield", "noretreat", "octolock", "powershift", "powertrick",
    "protect", "saltcure", "spikyshield", "syrupbomb", "allyswitch", "aquaring", "attract", "bounce", "charge",
    "counter", "destinybond", "dig", "dive", "endure", "fly", "focusenergy", "focuspunch", "followme",
    "glaiverush", "helpinghand", "ingrain", "leechseed", "lockon", "magnetrise", "minimize", "mirrorcoat",
    "perishsong", "phantomforce", "ragepowder", "roost", "smackdown", "stockpile", "substitute", "taunt",
    "throatchop", "torment", "uproar", "yawn", "healblock", "sparklingaria", "skyattack", "meteorbeam",
    // stage 3 (field.rs): weather, terrain, pseudo-weathers, side and slot conditions
    "sunnyday", "raindance", "sandstorm", "snowscape", "electricterrain", "grassyterrain", "mistyterrain",
    "psychicterrain", "trickroom", "gravity", "magicroom", "wonderroom", "fairylock", "reflect", "lightscreen",
    "auroraveil", "tailwind", "safeguard", "quickguard", "wideguard", "stealthrock", "spikes", "toxicspikes",
    "stickyweb", "wish", "healingwish", "futuremove", "revivalblessing", "chillyreception", "solarbeam",
    "solarblade", "electroshot",
    // stage 4: the conditions of abilities and of the moves that change abilities
    "flashfire", "unburden", "protosynthesis", "quarkdrive", "truant", "gastroacid", "imprison",
    // stage 5: the conditions of items
    "choicelock", "metronome", "gem", "fling",
];

fn mask_of(handlers: &[&str]) -> u128 {
    handlers.iter().fold(0, |m, h| m | Hook::of_handler(h).bit())
}

/// Abilities and items start during the switch-in event (Showdown's getCallback falls back to onStart).
fn holder_mask_of(handlers: &[&str]) -> u128 {
    let m = mask_of(handlers);
    if m & Hook::Start.bit() != 0 {
        m | Hook::SwitchIn.bit()
    } else {
        m
    }
}

pub fn masks() -> &'static Masks {
    static M: OnceLock<Masks> = OnceLock::new();
    M.get_or_init(|| {
        // a handler on a secondary or `self` effect acts at the Hit step
        let moves: Vec<u128> = MOVES
            .iter()
            .map(|m| {
                let sub = m.secondaries.iter().any(|x| x.has_handler) || m.self_effect.map_or(false, |x| x.has_handler);
                mask_of(m.handlers) | if sub { Hook::Hit.bit() } else { 0 }
            })
            .collect();
        let conds: Vec<u128> = CONDITIONS.iter().map(|c| mask_of(c.handlers)).collect();
        let impl_moves: Vec<bool> = MOVES.iter().map(|m| super::movefx::IMPLEMENTED_MOVES.contains(&m.id)).collect();
        let impl_conds: Vec<bool> = CONDITIONS.iter().map(|c| IMPLEMENTED_CONDS.contains(&c.id)).collect();
        let abilities: Vec<u128> = ABILITIES.iter().map(|a| holder_mask_of(a.handlers)).collect();
        Masks {
            abilities_unimpl: abilities
                .iter()
                .zip(ABILITIES.iter())
                .map(|(m, a)| if a.id.is_empty() || super::abilities::IMPLEMENTED_ABILITIES.contains(&a.id) { 0 } else { *m })
                .collect(),
            items_unimpl: ITEMS
                .iter()
                .map(|i| if i.id.is_empty() || super::items::IMPLEMENTED_ITEMS.contains(&i.id) { 0 } else { holder_mask_of(i.handlers) })
                .collect(),
            moves_unimpl: moves.iter().zip(&impl_moves).map(|(m, i)| if *i { 0 } else { *m }).collect(),
            conds_unimpl: conds.iter().zip(&impl_conds).map(|(m, i)| if *i { 0 } else { *m }).collect(),
            moves,
            abilities,
            items: ITEMS.iter().map(|i| holder_mask_of(i.handlers)).collect(),
            conds,
        }
    })
}

/// Whether the handler of an effect at a hook has a Rust implementation.
pub fn implemented(effect: EffectRef, hook: Hook) -> bool {
    let m = masks();
    match effect {
        EffectRef::Move(mv) => m.moves_unimpl[mv.0 as usize] & hook.bit() == 0 && m.moves[mv.0 as usize] & hook.bit() != 0,
        EffectRef::Cond(c) => m.conds_unimpl[c.0 as usize] & hook.bit() == 0 && m.conds[c.0 as usize] & hook.bit() != 0,
        EffectRef::Ability(a) => super::abilities::ability_implemented(a),
        EffectRef::Item(i) => m.items_unimpl[i.0 as usize] == 0,
    }
}

pub fn effect_kind(e: EffectRef) -> (u8, u16) {
    match e {
        EffectRef::Move(m) => (0, m.0),
        EffectRef::Ability(a) => (1, a.0),
        EffectRef::Item(i) => (2, i.0),
        EffectRef::Cond(c) => (3, c.0),
    }
}

pub fn effect_of(kind: u8, id: u16) -> EffectRef {
    match kind {
        0 => EffectRef::Move(MoveId(id)),
        1 => EffectRef::Ability(AbilityId(id)),
        2 => EffectRef::Item(ItemId(id)),
        _ => EffectRef::Cond(Cond(id)),
    }
}

pub fn hook_of(i: u8) -> Hook {
    ALL_HOOKS[i as usize]
}

impl Unimplemented {
    pub fn note(&mut self, e: EffectRef, hook: Hook) {
        self.count = self.count.saturating_add(1);
        let (k, id) = effect_kind(e);
        let entry = (k, id, hook as u8);
        if self.first[..self.nfirst as usize].contains(&entry) {
            return;
        }
        if (self.nfirst as usize) < MAX_UNIMPL {
            self.first[self.nfirst as usize] = entry;
            self.nfirst += 1;
        }
    }

    /// "kind:id:hook" for each recorded entry.
    pub fn describe(&self) -> Vec<String> {
        let kinds = ["move", "ability", "item", "condition"];
        self.first[..self.nfirst as usize]
            .iter()
            .map(|&(k, id, h)| format!("{}:{}:{}", kinds[k as usize], effect_of(k, id).id(), hook_of(h).name()))
            .collect()
    }
}

/// Runs a hook over every effect in play: each active Pokémon's status, volatiles, ability and item, each
/// side's side and slot conditions, and the field's weather, terrain and pseudo-weathers. In stage 1 no
/// such handler is implemented, so this only records what would have acted.
#[inline]
pub fn run_field_hook(s: &mut State, hook: Hook) {
    if quiet(s) {
        return;
    }
    run_field_hook_slow(s, hook);
}

/// No ability or item with handlers, and no condition in play.
#[inline(always)]
fn quiet(s: &State) -> bool {
    if !s.no_holder_fx || s.field.npseudo != 0 || !s.field.weather.id.is_none() || !s.field.terrain.id.is_none() {
        return false;
    }
    s.sides.iter().all(|sd| {
        sd.nvolatiles == 0
            && sd.nconds == 0
            && sd.nslot_conds == 0
            && (!sd.has_active || sd.mons[sd.active as usize].status == Status::None)
    })
}

#[inline(never)]
fn run_field_hook_slow(s: &mut State, hook: Hook) {
    let bit = hook.bit();
    if in_play_mask(s) & bit == 0 {
        return;
    }
    let mut found: [Option<EffectRef>; 24] = [None; 24];
    let mut n = 0;
    for_each_effect(s, |e, mask| {
        if mask & bit != 0 && n < found.len() {
            found[n] = Some(e);
            n += 1;
        }
    });
    for e in found[..n].iter().flatten() {
        s.unimpl.note(*e, hook);
    }
}

/// The union of the unimplemented-handler masks of every effect in play.
#[inline]
fn in_play_mask(s: &State) -> u128 {
    let m = masks();
    let (cu, ab, it) = (&m.conds_unimpl[..], &m.abilities_unimpl[..], &m.items_unimpl[..]);
    let mut acc = 0u128;
    for side in &s.sides {
        if side.has_active {
            let mon = &side.mons[side.active as usize];
            if !mon.fainted {
                if let Some(c) = status_cond(mon.status) {
                    acc |= cu[c.0 as usize];
                }
                for v in &side.volatiles[..side.nvolatiles as usize] {
                    acc |= cu[v.id.0 as usize];
                }
                acc |= ab[mon.ability.0 as usize] | it[mon.item.0 as usize];
            }
        }
        for c in side.conds().iter().chain(side.slot_conds()) {
            acc |= cu[c.id.0 as usize];
        }
    }
    let fd = &s.field;
    acc |= cu[fd.weather.id.0 as usize] | cu[fd.terrain.id.0 as usize];
    for c in &fd.pseudo[..fd.npseudo as usize] {
        acc |= cu[c.id.0 as usize];
    }
    acc
}

/// Every effect in play with its hook mask.
#[inline]
fn for_each_effect(s: &State, mut f: impl FnMut(EffectRef, u128)) {
    let m = masks();
    for side in &s.sides {
        if side.has_active {
            let mon = side.active_mon();
            if !mon.fainted {
                if let Some(c) = status_cond(mon.status) {
                    f(EffectRef::Cond(c), m.conds_unimpl[c.0 as usize]);
                }
                for v in side.volatiles() {
                    f(EffectRef::Cond(v.id), m.conds_unimpl[v.id.0 as usize]);
                }
                f(EffectRef::Ability(mon.ability), m.abilities_unimpl[mon.ability.0 as usize]);
                f(EffectRef::Item(mon.item), m.items_unimpl[mon.item.0 as usize]);
            }
        }
        for c in side.conds().iter().chain(side.slot_conds()) {
            f(EffectRef::Cond(c.id), m.conds_unimpl[c.id.0 as usize]);
        }
    }
    let fd = &s.field;
    for c in [fd.weather, fd.terrain].iter().chain(fd.pseudo[..fd.npseudo as usize].iter()) {
        if !c.id.is_none() {
            f(EffectRef::Cond(c.id), m.conds_unimpl[c.id.0 as usize]);
        }
    }
}

/// A move's own handler at a hook (Showdown's singleEvent on the move). Returns true when the move has a
/// handler there that stage 1 implements; records it when it has one that is not implemented.
pub fn run_move_hook(s: &mut State, mv: MoveId, hook: Hook) -> bool {
    if masks().moves_unimpl[mv.0 as usize] & hook.bit() == 0 {
        return masks().moves[mv.0 as usize] & hook.bit() != 0;
    }
    s.unimpl.note(EffectRef::Move(mv), hook);
    false
}

pub fn status_cond(st: Status) -> Option<Cond> {
    match st {
        Status::None => None,
        Status::Brn => Some(conds::BRN),
        Status::Par => Some(conds::PAR),
        Status::Slp => Some(conds::SLP),
        Status::Frz => Some(conds::FRZ),
        Status::Psn => Some(conds::PSN),
        Status::Tox => Some(conds::TOX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_names_map_to_hooks() {
        assert_eq!(Hook::of_handler("onSourceModifyDamage"), Hook::ModifyDamage);
        assert_eq!(Hook::of_handler("onAnyModifyDamage"), Hook::ModifyDamage);
        assert_eq!(Hook::of_handler("onResidual"), Hook::Residual);
        assert_eq!(Hook::of_handler("onSideResidual"), Hook::Residual);
        assert_eq!(Hook::of_handler("onFoeTrapPokemon"), Hook::TrapPokemon);
        assert_eq!(Hook::of_handler("basePowerCallback"), Hook::BasePower);
        assert_eq!(Hook::of_handler("onSwitchIn"), Hook::SwitchIn);
        let lo = ItemId::from_id("leftovers").unwrap();
        assert!(masks().items[lo.0 as usize] & Hook::Residual.bit() != 0);
        assert_eq!(masks().abilities[abilities::NOABILITY.0 as usize], 0);
        for &m in PLAIN_MOVES.iter() {
            assert_eq!(masks().moves[m.0 as usize], 0, "{}", m.id());
        }
    }
}
