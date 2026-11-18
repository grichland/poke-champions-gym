//! Showdown's runEvent (sim/battle.ts findEventHandlers, resolvePriority, speedSort, runEvent): which handlers
//! an event reaches (the Pokémon's status, volatiles and ability, its foe's, the source's, both sides' side
//! conditions, the field's pseudo-weathers, weather and terrain, in Showdown's list order), their order (order,
//! priority, speed, sub-order), ties drawn through the outcome source, the suppression checks (Mold Breaker,
//! Gastro Acid and Neutralizing Gas, Air Lock), and the relay rules (a falsy result stops the event, a number is
//! scaled by the event's chained modifier). The handlers themselves are in `conds.rs` (statuses, volatiles),
//! `field.rs` (side and field conditions) and `abilities.rs`.

use super::battle::Sim;
use super::data::*;
use super::outcome::*;
use super::state::*;
use std::sync::OnceLock;

/// A Pokémon in play: (side, slot).
pub type Pk = (usize, usize);

#[inline]
pub fn pk_code(p: Pk) -> u8 {
    (p.0 * 4 + p.1) as u8
}

#[inline]
pub fn pk_of(c: u8) -> Option<Pk> {
    if c == NO_MON {
        None
    } else {
        Some(((c / 4) as usize, (c % 4) as usize))
    }
}

macro_rules! events {
    ($($e:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Ev { $($e),* }
        pub const EV_NAMES: &[&str] = &[$(stringify!($e)),*];
        const ALL_EV: &[Ev] = &[$(Ev::$e),*];
    };
}

events!(
    BeforeMove, MoveAborted, AfterMove, LockMove, DisableMove, TrapPokemon, ModifyType, ModifyMove, TryHit,
    Invulnerability, Immunity, Accuracy, BasePower, ModifyCritRatio, ModifyDamage, Damage, TryPrimaryHit, Hit,
    DamagingHit, AfterMoveSecondary, Faint, SwitchIn, Update, StallMove, TryAddVolatile, SetStatus, Type,
    ModifySpe, TryHeal, DragOut, RedirectTarget, ModifyAccuracy, ModifyDef, ModifySpD, Weather,
    // events that only abilities and items handle
    ModifyAtk, ModifySpA, ModifySTAB, ModifyPriority, FractionalPriority, PrepareHit, DeductPP, TryMove,
    CriticalHit, ModifyBoost, ChangeBoost, TryBoost, AfterEachBoost, AfterBoost, AfterSetStatus, EmergencyExit,
    SwitchOut, AfterFaint, ModifySecondaries, AfterMoveSecondarySelf, Flinch, Effectiveness, HitProtect,
    WeatherModifyDamage, ModifyWeight, TryHitSide, Heal, MaybeTrapPokemon, WeatherChange, TerrainChange,
    BeforeSwitchOut, AfterMega, SetWeather, AfterUseItem, TakeItem, TryEatItem, EatItem, BeforeSwitchIn,
    AfterTerastallization, SetAbility, TryImmunity, UseItem, AfterTakeItem, AfterSubDamage,
);

/// The handler prefixes of findEventHandlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Pre {
    Own,
    Source,
    Foe,
    Any,
    Ally,
}

const PRE_NAMES: [&str; 5] = ["", "Source", "Foe", "Any", "Ally"];
const NPRE: usize = 5;

/// A handler's sort keys from the effect's data (resolvePriority).
#[derive(Clone, Copy, Debug, Default)]
struct Keys {
    present: bool,
    order: Option<f32>,
    priority: f32,
    sub_order: Option<f32>,
}

/// Handler keys of every condition (index = condition), ability (index = conditions + ability) and item
/// (index = conditions + abilities + item).
struct Table {
    nev: usize,
    keys: Vec<Keys>,
    /// per effect: the events it has a handler for, any prefix
    evmask: Vec<u128>,
    /// the events any condition has a handler for (the others only abilities handle)
    cond_union: u128,
}

#[inline]
fn ab_index(a: AbilityId) -> usize {
    CONDITIONS.len() + a.0 as usize
}

#[inline]
fn it_index(i: ItemId) -> usize {
    CONDITIONS.len() + ABILITIES.len() + i.0 as usize
}

