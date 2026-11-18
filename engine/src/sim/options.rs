//! Legal choices per side (Showdown's request: pokemon.getMoveRequestData, side.choose*), the public step
//! API and the constructor from teams and picks (team preview: unbrought Pokémon never exist).

use super::battle::{order, Sim, STRUGGLE};
use super::calc::stat_from_points;
use super::log::Row;
use super::data::*;
use super::outcome::Outcomes;
use super::state::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// a move by position in the move list (or STRUGGLE), with or without Mega Evolution
    Move { index: u8, mega: bool },
    /// switch to a slot
    Switch(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    Wait,
    Turn,
    Replace,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SideOptions {
    pub kind: Kind,
    /// positions that may be chosen
    pub moves: [bool; 4],
    /// no move can be chosen: the only move is Struggle
    pub struggle: bool,
    /// the only choice is a move outside the slots ("recharge", or a locked move the slots lack)
    pub forced: Option<&'static str>,
    pub mega: bool,
    pub switches: [bool; TEAM],
    /// switches the game's request offers but refuses (a trap by an ability not yet revealed)
    pub hidden_switches: [bool; TEAM],
    /// moves the request offers but the game refuses (disabled by Imprison, not yet revealed)
    pub hidden_moves: [bool; 4],
}

impl SideOptions {
    pub fn move_list(&self) -> Vec<u8> {
        (0..4).filter(|&i| self.moves[i as usize]).collect()
    }
    pub fn switch_list(&self) -> Vec<u8> {
        (0..TEAM as u8).filter(|&i| self.switches[i as usize]).collect()
    }
    pub fn hidden_switch_list(&self) -> Vec<u8> {
        (0..TEAM as u8).filter(|&i| self.hidden_switches[i as usize]).collect()
    }
    pub fn hidden_move_list(&self) -> Vec<u8> {
        (0..4).filter(|&i| self.hidden_moves[i as usize]).collect()
    }
    /// why the game refuses a choice it offers: "trapped" or "disabled"
    pub fn refusal(&self, c: Choice) -> Option<&'static str> {
        match (self.kind, c) {
            (Kind::Turn, Choice::Switch(s)) if (s as usize) < TEAM && self.hidden_switches[s as usize] => Some("trapped"),
            (Kind::Turn, Choice::Move { index, .. }) if (index as usize) < 4 && self.hidden_moves[index as usize] => Some("disabled"),
            _ => None,
        }
    }
    /// whether a choice is legal now
    pub fn allows(&self, c: Choice) -> bool {
        match (self.kind, c) {
            (Kind::Turn, Choice::Move { index: STRUGGLE, mega }) => (self.struggle || self.forced.is_some()) && !mega,
            (Kind::Turn, Choice::Move { index, mega }) => {
                (index as usize) < 4 && self.moves[index as usize] && (!mega || self.mega)
            }
            (Kind::Turn | Kind::Replace, Choice::Switch(s)) => (s as usize) < TEAM && self.switches[s as usize],
            _ => false,
        }
    }

    /// every legal choice
    pub fn choices(&self) -> Vec<Choice> {
        let mut out = Vec::new();
        match self.kind {
            Kind::Turn => {
                if self.struggle || self.forced.is_some() {
                    out.push(Choice::Move { index: STRUGGLE, mega: false });
                }
                for i in self.move_list() {
                    out.push(Choice::Move { index: i, mega: false });
                    if self.mega {
                        out.push(Choice::Move { index: i, mega: true });
                    }
                }
                for s in self.switch_list() {
                    out.push(Choice::Switch(s));
                }
            }
            Kind::Replace => {
                for s in self.switch_list() {
                    out.push(Choice::Switch(s));
                }
            }
            _ => {}
        }
        out
    }
}

/// A set as brought to team preview.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MonSpec {
    pub species: SpeciesId,
    pub ability: AbilityId,
    pub item: ItemId,
    pub moves: [MoveId; 4],
    pub nmoves: u8,
    pub nature: u8,
    /// stat points: hp, atk, def, spa, spd, spe
    pub evs: [u8; 6],
    /// 0 none, 1 M, 2 F
    pub gender: u8,
}

