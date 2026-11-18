//! The battle log: one `Row` per row the game's battle log writes, recorded by the resolver where the thing
//! happens, in the order it happens (Showdown's order of log lines). Rows carry the truth with engine ids and
//! sides (`Who` is side and slot); what one player may see of them is the emitter's business.
//!
//! Recording is off unless a row buffer is passed (`State::step_logged`, `State::start_logged`); with it off
//! the resolver only tests for the buffer. The buffer is kept outside the State.
//!
//! Where the phone's log differs from Showdown's order or content, the rows keep what an emitter needs:
//! - an HP row carries its cause (`cause`, `of`); the phone writes the cause as its own row before the HP row;
//! - an effectiveness row (`Result`) comes before the HP row of its hit, as in Showdown; the phone writes it
//!   after that HP row;
//! - no row for critical hits, damage rolls, PP spent by using a move, or a weather continuing;
//! - `Withdraw` has no Showdown line (the phone writes "come back" / "went back"); none for Baton Pass,
//!   Shed Tail, Emergency Exit, Wimp Out, Eject Button, Eject Pack, a drag or a fainted Pokémon;
//! - a `Boost` row holds every stat one boost call changed with nothing in between, as the phone writes one
//!   row for a multi-stat change; a stat already at its limit is a `BoostLimit` row.

use super::battle::Sim;
use super::data::*;
use super::events::{Eff, Pk};
use super::outcome::Outcomes;
use super::state::Winner;

/// A Pokémon: side and slot (the order brought, which never changes).
pub type Who = (u8, u8);

#[inline]
pub fn who(p: Pk) -> Who {
    (p.0 as u8, p.1 as u8)
}

/// What caused or names something.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Fx {
    #[default]
    None,
    Move(MoveId),
    Ability(AbilityId),
    Item(ItemId),
    /// a volatile, side, slot or field condition, weather or terrain (flinch, recharge, Leech Seed, ...)
    Cond(Cond),
    Status(Status),
    /// recoil of a move (Struggle's included)
    Recoil(MoveId),
    /// HP drained by a move
    Drain(MoveId),
    /// a move's crash damage (High Jump Kick)
    Crash(MoveId),
    /// HP a move costs its user (Mind Blown, Steel Beam, Belly Drum, Substitute, Curse, Fillet Away, ...)
    Cost(MoveId),
    /// the confusion self-hit
    Confusion,
    /// a heal that failed: the Pokémon's HP is full
    HpFull,
}

/// An effectiveness row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    Super,
    /// x4 ("extremely effective")
    Extreme,
    Resisted,
    /// x0.25 ("mostly ineffective")
    Mostly,
    Immune,
    Ohko,
}