impl Table {
    #[inline]
    fn at(&self, c: Cond, p: Pre, e: Ev) -> &Keys {
        &self.keys[(c.0 as usize * NPRE + p as usize) * self.nev + e as usize]
    }
    #[inline]
    fn at_ab(&self, a: AbilityId, p: Pre, e: Ev) -> &Keys {
        &self.keys[(ab_index(a) * NPRE + p as usize) * self.nev + e as usize]
    }
    #[inline]
    fn at_it(&self, i: ItemId, p: Pre, e: Ev) -> &Keys {
        &self.keys[(it_index(i) * NPRE + p as usize) * self.nev + e as usize]
    }
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let nev = EV_NAMES.len();
        let neff = CONDITIONS.len() + ABILITIES.len() + ITEMS.len();
        let mut keys = vec![Keys::default(); neff * NPRE * nev];
        let mut evmask = vec![0u128; neff];
        let mut add = |ei: usize, handlers: &[&str], nums: &[(&str, f32)]| {
            for h in handlers {
                let Some(rest) = h.strip_prefix("on") else { continue };
                for (pi, p) in PRE_NAMES.iter().enumerate().rev() {
                    let Some(ev) = rest.strip_prefix(p) else { continue };
                    let Some(evi) = EV_NAMES.iter().position(|x| *x == ev) else { continue };
                    let num = |suffix: &str| {
                        let name = format!("{}{}", h, suffix);
                        nums.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
                    };
                    evmask[ei] |= 1u128 << evi;
                    keys[(ei * NPRE + pi) * nev + evi] = Keys {
                        present: true,
                        order: num("Order"),
                        priority: num("Priority").unwrap_or(0.0),
                        sub_order: num("SubOrder"),
                    };
                    break;
                }
            }
        };
        for (ci, c) in CONDITIONS.iter().enumerate() {
            add(ci, c.handlers, c.nums);
        }
        for (ai, a) in ABILITIES.iter().enumerate() {
            add(CONDITIONS.len() + ai, a.handlers, a.nums);
        }
        for (ii, it) in ITEMS.iter().enumerate() {
            add(CONDITIONS.len() + ABILITIES.len() + ii, it.handlers, it.nums);
        }
        let cond_union = evmask[..CONDITIONS.len()].iter().fold(0, |m, x| m | x);
        Table { nev, keys, evmask, cond_union }
    })
}

/// Whether condition `c` has a handler for `e`, any prefix.
#[inline]
pub fn handles(c: Cond, e: Ev) -> bool {
    table().evmask[c.0 as usize] & (1u128 << e as u8) != 0
}

/// Whether ability `a` has a handler for `e`, any prefix.
#[inline]
pub fn ability_handles(a: AbilityId, e: Ev) -> bool {
    table().evmask[ab_index(a)] & (1u128 << e as u8) != 0
}

/// The events ability `a` has handlers for.
#[inline]
pub fn ability_evmask(a: AbilityId) -> u128 {
    table().evmask[ab_index(a)]
}

/// Whether condition `c` has a handler for `e` with prefix `p`.
#[inline]
pub fn has_handler(c: Cond, p: Pre, e: Ev) -> bool {
    table().at(c, p, e).present
}

/// Whether ability `a` has a handler for `e` with prefix `p`, and its priority.
#[inline]
pub fn ability_handler(a: AbilityId, p: Pre, e: Ev) -> Option<f32> {
    let k = table().at_ab(a, p, e);
    if k.present {
        Some(k.priority)
    } else {
        None
    }
}

/// Whether item `i` has a handler for `e`, any prefix.
#[inline]
pub fn item_handles(i: ItemId, e: Ev) -> bool {
    table().evmask[it_index(i)] & (1u128 << e as u8) != 0
}

/// The events item `i` has handlers for.
#[inline]
pub fn item_evmask(i: ItemId) -> u128 {
    table().evmask[it_index(i)]
}

/// Whether item `i` has a handler for `e` with prefix `p`, and its priority.
#[inline]
pub fn item_handler(i: ItemId, p: Pre, e: Ev) -> Option<f32> {
    let k = table().at_it(i, p, e);
    if k.present {
        Some(k.priority)
    } else {
        None
    }
}

/// The sub-order of an ability's handlers when the data gives none (resolvePriority's effect-type order).
pub fn ability_sub_order(a: AbilityId) -> f32 {
    match a {
        abilities::POISONTOUCH | abilities::PERISHBODY => 6.0,
        abilities::STALL => 9.0,
        _ => 7.0,
    }
}

