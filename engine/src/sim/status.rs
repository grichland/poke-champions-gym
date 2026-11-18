//! Types, immunities, major statuses, volatiles, stat stages, damage and healing (sim/pokemon.ts getTypes,
//! runStatusImmunity, isGrounded, setStatus, cureStatus, addVolatile, removeVolatile, clearVolatile;
//! sim/battle.ts boost, spreadDamage, directDamage, heal). Amounts are JavaScript numbers (f64) where
//! Showdown passes fractions, and are rounded where Showdown rounds.

use super::battle::Sim;
use super::data::*;
use super::events::*;
use super::outcome::*;
use super::state::*;
use super::log::*;

/// A type list as getTypes returns it (at most two types plus an added one).
#[derive(Clone, Copy, Debug)]
pub struct Types {
    pub t: [Type; 3],
    pub n: u8,
}

impl Types {
    pub fn as_slice(&self) -> &[Type] {
        &self.t[..self.n as usize]
    }
    pub fn has(&self, x: Type) -> bool {
        self.as_slice().contains(&x)
    }
}

/// What runStatusImmunity is asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Imm {
    Status(Status),
    Cond(Cond),
    Powder,
}

/// The type-chart immunity of one type to a non-type effect (dex.getImmunity on TypeChart damageTaken 3).
fn type_immune(imm: Imm, t: Type) -> bool {
    match imm {
        Imm::Status(Status::Brn) => t == Type::Fire,
        Imm::Status(Status::Par) => t == Type::Electric,
        Imm::Status(Status::Frz) => t == Type::Ice,
        Imm::Status(Status::Psn) | Imm::Status(Status::Tox) => t == Type::Poison || t == Type::Steel,
        Imm::Cond(c) if c == conds::TRAPPED => t == Type::Ghost,
        Imm::Cond(c) if c == conds::SANDSTORM => matches!(t, Type::Ground | Type::Rock | Type::Steel),
        Imm::Cond(c) if c == conds::HAIL => t == Type::Ice,
        Imm::Powder => t == Type::Grass,
        _ => false,
    }
}

/// `(effect as Move).status`: a move's own status, or the status Synchronize passes as its effect's
pub fn effect_status(e: Eff) -> bool {
    match e {
        Eff::Move(mv) => mv.data().status != Status::None,
        Eff::Ability(a) => a == abilities::SYNCHRONIZE,
        _ => false,
    }
}

#[inline]
pub fn js_trunc(x: f64) -> f64 {
    x.trunc()
}

impl<'a, O: Outcomes> Sim<'a, O> {
    #[inline]
    pub fn mon_at(&self, p: Pk) -> &Mon {
        &self.s.sides[p.0].mons[p.1]
    }
    #[inline]
    pub fn mon_at_mut(&mut self, p: Pk) -> &mut Mon {
        &mut self.s.sides[p.0].mons[p.1]
    }
    /// the Pokémon is its side's active one
    #[inline]
    pub fn is_active_pk(&self, p: Pk) -> bool {
        let sd = &self.s.sides[p.0];
        sd.has_active && sd.active as usize == p.1 && sd.mons[p.1].is_active
    }
    /// a volatile of an active Pokémon
    #[inline]
    pub fn has_vol(&self, p: Pk, c: Cond) -> bool {
        self.is_active_pk(p) && self.s.sides[p.0].has_vol(c)
    }
    pub fn vol(&self, p: Pk, c: Cond) -> Option<Volatile> {
        if !self.is_active_pk(p) {
            return None;
        }
        self.s.sides[p.0].vol(c).copied()
    }
    pub fn vol_mut(&mut self, p: Pk, c: Cond) -> Option<&mut Volatile> {
        if !self.is_active_pk(p) {
            return None;
        }
        self.s.sides[p.0].vol_mut(c)
    }
    /// the opposing active Pokémon (singles)
    #[inline]
    pub fn foe_of(&self, p: Pk) -> Pk {
        let f = 1 - p.0;
        (f, self.s.sides[f].active as usize)
    }

    // ------------------------------------------------------------------------------------------ types