/// A volatile's detail as the row shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Detail {
    #[default]
    None,
    Move(MoveId),
    Num(i8),
    Type(Type),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemWhat {
    /// an item acted ("Held Item:" row); whether it is spent is the item's nature
    Activated,
    /// an item used up (Focus Sash, a Berry eaten, a Gem, White Herb)
    Used,
    /// taken away or destroyed (Knock Off, Air Balloon popped, Incinerate, Corrosive Gas)
    Removed,
    /// given or taken (Trick, Thief, Bestow, Harvest, Recycle, Pickup)
    Received,
    /// revealed by another Pokémon (Frisk) or shown on entry (Air Balloon)
    Found,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideWhat {
    Start,
    End,
    /// the condition acted on a Pokémon (Sticky Web catching it)
    Acted,
}

/// What a stat reset did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reset {
    /// every Pokémon's stages (Haze)
    Field,
    /// one Pokémon's stages (Clear Smog)
    All,
    Negative,
    Positive,
    /// copied from `of` (Psych Up)
    Copy,
    /// swapped with `of` (Heart Swap, Power Swap, Guard Swap, Speed Swap)
    Swap,
    /// inverted (Topsy-Turvy)
    Invert,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// a new turn begins
    Turn(u16),
    /// sent out; `forced` when dragged in (Roar, Dragon Tail, Red Card)
    Enter { p: Who, forced: bool },
    /// called back; `pivot` is the move whose switch it is (U-turn, Volt Switch, Parting Shot, ...)
    Withdraw { p: Who, pivot: Option<MoveId> },
    /// a move used; `called_by` the move that called it (Sleep Talk, Copycat, Metronome, ...) or the
    /// ability (Magic Bounce, Dancer); `priority` the move's priority as used
    Move { p: Who, mv: MoveId, target: Option<Who>, called_by: Fx, priority: i8 },
    /// the charging turn of a two-turn move
    Prepare { p: Who, mv: MoveId },
    /// could not move: a status (slp, par, frz), a condition (flinch, recharge, Taunt, Disable, Imprison,
    /// Heal Block, Throat Chop, Gravity, choice lock refused...), an ability (Truant) or no PP; `mv` the move
    /// refused where the row names it
    Cant { p: Who, why: Fx, mv: MoveId },
    /// a protection blocked a move ("X protected itself!")
    Protect { p: Who, by: Cond },
    /// `p` avoided `by`'s move
    Miss { p: Who, by: Who },
    /// "But it failed!" (`p` the user, `why` None), or a failure the row explains on `p` ("already
    /// poisoned", "HP is full", "already has a substitute": `why` the status, move or condition)
    Fail { p: Option<Who>, why: Fx },
    /// an effectiveness row; `p` the Pokémon hit (the immune one for `Immune`)
    Result { how: How, p: Option<Who> },
    /// "The Pokémon was hit N times!"
    Hits { n: u8, p: Who },
    /// an HP change in exact HP; `cause` what changed it, `of` the other Pokémon involved (the attacker,
    /// the drained one, the holder of the ability or item that hurt it)
    Hp { p: Who, before: u16, after: u16, cause: Fx, of: Option<Who> },
    Faint { p: Who },
    /// a status began; `rest` for Rest's "slept and restored its HP"
    Status { p: Who, st: Status, by: Fx, of: Option<Who>, rest: bool },
    /// a status ended; `by` the item or ability that cured it, or None
    Cure { p: Who, st: Status, by: Fx },
    /// a condition began or its shown count changed
    Volatile { p: Who, c: Cond, detail: Detail, by: Fx, of: Option<Who> },
    VolatileEnd { p: Who, c: Cond, by: Fx },
    /// a condition acted (the Substitute took the hit, endured the hit, Destiny Bond took its attacker down,
    /// confusion's self-hit, "is confused!")
    VolatileActed { p: Who, c: Cond, detail: Detail, of: Option<Who> },
    /// stat stages changed: the signed change of each stat (atk, def, spa, spd, spe, accuracy, evasion)
    Boost { p: Who, by_stat: [i8; 7], by: Fx },
    /// stats that would not go higher (`up`) or lower
    BoostLimit { p: Who, stats: [bool; 7], up: bool, by: Fx },
    /// stages reset (Haze: `p` None)
    BoostReset { p: Option<Who>, what: Reset, of: Option<Who>, by: Fx },
    /// something was prevented: `what` (a stat drop, a status, a move, Berries) by `by` (an ability, item,
    /// terrain, condition) on `p`
    Blocked { p: Option<Who>, by: Fx, what: Fx },
    /// an ability announced ("Ability:" row, or a sentence naming it)
    Ability { p: Who, ab: AbilityId, of: Option<Who> },
    /// an ability changed ("Ability Changed: X: OLD -> NEW")
    AbilityChange { p: Who, from: AbilityId, to: AbilityId, by: Fx },
    /// an item row; `of` the other Pokémon (the Knock Off user, the frisker, the Red Card's target)
    Item { p: Who, item: ItemId, what: ItemWhat, of: Option<Who>, by: Fx },
    /// "X's STONE is reacting to TRAINER's Omni Ring!"
    MegaStone { p: Who, stone: ItemId },
    Mega { p: Who, form: SpeciesId },
    /// any other form change (Disguise busted, Zero to Hero, Stance Change, ...)
    Form { p: Who, form: SpeciesId, by: Fx },
    Transform { p: Who, into: Who },
    /// types changed to `types` (Typeless entries are empty); `lost` for a type used up (Burn Up, Double
    /// Shock, Roost's turn), `added` for a type added (Forest's Curse, Trick-or-Treat)
    Types { p: Who, types: [Type; 2], lost: Type, added: Type, by: Fx },
    /// weather began (`w` the weather) or ended (`w` NONE)
    Weather { w: Cond, by: Fx, of: Option<Who> },
    /// a terrain, room or Gravity began or ended
    Field { c: Cond, start: bool, by: Fx, of: Option<Who> },
    /// a side condition of `side` began, ended or acted on `p`
    Side { side: u8, c: Cond, what: SideWhat, p: Option<Who>, by: Fx },
    /// a one-off announcement (Perish Song's song, no moves left, Chilly Reception's joke, Destiny Bond's
    /// hope, Endure's bracing, a charge of power, Focus Punch's focus, Beak Blast's heat, ...)
    Once { p: Option<Who>, fx: Fx, of: Option<Who> },
    /// PP lost to Spite, Eerie Spell or Grudge
    Pp { p: Who, mv: MoveId, lost: u8, by: Fx },
    /// revived (Revival Blessing)
    Revive { p: Who },
    /// the battle ended
    End { winner: Winner },
}