/// Showdown's relay values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum R {
    Undef,
    Null,
    False,
    True,
    /// NOT_FAIL (''): falsy, but not a failure
    NotFail,
    Num(i32),
    Mv(MoveId),
}

impl R {
    #[inline]
    pub fn truthy(self) -> bool {
        match self {
            R::Undef | R::Null | R::False | R::NotFail => false,
            R::Num(n) => n != 0,
            R::True | R::Mv(_) => true,
        }
    }
    pub fn of_bool(b: bool) -> R {
        if b {
            R::True
        } else {
            R::False
        }
    }
}

/// The effect of an event (Showdown's sourceEffect / this.effect).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eff {
    None,
    /// a move (effectType Move)
    Move(MoveId),
    Cond(Cond),
    Ability(AbilityId),
    Item(ItemId),
    Recoil,
    Drain,
    /// the confusion self-hit's pseudo-move (effectType Move, typeless)
    Confused,
    StruggleRecoil,
}

impl Eff {
    pub fn is_move(self) -> bool {
        matches!(self, Eff::Move(_) | Eff::Confused)
    }
}

/// Where a handler lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    Status(u8),
    Volatile(u8),
    Ability,
    Item,
    /// a side condition: side, index in `Side::conds`
    Side(u8, u8),
    Weather,
    Terrain,
    /// a pseudo-weather: index in `Field::pseudo`
    Pseudo(u8),
}

/// One handler of an event, with its sort keys and a copy of its state (Showdown passes the state object,
/// which a handler can still use after its condition was removed earlier in the same event).
#[derive(Clone, Copy, Debug)]
pub struct H {
    pub cond: Cond,
    /// the ability of an ability handler (the one collected, even if the holder's ability changed since)
    pub ab: AbilityId,
    /// the item of an item handler
    pub item: ItemId,
    pub pre: Pre,
    pub holder: Holder,
    /// the holder Pokémon (side and field conditions: the event's target)
    pub pk: Pk,
    /// the side of the effect's holder (a side condition's side; None for the field), for ties
    pub hside: Option<u8>,
    pub state: Volatile,
    order: f64,
    priority: f32,
    speed: i32,
    sub_order: f32,
    /// the holder Pokémon's abilityState.effectOrder (compareRedirectOrder); None for side and field holders
    ab_order: Option<u16>,
}

impl H {
    pub fn effect(&self) -> EffectRef {
        match self.holder {
            Holder::Ability => EffectRef::Ability(self.ab),
            Holder::Item => EffectRef::Item(self.item),
            _ => EffectRef::Cond(self.cond),
        }
    }
}

/// A boosts table as Showdown passes it (an object whose key order is the insertion order).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Boosts {
    pub v: [(u8, i8); 7],
    pub n: u8,
}

impl Boosts {
    pub fn of(list: &[(usize, i8)]) -> Boosts {
        let mut b = Boosts::default();
        for &(i, v) in list {
            b.set(i, v);
        }
        b
    }
    /// the full table of a Pokémon's stages (the ModifyBoost event's relay)
    pub fn full(stages: &[i8; 7]) -> Boosts {
        let mut b = Boosts::default();
        for i in 0..7 {
            b.v[i] = (i as u8, stages[i]);
        }
        b.n = 7;
        b
    }
    pub fn as_slice(&self) -> &[(u8, i8)] {
        &self.v[..self.n as usize]
    }
    pub fn get(&self, i: usize) -> Option<i8> {
        self.as_slice().iter().find(|x| x.0 as usize == i).map(|x| x.1)
    }
    pub fn set(&mut self, i: usize, v: i8) {
        if let Some(x) = self.v[..self.n as usize].iter_mut().find(|x| x.0 as usize == i) {
            x.1 = v;
        } else if (self.n as usize) < 7 {
            self.v[self.n as usize] = (i as u8, v);
            self.n += 1;
        }
    }
    pub fn delete(&mut self, i: usize) {
        let n = self.n as usize;
        if let Some(k) = self.v[..n].iter().position(|x| x.0 as usize == i) {
            self.v.copy_within(k + 1..n, k);
            self.n -= 1;
        }
    }
}

