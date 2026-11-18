//! The handlers of statuses and volatile conditions (data/conditions.ts, the `condition` of each move in
//! data/moves.ts, data/mods/champions/conditions.ts and moves.ts). Messages are left out; only what changes
//! the battle is kept. Each function follows the Showdown handler of the same event.

use super::battle::Sim;
use super::data::*;
use super::events::*;
use super::moves::ActiveMove;
use super::outcome::*;
use super::state::*;
use super::log::*;

/// checkMoveBypassesProtect: no protect flag, or a status move when status moves are not blocked
fn bypasses_protect(am: &ActiveMove, block_status: bool) -> bool {
    !((am.category != Category::Status || block_status) && am.has_flag(flag::PROTECT))
}

impl<'a, O: Outcomes> Sim<'a, O> {
    /// queue.willMove(pokemon)
    pub fn will_move(&self, p: Pk) -> Option<Action> {
        if self.mon_at(p).fainted {
            return None;
        }
        self.s.queue.as_slice().iter().find(|a| a.kind == ActionKind::Move && a.side as usize == p.0 && a.slot as usize == p.1).copied()
    }

    /// queue.willAct()
    pub fn will_act(&self) -> bool {
        self.s.queue.as_slice().iter().any(|a| matches!(a.kind, ActionKind::Move | ActionKind::Switch | ActionKind::InstaSwitch))
    }

    /// queue.cancelMove(pokemon)
    pub fn cancel_move(&mut self, p: Pk) -> bool {
        let q = self.s.queue.as_slice();
        if let Some(i) = q.iter().position(|a| a.kind == ActionKind::Move && a.side as usize == p.0 && a.slot as usize == p.1) {
            self.s.queue.remove(i);
            return true;
        }
        false
    }

    /// pokemon.tryTrap
    pub fn try_trap(&mut self, p: Pk) -> bool {
        if !self.run_status_immunity(p, super::status::Imm::Cond(conds::TRAPPED)) {
            return false;
        }
        let m = self.mon_at_mut(p);
        m.trapped = true;
        m.trap_hidden = false;
        true
    }

    /// pokemon.disableMove
    pub fn disable_move(&mut self, p: Pk, mv: MoveId) {
        let m = self.mon_at_mut(p);
        let n = m.nmoves as usize;
        for ms in m.moves[..n].iter_mut() {
            if ms.id == mv {
                ms.disabled = true;
                ms.disabled_hidden = false;
            }
        }
    }

    /// pokemon.disableMove(id, true): disabled, but the request still offers it (Imprison)
    pub fn disable_move_hidden(&mut self, p: Pk, mv: MoveId) {
        let m = self.mon_at_mut(p);
        let n = m.nmoves as usize;
        for ms in m.moves[..n].iter_mut() {
            if ms.id == mv && !ms.disabled {
                ms.disabled = true;
                ms.disabled_hidden = true;
            }
        }
    }

    fn swap_atk_def(&mut self, p: Pk) {
        let m = self.mon_at_mut(p);
        m.stats.swap(0, 1);
    }

    // ------------------------------------------------------------------------------------------ duration

    /// durationCallback
    pub fn cond_duration(&mut self, p: Pk, c: Cond, src: Pk, eff: Eff) -> Option<i8> {
        match c {
            c if c == conds::PARTIALLYTRAPPED => {
                // (Grip Claw is not legal in Champions)
                let _ = src;
                let site = Site::new(Key::TrapTurns, Some(p.0 as u8));
                Some(self.o.range(&site, 5, 7) as i8)
            }
            c if c == conds::HEALBLOCK => {
                if eff == Eff::Move(moves::PSYCHICNOISE) {
                    Some(2)
                } else {
                    Some(5)
                }
            }
            _ => None,
        }
    }

    // ------------------------------------------------------------------------------------------ Start