impl Row {
    /// The row's kind as a short name.
    pub fn kind(&self) -> &'static str {
        match self {
            Row::Turn(_) => "turn",
            Row::Enter { .. } => "enter",
            Row::Withdraw { .. } => "withdraw",
            Row::Move { .. } => "move",
            Row::Prepare { .. } => "prepare",
            Row::Cant { .. } => "cant",
            Row::Protect { .. } => "protect",
            Row::Miss { .. } => "miss",
            Row::Fail { .. } => "fail",
            Row::Result { .. } => "result",
            Row::Hits { .. } => "hits",
            Row::Hp { .. } => "hp",
            Row::Faint { .. } => "faint",
            Row::Status { .. } => "status",
            Row::Cure { .. } => "cure",
            Row::Volatile { .. } => "volatile",
            Row::VolatileEnd { .. } => "volatile_end",
            Row::VolatileActed { .. } => "volatile_acted",
            Row::Boost { .. } => "boost",
            Row::BoostLimit { .. } => "boost_limit",
            Row::BoostReset { .. } => "boost_reset",
            Row::Blocked { .. } => "blocked",
            Row::Ability { .. } => "ability",
            Row::AbilityChange { .. } => "ability_change",
            Row::Item { .. } => "item",
            Row::MegaStone { .. } => "mega_stone",
            Row::Mega { .. } => "mega",
            Row::Form { .. } => "form",
            Row::Transform { .. } => "transform",
            Row::Types { .. } => "types",
            Row::Weather { .. } => "weather",
            Row::Field { .. } => "field",
            Row::Side { .. } => "side",
            Row::Once { .. } => "once",
            Row::Pp { .. } => "pp",
            Row::Revive { .. } => "revive",
            Row::End { .. } => "end",
        }
    }
}

/// The status a status condition is.
pub fn status_of(c: Cond) -> Option<Status> {
    Some(match c {
        c if c == conds::BRN => Status::Brn,
        c if c == conds::PAR => Status::Par,
        c if c == conds::SLP => Status::Slp,
        c if c == conds::FRZ => Status::Frz,
        c if c == conds::PSN => Status::Psn,
        c if c == conds::TOX => Status::Tox,
        _ => return None,
    })
}

impl<'a, O: Outcomes> Sim<'a, O> {
    /// Rows are being recorded.
    #[inline(always)]
    pub fn logging(&self) -> bool {
        self.log.is_some()
    }

    /// Records a row when recording.
    #[inline(always)]
    pub fn row(&mut self, r: Row) {
        if let Some(l) = self.log.as_deref_mut() {
            l.push(r);
        }
    }

