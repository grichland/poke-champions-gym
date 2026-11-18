//! Weather, terrain, pseudo-weathers (Trick Room, Gravity, Magic Room, Wonder Room, Fairy Lock), side
//! conditions (screens, Tailwind, Safeguard, Quick Guard, Wide Guard, hazards) and slot conditions (Wish,
//! Healing Wish, Future Sight, Revival Blessing): setting and ending them, their event handlers, the hazards
//! of a switch-in and their end-of-turn handlers (sim/field.ts, sim/side.ts, data/conditions.ts and the
//! conditions of data/moves.ts). Messages are left out.

use super::battle::Sim;
use super::data::*;
use super::events::*;
use super::moves::ActiveMove;
use super::outcome::*;
use super::state::*;
use super::status::Imm;
use super::log::*;

/// The hazards Rapid Spin, Mortal Spin, Defog and Tidy Up remove.
pub const HAZARDS: [Cond; 4] = [conds::SPIKES, conds::TOXICSPIKES, conds::STEALTHROCK, conds::STICKYWEB];

/// The side conditions Court Change swaps (the legal ones of its list).
const COURT: [Cond; 9] = [
    conds::LIGHTSCREEN,
    conds::REFLECT,
    conds::SPIKES,
    conds::SAFEGUARD,
    conds::TAILWIND,
    conds::TOXICSPIKES,
    conds::STEALTHROCK,
    conds::STICKYWEB,
    conds::AURORAVEIL,
];

/// Showdown's getOverflowedTurnCount (gen 8+)
#[inline]
fn overflowed(turn: u16) -> u16 {
    turn.wrapping_sub(1) % 255
}

impl<'a, O: Outcomes> Sim<'a, O> {
    // ------------------------------------------------------------------------------------------ queries

    #[inline]
    pub fn has_pseudo(&self, c: Cond) -> bool {
        let f = &self.s.field;
        f.pseudo[..f.npseudo as usize].iter().any(|x| x.id == c)
    }

    #[inline]
    pub fn gravity(&self) -> bool {
        self.s.field.npseudo != 0 && self.has_pseudo(conds::GRAVITY)
    }

    /// field.effectiveWeather: none while Air Lock or Cloud Nine is in play
    #[inline]
    pub fn weather(&self) -> Cond {
        let w = self.s.field.weather.id;
        if w.is_none() || !self.suppressing_weather() {
            w
        } else {
            Cond::NONE
        }
    }

    /// pokemon.effectiveWeather outside a move (no Utility Umbrella: items are not in play)
    #[inline]
    pub fn eff_weather(&self, _p: Pk) -> Cond {
        self.weather()
    }

    /// pokemon.effectiveWeather from a move's handler: sun when the Pokémon using the move has Mega Sol
    pub fn eff_weather_move(&self, p: Pk) -> Cond {
        if let Some(ap) = pk_of(self.s.active_pokemon) {
            if self.s.active_move != MoveId::NONE && self.has_ability(ap, abilities::MEGASOL) {
                return conds::SUNNYDAY;
            }
        }
        self.eff_weather(p)
    }

    /// field.effectiveTerrain: no TryTerrain handler can be in play
    #[inline]
    pub fn terrain(&self) -> Cond {
        self.s.field.terrain.id
    }

    pub fn side_cond(&self, side: usize, c: Cond) -> Option<SideCond> {
        self.s.sides[side].conds().iter().find(|x| x.id == c).copied()
    }

    pub fn slot_cond(&self, side: usize, c: Cond) -> Option<SideCond> {
        self.s.sides[side].slot_conds().iter().find(|x| x.id == c).copied()
    }

    /// eachEvent('WeatherChange') or eachEvent('TerrainChange'): the speed sort (a tie draws), then the
    /// event on each Pokémon
    pub fn each_change(&mut self, e: Ev) {
        let (order, n) = self.speed_order();
        for &side in &order[..n] {
            super::hooks::run_field_hook(self.s, super::hooks::Hook::Residual);
            let sd = &self.s.sides[side];
            if sd.has_active && self.any_handler(e) {
                let p = (side, sd.active as usize);
                let mut ctx = Ctx::new(Some(p), None, Eff::None);
                self.run_event(e, &mut ctx, R::True);
            }
        }
    }

    // ------------------------------------------------------------------------------------------ field

    /// field.setWeather from a move
    pub fn set_weather(&mut self, w: Cond, source: Pk, eff: Eff) -> R {
        if self.s.field.weather.id == w {
            return R::False;
        }
        // durationCallback: 8 turns with the setter's weather rock
        let duration = self.weather_duration(w, Some(source));
        self.s.field.weather = FieldCond { id: w, duration, targeted: false };
        if self.logging() {
            let (by, of) = self.field_by(eff, source);
            self.row(Row::Weather { w, by, of });
        }
        self.each_change(Ev::WeatherChange);
        R::True
    }

    /// field.setWeather from an ability
    pub fn set_weather_by(&mut self, w: Cond, source: Pk, eff: Eff) -> R {
        self.set_weather(w, source, eff)
    }

    /// field.clearWeather
    pub fn clear_weather(&mut self) -> bool {
        if self.s.field.weather.id.is_none() {
            return false;
        }
        self.s.field.weather = FieldCond::default();
        self.row(Row::Weather { w: Cond::NONE, by: Fx::None, of: None });
        self.each_change(Ev::WeatherChange);
        true
    }

    /// field.setTerrain from a move
    pub fn set_terrain(&mut self, t: Cond, source: Pk, eff: Eff) -> R {
        if self.s.field.terrain.id == t {
            return R::False;
        }
        // durationCallback: 8 turns with the setter's Terrain Extender
        let duration = self.terrain_duration(t, Some(source));
        self.s.field.terrain = FieldCond { id: t, duration, targeted: false };
        if self.logging() {
            let (by, of) = self.field_by(eff, source);
            self.row(Row::Field { c: t, start: true, by, of });
        }
        self.each_change(Ev::TerrainChange);
        R::True
    }

