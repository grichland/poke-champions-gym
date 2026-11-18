//! Using a move, in Showdown's steps (sim/battle-actions.ts runMove, useMove, useMoveInner, trySpreadMoveHit,
//! tryMoveHit, the hit steps, getDamage, modifyDamage; data/mods/champions/scripts.ts spreadMoveHit and
//! hitStepMoveHitLoop): can it move, PP and "used", targets, immunity, accuracy, the hits with per-hit crit
//! and roll, damage, the move's effects, self effects, secondaries, forced switches, recoil, faints.
//! Moves' own handlers are in `movefx.rs`, conditions' in `conds.rs`.

use super::battle::{Sim, RECHARGE, STRUGGLE};
use super::calc::*;
use super::data::*;
use super::events::*;
use super::hooks::*;
use super::outcome::*;
use super::state::*;
use super::log::*;

/// A secondary effect a handler appended to the move's list (Stench and King's Rock: a 10% flinch; Fling: the
/// flung item's status or volatile, always).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ExtraSec {
    pub chance: Option<u8>,
    pub status: Status,
    pub vol: Cond,
}

/// Showdown's ActiveMove: the move's data as this use of it sees it (handlers may change it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveMove {
    pub id: MoveId,
    /// which use of a move this is (battle.activeMove is the same object only while uids match)
    pub uid: u32,
    pub ty: Type,
    pub category: Category,
    pub base_power: u32,
    pub target: Target,
    pub flags: u64,
    pub priority: i8,
    pub ignore_immunity: bool,
    pub crit_ratio: u8,
    /// (min, max) hits; Beat Up sets it
    pub multihit: (u8, u8),
    /// the hit being made, from 1
    pub hit: u8,
    pub total_damage: u32,
    pub self_dropped: bool,
    pub infiltrates: bool,
    /// the move that called this one (Sleep Talk, Copycat) or Round when prioritized
    pub source_effect: MoveId,
    /// run as a locked move (Showdown's sourceEffect lockedmove)
    pub locked: bool,
    /// Clangorous Soul's onTryHit deleted the boosts
    pub no_boosts: bool,
    /// Parting Shot's onHit deleted selfSwitch
    pub no_self_switch: bool,
    /// moveHitData crit of the current target
    pub crit: bool,
    /// move.hitTargets: None before trySpreadMoveHit set it, else whether the target was hit
    pub hit_targets: Option<bool>,
    /// Beat Up: the base Attack of each ally still to hit
    pub allies: [u8; TEAM],
    pub nallies: u8,
    /// Shell Side Arm turned physical
    pub physical_ssa: bool,
    /// twice the last damageCallback result (Metal Burst's 1.5x can be a half point; a Substitute keeps it)
    pub raw_half: Option<u32>,
    /// 1..=100, or ACC_ALWAYS (Thunder, Hurricane and Blizzard change it by weather)
    pub accuracy: u8,
    /// overrideOffensiveStat (Wonder Room swaps Def and SpD)
    pub override_offensive: Option<Stat>,
    /// the boosts of the move's own effect when a handler replaced them (Growth in sun)
    pub boosts: Option<[i8; 7]>,
    /// a future attack's hit (Future Sight): the stored move data, without the move's handlers
    pub future: bool,
    /// ignoreAbility: the target's breakable abilities do not act (Mold Breaker, or the move's data)
    pub ignore_ability: bool,
    /// pranksterBoosted
    pub prankster: bool,
    /// typeChangerBoosted: the ability that changed the move's type
    pub type_changer: AbilityId,
    /// hasSheerForce: secondaries and the move's self effect deleted
    pub sheer_force: bool,
    /// Serene Grace doubled the secondaries' chances
    pub sec_double: bool,
    /// secondaries appended after the move's own (which Sheer Force may have deleted)
    pub extra: [ExtraSec; 2],
    pub nextra: u8,
    /// Fling: the item thrown (its onHit effect)
    pub fling_item: ItemId,
    /// ignoreImmunity for Fighting and Normal moves (Scrappy, Mind's Eye)
    pub scrappy: bool,
    pub ignore_evasion: bool,
    pub multiaccuracy: bool,
    /// a Magic Bounce reflection
    pub has_bounced: bool,
    /// multihitType parentalbond
    pub parental_bond: bool,
    /// moveHitData bypassProtect (Piercing Drill, Unseen Fist): a quarter of the damage
    pub bypass_protect: bool,
    /// moveHitData typeMod of the current hit
    pub type_mod: i8,
    /// auraBooster (Fairy Aura, Dark Aura): the holder that boosts, or NO_MON
    pub aura_booster: u8,
    /// ruinedAtk, ruinedDef, ruinedSpA, ruinedSpD: the Ruin ability holder that weakens, or NO_MON
    pub ruined: [u8; 4],
    /// hasAuraBreak
    pub aura_break: bool,
}

impl ActiveMove {
    pub fn new(id: MoveId) -> ActiveMove {
        let d = id.data();
        ActiveMove {
            id,
            uid: 0,
            ty: d.ty,
            category: d.category,
            base_power: d.base_power as u32,
            target: d.target,
            flags: d.flags,
            priority: d.priority,
            ignore_immunity: d.ignore_immunity,
            crit_ratio: d.crit_ratio,
            multihit: d.multihit,
            hit: 0,
            total_damage: 0,
            self_dropped: false,
            infiltrates: false,
            source_effect: MoveId::NONE,
            locked: false,
            no_boosts: false,
            no_self_switch: false,
            crit: false,
            hit_targets: None,
            allies: [0; TEAM],
            nallies: 0,
            physical_ssa: false,
            raw_half: None,
            accuracy: d.accuracy,
            override_offensive: d.override_offensive_stat,
            boosts: d.boosts,
            future: false,
            ignore_ability: d.ignore_ability,
            prankster: false,
            type_changer: AbilityId::NONE,
            sheer_force: false,
            sec_double: false,
            extra: [ExtraSec::default(); 2],
            nextra: 0,
            fling_item: ItemId::NONE,
            scrappy: false,
            ignore_evasion: d.ignore_evasion,
            multiaccuracy: d.multiaccuracy,
            has_bounced: false,
            parental_bond: false,
            bypass_protect: false,
            type_mod: 0,
            aura_booster: NO_MON,
            ruined: [NO_MON; 4],
            aura_break: false,
        }
    }
    #[inline]
    pub fn data(&self) -> &'static MoveData {
        self.id.data()
    }
    #[inline]
    pub fn has_flag(&self, f: u64) -> bool {
        self.flags & f != 0
    }
    /// move.secondaries is non-empty
    pub fn has_secondaries(&self) -> bool {
        (!self.sheer_force && !self.data().secondaries.is_empty()) || self.nextra > 0
    }
    /// a secondary with volatileStatus flinch is in move.secondaries
    pub fn has_flinch_secondary(&self) -> bool {
        (!self.sheer_force && self.data().secondaries.iter().any(|s| s.volatile_status == conds::FLINCH))
            || self.extra[..self.nextra as usize].iter().any(|x| x.vol == conds::FLINCH)
    }
    /// move.secondaries.push(...)
    pub fn push_secondary(&mut self, x: ExtraSec) {
        if (self.nextra as usize) < self.extra.len() {
            self.extra[self.nextra as usize] = x;
            self.nextra += 1;
        }
    }
}

/// getDamage's results: a number of HP, nothing (undefined), or a failure (false or null).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dmg {
    Fail,
    None,
    Hp(u32),
}

/// The draws and inputs of one hit, for callers that compute damage without the outcome source.
#[derive(Clone, Copy, Debug)]
pub struct HitDraws {
    pub crit: bool,
    pub roll: u32,
}

/// Which hit effect (Showdown's moveData) a spreadMoveHit call applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Md {
    Primary,
    Secondary(u8),
    /// the move's `self`
    SelfOf,
    /// a secondary's `self`
    SecSelf(u8),
    /// an appended secondary (ActiveMove::extra)
    Extra(u8),
    /// the move's `selfBoost`
    SelfBoost,
}

/// A spreadMoveHit target: a Pokémon, null (hit a Substitute) or false (no longer hit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum T {
    P(Pk),
    Null,
    False,
}

/// Showdown's combineResults.
pub fn combine(l: R, r: R) -> R {
    fn rank(x: R) -> u8 {
        match x {
            R::Undef => 0,
            R::NotFail => 1,
            R::Null => 2,
            R::False | R::True => 3,
            R::Num(_) | R::Mv(_) => 4,
        }
    }
    if rank(l) > rank(r) {
        return l;
    }
    if l.truthy() && !r.truthy() && r != R::Num(0) {
        return l;
    }
    if let (R::Num(a), R::Num(b)) = (l, r) {
        return R::Num(a + b);
    }
    r
}

/// The target a move action resolves to in singles (getTarget).
pub fn field_like(t: Target) -> bool {
    matches!(t, Target::All | Target::FoeSide | Target::AllySide | Target::AllyTeam)
}

/// The boosts of a hit effect, in the data's key order (only the order of the generated table matters).
fn boost_list(b: &[i8; 7]) -> ([(usize, i8); 7], usize) {
    let mut out = [(0usize, 0i8); 7];
    let mut n = 0;
    for (i, &v) in b.iter().enumerate() {
        if v != 0 {
            out[n] = (i, v);
            n += 1;
        }
    }
    (out, n)
}