    /// An HP row, when the HP changed.
    #[inline]
    pub fn row_hp(&mut self, p: Pk, before: u16, cause: Fx, of: Option<Pk>) {
        if let Some(l) = self.log.as_deref_mut() {
            let after = self.s.sides[p.0].mons[p.1].hp;
            if after != before {
                l.push(Row::Hp { p: who(p), before, after, cause, of: of.map(who) });
            }
        }
    }

    /// One stat of a boost call (`delta` 0: at its limit, `up` the way it was asked to go). Joins the row
    /// before when that row is this call's (made after `group`), is the last row and has the same change.
    pub fn row_boost(&mut self, p: Pk, stat: usize, delta: i8, up: bool, by: Fx, group: usize) {
        let Some(l) = self.log.as_deref_mut() else { return };
        let w = who(p);
        if delta == 0 {
            let mut i = l.len();
            while i > group {
                i -= 1;
                match &mut l[i] {
                    Row::BoostLimit { p: q, stats, up: u, .. } if *q == w => {
                        if *u == up {
                            stats[stat] = true;
                            return;
                        }
                    }
                    Row::Boost { p: q, .. } if *q == w => {}
                    _ => break,
                }
            }
            let mut stats = [false; 7];
            stats[stat] = true;
            l.push(Row::BoostLimit { p: w, stats, up, by });
            return;
        }
        // any row of this call's unbroken run of stage rows on this Pokémon with the same change
        let mut i = l.len();
        while i > group {
            i -= 1;
            match &mut l[i] {
                Row::Boost { p: q, by_stat, .. } if *q == w => {
                    if by_stat[stat] == 0 && by_stat.iter().all(|&x| x == 0 || x == delta) {
                        by_stat[stat] = delta;
                        return;
                    }
                }
                Row::BoostLimit { p: q, .. } if *q == w => {}
                _ => break,
            }
        }
        let mut by_stat = [0i8; 7];
        by_stat[stat] = delta;
        l.push(Row::Boost { p: w, by_stat, by });
    }

    /// The cause of an HP change or a status as a row names it, from the event's effect.
    pub fn fx_of(&self, eff: Eff) -> Fx {
        let mv = self.s.active_move;
        match eff {
            Eff::None => Fx::None,
            Eff::Move(m) => Fx::Move(m),
            Eff::Cond(c) if c.is_none() => Fx::Cost(mv),
            Eff::Cond(c) => match status_of(c) {
                Some(st) => Fx::Status(st),
                None => Fx::Cond(c),
            },
            Eff::Ability(a) => Fx::Ability(a),
            Eff::Item(i) => Fx::Item(i),
            Eff::Recoil => Fx::Recoil(mv),
            Eff::StruggleRecoil => Fx::Recoil(moves::STRUGGLE),
            Eff::Drain => Fx::Drain(mv),
            Eff::Confused => Fx::Confusion,
        }
    }

    /// The HP row of a damage call: the cause from the effect (or `hp_cause` when a caller set one).
    pub fn damage_row(&mut self, target: Pk, before: u16, source: Option<Pk>, eff: Eff) {
        let mut cause = match self.hp_cause.take() {
            Some(c) => c,
            None => self.fx_of(eff),
        };
        if let (Fx::Move(m), Some(sp)) = (cause, source) {
            if sp == target {
                cause = Fx::Cost(m);
            }
        }
        let of = source.filter(|&sp| sp != target);
        // spreadDamage writes its row also for no damage (Disguise, Ice Face)
        if let Some(l) = self.log.as_deref_mut() {
            let after = self.s.sides[target.0].mons[target.1].hp;
            l.push(Row::Hp { p: who(target), before, after, cause, of: of.map(who) });
        }
    }

    /// The rows of one stat of a boost call (battle.boost's messages).
    #[allow(clippy::too_many_arguments)]
    pub fn boost_row(&mut self, target: Pk, stat: usize, delta: i8, up: bool, eff: Eff, secondary: bool, is_self: bool, group: usize, shown: &mut bool) {
        let by = self.fx_of(eff);
        if delta != 0 {
            // an ability's boost is announced once per call (not for a secondary effect)
            if let Eff::Ability(a) = eff {
                if !secondary && !*shown {
                    self.row(Row::Ability { p: who(target), ab: a, of: None });
                    *shown = true;
                }
            }
            self.row_boost(target, stat, delta, up, by, group);
        } else {
            let show = if matches!(eff, Eff::Ability(_)) { secondary || is_self } else { !secondary && !is_self };
            if show {
                self.row_boost(target, stat, 0, up, by, group);
            }
        }
    }