    /// field.setTerrain from an ability
    pub fn set_terrain_by(&mut self, t: Cond, source: Pk, eff: Eff) -> R {
        self.set_terrain(t, source, eff)
    }

    /// field.clearTerrain
    pub fn clear_terrain(&mut self) -> bool {
        if self.s.field.terrain.id.is_none() {
            return false;
        }
        let old = self.s.field.terrain.id;
        self.s.field.terrain = FieldCond::default();
        self.row(Row::Field { c: old, start: false, by: Fx::None, of: None });
        self.each_change(Ev::TerrainChange);
        true
    }

    /// field.addPseudoWeather
    pub fn add_pseudo(&mut self, c: Cond, source: Pk, _eff: Eff) -> R {
        if self.has_pseudo(c) {
            // onFieldRestart: the rooms end; Gravity and Fairy Lock have none and fail
            if c == conds::TRICKROOM || c == conds::MAGICROOM || c == conds::WONDERROOM {
                self.remove_pseudo(c);
                return R::True;
            }
            return R::False;
        }
        let f = &mut self.s.field;
        if f.npseudo as usize >= MAX_PSEUDO {
            self.s.unimpl.note(EffectRef::Cond(c), super::hooks::Hook::Start);
            return R::False;
        }
        f.pseudo[f.npseudo as usize] = FieldCond { id: c, duration: c.data().duration as i8, targeted: false };
        f.npseudo += 1;
        if self.logging() {
            // the rooms name their user
            let of = if c == conds::GRAVITY || c == conds::FAIRYLOCK { None } else { Some(who(source)) };
            self.row(Row::Field { c, start: true, by: Fx::None, of });
        }
        if c == conds::GRAVITY {
            self.gravity_start();
        }
        R::True
    }

    /// Gravity's onFieldStart: Pokémon in the air come down
    fn gravity_start(&mut self) {
        for side in 0..SIDES {
            let sd = &self.s.sides[side];
            if !sd.has_active || sd.active_mon().fainted {
                continue;
            }
            let p = (side, sd.active as usize);
            let mut applies = false;
            if self.remove_volatile(p, conds::BOUNCE) || self.remove_volatile(p, conds::FLY) {
                applies = true;
                self.cancel_move(p);
                self.remove_volatile(p, conds::TWOTURNMOVE);
            }
            applies |= self.s.sides[side].delete_vol(conds::MAGNETRISE);
            if applies {
                self.row(Row::Once { p: Some(who(p)), fx: Fx::Cond(conds::GRAVITY), of: None });
            }
        }
    }

    /// field.removePseudoWeather
    pub fn remove_pseudo(&mut self, c: Cond) -> bool {
        let f = &mut self.s.field;
        let n = f.npseudo as usize;
        let Some(i) = f.pseudo[..n].iter().position(|x| x.id == c) else { return false };
        f.pseudo.copy_within(i + 1..n, i);
        f.pseudo[n - 1] = FieldCond::default();
        f.npseudo -= 1;
        if c != conds::FAIRYLOCK {
            self.row(Row::Field { c, start: false, by: Fx::None, of: None });
        }
        true
    }

    // ------------------------------------------------------------------------------------------ sides

    /// side.addSideCondition
    pub fn add_side_cond(&mut self, side: usize, c: Cond, source: Pk, _eff: Eff) -> R {
        let sd = &mut self.s.sides[side];
        if let Some(x) = sd.conds[..sd.nconds as usize].iter_mut().find(|x| x.id == c) {
            // onSideRestart: Spikes up to three layers, Toxic Spikes two; the others fail
            let max = if c == conds::SPIKES {
                3
            } else if c == conds::TOXICSPIKES {
                2
            } else {
                return R::False;
            };
            if x.layers >= max {
                return R::False;
            }
            x.layers += 1;
            self.row(Row::Side { side: side as u8, c, what: SideWhat::Start, p: None, by: Fx::None });
            return R::True;
        }
        if sd.nconds as usize >= MAX_SIDE_CONDS {
            self.s.unimpl.note(EffectRef::Cond(c), super::hooks::Hook::Start);
            return R::False;
        }
        self.s.effect_order = self.s.effect_order.wrapping_add(1);
        let order = self.s.effect_order;
        // durationCallback: 8 turns for a screen with the setter's Light Clay
        let duration = self.screen_duration(c, source);
        let sd = &mut self.s.sides[side];
        sd.conds[sd.nconds as usize] = SideCond {
            id: c,
            layers: 1,
            duration,
            order,
            data: 0,
            source: pk_code(source),
            turn: 0,
        };
        sd.nconds += 1;
        if c == conds::QUICKGUARD || c == conds::WIDEGUARD {
            self.row(Row::Once { p: Some(who(source)), fx: Fx::Cond(c), of: None });
        } else {
            self.row(Row::Side { side: side as u8, c, what: SideWhat::Start, p: None, by: Fx::None });
        }
        // runEvent('SideConditionStart', side, source, condition): Wind Power, Wind Rider
        if c == conds::TAILWIND && self.s.sides[side].has_active {
            let p = (side, self.s.sides[side].active as usize);
            if self.mon_at(p).is_active && !self.mon_at(p).fainted {
                if self.has_ability(p, abilities::WINDPOWER) {
                    self.add_volatile(p, conds::CHARGE, Some(source), Eff::Ability(abilities::WINDPOWER), false);
                } else if self.has_ability(p, abilities::WINDRIDER) && !self.suppressing_ability(p) {
                    self.boost(&[(0, 1)], p, Some(p), Eff::Ability(abilities::WINDRIDER), false, false);
                }
            }
        }
        R::True
    }