impl<'a, O: Outcomes> Sim<'a, O> {
    /// randomChance(num, den): a draw only when the outcome is uncertain
    #[inline]
    pub fn chance(&mut self, site: Site, num: u32, den: u32) -> bool {
        if num >= den {
            true
        } else if num == 0 {
            false
        } else {
            self.o.chance(&site, num, den)
        }
    }

    /// battle.setActiveMove(move, pokemon, target): activeTarget is the target or the user
    pub fn set_active(&mut self, am: &ActiveMove, user: Pk, target: Option<Pk>) {
        self.active = Some(*am);
        self.s.active_move = am.id;
        self.s.active_pokemon = pk_code(user);
        self.s.active_target = pk_code(target.unwrap_or(user));
    }

    /// keeps battle.activeMove's copy up to date with changes to the same use of a move
    pub fn sync_active(&mut self, am: &ActiveMove) {
        if let Some(a) = self.active.as_mut() {
            if a.uid == am.uid {
                *a = *am;
            }
        }
    }

    /// a new ActiveMove (dex.getActiveMove) with its own uid
    pub fn new_active(&mut self, id: MoveId) -> ActiveMove {
        let mut am = ActiveMove::new(id);
        self.s.active_uid = self.s.active_uid.wrapping_add(1);
        am.uid = self.s.active_uid;
        am
    }

    /// battle.clearActiveMove(failed)
    pub fn clear_active(&mut self, failed: bool) {
        if let Some(am) = self.active {
            if !failed {
                self.s.last_move = am.id;
            }
            self.active = None;
            self.s.active_move = MoveId::NONE;
            self.s.active_pokemon = NO_MON;
            self.s.active_target = NO_MON;
        }
    }

    /// battle.getTarget for an action's move in singles
    pub fn get_target(&self, user: Pk, t: Target) -> Option<Pk> {
        match t {
            Target::Self_ | Target::Allies | Target::All | Target::AllySide | Target::AllyTeam | Target::AdjacentAllyOrSelf => Some(user),
            Target::AdjacentAlly => None,
            _ => Some(self.foe_of(user)),
        }
    }

    /// battle.getRandomTarget in singles
    pub fn random_target(&self, user: Pk, t: Target) -> Option<Pk> {
        self.get_target(user, t)
    }

    // ------------------------------------------------------------------------------------------ runMove

    /// battleActions.runMove
    pub fn run_move(&mut self, a: Action) {
        let user = (a.side as usize, a.slot as usize);
        self.mon_at_mut(user).active_move_actions += 1;
        let mv = if a.move_index == STRUGGLE {
            moves::STRUGGLE
        } else if a.move_index == RECHARGE {
            MoveId::NONE
        } else {
            a.mv
        };
        let mut am = self.new_active(mv);
        // the action's priority as getActionSpeed set it (Grassy Glide, Prankster), without the fraction
        am.priority = ((a.priority - a.frac as i16) / 10) as i8;
        am.prankster = a.prankster;
        let mut target = self.get_target(user, am.target);
        // getTarget with tracksTarget (Stalwart): the Pokémon the action was aimed at, while it is in
        if let Some(o) = pk_of(a.orig_target) {
            if self.is_active_pk(o) && self.has_ability(user, abilities::STALWART) {
                target = Some(o);
            }
        }
        if mv != moves::STRUGGLE {
            run_field_hook(self.s, Hook::OverrideAction);
        }
        // Champions: a Curse resolved while the user was not a Ghost targets the user
        let curse_self = mv == moves::CURSE && a.curse_self;
        if curse_self {
            am.target = Target::Self_;
            target = Some(user);
        }
        self.set_active(&am, user, target);
        let mut ctx = Ctx::with_move(Some(user), target, &am);
        let will = self.run_event(Ev::BeforeMove, &mut ctx, R::True);
        if !will.truthy() {
            let mut ctx = Ctx::with_move(Some(user), target, &am);
            self.run_event(Ev::MoveAborted, &mut ctx, R::True);
            self.clear_active(true);
            self.mon_at_mut(user).move_this_turn_result = if will == R::Null { MoveResult::Null } else { MoveResult::False };
            return;
        }
        if self.before_move_callback(user, &am) {
            self.clear_active(true);
            self.mon_at_mut(user).move_this_turn_result = MoveResult::False;
            return;
        }
        self.mon_at_mut(user).last_damage = 0;
        let locked = self.locked_move(user).is_some();
        if !locked && !self.deduct_pp(user, mv, 1) && mv != moves::STRUGGLE {
            self.row(Row::Cant { p: who(user), why: Fx::None, mv });
            self.clear_active(true);
            self.mon_at_mut(user).move_this_turn_result = MoveResult::False;
            return;
        }
        {
            let m = self.mon_at_mut(user);
            m.last_move = mv;
            m.move_this_turn = mv;
            m.last_move_tloc = a.tloc;
        }
        self.curse_self_next = curse_self;
        let did = self.use_move(user, mv, if target.is_some() { Tgt::Some(target.unwrap()) } else { Tgt::None }, MoveId::NONE, locked, a.source_effect);
        let fin = self.active.unwrap_or(am);
        self.move_after_move(user, target, &fin);
        let mut ctx = Ctx::with_move(Some(user), target, &fin);
        self.run_event(Ev::AfterMove, &mut ctx, R::True);
        // Dancer: the foe dances the same dance
        if fin.has_flag(flag::DANCE) && did.truthy() {
            let foe = self.foe_of(user);
            if self.is_active_pk(foe) && self.has_ability(foe, abilities::DANCER) && !self.semi_invulnerable(foe) {
                if !self.resolve_faints(false, false, true) && !self.mon_at(foe).fainted {
                    self.ab_row(foe, abilities::DANCER);
                    self.run_move_external(foe, mv, user);
                }
            }
        }
        self.resolve_faints(false, false, true);
        self.check_win(None);
    }

    /// runMove(move, pokemon, targetLoc, {externalMove: true}) for Dancer: no PP, no lastMove, no lock
    pub fn run_move_external(&mut self, user: Pk, mv: MoveId, target: Pk) {
        self.mon_at_mut(user).active_move_actions += 1;
        let am = self.new_active(mv);
        let target = Some(target);
        self.set_active(&am, user, target);
        let mut ctx = Ctx::with_move(Some(user), target, &am);
        let will = self.run_event(Ev::BeforeMove, &mut ctx, R::True);
        if !will.truthy() {
            let mut ctx = Ctx::with_move(Some(user), target, &am);
            self.run_event(Ev::MoveAborted, &mut ctx, R::True);
            self.clear_active(true);
            self.mon_at_mut(user).move_this_turn_result = if will == R::Null { MoveResult::Null } else { MoveResult::False };
            return;
        }
        if self.before_move_callback(user, &am) {
            self.clear_active(true);
            self.mon_at_mut(user).move_this_turn_result = MoveResult::False;
            return;
        }
        self.mon_at_mut(user).last_damage = 0;
        let had_lock = self.has_vol(user, conds::LOCKEDMOVE);
        self.external_next = true;
        self.use_move(user, mv, Tgt::Some(target.unwrap()), MoveId::NONE, false, MoveId::NONE);
        let fin = self.active.unwrap_or(am);
        self.move_after_move(user, target, &fin);
        let mut ctx = Ctx::with_move(Some(user), target, &fin);
        self.run_event(Ev::AfterMove, &mut ctx, R::True);
        if !had_lock && self.is_active_pk(user) {
            self.s.sides[user.0].delete_vol(conds::LOCKEDMOVE);
        }
    }

    /// pokemon.deductPP: false when the move is not in a slot or has no PP
    pub fn deduct_pp(&mut self, p: Pk, mv: MoveId, amount: u8) -> bool {
        self.deduct_pp_amount(p, mv, amount) > 0
    }

    pub fn deduct_pp_amount(&mut self, p: Pk, mv: MoveId, amount: u8) -> u8 {
        let m = self.mon_at_mut(p);
        let n = m.nmoves as usize;
        let Some(ms) = m.moves[..n].iter_mut().find(|x| x.id == mv) else { return 0 };
        ms.used = true;
        if ms.pp == 0 {
            return 0;
        }
        let a = amount.min(ms.pp);
        ms.pp -= a;
        a
    }

    /// pokemon.getLockedMove (priorityEvent LockMove): Some(NONE) for Recharge
    pub fn locked_move(&mut self, p: Pk) -> Option<MoveId> {
        if !self.is_active_pk(p) {
            return None;
        }
        self.s.locked_move(p.0)
    }

    // ------------------------------------------------------------------------------------------ useMove

    /// battleActions.useMove: moveThisTurnResult follows the inner result unless the move set it
    pub fn use_move(&mut self, user: Pk, mv: MoveId, target: Tgt, caller: MoveId, locked: bool, action_source: MoveId) -> R {
        self.mon_at_mut(user).move_this_turn_result = MoveResult::Undefined;
        let r = self.use_move_inner(user, mv, target, caller, locked, action_source);
        let m = self.mon_at_mut(user);
        if m.move_this_turn_result == MoveResult::Undefined {
            m.move_this_turn_result = match r {
                R::True | R::Num(_) | R::Mv(_) => MoveResult::True,
                R::False => MoveResult::False,
                R::Undef => MoveResult::Undefined,
                R::Null | R::NotFail => MoveResult::Null,
            };
        }
        r
    }

