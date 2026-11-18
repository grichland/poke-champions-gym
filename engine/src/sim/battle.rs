//! The turn resolver, following Showdown's order (sim/battle.ts, sim/battle-queue.ts): the action queue and
//! its sort, actions, switching, fainting, the end of the turn and the win rule. Moves are in `moves.rs`.
//!
//! Every place where Showdown draws a random number (speed-tie shuffles of actions, Pokémon and handlers,
//! insertion among equals, accuracy, crits, rolls, hit counts, statuses' and volatiles' chances) draws
//! through the outcome source in the same order, so a tape recorded from Showdown replays exactly.

use super::calc::*;
use super::data::*;
use super::events::*;
use super::hooks::*;
use super::log::Row;
use super::moves::ActiveMove;
use super::outcome::*;
use super::state::*;

/// The resolver's working context: the state, the outcome source and battle.activeMove.
pub struct Sim<'a, O: Outcomes> {
    pub s: &'a mut State,
    pub o: &'a mut O,
    /// battle.activeMove (the use of a move being executed)
    pub active: Option<ActiveMove>,
    /// the next useMove is a Magic Bounce reflection
    pub bounce_next: bool,
    /// the next useMove is Dancer's
    pub external_next: bool,
    /// the next useMove is a Curse whose action was resolved while the user was not a Ghost (Champions
    /// resolveAction sets the action move's target to self)
    pub curse_self_next: bool,
    /// the battle log's row buffer when rows are recorded (`log.rs`)
    pub log: Option<&'a mut Vec<super::log::Row>>,
    /// the next switch-in writes a withdraw row for the Pokémon it replaces (and the pivot move)
    pub withdraw_next: Option<Option<MoveId>>,
    /// the cause the next HP row of a damage call names, when its effect does not say (a crash, a cost)
    pub hp_cause: Option<super::log::Fx>,
    /// the End handlers of a fainting Pokémon are running (Illusion ends without a row)
    pub fainting: bool,
    /// the last setStatus was stopped by a SetStatus handler returning null (Rest under Uproar)
    pub status_null: bool,
    /// the move's own "But it failed!" row is not written (its handler returned null in Showdown)
    pub no_fail_row: bool,
}

pub const NO_SIDE: u8 = 0xFF;
/// `Action::move_index` for Struggle
pub const STRUGGLE: u8 = 0xFF;
/// `Action::move_index` for the Recharge turn after Hyper Beam
pub const RECHARGE: u8 = 0xFE;
/// `Action::move_index` for a move that is in no slot (a locked move called by Copycat)
pub const NOSLOT: u8 = 0xFD;

/// Showdown's action orders (battle-queue.ts resolveAction).
pub mod order {
    pub const TEAM: u16 = 1;
    pub const START: u16 = 2;
    pub const INSTASWITCH: u16 = 3;
    pub const BEFORE_TURN: u16 = 4;
    pub const BEFORE_TURN_MOVE: u16 = 5;
    pub const REVIVAL_BLESSING: u16 = 6;
    pub const RUN_SWITCH: u16 = 101;
    pub const SWITCH: u16 = 103;
    pub const MEGA_EVO: u16 = 104;
    pub const PRIORITY_CHARGE: u16 = 107;
    pub const MOVE: u16 = 200;
    pub const RESIDUAL: u16 = 300;
}

/// comparePriority: negative when `a` goes first.
#[inline]
fn compare(a: &Action, b: &Action) -> i64 {
    let d = a.order as i64 - b.order as i64;
    if d != 0 {
        return d;
    }
    let d = b.priority as i64 - a.priority as i64;
    if d != 0 {
        return d;
    }
    let d = b.speed as i64 - a.speed as i64;
    if d != 0 {
        return d;
    }
    a.sub_order as i64 - b.sub_order as i64
}

