//! Items (data/items.ts and data/mods/champions/items.ts): the handlers of every legal item, the conditions
//! items create (choicelock, metronome, gem, fling), the item primitives of sim/pokemon.ts (eatItem, useItem,
//! takeItem, setItem), item suppression (pokemon.ignoringItem: Klutz, Magic Room) and the items read by
//! other effects (weather rocks, Light Clay, Terrain Extender, Binding Band). Messages are left out.

use super::battle::Sim;
use super::data::*;
use super::events::*;
use super::outcome::*;
use super::state::*;
use super::log::*;
use std::sync::atomic::{AtomicU32, Ordering};

const ZERO: AtomicU32 = AtomicU32::new(0);
/// Test coverage: item handlers that changed something (rig only, like abilities::TRIGGERS).
pub static TRIGGERS: [AtomicU32; 170] = [ZERO; 170];

#[inline]
pub fn count_trigger(i: ItemId) {
    TRIGGERS[(i.0 as usize).min(169)].fetch_add(1, Ordering::Relaxed);
}

/// The items other than Mega stones whose handlers are all implemented here (every stone is: its only handler
/// is onTakeItem).
pub const IMPLEMENTED_ITEMS: &[&str] = &[
    "airballoon", "aspearberry", "babiriberry", "bigroot", "bindingband", "blackbelt", "blackglasses", "brightpowder",
    "charcoal", "chartiberry", "cheriberry", "chestoberry", "chilanberry", "choicescarf", "chopleberry", "cobaberry",
    "colburberry", "damprock", "dragonfang", "ejectbutton", "electricseed", "expertbelt", "fairyfeather", "focusband",
    "focussash", "grassyseed", "habanberry", "hardstone", "heatrock", "icyrock", "ironball", "kasibberry", "kebiaberry",
    "kingsrock", "leek", "leftovers", "leppaberry", "lifeorb", "lightball", "lightclay", "lumberry", "magnet",
    "mentalherb", "metalcoat", "metronome", "miracleseed", "mistyseed", "muscleband", "mysticwater", "nevermeltice",
    "normalgem", "occaberry", "oranberry", "passhoberry", "payapaberry", "pechaberry", "persimberry", "poisonbarb",
    "psychicseed", "quickclaw", "rawstberry", "redcard", "rindoberry", "rockyhelmet", "roseliberry", "scopelens",
    "sharpbeak", "shedshell", "shellbell", "shucaberry", "silkscarf", "silverpowder", "sitrusberry", "smoothrock",
    "softsand", "spelltag", "tangaberry", "terrainextender", "twistedspoon", "wacanberry", "whiteherb", "widelens",
    "wiseglasses", "yacheberry", "zoomlens",
];

pub fn item_implemented(d: &ItemData) -> bool {
    !d.mega_stone.is_empty() || IMPLEMENTED_ITEMS.contains(&d.id)
}

/// What pokemon.takeItem returned: undefined (no item), false (blocked) or the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    NoItem,
    Blocked,
    Item(ItemId),
}

impl Taken {
    pub fn item(self) -> Option<ItemId> {
        match self {
            Taken::Item(i) => Some(i),
            _ => None,
        }
    }
}

/// The type a type-boosting item powers up (x4915/4096 on base power).
fn type_boost(i: ItemId) -> Option<Type> {
    use items as I;
    Some(match i {
        I::BLACKBELT => Type::Fighting,
        I::BLACKGLASSES => Type::Dark,
        I::CHARCOAL => Type::Fire,
        I::DRAGONFANG => Type::Dragon,
        I::FAIRYFEATHER => Type::Fairy,
        I::HARDSTONE => Type::Rock,
        I::MAGNET => Type::Electric,
        I::METALCOAT => Type::Steel,
        I::MIRACLESEED => Type::Grass,
        I::MYSTICWATER => Type::Water,
        I::NEVERMELTICE => Type::Ice,
        I::POISONBARB => Type::Poison,
        I::SHARPBEAK => Type::Flying,
        I::SILKSCARF => Type::Normal,
        I::SILVERPOWDER => Type::Bug,
        I::SOFTSAND => Type::Ground,
        I::SPELLTAG => Type::Ghost,
        I::TWISTEDSPOON => Type::Psychic,
        _ => return None,
    })
}

/// The type a resist berry halves (Chilan Berry: Normal, without the super-effective test).
fn resist_type(i: ItemId) -> Option<Type> {
    use items as I;
    Some(match i {
        I::OCCABERRY => Type::Fire,
        I::PASSHOBERRY => Type::Water,
        I::WACANBERRY => Type::Electric,
        I::RINDOBERRY => Type::Grass,
        I::YACHEBERRY => Type::Ice,
        I::CHOPLEBERRY => Type::Fighting,
        I::KEBIABERRY => Type::Poison,
        I::SHUCABERRY => Type::Ground,
        I::COBABERRY => Type::Flying,
        I::PAYAPABERRY => Type::Psychic,
        I::TANGABERRY => Type::Bug,
        I::CHARTIBERRY => Type::Rock,
        I::KASIBBERRY => Type::Ghost,
        I::HABANBERRY => Type::Dragon,
        I::BABIRIBERRY => Type::Steel,
        I::COLBURBERRY => Type::Dark,
        I::ROSELIBERRY => Type::Fairy,
        I::CHILANBERRY => Type::Normal,
        _ => return None,
    })
}