    /// battleActions.useMoveInner
    fn use_move_inner(&mut self, user: Pk, mv: MoveId, target_in: Tgt, caller: MoveId, locked: bool, action_source: MoveId) -> R {
        let mut am = self.new_active(mv);
        if std::mem::take(&mut self.curse_self_next) && mv == moves::CURSE {
            am.target = Target::Self_;
        }
        // a move with any handler that has no implementation makes the battle inexact
        let un = masks().moves_unimpl[mv.0 as usize];
        if un != 0 {
            self.note(EffectRef::Move(mv), hook_of(un.trailing_zeros() as u8));
        }
        am.locked = locked;
        am.source_effect = if caller != MoveId::NONE { caller } else { action_source };
        let bounced = std::mem::take(&mut self.bounce_next);
        // a move called by an ability (Magic Bounce, Dancer) pays no Pressure PP
        let by_ability = bounced | std::mem::take(&mut self.external_next);
        am.has_bounced = bounced;
        if let Some(prev) = self.active {
            am.priority = prev.priority;
            if !am.has_bounced {
                am.prankster = prev.prankster;
            }
        }
        self.mon_at_mut(user).last_move_used = mv;
        let base_target = am.target;
        let mut target = match target_in {
            Tgt::Some(p) => Some(p),
            Tgt::None => None,
            Tgt::Unset => self.random_target(user, am.target),
        };
        if let Some(t) = self.move_modify_target(user, &am) {
            target = Some(t);
        }
        if matches!(am.target, Target::Self_ | Target::Allies) {
            target = Some(user);
        }
        self.set_active(&am, user, target);
        self.move_modify_type(user, &mut am);
        self.move_modify_move(user, target, &mut am);
        if base_target != am.target {
            target = self.random_target(user, am.target);
        }
        let mut ctx = Ctx::with_move(Some(user), target, &am);
        self.run_event(Ev::ModifyType, &mut ctx, R::True);
        if let Some(x) = ctx.am {
            am = x;
        }
        run_field_hook(self.s, Hook::ModifyType);
        let mut ctx = Ctx::with_move(Some(user), target, &am);
        let r = self.run_event(Ev::ModifyMove, &mut ctx, R::True);
        if let Some(x) = ctx.am {
            am = x;
        }
        run_field_hook(self.s, Hook::ModifyMove);
        self.sync_active(&am);
        if base_target != am.target {
            target = self.random_target(user, am.target);
        }
        if !r.truthy() || self.mon_at(user).fainted {
            return R::False;
        }
        if self.logging() {
            let called_by = if caller != MoveId::NONE {
                Fx::Move(caller)
            } else if bounced {
                Fx::Ability(abilities::MAGICBOUNCE)
            } else if by_ability {
                Fx::Ability(abilities::DANCER)
            } else {
                Fx::None
            };
            self.row(Row::Move { p: who(user), mv: am.id, target: target.map(who), called_by, priority: am.priority });
        }
        let Some(target) = target else {
            self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
            return R::False;
        };
        let targets = self.move_targets(user, &mut am, target);
        let tgt0 = targets.unwrap_or(target);
        // Pressure: runEvent('DeductPP') on each pressure target; a locked or bounced move pays nothing
        if !locked && !by_ability && self.any_handler(Ev::DeductPP) {
            let mut extra = 0u8;
            for pt in self.pressure_targets(user, &am, targets) {
                let mut ctx = Ctx::with_move(Some(pt), Some(user), &am);
                if let R::Num(x) = self.run_event_chain(Ev::DeductPP, &mut ctx, R::True) {
                    extra = extra.saturating_add(x.max(0) as u8);
                }
            }
            if extra > 0 {
                let pay = if caller != MoveId::NONE { caller } else { mv };
                self.deduct_pp_amount(user, pay, extra);
            }
        }
        let r = self.move_try_move(user, tgt0, &mut am);
        if !r.truthy() && r != R::Undef {
            self.sync_active(&am);
            return r;
        }
        if self.any_handler(Ev::TryMove) {
            let mut ctx = Ctx::with_move(Some(user), Some(tgt0), &am);
            let r = self.run_event(Ev::TryMove, &mut ctx, R::True);
            if !r.truthy() {
                self.sync_active(&am);
                return r;
            }
        }
        run_field_hook(self.s, Hook::TryMove);
        if am.data().selfdestruct == SelfDestruct::Always {
            self.faint_by(user, Some(user), true);
        }
        let move_result;
        if field_like(am.target) {
            let d = self.try_move_hit(user, tgt0, &mut am);
            if d == R::NotFail {
                self.mon_at_mut(user).move_this_turn_result = MoveResult::Null;
            }
            move_result = d.truthy() || d == R::Num(0) || d == R::Undef;
        } else {
            let Some(t) = targets else {
                self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
                self.sync_active(&am);
                return R::False;
            };
            move_result = self.try_spread_move_hit(user, t, &mut am);
        }
        if am.data().self_boost.is_some() && move_result {
            self.move_hit(T::P(user), user, &mut am, Md::SelfBoost, false, true);
        }
        if self.mon_at(user).hp == 0 {
            self.faint_by(user, Some(user), true);
        }
        self.sync_active(&am);
        if !move_result {
            let orig = self.mon_at(user).hp;
            self.move_fail(user, tgt0, &am);
            if user != tgt0 && am.category != Category::Status {
                self.emergency_exit(user, Some(user), orig);
            }
            return R::False;
        }
        if !self.suppressing_secondaries() && !am.has_flag(flag::FUTUREMOVE) {
            let orig = self.mon_at(user).hp;
            self.move_after_move_secondary_self(user, tgt0, &am);
            // runEvent('AfterMoveSecondarySelf', pokemon, target, move): Life Orb, Shell Bell, Magician
            if self.any_handler(Ev::AfterMoveSecondarySelf) {
                let mut ctx = Ctx::with_move(Some(user), Some(target), &am);
                self.run_event(Ev::AfterMoveSecondarySelf, &mut ctx, R::True);
            }
            if user != tgt0 && am.category != Category::Status {
                self.emergency_exit(user, Some(user), orig);
            }
        }
        R::True
    }

    /// getMoveTargets' pressureTargets in singles: the active foe for a move aimed at it or at everyone
    fn pressure_targets(&self, user: Pk, am: &ActiveMove, targets: Option<Pk>) -> Vec<Pk> {
        let foe = self.foe_of(user);
        let foe_ok = self.s.sides[foe.0].has_active && self.mon_at(foe).is_active;
        if am.has_flag(flag::MUSTPRESSURE) {
            return if foe_ok && self.mon_at(foe).hp > 0 { vec![foe] } else { vec![] };
        }
        match am.target {
            Target::FoeSide => vec![],
            Target::All => {
                let mut v = vec![user];
                if foe_ok {
                    v.push(foe);
                }
                v
            }
            Target::AllySide | Target::AllyTeam | Target::Allies => vec![user],
            _ => targets.map(|t| vec![t]).unwrap_or_default(),
        }
    }

    /// pokemon.getMoveTargets in singles: None when the move has no target left
    fn move_targets(&mut self, user: Pk, am: &mut ActiveMove, target: Pk) -> Option<Pk> {
        let foe = self.foe_of(user);
        match am.target {
            Target::All | Target::FoeSide | Target::AllySide | Target::AllyTeam => {
                if self.mon_at(user).hp > 0 || am.target == Target::FoeSide {
                    Some(if am.target == Target::FoeSide { foe } else { user })
                } else if !matches!(am.target, Target::AllySide | Target::AllyTeam) {
                    Some(foe)
                } else {
                    None
                }
            }
            Target::AllAdjacent | Target::AllAdjacentFoes => {
                if self.s.sides[foe.0].has_active && self.mon_at(foe).hp > 0 {
                    Some(foe)
                } else {
                    None
                }
            }
            Target::Allies => {
                if self.mon_at(user).hp > 0 {
                    Some(user)
                } else {
                    None
                }
            }
            _ => {
                let mut t = target;
                if t.0 != user.0 && self.mon_at(t).fainted {
                    t = self.random_target(user, am.target)?;
                }
                // a future attack is aimed at the slot: a fainted target keeps it
                if self.mon_at(t).fainted && !am.has_flag(flag::FUTUREMOVE) {
                    return None;
                }
                Some(t)
            }
        }
    }

    /// battleActions.tryMoveHit (moves on the field or a side)
    fn try_move_hit(&mut self, user: Pk, target: Pk, am: &mut ActiveMove) -> R {
        self.set_active(am, user, Some(target));
        let r = self.move_try(user, target, am);
        if !r.truthy() && r != R::Undef {
            if r == R::False {
                self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
            }
            return R::False;
        }
        let r = self.move_prepare_hit(user, target, am);
        if !r.truthy() && r != R::Undef {
            if r == R::False {
                self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
            }
            return R::False;
        }
        if !self.prepare_hit_event(user, target, am) {
            self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
            return R::False;
        }
        if matches!(am.target, Target::FoeSide | Target::AllySide | Target::AllyTeam) && self.any_handler(Ev::TryHitSide) {
            let t = if am.target == Target::FoeSide { self.foe_of(user) } else { user };
            let mut ctx = Ctx::with_move(Some(t), Some(user), am);
            let r = self.run_event(Ev::TryHitSide, &mut ctx, R::True);
            if !r.truthy() {
                return R::False;
            }
        }
        self.move_hit(T::P(target), user, am, Md::Primary, false, false)
    }