impl State {
    /// The legal choices of each side now.
    pub fn options(&self) -> [SideOptions; SIDES] {
        let mut out = [SideOptions::default(); SIDES];
        for side in 0..SIDES {
            out[side] = self.side_options(side);
        }
        out
    }

    fn bench(&self, side: usize) -> [bool; TEAM] {
        let sd = &self.sides[side];
        let mut b = [false; TEAM];
        for i in 0..TEAM {
            b[i] = !sd.mons[i].fainted && !(sd.has_active && sd.active as usize == i);
        }
        b
    }

    fn side_options(&self, side: usize) -> SideOptions {
        if self.ended {
            return SideOptions { kind: Kind::Done, ..Default::default() };
        }
        let sd = &self.sides[side];
        match sd.request {
            Request::Wait | Request::TeamPreview => SideOptions::default(),
            Request::Switch => {
                // after Revival Blessing the choice names a fainted Pokemon
                let reviving = sd.slot_conds().iter().any(|c| c.id == conds::REVIVALBLESSING);
                let mut sw = self.bench(side);
                if reviving {
                    for i in 0..TEAM {
                        sw[i] = sd.mons[i].fainted;
                    }
                }
                SideOptions { kind: Kind::Replace, switches: sw, ..Default::default() }
            }
            Request::Move => {
                let m = sd.active_mon();
                let mut o = SideOptions { kind: Kind::Turn, ..Default::default() };
                match self.locked_move(side) {
                    Some(mv) if mv.is_none() => o.forced = Some("recharge"),
                    Some(mv) => match m.moves().iter().position(|ms| ms.id == mv) {
                        Some(i) => o.moves[i] = true,
                        None => o.forced = Some(mv.id()),
                    },
                    None => {
                        for (i, ms) in m.moves().iter().enumerate() {
                            o.moves[i] = !ms.disabled && ms.pp > 0;
                            o.hidden_moves[i] = ms.disabled && ms.disabled_hidden && ms.pp > 0 && sd.revealed & (2 << i) == 0;
                        }
                        // with every move disabled any move choice is Struggle (also when the disables are hidden)
                        if !o.moves.iter().any(|x| *x) {
                            o.struggle = true;
                            o.hidden_moves = [false; 4];
                        } else {
                            o.mega = m.can_mega.is_some();
                        }
                    }
                }
                if !m.trapped {
                    o.switches = self.bench(side);
                } else if m.trap_hidden && sd.revealed & 1 == 0 && self.locked_move(side).is_none() {
                    o.hidden_switches = self.bench(side);
                }
                o
            }
        }
    }

    /// pokemon.getLockedMove: the first volatile with a LockMove handler (Some(NONE) for Recharge)
    pub fn locked_move(&self, side: usize) -> Option<MoveId> {
        let sd = &self.sides[side];
        if !sd.has_active {
            return None;
        }
        for v in sd.volatiles() {
            if super::events::has_handler(v.id, super::events::Pre::Own, super::events::Ev::LockMove) {
                return Some(match v.id {
                    c if c == conds::MUSTRECHARGE => MoveId::NONE,
                    c if c == conds::UPROAR => moves::UPROAR,
                    _ => v.mv,
                });
            }
        }
        None
    }