pub fn is_resist_berry(i: ItemId) -> bool {
    resist_type(i).is_some()
}

/// The terrain a seed reacts to and the stat it raises.
fn seed(i: ItemId) -> Option<(Cond, usize)> {
    use items as I;
    match i {
        I::ELECTRICSEED => Some((conds::ELECTRICTERRAIN, 1)),
        I::GRASSYSEED => Some((conds::GRASSYTERRAIN, 1)),
        I::MISTYSEED => Some((conds::MISTYTERRAIN, 3)),
        I::PSYCHICSEED => Some((conds::PSYCHICTERRAIN, 3)),
        _ => None,
    }
}

/// item.fling: what a flung item does besides its power (status, volatile, or an onHit effect)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlingFx {
    None,
    Status(Status),
    Volatile(Cond),
    /// item.fling.effect (White Herb, Mental Herb) or a berry's onEat: run on the target
    OnHit,
}

pub fn fling_fx(i: ItemId) -> FlingFx {
    use items as I;
    let d = i.data();
    if d.is_berry {
        return FlingFx::OnHit;
    }
    match i {
        I::LIGHTBALL => FlingFx::Status(Status::Par),
        I::POISONBARB => FlingFx::Status(Status::Psn),
        I::KINGSROCK => FlingFx::Volatile(conds::FLINCH),
        I::WHITEHERB | I::MENTALHERB => FlingFx::OnHit,
        _ => FlingFx::None,
    }
}

const MENTAL: [Cond; 6] = [conds::ATTRACT, conds::TAUNT, conds::ENCORE, conds::TORMENT, conds::DISABLE, conds::HEALBLOCK];

impl<'a, O: Outcomes> Sim<'a, O> {
    // ------------------------------------------------------------------------------------------ state

    /// pokemon.ignoringItem(isFling)
    pub fn ignoring_item(&self, p: Pk, is_fling: bool) -> bool {
        let m = self.mon_at(p);
        if !m.is_active {
            return true;
        }
        if self.s.field.npseudo != 0 && self.has_pseudo(conds::MAGICROOM) {
            return true;
        }
        if is_fling {
            return self.has_ability(p, abilities::KLUTZ);
        }
        !m.item.data().ignore_klutz && m.ability == abilities::KLUTZ && self.has_ability(p, abilities::KLUTZ)
    }

    /// pokemon.hasItem
    #[inline]
    pub fn has_item(&self, p: Pk, i: ItemId) -> bool {
        self.mon_at(p).item == i && !self.ignoring_item(p, false)
    }

    /// A Mega stone's onTakeItem: false when the holder is of the species the stone is for
    pub fn stone_blocks(&self, item: ItemId, holder: Pk) -> bool {
        let d = item.data();
        if d.mega_stone.is_empty() {
            return false;
        }
        let bs = self.mon_at(holder).base_species;
        if item == items::MEOWSTICITE || item == items::FLOETTITE {
            return d.mega_stone.iter().any(|&(from, to)| from == bs || to == bs);
        }
        let base = bs.data().base_species;
        d.mega_stone.iter().any(|&(from, _)| from.data().base_species == base)
    }

    /// singleEvent('TakeItem', item, ..., holder): only the item's own onTakeItem (the stones)
    pub fn item_take_single(&self, item: ItemId, holder: Pk) -> bool {
        !self.stone_blocks(item, holder)
    }

    // ------------------------------------------------------------------------------------------ primitives

    /// runEvent('UseItem', pokemon, null, null, item)
    fn use_item_event(&mut self, p: Pk, item: ItemId) -> bool {
        if !self.any_handler(Ev::UseItem) {
            return true;
        }
        let mut ctx = Ctx::new(Some(p), None, Eff::None);
        ctx.arg = item.0;
        self.run_event(Ev::UseItem, &mut ctx, R::True).truthy()
    }

    /// runEvent('AfterUseItem', pokemon, null, null, item)
    pub fn after_use_item(&mut self, p: Pk, item: ItemId) {
        if !self.any_handler(Ev::AfterUseItem) {
            return;
        }
        let mut ctx = Ctx::new(Some(p), None, Eff::None);
        ctx.arg = item.0;
        self.run_event(Ev::AfterUseItem, &mut ctx, R::True);
    }

    /// runEvent('EatItem', pokemon, source, effect, item)
    pub fn eat_item_event(&mut self, p: Pk, source: Option<Pk>, eff: Eff, item: ItemId) {
        if !self.any_handler(Ev::EatItem) {
            return;
        }
        let mut ctx = Ctx::new(Some(p), source, eff);
        ctx.arg = item.0;
        self.run_event(Ev::EatItem, &mut ctx, R::True);
    }