    /// runEvent('PrepareHit', pokemon, target, move): Protean, Libero, Parental Bond
    pub fn prepare_hit_event(&mut self, user: Pk, target: Pk, am: &mut ActiveMove) -> bool {
        if !self.any_handler(Ev::PrepareHit) {
            return true;
        }
        let mut ctx = Ctx::with_move(Some(user), Some(target), am);
        let r = self.run_event(Ev::PrepareHit, &mut ctx, R::True);
        if let Some(x) = ctx.am {
            *am = x;
        }
        self.sync_active(am);
        r.truthy()
    }

    // ------------------------------------------------------------------------------------------ hit steps

    /// battleActions.trySpreadMoveHit for the one target of a singles move
    pub fn try_spread_move_hit(&mut self, user: Pk, t: Pk, am: &mut ActiveMove) -> bool {
        let r = self.move_try(user, t, am);
        let r = if r.truthy() || r == R::Undef { self.move_prepare_hit(user, t, am) } else { r };
        let r = if r.truthy() || r == R::Undef { R::of_bool(self.prepare_hit_event(user, t, am)) } else { r };
        if !(r.truthy() || r == R::Undef) {
            if r == R::False {
                self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
            }
            return r == R::NotFail;
        }
        run_field_hook(self.s, Hook::PrepareHit);
        let mut failure = false;
        // 0. semi-invulnerability
        let inv = if am.id == moves::HELPINGHAND {
            R::True
        } else if am.id == moves::TOXIC && self.has_type_pk(user, Type::Poison) {
            R::True
        } else {
            let mut ctx = Ctx::with_move(Some(t), Some(user), am);
            self.run_event(Ev::Invulnerability, &mut ctx, R::True)
        };
        if inv == R::False {
            self.row(Row::Miss { p: who(t), by: who(user) });
            return self.step_fail(user, true);
        }
        // 1. TryHit (Protect)
        let mut ctx = Ctx::with_move(Some(t), Some(user), am);
        let mut th = self.run_event(Ev::TryHit, &mut ctx, R::True);
        if th == R::False {
            self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
        }
        if let Some(x) = ctx.am {
            am.bypass_protect |= x.bypass_protect;
        }
        self.sync_active(am);
        run_field_hook(self.s, Hook::TryHit);
        if th != R::NotFail && !th.truthy() {
            th = R::False;
        }
        if th == R::False {
            failure = true;
        }
        if !(th.truthy() || th == R::Num(0)) {
            return self.step_fail(user, failure);
        }
        // 2. type immunity
        if !self.run_immunity(t, am) {
            if self.logging() {
                self.immune_rows(t, am);
            }
            return self.step_fail(user, true);
        }
        // 3. move-specific immunities
        if am.has_flag(flag::POWDER) && t != user && !self.type_immunity(t, super::status::Imm::Powder) {
            self.row(Row::Result { how: How::Immune, p: Some(who(t)) });
            return self.step_fail(user, true);
        }
        if !self.move_try_immunity(user, t, am) {
            self.row(Row::Result { how: How::Immune, p: Some(who(t)) });
            return self.step_fail(user, true);
        }
        // Prankster: Dark types are immune to the boosted move
        if am.prankster && self.has_ability(user, abilities::PRANKSTER) && t.0 != user.0 && self.has_type_pk(t, Type::Dark) {
            self.row(Row::Result { how: How::Immune, p: Some(who(t)) });
            return self.step_fail(user, true);
        }
        // 4. accuracy
        if !self.hit_step_accuracy(user, t, am) {
            return self.step_fail(user, true);
        }
        // 5. break protection
        if am.data().breaks_protect {
            let mut broke = false;
            for c in [conds::BANEFULBUNKER, conds::KINGSSHIELD, conds::PROTECT, conds::SPIKYSHIELD] {
                if self.remove_volatile(t, c) {
                    broke = true;
                }
            }
            for c in [conds::QUICKGUARD, conds::WIDEGUARD] {
                if self.remove_side_cond(t.0, c) {
                    broke = true;
                }
            }
            if broke {
                self.row(Row::Once { p: Some(who(t)), fx: Fx::Move(am.id), of: None });
                self.s.sides[t.0].delete_vol(conds::STALL);
            }
        }
        if am.data().steals_boosts {
            self.note(EffectRef::Move(am.id), Hook::TryHit);
        }
        // 7. the hits
        let d = self.hit_loop(user, t, am);
        let hit = d.truthy() || d == R::Num(0);
        if !hit && d == R::False {
            failure = true;
        }
        am.hit_targets = Some(hit);
        self.sync_active(am);
        if !hit && !failure {
            self.mon_at_mut(user).move_this_turn_result = MoveResult::Null;
        }
        hit
    }

    /// the end of trySpreadMoveHit when a step removed the only target
    fn step_fail(&mut self, user: Pk, failure: bool) -> bool {
        if !failure {
            self.mon_at_mut(user).move_this_turn_result = MoveResult::Null;
        }
        false
    }

    /// pokemon.runImmunity for a move (false = immune)
    pub fn run_immunity(&mut self, t: Pk, am: &ActiveMove) -> bool {
        if am.ignore_immunity || am.ty == Type::Typeless {
            return true;
        }
        if am.scrappy && (am.ty == Type::Fighting || am.ty == Type::Normal) {
            return true;
        }
        run_field_hook(self.s, Hook::NegateImmunity);
        if am.ty == Type::Ground {
            return self.grounded(t);
        }
        let types = self.get_types(t, false);
        types.as_slice().iter().all(|&x| type_affects(am.ty, x))
    }

    /// pokemon.isGrounded
    pub fn is_grounded(&mut self, side: usize, slot: usize) -> bool {
        self.grounded((side, slot))
    }

    /// semi-invulnerable (Fly, Bounce, Dig, Dive, Phantom Force)
    pub fn semi_invulnerable(&self, p: Pk) -> bool {
        [conds::FLY, conds::BOUNCE, conds::DIVE, conds::DIG, conds::PHANTOMFORCE].iter().any(|&c| self.has_vol(p, c))
    }

    /// battleActions.hitStepAccuracy
    fn hit_step_accuracy(&mut self, user: Pk, t: Pk, am: &ActiveMove) -> bool {
        let d = am.data();
        self.s.active_target = pk_code(t);
        let mut acc: Option<u32> = if am.accuracy == ACC_ALWAYS { None } else { Some(am.accuracy as u32) };
        if d.ohko != Ohko::No {
            if !self.semi_invulnerable(t) {
                acc = Some(30);
                if let Ohko::Type(ty) = d.ohko {
                    if ty == Type::Ice && !self.has_type_pk(user, Type::Ice) {
                        acc = Some(20);
                    }
                }
                let ok = match d.ohko {
                    Ohko::Type(ty) => !self.has_type_pk(t, ty),
                    _ => true,
                };
                if !ok {
                    self.row(Row::Result { how: How::Immune, p: Some(who(t)) });
                    return false;
                }
            }
        } else {
            if self.any_handler(Ev::ModifyAccuracy) {
                let mut ctx = Ctx::with_move(Some(t), Some(user), am);
                let r = self.run_event(Ev::ModifyAccuracy, &mut ctx, match acc {
                    Some(a) => R::Num(a as i32),
                    None => R::True,
                });
                if let R::Num(x) = r {
                    acc = Some(x.max(0) as u32);
                }
            }
            run_field_hook(self.s, Hook::ModifyAccuracy);
            if let Some(a) = acc {
                let mut boost = 0i32;
                if !d.ignore_accuracy {
                    run_field_hook(self.s, Hook::ModifyBoost);
                    boost = self.modified_boost(user, B_ACC).clamp(-6, 6) as i32;
                }
                if !am.ignore_evasion {
                    boost = (boost - self.modified_boost(t, B_EVA) as i32).clamp(-6, 6);
                }
                acc = Some(accuracy_with_stage(a, boost));
            }
        }
        let always = (am.id == moves::TOXIC && self.has_type_pk(user, Type::Poison))
            || (d.target == Target::Self_ && d.category == Category::Status && !self.semi_invulnerable(t));
        if always {
            acc = None;
        } else {
            let mut ctx = Ctx::with_move(Some(t), Some(user), am);
            let r = self.run_event(Ev::Accuracy, &mut ctx, match acc {
                Some(a) => R::Num(a as i32),
                None => R::True,
            });
            acc = match r {
                R::Num(a) => Some(a.max(0) as u32),
                R::True => None,
                _ => Some(0),
            };
            run_field_hook(self.s, Hook::Accuracy);
        }
        if let Some(a) = acc {
            let site = Site::of_move(Key::Accuracy, user.0 as u8, am.id, 0);
            if !self.chance(site, a, 100) {
                self.row(Row::Miss { p: who(t), by: who(user) });
                return false;
            }
        }
        true
    }