/// What an event passes to its handlers besides the relay value.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub target: Option<Pk>,
    pub source: Option<Pk>,
    pub eff: Eff,
    /// the move of the event when there is one (a copy; handlers that change it are read back by callers)
    pub am: Option<super::moves::ActiveMove>,
    /// this.event.modifier, 4096ths
    pub modifier: u32,
    /// an extra input: the status or type of Immunity, SetStatus and TryAddVolatile events
    pub arg: u16,
    /// the boosts table of ChangeBoost, TryBoost, AfterEachBoost, AfterBoost and ModifyBoost
    pub boosts: Boosts,
}

impl Ctx {
    pub fn new(target: Option<Pk>, source: Option<Pk>, eff: Eff) -> Ctx {
        Ctx { target, source, eff, am: None, modifier: 4096, arg: 0, boosts: Boosts::default() }
    }
    pub fn with_move(target: Option<Pk>, source: Option<Pk>, am: &super::moves::ActiveMove) -> Ctx {
        Ctx { target, source, eff: Eff::Move(am.id), am: Some(*am), modifier: 4096, arg: 0, boosts: Boosts::default() }
    }
    /// chainModify(num/den)
    pub fn chain_modify(&mut self, num: u32, den: u32) {
        let next = (num as u64 * 4096 / den as u64) as u64;
        self.modifier = ((self.modifier as u64 * next + 2048) >> 12) as u32;
    }
}

/// Events that are run through eachEvent and never see prefixed handlers.
fn prefixed(e: Ev) -> bool {
    !matches!(e, Ev::Update | Ev::Weather | Ev::WeatherChange | Ev::TerrainChange)
}

const MAX_H: usize = 24;

impl<'a, O: Outcomes> Sim<'a, O> {
    /// Handlers of one Pokémon for `e` with prefix `p` (findPokemonEventHandlers): status, volatiles, ability.
    fn push_pokemon(&mut self, pk: Pk, p: Pre, e: Ev, out: &mut [Option<H>; MAX_H], n: &mut usize) {
        let t = table();
        let sd = &self.s.sides[pk.0];
        if !sd.has_active || sd.active as usize != pk.1 {
            return;
        }
        let m = &sd.mons[pk.1];
        let speed = m.speed;
        let abo = Some(m.ab_order);
        if let Some(c) = super::hooks::status_cond(m.status) {
            let k = t.at(c, p, e);
            if k.present && *n < MAX_H {
                out[*n] = Some(H {
                    cond: c,
                    ab: AbilityId::NONE,
                    item: ItemId::NONE,
                    pre: p,
                    holder: Holder::Status(m.status as u8),
                    pk,
                    hside: Some(pk.0 as u8),
                    state: Volatile::default(),
                    order: k.order.map(|x| x as f64).unwrap_or(4294967296.0),
                    priority: k.priority,
                    speed,
                    sub_order: k.sub_order.unwrap_or(0.0),
                    ab_order: abo,
                });
                *n += 1;
            }
        }
        for (i, v) in sd.volatiles().iter().enumerate() {
            let k = t.at(v.id, p, e);
            if k.present && *n < MAX_H {
                out[*n] = Some(H {
                    cond: v.id,
                    ab: AbilityId::NONE,
                    item: ItemId::NONE,
                    pre: p,
                    holder: Holder::Volatile(i as u8),
                    pk,
                    hside: Some(pk.0 as u8),
                    state: *v,
                    order: k.order.map(|x| x as f64).unwrap_or(4294967296.0),
                    priority: k.priority,
                    speed,
                    sub_order: k.sub_order.unwrap_or(2.0),
                    ab_order: abo,
                });
                *n += 1;
            }
        }
        let ab = m.ability;
        let k = t.at_ab(ab, p, e);
        if k.present && *n < MAX_H {
            out[*n] = Some(H {
                cond: Cond::NONE,
                ab,
                item: ItemId::NONE,
                pre: p,
                holder: Holder::Ability,
                pk,
                hside: Some(pk.0 as u8),
                state: Volatile::default(),
                order: k.order.map(|x| x as f64).unwrap_or(4294967296.0),
                priority: k.priority,
                speed,
                sub_order: k.sub_order.filter(|x| *x != 0.0).unwrap_or(ability_sub_order(ab)),
                ab_order: abo,
            });
            *n += 1;
        }
        let it = m.item;
        if !it.is_none() {
            let k = t.at_it(it, p, e);
            if k.present && *n < MAX_H {
                out[*n] = Some(H {
                    cond: Cond::NONE,
                    ab: AbilityId::NONE,
                    item: it,
                    pre: p,
                    holder: Holder::Item,
                    pk,
                    hside: Some(pk.0 as u8),
                    state: Volatile::default(),
                    order: k.order.map(|x| x as f64).unwrap_or(4294967296.0),
                    priority: k.priority,
                    speed,
                    sub_order: k.sub_order.filter(|x| *x != 0.0).unwrap_or(8.0),
                    ab_order: abo,
                });
                *n += 1;
            }
        }
    }