    /// side.removeSideCondition
    pub fn remove_side_cond(&mut self, side: usize, c: Cond) -> bool {
        let sd = &mut self.s.sides[side];
        let n = sd.nconds as usize;
        let Some(i) = sd.conds[..n].iter().position(|x| x.id == c) else { return false };
        sd.conds.copy_within(i + 1..n, i);
        sd.conds[n - 1] = SideCond::default();
        sd.nconds -= 1;
        // the conditions whose End handler writes a row; a hazard's removal is written by its remover
        if [conds::REFLECT, conds::LIGHTSCREEN, conds::AURORAVEIL, conds::SAFEGUARD, conds::TAILWIND].contains(&c) {
            self.row(Row::Side { side: side as u8, c, what: SideWhat::End, p: None, by: Fx::None });
        }
        true
    }

    /// A hazard removed by a move (Rapid Spin, Defog, Tidy Up), with its row
    pub fn remove_hazard(&mut self, side: usize, c: Cond, by: Fx) -> bool {
        if !self.remove_side_cond(side, c) {
            return false;
        }
        self.row(Row::Side { side: side as u8, c, what: SideWhat::End, p: None, by });
        true
    }

    /// side.addSlotCondition for the active position
    pub fn add_slot_cond(&mut self, side: usize, c: Cond, source: Pk, _eff: Eff) -> R {
        let sd = &self.s.sides[side];
        // onRestart of Wish and Future Sight returns false; the others have none
        if sd.slot_conds().iter().any(|x| x.id == c) {
            return R::False;
        }
        if sd.nslot_conds as usize >= MAX_SLOT_CONDS {
            self.s.unimpl.note(EffectRef::Cond(c), super::hooks::Hook::Start);
            return R::False;
        }
        let turn = self.s.turn;
        let mut x = SideCond { id: c, layers: 0, duration: c.data().duration as i8, order: 0, data: 0, source: pk_code(source), turn: 0 };
        if c == conds::WISH {
            // the heal truncates source.maxhp / 2 (at least 1)
            x.data = (self.mon_at(source).maxhp / 2).max(1) as i16;
            x.turn = overflowed(turn);
        } else if c == conds::FUTUREMOVE {
            x.turn = turn.wrapping_sub(1).wrapping_add(2);
            x.data = moves::FUTURESIGHT.0 as i16;
        }
        let sd = &mut self.s.sides[side];
        sd.slot_conds[sd.nslot_conds as usize] = x;
        sd.nslot_conds += 1;
        R::True
    }

    /// side.removeSlotCondition: the End handler, then the removal
    pub fn remove_slot_cond(&mut self, side: usize, c: Cond) -> bool {
        let Some(x) = self.slot_cond(side, c) else { return false };
        if c == conds::WISH {
            self.wish_end(side, &x);
        } else if c == conds::FUTUREMOVE {
            self.future_end(side, &x);
        }
        let sd = &mut self.s.sides[side];
        let n = sd.nslot_conds as usize;
        if let Some(i) = sd.slot_conds[..n].iter().position(|y| y.id == c) {
            sd.slot_conds.copy_within(i + 1..n, i);
            sd.slot_conds[n - 1] = SideCond::default();
            sd.nslot_conds -= 1;
        }
        true
    }

    /// Wish's onEnd: the Pokémon in the slot is healed
    fn wish_end(&mut self, side: usize, x: &SideCond) {
        let sd = &self.s.sides[side];
        if !sd.has_active {
            return;
        }
        let p = (side, sd.active as usize);
        if self.mon_at(p).fainted {
            return;
        }
        self.heal(x.data as f64, p, Some(p), Eff::Cond(conds::WISH));
    }

    /// futuremove's onEnd: the attack hits whoever is in the slot, with the user's stats as they are now
    fn future_end(&mut self, side: usize, x: &SideCond) {
        let sd = &self.s.sides[side];
        if !sd.has_active {
            return;
        }
        let t = (side, sd.active as usize);
        let Some(src) = pk_of(x.source) else { return };
        if self.mon_at(t).fainted || t == src {
            return;
        }
        self.row(Row::Once { p: Some(who(t)), fx: Fx::Move(MoveId(x.data as u16)), of: None });
        // the hit is a new move from the stored move data: Future Sight's numbers, no handlers
        let mut am = self.new_active(MoveId(x.data as u16));
        am.future = true;
        am.ignore_immunity = false;
        // the stored move data: Infiltrator and Normalize of the user, if it is still in
        am.ignore_ability = false;
        if self.has_ability(src, abilities::INFILTRATOR) {
            am.infiltrates = true;
        }
        if self.has_ability(src, abilities::NORMALIZE) {
            am.ty = Type::Normal;
        }
        // trySpreadMoveHit(notActive): setActiveMove, then the hit steps
        self.set_active(&am, src, Some(t));
        self.try_spread_move_hit(src, t, &mut am);
        // the user's Life Orb, called by the condition itself (singleEvent with the item as the move)
        if self.is_active_pk(src) && self.has_item(src, items::LIFEORB) && src != t && !self.mon_at(src).force_switch_flag {
            let a = self.mon_at(src).base_maxhp as f64 / 10.0;
            self.damage(a, src, Some(src), Eff::Item(items::LIFEORB));
        }
        // activeMove is cleared; activePokemon stays
        self.active = None;
        self.s.active_move = MoveId::NONE;
        self.check_win(None);
    }

    // ------------------------------------------------------------------------------------------ events