    /// accuracy of a later hit of a multiaccuracy move (Triple Axel, Population Bomb)
    fn multi_accuracy(&mut self, user: Pk, t: Pk, am: &ActiveMove) -> bool {
        let d = am.data();
        if am.accuracy == ACC_ALWAYS {
            return true;
        }
        let table = [1.0, 4.0 / 3.0, 5.0 / 3.0, 2.0, 7.0 / 3.0, 8.0 / 3.0, 3.0];
        let mut acc = am.accuracy as f64;
        if !d.ignore_accuracy {
            let b = self.modified_boost(user, B_ACC).clamp(-6, 6);
            if b > 0 {
                acc *= table[b as usize];
            } else {
                acc /= table[(-b) as usize];
            }
        }
        if !am.ignore_evasion {
            let b = self.modified_boost(t, B_EVA).clamp(-6, 6);
            if b > 0 {
                acc /= table[b as usize];
            } else if b < 0 {
                acc *= table[(-b) as usize];
            }
        }
        // ModifyAccuracy (Gravity): runEvent scales only a whole-number accuracy
        if self.any_handler(Ev::ModifyAccuracy) {
            let mut ctx = Ctx::with_move(Some(t), Some(user), am);
            self.run_event_chain(Ev::ModifyAccuracy, &mut ctx, R::Num(acc.ceil() as i32));
            if acc.fract() == 0.0 && ctx.modifier != 4096 {
                acc = super::calc::modify(acc as u32, ctx.modifier) as f64;
            }
        }
        let mut ctx = Ctx::with_move(Some(t), Some(user), am);
        let r = self.run_event(Ev::Accuracy, &mut ctx, R::Num(acc.ceil() as i32));
        if r == R::True {
            return true;
        }
        // randomChance(accuracy, 100) with a fractional accuracy: Showdown compares random(100) < accuracy
        let site = Site::of_move(Key::Accuracy, user.0 as u8, am.id, am.hit.saturating_sub(1));
        let a = if let R::Num(x) = r { if x as f64 == acc.ceil() { acc } else { x as f64 } } else { acc };
        let n = a.ceil() as u32;
        self.chance(site, n, 100)
    }

    /// Champions hitStepMoveHitLoop for one target: the damage of the target (a number, or false)
    fn hit_loop(&mut self, user: Pk, t: Pk, am: &mut ActiveMove) -> R {
        let mut damage = R::Num(0);
        am.total_damage = 0;
        self.mon_at_mut(user).last_damage = 0;
        let mut target_hits = 1i32;
        if am.multihit.1 > 0 {
            if am.multihit.0 == am.multihit.1 {
                target_hits = am.multihit.0 as i32;
            } else {
                let site = Site { mv: Some(am.id), ..Site::new(Key::Hits, Some(user.0 as u8)) };
                if am.multihit == (2, 5) {
                    target_hits = self.o.sample(&site, &[2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5]);
                } else {
                    target_hits = self.o.range(&site, am.multihit.0 as i32, am.multihit.1 as i32 + 1);
                }
            }
        }
        let sleep_usable = am.data().sleep_usable || (!am.source_effect.is_none() && am.source_effect.data().sleep_usable);
        let mut null_damage = true;
        let mut move_damage = R::Undef;
        let mut tcopy = T::P(t);
        let mut hit = 1i32;
        while hit <= target_hits {
            if damage == R::False {
                break;
            }
            if hit > 1 && self.mon_at(user).status == Status::Slp && !sleep_usable {
                break;
            }
            if self.mon_at(t).hp == 0 {
                break;
            }
            am.hit = hit as u8;
            tcopy = T::P(t);
            if am.multiaccuracy && hit > 1 && !self.multi_accuracy(user, t, am) {
                break;
            }
            let (md, tc) = self.spread_move_hit(T::P(t), user, am, Md::Primary, false, false);
            move_damage = md;
            tcopy = tc;
            if move_damage == R::False {
                break;
            }
            null_damage = false;
            damage = match move_damage {
                R::Num(x) => R::Num(x),
                _ => R::Num(0),
            };
            if let R::Num(x) = damage {
                am.total_damage += x.max(0) as u32;
            }
            self.each_event(Hook::Update);
            if self.mon_at(user).hp == 0 {
                hit += 1;
                break;
            }
            hit += 1;
        }
        if hit == 1 {
            return R::False;
        }
        if null_damage {
            damage = R::False;
        }
        let user_out = self.mon_at(user).hp == 0;
        self.resolve_faints(false, false, user_out);
        if am.multihit.1 > 0 && !am.data().smart_target && !(am.hit == 1 && am.parental_bond) {
            self.row(Row::Hits { n: (hit - 1) as u8, p: who(t) });
        }
        if am.total_damage > 0 {
            self.apply_recoil(user, am.total_damage as f64, am);
        }
        if let T::P(tp) = tcopy {
            if tp != user {
                let numeric = matches!(move_damage, R::Num(_));
                self.got_attacked(tp, user, if let R::Num(x) = move_damage { x.max(0) as u32 } else { 0 }, numeric);
                if numeric {
                    let m = self.mon_at_mut(tp);
                    m.times_attacked = m.times_attacked.saturating_add((hit - 1) as u8);
                }
            }
        }
        if am.data().ohko != Ohko::No && self.mon_at(t).hp == 0 {
            self.row(Row::Result { how: How::Ohko, p: Some(who(t)) });
        }
        if !(damage.truthy() || damage == R::Num(0)) {
            return damage;
        }
        self.each_event(Hook::Update);
        if let T::P(tp) = tcopy {
            let mut ctx = Ctx::with_move(Some(tp), Some(user), am);
            self.run_event(Ev::AfterMoveSecondary, &mut ctx, R::True);
        }
        run_field_hook(self.s, Hook::AfterMoveSecondary);
        run_field_hook(self.s, Hook::EmergencyExit);
        if let R::Num(_) = damage {
            if self.mon_at(t).hp > 0 {
                let before = self.mon_at(t).hurt_this_turn.unwrap_or(0) as u32 + am.total_damage;
                self.emergency_exit(t, Some(user), before.min(u16::MAX as u32) as u16);
            }
        }
        damage
    }

    /// pokemon.gotAttacked: a new attackedBy entry
    fn got_attacked(&mut self, t: Pk, source: Pk, damage: u32, numeric: bool) {
        let m = self.mon_at_mut(t);
        let e = Attack { source: pk_code(source), damage: damage.min(u16::MAX as u32) as u16, numeric, this_turn: true };
        if (m.nattacked as usize) < MAX_ATTACKS {
            m.attacked[m.nattacked as usize] = e;
            m.nattacked += 1;
        } else {
            m.attacked.copy_within(1..MAX_ATTACKS, 0);
            m.attacked[MAX_ATTACKS - 1] = e;
        }
    }

    // ------------------------------------------------------------------------------------------ spreadMoveHit

    /// Champions spreadMoveHit for one target. Returns (damage, target after the steps).
    pub fn spread_move_hit(&mut self, target: T, user: Pk, am: &mut ActiveMove, md: Md, secondary: bool, is_self: bool) -> (R, T) {
        let mut damage = R::True;
        let mut tgt = target;
        let d = am.data();
        // TryHit of the move data (only the move itself has one)
        if md == Md::Primary {
            let r = if am.target == Target::All && !is_self {
                R::True
            } else if matches!(am.target, Target::FoeSide | Target::AllySide | Target::AllyTeam) && !is_self {
                R::True
            } else if let T::P(tp) = tgt {
                self.move_try_hit(tp, user, am)
            } else {
                R::True
            };
            if !(r.truthy() || r == R::Undef) {
                if r == R::False {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
                }
                return (R::False, tgt);
            }
        }
        // 0. Substitute
        if !secondary && !is_self && !field_like(am.target) {
            if let T::P(tp) = tgt {
                let mut ctx = Ctx::with_move(Some(tp), Some(user), am);
                damage = self.run_event(Ev::TryPrimaryHit, &mut ctx, R::True);
                if let Some(x) = ctx.am {
                    // the Substitute's getDamage drew the hit's crit and used up a Beat Up ally
                    *am = x;
                }
                run_field_hook(self.s, Hook::TryPrimaryHit);
            }
        }
        if damage == R::Num(0) {
            damage = R::True;
            tgt = T::Null;
        }
        if matches!(tgt, T::P(_)) && secondary && !self.md_has_self(am, md) {
            damage = R::True;
        }
        if !damage.truthy() {
            tgt = T::False;
        }
        // 1. getDamage (getSpreadDamage sets activeTarget)
        if let T::P(tp) = tgt {
            self.s.active_target = pk_code(tp);
            damage = match self.get_damage_md(user, tp, am, md) {
                Dmg::Hp(x) => R::Num(x as i32),
                Dmg::None => R::Undef,
                Dmg::Fail => R::False,
            };
            if damage == R::False {
                tgt = T::False;
            }
        }
        // 2. spreadDamage
        if damage.truthy() || damage == R::Num(0) {
            damage = match tgt {
                T::P(tp) => {
                    let amount = if let R::Num(x) = damage { x } else { 1 };
                    if self.mon_at(tp).hp == 0 {
                        R::Num(0)
                    } else if !self.mon_at(tp).is_active {
                        R::False
                    } else if damage == R::True {
                        R::Num(0)
                    } else {
                        run_field_hook(self.s, Hook::Damage);
                        match self.damage(amount as f64, tp, Some(user), Eff::Move(am.id)) {
                            Some(x) => R::Num(x as i32),
                            None => R::False,
                        }
                    }
                }
                _ => R::Num(0),
            };
            if damage == R::False {
                tgt = T::False;
            }
        }
        // 3. the move's effects on the target
        damage = self.run_move_effects(damage, tgt, user, am, md, secondary, is_self);
        if !(damage.truthy() || damage == R::Num(0)) {
            tgt = T::False;
        }
        // 4. self effects; steps 4 and 5 keep activeTarget
        let active_target = self.s.active_target;
        if self.md_has_self(am, md) && !am.self_dropped {
            self.self_drops(tgt, user, am, md, secondary);
        }
        // 5. secondaries (Sheer Force deletes the move's own; Stench, King's Rock and Fling add some)
        if md == Md::Primary && am.has_secondaries() {
            self.secondaries(tgt, user, am, is_self);
        }
        self.s.active_target = active_target;
        // 6. forced switch
        if md == Md::Primary && d.force_switch {
            if let T::P(tp) = tgt {
                if self.mon_at(tp).hp > 0 && self.mon_at(user).hp > 0 && self.can_switch(tp.0) {
                    let mut ctx = Ctx::with_move(Some(tp), Some(user), am);
                    let r = self.run_event(Ev::DragOut, &mut ctx, R::True);
                    run_field_hook(self.s, Hook::DragOut);
                    if r.truthy() {
                        self.mon_at_mut(tp).force_switch_flag = true;
                    } else if r == R::False && am.category == Category::Status {
                        self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
                        damage = R::False;
                    }
                }
            }
        }
        if !(damage.truthy() || damage == R::Num(0)) {
            tgt = T::False;
        }
        if let (R::Num(x), T::P(tp)) = (damage, tgt) {
            if !secondary && !is_self {
                let orig = self.mon_at(user).hp;
                let mut ctx = Ctx::with_move(Some(tp), Some(user), am);
                ctx.arg = x.max(0) as u16;
                self.run_event(Ev::DamagingHit, &mut ctx, R::Num(x));
                run_field_hook(self.s, Hook::DamagingHit);
                if run_move_hook(self.s, am.id, Hook::AfterHit) {
                    self.move_after_hit(tp, user, am);
                }
                run_field_hook(self.s, Hook::EmergencyExit);
                self.emergency_exit(user, None, orig);
            }
        }
        (damage, tgt)
    }