    /// The DisableMove pass of a turn start, without drawing: the slots its handlers disable.
    fn disable_pass(&self, side: usize) -> [bool; 4] {
        let sd = &self.sides[side];
        let m = sd.active_mon();
        let n = m.nmoves as usize;
        let mut out = [false; 4];
        for v in sd.volatiles() {
            for i in 0..n {
                let id = m.moves[i].id;
                let d = id.data();
                let hit = match v.id {
                    c if c == conds::DISABLE => id == v.mv,
                    c if c == conds::ENCORE => !v.mv.is_none() && m.moves[..n].iter().any(|x| x.id == v.mv) && id != v.mv,
                    c if c == conds::TAUNT => d.category == Category::Status,
                    c if c == conds::THROATCHOP => d.has_flag(flag::SOUND),
                    c if c == conds::HEALBLOCK => d.has_flag(flag::HEAL),
                    c if c == conds::TORMENT => !m.last_move.is_none() && m.last_move != moves::STRUGGLE && id == m.last_move,
                    _ => false,
                };
                out[i] |= hit;
            }
        }
        // choicelock: the other moves, while the Choice item acts
        if let Some(v) = sd.vol(conds::CHOICELOCK) {
            let magic_room = self.field.pseudo[..self.field.npseudo as usize].iter().any(|c| c.id == conds::MAGICROOM);
            let klutz = m.ability == abilities::KLUTZ && !sd.has_vol(conds::GASTROACID) && !self.neutralizing_gas(side);
            if m.item.data().is_choice && m.moves[..n].iter().any(|x| x.id == v.mv) && !magic_room && !klutz {
                for i in 0..n {
                    if m.moves[i].id != v.mv {
                        out[i] = true;
                    }
                }
            }
        }
        // Gorilla Tactics' choice lock
        if m.ability == abilities::GORILLATACTICS && m.ab_data != 0 && !sd.has_vol(conds::GASTROACID) {
            for i in 0..n {
                if m.moves[i].id.0 as i16 != m.ab_data {
                    out[i] = true;
                }
            }
        }
        // Imprison on the foe: the moves it knows too
        let fs = &self.sides[1 - side];
        if fs.has_active && fs.has_vol(conds::IMPRISON) {
            let foe = fs.active_mon();
            for i in 0..n {
                let id = m.moves[i].id;
                if id != moves::STRUGGLE && foe.moves().iter().any(|x| x.id == id) {
                    out[i] = true;
                }
            }
        }
        let gravity = self.field.pseudo[..self.field.npseudo as usize].iter().any(|c| c.id == conds::GRAVITY);
        for i in 0..n {
            let id = m.moves[i].id;
            if gravity && id.data().has_flag(flag::GRAVITY) {
                out[i] = true;
            }
            if (id == moves::FAKEOUT || id == moves::FIRSTIMPRESSION) && m.active_move_actions > 0 {
                out[i] = true;
            }
            if id.data().has_flag(flag::CANTUSETWICE) && m.last_move == id {
                out[i] = true;
            }
        }
        out
    }

    /// An active Neutralizing Gas suppresses the ability of `side`'s active Pokémon (no hasAbility, no Sim here)
    fn neutralizing_gas(&self, side: usize) -> bool {
        let me = self.sides[side].active_mon();
        if me.ability.data().flags & aflag::CANTSUPPRESS != 0 || me.ability == abilities::NEUTRALIZINGGAS {
            return false;
        }
        self.sides.iter().any(|sd| {
            sd.has_active && {
                let x = sd.active_mon();
                x.is_active && x.ability == abilities::NEUTRALIZINGGAS && !sd.has_vol(conds::GASTROACID) && !x.transformed
                    && x.ab_flags & super::abilities::abf::ENDING == 0
            }
        })
    }

    /// The move-slot flags the rig snapshot reports: true when the move cannot be chosen now. A side with
    /// no move decision shows the flags its next turn starts with.
    pub fn disabled_flags(&self, side: usize, slot: usize) -> [bool; 4] {
        let sd = &self.sides[side];
        let m = &sd.mons[slot];
        let mut out = [false; 4];
        if !m.is_active || m.fainted {
            return out;
        }
        let n = m.nmoves as usize;
        if let Some(mv) = self.locked_move(side) {
            for i in 0..n {
                out[i] = mv.is_none() || m.moves[i].id != mv;
            }
            return out;
        }
        let dis = if sd.request == Request::Move && !self.ended {
            [m.moves[0].disabled, m.moves[1].disabled, m.moves[2].disabled, m.moves[3].disabled]
        } else {
            self.disable_pass(side)
        };
        let usable: Vec<bool> = (0..n).map(|i| !dis[i] && m.moves[i].pp > 0).collect();
        let any = usable.iter().any(|x| *x);
        for i in 0..n {
            out[i] = !any || !usable[i];
        }
        out
    }