    /// pokemon.eatItem(force, source, sourceEffect)
    pub fn eat_item(&mut self, p: Pk, force: bool, source: Option<Pk>, eff: Eff) -> bool {
        let m = *self.mon_at(p);
        if m.item.is_none() || m.hp == 0 || !m.is_active {
            return false;
        }
        let item = m.item;
        // an item telling us to eat it but we are not holding it
        if let Eff::Item(i) = eff {
            if i != item && source == Some(p) {
                return false;
            }
        }
        if !self.use_item_event(p, item) {
            return false;
        }
        if !force && self.any_handler(Ev::TryEatItem) {
            let mut ctx = Ctx::new(Some(p), None, Eff::None);
            ctx.arg = item.0;
            if !self.run_event(Ev::TryEatItem, &mut ctx, R::True).truthy() {
                return false;
            }
        }
        self.row(Row::Item { p: who(p), item, what: ItemWhat::Used, of: None, by: Fx::None });
        self.item_eat(item, p, source, eff);
        self.eat_item_event(p, source, eff, item);
        let m = self.mon_at_mut(p);
        m.last_item = m.item;
        m.item = ItemId::NONE;
        m.item_order = 0;
        m.used_item_this_turn = true;
        m.ate_berry = true;
        self.after_use_item(p, item);
        true
    }

    /// pokemon.useItem(source); `source` is also where the item's boosts come from
    pub fn use_item(&mut self, p: Pk, source: Option<Pk>, eff: Eff) -> bool {
        let m = *self.mon_at(p);
        if (m.hp == 0 && m.item != items::NORMALGEM) || !m.is_active || m.item.is_none() {
            return false;
        }
        let item = m.item;
        if let Eff::Item(i) = eff {
            if i != item && source == Some(p) {
                return false;
            }
        }
        if !self.use_item_event(p, item) {
            return false;
        }
        if self.logging() {
            let of = if item == items::REDCARD { source.map(who) } else { None };
            self.row(Row::Item { p: who(p), item, what: ItemWhat::Used, of, by: Fx::None });
        }
        if let Some((_, stat)) = seed(item) {
            self.boost(&[(stat, 1)], p, source, Eff::Item(item), false, false);
        }
        if item == items::WHITEHERB {
            self.item_use_white_herb(p);
        }
        let m = self.mon_at_mut(p);
        m.last_item = m.item;
        m.item = ItemId::NONE;
        m.item_order = 0;
        m.used_item_this_turn = true;
        self.after_use_item(p, item);
        true
    }

    /// pokemon.takeItem(source)
    pub fn take_item(&mut self, p: Pk, source: Option<Pk>) -> Taken {
        let source = source.unwrap_or(p);
        let item = self.mon_at(p).item;
        if item.is_none() {
            return Taken::NoItem;
        }
        let mut ctx = Ctx::new(Some(p), Some(source), Eff::None);
        ctx.arg = item.0;
        if !self.run_event(Ev::TakeItem, &mut ctx, R::True).truthy() {
            return Taken::Blocked;
        }
        let m = self.mon_at_mut(p);
        m.item = ItemId::NONE;
        m.item_order = 0;
        // singleEvent('End', item): no legal item has an End handler
        if self.any_handler(Ev::AfterTakeItem) {
            let mut ctx = Ctx::new(Some(p), None, Eff::None);
            ctx.arg = item.0;
            self.run_event(Ev::AfterTakeItem, &mut ctx, R::True);
        }
        Taken::Item(item)
    }

    /// pokemon.setItem(item, source, effect)
    pub fn set_item(&mut self, p: Pk, item: ItemId, source: Option<Pk>, eff: Eff) -> bool {
        let m = self.mon_at(p);
        if m.hp == 0 || !m.is_active {
            return false;
        }
        let order = if item.is_none() {
            0
        } else {
            self.s.effect_order = self.s.effect_order.wrapping_add(1);
            self.s.effect_order
        };
        let m = self.mon_at_mut(p);
        m.item = item;
        m.item_order = order;
        self.s.it_union |= item_evmask(item);
        // singleEvent('End', oldItem): nothing; singleEvent('Start', item): not suppressed
        if !item.is_none() {
            self.item_start(p, item, source, eff);
        }
        true
    }

    /// The item is set back without setItem (Trick, Thief, Covet, Magician failing): no Start, no state
    pub fn restore_item(&mut self, p: Pk, item: ItemId) {
        let m = self.mon_at_mut(p);
        m.item = item;
    }

    // ------------------------------------------------------------------------------------------ single events

    /// singleEvent('Start', item): on switch-in (as SwitchIn) and when the item is received
    pub fn item_start(&mut self, p: Pk, item: ItemId, _source: Option<Pk>, _eff: Eff) {
        use items as I;
        match item {
            I::METRONOME => {
                self.add_volatile(p, conds::METRONOME, Some(p), Eff::Item(item), false);
            }
            I::CHOICESCARF => {
                self.remove_volatile(p, conds::CHOICELOCK);
            }
            I::WHITEHERB => self.white_herb(p),
            I::AIRBALLOON => {
                // not shown under Gravity, Klutz or Magic Room
                if !self.ignoring_item(p, false) && !self.gravity() {
                    self.row(Row::Item { p: who(p), item, what: ItemWhat::Found, of: None, by: Fx::None });
                }
            }
            _ => {
                if let Some((t, _)) = seed(item) {
                    if !self.ignoring_item(p, false) && self.terrain() == t {
                        self.use_item(p, Some(p), Eff::Item(item));
                    }
                }
            }
        }
    }