    /// battleActions.moveHit: the damage of the one target (undefined for true)
    pub fn move_hit(&mut self, target: T, user: Pk, am: &mut ActiveMove, md: Md, secondary: bool, is_self: bool) -> R {
        let (d, _) = self.spread_move_hit(target, user, am, md, secondary, is_self);
        if d == R::True {
            R::Undef
        } else {
            d
        }
    }

    fn md_has_self(&self, am: &ActiveMove, md: Md) -> bool {
        let d = am.data();
        match md {
            Md::Primary => d.self_effect.is_some() && !am.sheer_force,
            Md::Secondary(i) if (i as usize) < d.secondaries.len() => {
                let s = &d.secondaries[i as usize];
                s.self_boosts.is_some() || !s.self_volatile.is_none()
            }
            _ => false,
        }
    }

    /// The boosts, heal, status and volatile of a hit effect.
    fn md_parts(&self, am: &ActiveMove, md: Md) -> (Option<[i8; 7]>, (u8, u8), Status, Cond) {
        let d = am.data();
        match md {
            Md::Primary => {
                let boosts = if am.no_boosts { None } else { am.boosts };
                (boosts, d.heal, d.status, d.volatile_status)
            }
            Md::Extra(i) => {
                let x = am.extra[i as usize];
                (None, (0, 0), x.status, x.vol)
            }
            Md::Secondary(i) => {
                let s = &d.secondaries[i as usize];
                (s.boosts, (0, 0), s.status, s.volatile_status)
            }
            Md::SelfOf => {
                let s = d.self_effect.as_ref().unwrap();
                (s.boosts, (0, 0), s.status, s.volatile_status)
            }
            Md::SecSelf(i) if i as usize >= d.secondaries.len() => (None, (0, 0), Status::None, Cond::NONE),
            Md::SecSelf(i) => {
                let s = &d.secondaries[i as usize];
                (s.self_boosts, (0, 0), Status::None, s.self_volatile)
            }
            Md::SelfBoost => (d.self_boost, (0, 0), Status::None, Cond::NONE),
        }
    }

    /// battleActions.runMoveEffects for one target
    fn run_move_effects(&mut self, damage: R, tgt: T, user: Pk, am: &mut ActiveMove, md: Md, secondary: bool, is_self: bool) -> R {
        let mut damage = damage;
        let mut did_anything = damage;
        if tgt == T::False {
            return self.after_effects(did_anything, damage, user, am, md, secondary, is_self);
        }
        let d = am.data();
        let (boosts, heal, status, vol) = self.md_parts(am, md);
        let mut did = R::Undef;
        let eff = Eff::Move(am.id);
        if let T::P(tp) = tgt {
            if let Some(b) = boosts {
                if !self.mon_at(tp).fainted {
                    let (list, n) = boost_list(&b);
                    let r = self.boost(&list[..n], tp, Some(user), eff, secondary, is_self);
                    did = combine(did, r);
                }
            }
            if heal.1 > 0 && !self.mon_at(tp).fainted {
                let m = self.mon_at(tp);
                if m.hp >= m.maxhp {
                    self.row(Row::Fail { p: Some(who(tp)), why: Fx::HpFull });
                    damage = combine(damage, R::False);
                    did_anything = combine(did_anything, R::Null);
                    return self.finish_effects(did_anything, damage, user, am, md, secondary, is_self);
                }
                let amount = (m.base_maxhp as f64 * heal.0 as f64 / heal.1 as f64).round();
                let r = self.heal(amount, tp, Some(user), eff);
                if !r.truthy() && r != R::Num(0) {
                    if r != R::Null {
                        self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
                    }
                    damage = combine(damage, R::False);
                    did_anything = combine(did_anything, R::Null);
                    return self.finish_effects(did_anything, damage, user, am, md, secondary, is_self);
                }
                did = R::True;
            }
            if status != Status::None {
                let ok = self.try_set_status(tp, status, Some(user), eff);
                if !ok && d.status != Status::None {
                    damage = combine(damage, R::False);
                    did_anything = combine(did_anything, R::Null);
                    return self.finish_effects(did_anything, damage, user, am, md, secondary, is_self);
                }
                did = combine(did, R::of_bool(ok));
            }
            if !vol.is_none() {
                let r = self.add_volatile(tp, vol, Some(user), eff, false);
                did = combine(did, r);
            }
            if md == Md::Primary {
                if !d.side_condition.is_none() {
                    let r = self.add_side_cond(tp.0, d.side_condition, user, eff);
                    did = combine(did, r);
                }
                if !d.slot_condition.is_none() {
                    let r = self.add_slot_cond(tp.0, d.slot_condition, user, eff);
                    did = combine(did, r);
                }
                if !d.weather.is_none() {
                    let r = self.set_weather(d.weather, user, eff);
                    did = combine(did, r);
                }
                if !d.terrain.is_none() {
                    let r = self.set_terrain(d.terrain, user, eff);
                    did = combine(did, r);
                }
                if !d.pseudo_weather.is_none() {
                    let r = self.add_pseudo(d.pseudo_weather, user, eff);
                    did = combine(did, r);
                }
            }
            if md == Md::Primary && d.force_switch {
                let r = R::of_bool(self.can_switch(tp.0));
                did = combine(did, r);
            }
            // Hit events
            if am.target == Target::All && !is_self {
                if md == Md::Primary {
                    if let Some(r) = self.move_hit_field(tp, user, am) {
                        // singleEvent: a handler that returns nothing gives true
                        did = combine(did, if r == R::Undef { R::True } else { r });
                    }
                }
            } else if matches!(am.target, Target::FoeSide | Target::AllySide) && !is_self {
                if md == Md::Primary {
                    if let Some(r) = self.move_hit_side(tp.0, user, am) {
                        did = combine(did, if r == R::Undef { R::True } else { r });
                    }
                }
            } else {
                if let Some(r) = self.move_on_hit(tp, user, am, md) {
                    did = combine(did, if r == R::Undef { R::True } else { r });
                }
                if !is_self && !secondary {
                    let mut ctx = Ctx::with_move(Some(tp), Some(user), am);
                    self.run_event(Ev::Hit, &mut ctx, R::True);
                    run_field_hook(self.s, Hook::Hit);
                }
            }
        }
        if md == Md::Primary && d.selfdestruct == SelfDestruct::IfHit && damage != R::False {
            self.faint_by(user, Some(user), true);
        }
        if md == Md::Primary && d.self_switch != SelfSwitch::No && !am.no_self_switch {
            if self.can_switch(user.0) {
                did = R::True;
            } else {
                did = combine(did, R::False);
            }
        }
        if did == R::Undef {
            did = R::True;
        }
        damage = combine(damage, if did == R::Null { R::False } else { did });
        did_anything = combine(did_anything, did);
        self.finish_effects(did_anything, damage, user, am, md, secondary, is_self)
    }

    fn after_effects(&mut self, did_anything: R, damage: R, user: Pk, am: &mut ActiveMove, md: Md, secondary: bool, is_self: bool) -> R {
        self.finish_effects(did_anything, damage, user, am, md, secondary, is_self)
    }