    /// pokemon.getTypes: Roost's Type handler removes Flying; an empty list becomes Normal.
    pub fn get_types(&self, p: Pk, exclude_added: bool) -> Types {
        let m = self.mon_at(p);
        let roost = self.has_vol(p, conds::ROOST);
        let mut out = Types { t: [Type::Typeless; 3], n: 0 };
        for &t in &m.types[..m.ntypes as usize] {
            if roost && t == Type::Flying {
                continue;
            }
            out.t[out.n as usize] = t;
            out.n += 1;
        }
        if out.n == 0 {
            out.t[0] = Type::Normal;
            out.n = 1;
        }
        if !exclude_added && m.added_type != Type::Typeless {
            out.t[out.n as usize] = m.added_type;
            out.n += 1;
        }
        out
    }

    pub fn has_type_pk(&self, p: Pk, t: Type) -> bool {
        self.get_types(p, false).has(t)
    }

    /// dex.getImmunity against the Pokémon's types (no Immunity event)
    pub fn type_immunity(&self, p: Pk, imm: Imm) -> bool {
        let types = self.get_types(p, false);
        !types.as_slice().iter().any(|&t| type_immune(imm, t))
    }

    /// runStatusImmunity: false when immune (fainted, type chart, Immunity handlers).
    pub fn run_status_immunity(&mut self, p: Pk, imm: Imm) -> bool {
        if self.mon_at(p).fainted {
            return false;
        }
        if !self.type_immunity(p, imm) {
            return false;
        }
        // runEvent('Immunity', pokemon, null, null, type): Dig and Dive (weather), sun (freezing)
        if self.any_handler(Ev::Immunity) {
            let mut ctx = Ctx::new(Some(p), None, Eff::None);
            ctx.arg = super::field::imm_code(imm);
            if !self.run_event(Ev::Immunity, &mut ctx, R::True).truthy() {
                return false;
            }
        }
        true
    }

    /// pokemon.isGrounded
    pub fn grounded(&mut self, p: Pk) -> bool {
        if self.gravity() {
            return true;
        }
        if self.has_vol(p, conds::INGRAIN) || self.has_vol(p, conds::SMACKDOWN) {
            return true;
        }
        let m = *self.mon_at(p);
        let item = if m.item.is_none() || self.ignoring_item(p, false) { ItemId::NONE } else { m.item };
        if item == items::IRONBALL {
            return true;
        }
        let types = self.get_types(p, false);
        if types.has(Type::Flying) && !(types.has(Type::Typeless) && self.has_vol(p, conds::ROOST)) {
            return false;
        }
        if (m.ability == abilities::LEVITATE || m.ability == abilities::EELEVATE) && !self.ignoring_ability(p) && !self.suppressing_ability(p) {
            return false;
        }
        if self.has_vol(p, conds::MAGNETRISE) {
            return false;
        }
        item != items::AIRBALLOON
    }

    // ------------------------------------------------------------------------------------------ status