impl<'a, O: Outcomes> Sim<'a, O> {
    pub fn new(s: &'a mut State, o: &'a mut O) -> Sim<'a, O> {
        Sim { s, o, active: None, bounce_next: false, external_next: false, curse_self_next: false, log: None, withdraw_next: None, hp_cause: None, fainting: false, status_null: false, no_fail_row: false }
    }

    /// A context that records the battle log's rows into `log`.
    pub fn with_log(s: &'a mut State, o: &'a mut O, log: Option<&'a mut Vec<super::log::Row>>) -> Sim<'a, O> {
        Sim { log, ..Sim::new(s, o) }
    }

    // ------------------------------------------------------------------------------------------ speed

    pub fn trick_room(&self) -> bool {
        let f = &self.s.field;
        f.pseudo[..f.npseudo as usize].iter().any(|c| c.id == conds::TRICKROOM)
    }

    /// pokemon.getStat
    pub fn get_stat(&mut self, side: usize, slot: usize, stat: Stat, unboosted: bool, unmodified: bool) -> u32 {
        let m = &self.s.sides[side].mons[slot];
        let mut stat = stat;
        // Download's unmodified read under Wonder Room: the other defence's stage
        let v0 = m.stats[stat.boost_index()] as u32;
        if unmodified && self.has_pseudo(conds::WONDERROOM) {
            stat = match stat {
                Stat::Def => Stat::Spd,
                Stat::Spd => Stat::Def,
                s => s,
            };
        }
        let mut v = v0;
        let mut boost = m.boosts[stat.boost_index()];
        if !unboosted {
            if !unmodified {
                run_field_hook(self.s, Hook::ModifyBoost);
                if self.any_handler(Ev::ModifyBoost) {
                    let mut ctx = Ctx::new(Some((side, slot)), None, Eff::None);
                    ctx.boosts = Boosts::full(&self.s.sides[side].mons[slot].boosts);
                    self.run_event(Ev::ModifyBoost, &mut ctx, R::True);
                    boost = ctx.boosts.get(stat.boost_index()).unwrap_or(0);
                }
            }
            v = apply_stage(v, boost);
        }
        if !unmodified {
            let (hook, ev) = match stat {
                Stat::Atk => (Hook::ModifyAtk, Ev::ModifyAtk),
                Stat::Def => (Hook::ModifyDef, Ev::ModifyDef),
                Stat::Spa => (Hook::ModifySpA, Ev::ModifySpA),
                Stat::Spd => (Hook::ModifySpD, Ev::ModifySpD),
                _ => (Hook::ModifySpe, Ev::ModifySpe),
            };
            // only an active holder's handlers run
            if self.s.sides[side].mons[slot].is_active {
                if self.s.sides[side].active as usize == slot && self.any_handler(ev) {
                    let mut ctx = Ctx::new(Some((side, slot)), None, Eff::None);
                    if let R::Num(x) = self.run_event(ev, &mut ctx, R::Num(v as i32)) {
                        v = x.max(0) as u32;
                    }
                }
                run_field_hook(self.s, hook);
            }
        }
        if stat == Stat::Spe && v > 10000 {
            v = 10000;
        }
        v
    }

    /// Champions getActionSpeed: getStat('spe'), negated under Trick Room (no 13-bit wrap).
    pub fn action_speed(&mut self, side: usize, slot: usize) -> i32 {
        let v = self.get_stat(side, slot, Stat::Spe, false, false) as i32;
        if self.trick_room() {
            -v
        } else {
            v
        }
    }

    /// battle.updateSpeed: every active, non-fainted Pokémon
    pub fn update_speed(&mut self) {
        for side in 0..SIDES {
            if self.s.sides[side].has_active && !self.s.active(side).fainted {
                let slot = self.s.sides[side].active as usize;
                let v = self.action_speed(side, slot);
                self.s.sides[side].mons[slot].speed = v;
            }
        }
    }

    /// getActionSpeed for a queued action: priority (moves) and speed.
    fn resolve_speed(&mut self, a: &mut Action) {
        if a.kind == ActionKind::Move {
            let mut pr = a.mv.data().priority as i16;
            // the move's onModifyPriority: Grassy Glide on Grassy Terrain, for a grounded user
            if run_move_hook(self.s, a.mv, Hook::ModifyPriority) && a.mv == moves::GRASSYGLIDE && a.has_pokemon() {
                if self.terrain() == conds::GRASSYTERRAIN && self.grounded((a.side as usize, a.slot as usize)) {
                    pr += 1;
                }
            }
            // runEvent('ModifyPriority', pokemon, null, move, priority): Prankster, Gale Wings, Triage
            if a.has_pokemon() && self.any_handler(Ev::ModifyPriority) {
                let mut am = ActiveMove::new(if a.move_index == STRUGGLE { moves::STRUGGLE } else { a.mv });
                if a.move_index == RECHARGE {
                    // Showdown's 'recharge' is no move: its category is undefined, not Status
                    am.category = Category::Physical;
                }
                am.prankster = a.prankster;
                let mut ctx = Ctx::with_move(Some((a.side as usize, a.slot as usize)), None, &am);
                if let R::Num(x) = self.run_event_chain(Ev::ModifyPriority, &mut ctx, R::Num(pr as i32)) {
                    pr = x as i16;
                }
                if let Some(x) = ctx.am {
                    a.prankster = x.prankster;
                }
            }
            run_field_hook(self.s, Hook::ModifyPriority);
            a.priority = pr * 10 + a.frac as i16;
        }
        a.speed = if a.has_pokemon() { self.action_speed(a.side as usize, a.slot as usize) } else { 1 };
    }

    // ------------------------------------------------------------------------------------------ order

    /// speedSort of the queue: selection sort, ties shuffled through the outcome source.
    pub fn sort_queue(&mut self) {
        let n = self.s.queue.len as usize;
        let mut sorted = 0;
        while sorted + 1 < n {
            let mut next = [0usize; MAX_QUEUE];
            let mut nn = 1;
            next[0] = sorted;
            for i in sorted + 1..n {
                let delta = compare(&self.s.queue.list[next[0]], &self.s.queue.list[i]);
                if delta < 0 {
                    continue;
                }
                if delta > 0 {
                    nn = 1;
                    next[0] = i;
                } else {
                    next[nn] = i;
                    nn += 1;
                }
            }
            for (k, &idx) in next[..nn].iter().enumerate() {
                if idx != sorted + k {
                    self.s.queue.list.swap(sorted + k, idx);
                }
            }
            if nn > 1 {
                self.shuffle_actions(sorted, sorted + nn);
            }
            sorted += nn;
        }
    }

    fn shuffle_actions(&mut self, start: usize, end: usize) {
        if end - start == 2 {
            let a = self.s.queue.list[start].side;
            let b = self.s.queue.list[start + 1].side;
            if a != b && a != NO_SIDE && b != NO_SIDE {
                let site = Site::new(Key::SpeedTie, None);
                if self.o.tie(&site, a, b) == b {
                    self.s.queue.list.swap(start, start + 1);
                }
                return;
            }
        }
        // more than two tied actions, or two of one side
        self.s.unimpl.note(EffectRef::Move(MoveId::NONE), Hook::Other);
    }

    /// Showdown's speedSort of two Pokémon (eachEvent, runSwitch): the order as sides.
    fn sort_two_pokemon(&mut self, sides: [usize; 2]) -> [usize; 2] {
        let a = self.s.active(sides[0]).speed;
        let b = self.s.active(sides[1]).speed;
        if a > b {
            sides
        } else if b > a {
            [sides[1], sides[0]]
        } else {
            let site = Site { effect: Effect::Pokemon, ..Site::new(Key::SpeedTie, None) };
            if self.o.tie(&site, sides[0] as u8, sides[1] as u8) == sides[0] as u8 {
                sides
            } else {
                [sides[1], sides[0]]
            }
        }
    }

    /// The active Pokémon in eachEvent's speed order (a tie draws): the sides and how many.
    pub fn speed_order(&mut self) -> ([usize; SIDES], usize) {
        let mut alive = [0usize; SIDES];
        let mut n = 0;
        for i in 0..SIDES {
            if self.s.sides[i].has_active && !self.s.active(i).fainted {
                alive[n] = i;
                n += 1;
            }
        }
        if n == 2 {
            (self.sort_two_pokemon([0, 1]), 2)
        } else {
            (alive, n)
        }
    }

    /// eachEvent: runs a hook on each active Pokémon in speed order.
    pub fn each_event(&mut self, hook: Hook) {
        let (order, n) = self.speed_order();
        for &side in &order[..n] {
            run_field_hook(self.s, hook);
            if hook == Hook::Update && self.s.sides[side].has_active && self.any_handler(Ev::Update) {
                let p = (side, self.s.sides[side].active as usize);
                let mut ctx = Ctx::new(Some(p), None, Eff::None);
                self.run_event(Ev::Update, &mut ctx, R::True);
            }
        }
    }

    /// queue.insertChoice: puts an action where it would have sorted, without re-sorting the rest.
    fn insert_choice(&mut self, mut a: Action) {
        if a.has_pokemon() {
            let (side, slot) = (a.side as usize, a.slot as usize);
            let v = self.action_speed(side, slot);
            self.s.sides[side].mons[slot].speed = v;
        }
        self.resolve_speed(&mut a);
        self.insert_resolved(&[a]);
    }

    /// The insertion step of insertChoice for actions already resolved (all at the first one's place).
    fn insert_resolved(&mut self, acts: &[Action]) {
        let a = acts[0];
        let mut first = None;
        let mut last = None;
        for (i, cur) in self.s.queue.as_slice().iter().enumerate() {
            let c = compare(&a, cur);
            if c <= 0 && first.is_none() {
                first = Some(i);
            }
            if c < 0 {
                last = Some(i);
                break;
            }
        }
        let index = match first {
            None => self.s.queue.len as usize,
            Some(f) => {
                let l = last.unwrap_or(self.s.queue.len as usize);
                if f == l {
                    f
                } else if l + 1 - f == 2 && a.side != self.s.queue.list[f].side && a.side != NO_SIDE {
                    let site = Site { effect: Effect::Insert, ..Site::new(Key::SpeedTie, None) };
                    let mine = a.side;
                    if self.o.tie(&site, mine, self.s.queue.list[f].side) == mine {
                        f
                    } else {
                        f + 1
                    }
                } else {
                    self.s.unimpl.note(EffectRef::Move(MoveId::NONE), Hook::Other);
                    f
                }
            }
        };
        for (k, x) in acts.iter().enumerate() {
            self.s.queue.insert(index + k, *x);
        }
    }

    fn cancel_action(&mut self, side: u8, slot: u8) {
        let mut i = 0;
        while i < self.s.queue.len as usize {
            let a = self.s.queue.list[i];
            if a.side == side && a.slot == slot && a.kind != ActionKind::RunSwitch {
                self.s.queue.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// The targetLoc of a move action: a locked move keeps the last one (lastMoveTargetLoc; a two-turn move
    /// the foe's), any other is resolved by getRandomTarget.
    fn action_tloc(&self, p: Pk, mv: MoveId, locked: bool) -> i8 {
        if locked {
            if self.s.sides[p.0].has_vol(conds::TWOTURNMOVE) {
                return 1;
            }
            return self.mon_at(p).last_move_tloc;
        }
        let mut t = mv.data().target;
        if mv == moves::CURSE && !self.has_type_pk(p, Type::Ghost) {
            t = Target::Self_;
        }
        match t {
            Target::Self_ | Target::All | Target::AllySide | Target::AllyTeam | Target::AdjacentAllyOrSelf => -1,
            Target::AdjacentAlly => 0,
            _ => 1,
        }
    }

    /// The actions resolveAction makes of a move choice: the turn-start callbacks of the move, the Mega
    /// Evolution, then the move.
    fn move_actions(&mut self, side: usize, index: u8, mv: MoveId, mega: bool, tloc: i8) -> ([Action; 4], usize) {
        let slot = self.s.sides[side].active;
        let base = Action { side: side as u8, slot, ..Default::default() };
        let mut out = [Action::default(); 4];
        let mut n = 0;
        let h = mv.data().handlers;
        if h.contains(&"priorityChargeCallback") {
            let mut x = Action { kind: ActionKind::PriorityChargeMove, order: order::PRIORITY_CHARGE, mv, ..base };
            self.resolve_speed(&mut x);
            out[n] = x;
            n += 1;
        }
        if mega {
            let mut m = Action { kind: ActionKind::MegaEvo, order: order::MEGA_EVO, ..base };
            self.resolve_speed(&mut m);
            out[n] = m;
            n += 1;
        }
        if h.contains(&"beforeTurnCallback") {
            let mut x = Action { kind: ActionKind::BeforeTurnMove, order: order::BEFORE_TURN_MOVE, mv, ..base };
            self.resolve_speed(&mut x);
            out[n] = x;
            n += 1;
        }
        run_field_hook(self.s, Hook::FractionalPriority);
        // runEvent('FractionalPriority', pokemon, null, move, 0): Quick Draw, Mycelium Might, Stall, in tenths
        let mut frac = 0i8;
        if self.any_handler(Ev::FractionalPriority) {
            let mut am = ActiveMove::new(if index == STRUGGLE { moves::STRUGGLE } else { mv });
            if index == RECHARGE {
                am.category = Category::Physical;
            }
            let mut ctx = Ctx::with_move(Some((side, slot as usize)), None, &am);
            if let R::Num(x) = self.run_event_chain(Ev::FractionalPriority, &mut ctx, R::Num(0)) {
                frac = x as i8;
            }
        }
        // originalTarget: the Pokémon at targetLoc now
        let orig_target = match tloc {
            1 => pk_code(self.foe_of((side, slot as usize))),
            -1 => pk_code((side, slot as usize)),
            _ => NO_MON,
        };
        let curse_self = mv == moves::CURSE && !self.has_type_pk((side, slot as usize), Type::Ghost);
        let mut a = Action { kind: ActionKind::Move, order: order::MOVE, move_index: index, mv, mega, frac, tloc, orig_target, curse_self, ..base };
        self.resolve_speed(&mut a);
        out[n] = a;
        n += 1;
        (out, n)
    }

    /// queue.changeAction(pokemon, move): Encore replacing a queued move
    pub fn change_action_move(&mut self, p: Pk, mv: MoveId) {
        self.cancel_action(p.0 as u8, p.1 as u8);
        let index = self.mon_at(p).moves().iter().position(|ms| ms.id == mv).map(|i| i as u8).unwrap_or(NOSLOT);
        let v = self.action_speed(p.0, p.1);
        self.s.sides[p.0].mons[p.1].speed = v;
        let tloc = self.action_tloc(p, mv, false);
        let (acts, n) = self.move_actions(p.0, index, mv, false, tloc);
        // insertChoice splices the resolved actions in together, where the first one sorts
        self.insert_resolved(&acts[..n]);
    }

    /// queue.prioritizeAction of a new move action (Instruct): order 3, at the front
    pub fn prioritize_move(&mut self, p: Pk, mv: MoveId, index: u8) {
        let tloc = match self.mon_at(p).last_move_tloc {
            0 => self.action_tloc(p, mv, false),
            t => t,
        };
        let (acts, _) = self.move_actions(p.0, index, mv, false, tloc);
        // resolveAction(...)[0]: the first action (a turn-start callback when the move has one)
        let mut a = acts[0];
        a.order = 3;
        self.s.queue.insert(0, a);
    }

    // ------------------------------------------------------------------------------------------ choices

    /// battle.commitChoices: the new actions are sorted and run before what was left of the turn.
    pub fn commit(&mut self, choices: [Option<super::options::Choice>; 2]) {
        self.update_speed();
        let old = self.s.queue;
        self.s.queue.clear();
        for side in 0..SIDES {
            if let Some(c) = choices[side] {
                self.add_choice(side, c);
            }
        }
        self.s.request = Request::Wait;
        for side in &mut self.s.sides {
            side.request = Request::Wait;
        }
        self.sort_queue();
        for a in old.as_slice() {
            self.s.queue.push(*a);
        }
        self.turn_loop();
    }

    fn add_choice(&mut self, side: usize, c: super::options::Choice) {
        use super::options::Choice;
        let slot = self.s.sides[side].active;
        let base = Action { side: side as u8, slot, ..Default::default() };
        match c {
            Choice::Move { index, mega } => {
                let p = (side, slot as usize);
                // a locked Pokémon uses its locked move whatever was chosen (side.chooseMove)
                let lock = self.locked_move(p);
                let (index, mv) = match lock {
                    Some(m) if m.is_none() => (RECHARGE, MoveId::NONE),
                    Some(m) => {
                        let i = self.mon_at(p).moves().iter().position(|ms| ms.id == m).map(|i| i as u8).unwrap_or(NOSLOT);
                        (i, m)
                    }
                    None => {
                        let mon = self.s.active(side);
                        let mv = if index == STRUGGLE { moves::STRUGGLE } else { mon.moves[index as usize].id };
                        (index, mv)
                    }
                };
                let mega = mega && index != STRUGGLE && index != RECHARGE;
                let tloc = self.action_tloc(p, mv, lock.is_some());
                let (acts, n) = self.move_actions(side, index, mv, mega, tloc);
                for a in &acts[..n] {
                    self.s.queue.push(*a);
                }
            }
            Choice::Switch(target) if self.s.request == Request::Switch && self.slot_cond(side, conds::REVIVALBLESSING).is_some() => {
                // side.chooseSwitch under Revival Blessing: the choice names a fainted Pokémon
                self.s.active_mut(side).switch_flag = false;
                let mut a = Action { kind: ActionKind::RevivalBlessing, order: order::REVIVAL_BLESSING, target, ..base };
                self.resolve_speed(&mut a);
                self.s.queue.push(a);
            }
            Choice::Switch(target) => {
                let insta = self.s.request == Request::Switch;
                let kind = if insta { ActionKind::InstaSwitch } else { ActionKind::Switch };
                let ord = if insta { order::INSTASWITCH } else { order::SWITCH };
                let m = self.s.active_mut(side);
                let src = if m.switch_flag { m.switch_flag_move } else { MoveId::NONE };
                m.switch_flag = false;
                m.switch_flag_move = MoveId::NONE;
                let mut a = Action { kind, order: ord, target, source_effect: src, ..base };
                self.resolve_speed(&mut a);
                self.s.queue.push(a);
            }
        }
    }

    // ------------------------------------------------------------------------------------------ turn

    /// battle.turnLoop
    pub fn turn_loop(&mut self) {
        if !self.s.mid_turn {
            self.insert_choice(Action { kind: ActionKind::BeforeTurn, order: order::BEFORE_TURN, side: NO_SIDE, ..Default::default() });
            self.s.queue.push(Action { kind: ActionKind::Residual, order: order::RESIDUAL, side: NO_SIDE, speed: 1, ..Default::default() });
            self.s.mid_turn = true;
        }
        while let Some(a) = self.s.queue.shift() {
            self.run_action(a);
            if self.s.request != Request::Wait || self.s.ended {
                return;
            }
        }
        self.end_turn();
        self.s.mid_turn = false;
        self.s.queue.clear();
    }

    fn run_action(&mut self, a: Action) {
        let mut residual_check = false;
        let mut residual_hp: [Option<(usize, u16)>; SIDES] = [None; SIDES];
        let orig_hp = if a.has_pokemon() { self.s.mon(a.side as usize, a.slot as usize).hp } else { 0 };
        match a.kind {
            ActionKind::Start => {
                for side in 0..SIDES {
                    if self.s.sides[side].pokemon_left > 0 {
                        self.s.sides[side].pokemon_left = TEAM as u8;
                    }
                }
                for side in 0..SIDES {
                    self.switch_in(side, 0, MoveId::NONE, false);
                }
                self.s.mid_turn = true;
            }
            ActionKind::Move => {
                let m = self.s.mon(a.side as usize, a.slot as usize);
                if !m.is_active || m.fainted {
                    return;
                }
                self.run_move(a);
            }
            ActionKind::BeforeTurnMove | ActionKind::PriorityChargeMove => {
                let m = self.s.mon(a.side as usize, a.slot as usize);
                if !m.is_active || m.fainted {
                    return;
                }
                let p = (a.side as usize, a.slot as usize);
                if a.kind == ActionKind::BeforeTurnMove {
                    self.move_before_turn(p, a.mv);
                } else {
                    self.move_priority_charge(p, a.mv);
                }
            }
            ActionKind::MegaEvo => self.run_mega_evo(a.side as usize, a.slot as usize),
            ActionKind::Switch | ActionKind::InstaSwitch => {
                if self.logging() {
                    // a chosen switch or a pivot move's; not Baton Pass, Shed Tail, a faint or an item's or
                    // ability's switch (Eject Button, Emergency Exit)
                    let src = a.source_effect;
                    self.withdraw_next = if a.kind == ActionKind::Switch {
                        Some(None)
                    } else if !src.is_none() && matches!(src.data().self_switch, SelfSwitch::Yes) {
                        Some(Some(src))
                    } else {
                        None
                    };
                }
                self.switch_in(a.side as usize, a.target as usize, a.source_effect, false);
                self.withdraw_next = None;
            }
            ActionKind::RunSwitch => self.run_switch(a.side as usize, a.slot as usize),
            ActionKind::RevivalBlessing => self.revive(a.side as usize, a.target as usize),
            ActionKind::BeforeTurn => self.each_event(Hook::BeforeTurn),
            ActionKind::Residual => {
                self.clear_active(true);
                self.update_speed();
                for side in 0..SIDES {
                    let sd = &self.s.sides[side];
                    if sd.has_active && sd.active_mon().is_active {
                        residual_hp[side] = Some((sd.active as usize, sd.active_mon().hp));
                    }
                }
                self.residual();
                residual_check = true;
            }
        }

        // phazing (Roar, Dragon Tail)
        for side in 0..SIDES {
            if self.s.sides[side].has_active && self.s.active(side).force_switch_flag {
                if self.s.active(side).hp > 0 {
                    self.drag_in(side);
                }
                self.s.active_mut(side).force_switch_flag = false;
            }
        }
        self.clear_active(false);

        self.resolve_faints(false, true, true);
        if self.s.ended {
            return;
        }
        let next = self.s.queue.peek().map(|x| x.kind);
        if next.is_none() {
            self.check_fainted();
        }
        if a.kind != ActionKind::Start && next != Some(ActionKind::InstaSwitch) {
            self.each_event(Hook::Update);
            if residual_check {
                run_field_hook(self.s, Hook::EmergencyExit);
                for side in 0..SIDES {
                    if let Some((slot, hp)) = residual_hp[side] {
                        self.emergency_exit((side, slot), None, hp);
                    }
                }
            }
        }
        if a.kind == ActionKind::RunSwitch {
            run_field_hook(self.s, Hook::EmergencyExit);
            self.emergency_exit((a.side as usize, a.slot as usize), None, orig_hp);
        }

        let mut switches = [false; SIDES];
        for side in 0..SIDES {
            switches[side] = self.s.sides[side].has_active && self.s.active(side).switch_flag;
        }
        for side in 0..SIDES {
            if switches[side] && !self.can_switch(side) {
                // the request after Revival Blessing stays: it picks a fainted Pokémon
                if self.slot_cond(side, conds::REVIVALBLESSING).is_some() {
                    continue;
                }
                let m = self.s.active_mut(side);
                m.switch_flag = false;
                m.switch_flag_move = MoveId::NONE;
                switches[side] = false;
            } else if switches[side] {
                let m = self.s.active(side);
                if m.hp > 0 && m.switch_flag && m.switch_flag_move != moves::REVIVALBLESSING {
                    self.run_pursuit_activation(side);
                    if self.s.ended {
                        return;
                    }
                }
            }
        }
        if switches.iter().any(|x| *x) {
            self.make_request(Request::Switch);
            return;
        }

        if self.s.queue.peek().map(|x| x.kind) == Some(ActionKind::Move) {
            self.update_speed();
            for i in 0..self.s.queue.len as usize {
                let mut q = self.s.queue.list[i];
                if q.has_pokemon() {
                    self.resolve_speed(&mut q);
                    self.s.queue.list[i] = q;
                }
            }
            self.sort_queue();
        }
    }

    /// battle.checkFainted: fainted actives must be replaced when their side can switch.
    fn check_fainted(&mut self) {
        for side in 0..SIDES {
            if !self.can_switch(side) {
                continue;
            }
            if self.s.sides[side].has_active && self.s.active(side).fainted {
                let m = self.s.active_mut(side);
                // status 'fnt'
                m.status = Status::None;
                m.switch_flag = true;
            }
        }
    }

    /// battle.canSwitch: a benched Pokémon that has not fainted
    pub fn can_switch(&self, side: usize) -> bool {
        let sd = &self.s.sides[side];
        sd.pokemon_left > 0
            && (0..TEAM).any(|i| (i as u8 != sd.active || !sd.has_active) && !sd.mons[i].fainted)
    }

    pub fn make_request(&mut self, kind: Request) {
        self.s.request = kind;
        for side in 0..SIDES {
            let sd = &mut self.s.sides[side];
            sd.revealed = 0;
            sd.request = match kind {
                Request::Move if sd.pokemon_left > 0 => Request::Move,
                Request::Switch if sd.pokemon_left > 0 && sd.has_active && sd.active_mon().switch_flag => Request::Switch,
                Request::TeamPreview => Request::TeamPreview,
                _ => Request::Wait,
            };
        }
        if kind == Request::Move {
            // getMoveRequestData: a locked Pokémon is trapped
            for side in 0..SIDES {
                let sd = &self.s.sides[side];
                if sd.has_active && sd.request == Request::Move {
                    let p = (side, sd.active as usize);
                    if self.locked_move(p).is_some() {
                        let m = self.mon_at_mut(p);
                        m.trapped = true;
                        m.trap_hidden = false;
                    }
                }
            }
        }
    }

    /// battle.endTurn
    fn end_turn(&mut self) {
        self.s.turn += 1;
        for side in 0..SIDES {
            if !self.s.sides[side].has_active {
                continue;
            }
            let turn = self.s.turn;
            let p = (side, self.s.sides[side].active as usize);
            {
                let m = self.s.active_mut(side);
                m.move_this_turn = MoveId::NONE;
                m.newly_switched = false;
                m.move_last_turn_result = m.move_this_turn_result;
                m.move_this_turn_result = MoveResult::Undefined;
                if turn != 1 {
                    m.used_item_this_turn = false;
                    m.stats_raised_this_turn = false;
                    m.stats_lowered_this_turn = false;
                    m.hurt_this_turn = None;
                }
                for ms in m.moves.iter_mut() {
                    ms.disabled = false;
                    ms.disabled_hidden = false;
                }
            }
            let mut ctx = Ctx::new(Some(p), None, Eff::None);
            self.run_event(Ev::DisableMove, &mut ctx, R::True);
            run_field_hook(self.s, Hook::DisableMove);
            let nm = self.s.active(side).nmoves as usize;
            for i in 0..nm {
                let id = self.s.active(side).moves[i].id;
                if id == moves::FAKEOUT || id == moves::FIRSTIMPRESSION {
                    self.move_disable_move(p, id);
                } else {
                    run_move_hook(self.s, id, Hook::DisableMove);
                }
                let m = self.s.active_mut(side);
                if id.data().has_flag(flag::CANTUSETWICE) && m.last_move == id {
                    m.moves[i].disabled = true;
                }
            }
            // attackedBy: entries of attackers still in play stay, for earlier turns
            {
                let mut keep = [Attack::default(); MAX_ATTACKS];
                let mut n = 0;
                let m = self.s.active(side);
                let (atts, na) = (m.attacked, m.nattacked as usize);
                for &a in &atts[..na] {
                    if pk_of(a.source).map_or(false, |s| self.is_active_pk(s)) {
                        keep[n] = Attack { this_turn: false, ..a };
                        n += 1;
                    }
                }
                let m = self.s.active_mut(side);
                m.attacked = keep;
                m.nattacked = n as u8;
            }
            {
                let m = self.s.active_mut(side);
                m.trapped = false;
                m.trap_hidden = false;
            }
            let mut ctx = Ctx::new(Some(p), None, Eff::None);
            self.run_event(Ev::TrapPokemon, &mut ctx, R::True);
            run_field_hook(self.s, Hook::TrapPokemon);
            run_field_hook(self.s, Hook::MaybeTrapPokemon);
            if self.s.active(side).fainted {
                continue;
            }
            self.s.active_mut(side).active_turns += 1;
        }
        for sd in &mut self.s.sides {
            sd.fainted_last_turn = sd.fainted_this_turn;
            sd.fainted_this_turn = false;
        }
        if self.s.turn > 1000 {
            self.win(None);
            return;
        }
        let turn = self.s.turn;
        self.row(Row::Turn(turn));
        self.make_request(Request::Move);
    }

    // ------------------------------------------------------------------------------------------ switching

    /// battleActions.switchIn; `source` is the switch's sourceEffect (a pivot move), NONE otherwise
    pub fn switch_in(&mut self, side: usize, slot: usize, source: MoveId, is_drag: bool) -> bool {
        if self.s.sides[side].mons[slot].is_active {
            return false;
        }
        let had_active = self.s.sides[side].has_active;
        let old = self.s.sides[side].active as usize;
        if had_active && self.s.sides[side].mons[old].hp > 0 {
            if let Some(pivot) = self.withdraw_next.take() {
                self.row(Row::Withdraw { p: (side as u8, old as u8), pivot });
            }
            self.s.sides[side].mons[old].being_called_back = true;
            let copy = if source.is_none() { SelfSwitch::No } else { source.data().self_switch };
            if !is_drag {
                self.run_pursuit_activation(side);
                if self.s.ended {
                    return false;
                }
            }
            run_field_hook(self.s, Hook::SwitchOut);
            if self.any_handler(Ev::SwitchOut) {
                let mut ctx = Ctx::new(Some((side, old)), None, Eff::None);
                self.run_event(Ev::SwitchOut, &mut ctx, R::True);
            }
            if self.s.sides[side].mons[old].hp == 0 {
                return false;
            }
            self.mon_hook(side, Hook::End);
            self.cancel_action(side as u8, old as u8);
            match copy {
                SelfSwitch::CopyVolatile | SelfSwitch::ShedTail => self.copy_volatile_from(side, old, slot, copy),
                _ => self.clear_volatile(side, old, true),
            }
        }
        if had_active {
            let sd = &mut self.s.sides[side];
            let m = &mut sd.mons[old];
            m.is_active = false;
            m.is_started = false;
            m.used_item_this_turn = false;
            m.stats_raised_this_turn = false;
            m.stats_lowered_this_turn = false;
            m.skip_pursuit = false;
            if m.fainted {
                m.status = Status::None;
            }
            // side.pokemon positions: the two swap
            let pi = sd.order.iter().position(|&x| x as usize == slot).unwrap();
            let po = sd.order.iter().position(|&x| x as usize == old).unwrap();
            sd.order.swap(pi, po);
        }
        let sd = &mut self.s.sides[side];
        sd.active = slot as u8;
        sd.has_active = true;
        let m = &mut sd.mons[slot];
        m.is_active = true;
        m.active_turns = 0;
        m.active_move_actions = 0;
        for ms in m.moves.iter_mut() {
            ms.used = false;
        }
        // abilityState and itemState start again (the ability's id is never empty: No Ability has one)
        self.init_ability_state((side, slot));
        let order = if self.s.sides[side].mons[slot].item.is_none() {
            0
        } else {
            self.s.effect_order = self.s.effect_order.wrapping_add(1);
            self.s.effect_order
        };
        self.s.sides[side].mons[slot].item_order = order;
        self.s.sides[side].mons[slot].being_called_back = false;
        run_field_hook(self.s, Hook::BeforeSwitchIn);
        // Illusion's onBeforeSwitchIn: disguised as the last Pokémon behind it that has not fainted
        if self.has_ability((side, slot), abilities::ILLUSION) {
            let sd = &self.s.sides[side];
            let up = sd.order[1..].iter().any(|&x| !sd.mons[x as usize].fainted);
            self.s.sides[side].mons[slot].illusion = up;
        }
        self.row(Row::Enter { p: (side as u8, slot as u8), forced: is_drag });
        if is_drag {
            self.run_switch(side, slot);
        } else {
            self.insert_choice(Action {
                kind: ActionKind::RunSwitch,
                order: order::RUN_SWITCH,
                side: side as u8,
                slot: slot as u8,
                ..Default::default()
            });
        }
        self.s.sides[side].mons[slot].hp > 0
    }

    /// battleActions.dragIn: a random benched Pokémon replaces the active one
    fn drag_in(&mut self, side: usize) {
        let sd = &self.s.sides[side];
        let mut cand = [0i32; TEAM];
        let mut n = 0;
        for &slot in &sd.order {
            if slot == sd.active {
                continue;
            }
            if !sd.mons[slot as usize].fainted {
                cand[n] = slot as i32;
                n += 1;
            }
        }
        if n == 0 || sd.pokemon_left == 0 {
            return;
        }
        let eff = if self.s.active_move.is_none() { MoveId::NONE } else { self.s.active_move };
        // the attacker itself dragged out: Red Card (the oracle's site)
        let red = !eff.is_none() && pk_of(self.s.active_pokemon).map_or(false, |p| p.0 == side);
        let effect = if red {
            Effect::One(EffectRef::Item(items::REDCARD))
        } else if eff.is_none() {
            Effect::DragIn
        } else {
            Effect::One(EffectRef::Move(eff))
        };
        let site = Site { effect, ..Site::new(Key::Pick, Some(side as u8)) };
        let pick = if n == 1 { cand[0] } else { self.o.sample(&site, &cand[..n]) } as usize;
        let oldp = (side, self.s.sides[side].active as usize);
        if self.mon_at(oldp).hp == 0 {
            return;
        }
        let mut ctx = Ctx::new(Some(oldp), None, Eff::None);
        if !self.run_event(Ev::DragOut, &mut ctx, R::True).truthy() {
            return;
        }
        run_field_hook(self.s, Hook::DragOut);
        self.switch_in(side, pick, MoveId::NONE, true);
    }

    /// battleActions.runPursuitActivation for the side's active Pokémon
    fn run_pursuit_activation(&mut self, side: usize) {
        if self.s.active(side).skip_pursuit {
            return;
        }
        run_field_hook(self.s, Hook::BeforeSwitchOut);
        self.clear_active(false);
        self.each_event(Hook::Update);
        let m = self.s.active_mut(side);
        m.skip_pursuit = !(m.hp > 0 && !m.switch_flag);
    }

    /// battleActions.runSwitch: every queued switch-in runs its entry effects as one batch.
    fn run_switch(&mut self, side: usize, slot: usize) {
        let mut switchers = [(side, slot); SIDES];
        let mut n = 1;
        while self.s.queue.peek().map(|a| a.kind) == Some(ActionKind::RunSwitch) {
            let a = self.s.queue.shift().unwrap();
            if n < SIDES {
                switchers[n] = (a.side as usize, a.slot as usize);
                n += 1;
            }
        }
        let switchers = &switchers[..n];
        // battle.speedOrder: the active Pokémon (fainted ones too) by speed, a tie drawn
        let order = if self.s.sides.iter().all(|sd| sd.has_active) { self.sort_two_pokemon([0, 1]) } else { [0, 1] };
        // fieldEvent('SwitchIn'): each switch-in's handlers in speed order (Toxic's counter, Healing Wish,
        // hazards); abilities and items are recorded
        run_field_hook(self.s, Hook::SwitchIn);
        let mut list = [(0usize, 0usize); SIDES];
        let mut k = 0;
        for &sd in &order {
            if let Some(&x) = switchers.iter().find(|x| x.0 == sd) {
                list[k] = x;
                k += 1;
            }
        }
        if self.switch_in_handlers(&list[..k], order) {
            return;
        }
        for &(sd, sl) in switchers {
            let m = &mut self.s.sides[sd].mons[sl];
            if m.hp > 0 {
                m.is_started = true;
            }
        }
    }

    /// The per-Pokémon part of pokemon.clearVolatile (Champions).
    fn clear_mon(&mut self, side: usize, slot: usize, include_switch_flags: bool) {
        let m = &mut self.s.sides[side].mons[slot];
        m.boosts = [0; 7];
        if m.transformed {
            m.moves = m.base_moves;
            m.nmoves = m.base_moves.iter().filter(|x| !x.id.is_none()).count() as u8;
            m.transformed = false;
        }
        m.ability = m.base_ability;
        if include_switch_flags {
            m.switch_flag = false;
            m.switch_flag_move = MoveId::NONE;
            m.force_switch_flag = false;
        }
        m.last_move = MoveId::NONE;
        m.last_move_used = MoveId::NONE;
        m.move_this_turn = MoveId::NONE;
        m.move_last_turn_result = MoveResult::Undefined;
        m.move_this_turn_result = MoveResult::Undefined;
        m.last_damage = 0;
        m.nattacked = 0;
        m.hurt_this_turn = None;
        m.newly_switched = true;
        m.times_attacked = 0;
        let base = m.base_species;
        self.set_species(side, slot, base);
    }

    /// pokemon.clearVolatile (Champions): stages, volatiles (and their links), last move; base form
    pub fn clear_volatile(&mut self, side: usize, slot: usize, include_switch_flags: bool) {
        if self.s.sides[side].has_active && self.s.sides[side].active as usize == slot {
            self.unlink_all(side, slot);
            let sd = &mut self.s.sides[side];
            sd.nvolatiles = 0;
            sd.volatiles = [Volatile::default(); MAX_VOLATILES];
        }
        self.clear_mon(side, slot, include_switch_flags);
    }

    /// removeLinkedVolatiles for every linked volatile of the active Pokémon
    fn unlink_all(&mut self, side: usize, slot: usize) {
        let vols: Vec<Volatile> = self.s.sides[side].volatiles().iter().filter(|v| v.linked).copied().collect();
        for v in vols {
            let Some(other) = pk_of(v.source) else { continue };
            let counterpart = if v.id == conds::TRAPPED { conds::TRAPPER } else { conds::TRAPPED };
            if self.is_active_pk(other) {
                let linked = self.s.sides[other.0].vol(counterpart).map_or(false, |x| x.linked && x.source == pk_code((side, slot)));
                if linked {
                    self.remove_volatile(other, counterpart);
                }
            }
        }
    }

    /// pokemon.copyVolatileFrom (Baton Pass, Shed Tail) as part of a switch from `old` to `new`
    fn copy_volatile_from(&mut self, side: usize, old: usize, new: usize, why: SelfSwitch) {
        self.clear_mon(side, new, true);
        if why != SelfSwitch::ShedTail {
            let b = self.s.sides[side].mons[old].boosts;
            self.s.sides[side].mons[new].boosts = b;
        }
        let kept: Vec<Volatile> = self.s.sides[side]
            .volatiles()
            .iter()
            .filter(|v| if why == SelfSwitch::ShedTail { v.id == conds::SUBSTITUTE } else { !v.id.data().no_copy })
            .copied()
            .collect();
        self.unlink_all(side, old);
        {
            let sd = &mut self.s.sides[side];
            sd.nvolatiles = 0;
            sd.volatiles = [Volatile::default(); MAX_VOLATILES];
        }
        self.clear_mon(side, old, true);
        let sd = &mut self.s.sides[side];
        for (i, v) in kept.iter().enumerate() {
            sd.volatiles[i] = *v;
        }
        sd.nvolatiles = kept.len() as u8;
        for v in kept {
            self.cond_copy((side, new), v.id);
        }
    }

    /// pokemon.setSpecies: types, weight and stats of a form; speed back to the stored stat.
    pub fn set_species(&mut self, side: usize, slot: usize, sp: SpeciesId) {
        run_field_hook(self.s, Hook::ModifySpecies);
        let m = &mut self.s.sides[side].mons[slot];
        let d = sp.data();
        m.species = sp;
        let n = if d.types[1] == Type::Typeless { 1 } else { 2 };
        m.set_types(&d.types[..n]);
        m.weight_hg = d.weight_hg;
        let stats = [Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe];
        for (i, st) in stats.iter().enumerate() {
            m.stats[i] = stat_from_points(d.base_stats[*st as usize], m.evs[*st as usize], *st, m.nature);
        }
        if m.maxhp == 0 {
            let hp = if d.max_hp > 0 { d.max_hp } else { stat_from_points(d.base_stats[0], m.evs[0], Stat::Hp, m.nature) };
            m.maxhp = hp;
            m.base_maxhp = hp;
            m.hp = hp;
        }
        m.speed = m.stats[4] as i32;
    }

    /// The handlers of one Pokémon's own effects (singleEvent on its ability and item).
    fn mon_hook(&mut self, side: usize, hook: Hook) {
        let mon = *self.s.active(side);
        let mk = masks();
        if hook == Hook::End {
            let p = (side, self.s.sides[side].active as usize);
            self.ability_end(p, mon.ability);
        }
        if mk.abilities_unimpl[mon.ability.0 as usize] & hook.bit() != 0 {
            self.s.unimpl.note(EffectRef::Ability(mon.ability), hook);
        }
        if mk.items_unimpl[mon.item.0 as usize] & hook.bit() != 0 {
            self.s.unimpl.note(EffectRef::Item(mon.item), hook);
        }
    }

    /// battleActions.runMegaEvo
    fn run_mega_evo(&mut self, side: usize, slot: usize) {
        let Some(mega) = self.s.sides[side].mons[slot].can_mega else { return };
        // formeChange(mega, item, permanent)
        self.set_species(side, slot, mega);
        self.s.sides[side].mons[slot].base_species = mega;
        if self.logging() {
            let w = (side as u8, slot as u8);
            let stone = self.s.sides[side].mons[slot].item;
            self.row(Row::MegaStone { p: w, stone });
            self.row(Row::Mega { p: w, form: mega });
        }
        self.update_max_hp(side, slot);
        let ability = mega.data().abilities[0];
        // Mega Evolution counts as an action for Truant
        self.s.sides[side].mons[slot].move_this_turn_result = MoveResult::True;
        self.set_form_ability(side, slot, ability);
        self.s.sides[side].mons[slot].base_ability = ability;
        for m in self.s.sides[side].mons.iter_mut() {
            m.can_mega = None;
        }
        self.s.sides[side].mega_spent = true;
        run_field_hook(self.s, Hook::AfterMega);
        // runEvent('AfterMega', pokemon): White Herb, Opportunist
        if self.any_handler(Ev::AfterMega) {
            let mut ctx = Ctx::new(Some((side, slot)), None, Eff::None);
            self.run_event(Ev::AfterMega, &mut ctx, R::True);
        }
    }

    pub fn update_max_hp(&mut self, side: usize, slot: usize) {
        let m = &mut self.s.sides[side].mons[slot];
        let d = m.species.data();
        let new = stat_from_points(d.base_stats[0], m.evs[0], Stat::Hp, m.nature);
        if new == m.base_maxhp {
            return;
        }
        m.base_maxhp = new;
        m.hp = if m.hp == 0 { 0 } else { (new as i32 - (m.maxhp as i32 - m.hp as i32)).max(1) as u16 };
        m.maxhp = new;
    }

    /// pokemon.setAbility from a form change: End of the old ability, Start of the new one.
    fn set_form_ability(&mut self, side: usize, slot: usize, ability: AbilityId) {
        self.set_ability((side, slot), ability, None, true, false);
    }

    // ------------------------------------------------------------------------------------------ fainting

    /// pokemon.faint with no source: queued, resolved by resolve_faints
    pub fn faint(&mut self, side: usize, slot: usize) -> u16 {
        self.faint_by((side, slot), None, false) as u16
    }

    /// battle.resolveFaints; returns true when the battle ended.
    pub fn resolve_faints(&mut self, last_faint_first: bool, check_win_if_no_faints: bool, mut check_win: bool) -> bool {
        if self.s.ended {
            return false;
        }
        let n = self.s.nfaints as usize;
        if n == 0 {
            return check_win_if_no_faints && self.check_win(None);
        }
        if last_faint_first {
            self.s.faint_queue[..n].rotate_right(1);
        }
        let mut last = None;
        while self.s.nfaints > 0 {
            let left = self.s.nfaints;
            let f = self.s.faint_queue[0];
            self.s.faint_queue.copy_within(1..MAX_FAINTS, 0);
            self.s.nfaints -= 1;
            last = Some(f);
            let (side, slot) = (f.side as usize, f.slot as usize);
            if self.s.sides[side].mons[slot].fainted {
                continue;
            }
            run_field_hook(self.s, Hook::BeforeFaint);
            self.row(Row::Faint { p: (side as u8, slot as u8) });
            {
                let sd = &mut self.s.sides[side];
                if sd.pokemon_left > 0 {
                    sd.pokemon_left -= 1;
                }
                sd.total_fainted = sd.total_fainted.saturating_add(1);
            }
            let mut ctx = Ctx::new(Some((side, slot)), pk_of(f.source), Eff::None);
            ctx.arg = (f.by_move && !f.future) as u16;
            self.run_event(Ev::Faint, &mut ctx, R::True);
            run_field_hook(self.s, Hook::Faint);
            if self.s.sides[side].active as usize == slot {
                self.fainting = true;
                self.mon_hook(side, Hook::End);
                self.fainting = false;
            }
            self.clear_volatile(side, slot, false);
            // the status stays until checkFainted or the switch-out (its residual handler is still sorted)
            let m = &mut self.s.sides[side].mons[slot];
            m.fainted = true;
            m.illusion = false;
            m.is_active = false;
            m.is_started = false;
            self.s.sides[side].fainted_this_turn = true;
            if self.s.nfaints >= left {
                check_win = true;
            }
        }
        if check_win && self.check_win(last) {
            return true;
        }
        if let Some(f) = last {
            run_field_hook(self.s, Hook::AfterFaint);
            // runEvent('AfterFaint', target, source, effect, length): Moxie and the like on the source
            if self.any_handler(Ev::AfterFaint) {
                let mut ctx = Ctx::new(Some((f.side as usize, f.slot as usize)), pk_of(f.source), Eff::None);
                ctx.arg = (f.by_move as u16) << 8;
                self.run_event_chain(Ev::AfterFaint, &mut ctx, R::Num(n as i32));
            }
        }
        false
    }

    /// battle.checkWin: when both sides are out, the side whose Pokémon fainted last wins.
    pub fn check_win(&mut self, last: Option<FaintEntry>) -> bool {
        if self.s.sides.iter().all(|s| s.pokemon_left == 0) {
            self.win(last.map(|f| f.side));
            return true;
        }
        for side in 0..SIDES {
            if self.s.sides[1 - side].pokemon_left == 0 {
                self.win(Some(side as u8));
                return true;
            }
        }
        false
    }

    pub fn win(&mut self, side: Option<u8>) {
        if self.s.ended {
            return;
        }
        self.s.winner = match side {
            Some(s) => Winner::Side(s),
            None => Winner::Tie,
        };
        self.s.ended = true;
        let winner = self.s.winner;
        self.row(Row::End { winner });
        self.s.request = Request::Wait;
        for sd in &mut self.s.sides {
            sd.request = Request::Wait;
        }
    }

    // ------------------------------------------------------------------------------------------ residual

    /// fieldEvent('Residual'): every effect with a residual handler or a duration, in Showdown's order
    /// (order, priority, speed, sub-order), durations counted down and ended.
    fn residual(&mut self) {
        let mut hs = super::residual::collect(self.s);
        super::residual::sort(self, &mut hs);
        for h in hs.iter() {
            if super::residual::run(self, h) {
                return;
            }
        }
    }

    /// runEvent('EmergencyExit', pokemon, source, null, originalHp)
    pub fn emergency_exit(&mut self, p: Pk, source: Option<Pk>, orig_hp: u16) {
        if !self.any_handler(Ev::EmergencyExit) {
            return;
        }
        let mut ctx = Ctx::new(Some(p), source, Eff::None);
        self.run_event_chain(Ev::EmergencyExit, &mut ctx, R::Num(orig_hp as i32));
    }

    pub fn note(&mut self, e: EffectRef, hook: Hook) {
        self.s.unimpl.note(e, hook);
    }
}