    /// Handlers of a side's side conditions for `e` with prefix `p` (findSideEventHandlers), in creation order.
    fn push_side(&mut self, side: usize, at: Pk, p: Pre, e: Ev, out: &mut [Option<H>; MAX_H], n: &mut usize) {
        let t = table();
        let sd = &self.s.sides[side];
        for (i, c) in sd.conds().iter().enumerate() {
            let k = t.at(c.id, p, e);
            if k.present && *n < MAX_H {
                out[*n] = Some(H {
                    cond: c.id,
                    ab: AbilityId::NONE,
                    item: ItemId::NONE,
                    pre: p,
                    holder: Holder::Side(side as u8, i as u8),
                    pk: at,
                    hside: Some(side as u8),
                    state: Volatile::default(),
                    order: k.order.map(|x| x as f64).unwrap_or(4294967296.0),
                    priority: k.priority,
                    speed: 0,
                    sub_order: k.sub_order.filter(|x| *x != 0.0).unwrap_or(4.0),
                    ab_order: None,
                });
                *n += 1;
            }
        }
    }

    /// Handlers of the field for `e` (findFieldEventHandlers): pseudo-weathers in creation order, weather,
    /// terrain. Sub-orders by effect type: a weather 5, a terrain 0, a pseudo-weather 5 once its state has a
    /// target and 2 before.
    fn push_field(&mut self, at: Pk, e: Ev, out: &mut [Option<H>; MAX_H], n: &mut usize) {
        let t = table();
        let f = &self.s.field;
        let add = |c: Cond, holder: Holder, def_sub: f32, out: &mut [Option<H>; MAX_H], n: &mut usize| {
            let k = t.at(c, Pre::Own, e);
            if k.present && *n < MAX_H {
                out[*n] = Some(H {
                    cond: c,
                    ab: AbilityId::NONE,
                    item: ItemId::NONE,
                    pre: Pre::Own,
                    holder,
                    pk: at,
                    hside: None,
                    state: Volatile::default(),
                    order: k.order.map(|x| x as f64).unwrap_or(4294967296.0),
                    priority: k.priority,
                    speed: 0,
                    sub_order: k.sub_order.filter(|x| *x != 0.0).unwrap_or(def_sub),
                    ab_order: None,
                });
                *n += 1;
            }
        };
        for (i, c) in f.pseudo[..f.npseudo as usize].iter().enumerate() {
            add(c.id, Holder::Pseudo(i as u8), if c.targeted { 5.0 } else { 2.0 }, out, n);
        }
        if !f.weather.id.is_none() {
            add(f.weather.id, Holder::Weather, 5.0, out, n);
        }
        if !f.terrain.id.is_none() {
            add(f.terrain.id, Holder::Terrain, 0.0, out, n);
        }
    }