    /// One handler of a side or field condition in a runEvent.
    pub fn field_cond_event(&mut self, ev: Ev, h: &H, ctx: &mut Ctx, relay: R) -> R {
        let c = h.cond;
        match ev {
            Ev::ModifyDamage => {
                // onAnyModifyDamage(damage, source, target, move): the event's target is the attacker
                let (Some(att), Some(def), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                let Holder::Side(hs, _) = h.holder else { return R::Undef };
                if def == att || def.0 != hs as usize {
                    return R::Undef;
                }
                let physical = am.category == Category::Physical;
                let special = am.category == Category::Special;
                let applies = if c == conds::REFLECT {
                    physical
                } else if c == conds::LIGHTSCREEN {
                    special
                } else if c == conds::AURORAVEIL {
                    let ds = def.0;
                    let screened = (physical && self.side_cond(ds, conds::REFLECT).is_some())
                        || (special && self.side_cond(ds, conds::LIGHTSCREEN).is_some());
                    !screened
                } else {
                    false
                };
                if applies && !am.crit && !am.infiltrates {
                    ctx.chain_modify(1, 2);
                }
                R::Undef
            }
            Ev::BasePower => {
                // onBasePower(basePower, attacker, defender, move)
                let (Some(att), Some(def), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                if c == conds::ELECTRICTERRAIN {
                    if am.ty == Type::Electric && self.grounded(att) && !self.semi_invulnerable(att) {
                        ctx.chain_modify(5325, 4096);
                    }
                } else if c == conds::GRASSYTERRAIN {
                    if [moves::EARTHQUAKE, moves::BULLDOZE].contains(&am.id) && self.grounded(def) && !self.semi_invulnerable(def) {
                        ctx.chain_modify(1, 2);
                    } else if am.ty == Type::Grass && self.grounded(att) {
                        ctx.chain_modify(5325, 4096);
                    }
                } else if c == conds::MISTYTERRAIN {
                    if am.ty == Type::Dragon && self.grounded(def) && !self.semi_invulnerable(def) {
                        ctx.chain_modify(1, 2);
                    }
                } else if c == conds::PSYCHICTERRAIN {
                    if am.ty == Type::Psychic && self.grounded(att) && !self.semi_invulnerable(att) {
                        ctx.chain_modify(5325, 4096);
                    }
                }
                R::Undef
            }
            Ev::SetStatus => {
                let Some(t) = ctx.target else { return R::Undef };
                let st = ctx.arg;
                if c == conds::SAFEGUARD {
                    let Some(src) = ctx.source else { return R::Undef };
                    if ctx.eff == Eff::None || is_yawn(ctx.eff) {
                        return R::Undef;
                    }
                    if self.infiltrating(ctx.eff) && t.0 != src.0 {
                        return R::Undef;
                    }
                    if t != src {
                        if ctx.eff == Eff::Ability(abilities::SYNCHRONIZE) || status_move(ctx.eff) {
                            self.row(Row::Blocked { p: Some(who(t)), by: Fx::Cond(c), what: self.fx_of(ctx.eff) });
                        }
                        return R::Null;
                    }
                    R::Undef
                } else if c == conds::ELECTRICTERRAIN {
                    if st == Status::Slp as u16 && self.grounded(t) && !self.semi_invulnerable(t) {
                        if is_yawn(ctx.eff) || status_move(ctx.eff) {
                            self.row(Row::Blocked { p: Some(who(t)), by: Fx::Cond(c), what: self.fx_of(ctx.eff) });
                        }
                        return R::False;
                    }
                    R::Undef
                } else if c == conds::MISTYTERRAIN {
                    if !self.grounded(t) || self.semi_invulnerable(t) {
                        return R::Undef;
                    }
                    if super::status::effect_status(ctx.eff) || is_yawn(ctx.eff) {
                        self.row(Row::Blocked { p: Some(who(t)), by: Fx::Cond(c), what: self.fx_of(ctx.eff) });
                    }
                    R::False
                } else {
                    R::Undef
                }
            }
            Ev::TryAddVolatile => {
                let Some(t) = ctx.target else { return R::Undef };
                let vol = Cond(ctx.arg);
                if c == conds::SAFEGUARD {
                    let Some(src) = ctx.source else { return R::Undef };
                    if ctx.eff == Eff::None {
                        return R::Undef;
                    }
                    if self.infiltrating(ctx.eff) && t.0 != src.0 {
                        return R::Undef;
                    }
                    if (vol == conds::CONFUSION || vol == conds::YAWN) && t != src {
                        if status_move(ctx.eff) {
                            self.row(Row::Blocked { p: Some(who(t)), by: Fx::Cond(c), what: Fx::Cond(vol) });
                        }
                        return R::Null;
                    }
                    R::Undef
                } else if c == conds::ELECTRICTERRAIN || c == conds::MISTYTERRAIN {
                    if !self.grounded(t) || self.semi_invulnerable(t) {
                        return R::Undef;
                    }
                    let blocked = if c == conds::ELECTRICTERRAIN { conds::YAWN } else { conds::CONFUSION };
                    if vol == blocked {
                        if c == conds::ELECTRICTERRAIN || status_move(ctx.eff) {
                            self.row(Row::Blocked { p: Some(who(t)), by: Fx::Cond(c), what: Fx::Cond(vol) });
                        }
                        return R::Null;
                    }
                    R::Undef
                } else {
                    R::Undef
                }
            }
            Ev::TryHit => {
                let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                if c == conds::PSYCHICTERRAIN {
                    if am.priority <= 0 || am.target == Target::Self_ {
                        return R::Undef;
                    }
                    if self.semi_invulnerable(t) || t.0 == src.0 {
                        return R::Undef;
                    }
                    if !self.grounded(t) {
                        return R::Undef;
                    }
                    self.row(Row::Blocked { p: Some(who(t)), by: Fx::Cond(c), what: Fx::Move(am.id) });
                    return R::Null;
                }
                let blocks = if c == conds::QUICKGUARD {
                    am.priority > 0
                } else if c == conds::WIDEGUARD {
                    matches!(am.target, Target::AllAdjacent | Target::AllAdjacentFoes)
                } else {
                    false
                };
                if !blocks || !am.has_flag(flag::PROTECT) {
                    return R::Undef;
                }
                // checkMoveBypassesProtect: HitProtect false lets the move through (Unseen Fist, Piercing Drill)
                if self.any_handler(Ev::HitProtect) {
                    let mut hc = Ctx::with_move(Some(src), Some(t), &am);
                    if !self.run_event(Ev::HitProtect, &mut hc, R::True).truthy() {
                        if let (Some(a), Some(x)) = (ctx.am.as_mut(), hc.am) {
                            a.bypass_protect = x.bypass_protect;
                        }
                        return R::Undef;
                    }
                }
                self.row(Row::Protect { p: who(t), by: c });
                if self.s.sides[src.0].vol(conds::LOCKEDMOVE).map_or(false, |v| v.duration == 2) {
                    self.s.sides[src.0].delete_vol(conds::LOCKEDMOVE);
                }
                R::NotFail
            }
            Ev::ModifySpe => {
                if c == conds::TAILWIND {
                    ctx.chain_modify(2, 1);
                }
                R::Undef
            }
            Ev::Immunity => {
                // onImmunity(type, pokemon): no freezing in sun
                if c == conds::SUNNYDAY && ctx.arg == Status::Frz as u16 {
                    if let Some(t) = ctx.target {
                        if self.eff_weather(t) == conds::SUNNYDAY {
                            return R::False;
                        }
                    }
                }
                R::Undef
            }
            Ev::BeforeMove | Ev::ModifyMove => {
                let Some(am) = ctx.am else { return R::Undef };
                if c == conds::GRAVITY {
                    if am.has_flag(flag::GRAVITY) {
                        if let Some(u) = ctx.target {
                            self.row(Row::Cant { p: who(u), why: Fx::Cond(c), mv: am.id });
                        }
                        return R::False;
                    }
                } else if c == conds::WONDERROOM && ev == Ev::ModifyMove {
                    if let Some(a) = ctx.am.as_mut() {
                        a.override_offensive = match a.override_offensive {
                            Some(Stat::Def) => Some(Stat::Spd),
                            Some(Stat::Spd) => Some(Stat::Def),
                            x => x,
                        };
                    }
                }
                R::Undef
            }
            Ev::DisableMove => {
                if c == conds::GRAVITY {
                    if let Some(p) = ctx.target {
                        let m = *self.mon_at(p);
                        for ms in m.moves() {
                            if ms.id.data().has_flag(flag::GRAVITY) {
                                self.disable_move(p, ms.id);
                            }
                        }
                    }
                }
                R::Undef
            }
            Ev::ModifyAccuracy => {
                if c == conds::GRAVITY {
                    if let R::Num(_) = relay {
                        ctx.chain_modify(6840, 4096);
                    }
                }
                R::Undef
            }
            Ev::TrapPokemon => {
                if c == conds::FAIRYLOCK {
                    if let Some(p) = ctx.target {
                        self.try_trap(p);
                    }
                }
                R::Undef
            }
            Ev::ModifySpD | Ev::ModifyDef => {
                // applied to the stat directly, before chained modifiers (priority 10)
                let Some(p) = ctx.target else { return R::Undef };
                let R::Num(x) = relay else { return R::Undef };
                let (w, ty) = if ev == Ev::ModifySpD { (conds::SANDSTORM, Type::Rock) } else { (conds::SNOWSCAPE, Type::Ice) };
                // pokemon.effectiveWeather from the weather's handler: Mega Sol on the attacker makes it sun
                if c == w && self.has_type_pk(p, ty) && self.eff_weather_move(p) == w {
                    return R::Num(super::calc::modify(x.max(0) as u32, 6144) as i32);
                }
                R::Undef
            }
            Ev::Weather => {
                if c == conds::SANDSTORM {
                    if let Some(p) = ctx.target {
                        let a = self.mon_at(p).base_maxhp as f64 / 16.0;
                        self.damage(a, p, None, Eff::Cond(c));
                    }
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    // ------------------------------------------------------------------------------------------ switch-in

    /// fieldEvent('SwitchIn') of a batch of switch-ins: for each switch-in its status (sub-order 0), its slot's
    /// conditions (3), its side's hazards in creation order (4) and its ability (7), and the onAnySwitchIn
    /// abilities of every active Pokémon, sorted by priority, speed (each Pokémon's speed less its place in
    /// `order`, Showdown's speedOrder, over two), sub-order and creation order. Returns true when the battle
    /// ended.
    pub fn switch_in_handlers(&mut self, switchers: &[Pk], order: [usize; 2]) -> bool {
        #[derive(Clone, Copy, Debug)]
        enum K {
            Tox,
            HealingWish,
            Hazard(u16, Cond),
            Ability(AbilityId, u16),
            AnyAbility(AbilityId, u16),
            Item(ItemId, u16),
            AnyItem(ItemId, u16),
        }
        // at most: per switch-in a status, a slot condition, four hazards, an ability and an item; per active
        // an onAnySwitchIn ability and item
        let mut buf = [((0usize, 0usize), K::Tox, 0f32, 0f64, 0f32, 0u16); 20];
        let mut nh = 0usize;
        let mut push = |x: (Pk, K, f32, f64, f32, u16)| {
            if nh < buf.len() {
                buf[nh] = x;
                nh += 1;
            }
        };
        let frac = |side: usize| order.iter().position(|&x| x == side).unwrap_or(0) as f64 / 2.0;
        for &p in switchers {
            let m = *self.mon_at(p);
            let speed = m.speed as f64 - frac(p.0);
            if m.status == Status::Tox {
                push((p, K::Tox, 0.0, speed, 0.0, 0));
            }
            if self.slot_cond(p.0, conds::HEALINGWISH).is_some() {
                push((p, K::HealingWish, 0.0, speed, 3.0, 0));
            }
            for x in self.s.sides[p.0].conds() {
                if HAZARDS.contains(&x.id) {
                    push((p, K::Hazard(x.order, x.id), 0.0, speed, 4.0, x.order));
                }
            }
            let ab = m.ability;
            let d = ab.data();
            let has_switch_in = d.handlers.contains(&"onSwitchIn");
            let fallback = !has_switch_in && !d.handlers.contains(&"onAnySwitchIn") && d.handlers.contains(&"onStart");
            if has_switch_in || fallback {
                let pr = super::events::ability_handler(ab, Pre::Own, Ev::SwitchIn).unwrap_or_else(|| {
                    d.nums.iter().find(|(k, _)| *k == "onSwitchInPriority").map(|(_, v)| *v).unwrap_or(0.0)
                });
                push((p, K::Ability(ab, m.ab_order), pr, speed, super::events::ability_sub_order(ab), m.ab_order));
            }
            let it = m.item;
            if !it.is_none() {
                let d = it.data();
                let fallback = !d.handlers.contains(&"onSwitchIn") && !d.handlers.contains(&"onAnySwitchIn") && d.handlers.contains(&"onStart");
                if fallback {
                    let pr = d.nums.iter().find(|(k, _)| *k == "onSwitchInPriority").map(|(_, v)| *v).unwrap_or(0.0);
                    push((p, K::Item(it, m.item_order), pr, speed, 8.0, m.item_order));
                }
            }
        }
        for side in 0..SIDES {
            let sd = &self.s.sides[side];
            if !sd.has_active {
                continue;
            }
            let p = (side, sd.active as usize);
            let m = *self.mon_at(p);
            let ab = m.ability;
            if ab.data().handlers.contains(&"onAnySwitchIn") {
                let pr = ab.data().nums.iter().find(|(k, _)| *k == "onAnySwitchInPriority").map(|(_, v)| *v).unwrap_or(0.0);
                let speed = m.speed as f64 - frac(side);
                push((p, K::AnyAbility(ab, m.ab_order), pr, speed, super::events::ability_sub_order(ab), m.ab_order));
            }
            let it = m.item;
            if !it.is_none() && it.data().handlers.contains(&"onAnySwitchIn") {
                let pr = it.data().nums.iter().find(|(k, _)| *k == "onAnySwitchInPriority").map(|(_, v)| *v).unwrap_or(0.0);
                let speed = m.speed as f64 - frac(side);
                push((p, K::AnyItem(it, m.item_order), pr, speed, 8.0, m.item_order));
            }
        }
        if nh == 0 {
            return false;
        }
        let hs = &mut buf[..nh];
        // comparePriority; the fractional speeds leave no ties between Pokémon
        hs.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap()
                .then(b.3.partial_cmp(&a.3).unwrap())
                .then(a.4.partial_cmp(&b.4).unwrap())
                .then(a.5.cmp(&b.5))
        });
        for &(p, k, ..) in hs.iter() {
            if self.mon_at(p).fainted && !matches!(k, K::HealingWish) {
                continue;
            }
            match k {
                K::Tox => {
                    // Toxic's onSwitchIn
                    let m = self.mon_at_mut(p);
                    if m.status == Status::Tox {
                        m.tox_stage = 0;
                    }
                }
                K::HealingWish => {
                    if self.slot_cond(p.0, conds::HEALINGWISH).is_some() {
                        self.healing_wish_swap(p);
                    }
                }
                K::Hazard(order, c) => {
                    // the condition may have been removed (Toxic Spikes absorbed) or replaced
                    if self.side_cond(p.0, c).map_or(true, |x| x.order != order) {
                        continue;
                    }
                    self.hazard(p, c);
                }
                K::Ability(ab, ord) => {
                    let m = self.mon_at(p);
                    if m.ability != ab || m.ab_order != ord {
                        continue;
                    }
                    self.ability_switch_in(p);
                }
                K::AnyAbility(ab, ord) => {
                    let m = self.mon_at(p);
                    if m.ability != ab || m.ab_order != ord || !m.is_active {
                        continue;
                    }
                    if !self.ignoring_ability(p) {
                        if ab == abilities::PASTELVEIL {
                            self.ability_start(p, ab);
                        } else if ab == abilities::OPPORTUNIST {
                            self.opportunist_copy(p);
                        }
                    }
                }
                K::Item(it, ord) => {
                    let m = self.mon_at(p);
                    if m.item != it || m.item_order != ord {
                        continue;
                    }
                    self.item_switch_in(p);
                }
                K::AnyItem(it, ord) => {
                    let m = self.mon_at(p);
                    if m.item != it || m.item_order != ord || !m.is_active || self.ignoring_item(p, false) {
                        continue;
                    }
                    if it == items::WHITEHERB {
                        self.white_herb(p);
                    }
                }
            }
            if self.resolve_faints(false, false, true) || self.s.ended {
                return true;
            }
        }
        false
    }

    /// Healing Wish's onSwitchIn and onSwap
    fn healing_wish_swap(&mut self, p: Pk) {
        let m = *self.mon_at(p);
        if m.fainted || !(m.hp < m.maxhp || m.status != Status::None) {
            return;
        }
        // pokemon.heal (no TryHeal event), then clearStatus
        let before = m.hp;
        let mm = self.mon_at_mut(p);
        if mm.hp > 0 && mm.hp < mm.maxhp {
            mm.hp = mm.maxhp;
        }
        self.cure_status_by(p, Fx::None, false);
        if self.logging() {
            self.row(Row::Hp { p: who(p), before, after: self.mon_at(p).hp, cause: Fx::Cond(conds::HEALINGWISH), of: None });
        }
        let sd = &mut self.s.sides[p.0];
        let n = sd.nslot_conds as usize;
        if let Some(i) = sd.slot_conds[..n].iter().position(|y| y.id == conds::HEALINGWISH) {
            sd.slot_conds.copy_within(i + 1..n, i);
            sd.slot_conds[n - 1] = SideCond::default();
            sd.nslot_conds -= 1;
        }
    }

    /// One hazard's onSwitchIn
    fn hazard(&mut self, p: Pk, c: Cond) {
        let eff = Eff::Cond(c);
        if c == conds::STEALTHROCK {
            let types = self.get_types(p, false);
            let mut tm = 0;
            for &ty in types.as_slice() {
                tm += effectiveness(Type::Rock, ty);
            }
            let tm = tm.clamp(-6, 6);
            let a = self.mon_at(p).maxhp as f64 * 2f64.powi(tm) / 8.0;
            self.damage(a, p, None, eff);
        } else if c == conds::SPIKES {
            if !self.grounded(p) {
                return;
            }
            let layers = self.side_cond(p.0, c).map_or(1, |x| x.layers).min(3) as usize;
            let a = [0.0, 3.0, 4.0, 6.0][layers] * self.mon_at(p).maxhp as f64 / 24.0;
            self.damage(a, p, None, eff);
        } else if c == conds::TOXICSPIKES {
            if !self.grounded(p) {
                return;
            }
            if self.has_type_pk(p, Type::Poison) {
                self.remove_side_cond(p.0, c);
                self.row(Row::Side { side: p.0 as u8, c, what: SideWhat::End, p: Some(who(p)), by: Fx::None });
            } else if self.has_type_pk(p, Type::Steel) {
            } else {
                let layers = self.side_cond(p.0, c).map_or(1, |x| x.layers);
                let foe = self.foe_of(p);
                let st = if layers >= 2 { Status::Tox } else { Status::Psn };
                self.try_set_status(p, st, Some(foe), eff);
            }
        } else if c == conds::STICKYWEB {
            if !self.grounded(p) {
                return;
            }
            let foe = self.foe_of(p);
            self.row(Row::Side { side: p.0 as u8, c, what: SideWhat::Acted, p: Some(who(p)), by: Fx::None });
            self.boost(&[(4, -1)], p, Some(foe), Eff::Move(moves::STICKYWEB), false, false);
        }
    }

    // ------------------------------------------------------------------------------------------ residual

    /// The effect is the move being used and it infiltrates (Infiltrator)
    fn infiltrating(&self, eff: Eff) -> bool {
        match (eff, self.active) {
            (Eff::Move(m), Some(am)) => am.id == m && am.infiltrates,
            _ => false,
        }
    }

    /// A field condition's onFieldResidual (the weathers): the Weather event on each Pokémon, then Update.
    pub fn weather_residual(&mut self, w: Cond) {
        if self.s.field.weather.id != w {
            return;
        }
        let checks = [conds::SANDSTORM, conds::HAIL, conds::SNOWSCAPE].contains(&w);
        if checks && self.weather() != w {
            return;
        }
        let (order, n) = self.speed_order();
        for &side in &order[..n] {
            let sd = &self.s.sides[side];
            if !sd.has_active {
                continue;
            }
            let p = (side, sd.active as usize);
            let mut ctx = Ctx::new(Some(p), None, Eff::Cond(w));
            self.run_event(Ev::Weather, &mut ctx, R::True);
        }
        self.each_event(super::hooks::Hook::Update);
    }

    /// Grassy Terrain's onResidual for one Pokémon
    pub fn grassy_heal(&mut self, p: Pk) {
        if self.terrain() != conds::GRASSYTERRAIN {
            return;
        }
        if self.grounded(p) && !self.semi_invulnerable(p) {
            let a = self.mon_at(p).base_maxhp as f64 / 16.0;
            self.heal(a, p, Some(p), Eff::Cond(conds::GRASSYTERRAIN));
        }
    }

    /// The onResidual of a slot condition (Wish, Future Sight)
    pub fn slot_residual(&mut self, side: usize, c: Cond) {
        let Some(x) = self.slot_cond(side, c) else { return };
        let count = overflowed(self.s.turn);
        if c == conds::WISH {
            if count <= x.turn {
                return;
            }
            self.remove_slot_cond(side, c);
        } else if c == conds::FUTUREMOVE {
            if count < x.turn {
                return;
            }
            self.remove_slot_cond(side, c);
        }
    }

    // ------------------------------------------------------------------------------------------ moves

    /// Brick Break, Psychic Fangs, Raging Bull: the target's side loses its screens
    pub fn break_screens(&mut self, side: usize) {
        for c in [conds::REFLECT, conds::LIGHTSCREEN, conds::AURORAVEIL] {
            self.remove_side_cond(side, c);
        }
    }

    /// Rapid Spin and Mortal Spin's onAfterHit (and onAfterSubDamage when the user still has HP)
    pub fn spin_clear(&mut self, user: Pk) {
        let by = Fx::Move(self.s.active_move);
        if self.remove_volatile(user, conds::LEECHSEED) {
            self.row(Row::VolatileEnd { p: who(user), c: conds::LEECHSEED, by });
        }
        for c in HAZARDS {
            self.remove_hazard(user.0, c, by);
        }
        if self.has_vol(user, conds::PARTIALLYTRAPPED) {
            self.remove_volatile(user, conds::PARTIALLYTRAPPED);
        }
    }

    /// Defog's onHit
    pub fn defog(&mut self, t: Pk, user: Pk, am: &ActiveMove) -> R {
        let mut success = false;
        if !self.has_vol(t, conds::SUBSTITUTE) || am.infiltrates {
            success = self.boost(&[(6, -1)], t, Some(user), Eff::Move(am.id), false, false).truthy();
        }
        for c in [conds::REFLECT, conds::LIGHTSCREEN, conds::AURORAVEIL, conds::SAFEGUARD] {
            self.remove_side_cond(t.0, c);
        }
        let by = Fx::Move(am.id);
        for c in HAZARDS {
            if self.remove_hazard(t.0, c, by) {
                success = true;
            }
        }
        for c in HAZARDS {
            if self.remove_hazard(user.0, c, by) {
                success = true;
            }
        }
        self.clear_terrain();
        R::of_bool(success)
    }

    /// Tidy Up's onHit
    pub fn tidy_up(&mut self, user: Pk) -> R {
        let mut success = false;
        for side in 0..SIDES {
            let sd = &self.s.sides[side];
            if sd.has_active && !sd.active_mon().fainted {
                let p = (side, sd.active as usize);
                if self.remove_volatile(p, conds::SUBSTITUTE) {
                    success = true;
                }
            }
        }
        for side in [user.0, 1 - user.0] {
            for c in HAZARDS {
                if self.remove_hazard(side, c, Fx::None) {
                    success = true;
                }
            }
        }
        if success {
            self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(moves::TIDYUP), of: None });
        }
        let b = self.boost(&[(0, 1), (4, 1)], user, Some(user), Eff::None, false, true);
        R::of_bool(b.truthy() || success)
    }

    /// Court Change's onHitField: the two sides' listed conditions change places, keeping their states
    pub fn court_change(&mut self, user: Pk) -> R {
        let (a, b) = (user.0, 1 - user.0);
        let take = |sd: &mut Side| -> Vec<SideCond> {
            let mut moved = Vec::new();
            let mut kept = Vec::new();
            for x in sd.conds() {
                if COURT.contains(&x.id) {
                    moved.push(*x);
                } else {
                    kept.push(*x);
                }
            }
            sd.conds = [SideCond::default(); MAX_SIDE_CONDS];
            for (i, x) in kept.iter().enumerate() {
                sd.conds[i] = *x;
            }
            sd.nconds = kept.len() as u8;
            moved
        };
        let from_a = take(&mut self.s.sides[a]);
        let from_b = take(&mut self.s.sides[b]);
        if from_a.is_empty() && from_b.is_empty() {
            return R::False;
        }
        for (side, list) in [(b, from_a), (a, from_b)] {
            let sd = &mut self.s.sides[side];
            for x in list {
                if (sd.nconds as usize) < MAX_SIDE_CONDS {
                    sd.conds[sd.nconds as usize] = x;
                    sd.nconds += 1;
                }
            }
        }
        self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(moves::COURTCHANGE), of: None });
        R::Undef
    }