    /// The row a volatile's Start handler writes, before the handler runs; its index.
    pub fn vol_start_row(&mut self, p: Pk, c: Cond, src: Pk, eff: Eff) -> Option<usize> {
        let show = vol_start_show(c);
        // the crit volatiles copied by Psych Up and Transform start silently
        let copied = (c == conds::FOCUSENERGY || c == conds::DRAGONCHEER) && (eff == Eff::Move(moves::PSYCHUP) || eff == Eff::Move(moves::TRANSFORM));
        if show == VolShow::Silent || copied {
            return None;
        }
        // Charge names no source
        let of = if src != p && c != conds::CHARGE { Some(who(src)) } else { None };
        // a row names an ability or item as its cause only for confusion, Disable, Attract and Charge
        let named = c == conds::CONFUSION || c == conds::DISABLE || c == conds::ATTRACT || c == conds::CHARGE;
        let by = match eff {
            Eff::Ability(_) | Eff::Item(_) if named => self.fx_of(eff),
            Eff::Move(_) => self.fx_of(eff),
            Eff::Cond(x) if x == conds::LOCKEDMOVE => Fx::Cond(x),
            _ => Fx::None,
        };
        if c == conds::PROTOSYNTHESIS || c == conds::QUARKDRIVE {
            let ab = self.mon_at(p).ability;
            self.row(Row::Ability { p: who(p), ab, of: None });
        }
        let i = self.row_mark();
        match show {
            VolShow::Start => self.row(Row::Volatile { p: who(p), c, detail: Detail::None, by, of }),
            _ => self.row(Row::Once { p: Some(who(p)), fx: Fx::Cond(c), of }),
        }
        Some(i)
    }

    /// Fills in a volatile's start row from its state after the Start handler.
    pub fn vol_start_detail(&mut self, p: Pk, c: Cond, i: usize) {
        let Some(v) = self.s.sides[p.0].vol(c).copied() else { return };
        let d = match c {
            c if c == conds::DISABLE || c == conds::ENCORE || c == conds::PARTIALLYTRAPPED => Detail::Move(v.mv),
            c if c == conds::STOCKPILE || c == conds::PROTOSYNTHESIS || c == conds::QUARKDRIVE => Detail::Num(v.data),
            _ => return,
        };
        if let Some(Row::Volatile { detail, .. }) = self.log.as_deref_mut().and_then(|l| l.get_mut(i)) {
            *detail = d;
        }
    }

    /// pokemon.runImmunity's rows for a move: Levitate (Eelevate) named when it is why a Ground move misses.
    pub fn immune_rows(&mut self, t: Pk, am: &super::moves::ActiveMove) {
        if am.ty == Type::Ground {
            let ab = self.mon_at(t).ability;
            let types = self.get_types(t, false);
            let flying = types.has(Type::Flying) && !(types.has(Type::Typeless) && self.has_vol(t, conds::ROOST));
            if (ab == abilities::LEVITATE || ab == abilities::EELEVATE) && !flying && !self.ignoring_ability(t) && !self.suppressing_ability(t) {
                self.row(Row::Ability { p: who(t), ab, of: None });
            }
        }
        self.row(Row::Result { how: How::Immune, p: Some(who(t)) });
    }

    /// What a weather or terrain row names as its setter: an ability (or item) and its holder.
    pub fn field_by(&self, eff: Eff, source: Pk) -> (Fx, Option<Who>) {
        match eff {
            Eff::Ability(_) | Eff::Item(_) => (self.fx_of(eff), Some(who(source))),
            _ => (Fx::None, None),
        }
    }

    /// An ability announced ("Ability:" row) for its holder.
    #[inline]
    pub fn ab_row(&mut self, p: Pk, ab: AbilityId) {
        self.row(Row::Ability { p: who(p), ab, of: None });
    }