    /// findEventHandlers for an event aimed at Pokémon `target` with `source`, singles. Slot conditions are
    /// left out: none of the legal ones has a handler for an event run through runEvent.
    fn collect(&mut self, e: Ev, target: Option<Pk>, source: Option<Pk>, out: &mut [Option<H>; MAX_H]) -> usize {
        let mut n = 0;
        let active = |s: &State, p: Option<Pk>| {
            p.map_or(false, |(sd, sl)| {
                let x = &s.sides[sd];
                x.has_active && x.active as usize == sl && x.mons[sl].is_active
            })
        };
        let mut bubble = None;
        if let Some(t) = target {
            if active(self.s, Some(t)) || active(self.s, source) {
                self.push_pokemon(t, Pre::Own, e, out, &mut n);
                if prefixed(e) {
                    // alliesAndSelf: the target side's active Pokémon with HP left (the target may be benched)
                    let ts = &self.s.sides[t.0];
                    if ts.has_active && ts.active_mon().hp > 0 {
                        let ap = (t.0, ts.active as usize);
                        self.push_pokemon(ap, Pre::Ally, e, out, &mut n);
                        self.push_pokemon(ap, Pre::Any, e, out, &mut n);
                    }
                    let f = 1 - t.0;
                    let fs = &self.s.sides[f];
                    if fs.has_active && fs.active_mon().hp > 0 {
                        let fp = (f, fs.active as usize);
                        self.push_pokemon(fp, Pre::Foe, e, out, &mut n);
                        self.push_pokemon(fp, Pre::Any, e, out, &mut n);
                    }
                }
                bubble = Some(t);
            }
        }
        if let Some(s) = source {
            if prefixed(e) {
                self.push_pokemon(s, Pre::Source, e, out, &mut n);
            }
        }
        let at = target.unwrap_or((0, 0));
        if let Some(t) = bubble {
            for side in 0..SIDES {
                if self.s.sides[side].nconds == 0 {
                    continue;
                }
                if side == t.0 {
                    self.push_side(side, at, Pre::Own, e, out, &mut n);
                } else if prefixed(e) {
                    self.push_side(side, at, Pre::Foe, e, out, &mut n);
                }
                if prefixed(e) {
                    self.push_side(side, at, Pre::Any, e, out, &mut n);
                }
            }
        }
        if self.s.field.npseudo != 0 || !self.s.field.weather.id.is_none() || !self.s.field.terrain.id.is_none() {
            self.push_field(at, e, out, &mut n);
        }
        n
    }

    /// speedSort of handlers with Showdown's comparePriority; a tie of two draws a `speedtie`.
    fn sort_handlers(&mut self, hs: &mut [Option<H>], left_to_right: bool, fast_exit: bool) {
        let n = hs.len();
        if n < 2 {
            return;
        }
        let cmp = |a: &H, b: &H| -> f64 {
            if a.order != b.order {
                return a.order - b.order;
            }
            if a.priority != b.priority {
                return (b.priority - a.priority) as f64;
            }
            if left_to_right {
                return 0.0;
            }
            if a.speed != b.speed {
                return (b.speed - a.speed) as f64;
            }
            if fast_exit {
                return 0.0;
            }
            (a.sub_order - b.sub_order) as f64
        };
        if left_to_right || fast_exit {
            // Array.prototype.sort: stable, no draws (compareLeftToRightOrder, compareRedirectOrder)
            let mut v: Vec<H> = hs.iter().flatten().copied().collect();
            if fast_exit {
                v.sort_by(|a, b| {
                    let pa = (b.priority - a.priority) as f64;
                    let mut d = if pa != 0.0 { pa } else { (b.speed - a.speed) as f64 };
                    if d == 0.0 {
                        if let (Some(x), Some(y)) = (a.ab_order, b.ab_order) {
                            d = x as f64 - y as f64;
                        }
                    }
                    d.partial_cmp(&0.0).unwrap()
                });
            } else {
                v.sort_by(|a, b| cmp(a, b).partial_cmp(&0.0).unwrap());
            }
            for (i, h) in v.into_iter().enumerate() {
                hs[i] = Some(h);
            }
            return;
        }
        let mut sorted = 0;
        while sorted + 1 < n {
            let mut next = [0usize; MAX_H];
            let mut nn = 1;
            next[0] = sorted;
            for i in sorted + 1..n {
                let d = cmp(hs[next[0]].as_ref().unwrap(), hs[i].as_ref().unwrap());
                if d < 0.0 {
                    continue;
                }
                if d > 0.0 {
                    nn = 1;
                    next[0] = i;
                } else {
                    next[nn] = i;
                    nn += 1;
                }
            }
            for (k, &idx) in next[..nn].iter().enumerate() {
                if idx != sorted + k {
                    hs.swap(sorted + k, idx);
                }
            }
            if nn == 2 {
                let a = hs[sorted].unwrap();
                let b = hs[sorted + 1].unwrap();
                let ea = a.effect();
                let eb = b.effect();
                let site = Site { effect: Effect::Pair(ea, eb), ..Site::new(Key::SpeedTie, None) };
                // the oracle's tieSite: two holder sides, else two effect ids, else a raw shuffle draw
                let first_a = match (a.hside, b.hside) {
                    (Some(x), Some(y)) if x != y => self.o.tie(&site, x, y) == x,
                    _ if ea != eb => {
                        let site = Site { side: a.hside.or(b.hside), ..site };
                        self.o.tie_effects(&site, ea, eb)
                    }
                    _ => {
                        let side = pk_of(self.s.active_pokemon).map(|p| p.0 as u8);
                        let s0 = sorted as i32;
                        let site = Site { args: Some((s0, s0 + 2)), ..Site { side, ..Site::new(Key::Raw, None) } };
                        let site = Site { effect: Effect::Pair(ea, eb), ..site };
                        self.o.range(&site, s0, s0 + 2) == s0
                    }
                };
                if !first_a {
                    hs.swap(sorted, sorted + 1);
                }
            } else if nn > 2 {
                // prng.shuffle: one draw per position (the oracle's raw shuffle events)
                let end = sorted + nn;
                let side = pk_of(self.s.active_pokemon).map(|p| p.0 as u8);
                for i in sorted..end - 1 {
                    let (a, b) = (hs[i].unwrap(), hs[i + 1].unwrap());
                    let site = Site {
                        effect: Effect::Pair(a.effect(), b.effect()),
                        args: Some((i as i32, end as i32)),
                        ..Site::new(Key::Raw, side)
                    };
                    let r = self.o.range(&site, i as i32, end as i32) as usize;
                    if r != i {
                        hs.swap(i, r);
                    }
                }
            }
            sorted += nn;
        }
    }