    /// pokemon.setStatus
    pub fn set_status(&mut self, p: Pk, st: Status, source: Option<Pk>, eff: Eff, ignore_imm: bool) -> bool {
        let m = *self.mon_at(p);
        if m.hp == 0 {
            return false;
        }
        if !m.is_active && st != Status::None {
            return false;
        }
        // the move's own status (Thunder Wave, Toxic): its failure has a row
        let move_status = match eff {
            Eff::Move(mv) => mv.data().status,
            // Synchronize passes its status as the effect's own
            Eff::Ability(a) if a == abilities::SYNCHRONIZE => st,
            _ => Status::None,
        };
        if m.status == st {
            // (Synchronize's failure rows are written by its handler, which knows the status it passed)
            if self.logging() && move_status != Status::None && !matches!(eff, Eff::Ability(_)) {
                if move_status == st {
                    self.row(Row::Fail { p: Some(who(p)), why: Fx::Status(st) });
                } else {
                    let src = source.unwrap_or(p);
                    self.row(Row::Fail { p: Some(who(src)), why: Fx::None });
                }
            }
            return false;
        }
        let corrosion = matches!(st, Status::Psn | Status::Tox) && self.has_ability(source.unwrap_or(p), abilities::CORROSION);
        if !ignore_imm && st != Status::None && !corrosion {
            let imm = Imm::Status(if st == Status::Tox { Status::Psn } else { st });
            if !self.run_status_immunity(p, imm) {
                if move_status != Status::None {
                    self.row(Row::Result { how: How::Immune, p: Some(who(p)) });
                }
                return false;
            }
        }
        if st != Status::None {
            // setStatus: a missing source is the Pokémon itself
            let mut ctx = Ctx::new(Some(p), Some(source.unwrap_or(p)), eff);
            ctx.arg = st as u16;
            let r = self.run_event(Ev::SetStatus, &mut ctx, R::True);
            if !r.truthy() {
                // a null result wrote its own row (Uproar): the move's "But it failed!" is not written
                self.status_null = r == R::Null;
                return false;
            }
        }
        {
            let m = self.mon_at_mut(p);
            m.status = st;
            m.status_time = 0;
            m.tox_stage = 0;
        }
        if st != Status::None && self.logging() {
            let by = self.fx_of(eff);
            let of = if matches!(eff, Eff::Ability(_)) { Some(who(source.unwrap_or(p))) } else { None };
            self.row(Row::Status { p: who(p), st, by, of, rest: eff == Eff::Move(moves::REST) });
        }
        match st {
            Status::Slp => {
                // champions slp onStart: 1/3 chance to wake on the second attempt
                let site = Site::new(Key::SleepTurns, Some(p.0 as u8));
                let t = self.o.sample(&site, &[2, 3, 3]);
                self.mon_at_mut(p).status_time = t as i8;
            }
            Status::Frz => self.mon_at_mut(p).status_time = 3,
            _ => {}
        }
        if st != Status::None && self.any_handler(Ev::AfterSetStatus) {
            let mut ctx = Ctx::new(Some(p), Some(source.unwrap_or(p)), eff);
            ctx.arg = st as u16;
            if !self.run_event(Ev::AfterSetStatus, &mut ctx, R::True).truthy() {
                return false;
            }
        }
        true
    }

    /// pokemon.trySetStatus: a statused Pokémon keeps its status (and the call fails)
    pub fn try_set_status(&mut self, p: Pk, st: Status, source: Option<Pk>, eff: Eff) -> bool {
        let cur = self.mon_at(p).status;
        self.set_status(p, if cur != Status::None { cur } else { st }, source, eff, false)
    }

    /// pokemon.cureStatus, with its row
    pub fn cure_status(&mut self, p: Pk) -> bool {
        self.cure_status_by(p, Fx::None, true)
    }

    /// pokemon.cureStatus (`shown`) or clearStatus; `by` the item or ability that cures
    pub fn cure_status_by(&mut self, p: Pk, by: Fx, shown: bool) -> bool {
        let m = self.mon_at(p);
        if m.hp == 0 || m.status == Status::None {
            return false;
        }
        if shown {
            let st = m.status;
            self.row(Row::Cure { p: who(p), st, by });
        }
        self.set_status(p, Status::None, None, Eff::None, false);
        true
    }

    // ------------------------------------------------------------------------------------------ volatiles