    /// An ability made its holder immune: the Ability row and "It doesn't affect X".
    pub fn ab_immune(&mut self, t: Pk, ab: AbilityId) {
        if self.logging() {
            self.row(Row::Ability { p: who(t), ab, of: None });
            self.row(Row::Result { how: How::Immune, p: Some(who(t)) });
        }
    }

    /// A type change row with the Pokémon's types now.
    pub fn types_row(&mut self, p: Pk, by: Fx) {
        if self.logging() {
            let m = self.mon_at(p);
            let mut types = [Type::Typeless; 2];
            types[..m.ntypes as usize].copy_from_slice(&m.types[..m.ntypes as usize]);
            self.row(Row::Types { p: who(p), types, lost: Type::Typeless, added: Type::Typeless, by });
        }
    }

    /// Drops the rows recorded after `mark` (an announcement whose effect did not happen).
    pub fn row_truncate(&mut self, mark: usize) {
        if let Some(l) = self.log.as_deref_mut() {
            l.truncate(mark);
        }
    }

    /// pokemon.effectiveWeather(.., message): Mega Sol announced when it makes a move see sun that is not there
    pub fn megasol_row(&mut self, p: Pk) {
        if !self.logging() {
            return;
        }
        if let Some(ap) = super::events::pk_of(self.s.active_pokemon) {
            if self.s.active_move != MoveId::NONE && self.has_ability(ap, abilities::MEGASOL) && self.weather() != conds::SUNNYDAY {
                self.row(Row::Ability { p: who(p), ab: abilities::MEGASOL, of: None });
            }
        }
    }

    /// The number of rows recorded so far (a boost call's group mark).
    #[inline]
    pub fn row_mark(&self) -> usize {
        self.log.as_deref().map_or(0, |l| l.len())
    }
}

/// How a volatile's start shows in the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolShow {
    Silent,
    /// a lasting condition row ("-start")
    Start,
    /// a one-off announcement ("-singleturn", "-singlemove")
    Once,
}

/// The row a volatile's Start handler writes.
pub fn vol_start_show(c: Cond) -> VolShow {
    use conds::*;
    const START: [Cond; 29] = [
        AQUARING, ATTRACT, CHARGE, CONFUSION, CURSE, DISABLE, DRAGONCHEER, ENCORE, FLASHFIRE, FOCUSENERGY, GASTROACID,
        HEALBLOCK, IMPRISON, INGRAIN, LEECHSEED, MAGNETRISE, MUSTRECHARGE, OCTOLOCK, PARTIALLYTRAPPED,
        POWERSHIFT, POWERTRICK, SALTCURE, SMACKDOWN, STOCKPILE, SUBSTITUTE, SYRUPBOMB, TAUNT, TORMENT, UPROAR,
    ];
    const MORE: [Cond; 4] = [YAWN, TRAPPED, PROTOSYNTHESIS, QUARKDRIVE];
    const ONCE: [Cond; 13] = [
        BANEFULBUNKER, BEAKBLAST, DESTINYBOND, ELECTRIFY, ENDURE, FOCUSPUNCH, FOLLOWME, HELPINGHAND, KINGSSHIELD,
        PROTECT, RAGEPOWDER, ROOST, SPIKYSHIELD,
    ];
    if START.contains(&c) || MORE.contains(&c) {
        VolShow::Start
    } else if ONCE.contains(&c) {
        VolShow::Once
    } else {
        VolShow::Silent
    }
}

/// The volatile's End handler writes a row ("-end" without [silent]).
pub fn vol_end_shown(c: Cond) -> bool {
    use conds::*;
    const END: [Cond; 16] = [
        CONFUSION, DISABLE, ENCORE, HEALBLOCK, MAGNETRISE, PARTIALLYTRAPPED, POWERSHIFT, POWERTRICK, PROTOSYNTHESIS,
        QUARKDRIVE, SALTCURE, STOCKPILE, SUBSTITUTE, TAUNT, TORMENT, UPROAR,
    ];
    END.contains(&c)
}