    /// the end of runMoveEffects: a failure, or the self-switch flag
    fn finish_effects(&mut self, did_anything: R, damage: R, user: Pk, am: &mut ActiveMove, md: Md, secondary: bool, is_self: bool) -> R {
        let d = am.data();
        let has_self = self.md_has_self(am, md);
        let selfdestruct = md == Md::Primary && d.selfdestruct != SelfDestruct::No;
        if !did_anything.truthy() && did_anything != R::Num(0) && !has_self && !selfdestruct {
            // the move failed because it did nothing
            if !is_self && !secondary && did_anything == R::False && !std::mem::take(&mut self.no_fail_row) {
                self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
            }
        } else if md == Md::Primary && d.self_switch != SelfSwitch::No && !am.no_self_switch && self.mon_at(user).hp > 0
            && (self.can_switch(user.0) || am.id == moves::REVIVALBLESSING)
        {
            let m = self.mon_at_mut(user);
            m.switch_flag = true;
            m.switch_flag_move = am.id;
        }
        damage
    }

    /// battleActions.selfDrops
    fn self_drops(&mut self, tgt: T, user: Pk, am: &mut ActiveMove, md: Md, secondary: bool) {
        if tgt == T::False {
            return;
        }
        let self_md = match md {
            Md::Primary => Md::SelfOf,
            Md::Secondary(i) => Md::SecSelf(i),
            _ => return,
        };
        let has_boosts = self.md_parts(am, self_md).0.is_some();
        if !secondary && has_boosts {
            // a `self` with boosts rolls random(100) against its chance; no legal move gives it one
            self.move_hit(T::P(user), user, am, self_md, secondary, true);
            if am.multihit.1 == 0 {
                am.self_dropped = true;
            }
        } else {
            self.move_hit(T::P(user), user, am, self_md, secondary, true);
        }
    }

    /// battleActions.secondaries: one roll per secondary with a chance below 100
    fn secondaries(&mut self, tgt: T, user: Pk, am: &mut ActiveMove, is_self: bool) {
        if tgt == T::False {
            return;
        }
        run_field_hook(self.s, Hook::ModifySecondaries);
        let own = if am.sheer_force { 0 } else { am.data().secondaries.len() };
        let n = own + am.nextra as usize;
        // Shield Dust: only the secondaries with a `self` effect stay
        let shield = if let T::P(tp) = tgt {
            tp != user && self.mon_at(tp).ability == abilities::SHIELDDUST && !self.ability_skipped(tp, abilities::SHIELDDUST, Ev::ModifySecondaries)
        } else {
            false
        };
        for i in 0..n {
            let (chance, has_self) = if i < own {
                let sec = am.data().secondaries[i];
                (sec.chance, sec.self_boosts.is_some() || !sec.self_volatile.is_none())
            } else {
                (am.extra[i - own].chance, false)
            };
            if shield && !has_self {
                continue;
            }
            // Serene Grace doubles the chance
            let chance = chance.map(|c| if am.sec_double { c as u32 * 2 } else { c as u32 });
            let ok = match chance {
                None => true,
                Some(c) => {
                    let site = Site {
                        index: Some(SecIndex::At(i as u8)),
                        ..Site::of_move(Key::Secondary, user.0 as u8, am.id, am.hit.saturating_sub(1))
                    };
                    self.chance(site, c, 100)
                }
            };
            if ok {
                let md = if i < own { Md::Secondary(i as u8) } else { Md::Extra((i - own) as u8) };
                self.move_hit(tgt, user, am, md, true, is_self);
            }
        }
    }

    /// battleActions.applyRecoilDamage
    pub fn apply_recoil(&mut self, user: Pk, dealt: f64, am: &ActiveMove) {
        let d = am.data();
        let m = *self.mon_at(user);
        if d.struggle_recoil {
            let r = ((m.base_maxhp as f64) / 4.0).round().max(1.0);
            self.direct_damage(r, user, Some(user), Eff::StruggleRecoil);
        } else if d.mind_blown_recoil {
            let r = (m.maxhp as f64 / 2.0).round();
            self.damage(r, user, Some(user), Eff::Cond(Cond::NONE));
        } else if d.recoil.1 > 0 {
            let r = (dealt * d.recoil.0 as f64 / d.recoil.1 as f64).round().max(1.0);
            self.damage(r, user, Some(user), Eff::Recoil);
        } else {
            return;
        }
        run_field_hook(self.s, Hook::EmergencyExit);
        self.emergency_exit(user, Some(user), m.hp);
    }

    // ------------------------------------------------------------------------------------------ damage

    /// getDamage for a hit effect: secondaries and self effects deal nothing
    fn get_damage_md(&mut self, user: Pk, t: Pk, am: &mut ActiveMove, md: Md) -> Dmg {
        if md != Md::Primary {
            return Dmg::None;
        }
        self.get_damage(user.0, user.1, t.0, t.1, am)
    }

    /// battleActions.getDamage: draws the crit and the roll, then computes the hit.
    pub fn get_damage(&mut self, side: usize, slot: usize, ts: usize, tslot: usize, am: &mut ActiveMove) -> Dmg {
        let user = (side, slot);
        let t = (ts, tslot);
        if !self.run_immunity(t, am) {
            return Dmg::Fail;
        }
        let d = am.data();
        if d.ohko != Ohko::No {
            return Dmg::Hp(self.mon_at(t).maxhp as u32);
        }
        am.raw_half = None;
        if let Some(x) = self.move_damage_callback(user, t, am) {
            return match x {
                Some(v) => {
                    // a number, 0 included; spreadDamage floors it
                    let v = v.max(0.0);
                    am.raw_half = Some((2.0 * v).round() as u32);
                    Dmg::Hp(v.floor() as u32)
                }
                None => Dmg::Fail,
            };
        }
        match d.damage {
            FixedDamage::Level => return Dmg::Hp(self.mon_at(user).level as u32),
            FixedDamage::Value(v) => return Dmg::Hp(v as u32),
            FixedDamage::None => {}
        }
        let mut bp = am.base_power as f64;
        if let Some(x) = self.move_base_power_callback(user, t, am) {
            match x {
                Some(v) => bp = v,
                None => return Dmg::Fail,
            }
        }
        if bp == 0.0 {
            return Dmg::None;
        }
        let mut bp = (bp.floor() as i64).max(1) as u32;
        let mut ctx = Ctx::with_move(Some(user), Some(t), am);
        let r = self.run_event(Ev::ModifyCritRatio, &mut ctx, R::Num(am.crit_ratio as i32));
        run_field_hook(self.s, Hook::ModifyCritRatio);
        let ratio = if let R::Num(x) = r { x.clamp(0, 4) as usize } else { 0 };
        let mut crit = d.will_crit.unwrap_or(false);
        if d.will_crit.is_none() && ratio > 0 {
            let site = Site::of_move(Key::Crit, side as u8, am.id, am.hit.saturating_sub(1));
            crit = self.chance(site, 1, [0, 24, 8, 2, 1][ratio]);
        }
        if crit {
            run_field_hook(self.s, Hook::CriticalHit);
            if self.any_handler(Ev::CriticalHit) {
                let mut ctx = Ctx::with_move(Some(t), None, am);
                crit = self.run_event(Ev::CriticalHit, &mut ctx, R::True).truthy();
            }
        }
        am.crit = crit;
        // BasePower: the conditions' handlers by priority (Charge 9, terrains 6), then the move's own
        // onBasePower (priority 0, the user's speed: after them), one chained modifier
        let mut ctx = Ctx::with_move(Some(user), Some(t), am);
        let r = self.run_event_chain(Ev::BasePower, &mut ctx, R::Num(bp as i32));
        if let R::Num(_) = r {
            self.move_on_base_power(user, t, am, &mut ctx);
        }
        run_field_hook(self.s, Hook::BasePower);
        match r {
            R::Num(x) => bp = super::calc::modify(x.max(0) as u32, ctx.modifier),
            _ => return Dmg::Hp(0),
        }
        if bp == 0 {
            return Dmg::Hp(0);
        }
        let roll_site = Site::of_move(Key::Roll, side as u8, am.id, am.hit.saturating_sub(1));
        let mut draws = HitDraws { crit, roll: 0 };
        let input = self.hit_input_bp(user, t, am, &draws, bp);
        am.type_mod = input.type_mod.clamp(-6, 6) as i8;
        if am.type_mod != 0 && self.logging() {
            let how = match am.type_mod {
                1 => How::Super,
                x if x > 1 => How::Extreme,
                -1 => How::Resisted,
                _ => How::Mostly,
            };
            self.row(Row::Result { how, p: Some(who(t)) });
        }
        draws.roll = self.o.range(&roll_site, 0, 16) as u32;
        let final_mod = self.modify_damage_event(user, t, am);
        if am.bypass_protect && self.logging() {
            // the protection let a quarter through: the attacker's ability, "couldn't fully protect"
            let ab = self.mon_at(user).ability;
            self.row(Row::Ability { p: who(user), ab, of: None });
            self.row(Row::Once { p: Some(who(t)), fx: Fx::Ability(ab), of: Some(who(user)) });
        }
        Dmg::Hp(hit_damage(&HitInput { roll: draws.roll, final_mod, quarter: am.bypass_protect, ..input }))
    }

    /// the ModifyDamage event of modifyDamage (after the roll): the target's onSourceModifyDamage (Dig,
    /// Dive, Fly, Glaive Rush, Minimize), as a chained modifier
    fn modify_damage_event(&mut self, user: Pk, t: Pk, am: &ActiveMove) -> u32 {
        let mut final_mod = 4096;
        if self.any_handler(Ev::ModifyDamage) {
            let mut ctx = Ctx::with_move(Some(user), Some(t), am);
            self.run_event(Ev::ModifyDamage, &mut ctx, R::Num(1));
            final_mod = ctx.modifier;
        }
        run_field_hook(self.s, Hook::ModifyDamage);
        final_mod
    }