    /// Nothing that can hold a handler is in play: no status, volatile, side, slot or field condition, and
    /// no active ability with handlers.
    #[inline]
    pub fn nothing_in_play(&self) -> bool {
        let f = &self.s.field;
        let quiet = f.npseudo == 0
            && f.weather.id.is_none()
            && f.terrain.id.is_none()
            && self.s.sides.iter().all(|sd| sd.nvolatiles == 0 && sd.nconds == 0 && sd.mons.iter().all(|m| m.status == Status::None));
        if !quiet {
            return false;
        }
        if self.s.ab_union == 0 && self.s.it_union == 0 {
            return true;
        }
        let t = table();
        self.s.sides.iter().all(|sd| {
            !sd.has_active || {
                let m = &sd.mons[sd.active as usize];
                t.evmask[ab_index(m.ability)] == 0 && t.evmask[it_index(m.item)] == 0
            }
        })
    }

    /// runEvent(e, target, source, effect, relay) over the effects in play.
    pub fn run_event(&mut self, e: Ev, ctx: &mut Ctx, relay: R) -> R {
        let r = self.run_event_chain(e, ctx, relay);
        if let R::Num(x) = r {
            if x >= 0 && ctx.modifier != 4096 {
                return R::Num(super::calc::modify(x as u32, ctx.modifier) as i32);
            }
        }
        r
    }

    /// runEvent without its last step: the chained modifier stays in `ctx` for the caller to apply.
    pub fn run_event_chain(&mut self, e: Ev, ctx: &mut Ctx, relay: R) -> R {
        if self.nothing_in_play() {
            return relay;
        }
        let mut hs: [Option<H>; MAX_H] = [None; MAX_H];
        let n = self.collect(e, ctx.target, ctx.source, &mut hs);
        if n == 0 {
            return relay;
        }
        let ltr = matches!(e, Ev::Invulnerability | Ev::TryHit | Ev::DamagingHit);
        let fast = matches!(e, Ev::LockMove | Ev::RedirectTarget | Ev::WeatherModifyDamage);
        self.sort_handlers(&mut hs[..n], ltr, fast);
        let mut relay = relay;
        for h in hs[..n].iter().flatten() {
            // Air Lock and Cloud Nine: weather handlers and the Weather event do nothing
            if (h.holder == Holder::Weather || e == Ev::Weather) && self.suppressing_weather() {
                continue;
            }
            let r = match h.holder {
                Holder::Status(st) => {
                    let m = &self.s.sides[h.pk.0].mons[h.pk.1];
                    if m.status as u8 != st {
                        continue;
                    }
                    self.cond_event(e, h, ctx, relay)
                }
                Holder::Volatile(_) => self.cond_event(e, h, ctx, relay),
                Holder::Ability => {
                    if self.ability_skipped(h.pk, h.ab, e) {
                        continue;
                    }
                    if super::abilities::coverage_on() {
                        let (before, cb) = (*self.s, *ctx);
                        let r = self.ability_event(e, h, ctx, relay);
                        if r != R::Undef || *self.s != before || ctx.modifier != cb.modifier || ctx.boosts != cb.boosts
                            || ctx.am != cb.am
                        {
                            super::abilities::count_trigger(h.ab);
                        }
                        r
                    } else {
                        self.ability_event(e, h, ctx, relay)
                    }
                }
                Holder::Item => {
                    // Embargo, Klutz and Magic Room: no item handler runs, except on taking the item
                    if !matches!(e, Ev::SwitchIn | Ev::TakeItem) && self.ignoring_item(h.pk, false) {
                        continue;
                    }
                    if super::abilities::coverage_on() {
                        let (before, cb) = (*self.s, *ctx);
                        let r = self.item_event(e, h, ctx, relay);
                        if r != R::Undef || *self.s != before || ctx.modifier != cb.modifier || ctx.boosts != cb.boosts
                            || ctx.am != cb.am
                        {
                            super::items::count_trigger(h.item);
                        }
                        r
                    } else {
                        self.item_event(e, h, ctx, relay)
                    }
                }
                Holder::Pseudo(i) => {
                    // the handler's state gets its target (resolvePriority reads it for the sub-order)
                    let f = &mut self.s.field;
                    if (i as usize) < f.npseudo as usize && f.pseudo[i as usize].id == h.cond {
                        f.pseudo[i as usize].targeted = true;
                    }
                    self.field_cond_event(e, h, ctx, relay)
                }
                Holder::Side(..) | Holder::Weather | Holder::Terrain => self.field_cond_event(e, h, ctx, relay),
            };
            if r != R::Undef {
                relay = r;
                if !relay.truthy() || fast {
                    break;
                }
            }
        }
        relay
    }