    /// pokemon.addVolatile; `linked` adds the trapper counterpart on the source (Mean Look and the like)
    pub fn add_volatile(&mut self, p: Pk, c: Cond, source: Option<Pk>, eff: Eff, linked: bool) -> R {
        // the gem's condition affectsFainted (a self-destructing user)
        if self.mon_at(p).hp == 0 && c != conds::GEM {
            return R::False;
        }
        if linked {
            if let Some(sp) = source {
                if self.mon_at(sp).hp == 0 {
                    return R::False;
                }
            }
        }
        let src = source.unwrap_or(p);
        if !self.is_active_pk(p) {
            return R::False;
        }
        if self.s.sides[p.0].has_vol(c) {
            if !self.logging() {
                return self.cond_restart(p, c, src, eff);
            }
            // the Restart handlers that write a start row again: Charge always, Stockpile for a new layer
            // (before its boosts), Smack Down when it brings a flier down
            let v = self.s.sides[p.0].vol(c).copied().unwrap();
            let at = if c == conds::CHARGE || (c == conds::STOCKPILE && v.data < 3) { self.vol_start_row(p, c, src, eff) } else { None };
            if let (Some(i), true) = (at, c == conds::STOCKPILE) {
                if let Some(Row::Volatile { detail, .. }) = self.log.as_deref_mut().and_then(|l| l.get_mut(i)) {
                    *detail = Detail::Num(v.data + 1);
                }
            }
            let flier = c == conds::SMACKDOWN && (self.has_vol(p, conds::FLY) || self.has_vol(p, conds::BOUNCE));
            let r = self.cond_restart(p, c, src, eff);
            if flier {
                if let Some(i) = self.vol_start_row(p, c, src, eff) {
                    self.vol_start_detail(p, c, i);
                }
            }
            return r;
        }
        if !self.run_status_immunity(p, Imm::Cond(c)) {
            return R::False;
        }
        if has_handler(c, Pre::Own, Ev::TryAddVolatile) || self.any_handler(Ev::TryAddVolatile) {
            let mut ctx = Ctx::new(Some(p), Some(src), eff);
            ctx.arg = c.0;
            let r = self.run_event(Ev::TryAddVolatile, &mut ctx, R::True);
            if !r.truthy() {
                return r;
            }
        }
        let sd = &mut self.s.sides[p.0];
        if sd.nvolatiles as usize >= MAX_VOLATILES {
            self.s.unimpl.note(EffectRef::Cond(c), super::hooks::Hook::Start);
            return R::False;
        }
        let n = sd.nvolatiles as usize;
        sd.volatiles[n] = Volatile { id: c, duration: c.data().duration as i8, source: pk_code(src), ..Default::default() };
        sd.nvolatiles += 1;
        if let Some(d) = self.cond_duration(p, c, src, eff) {
            if let Some(v) = self.s.sides[p.0].vol_mut(c) {
                v.duration = d;
            }
        }
        let at = if self.logging() { self.vol_start_row(p, c, src, eff) } else { None };
        let r = self.cond_start(p, c, src, eff);
        if !r.truthy() && r != R::Undef {
            self.s.sides[p.0].delete_vol(c);
            if let (Some(i), Some(l)) = (at, self.log.as_deref_mut()) {
                if i < l.len() {
                    l.remove(i);
                }
            }
            return r;
        }
        if let Some(i) = at {
            self.vol_start_detail(p, c, i);
        }
        if linked {
            if let Some(sp) = source {
                if self.is_active_pk(sp) && !self.s.sides[sp.0].has_vol(conds::TRAPPER) {
                    self.add_volatile(sp, conds::TRAPPER, Some(p), eff, false);
                }
                if let Some(v) = self.s.sides[sp.0].vol_mut(conds::TRAPPER) {
                    v.linked = true;
                    v.source = pk_code(p);
                }
                if let Some(v) = self.s.sides[p.0].vol_mut(c) {
                    v.linked = true;
                }
            }
        }
        R::True
    }

    /// pokemon.removeVolatile: End handler, removal, then the linked counterpart
    pub fn remove_volatile(&mut self, p: Pk, c: Cond) -> bool {
        if self.mon_at(p).hp == 0 || !self.is_active_pk(p) {
            return false;
        }
        let Some(v) = self.s.sides[p.0].vol(c).copied() else { return false };
        // Stockpile's row comes after the stages it gives back
        if vol_end_shown(c) && c != conds::STOCKPILE && self.logging() {
            self.row(Row::VolatileEnd { p: who(p), c, by: Fx::None });
        }
        self.cond_end(p, c, &v);
        if c == conds::STOCKPILE {
            self.row(Row::VolatileEnd { p: who(p), c, by: Fx::None });
        }
        self.s.sides[p.0].delete_vol(c);
        if v.linked {
            self.remove_linked(p, &v);
        }
        true
    }

    /// removeLinkedVolatiles: the counterpart on the linked Pokémon goes when its last link does
    fn remove_linked(&mut self, p: Pk, v: &Volatile) {
        let Some(other) = pk_of(v.source) else { return };
        let counterpart = if v.id == conds::TRAPPED { conds::TRAPPER } else { conds::TRAPPED };
        if !self.is_active_pk(other) {
            return;
        }
        let linked_here = self.s.sides[other.0].vol(counterpart).map_or(false, |x| x.linked && x.source == pk_code(p));
        if linked_here {
            self.remove_volatile(other, counterpart);
        }
    }

    // ------------------------------------------------------------------------------------------ stages