    /// Future Sight's onTry: the attack is stored on the target's slot
    pub fn future_sight_try(&mut self, user: Pk, t: Pk) -> R {
        if !self.add_slot_cond(t.0, conds::FUTUREMOVE, user, Eff::Move(moves::FUTURESIGHT)).truthy() {
            return R::False;
        }
        self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(moves::FUTURESIGHT), of: None });
        R::NotFail
    }

    /// Morning Sun, Moonlight, Synthesis: the heal by weather
    pub fn weather_heal(&mut self, p: Pk, am: &ActiveMove) -> R {
        self.megasol_row(p);
        let w = self.eff_weather_move(p);
        let factor = if w == conds::SUNNYDAY {
            2732
        } else if w.is_none() {
            2048
        } else {
            1024
        };
        let a = super::calc::modify(self.mon_at(p).maxhp as u32, factor) as f64;
        if !self.heal(a, p, Some(p), Eff::Move(am.id)).truthy() {
            self.row(Row::Fail { p: Some(who(p)), why: Fx::HpFull });
            return R::NotFail;
        }
        R::True
    }

    /// The Revival Blessing action: a fainted Pokémon of the side comes back with half its HP
    pub fn revive(&mut self, side: usize, slot: usize) {
        let sd = &mut self.s.sides[side];
        sd.pokemon_left += 1;
        let m = &mut sd.mons[slot];
        m.fainted = false;
        m.faint_queued = false;
        m.status = Status::None;
        m.hp = ((m.maxhp / 2).max(1)).min(m.maxhp);
        self.row(Row::Revive { p: (side as u8, slot as u8) });
        let sd = &mut self.s.sides[side];
        let n = sd.nslot_conds as usize;
        if let Some(i) = sd.slot_conds[..n].iter().position(|y| y.id == conds::REVIVALBLESSING) {
            sd.slot_conds.copy_within(i + 1..n, i);
            sd.slot_conds[n - 1] = SideCond::default();
            sd.nslot_conds -= 1;
        }
    }
}

/// a status move's own effect (Showdown's `effect.effectType === 'Move' && !effect.secondaries`)
fn status_move(e: Eff) -> bool {
    matches!(e, Eff::Move(m) if m.data().secondaries.is_empty())
}

/// the effect is Yawn (its condition ending, or the move)
fn is_yawn(e: Eff) -> bool {
    e == Eff::Cond(conds::YAWN) || e == Eff::Move(moves::YAWN)
}

/// runStatusImmunity's argument as a runEvent extra input
pub fn imm_code(imm: Imm) -> u16 {
    match imm {
        Imm::Status(st) => st as u16,
        Imm::Cond(c) => 0x100 + c.0,
        Imm::Powder => 0xFFFF,
    }
}