    /// A choice the game offers and refuses: the side learns why (the request is updated) and both sides
    /// choose again. Returns the refused sides and reasons, or None when every choice is accepted.
    pub fn refuse(&mut self, choices: [Option<Choice>; SIDES]) -> Option<[Option<&'static str>; SIDES]> {
        let opts = self.options();
        let mut out = [None; SIDES];
        for side in 0..SIDES {
            if let Some(c) = choices[side] {
                out[side] = opts[side].refusal(c);
            }
        }
        if out.iter().all(|x| x.is_none()) {
            return None;
        }
        for side in 0..SIDES {
            match (out[side], choices[side]) {
                (Some("trapped"), _) => self.sides[side].revealed |= 1,
                // only the chosen move is shown disabled
                (Some(_), Some(Choice::Move { index, .. })) => self.sides[side].revealed |= 2 << index,
                _ => {}
            }
        }
        Some(out)
    }

    /// Plays one decision step: both sides' choices (None for a side that has no decision).
    pub fn step<O: Outcomes>(&mut self, choices: [Option<Choice>; SIDES], o: &mut O) -> Result<(), String> {
        self.step_with(choices, o, None)
    }

    /// `step`, recording the battle log's rows into `log` (appended).
    pub fn step_logged<O: Outcomes>(&mut self, choices: [Option<Choice>; SIDES], o: &mut O, log: &mut Vec<Row>) -> Result<(), String> {
        self.step_with(choices, o, Some(log))
    }

    fn step_with<O: Outcomes>(&mut self, choices: [Option<Choice>; SIDES], o: &mut O, log: Option<&mut Vec<Row>>) -> Result<(), String> {
        if self.ended {
            return Err("battle has ended".into());
        }
        let opts = self.options();
        let mut cs = [None; SIDES];
        for side in 0..SIDES {
            let op = &opts[side];
            match (op.kind, choices[side]) {
                (Kind::Turn | Kind::Replace, None) => return Err(format!("p{} must choose", side + 1)),
                (Kind::Wait | Kind::Done, Some(_)) => return Err(format!("p{} has no decision", side + 1)),
                (_, None) => {}
                (_, Some(c)) => {
                    let c = match c {
                        // a move position when only Struggle is left means Struggle
                        Choice::Move { index: 0, .. } if op.struggle => Choice::Move { index: STRUGGLE, mega: false },
                        c => c,
                    };
                    if !op.allows(c) {
                        return Err(format!("p{}: {:?} not allowed", side + 1, c));
                    }
                    cs[side] = Some(c);
                }
            }
        }
        Sim::with_log(self, o, log).commit(cs);
        Ok(())
    }

    /// A battle at the first decision of turn 1: teams of six, picks (lead first), team preview resolved,
    /// leads in.
    pub fn start<O: Outcomes>(teams: [&[MonSpec]; SIDES], picks: [[u8; 3]; SIDES], o: &mut O) -> Result<State, String> {
        State::start_with(teams, picks, o, None)
    }

    /// `start`, recording the battle log's rows of the leads' entry into `log` (appended).
    pub fn start_logged<O: Outcomes>(teams: [&[MonSpec]; SIDES], picks: [[u8; 3]; SIDES], o: &mut O, log: &mut Vec<Row>) -> Result<State, String> {
        State::start_with(teams, picks, o, Some(log))
    }