    /// battle.boost; `b` in the data's order. Returns Showdown's success value (null, true, 0 or false).
    pub fn boost(&mut self, b: &[(usize, i8)], target: Pk, source: Option<Pk>, eff: Eff, _secondary: bool, _is_self: bool) -> R {
        let m = self.mon_at(target);
        if m.hp == 0 {
            return R::Num(0);
        }
        if !m.is_active {
            return R::False;
        }
        if self.s.sides[1 - target.0].pokemon_left == 0 {
            return R::False;
        }
        let mut table = Boosts::of(b);
        if self.any_handler(Ev::ChangeBoost) {
            let mut ctx = Ctx::new(Some(target), source, eff);
            ctx.boosts = table;
            self.run_event(Ev::ChangeBoost, &mut ctx, R::True);
            table = ctx.boosts;
        }
        // getCappedBoost: requested zeros dropped, the rest capped at +-6
        let mut capped = Boosts::default();
        for &(i, v) in table.as_slice() {
            if v == 0 {
                continue;
            }
            let cur = self.mon_at(target).boosts[i as usize] as i32;
            capped.set(i as usize, ((cur + v as i32).clamp(-6, 6) - cur) as i8);
        }
        if self.any_handler(Ev::TryBoost) {
            let mut ctx = Ctx::new(Some(target), source, eff);
            ctx.boosts = capped;
            self.run_event(Ev::TryBoost, &mut ctx, R::True);
            capped = ctx.boosts;
        }
        let mut success = R::Null;
        let list = capped;
        let group = self.row_mark();
        let mut shown = false;
        for &(i, v) in list.as_slice() {
            let m = self.mon_at_mut(target);
            let cur = m.boosts[i as usize] as i32;
            let delta = (cur + v as i32).clamp(-6, 6) - cur;
            m.boosts[i as usize] += delta as i8;
            if self.logging() {
                // a stage at its limit reads "won't go lower" only at -6
                let up = if delta != 0 { delta > 0 } else { self.mon_at(target).boosts[i as usize] != -6 };
                self.boost_row(target, i as usize, delta as i8, up, eff, _secondary, _is_self, group, &mut shown);
            }
            if delta != 0 {
                success = R::True;
                if self.any_handler(Ev::AfterEachBoost) {
                    let mut ctx = Ctx::new(Some(target), source, eff);
                    ctx.boosts = Boosts::of(&[(i as usize, v)]);
                    self.run_event(Ev::AfterEachBoost, &mut ctx, R::True);
                }
            }
        }
        if self.any_handler(Ev::AfterBoost) {
            let mut ctx = Ctx::new(Some(target), source, eff);
            ctx.boosts = capped;
            self.run_event(Ev::AfterBoost, &mut ctx, R::True);
        }
        if success == R::True {
            let m = self.mon_at_mut(target);
            if capped.as_slice().iter().any(|x| x.1 > 0) {
                m.stats_raised_this_turn = true;
            }
            if capped.as_slice().iter().any(|x| x.1 < 0) {
                m.stats_lowered_this_turn = true;
            }
        }
        success
    }

    // ------------------------------------------------------------------------------------------ hp

    /// pokemon.faint: queued with its source and whether the effect was a move
    pub fn faint_by(&mut self, p: Pk, source: Option<Pk>, by_move: bool) -> u32 {
        let m = &mut self.s.sides[p.0].mons[p.1];
        if m.fainted || m.faint_queued {
            return 0;
        }
        let d = m.hp as u32;
        m.hp = 0;
        m.switch_flag = false;
        m.faint_queued = true;
        let n = self.s.nfaints as usize;
        assert!(n < MAX_FAINTS, "faint queue full");
        self.s.faint_queue[n] = FaintEntry {
            side: p.0 as u8,
            slot: p.1 as u8,
            source: source.map(pk_code).unwrap_or(NO_MON),
            by_move,
            future: false,
        };
        self.s.nfaints += 1;
        d
    }

    /// pokemon.damage
    pub fn hp_damage(&mut self, p: Pk, d: u32, source: Option<Pk>, eff: Eff) -> u32 {
        let m = &mut self.s.sides[p.0].mons[p.1];
        if m.hp == 0 || d == 0 {
            return 0;
        }
        let hp = m.hp as u32;
        if d >= hp {
            m.hp = 0;
            self.faint_by(p, source, eff.is_move());
            if let Eff::Move(mv) = eff {
                if mv.data().has_flag(super::data::flag::FUTUREMOVE) {
                    let n = self.s.nfaints as usize;
                    if n > 0 {
                        self.s.faint_queue[n - 1].future = true;
                    }
                }
            }
            return hp;
        }
        m.hp = (hp - d) as u16;
        d
    }