    /// Everything the damage formula needs once the crit is known (roll left at 0).
    pub fn hit_input(&mut self, side: usize, slot: usize, ts: usize, tslot: usize, am: &ActiveMove, draws: &HitDraws) -> HitInput {
        let bp = am.base_power.max(1);
        self.hit_input_bp((side, slot), (ts, tslot), am, draws, bp)
    }

    fn hit_input_bp(&mut self, user: Pk, t: Pk, am: &ActiveMove, draws: &HitDraws, bp: u32) -> HitInput {
        let d = am.data();
        let (aside, aslot) = if d.offensive_pokemon_target { t } else { user };
        let (dside, dslot) = if d.defensive_pokemon_source { user } else { t };
        let physical = am.category == Category::Physical;
        let atk_stat = am.override_offensive.unwrap_or(if physical { Stat::Atk } else { Stat::Spa });
        let def_stat = d.override_defensive_stat.unwrap_or(if physical { Stat::Def } else { Stat::Spd });
        let mut atk_boost = self.s.mon(aside, aslot).boosts[atk_stat.boost_index()];
        let mut def_boost = self.s.mon(dside, dslot).boosts[def_stat.boost_index()];
        let ign_neg_off = d.ignore_negative_offensive || draws.crit;
        let ign_pos_def = d.ignore_positive_defensive || draws.crit;
        if d.ignore_offensive || (ign_neg_off && atk_boost < 0) {
            atk_boost = 0;
        }
        if d.ignore_defensive || (ign_pos_def && def_boost > 0) {
            def_boost = 0;
        }
        let mut attack = self.calculate_stat_for(aside, aslot, atk_stat, atk_boost, user);
        let mut defense = self.calculate_stat_for(dside, dslot, def_stat, def_boost, t);
        run_field_hook(self.s, if physical { Hook::ModifyAtk } else { Hook::ModifySpA });
        // runEvent('ModifyAtk'/'ModifySpA', source, target, move): by the category, not the stat used
        let aev = if physical { Ev::ModifyAtk } else { Ev::ModifySpA };
        if self.any_handler(aev) {
            let mut ctx = Ctx::with_move(Some(user), Some(t), am);
            if let R::Num(x) = self.run_event(aev, &mut ctx, R::Num(attack as i32)) {
                attack = x.max(0) as u32;
            }
        }
        // runEvent('ModifyDef'/'ModifySpD', target, source, move): sandstorm and snow
        let dev = match def_stat {
            Stat::Def => Some(Ev::ModifyDef),
            Stat::Spd => Some(Ev::ModifySpD),
            _ => None,
        };
        if let Some(e) = dev {
            if self.any_handler(e) {
                let mut ctx = Ctx::with_move(Some(t), Some(user), am);
                if let R::Num(x) = self.run_event(e, &mut ctx, R::Num(defense as i32)) {
                    defense = x.max(0) as u32;
                }
            }
        }
        run_field_hook(self.s, if def_stat == Stat::Def { Hook::ModifyDef } else { Hook::ModifySpD });
        let user_m = *self.mon_at(user);
        let weather = self.weather_modify_damage(user, t, am);
        run_field_hook(self.s, Hook::WeatherModifyDamage);
        let stab = if am.ty == Type::Typeless {
            None
        } else {
            run_field_hook(self.s, Hook::ModifySTAB);
            let mut st = if d.force_stab || self.has_type_pk(user, am.ty) { 6144 } else { 4096 };
            if self.any_handler(Ev::ModifySTAB) {
                let mut ctx = Ctx::with_move(Some(user), Some(t), am);
                if let R::Num(x) = self.run_event_chain(Ev::ModifySTAB, &mut ctx, R::Num(st as i32)) {
                    st = x as u32;
                }
            }
            Some(st)
        };
        run_field_hook(self.s, Hook::Effectiveness);
        let type_mod = self.run_effectiveness(t, am);
        // ModifyDamage runs after the roll: see modify_damage_event
        let final_mod = 4096;
        HitInput {
            level: user_m.level as u32,
            base_power: bp,
            attack,
            defense,
            spread: false,
            parental_second: (d.multihit_parental_bond || am.parental_bond) && am.hit > 1,
            weather,
            crit: draws.crit,
            crit_modifier: d.crit_modifier,
            roll: draws.roll,
            stab,
            type_mod,
            burn: user_m.status == Status::Brn && am.category == Category::Physical && !self.has_ability(user, abilities::GUTS)
                && am.id != moves::FACADE,
            final_mod,
            quarter: false,
        }
    }

    /// priorityEvent('WeatherModifyDamage', attacker, defender, move): the weather's handler, by the
    /// defender's weather (sun: Fire 1.5, Water 0.5; rain: Water 1.5, Fire 0.5)
    fn weather_modify_damage(&mut self, user: Pk, t: Pk, am: &ActiveMove) -> u32 {
        // Mega Sol's handler runs the sun's (a fast exit: the real weather's handler does not run)
        let w = if self.has_ability(user, abilities::MEGASOL) { conds::SUNNYDAY } else { self.eff_weather(t) };
        let (up, down) = if w == conds::SUNNYDAY {
            (Type::Fire, Type::Water)
        } else if w == conds::RAINDANCE {
            (Type::Water, Type::Fire)
        } else {
            return 4096;
        };
        if am.ty == up {
            6144
        } else if am.ty == down {
            2048
        } else {
            4096
        }
    }

    /// pokemon.calculateStat: the stored stat with the given stage (Wonder Room swaps the defences)
    pub fn calculate_stat(&mut self, side: usize, slot: usize, stat: Stat, boost: i8) -> u32 {
        self.calculate_stat_for(side, slot, stat, boost, (side, slot))
    }

    /// calculateStat with statUser: the Pokémon the ModifyBoost event is run on (Unaware)
    pub fn calculate_stat_for(&mut self, side: usize, slot: usize, stat: Stat, boost: i8, user: Pk) -> u32 {
        let mut boost = boost;
        if self.any_handler(Ev::ModifyBoost) {
            let mut ctx = Ctx::new(Some(user), None, Eff::None);
            ctx.boosts = Boosts::of(&[(stat.boost_index(), boost)]);
            self.run_event(Ev::ModifyBoost, &mut ctx, R::True);
            boost = ctx.boosts.get(stat.boost_index()).unwrap_or(0);
        }
        let m = self.s.mon(side, slot);
        let mut st = stat;
        if self.s.field.pseudo[..self.s.field.npseudo as usize].iter().any(|c| c.id == conds::WONDERROOM) {
            st = match stat {
                Stat::Def => Stat::Spd,
                Stat::Spd => Stat::Def,
                s => s,
            };
        }
        let v = m.stats[st.boost_index()] as u32;
        run_field_hook(self.s, Hook::ModifyBoost);
        apply_stage(v, boost)
    }

    /// a stage after runEvent('ModifyBoost', pokemon) (accuracy and evasion in hitStepAccuracy)
    pub fn modified_boost(&mut self, p: Pk, i: usize) -> i8 {
        let b = self.mon_at(p).boosts;
        if !self.any_handler(Ev::ModifyBoost) {
            return b[i];
        }
        let mut ctx = Ctx::new(Some(p), None, Eff::None);
        ctx.boosts = Boosts::full(&b);
        self.run_event(Ev::ModifyBoost, &mut ctx, R::True);
        ctx.boosts.get(i).unwrap_or(0)
    }

    /// pokemon.runEffectiveness: the move's own Effectiveness handler and the Effectiveness event per type
    pub fn run_effectiveness(&mut self, t: Pk, am: &ActiveMove) -> i32 {
        let mut total = 0;
        let types = self.get_types(t, false);
        let handled = self.any_handler(Ev::Effectiveness);
        for &ty in types.as_slice() {
            let mut m = self.move_effectiveness(am, ty, effectiveness(am.ty, ty));
            if handled {
                let mut ctx = Ctx::with_move(Some(t), None, am);
                ctx.arg = ty as u16;
                if let R::Num(x) = self.run_event_chain(Ev::Effectiveness, &mut ctx, R::Num(m)) {
                    m = x;
                }
            }
            total += m;
        }
        total
    }

    /// actions.getConfusionDamage
    pub fn confusion_damage(&mut self, p: Pk) -> u32 {
        let m = *self.mon_at(p);
        let attack = self.calculate_stat(p.0, p.1, Stat::Atk, m.boosts[0]) as u64;
        let defense = self.calculate_stat(p.0, p.1, Stat::Def, m.boosts[1]).max(1) as u64;
        let l = (2 * m.level as u64 / 5 + 2) as u64;
        let base = tr(tr(tr(l * 40 * attack) / defense) / 50) + 2;
        let dmg = base % 65536;
        let site = Site { mv_name: Some("confusion"), hit: Some(0), ..Site::new(Key::Roll, Some(p.0 as u8)) };
        let roll = self.o.range(&site, 0, 16) as u64;
        let dmg = tr(tr(dmg * (100 - roll)) / 100);
        dmg.max(1) as u32
    }
}

/// What a useMove call was given as its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tgt {
    /// no target (an ally-targeting move in singles)
    None,
    Some(Pk),
    /// undefined: a called move (Sleep Talk, Copycat) picks one
    Unset,
}