    fn start_with<O: Outcomes>(teams: [&[MonSpec]; SIDES], picks: [[u8; 3]; SIDES], o: &mut O, log: Option<&mut Vec<Row>>) -> Result<State, String> {
        let mut s = State::default();
        for side in 0..SIDES {
            let p = picks[side];
            if p.iter().any(|&i| i as usize >= teams[side].len()) || p[0] == p[1] || p[0] == p[2] || p[1] == p[2] {
                return Err(format!("bad picks for p{}", side + 1));
            }
            for (slot, &i) in p.iter().enumerate() {
                s.sides[side].mons[slot] = build_mon(&teams[side][i as usize]);
            }
            s.sides[side].pokemon_left = TEAM as u8;
            s.sides[side].order = [0, 1, 2];
        }
        s.active_pokemon = NO_MON;
        let mk = super::hooks::masks();
        s.no_holder_fx = s.sides.iter().flat_map(|sd| sd.mons.iter()).all(|m| {
            mk.abilities_unimpl[m.ability.0 as usize] == 0 && mk.items_unimpl[m.item.0 as usize] == 0
        });
        // the abilities this battle can have: the brought Pokémon's, their Mega forms', and the ones Simple
        // Beam and Worry Seed give (set_ability adds any other)
        let mut union = 0u128;
        for side in 0..SIDES {
            for slot in 0..TEAM {
                let m = &s.sides[side].mons[slot];
                union |= super::events::ability_evmask(m.ability);
                if let Some(mega) = m.can_mega {
                    union |= super::events::ability_evmask(mega.data().abilities[0]);
                }
                for ms in m.moves() {
                    if ms.id == moves::SIMPLEBEAM {
                        union |= super::events::ability_evmask(abilities::SIMPLE);
                    } else if ms.id == moves::WORRYSEED {
                        union |= super::events::ability_evmask(abilities::INSOMNIA);
                    }
                }
            }
        }
        s.ab_union = union;
        // the items this battle can have: only the brought Pokémon's ever move between them
        let mut iu = 0u128;
        for sd in &s.sides {
            for m in &sd.mons {
                iu |= super::events::item_evmask(m.item);
            }
        }
        s.it_union = iu;
        // an ability or item without an implementation makes the battle inexact from the start
        for side in 0..SIDES {
            for slot in 0..TEAM {
                let a = s.sides[side].mons[slot].ability;
                if !super::abilities::ability_implemented(a) {
                    s.unimpl.note(super::outcome::EffectRef::Ability(a), super::hooks::Hook::Start);
                }
                let i = s.sides[side].mons[slot].item;
                if !i.is_none() && !super::items::item_implemented(i.data()) {
                    s.unimpl.note(super::outcome::EffectRef::Item(i), super::hooks::Hook::Start);
                }
            }
        }
        let mut sim = Sim::with_log(&mut s, o, log);
        // the team actions of team preview: order 1, priority -index, speed; ties are drawn
        for side in 0..SIDES {
            for slot in 0..TEAM {
                let speed = sim.action_speed(side, slot);
                sim.s.queue.push(Action {
                    kind: ActionKind::Start,
                    order: order::TEAM,
                    priority: -(slot as i16) * 10,
                    speed,
                    side: side as u8,
                    slot: slot as u8,
                    ..Default::default()
                });
            }
        }
        sim.sort_queue();
        sim.s.queue.clear();
        sim.s.queue.push(Action { kind: ActionKind::Start, order: order::START, side: super::battle::NO_SIDE, speed: 1, ..Default::default() });
        sim.s.mid_turn = true;
        sim.turn_loop();
        Ok(s)
    }
}

/// new Pokemon(set) with the Champions stat formula
pub fn build_mon(spec: &MonSpec) -> Mon {
    let d = spec.species.data();
    let mut m = Mon {
        species: spec.species,
        base_species: spec.species,
        set_species: spec.species,
        ability: spec.ability,
        base_ability: spec.ability,
        item: spec.item,
        evs: spec.evs,
        nature: spec.nature,
        level: LEVEL,
        gender: spec.gender,
        nmoves: spec.nmoves,
        // the constructor ends with clearVolatile
        newly_switched: true,
        ..Default::default()
    };
    for i in 0..spec.nmoves as usize {
        let mv = spec.moves[i];
        let pp = mv.data().max_pp;
        m.base_moves[i] = MoveSlot { id: mv, pp, max_pp: pp, disabled: false, disabled_hidden: false, used: false };
    }
    m.moves = m.base_moves;
    let nt = if d.types[1] == Type::Typeless { 1 } else { 2 };
    m.set_types(&d.types[..nt]);
    m.weight_hg = d.weight_hg;
    let hp = if d.max_hp > 0 { d.max_hp } else { stat_from_points(d.base_stats[0], spec.evs[0], Stat::Hp, spec.nature) };
    m.hp = hp;
    m.maxhp = hp;
    m.base_maxhp = hp;
    let stats = [Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe];
    for (i, st) in stats.iter().enumerate() {
        m.stats[i] = stat_from_points(d.base_stats[*st as usize], spec.evs[*st as usize], *st, spec.nature);
    }
    m.speed = m.stats[4] as i32;
    // canMegaEvo (Champions): the held stone's form for this species
    m.can_mega = spec.item.data().mega_stone.iter().find(|(from, _)| *from == spec.species).map(|(_, to)| *to);
    m
}