    /// priorityEvent: the first handler that returns anything decides.
    pub fn priority_event(&mut self, e: Ev, ctx: &mut Ctx, relay: R) -> R {
        self.run_event(e, ctx, relay)
    }

    /// Whether any effect in play could react to `e` (a fast path before building a context).
    #[inline]
    pub fn any_handler(&self, e: Ev) -> bool {
        let bit = 1u128 << e as u8;
        let t = table();
        if self.s.ab_union & bit != 0 {
            for sd in &self.s.sides {
                if sd.has_active && t.evmask[ab_index(sd.active_mon().ability)] & bit != 0 {
                    return true;
                }
            }
        }
        if self.s.it_union & bit != 0 {
            for sd in &self.s.sides {
                if sd.has_active && t.evmask[it_index(sd.active_mon().item)] & bit != 0 {
                    return true;
                }
            }
        }
        // an event no condition handles: the abilities and items decided
        if t.cond_union & bit == 0 {
            return false;
        }
        // no condition in play at all (the common case of a quiet field)
        let f = &self.s.field;
        let [a, b] = &self.s.sides;
        let active_status = |sd: &Side| sd.has_active && sd.mons[sd.active as usize].status != Status::None;
        if f.npseudo == 0 && f.weather.id.is_none() && f.terrain.id.is_none() && a.nvolatiles == 0 && b.nvolatiles == 0
            && a.nconds == 0 && b.nconds == 0 && !active_status(a) && !active_status(b)
        {
            return false;
        }
        for sd in &self.s.sides {
            if !sd.has_active {
                continue;
            }
            let m = sd.active_mon();
            if let Some(c) = super::hooks::status_cond(m.status) {
                if t.evmask[c.0 as usize] & bit != 0 {
                    return true;
                }
            }
            for v in sd.volatiles() {
                if t.evmask[v.id.0 as usize] & bit != 0 {
                    return true;
                }
            }
            for c in sd.conds() {
                if t.evmask[c.id.0 as usize] & bit != 0 {
                    return true;
                }
            }
        }
        let f = &self.s.field;
        for c in f.pseudo[..f.npseudo as usize].iter().chain([f.weather, f.terrain].iter()) {
            if t.evmask[c.id.0 as usize] & bit != 0 {
                return true;
            }
        }
        false
    }

    /// Whether an active Pokémon's ability has a handler for `e` (any prefix).
    #[inline]
    pub fn any_ability_handler(&self, e: Ev) -> bool {
        let bit = 1u128 << e as u8;
        if self.s.ab_union & bit == 0 {
            return false;
        }
        let t = table();
        self.s.sides.iter().any(|sd| sd.has_active && t.evmask[ab_index(sd.active_mon().ability)] & bit != 0)
    }
}

#[allow(dead_code)]
fn ev_of(name: &str) -> Option<Ev> {
    EV_NAMES.iter().position(|x| *x == name).map(|i| ALL_EV[i])
}