    /// the Start handler (falsy cancels the volatile)
    pub fn cond_start(&mut self, p: Pk, c: Cond, src: Pk, eff: Eff) -> R {
        match c {
            c if c == conds::CONFUSION => {
                let min = if eff == Eff::Move(moves::AXEKICK) { 3 } else { 2 };
                let site = Site::new(Key::ConfusionTurns, Some(p.0 as u8));
                let t = self.o.range(&site, min, 6);
                self.s.sides[p.0].vol_mut(c).unwrap().data = t as i8;
                R::Undef
            }
            c if c == conds::PARTIALLYTRAPPED => {
                let mv = if let Eff::Move(m) = eff { m } else { MoveId::NONE };
                let bound = if self.has_item(src, items::BINDINGBAND) { 6 } else { 8 };
                let v = self.s.sides[p.0].vol_mut(c).unwrap();
                v.mv = mv;
                v.data = bound;
                R::Undef
            }
            c if c == conds::LOCKEDMOVE => {
                let site = Site::new(Key::LockTurns, Some(p.0 as u8));
                let t = self.o.range(&site, 2, 4);
                let mv = if let Eff::Move(m) = eff { m } else { MoveId::NONE };
                let v = self.s.sides[p.0].vol_mut(c).unwrap();
                v.data = t as i8;
                v.mv = mv;
                R::Undef
            }
            c if c == conds::TWOTURNMOVE => {
                let mv = if let Eff::Move(m) = eff { m } else { MoveId::NONE };
                self.s.sides[p.0].vol_mut(c).unwrap().mv = mv;
                if let Some(mc) = Cond::from_id(mv.id()) {
                    self.add_volatile(p, mc, Some(src), eff, false);
                }
                // the side effects of hitting run now (Protean, Libero)
                if let Some(mut am) = self.active.filter(|a| a.id == mv) {
                    self.prepare_hit_event(p, src, &mut am);
                }
                R::Undef
            }
            c if c == conds::STALL || c == conds::ALLYSWITCH => {
                self.s.sides[p.0].vol_mut(c).unwrap().counter = 3;
                R::Undef
            }
            c if c == conds::DISABLE => {
                let will = self.will_move(p).is_some();
                let acting = self.s.active_pokemon == pk_code(p) && self.active.is_some();
                if will || acting {
                    self.s.sides[p.0].vol_mut(c).unwrap().duration -= 1;
                }
                let m = *self.mon_at(p);
                if m.last_move.is_none() {
                    return R::False;
                }
                if m.moves().iter().any(|ms| ms.id == m.last_move && ms.pp == 0) {
                    return R::False;
                }
                self.s.sides[p.0].vol_mut(c).unwrap().mv = m.last_move;
                R::Undef
            }
            c if c == conds::DRAGONCHEER => {
                if self.s.sides[p.0].has_vol(conds::FOCUSENERGY) {
                    return R::False;
                }
                let dragon = self.has_type_pk(p, Type::Dragon);
                self.s.sides[p.0].vol_mut(c).unwrap().data2 = dragon as i8;
                R::Undef
            }
            c if c == conds::FOCUSENERGY => {
                if self.s.sides[p.0].has_vol(conds::DRAGONCHEER) {
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::ENCORE => {
                let m = *self.mon_at(p);
                let mv = m.last_move;
                if mv.is_none() {
                    return R::False;
                }
                let slot = m.moves().iter().find(|ms| ms.id == mv).copied();
                if mv.data().has_flag(flag::FAILENCORE) || slot.map_or(true, |s| s.pp == 0) {
                    return R::False;
                }
                self.s.sides[p.0].vol_mut(c).unwrap().mv = mv;
                match self.will_move(p) {
                    None => self.s.sides[p.0].vol_mut(c).unwrap().duration += 1,
                    // Champions: no new action when Mental Herb will cure the Encore first
                    Some(a) if a.mv != mv && !self.has_item(p, items::MENTALHERB) => {
                        self.change_action_move(p, mv);
                    }
                    _ => {}
                }
                R::Undef
            }
            c if c == conds::NORETREAT => {
                // its row only when the trap holds
                if self.add_volatile(p, conds::TRAPPED, Some(src), Eff::Cond(c), false).truthy() {
                    self.row(Row::Volatile { p: who(p), c, detail: Detail::None, by: Fx::None, of: None });
                }
                R::Undef
            }
            c if c == conds::POWERTRICK || c == conds::POWERSHIFT => {
                self.swap_atk_def(p);
                R::Undef
            }
            c if c == conds::ATTRACT => {
                let (a, b) = (self.mon_at(p).gender, self.mon_at(src).gender);
                if !((a == 1 && b == 2) || (a == 2 && b == 1)) {
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::COUNTER || c == conds::MIRRORCOAT => {
                let v = self.s.sides[p.0].vol_mut(c).unwrap();
                v.data = 0;
                v.hp = 0;
                R::Undef
            }
            c if c == conds::SMACKDOWN => {
                let mut applies = self.has_type_pk(p, Type::Flying) || self.has_ability(p, abilities::LEVITATE)
                    || self.has_ability(p, abilities::EELEVATE);
                if self.has_item(p, items::IRONBALL) || self.s.sides[p.0].has_vol(conds::INGRAIN) || self.gravity() {
                    applies = false;
                }
                let fly = self.remove_volatile(p, conds::FLY);
                let bounce = if fly { false } else { self.remove_volatile(p, conds::BOUNCE) };
                if fly || bounce {
                    applies = true;
                    self.cancel_move(p);
                    self.remove_volatile(p, conds::TWOTURNMOVE);
                }
                if self.s.sides[p.0].has_vol(conds::MAGNETRISE) {
                    applies = true;
                    self.s.sides[p.0].delete_vol(conds::MAGNETRISE);
                }
                if !applies {
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::STOCKPILE => {
                {
                    let v = self.s.sides[p.0].vol_mut(c).unwrap();
                    v.data = 1;
                    v.data2 = 0;
                    v.data3 = 0;
                }
                self.stockpile_boost(p);
                R::Undef
            }
            c if c == conds::SUBSTITUTE => {
                // in half points: a Substitute can be left with a half point (Metal Burst's 1.5x)
                let hp = self.mon_at(p).maxhp / 4;
                self.s.sides[p.0].vol_mut(c).unwrap().hp = 2 * hp as i16;
                self.s.sides[p.0].delete_vol(conds::PARTIALLYTRAPPED);
                R::Undef
            }
            c if c == conds::TAUNT => {
                if self.mon_at(p).active_turns > 0 && self.will_move(p).is_none() {
                    self.s.sides[p.0].vol_mut(c).unwrap().duration += 1;
                }
                R::Undef
            }
            c if c == conds::HEALBLOCK => {
                self.mon_at_mut(src).move_this_turn_result = MoveResult::True;
                R::Undef
            }
            c if c == conds::LEECHSEED => {
                // the seeder's slot: whoever stands there drains
                self.s.sides[p.0].vol_mut(c).unwrap().source = pk_code(src);
                R::Undef
            }
            c if c == conds::PROTOSYNTHESIS || c == conds::QUARKDRIVE => {
                // bestStat with stages, without modifiers
                let best = self.best_stat(p, false, true);
                self.s.sides[p.0].vol_mut(c).unwrap().data = best as i8;
                R::Undef
            }
            c if c == conds::GASTROACID => {
                // singleEvent('End') of the suppressed ability
                let ab = self.mon_at(p).ability;
                self.ability_end(p, ab);
                R::Undef
            }
            c if c == conds::CHOICELOCK || c == conds::METRONOME => self.item_cond_start(p, c),
            _ => R::Undef,
        }
    }

    /// Stockpile's boost of one layer, remembering what changed
    fn stockpile_boost(&mut self, p: Pk) {
        let before = self.mon_at(p).boosts;
        self.boost(&[(1, 1), (3, 1)], p, Some(p), Eff::Cond(conds::STOCKPILE), false, false);
        let after = self.mon_at(p).boosts;
        let v = self.s.sides[p.0].vol_mut(conds::STOCKPILE).unwrap();
        if before[1] != after[1] {
            v.data2 -= 1;
        }
        if before[3] != after[3] {
            v.data3 -= 1;
        }
    }

    // ------------------------------------------------------------------------------------------ Restart

    /// addVolatile on a condition already present: false without a Restart handler
    pub fn cond_restart(&mut self, p: Pk, c: Cond, _src: Pk, eff: Eff) -> R {
        match c {
            c if c == conds::LOCKEDMOVE => {
                let v = self.s.sides[p.0].vol_mut(c).unwrap();
                if v.data >= 2 {
                    v.duration = 2;
                }
                R::True
            }
            c if c == conds::STALL => {
                let v = self.s.sides[p.0].vol_mut(c).unwrap();
                if v.counter < 729 {
                    v.counter *= 3;
                }
                v.duration = 2;
                R::True
            }
            c if c == conds::ALLYSWITCH => {
                let counter = self.s.sides[p.0].vol(c).unwrap().counter.max(1) as u32;
                let site = Site { effect: Effect::One(EffectRef::Cond(c)), ..Site::new(Key::Proc, Some(p.0 as u8)) };
                if !self.chance(site, 1, counter) {
                    self.s.sides[p.0].delete_vol(c);
                    return R::False;
                }
                let v = self.s.sides[p.0].vol_mut(c).unwrap();
                if v.counter < 729 {
                    v.counter *= 3;
                }
                v.duration = 2;
                R::True
            }
            c if c == conds::CHARGE => R::True,
            c if c == conds::POWERTRICK || c == conds::POWERSHIFT => {
                self.remove_volatile(p, c);
                R::True
            }
            c if c == conds::MINIMIZE => R::Null,
            c if c == conds::SMACKDOWN => {
                let fly = self.remove_volatile(p, conds::FLY);
                let bounce = if fly { false } else { self.remove_volatile(p, conds::BOUNCE) };
                if fly || bounce {
                    self.cancel_move(p);
                    self.remove_volatile(p, conds::TWOTURNMOVE);
                }
                R::True
            }
            c if c == conds::STOCKPILE => {
                if self.s.sides[p.0].vol(c).unwrap().data >= 3 {
                    return R::False;
                }
                self.s.sides[p.0].vol_mut(c).unwrap().data += 1;
                self.stockpile_boost(p);
                R::True
            }
            c if c == conds::HEALBLOCK => {
                if eff == Eff::Move(moves::PSYCHICNOISE) {
                    return R::True;
                }
                // a fresh Heal Block on a blocked target: the move counts as failed
                R::True
            }
            _ => R::False,
        }
    }

    // ------------------------------------------------------------------------------------------ End

    /// the End handler, before the volatile is deleted
    pub fn cond_end(&mut self, p: Pk, c: Cond, v: &Volatile) {
        match c {
            c if c == conds::LOCKEDMOVE => {
                if v.data > 1 {
                    return;
                }
                self.add_volatile(p, conds::CONFUSION, Some(p), Eff::Cond(conds::LOCKEDMOVE), false);
            }
            c if c == conds::TWOTURNMOVE => {
                if let Some(mc) = Cond::from_id(v.mv.id()) {
                    if !v.mv.is_none() {
                        self.remove_volatile(p, mc);
                    }
                }
            }
            c if c == conds::POWERTRICK || c == conds::POWERSHIFT => self.swap_atk_def(p),
            c if c == conds::STOCKPILE => {
                let mut b = [(0usize, 0i8); 2];
                let mut n = 0;
                if v.data2 != 0 {
                    b[n] = (1, v.data2);
                    n += 1;
                }
                if v.data3 != 0 {
                    b[n] = (3, v.data3);
                    n += 1;
                }
                if n > 0 {
                    self.boost(&b[..n], p, Some(p), Eff::Cond(c), false, false);
                }
            }
            c if c == conds::PERISHSONG => {
                self.row(Row::Volatile { p: who(p), c, detail: Detail::Num(0), by: Fx::None, of: None });
                self.faint_by(p, None, false);
            }
            c if c == conds::YAWN => {
                let src = pk_of(v.source);
                self.try_set_status(p, Status::Slp, src, Eff::Cond(c));
            }
            _ => {}
        }
    }

    /// singleEvent('Copy') after Baton Pass
    pub fn cond_copy(&mut self, p: Pk, c: Cond) {
        if c == conds::POWERTRICK || c == conds::POWERSHIFT {
            self.swap_atk_def(p);
        }
        if c == conds::GASTROACID && self.mon_at(p).ability.data().flags & aflag::CANTSUPPRESS != 0 {
            self.remove_volatile(p, c);
        }
    }

    // ------------------------------------------------------------------------------------------ events

    /// One handler of a runEvent.
    pub fn cond_event(&mut self, ev: Ev, h: &H, ctx: &mut Ctx, relay: R) -> R {
        let p = h.pk;
        let c = h.cond;
        if c == conds::CHOICELOCK || c == conds::METRONOME || c == conds::GEM || c == conds::FLING {
            return self.item_cond_event(ev, h, ctx, relay);
        }
        match ev {
            Ev::BeforeMove => self.before_move_handler(p, c, h, ctx),
            Ev::MoveAborted => {
                let am = ctx.am.unwrap();
                if c == conds::TWOTURNMOVE {
                    self.remove_volatile(p, c);
                } else if c == conds::CHARGE {
                    if am.ty == Type::Electric && am.id != moves::CHARGE {
                        self.remove_volatile(p, c);
                    }
                } else if c == conds::DESTINYBOND {
                    self.remove_volatile(p, c);
                }
                R::Undef
            }
            Ev::AfterMove => {
                let am = ctx.am.unwrap();
                if c == conds::LOCKEDMOVE {
                    if self.s.sides[p.0].vol(c).map_or(false, |v| v.duration == 1) {
                        self.remove_volatile(p, c);
                    }
                } else if c == conds::CHARGE && am.ty == Type::Electric && am.id != moves::CHARGE {
                    self.remove_volatile(p, c);
                }
                R::Undef
            }
            Ev::DisableMove => {
                let m = *self.mon_at(p);
                let n = m.nmoves as usize;
                match c {
                    c if c == conds::DISABLE => {
                        let mv = h.state.mv;
                        self.disable_move(p, mv);
                    }
                    c if c == conds::ENCORE => {
                        let mv = h.state.mv;
                        if mv.is_none() || !m.moves[..n].iter().any(|ms| ms.id == mv) {
                            return R::Undef;
                        }
                        for ms in m.moves[..n].iter() {
                            if ms.id != mv {
                                self.disable_move(p, ms.id);
                            }
                        }
                    }
                    c if c == conds::TAUNT => {
                        for ms in m.moves[..n].iter() {
                            if ms.id.data().category == Category::Status {
                                self.disable_move(p, ms.id);
                            }
                        }
                    }
                    c if c == conds::THROATCHOP => {
                        for ms in m.moves[..n].iter() {
                            if ms.id.data().has_flag(flag::SOUND) {
                                self.disable_move(p, ms.id);
                            }
                        }
                    }
                    c if c == conds::HEALBLOCK => {
                        for ms in m.moves[..n].iter() {
                            if ms.id.data().has_flag(flag::HEAL) {
                                self.disable_move(p, ms.id);
                            }
                        }
                    }
                    c if c == conds::TORMENT => {
                        if !m.last_move.is_none() && m.last_move != moves::STRUGGLE {
                            self.disable_move(p, m.last_move);
                        }
                    }
                    c if c == conds::IMPRISON && h.pre == Pre::Foe => {
                        let src = *self.mon_at(h.pk);
                        let Some(t) = ctx.target else { return R::Undef };
                        for ms in src.moves() {
                            if ms.id != moves::STRUGGLE {
                                self.disable_move_hidden(t, ms.id);
                            }
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::TrapPokemon => {
                if c == conds::TRAPPED || c == conds::INGRAIN {
                    self.try_trap(p);
                } else if c == conds::PARTIALLYTRAPPED || c == conds::OCTOLOCK {
                    if pk_of(h.state.source).map_or(false, |s| self.is_active_pk(s)) {
                        self.try_trap(p);
                    }
                }
                R::Undef
            }
            Ev::ModifyType => {
                if c == conds::ELECTRIFY {
                    if let Some(am) = ctx.am.as_mut() {
                        if am.id != moves::STRUGGLE {
                            am.ty = Type::Electric;
                        }
                    }
                }
                R::Undef
            }
            Ev::ModifyMove => {
                let am = ctx.am.unwrap();
                if c == conds::FRZ {
                    if am.has_flag(flag::DEFROST) {
                        self.cure_status_by(p, Fx::Move(am.id), true);
                    }
                } else if c == conds::THROATCHOP {
                    if am.has_flag(flag::SOUND) {
                        self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: MoveId::NONE });
                        return R::False;
                    }
                } else if c == conds::HEALBLOCK && am.has_flag(flag::HEAL) {
                    self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: am.id });
                    return R::False;
                }
                R::Undef
            }
            Ev::TryHit => {
                let am = ctx.am.unwrap();
                let src = ctx.source.unwrap();
                let block_status = c != conds::KINGSSHIELD;
                if !(c == conds::PROTECT || c == conds::KINGSSHIELD || c == conds::SPIKYSHIELD || c == conds::BANEFULBUNKER) {
                    return R::Undef;
                }
                if bypasses_protect(&am, block_status) {
                    return R::Undef;
                }
                if self.any_handler(Ev::HitProtect) {
                    // runEvent('HitProtect', attacker, defender, move): false lets the move through
                    let mut hc = Ctx::with_move(Some(src), Some(p), &am);
                    if !self.run_event(Ev::HitProtect, &mut hc, R::True).truthy() {
                        if let (Some(a), Some(x)) = (ctx.am.as_mut(), hc.am) {
                            a.bypass_protect = x.bypass_protect;
                        }
                        return R::Undef;
                    }
                }
                self.row(Row::Protect { p: who(p), by: c });
                if self.s.sides[src.0].vol(conds::LOCKEDMOVE).map_or(false, |v| v.duration == 2) {
                    self.s.sides[src.0].delete_vol(conds::LOCKEDMOVE);
                }
                if am.has_flag(flag::CONTACT) {
                    if c == conds::KINGSSHIELD {
                        self.boost(&[(0, -1)], src, Some(p), Eff::Move(moves::KINGSSHIELD), false, false);
                    } else if c == conds::SPIKYSHIELD {
                        let a = self.mon_at(src).base_maxhp as f64 / 8.0;
                        self.damage(a, src, Some(p), Eff::Cond(c));
                    } else if c == conds::BANEFULBUNKER {
                        self.try_set_status(src, Status::Psn, Some(p), Eff::Move(moves::BANEFULBUNKER));
                    }
                }
                R::NotFail
            }
            Ev::Hit => {
                let am = ctx.am.unwrap();
                let src = ctx.source.unwrap();
                if c == conds::BEAKBLAST {
                    if am.has_flag(flag::CONTACT) {
                        self.try_set_status(src, Status::Brn, Some(p), Eff::Cond(c));
                    }
                } else if c == conds::FOCUSPUNCH && am.category != Category::Status {
                    if let Some(v) = self.s.sides[p.0].vol_mut(c) {
                        v.data2 = 1;
                    }
                }
                R::Undef
            }
            Ev::TryAddVolatile => {
                if c == conds::FOCUSPUNCH && ctx.arg == conds::FLINCH.0 {
                    return R::Null;
                }
                R::Undef
            }
            Ev::Invulnerability => {
                let am = ctx.am.unwrap();
                let id = am.id.id();
                match c {
                    c if c == conds::FLY || c == conds::BOUNCE => {
                        if ["gust", "twister", "skyuppercut", "thunder", "hurricane", "smackdown", "thousandarrows"].contains(&id) {
                            R::Undef
                        } else {
                            R::False
                        }
                    }
                    c if c == conds::DIG => {
                        if ["earthquake", "magnitude"].contains(&id) {
                            R::Undef
                        } else {
                            R::False
                        }
                    }
                    c if c == conds::DIVE => {
                        if ["surf", "whirlpool"].contains(&id) {
                            R::Undef
                        } else {
                            R::False
                        }
                    }
                    c if c == conds::PHANTOMFORCE => R::False,
                    c if c == conds::LOCKON && h.pre == Pre::Source => {
                        // the Lock-On user attacking its locked target
                        if ctx.target.map(pk_code) == Some(h.state.source) {
                            R::Num(0)
                        } else {
                            R::Undef
                        }
                    }
                    _ => R::Undef,
                }
            }
            Ev::Accuracy => {
                let am = ctx.am.unwrap();
                match c {
                    c if c == conds::GLAIVERUSH && h.pre == Pre::Own => R::True,
                    c if c == conds::MINIMIZE && h.pre == Pre::Own => {
                        if am.has_flag(flag::MINIMIZE) {
                            R::True
                        } else {
                            relay
                        }
                    }
                    c if c == conds::LOCKON && h.pre == Pre::Source => {
                        if ctx.target.map(pk_code) == Some(h.state.source) {
                            R::True
                        } else {
                            R::Undef
                        }
                    }
                    _ => R::Undef,
                }
            }
            Ev::BasePower => {
                let am = ctx.am.unwrap();
                if c == conds::CHARGE && h.pre == Pre::Own && am.ty == Type::Electric {
                    ctx.chain_modify(2, 1);
                }
                R::Undef
            }
            Ev::ModifyAtk | Ev::ModifySpA | Ev::ModifyDef | Ev::ModifySpD if h.pre == Pre::Own => {
                if c == conds::FLASHFIRE {
                    let am = ctx.am.unwrap();
                    if (ev == Ev::ModifyAtk || ev == Ev::ModifySpA) && am.ty == Type::Fire && self.has_ability(p, abilities::FLASHFIRE) {
                        ctx.chain_modify(3, 2);
                    }
                } else if c == conds::PROTOSYNTHESIS || c == conds::QUARKDRIVE {
                    let want = match ev {
                        Ev::ModifyAtk => 0,
                        Ev::ModifyDef => 1,
                        Ev::ModifySpA => 2,
                        _ => 3,
                    };
                    if h.state.data == want && !self.ignoring_ability(p) {
                        ctx.chain_modify(5325, 4096);
                    }
                }
                R::Undef
            }
            Ev::ModifyCritRatio => {
                let R::Num(cr) = relay else { return R::Undef };
                if c == conds::FOCUSENERGY {
                    R::Num(cr + 2)
                } else if c == conds::DRAGONCHEER {
                    R::Num(cr + if h.state.data2 != 0 { 2 } else { 1 })
                } else {
                    R::Undef
                }
            }
            Ev::ModifyDamage => {
                let am = ctx.am.unwrap();
                if h.pre != Pre::Source {
                    return R::Undef;
                }
                let id = am.id.id();
                match c {
                    c if c == conds::DIG => {
                        if id == "earthquake" || id == "magnitude" {
                            ctx.chain_modify(2, 1);
                        }
                    }
                    c if c == conds::DIVE => {
                        if id == "surf" || id == "whirlpool" {
                            ctx.chain_modify(2, 1);
                        }
                    }
                    c if c == conds::FLY => {
                        if id == "gust" || id == "twister" {
                            ctx.chain_modify(2, 1);
                        }
                    }
                    c if c == conds::GLAIVERUSH => ctx.chain_modify(2, 1),
                    c if c == conds::MINIMIZE => {
                        if am.has_flag(flag::MINIMIZE) {
                            ctx.chain_modify(2, 1);
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::Damage => {
                if c == conds::ENDURE && ctx.eff.is_move() {
                    if let R::Num(d) = relay {
                        let hp = self.mon_at(p).hp as i32;
                        if d >= hp {
                            self.row(Row::VolatileActed { p: who(p), c, detail: Detail::None, of: None });
                            return R::Num(hp - 1);
                        }
                    }
                }
                R::Undef
            }
            Ev::TryPrimaryHit => {
                if c == conds::SUBSTITUTE {
                    return self.substitute_hit(p, ctx);
                }
                R::Undef
            }
            Ev::DamagingHit => {
                let am = ctx.am.unwrap();
                let src = ctx.source.unwrap();
                match c {
                    c if c == conds::FRZ => {
                        if am.ty == Type::Fire && am.category != Category::Status {
                            self.cure_status(p);
                        }
                    }
                    c if c == conds::COUNTER || c == conds::MIRRORCOAT => {
                        let want = if c == conds::COUNTER { Category::Physical } else { Category::Special };
                        if src.0 != p.0 && am.category == want {
                            let d = if let R::Num(x) = relay { x } else { ctx.arg as i32 };
                            if let Some(v) = self.s.sides[p.0].vol_mut(c) {
                                v.data = 1;
                                v.source = pk_code(src);
                                v.hp = (2 * d).min(i16::MAX as i32) as i16;
                            }
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::AfterMoveSecondary => {
                let am = ctx.am.unwrap();
                if c == conds::FRZ && am.data().thaws_target {
                    self.cure_status(p);
                }
                R::Undef
            }
            Ev::Faint => {
                if c == conds::DESTINYBOND {
                    // ctx.source: the faint's source; ctx.arg: 1 when its effect was a move
                    if let Some(src) = ctx.source {
                        if self.mon_at(src).hp > 0 && src.0 != p.0 && ctx.arg == 1 {
                            self.row(Row::VolatileActed { p: who(p), c, detail: Detail::None, of: Some(who(src)) });
                            self.faint_by(src, None, false);
                        }
                    }
                }
                R::Undef
            }
            Ev::SwitchIn => {
                if c == conds::TOX {
                    self.mon_at_mut(p).tox_stage = 0;
                }
                R::Undef
            }
            Ev::Update => {
                if c == conds::ATTRACT || c == conds::SYRUPBOMB {
                    let src = pk_of(h.state.source);
                    if src.map_or(false, |s| !self.is_active_pk(s)) && self.s.sides[p.0].has_vol(c) {
                        self.remove_volatile(p, c);
                    }
                }
                R::Undef
            }
            Ev::StallMove => {
                if c == conds::STALL {
                    let counter = self.s.sides[p.0].vol(c).map_or(1, |v| v.counter.max(1)) as u32;
                    let site = Site::new(Key::Protect, Some(p.0 as u8));
                    let ok = self.chance(site, 1, counter);
                    if !ok {
                        self.s.sides[p.0].delete_vol(c);
                    }
                    return R::of_bool(ok);
                }
                R::Undef
            }
            Ev::SetStatus => {
                if c == conds::UPROAR && h.pre == Pre::Any && ctx.arg == Status::Slp as u16 {
                    if let Some(t) = ctx.target {
                        self.row(Row::Fail { p: Some(who(t)), why: Fx::Cond(c) });
                    }
                    return R::Null;
                }
                R::Undef
            }
            Ev::ModifySpe => {
                if (c == conds::PROTOSYNTHESIS || c == conds::QUARKDRIVE) && h.pre == Pre::Own {
                    if h.state.data == 4 && !self.ignoring_ability(p) {
                        ctx.chain_modify(3, 2);
                    }
                    return R::Undef;
                }
                if c == conds::UNBURDEN && h.pre == Pre::Own {
                    if self.mon_at(p).item == ItemId::NONE && !self.ignoring_ability(p) {
                        ctx.chain_modify(2, 1);
                    }
                    return R::Undef;
                }
                if c == conds::PAR {
                    let R::Num(spe) = relay else { return R::Undef };
                    let spe = super::calc::modify(spe as u32, ctx.modifier);
                    ctx.modifier = 4096;
                    if self.has_ability(p, abilities::QUICKFEET) {
                        return R::Num(spe as i32);
                    }
                    return R::Num((spe * 50 / 100) as i32);
                }
                R::Undef
            }
            Ev::TryHeal => {
                if c == conds::HEALBLOCK {
                    return R::False;
                }
                R::Undef
            }
            Ev::DragOut => {
                if c == conds::INGRAIN {
                    self.row(Row::Blocked { p: Some(who(p)), by: Fx::Cond(c), what: Fx::None });
                    return R::Null;
                }
                R::Undef
            }
            Ev::Immunity => {
                // Dig and Dive: no weather damage underground or underwater
                let sand = super::field::imm_code(super::status::Imm::Cond(conds::SANDSTORM));
                let hail = super::field::imm_code(super::status::Imm::Cond(conds::HAIL));
                if (c == conds::DIG || c == conds::DIVE) && (ctx.arg == sand || ctx.arg == hail) {
                    return R::False;
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    /// The BeforeMove handlers (the can-it-move checks), in Showdown's priority order by the sort.
    fn before_move_handler(&mut self, p: Pk, c: Cond, h: &H, ctx: &mut Ctx) -> R {
        let am = ctx.am.unwrap();
        match c {
            c if c == conds::MUSTRECHARGE => {
                self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: MoveId::NONE });
                self.remove_volatile(p, c);
                self.remove_volatile(p, conds::TRUANT);
                R::Null
            }
            c if c == conds::SLP => {
                if self.has_ability(p, abilities::EARLYBIRD) {
                    self.mon_at_mut(p).status_time -= 1;
                }
                let m = self.mon_at_mut(p);
                m.status_time -= 1;
                if m.status_time <= 0 {
                    self.cure_status(p);
                    return R::Undef;
                }
                self.row(Row::Cant { p: who(p), why: Fx::Status(Status::Slp), mv: MoveId::NONE });
                if am.data().sleep_usable {
                    return R::Undef;
                }
                R::False
            }
            c if c == conds::FRZ => {
                if am.has_flag(flag::DEFROST) && !(am.id == moves::BURNUP && !self.has_type_pk(p, Type::Fire)) {
                    return R::Undef;
                }
                self.mon_at_mut(p).status_time -= 1;
                let t = self.mon_at(p).status_time;
                let thaw = t <= 0 || {
                    let site = Site::new(Key::Thaw, Some(p.0 as u8));
                    self.chance(site, 1, 4)
                };
                if thaw {
                    self.cure_status(p);
                    return R::Undef;
                }
                self.row(Row::Cant { p: who(p), why: Fx::Status(Status::Frz), mv: MoveId::NONE });
                R::False
            }
            c if c == conds::FLINCH => {
                self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: MoveId::NONE });
                if self.any_handler(Ev::Flinch) {
                    let mut ctx = Ctx::new(Some(p), None, Eff::None);
                    self.run_event(Ev::Flinch, &mut ctx, R::True);
                }
                R::False
            }
            c if c == conds::IMPRISON && h.pre == Pre::Foe => {
                let src = h.pk;
                let m = *self.mon_at(src);
                if am.id != moves::STRUGGLE && m.moves().iter().any(|ms| ms.id == am.id) {
                    // the row names the Pokémon that cannot use its move, not the Imprison user
                    let mover = ctx.target.unwrap_or(p);
                    self.row(Row::Cant { p: who(mover), why: Fx::Cond(c), mv: am.id });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::DISABLE => {
                if am.id == h.state.mv && !am.has_flag(flag::CANTUSETWICE) {
                    self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: am.id });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::THROATCHOP => {
                if am.has_flag(flag::SOUND) {
                    self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: MoveId::NONE });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::HEALBLOCK => {
                if am.has_flag(flag::HEAL) {
                    self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: am.id });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::TAUNT => {
                if am.category == Category::Status {
                    self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: am.id });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::CONFUSION => {
                let Some(v) = self.s.sides[p.0].vol_mut(c) else { return R::Undef };
                v.data -= 1;
                if v.data == 0 {
                    self.remove_volatile(p, c);
                    return R::Undef;
                }
                self.row(Row::VolatileActed { p: who(p), c, detail: Detail::None, of: None });
                let site = Site::new(Key::ConfusionHit, Some(p.0 as u8));
                if !self.chance(site, 33, 100) {
                    return R::Undef;
                }
                self.s.active_target = pk_code(p);
                let d = self.confusion_damage(p);
                self.damage(d as f64, p, Some(p), Eff::Confused);
                R::False
            }
            c if c == conds::ATTRACT => {
                let of = pk_of(h.state.source).map(who);
                self.row(Row::VolatileActed { p: who(p), c, detail: Detail::None, of });
                let site = Site { effect: Effect::One(EffectRef::Cond(c)), ..Site::new(Key::Proc, Some(p.0 as u8)) };
                if self.chance(site, 1, 2) {
                    self.row(Row::Cant { p: who(p), why: Fx::Cond(c), mv: MoveId::NONE });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::PAR => {
                let site = Site::new(Key::FullPara, Some(p.0 as u8));
                if self.chance(site, 1, 8) {
                    self.row(Row::Cant { p: who(p), why: Fx::Status(Status::Par), mv: MoveId::NONE });
                    return R::False;
                }
                R::Undef
            }
            c if c == conds::DESTINYBOND => {
                if am.id == moves::DESTINYBOND {
                    return R::Undef;
                }
                self.remove_volatile(p, c);
                R::Undef
            }
            c if c == conds::GLAIVERUSH => {
                self.remove_volatile(p, c);
                R::Undef
            }
            c if c == conds::CHILLYRECEPTION => {
                if am.id == moves::CHILLYRECEPTION {
                    self.row(Row::Once { p: Some(who(p)), fx: Fx::Move(moves::CHILLYRECEPTION), of: None });
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    /// Substitute's onTryPrimaryHit: the hit goes into the Substitute (HIT_SUBSTITUTE = 0)
    fn substitute_hit(&mut self, p: Pk, ctx: &mut Ctx) -> R {
        let mut am = ctx.am.unwrap();
        let src = ctx.source.unwrap();
        if src == p || am.has_flag(flag::BYPASSSUB) || am.infiltrates {
            return R::Undef;
        }
        let dmg = self.get_damage(src.0, src.1, p.0, p.1, &mut am);
        ctx.am = Some(am);
        // the unfloored damage, in half points
        let mut damage2 = match dmg {
            super::moves::Dmg::Hp(x) => am.raw_half.unwrap_or(2 * x) as i32,
            super::moves::Dmg::Fail | super::moves::Dmg::None => {
                self.row(Row::Fail { p: Some(who(src)), why: Fx::None });
                return R::Null;
            }
        };
        let sub_hp2 = self.s.sides[p.0].vol(conds::SUBSTITUTE).map_or(0, |v| v.hp) as i32;
        if damage2 > sub_hp2 {
            damage2 = sub_hp2;
        }
        if let Some(v) = self.s.sides[p.0].vol_mut(conds::SUBSTITUTE) {
            v.hp -= damage2 as i16;
        }
        let damage = damage2 as f64 / 2.0;
        self.mon_at_mut(src).last_damage = damage as u16;
        if self.s.sides[p.0].vol(conds::SUBSTITUTE).map_or(false, |v| v.hp <= 0) {
            if am.data().ohko != Ohko::No {
                self.row(Row::Result { how: How::Ohko, p: Some(who(p)) });
            }
            self.remove_volatile(p, conds::SUBSTITUTE);
        } else {
            self.row(Row::VolatileActed { p: who(p), c: conds::SUBSTITUTE, detail: Detail::None, of: None });
        }
        if damage2 > 0 {
            self.apply_recoil(src, damage, &am);
        }
        let dr = am.data().drain;
        if dr.1 > 0 {
            let amount = (damage * dr.0 as f64 / dr.1 as f64).ceil();
            self.heal(amount, src, Some(p), Eff::Drain);
        }
        self.move_after_sub_damage(p, src, &am);
        // runEvent('AfterSubDamage', target, source, move, damage): Air Balloon
        if self.any_handler(Ev::AfterSubDamage) {
            let mut c2 = Ctx::with_move(Some(p), Some(src), &am);
            c2.arg = damage as u16;
            self.run_event(Ev::AfterSubDamage, &mut c2, R::Num(damage as i32));
        }
        R::Num(0)
    }

    // ------------------------------------------------------------------------------------------ Residual

    /// The onResidual callback of a status or volatile (after its duration counted down).
    pub fn cond_residual(&mut self, p: Pk, c: Cond) {
        let m = *self.mon_at(p);
        let eff = Eff::Cond(c);
        match c {
            c if c == conds::BRN => {
                self.damage(m.base_maxhp as f64 / 16.0, p, None, eff);
            }
            c if c == conds::PSN => {
                self.damage(m.base_maxhp as f64 / 8.0, p, None, eff);
            }
            c if c == conds::TOX => {
                if m.tox_stage < 15 {
                    self.mon_at_mut(p).tox_stage += 1;
                }
                let stage = self.mon_at(p).tox_stage as f64;
                let per = (m.base_maxhp as f64 / 16.0).floor().max(1.0);
                self.damage(per * stage, p, None, eff);
            }
            c if c == conds::PARTIALLYTRAPPED || c == conds::OCTOLOCK => {
                let v = self.s.sides[p.0].vol(c).copied().unwrap();
                if let Some(src) = pk_of(v.source) {
                    let sm = self.mon_at(src);
                    if !self.is_active_pk(src) || sm.hp == 0 || sm.active_turns == 0 {
                        self.s.sides[p.0].delete_vol(c);
                        return;
                    }
                }
                if c == conds::PARTIALLYTRAPPED {
                    self.damage(m.base_maxhp as f64 / v.data.max(1) as f64, p, None, eff);
                } else {
                    let src = pk_of(v.source);
                    self.boost(&[(1, -1), (3, -1)], p, src, Eff::Move(moves::OCTOLOCK), false, false);
                }
            }
            c if c == conds::LOCKEDMOVE => {
                if m.status == Status::Slp {
                    self.s.sides[p.0].delete_vol(c);
                }
                // the state object lives on after a delete; only a present lock matters later
                if let Some(v) = self.s.sides[p.0].vol_mut(c) {
                    v.data -= 1;
                }
            }
            c if c == conds::CURSE => {
                self.damage(m.base_maxhp as f64 / 4.0, p, None, eff);
            }
            c if c == conds::ENCORE => {
                let mv = self.s.sides[p.0].vol(c).unwrap().mv;
                let slot = m.moves().iter().find(|ms| ms.id == mv);
                if slot.map_or(true, |s| s.pp == 0) {
                    self.remove_volatile(p, c);
                }
            }
            c if c == conds::SALTCURE => {
                let d = if self.has_type_pk(p, Type::Water) || self.has_type_pk(p, Type::Steel) { 8.0 } else { 16.0 };
                self.damage(m.base_maxhp as f64 / d, p, None, eff);
            }
            c if c == conds::SYRUPBOMB => {
                let src = pk_of(self.s.sides[p.0].vol(c).unwrap().source);
                self.boost(&[(4, -1)], p, src, eff, false, false);
            }
            c if c == conds::AQUARING || c == conds::INGRAIN => {
                self.heal(m.base_maxhp as f64 / 16.0, p, None, eff);
            }
            c if c == conds::LEECHSEED => {
                let src = self.s.sides[p.0].vol(c).unwrap().source;
                let Some(sp) = pk_of(src) else { return };
                let side = sp.0;
                let sd = &self.s.sides[side];
                if !sd.has_active {
                    return;
                }
                let target = (side, sd.active as usize);
                let tm = self.mon_at(target);
                if tm.fainted || tm.hp == 0 {
                    return;
                }
                if let Some(d) = self.damage(m.base_maxhp as f64 / 8.0, p, Some(target), eff) {
                    if d > 0 {
                        self.heal(d as f64, target, Some(p), eff);
                    }
                }
            }
            c if c == conds::PERISHSONG => {
                let d = self.s.sides[p.0].vol(c).map_or(0, |v| v.duration);
                self.row(Row::Volatile { p: who(p), c, detail: Detail::Num(d), by: Fx::None, of: None });
            }
            c if c == conds::UPROAR => {
                if self.s.sides[p.0].has_vol(conds::THROATCHOP) {
                    self.remove_volatile(p, c);
                    return;
                }
                if m.last_move == moves::STRUGGLE {
                    self.s.sides[p.0].delete_vol(c);
                    return;
                }
                self.row(Row::Once { p: Some(who(p)), fx: Fx::Cond(c), of: None });
            }
            _ => {}
        }
    }
}