    /// singleEvent('Eat', item, ..., eater, source, effect): a berry's onEat (suppressed while the eater ignores
    /// items)
    pub fn item_eat(&mut self, item: ItemId, eater: Pk, source: Option<Pk>, _eff: Eff) {
        use items as I;
        if self.ignoring_item(eater, false) {
            return;
        }
        let st = self.mon_at(eater).status;
        let by = Fx::Item(item);
        match item {
            I::CHERIBERRY if st == Status::Par => {
                self.cure_status_by(eater, by, true);
            }
            I::CHESTOBERRY if st == Status::Slp => {
                self.cure_status_by(eater, by, true);
            }
            I::PECHABERRY if st == Status::Psn || st == Status::Tox => {
                self.cure_status_by(eater, by, true);
            }
            I::RAWSTBERRY if st == Status::Brn => {
                self.cure_status_by(eater, by, true);
            }
            I::ASPEARBERRY if st == Status::Frz => {
                self.cure_status_by(eater, by, true);
            }
            I::PERSIMBERRY => {
                self.remove_volatile(eater, conds::CONFUSION);
            }
            I::LUMBERRY => {
                self.cure_status_by(eater, by, true);
                self.remove_volatile(eater, conds::CONFUSION);
            }
            I::SITRUSBERRY => {
                let a = self.mon_at(eater).base_maxhp as f64 / 4.0;
                self.heal(a, eater, source, Eff::Item(item));
            }
            I::ORANBERRY => {
                self.heal(10.0, eater, source, Eff::Item(item));
            }
            I::LEPPABERRY => {
                let m = *self.mon_at(eater);
                let n = m.nmoves as usize;
                let i = m.moves[..n].iter().position(|ms| ms.pp == 0).or_else(|| m.moves[..n].iter().position(|ms| ms.pp < ms.max_pp));
                if let Some(i) = i {
                    self.row(Row::Item { p: who(eater), item, what: ItemWhat::Activated, of: None, by: Fx::None });
                    let add = if self.has_ability(eater, abilities::RIPEN) { 20 } else { 10 };
                    let ms = &mut self.mon_at_mut(eater).moves[i];
                    ms.pp = (ms.pp as u16 + add).min(ms.max_pp as u16) as u8;
                }
            }
            _ => {}
        }
    }

    /// White Herb's onStart: negative stages back to 0, the herb used up
    pub fn white_herb(&mut self, p: Pk) {
        if self.mon_at(p).item != items::WHITEHERB {
            return;
        }
        if self.mon_at(p).boosts.iter().any(|b| *b < 0) {
            self.use_item(p, Some(p), Eff::Item(items::WHITEHERB));
        }
    }

    /// White Herb's onUse (singleEvent 'Use'): the recorded negative stages set to 0
    fn item_use_white_herb(&mut self, p: Pk) {
        if self.ignoring_item(p, false) {
            return;
        }
        let m = self.mon_at_mut(p);
        for b in m.boosts.iter_mut() {
            if *b < 0 {
                *b = 0;
            }
        }
    }

    /// Fling's effect on the target (item.fling.effect, or the berry eaten)
    pub fn fling_on_hit(&mut self, t: Pk, user: Pk, item: ItemId, mv: MoveId) {
        use items as I;
        if item.data().is_berry {
            // singleEvent('Eat', item, ..., foe, source, move): a handler returning nothing is truthy
            self.item_eat(item, t, Some(user), Eff::Move(mv));
            self.eat_item_event(t, Some(user), Eff::Move(mv), item);
            self.mon_at_mut(t).ate_berry = true;
            return;
        }
        match item {
            I::WHITEHERB => {
                let m = self.mon_at_mut(t);
                for b in m.boosts.iter_mut() {
                    if *b < 0 {
                        *b = 0;
                    }
                }
            }
            I::MENTALHERB => {
                if MENTAL.iter().any(|&c| self.has_vol(t, c)) {
                    for c in MENTAL {
                        self.remove_volatile(t, c);
                    }
                }
            }
            _ => {}
        }
    }

    /// The item's onResidual (fieldEvent 'Residual', suppressed while the holder ignores items)
    pub fn item_residual(&mut self, p: Pk, item: ItemId) {
        if coverage_on() {
            let before = *self.s;
            self.item_residual_inner(p, item);
            if *self.s != before {
                count_trigger(item);
            }
            return;
        }
        self.item_residual_inner(p, item);
    }

    fn item_residual_inner(&mut self, p: Pk, item: ItemId) {
        if self.mon_at(p).item != item || self.ignoring_item(p, false) {
            return;
        }
        match item {
            items::LEFTOVERS => {
                let a = self.mon_at(p).base_maxhp as f64 / 16.0;
                self.heal(a, p, None, Eff::Item(item));
            }
            items::WHITEHERB => self.white_herb(p),
            _ => {}
        }
    }

    /// The item's switch-in handler (fieldEvent 'SwitchIn': its onStart when it has no onSwitchIn and no
    /// onAnySwitchIn), suppressed while the holder ignores items
    pub fn item_switch_in(&mut self, p: Pk) {
        let item = self.mon_at(p).item;
        if item.is_none() || self.ignoring_item(p, false) {
            return;
        }
        if coverage_on() {
            let before = *self.s;
            self.item_start(p, item, None, Eff::None);
            if *self.s != before {
                count_trigger(item);
            }
            return;
        }
        self.item_start(p, item, None, Eff::None);
    }