    /// battle.damage / spreadDamage for one target: the Damage event (Endure), HP, hurtThisTurn, drain.
    /// Returns the damage dealt, or None for Showdown's false.
    pub fn damage(&mut self, amount: f64, target: Pk, source: Option<Pk>, eff: Eff) -> Option<u32> {
        let m = self.mon_at(target);
        if m.hp == 0 {
            return Some(0);
        }
        if !m.is_active {
            return None;
        }
        let mut d = if amount != 0.0 { amount.floor().max(1.0) as i64 } else { 0 };
        if let Eff::Cond(c) = eff {
            // spreadDamage: weather damage checks the weather's immunity (type, Dig, Dive)
            if c.data().kind & ckind::WEATHER != 0 && !self.run_status_immunity(target, Imm::Cond(c)) {
                return Some(0);
            }
        }
        if eff != Eff::StruggleRecoil && (self.any_handler(Ev::Damage)) {
            let mut ctx = Ctx::new(Some(target), source, eff);
            match self.run_event(Ev::Damage, &mut ctx, R::Num(d as i32)) {
                R::Num(x) => d = x as i64,
                R::True => {}
                _ => return None,
            }
            if d != 0 {
                d = d.max(1);
            }
        }
        let before = self.mon_at(target).hp;
        let dealt = self.hp_damage(target, d as u32, source, eff);
        if self.logging() {
            self.damage_row(target, before, source, eff);
        }
        if dealt != 0 {
            let hp = self.mon_at(target).hp;
            self.mon_at_mut(target).hurt_this_turn = Some(hp);
        }
        if let (Some(sp), true) = (source, eff.is_move()) {
            self.mon_at_mut(sp).last_damage = dealt as u16;
        }
        if dealt > 0 {
            if let (Eff::Move(mv), Some(sp)) = (eff, source) {
                let dr = mv.data().drain;
                if dr.1 > 0 {
                    let amount = (dealt as f64 * dr.0 as f64 / dr.1 as f64).round();
                    self.heal(amount, sp, Some(target), Eff::Drain);
                }
            }
        }
        Some(dealt)
    }

    /// battle.directDamage: no Damage event
    pub fn direct_damage(&mut self, amount: f64, target: Pk, source: Option<Pk>, eff: Eff) -> u32 {
        if self.mon_at(target).hp == 0 || amount == 0.0 {
            return 0;
        }
        let d = amount.floor().max(1.0) as u32;
        let before = self.mon_at(target).hp;
        let dealt = self.hp_damage(target, d, source, eff);
        if self.logging() {
            self.damage_row(target, before, source, eff);
        }
        dealt
    }

    /// battle.heal: TryHeal (Heal Block), then pokemon.heal. Num(healed), or Showdown's false/null/0.
    pub fn heal(&mut self, amount: f64, target: Pk, source: Option<Pk>, eff: Eff) -> R {
        let mut a = amount;
        if a != 0.0 && a <= 1.0 {
            a = 1.0;
        }
        let a = js_trunc(a) as i64;
        let mut r = R::Num(a as i32);
        if self.any_handler(Ev::TryHeal) {
            let mut ctx = Ctx::new(Some(target), source, eff);
            r = self.run_event(Ev::TryHeal, &mut ctx, r);
        }
        if !r.truthy() {
            return r;
        }
        let R::Num(a) = r else { return r };
        let m = self.mon_at_mut(target);
        if m.hp == 0 || !m.is_active || m.hp >= m.maxhp {
            return R::False;
        }
        let room = (m.maxhp - m.hp) as i64;
        let h = (a as i64).min(room);
        let before = m.hp;
        m.hp += h as u16;
        if self.logging() {
            let cause = self.fx_of(eff);
            let of = source.filter(|&sp| sp != target);
            self.row_hp(target, before, cause, of);
        }
        R::Num(h as i32)
    }

    /// pokemon.sethp (Pain Split)
    pub fn set_hp(&mut self, p: Pk, d: f64) {
        let m = self.mon_at_mut(p);
        if m.hp == 0 {
            return;
        }
        let mut d = js_trunc(d) as i64;
        if d < 1 {
            d = 1;
        }
        m.hp = d.min(m.maxhp as i64) as u16;
    }
}