    // ------------------------------------------------------------------------------------------ events

    /// One item handler of a runEvent.
    pub fn item_event(&mut self, e: Ev, h: &H, ctx: &mut Ctx, relay: R) -> R {
        use items as I;
        let it = h.item;
        let me = h.pk;
        match e {
            Ev::BasePower => {
                let Some(am) = ctx.am else { return R::Undef };
                if h.pre != Pre::Own {
                    return R::Undef;
                }
                if let Some(t) = type_boost(it) {
                    if am.ty == t {
                        ctx.chain_modify(4915, 4096);
                    }
                } else if (it == I::MUSCLEBAND && am.category == Category::Physical)
                    || (it == I::WISEGLASSES && am.category == Category::Special)
                {
                    ctx.chain_modify(4505, 4096);
                }
                R::Undef
            }
            Ev::ModifyAtk | Ev::ModifySpA => {
                if it == I::LIGHTBALL && h.pre == Pre::Own {
                    let bs = self.mon_at(me).base_species.data().base_species;
                    if bs.id() == "pikachu" {
                        ctx.chain_modify(2, 1);
                    }
                }
                R::Undef
            }
            Ev::ModifySpe => {
                if it == I::CHOICESCARF {
                    ctx.chain_modify(3, 2);
                } else if it == I::IRONBALL {
                    ctx.chain_modify(1, 2);
                }
                R::Undef
            }
            Ev::ModifyDamage => self.item_modify_damage(it, h.pre, ctx),
            Ev::ModifyAccuracy => {
                let R::Num(_) = relay else { return R::Undef };
                match (it, h.pre) {
                    (I::BRIGHTPOWDER, Pre::Own) => ctx.chain_modify(3686, 4096),
                    (I::WIDELENS, Pre::Source) => ctx.chain_modify(4505, 4096),
                    (I::ZOOMLENS, Pre::Source) => {
                        if let Some(t) = ctx.target {
                            if self.will_move(t).is_none() {
                                ctx.chain_modify(4915, 4096);
                            }
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::ModifyCritRatio => {
                let R::Num(cr) = relay else { return R::Undef };
                match it {
                    I::SCOPELENS => R::Num(cr + 1),
                    I::LEEK => {
                        let id = self.mon_at(me).base_species.data().base_species.id();
                        if id == "farfetchd" || id == "sirfetchd" {
                            R::Num(cr + 2)
                        } else {
                            R::Undef
                        }
                    }
                    _ => R::Undef,
                }
            }
            Ev::ModifyMove => {
                match it {
                    I::CHOICESCARF => {
                        self.add_volatile(me, conds::CHOICELOCK, Some(me), Eff::Item(it), false);
                    }
                    I::KINGSROCK => {
                        if let Some(am) = ctx.am.as_mut() {
                            if am.category != Category::Status && !am.has_flinch_secondary() {
                                am.push_secondary(super::moves::ExtraSec { chance: Some(10), status: Status::None, vol: conds::FLINCH });
                            }
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::FractionalPriority => {
                if it != I::QUICKCLAW {
                    return R::Undef;
                }
                let R::Num(pr) = relay else { return R::Undef };
                let Some(am) = ctx.am else { return R::Undef };
                if am.category == Category::Status && self.has_ability(me, abilities::MYCELIUMMIGHT) {
                    return R::Undef;
                }
                if pr <= 0 {
                    let site = Site { effect: Effect::One(EffectRef::Item(it)), ..Site::new(Key::Proc, Some(me.0 as u8)) };
                    if self.chance(site, 1, 5) {
                        self.row(Row::Item { p: who(me), item: it, what: ItemWhat::Activated, of: None, by: Fx::None });
                        return R::Num(1);
                    }
                }
                R::Undef
            }
            Ev::Effectiveness => {
                if it != I::IRONBALL {
                    return R::Undef;
                }
                let (Some(t), Some(am)) = (ctx.target, ctx.am) else { return R::Undef };
                if self.has_vol(t, conds::INGRAIN) || self.has_vol(t, conds::SMACKDOWN) || self.gravity() {
                    return R::Undef;
                }
                if am.ty == Type::Ground && self.has_type_pk(t, Type::Flying) {
                    return R::Num(0);
                }
                R::Undef
            }
            Ev::Damage => {
                let Some(t) = ctx.target else { return R::Undef };
                let R::Num(d) = relay else { return R::Undef };
                match it {
                    I::FOCUSSASH => {
                        let m = *self.mon_at(t);
                        if m.hp == m.maxhp && d >= m.hp as i32 && ctx.eff.is_move() && self.use_item(t, Some(t), Eff::Item(it)) {
                            return R::Num(m.hp as i32 - 1);
                        }
                        R::Undef
                    }
                    I::FOCUSBAND => {
                        let site = Site { effect: Effect::One(EffectRef::Item(it)), ..Site::new(Key::Proc, Some(t.0 as u8)) };
                        if self.chance(site, 1, 10) && d >= self.mon_at(t).hp as i32 && ctx.eff.is_move() {
                            self.row(Row::Item { p: who(t), item: it, what: ItemWhat::Activated, of: None, by: Fx::None });
                            return R::Num(self.mon_at(t).hp as i32 - 1);
                        }
                        R::Undef
                    }
                    _ => R::Undef,
                }
            }
            Ev::TryHeal => {
                if it == I::BIGROOT {
                    let big = matches!(ctx.eff, Eff::Drain)
                        || ctx.eff == Eff::Cond(conds::LEECHSEED)
                        || ctx.eff == Eff::Cond(conds::INGRAIN)
                        || ctx.eff == Eff::Cond(conds::AQUARING)
                        || ctx.eff == Eff::Move(moves::STRENGTHSAP);
                    if big {
                        ctx.chain_modify(5324, 4096);
                    }
                }
                R::Undef
            }
            Ev::DamagingHit => {
                let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                match it {
                    I::ROCKYHELMET => {
                        if self.makes_contact(&am) {
                            let a = self.mon_at(src).base_maxhp as f64 / 6.0;
                            self.damage(a, src, Some(t), Eff::Item(it));
                        }
                    }
                    I::AIRBALLOON => self.pop_balloon(t),
                    _ => {}
                }
                R::Undef
            }
            Ev::AfterSubDamage => {
                if it == I::AIRBALLOON && ctx.eff.is_move() {
                    if let Some(t) = ctx.target {
                        self.pop_balloon(t);
                    }
                }
                R::Undef
            }
            Ev::AfterMoveSecondary => {
                // onAfterMoveSecondary(target, source, move): the holder is the move's target
                let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                match it {
                    I::EJECTBUTTON => {
                        if src == t || self.mon_at(t).hp == 0 || am.category == Category::Status || am.has_flag(flag::FUTUREMOVE) {
                            return R::Undef;
                        }
                        let m = *self.mon_at(t);
                        if !self.can_switch(t.0) || m.force_switch_flag || m.being_called_back {
                            return R::Undef;
                        }
                        for side in 0..SIDES {
                            let sd = &self.s.sides[side];
                            if sd.has_active && !sd.active_mon().fainted {
                                let x = sd.active_mon();
                                if x.switch_flag && x.switch_flag_move.is_none() {
                                    return R::Undef;
                                }
                            }
                        }
                        // Champions: the attacker's own pending switch stays
                        {
                            let mm = self.mon_at_mut(t);
                            mm.switch_flag = true;
                            mm.switch_flag_move = MoveId::NONE;
                        }
                        if !self.use_item(t, Some(t), Eff::Item(it)) {
                            self.mon_at_mut(t).switch_flag = false;
                        }
                    }
                    I::REDCARD => {
                        if src == t || self.mon_at(src).hp == 0 || self.mon_at(t).hp == 0 || am.category == Category::Status {
                            return R::Undef;
                        }
                        if !self.is_active_pk(src) || !self.can_switch(src.0) || self.mon_at(src).force_switch_flag
                            || self.mon_at(t).force_switch_flag
                        {
                            return R::Undef;
                        }
                        if self.use_item(t, Some(src), Eff::Item(it)) {
                            let mut c2 = Ctx::with_move(Some(src), Some(t), &am);
                            if self.run_event(Ev::DragOut, &mut c2, R::True).truthy() {
                                self.mon_at_mut(src).force_switch_flag = true;
                            }
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::AfterMoveSecondarySelf => {
                // onAfterMoveSecondarySelf(source, target, move): the holder is the user
                let (Some(user), Some(am)) = (ctx.target, ctx.am) else { return R::Undef };
                let t = ctx.source;
                match it {
                    I::LIFEORB => {
                        if t != Some(user) && am.category != Category::Status && !self.mon_at(user).force_switch_flag {
                            let a = self.mon_at(user).base_maxhp as f64 / 10.0;
                            self.damage(a, user, Some(user), Eff::Item(it));
                        }
                    }
                    I::SHELLBELL => {
                        if am.total_damage > 0 && !self.mon_at(user).force_switch_flag {
                            self.heal(am.total_damage as f64 / 8.0, user, t, Eff::Item(it));
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::TryPrimaryHit => {
                // Normal Gem: onSourceTryPrimaryHit(target, source, move)
                if it != I::NORMALGEM || h.pre != Pre::Source {
                    return R::Undef;
                }
                let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                if t == src || am.category == Category::Status {
                    return R::Undef;
                }
                if am.ty == Type::Normal && self.use_item(src, Some(t), Eff::Item(it)) {
                    self.add_volatile(src, conds::GEM, Some(src), Eff::Item(it), false);
                }
                R::Undef
            }
            Ev::Update => {
                self.item_update(it, me);
                R::Undef
            }
            Ev::AfterSetStatus => {
                if it == I::LUMBERRY {
                    if let Some(t) = ctx.target {
                        self.eat_item(t, false, Some(t), Eff::Item(it));
                    }
                }
                R::Undef
            }
            Ev::TerrainChange => {
                if let Some((t, _)) = seed(it) {
                    if self.terrain() == t {
                        let p = ctx.target.unwrap_or(me);
                        self.use_item(p, Some(p), Eff::Item(it));
                    }
                }
                R::Undef
            }
            Ev::AfterMega | Ev::AfterMove => {
                if it == I::WHITEHERB && h.pre == Pre::Any {
                    self.white_herb(me);
                }
                R::Undef
            }
            Ev::TrapPokemon => {
                if it == I::SHEDSHELL {
                    if let Some(t) = ctx.target {
                        let m = self.mon_at_mut(t);
                        m.trapped = false;
                        m.trap_hidden = false;
                    }
                }
                R::Undef
            }
            Ev::TakeItem => {
                if !it.data().mega_stone.is_empty() {
                    let Some(t) = ctx.target else { return R::Undef };
                    return R::of_bool(!self.stone_blocks(it, t));
                }
                R::Undef
            }
            Ev::TryEatItem => {
                if it == I::SITRUSBERRY || it == I::ORANBERRY {
                    let Some(t) = ctx.target else { return R::Undef };
                    let amount = if it == I::SITRUSBERRY { self.mon_at(t).base_maxhp as f64 / 4.0 } else { 10.0 };
                    if !self.try_heal_event(t, amount, Eff::Item(it)) {
                        return R::False;
                    }
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    /// runEvent('TryHeal', pokemon, null, effect, amount) as a test (Sitrus and Oran Berry)
    fn try_heal_event(&mut self, p: Pk, amount: f64, eff: Eff) -> bool {
        if !self.any_handler(Ev::TryHeal) {
            return amount != 0.0;
        }
        let mut ctx = Ctx::new(Some(p), None, eff);
        // a fractional amount stays truthy; only its truthiness is read
        let a = (amount.ceil() as i32).max(1);
        self.run_event(Ev::TryHeal, &mut ctx, R::Num(a)).truthy()
    }

    fn item_modify_damage(&mut self, it: ItemId, pre: Pre, ctx: &mut Ctx) -> R {
        use items as I;
        let (Some(_att), Some(def), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
        match pre {
            Pre::Own => match it {
                I::LIFEORB => ctx.chain_modify(5324, 4096),
                I::EXPERTBELT if am.type_mod > 0 => ctx.chain_modify(4915, 4096),
                _ => {}
            },
            Pre::Source => {
                // a resist berry of the target: onSourceModifyDamage(damage, source, target, move)
                let Some(ty) = resist_type(it) else { return R::Undef };
                if am.ty != ty || (it != I::CHILANBERRY && am.type_mod <= 0) {
                    return R::Undef;
                }
                let hit_sub = self.has_vol(def, conds::SUBSTITUTE) && !am.has_flag(flag::BYPASSSUB) && !am.infiltrates;
                if hit_sub {
                    return R::Undef;
                }
                if self.eat_item(def, false, ctx.target, Eff::Item(it)) {
                    self.row(Row::Item { p: who(def), item: it, what: ItemWhat::Activated, of: None, by: Fx::None });
                    ctx.chain_modify(1, 2);
                }
            }
            _ => {}
        }
        R::Undef
    }

    /// The item's onUpdate (berries, Mental Herb)
    fn item_update(&mut self, it: ItemId, p: Pk) {
        use items as I;
        let m = *self.mon_at(p);
        if m.item != it {
            return;
        }
        let eat = match it {
            I::CHERIBERRY => m.status == Status::Par,
            I::CHESTOBERRY => m.status == Status::Slp,
            I::PECHABERRY => m.status == Status::Psn || m.status == Status::Tox,
            I::RAWSTBERRY => m.status == Status::Brn,
            I::ASPEARBERRY => m.status == Status::Frz,
            I::PERSIMBERRY => self.has_vol(p, conds::CONFUSION),
            I::LUMBERRY => m.status != Status::None || self.has_vol(p, conds::CONFUSION),
            I::SITRUSBERRY | I::ORANBERRY => (m.hp as f64) <= m.maxhp as f64 / 2.0,
            I::LEPPABERRY => m.hp > 0 && m.moves().iter().any(|ms| ms.pp == 0),
            I::MENTALHERB => {
                if MENTAL.iter().any(|&c| self.has_vol(p, c)) && self.use_item(p, Some(p), Eff::Item(it)) {
                    for c in MENTAL {
                        let had = self.has_vol(p, c);
                        self.remove_volatile(p, c);
                        if had && c == conds::ATTRACT {
                            self.row(Row::VolatileEnd { p: who(p), c, by: Fx::Item(it) });
                        }
                    }
                }
                false
            }
            _ => false,
        };
        if eat {
            self.eat_item(p, false, Some(p), Eff::Item(it));
        }
    }

    /// Air Balloon pops: the item goes without useItem (no lastItem), then AfterUseItem
    fn pop_balloon(&mut self, t: Pk) {
        let m = self.mon_at_mut(t);
        if m.item != items::AIRBALLOON {
            return;
        }
        m.item = ItemId::NONE;
        m.item_order = 0;
        self.row(Row::Item { p: who(t), item: items::AIRBALLOON, what: ItemWhat::Removed, of: None, by: Fx::None });
        self.after_use_item(t, items::AIRBALLOON);
    }

    // ------------------------------------------------------------------------------------------ item conditions

    /// One handler of choicelock, metronome, gem or fling in a runEvent.
    pub fn item_cond_event(&mut self, e: Ev, h: &H, ctx: &mut Ctx, _relay: R) -> R {
        let p = h.pk;
        let c = h.cond;
        match (c, e) {
            (c, Ev::BeforeMove) if c == conds::CHOICELOCK => {
                let Some(am) = ctx.am else { return R::Undef };
                if !self.mon_at(p).item.data().is_choice {
                    self.remove_volatile(p, c);
                    return R::Undef;
                }
                if !self.ignoring_item(p, false) && am.id != h.state.mv && am.id != moves::STRUGGLE {
                    self.row(Row::Move { p: who(p), mv: am.id, target: None, called_by: Fx::None, priority: am.priority });
                    self.row(Row::Fail { p: Some(who(p)), why: Fx::None });
                    return R::False;
                }
                R::Undef
            }
            (c, Ev::DisableMove) if c == conds::CHOICELOCK => {
                let m = *self.mon_at(p);
                let mv = h.state.mv;
                if !m.item.data().is_choice || !m.moves().iter().any(|ms| ms.id == mv) {
                    self.remove_volatile(p, c);
                    return R::Undef;
                }
                if self.ignoring_item(p, false) {
                    return R::Undef;
                }
                for ms in m.moves() {
                    if ms.id != mv {
                        self.disable_move(p, ms.id);
                    }
                }
                R::Undef
            }
            (c, Ev::TryMove) if c == conds::METRONOME => {
                let Some(am) = ctx.am else { return R::Undef };
                if !self.has_item(p, items::METRONOME) {
                    self.remove_volatile(p, c);
                    return R::Undef;
                }
                if am.data().calls_move {
                    return R::Undef;
                }
                let last_ok = self.mon_at(p).move_last_turn_result == MoveResult::True;
                let two = self.s.sides[p.0].has_vol(conds::TWOTURNMOVE);
                if let Some(v) = self.s.sides[p.0].vol_mut(c) {
                    if v.mv == am.id && last_ok {
                        v.data = v.data.saturating_add(1);
                    } else if two {
                        if v.mv != am.id {
                            v.data = 1;
                        } else {
                            v.data = v.data.saturating_add(1);
                        }
                    } else {
                        v.data = 0;
                    }
                    v.mv = am.id;
                }
                R::Undef
            }
            (c, Ev::ModifyDamage) if c == conds::METRONOME && h.pre == Pre::Own => {
                let n = self.s.sides[p.0].vol(c).map_or(h.state.data, |v| v.data).clamp(0, 5) as usize;
                ctx.chain_modify([4096, 4915, 5734, 6553, 7372, 8192][n], 4096);
                R::Undef
            }
            (c, Ev::BasePower) if c == conds::GEM && h.pre == Pre::Own => {
                ctx.chain_modify(5325, 4096);
                R::Undef
            }
            (c, Ev::Update) if c == conds::FLING => {
                let item = self.mon_at(p).item;
                self.row(Row::Item { p: who(p), item, what: ItemWhat::Removed, of: None, by: Fx::Move(moves::FLING) });
                self.set_item(p, ItemId::NONE, None, Eff::None);
                let m = self.mon_at_mut(p);
                m.last_item = item;
                m.used_item_this_turn = true;
                self.after_use_item(p, item);
                self.remove_volatile(p, c);
                R::Undef
            }
            _ => R::Undef,
        }
    }

    /// The Start handler of an item condition
    pub fn item_cond_start(&mut self, p: Pk, c: Cond) -> R {
        if c == conds::CHOICELOCK {
            let Some(am) = self.active else { return R::False };
            if am.id.is_none() || am.has_bounced {
                return R::False;
            }
            self.s.sides[p.0].vol_mut(c).unwrap().mv = am.id;
        } else if c == conds::METRONOME {
            let v = self.s.sides[p.0].vol_mut(c).unwrap();
            v.mv = MoveId::NONE;
            v.data = 0;
        }
        R::Undef
    }

    // ------------------------------------------------------------------------------------------ read by others

    /// The weather's durationCallback: 8 turns with the setter's rock
    pub fn weather_duration(&self, w: Cond, source: Option<Pk>) -> i8 {
        let base = w.data().duration as i8;
        let Some(src) = source else { return base };
        let rock = match w {
            c if c == conds::RAINDANCE => items::DAMPROCK,
            c if c == conds::SUNNYDAY => items::HEATROCK,
            c if c == conds::SANDSTORM => items::SMOOTHROCK,
            c if c == conds::SNOWSCAPE || c == conds::HAIL => items::ICYROCK,
            _ => return base,
        };
        if base > 0 && self.has_item(src, rock) {
            8
        } else {
            base
        }
    }

    /// A terrain's durationCallback: 8 turns with the setter's Terrain Extender
    pub fn terrain_duration(&self, t: Cond, source: Option<Pk>) -> i8 {
        let base = t.data().duration as i8;
        match source {
            Some(src) if base > 0 && self.has_item(src, items::TERRAINEXTENDER) => 8,
            _ => base,
        }
    }

    /// A screen's durationCallback: 8 turns with the setter's Light Clay
    pub fn screen_duration(&self, c: Cond, source: Pk) -> i8 {
        let base = c.data().duration as i8;
        if (c == conds::REFLECT || c == conds::LIGHTSCREEN || c == conds::AURORAVEIL) && self.has_item(source, items::LIGHTCLAY) {
            8
        } else {
            base
        }
    }
}

#[inline]
fn coverage_on() -> bool {
    super::abilities::coverage_on()
}
