//! Moves' own handlers (the functions on each move in data/moves.ts and data/mods/champions/moves.ts):
//! onTry, onPrepareHit, onTryMove, onTryHit, onTryImmunity, onModifyType, onModifyMove, onModifyTarget,
//! onHit and its secondary and self variants, onHitField, basePowerCallback, damageCallback, onBasePower,
//! onEffectiveness, onAfterMove, onMoveFail, onAfterMoveSecondarySelf, onDisableMove and the turn-start
//! callbacks. Messages are left out.

use super::battle::Sim;
use super::data::*;
use super::events::*;
use super::moves::*;
use super::outcome::*;
use super::state::*;
use super::log::*;

/// The moves whose own handlers are implemented here (hooks.rs treats the rest as unimplemented).
pub const IMPLEMENTED_MOVES: &[&str] = &[
    "acupressure", "afteryou", "alluringvoice", "allyswitch", "aquaring", "assurance", "attract", "aurawheel", "avalanche", "axekick",
    "banefulbunker", "barbbarrage", "batonpass", "beakblast", "beatup", "bellydrum", "block", "bounce",
    "burningjealousy", "burnup", "charge", "clangoroussoul", "clearsmog", "comeuppance", "copycat", "counter",
    "curse", "destinybond", "detect", "dig", "direclaw", "disable", "dive", "doubleshock", "dragoncheer",
    "eeriespell", "electrify", "electroball", "encore", "endeavor", "endure", "eruption", "facade", "fakeout",
    "fellstinger", "ficklebeam", "finalgambit", "firstimpression", "flail", "fly", "flyingpress", "focusenergy",
    "focuspunch", "followme", "forestscurse", "freezedry", "glaiverush", "grassknot", "guardsplit", "guardswap",
    "gyroball", "hardpress", "haze", "healbell", "healpulse", "heatcrash", "heavyslam", "helpinghand", "hex",
    "highjumpkick", "infernalparade", "ingrain", "instruct", "jawlock", "kingsshield", "lashout", "lastresort",
    "lastrespects", "leechseed", "lockon", "lowkick", "magicpowder", "magnetrise", "meanlook", "metalburst",
    "meteorbeam", "minimize", "mirrorcoat", "noretreat", "octolock", "painsplit", "partingshot", "payback",
    "perishsong", "phantomforce", "pollenpuff", "powershift", "powersplit", "powerswap", "powertrick", "powertrip",
    "protect", "psychup", "quash", "ragefist", "ragepowder", "reflecttype", "rest", "reversal", "roost", "round",
    "saltcure", "shedtail", "shellsidearm", "skyattack", "sleeptalk", "smackdown", "snore", "soak", "sparklingaria",
    "speedswap", "spikyshield", "spiritshackle", "spite", "spitup", "steelbeam", "stockpile", "stompingtantrum",
    "storedpower", "strengthsap", "struggle", "substitute", "suckerpunch", "supercellslam", "superfang", "swallow",
    "syrupbomb", "taunt", "temperflare", "throatchop", "topsyturvy", "torment", "transform", "triattack",
    "trickortreat", "tripleaxel", "upperhand", "uproar", "venoshock", "watershuriken", "waterspout", "yawn",
    // stage 3: moves of the field
    "auroraveil", "blizzard", "brickbreak", "ceaselessedge", "chillyreception", "courtchange", "defog",
    "electroshot", "expandingforce", "futuresight", "grassyglide", "gravapple", "growth", "healingwish",
    "hurricane", "icespinner", "magneticflux", "mistyexplosion", "moonlight", "morningsun", "mortalspin",
    "psychicfangs", "quickguard", "ragingbull", "rapidspin", "revivalblessing", "risingvoltage", "solarbeam",
    "solarblade", "steelroller", "stoneaxe", "synthesis", "terrainpulse", "thunder", "tidyup", "weatherball",
    "wideguard",
    // stage 4: moves that change or suppress abilities, Imprison
    "skillswap", "roleplay", "entrainment", "simplebeam", "worryseed", "gastroacid", "imprison",
    // stage 5: moves that need items
    "acrobatics", "belch", "bugbite", "corrosivegas", "covet", "fling", "knockoff", "pluck", "poltergeist", "recycle",
    "stuffcheeks", "teatime", "thief", "trick", "switcheroo",
];

const PROTECTS: [MoveId; 6] = [moves::PROTECT, moves::DETECT, moves::KINGSSHIELD, moves::SPIKYSHIELD, moves::BANEFULBUNKER, moves::ENDURE];

/// the move has no handler of its own (nor on a secondary or self effect)
#[inline]
fn no_handlers(mv: MoveId) -> bool {
    super::hooks::masks().moves[mv.0 as usize] == 0
}

fn is_two_turn(mv: MoveId) -> bool {
    [
        moves::BOUNCE,
        moves::DIG,
        moves::DIVE,
        moves::FLY,
        moves::PHANTOMFORCE,
        moves::SKYATTACK,
        moves::METEORBEAM,
        moves::SOLARBEAM,
        moves::SOLARBLADE,
        moves::ELECTROSHOT,
    ]
    .contains(&mv)
}

/// the weathers that weaken Solar Beam and Solar Blade
fn weak_weather(w: Cond) -> bool {
    [conds::RAINDANCE, conds::PRIMORDIALSEA, conds::SANDSTORM, conds::HAIL, conds::SNOWSCAPE].contains(&w)
}

impl<'a, O: Outcomes> Sim<'a, O> {
    /// pokemon.getLastDamagedBy(true): the last attack that dealt a number, from the other side
    pub fn last_damaged_by(&self, p: Pk) -> Option<Attack> {
        self.mon_at(p).attacks().iter().rev().find(|a| a.numeric && (a.source / 4) as usize != p.0).copied()
    }

    /// stat with stages and no modifiers: getStat(stat, false, true); under Wonder Room the stored stat
    /// keeps its name and the stage is the other defence's
    fn stat_boosted(&self, p: Pk, i: usize) -> u32 {
        let m = self.mon_at(p);
        let b = match i {
            1 if self.has_pseudo(conds::WONDERROOM) => 3,
            3 if self.has_pseudo(conds::WONDERROOM) => 1,
            _ => i,
        };
        super::calc::apply_stage(m.stats[i] as u32, m.boosts[b])
    }

    /// pokemon.getWeight: runEvent('ModifyWeight') (Heavy Metal, Light Metal), at least 1
    pub fn get_weight(&mut self, p: Pk) -> u32 {
        let w = self.mon_at(p).weight_hg;
        if !self.any_handler(Ev::ModifyWeight) {
            return w.max(1);
        }
        let mut ctx = Ctx::new(Some(p), None, Eff::None);
        match self.run_event(Ev::ModifyWeight, &mut ctx, R::Num(w as i32)) {
            R::Num(x) => (x.max(1)) as u32,
            _ => w.max(1),
        }
    }

    /// pokemon.getStat('spe') with its modifiers (paralysis)
    pub fn speed_stat(&mut self, p: Pk) -> u32 {
        self.get_stat(p.0, p.1, Stat::Spe, false, false)
    }

    // ------------------------------------------------------------------------------------------ Try

    /// onTry (singleEvent 'Try' with the user and the target)
    pub fn move_try(&mut self, user: Pk, t: Pk, am: &mut ActiveMove) -> R {
        if no_handlers(am.id) || am.future {
            return R::Undef;
        }
        let m = *self.mon_at(user);
        match am.id {
            // Champions: Belch and Stuff Cheeks stay selectable and fail here
            moves::BELCH => R::of_bool(m.ate_berry),
            moves::STUFFCHEEKS => R::of_bool(m.item.data().is_berry),
            moves::POLTERGEIST => R::of_bool(!self.mon_at(t).item.is_none()),
            moves::AURAWHEEL => {
                if m.species.data().name.starts_with("Morpeko") {
                    R::Undef
                } else {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::Move(am.id) });
                    R::Null
                }
            }
            moves::CLANGOROUSSOUL => {
                if (m.hp as f64) <= m.maxhp as f64 * 33.0 / 100.0 || m.maxhp == 1 {
                    R::False
                } else {
                    R::Undef
                }
            }
            moves::COMEUPPANCE | moves::METALBURST => {
                if !self.last_damaged_by(user).map_or(false, |a| a.this_turn) {
                    R::False
                } else {
                    R::Undef
                }
            }
            moves::COUNTER | moves::MIRRORCOAT => {
                let c = if am.id == moves::COUNTER { conds::COUNTER } else { conds::MIRRORCOAT };
                match self.vol(user, c) {
                    None => R::False,
                    Some(v) if v.data == 0 => R::False,
                    _ => R::Undef,
                }
            }
            moves::FAKEOUT | moves::FIRSTIMPRESSION => {
                if m.active_move_actions > 1 {
                    R::False
                } else {
                    R::Undef
                }
            }
            moves::FOLLOWME | moves::RAGEPOWDER => R::False,
            moves::LASTRESORT => {
                if m.nmoves < 2 {
                    return R::False;
                }
                let mut has = false;
                for ms in m.moves() {
                    if ms.id == moves::LASTRESORT {
                        has = true;
                        continue;
                    }
                    if !ms.used {
                        return R::False;
                    }
                }
                R::of_bool(has)
            }
            moves::MAGNETRISE => {
                if self.has_vol(t, conds::SMACKDOWN) || self.has_vol(t, conds::INGRAIN) {
                    R::False
                } else if self.gravity() {
                    self.row(Row::Cant { p: who(user), why: Fx::Cond(conds::GRAVITY), mv: am.id });
                    R::Null
                } else {
                    R::Undef
                }
            }
            moves::AURORAVEIL => {
                let w = self.weather();
                R::of_bool(w == conds::HAIL || w == conds::SNOWSCAPE)
            }
            moves::STEELROLLER => R::of_bool(!self.terrain().is_none()),
            moves::QUICKGUARD | moves::WIDEGUARD => R::of_bool(self.will_act()),
            moves::FUTURESIGHT => self.future_sight_try(user, t),
            moves::NORETREAT => {
                if self.has_vol(user, conds::NORETREAT) {
                    R::False
                } else {
                    R::Undef
                }
            }
            moves::REST => {
                if m.status == Status::Slp || self.has_ability(user, abilities::COMATOSE) {
                    R::False
                } else if m.hp == m.maxhp {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::HpFull });
                    R::Null
                } else if self.has_ability(user, abilities::INSOMNIA) || self.has_ability(user, abilities::VITALSPIRIT) {
                    let ab = m.ability;
                    self.ab_row(user, ab);
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::Ability(ab) });
                    R::Null
                } else {
                    R::Undef
                }
            }
            moves::ROUND => {
                let q = self.s.queue.as_slice();
                if let Some(i) = q.iter().position(|a| a.kind == ActionKind::Move && a.mv == moves::ROUND) {
                    let mut a = self.s.queue.list[i];
                    self.s.queue.remove(i);
                    a.source_effect = moves::ROUND;
                    a.order = 3;
                    self.s.queue.insert(0, a);
                }
                R::Undef
            }
            moves::SLEEPTALK | moves::SNORE => R::of_bool(m.status == Status::Slp || self.has_ability(user, abilities::COMATOSE)),
            moves::SPITUP | moves::SWALLOW => R::of_bool(self.has_vol(user, conds::STOCKPILE)),
            moves::STOCKPILE => {
                if self.vol(user, conds::STOCKPILE).map_or(false, |v| v.data >= 3) {
                    R::False
                } else {
                    R::Undef
                }
            }
            moves::SUCKERPUNCH => {
                let a = self.will_move(t);
                let ok = match a {
                    Some(a) if a.move_index != super::battle::RECHARGE => {
                        let mv = if a.move_index == super::battle::STRUGGLE { moves::STRUGGLE } else { a.mv };
                        mv.data().category != Category::Status && !self.has_vol(t, conds::MUSTRECHARGE)
                    }
                    _ => false,
                };
                if ok {
                    R::Undef
                } else {
                    R::False
                }
            }
            moves::UPPERHAND => {
                let a = self.will_move(t);
                let ok = match a {
                    Some(a) if a.move_index != super::battle::RECHARGE => {
                        let mv = if a.move_index == super::battle::STRUGGLE { moves::STRUGGLE } else { a.mv };
                        a.priority / 10 > 0 && mv.data().category != Category::Status
                    }
                    _ => false,
                };
                if ok {
                    R::Undef
                } else {
                    R::False
                }
            }
            _ => R::Undef,
        }
    }

    /// onPrepareHit (singleEvent 'PrepareHit' with the target and the user)
    pub fn move_prepare_hit(&mut self, user: Pk, t: Pk, am: &mut ActiveMove) -> R {
        if PROTECTS.contains(&am.id) {
            if !self.will_act() {
                return R::False;
            }
            if !self.has_vol(t, conds::STALL) {
                return R::True;
            }
            let mut ctx = Ctx::with_move(Some(t), None, am);
            return self.run_event(Ev::StallMove, &mut ctx, R::True);
        }
        match am.id {
            moves::FLING => self.fling_prepare(user, am),
            moves::DESTINYBOND => R::of_bool(!self.remove_volatile(t, conds::DESTINYBOND)),
            moves::ALLYSWITCH => self.add_volatile(t, conds::ALLYSWITCH, Some(user), Eff::Move(am.id), false),
            _ => R::Undef,
        }
    }

    /// onTryMove (singleEvent 'TryMove' with the user and the target)
    pub fn move_try_move(&mut self, user: Pk, t: Pk, am: &mut ActiveMove) -> R {
        if is_two_turn(am.id) {
            if let Some(c) = Cond::from_id(am.id.id()) {
                if self.remove_volatile(user, c) {
                    return R::Undef;
                }
            }
            self.row(Row::Prepare { p: who(user), mv: am.id });
            if am.id == moves::METEORBEAM || am.id == moves::ELECTROSHOT {
                self.boost(&[(2, 1)], user, Some(user), Eff::Move(am.id), false, false);
            }
            // no charge turn in sun (Solar Beam, Solar Blade) or rain (Electro Shot)
            if am.id == moves::SOLARBEAM || am.id == moves::SOLARBLADE {
                self.megasol_row(user);
            }
            let w = self.eff_weather_move(user);
            let sun = w == conds::SUNNYDAY || w == conds::DESOLATELAND;
            let rain = w == conds::RAINDANCE || w == conds::PRIMORDIALSEA;
            if ((am.id == moves::SOLARBEAM || am.id == moves::SOLARBLADE) && sun) || (am.id == moves::ELECTROSHOT && rain) {
                return R::Undef;
            }
            self.add_volatile(user, conds::TWOTURNMOVE, Some(t), Eff::Move(am.id), false);
            return R::Null;
        }
        match am.id {
            moves::BURNUP => {
                if self.has_type_pk(user, Type::Fire) {
                    R::Undef
                } else {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::Move(am.id) });
                    R::Null
                }
            }
            moves::DOUBLESHOCK => {
                if self.has_type_pk(user, Type::Electric) {
                    R::Undef
                } else {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::Move(am.id) });
                    R::Null
                }
            }
            _ => R::Undef,
        }
    }

    /// the move's onTryHit (singleEvent 'TryHit' with the target and the user)
    pub fn move_try_hit(&mut self, t: Pk, user: Pk, am: &mut ActiveMove) -> R {
        match am.id {
            moves::POLTERGEIST => {
                let item = self.mon_at(t).item;
                self.row(Row::Item { p: who(t), item, what: ItemWhat::Found, of: None, by: Fx::Move(am.id) });
                R::Undef
            }
            moves::SUBSTITUTE => {
                let m = self.mon_at(t);
                if self.has_vol(t, conds::SUBSTITUTE) || m.hp as u32 * 4 <= m.maxhp as u32 || m.maxhp == 1 {
                    self.row(Row::Fail { p: Some(who(t)), why: Fx::Cond(conds::SUBSTITUTE) });
                    return R::NotFail;
                }
                R::Undef
            }
            moves::SHEDTAIL => {
                let m = *self.mon_at(t);
                if !self.can_switch(t.0) || self.has_vol(t, conds::SUBSTITUTE) || m.hp <= (m.maxhp + 1) / 2 {
                    let why = if self.can_switch(t.0) { Fx::Move(am.id) } else { Fx::None };
                    self.row(Row::Fail { p: Some(who(t)), why });
                    return R::NotFail;
                }
                R::Undef
            }
            moves::CURSE => {
                if self.has_type_pk(user, Type::Ghost) && self.has_vol(t, conds::CURSE) {
                    return R::False;
                }
                R::Undef
            }
            moves::DISABLE => {
                let lm = self.mon_at(t).last_move;
                if lm.is_none() || lm == moves::STRUGGLE {
                    return R::False;
                }
                R::Undef
            }
            moves::CLANGOROUSSOUL => {
                let r = self.boost(&[(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)], t, Some(user), Eff::Move(am.id), false, false);
                if !r.truthy() {
                    return R::Null;
                }
                am.no_boosts = true;
                R::Undef
            }
            moves::ELECTRIFY => {
                if self.will_move(t).is_none() && self.mon_at(t).active_turns > 0 {
                    return R::False;
                }
                R::Undef
            }
            moves::LOCKON => {
                if self.has_vol(user, conds::LOCKON) {
                    return R::False;
                }
                R::Undef
            }
            moves::UPROAR => {
                for side in [t.0, 1 - t.0] {
                    let sd = &self.s.sides[side];
                    if sd.has_active && sd.active_mon().status == Status::Slp {
                        let p = (side, sd.active as usize);
                        self.cure_status(p);
                    }
                }
                R::Undef
            }
            moves::YAWN => {
                if self.mon_at(t).status != Status::None || !self.run_status_immunity(t, super::status::Imm::Status(Status::Slp)) {
                    return R::False;
                }
                R::Undef
            }
            moves::BRICKBREAK | moves::PSYCHICFANGS | moves::RAGINGBULL => {
                self.break_screens(t.0);
                R::Undef
            }
            moves::HEALINGWISH => {
                if !self.can_switch(user.0) {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::None });
                    return R::NotFail;
                }
                R::Undef
            }
            moves::REVIVALBLESSING => {
                if !self.s.sides[user.0].mons.iter().any(|m| m.fainted) {
                    return R::False;
                }
                R::Undef
            }
            moves::ROLEPLAY => {
                let (ta, ua) = (self.mon_at(t).ability, self.mon_at(user).ability);
                if ta == ua || ta.data().flags & aflag::FAILROLEPLAY != 0 || ua.data().flags & aflag::CANTSUPPRESS != 0 {
                    return R::False;
                }
                R::Undef
            }
            moves::ENTRAINMENT => {
                let (ta, ua) = (self.mon_at(t).ability, self.mon_at(user).ability);
                if t == user || ta == ua || ta.data().flags & aflag::CANTSUPPRESS != 0 || ta == abilities::TRUANT
                    || ua.data().flags & aflag::NOENTRAIN != 0
                {
                    return R::False;
                }
                R::Undef
            }
            moves::SIMPLEBEAM => {
                let ta = self.mon_at(t).ability;
                if ta.data().flags & aflag::CANTSUPPRESS != 0 || ta == abilities::SIMPLE || ta == abilities::TRUANT {
                    return R::False;
                }
                R::Undef
            }
            moves::WORRYSEED | moves::GASTROACID => {
                if self.mon_at(t).ability.data().flags & aflag::CANTSUPPRESS != 0 {
                    return R::False;
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    /// onTryImmunity: false = immune
    pub fn move_try_immunity(&mut self, user: Pk, t: Pk, am: &ActiveMove) -> bool {
        match am.id {
            moves::ATTRACT => {
                let (a, b) = (self.mon_at(t).gender, self.mon_at(user).gender);
                (a == 1 && b == 2) || (a == 2 && b == 1)
            }
            moves::ENDEAVOR => self.mon_at(user).hp < self.mon_at(t).hp,
            moves::LEECHSEED => !self.has_type_pk(t, Type::Grass),
            moves::OCTOLOCK => !self.has_type_pk(t, Type::Ghost),
            moves::TRICK | moves::SWITCHEROO => !self.has_ability(t, abilities::STICKYHOLD),
            moves::WORRYSEED => {
                let a = self.mon_at(t).ability;
                a != abilities::TRUANT && a != abilities::INSOMNIA
            }
            _ => true,
        }
    }

    // ------------------------------------------------------------------------------------------ Modify

    pub fn move_modify_type(&mut self, user: Pk, am: &mut ActiveMove) {
        match am.id {
            moves::AURAWHEEL => {
                am.ty = if self.mon_at(user).species.id() == "morpekohangry" { Type::Dark } else { Type::Electric };
            }
            moves::WEATHERBALL => {
                let w = self.eff_weather_move(user);
                if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
                    am.ty = Type::Fire;
                } else if w == conds::RAINDANCE || w == conds::PRIMORDIALSEA {
                    am.ty = Type::Water;
                } else if w == conds::SANDSTORM {
                    am.ty = Type::Rock;
                } else if w == conds::HAIL || w == conds::SNOWSCAPE {
                    am.ty = Type::Ice;
                }
            }
            moves::TERRAINPULSE => {
                if !self.grounded(user) {
                    return;
                }
                // the field's terrain itself, not the effective one
                let tr = self.s.field.terrain.id;
                if tr == conds::ELECTRICTERRAIN {
                    am.ty = Type::Electric;
                } else if tr == conds::GRASSYTERRAIN {
                    am.ty = Type::Grass;
                } else if tr == conds::MISTYTERRAIN {
                    am.ty = Type::Fairy;
                } else if tr == conds::PSYCHICTERRAIN {
                    am.ty = Type::Psychic;
                }
            }
            moves::RAGINGBULL => {
                let name = self.mon_at(user).species.data().name;
                am.ty = match name {
                    "Tauros-Paldea-Combat" => Type::Fighting,
                    "Tauros-Paldea-Blaze" => Type::Fire,
                    "Tauros-Paldea-Aqua" => Type::Water,
                    _ => am.ty,
                };
            }
            _ => {}
        }
    }

    pub fn move_modify_move(&mut self, user: Pk, target: Option<Pk>, am: &mut ActiveMove) {
        match am.id {
            moves::STRUGGLE => {
                am.ty = Type::Typeless;
                self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(moves::STRUGGLE), of: None });
            }
            moves::WEATHERBALL => {
                let w = self.eff_weather_move(user);
                let any = [conds::SUNNYDAY, conds::DESOLATELAND, conds::RAINDANCE, conds::PRIMORDIALSEA, conds::SANDSTORM, conds::HAIL, conds::SNOWSCAPE];
                if any.contains(&w) {
                    am.base_power *= 2;
                }
            }
            moves::TERRAINPULSE => {
                if !self.s.field.terrain.id.is_none() && self.grounded(user) {
                    am.base_power *= 2;
                }
            }
            moves::THUNDER | moves::HURRICANE => {
                let Some(t) = target else { return };
                let w = self.eff_weather_move(t);
                if w == conds::RAINDANCE || w == conds::PRIMORDIALSEA {
                    am.accuracy = ACC_ALWAYS;
                } else if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
                    am.accuracy = 50;
                }
            }
            moves::BLIZZARD => {
                let w = self.weather();
                if w == conds::HAIL || w == conds::SNOWSCAPE {
                    am.accuracy = ACC_ALWAYS;
                }
            }
            moves::GROWTH => {
                let w = self.eff_weather_move(user);
                if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
                    am.boosts = Some([2, 0, 2, 0, 0, 0, 0]);
                }
            }
            moves::EXPANDINGFORCE => {
                if self.terrain() == conds::PSYCHICTERRAIN && self.grounded(user) {
                    am.target = Target::AllAdjacentFoes;
                }
            }
            moves::CURSE => {
                if !self.has_type_pk(user, Type::Ghost) {
                    am.target = Target::Self_;
                }
            }
            moves::BEATUP => {
                let sd = &self.s.sides[user.0];
                let mut n = 0;
                for &slot in &sd.order {
                    let m = &sd.mons[slot as usize];
                    if slot as usize == user.1 || (!m.fainted && m.status == Status::None) {
                        am.allies[n] = (m.set_species.data().base_stats[1] / 10) as u8;
                        n += 1;
                    }
                }
                am.nallies = n as u8;
                am.multihit = (n as u8, n as u8);
            }
            moves::SHELLSIDEARM => {
                let Some(t) = target else { return };
                let lvl = (2 * self.mon_at(user).level as u64 / 5 + 2) as u64;
                let atk = self.stat_boosted(user, 0) as u64;
                let spa = self.stat_boosted(user, 2) as u64;
                let def = self.stat_boosted(t, 1).max(1) as u64;
                let spd = self.stat_boosted(t, 3).max(1) as u64;
                let physical = (lvl * 90 * atk / def) / 50;
                let special = (lvl * 90 * spa / spd) / 50;
                let phys = physical > special || (physical == special && {
                    let site = Site { effect: Effect::One(EffectRef::Move(am.id)), ..Site::new(Key::Proc, Some(user.0 as u8)) };
                    self.chance(site, 1, 2)
                });
                if phys {
                    am.category = Category::Physical;
                    am.flags |= flag::CONTACT;
                    am.physical_ssa = true;
                }
            }
            _ => {}
        }
    }

    /// onModifyTarget (Comeuppance, Metal Burst): the slot of the last attacker
    pub fn move_modify_target(&mut self, user: Pk, am: &ActiveMove) -> Option<Pk> {
        if am.id != moves::COMEUPPANCE && am.id != moves::METALBURST {
            return None;
        }
        let a = self.last_damaged_by(user)?;
        let side = (a.source / 4) as usize;
        Some((side, self.s.sides[side].active as usize))
    }

    /// beforeMoveCallback: Focus Punch loses its focus when hit
    pub fn before_move_callback(&mut self, user: Pk, am: &ActiveMove) -> bool {
        if am.id == moves::FOCUSPUNCH {
            let lost = self.vol(user, conds::FOCUSPUNCH).map_or(false, |v| v.data2 != 0);
            if lost {
                self.row(Row::Cant { p: who(user), why: Fx::Move(moves::FOCUSPUNCH), mv: moves::FOCUSPUNCH });
            }
            return lost;
        }
        false
    }

    // ------------------------------------------------------------------------------------------ Hit

    /// singleEvent('HitField') of the move
    pub fn move_hit_field(&mut self, _t: Pk, user: Pk, am: &mut ActiveMove) -> Option<R> {
        match am.id {
            moves::HAZE => {
                self.row(Row::BoostReset { p: None, what: Reset::Field, of: None, by: Fx::Move(am.id) });
                for side in 0..SIDES {
                    if self.s.sides[side].has_active && !self.s.active(side).fainted && self.s.active(side).hp > 0 {
                        self.s.active_mut(side).boosts = [0; 7];
                    }
                }
                Some(R::Undef)
            }
            moves::COURTCHANGE => Some(self.court_change(user)),
            moves::TEATIME => Some(self.teatime(user, am)),
            moves::PERISHSONG => {
                let mut result = false;
                let mut message = false;
                for side in 0..SIDES {
                    let sd = &self.s.sides[side];
                    if !sd.has_active || sd.active_mon().hp == 0 {
                        continue;
                    }
                    let p = (side, sd.active as usize);
                    let mut ctx = Ctx::with_move(Some(p), Some(user), am);
                    if self.run_event(Ev::Invulnerability, &mut ctx, R::True) == R::False {
                        self.row(Row::Miss { p: who(p), by: who(user) });
                        result = true;
                        continue;
                    }
                    let mut ctx = Ctx::with_move(Some(p), Some(user), am);
                    if self.run_event(Ev::TryHit, &mut ctx, R::True) == R::Null {
                        result = true;
                        continue;
                    }
                    if !self.has_vol(p, conds::PERISHSONG) {
                        self.add_volatile(p, conds::PERISHSONG, Some(user), Eff::Move(am.id), false);
                        result = true;
                        message = true;
                    }
                }
                if result && message {
                    self.row(Row::Once { p: None, fx: Fx::Move(am.id), of: None });
                }
                Some(if result { R::Undef } else { R::False })
            }
            _ => None,
        }
    }

    /// singleEvent('Hit') of a hit effect: the move's onHit, a secondary's or a self effect's
    pub fn move_on_hit(&mut self, t: Pk, user: Pk, am: &mut ActiveMove, md: Md) -> Option<R> {
        match md {
            Md::Primary => self.primary_on_hit(t, user, am),
            // Stench's added flinch has no handler of the move
            Md::Secondary(i) if (i as usize) < am.data().secondaries.len() => self.secondary_on_hit(t, user, am),
            Md::SelfOf => self.self_on_hit(t, am),
            _ => None,
        }
    }

    fn secondary_on_hit(&mut self, t: Pk, user: Pk, am: &mut ActiveMove) -> Option<R> {
        let eff = Eff::Move(am.id);
        match am.id {
            moves::ALLURINGVOICE => {
                if self.mon_at(t).stats_raised_this_turn {
                    self.add_volatile(t, conds::CONFUSION, Some(user), eff, false);
                }
                Some(R::Undef)
            }
            moves::BURNINGJEALOUSY => {
                if self.mon_at(t).stats_raised_this_turn {
                    self.try_set_status(t, Status::Brn, Some(user), eff);
                }
                Some(R::Undef)
            }
            moves::DIRECLAW | moves::TRIATTACK => {
                let vals: [&'static str; 3] = if am.id == moves::DIRECLAW { ["psn", "par", "slp"] } else { ["brn", "par", "frz"] };
                let site = Site { effect: Effect::One(EffectRef::Move(am.id)), ..Site::new(Key::Pick, Some(user.0 as u8)) };
                let i = self.o.sample_str(&site, &vals);
                let st = match vals[i] {
                    "psn" => Status::Psn,
                    "par" => Status::Par,
                    "slp" => Status::Slp,
                    "brn" => Status::Brn,
                    _ => Status::Frz,
                };
                self.try_set_status(t, st, Some(user), eff);
                Some(R::Undef)
            }
            moves::EERIESPELL => {
                if self.mon_at(t).hp == 0 {
                    return Some(R::Undef);
                }
                let lm = self.mon_at(t).last_move;
                if !lm.is_none() {
                    let lost = self.deduct_pp_amount(t, lm, 3);
                    if lost > 0 {
                        self.row(Row::Pp { p: who(t), mv: lm, lost, by: Fx::Move(am.id) });
                    }
                }
                Some(R::Undef)
            }
            moves::SPIRITSHACKLE => {
                if self.is_active_pk(user) {
                    self.add_volatile(t, conds::TRAPPED, Some(user), eff, true);
                }
                Some(R::Undef)
            }
            moves::THROATCHOP => {
                self.add_volatile(t, conds::THROATCHOP, Some(user), eff, false);
                Some(R::Undef)
            }
            _ => None,
        }
    }

    fn self_on_hit(&mut self, p: Pk, am: &mut ActiveMove) -> Option<R> {
        let lose = match am.id {
            moves::BURNUP => Type::Fire,
            moves::DOUBLESHOCK => Type::Electric,
            moves::BATONPASS | moves::SHEDTAIL => {
                self.mon_at_mut(p).skip_pursuit = true;
                return Some(R::Undef);
            }
            _ => return None,
        };
        let types = self.get_types(p, true);
        let mut out = [Type::Typeless; 2];
        let n = (types.n as usize).min(2);
        for i in 0..n {
            out[i] = if types.t[i] == lose { Type::Typeless } else { types.t[i] };
        }
        self.mon_at_mut(p).set_types(&out[..n]);
        if self.logging() {
            self.row(Row::Types { p: who(p), types: out, lost: lose, added: Type::Typeless, by: Fx::Move(am.id) });
        }
        Some(R::Undef)
    }

    fn primary_on_hit(&mut self, t: Pk, user: Pk, am: &mut ActiveMove) -> Option<R> {
        let eff = Eff::Move(am.id);
        if PROTECTS.contains(&am.id) {
            self.add_volatile(t, conds::STALL, Some(user), eff, false);
            return Some(R::Undef);
        }
        let r = match am.id {
            moves::ACUPRESSURE => {
                let b = self.mon_at(t).boosts;
                let mut names: [&'static str; 7] = [""; 7];
                let mut idx = [0usize; 7];
                let mut n = 0;
                for i in 0..7 {
                    if b[i] < 6 {
                        names[n] = BOOST_IDS[i];
                        idx[n] = i;
                        n += 1;
                    }
                }
                if n == 0 {
                    return Some(R::False);
                }
                let site = Site { effect: Effect::One(EffectRef::Move(am.id)), ..Site::new(Key::Pick, Some(user.0 as u8)) };
                let k = if n == 1 { 0 } else { self.o.sample_str(&site, &names[..n]) };
                self.boost(&[(idx[k], 2)], t, Some(user), eff, false, false);
                R::Undef
            }
            moves::AFTERYOU | moves::QUASH => R::False,
            moves::ALLYSWITCH => {
                self.row(Row::Fail { p: Some(who(user)), why: Fx::Move(am.id) });
                R::NotFail
            }
            moves::BATONPASS => {
                if !self.can_switch(t.0) {
                    self.row(Row::Fail { p: Some(who(t)), why: Fx::None });
                    R::NotFail
                } else {
                    R::Undef
                }
            }
            moves::BELLYDRUM => {
                let m = *self.mon_at(t);
                if m.hp as u32 * 2 <= m.maxhp as u32 || m.boosts[0] >= 6 || m.maxhp == 1 {
                    return Some(R::False);
                }
                self.direct_damage(m.maxhp as f64 / 2.0, t, Some(user), eff);
                self.boost(&[(0, 12)], t, Some(user), eff, false, false);
                R::Undef
            }
            moves::BLOCK | moves::MEANLOOK => self.add_volatile(t, conds::TRAPPED, Some(user), eff, true),
            moves::CLEARSMOG => {
                self.row(Row::BoostReset { p: Some(who(t)), what: Reset::All, of: None, by: Fx::Move(am.id) });
                self.mon_at_mut(t).boosts = [0; 7];
                R::Undef
            }
            moves::CLANGOROUSSOUL => {
                let mx = self.mon_at(t).maxhp as f64;
                self.direct_damage(mx * 33.0 / 100.0, t, Some(user), eff);
                R::Undef
            }
            moves::COPYCAT => {
                let mv = self.s.last_move;
                if mv.is_none() {
                    return Some(R::Undef);
                }
                if mv.data().has_flag(flag::FAILCOPYCAT) {
                    return Some(R::False);
                }
                self.use_move(user, mv, Tgt::Unset, am.id, false, MoveId::NONE);
                R::Undef
            }
            moves::CURSE => {
                if !self.has_type_pk(user, Type::Ghost) {
                    let r = self.boost(&[(4, -1), (0, 1), (1, 1)], user, Some(user), eff, false, false);
                    return Some(R::of_bool(r.truthy()));
                }
                let mx = self.mon_at(user).maxhp as f64;
                self.direct_damage(mx / 2.0, user, Some(user), eff);
                // Champions: aimed at the user (it became a Ghost after the target was chosen), the foe is cursed
                let t = if t.0 == user.0 { self.foe_of(user) } else { t };
                if self.is_active_pk(t) {
                    self.s.sides[t.0].delete_vol(conds::CURSE);
                }
                self.add_volatile(t, conds::CURSE, Some(user), eff, false);
                R::Undef
            }
            moves::FORESTSCURSE | moves::TRICKORTREAT => {
                let ty = if am.id == moves::FORESTSCURSE { Type::Grass } else { Type::Ghost };
                if self.has_type_pk(t, ty) {
                    return Some(R::False);
                }
                self.mon_at_mut(t).added_type = ty;
                if self.logging() {
                    let m = self.mon_at(t);
                    let mut types = [Type::Typeless; 2];
                    types[..m.ntypes as usize].copy_from_slice(&m.types[..m.ntypes as usize]);
                    self.row(Row::Types { p: who(t), types, lost: Type::Typeless, added: ty, by: Fx::Move(am.id) });
                }
                R::Undef
            }
            moves::GUARDSPLIT | moves::POWERSPLIT => {
                let (a, b) = if am.id == moves::GUARDSPLIT { (1, 3) } else { (0, 2) };
                for i in [a, b] {
                    let v = (self.mon_at(t).stats[i] as u32 + self.mon_at(user).stats[i] as u32) / 2;
                    self.mon_at_mut(t).stats[i] = v as u16;
                    self.mon_at_mut(user).stats[i] = v as u16;
                }
                self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(am.id), of: Some(who(t)) });
                R::Undef
            }
            moves::GUARDSWAP | moves::POWERSWAP => {
                let (a, b) = if am.id == moves::GUARDSWAP { (1, 3) } else { (0, 2) };
                self.row(Row::BoostReset { p: Some(who(user)), what: Reset::Swap, of: Some(who(t)), by: Fx::Move(am.id) });
                for i in [a, b] {
                    let x = self.mon_at(t).boosts[i];
                    let y = self.mon_at(user).boosts[i];
                    self.mon_at_mut(user).boosts[i] = x;
                    self.mon_at_mut(t).boosts[i] = y;
                }
                R::Undef
            }
            moves::HEALBELL => {
                self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(am.id), of: None });
                let mut success = false;
                // in the side's party order, the active Pokémon first
                let order = self.s.sides[t.0].order;
                for slot in order {
                    if self.cure_status((t.0, slot as usize)) {
                        success = true;
                    }
                }
                R::of_bool(success)
            }
            moves::HEALPULSE => {
                let a = if self.has_ability(user, abilities::MEGALAUNCHER) {
                    super::calc::modify(self.mon_at(t).base_maxhp as u32, 3072) as f64
                } else {
                    (self.mon_at(t).base_maxhp as f64 * 0.5).ceil()
                };
                let r = self.heal(a, t, Some(user), eff);
                if !r.truthy() {
                    self.row(Row::Fail { p: Some(who(t)), why: Fx::HpFull });
                    R::NotFail
                } else {
                    R::True
                }
            }
            moves::INSTRUCT => self.instruct(t, user),
            moves::JAWLOCK => {
                self.add_volatile(user, conds::TRAPPED, Some(t), eff, true);
                self.add_volatile(t, conds::TRAPPED, Some(user), eff, true);
                R::Undef
            }
            moves::LOCKON => {
                self.add_volatile(user, conds::LOCKON, Some(t), eff, false);
                self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(am.id), of: Some(who(t)) });
                R::Undef
            }
            moves::MAGICPOWDER | moves::SOAK => {
                let ty = if am.id == moves::MAGICPOWDER { Type::Psychic } else { Type::Water };
                let types = self.get_types(t, false);
                if types.as_slice() == [ty] {
                    if am.id == moves::SOAK {
                        self.row(Row::Fail { p: Some(who(t)), why: Fx::None });
                    }
                    return Some(if am.id == moves::SOAK { R::Null } else { R::False });
                }
                self.mon_at_mut(t).set_types(&[ty]);
                self.types_row(t, Fx::Move(am.id));
                R::Undef
            }
            moves::PAINSPLIT => {
                let th = self.mon_at(t).hp as i64;
                let avg = ((th + self.mon_at(user).hp as i64) / 2).max(1);
                let change = th - avg;
                let (bt, bu) = (self.mon_at(t).hp, self.mon_at(user).hp);
                self.set_hp(t, (self.mon_at(t).hp as i64 - change) as f64);
                let at = self.mon_at(t).hp;
                self.row(Row::Hp { p: who(t), before: bt, after: at, cause: Fx::Move(am.id), of: Some(who(user)) });
                self.set_hp(user, avg as f64);
                let au = self.mon_at(user).hp;
                self.row(Row::Hp { p: who(user), before: bu, after: au, cause: Fx::Move(am.id), of: Some(who(t)) });
                R::Undef
            }
            moves::PARTINGSHOT => {
                let r = self.boost(&[(0, -1), (2, -1)], t, Some(user), eff, false, false);
                if !r.truthy() && !self.has_ability(t, abilities::MIRRORARMOR) {
                    am.no_self_switch = true;
                }
                R::Undef
            }
            moves::SKILLSWAP => {
                if self.skill_swap(user, t) {
                    R::Undef
                } else {
                    R::False
                }
            }
            moves::ROLEPLAY => {
                let ta = self.mon_at(t).ability;
                R::of_bool(self.set_ability_by(user, ta, Some(t), Fx::Move(am.id)))
            }
            moves::ENTRAINMENT => {
                let ua = self.mon_at(user).ability;
                R::of_bool(self.set_ability_by(t, ua, Some(user), Fx::Move(am.id)))
            }
            moves::SIMPLEBEAM => R::of_bool(self.set_ability_by(t, abilities::SIMPLE, None, Fx::Move(am.id))),
            moves::WORRYSEED => {
                if !self.set_ability_by(t, abilities::INSOMNIA, None, Fx::Move(am.id)) {
                    return Some(R::False);
                }
                if self.mon_at(t).status == Status::Slp {
                    self.cure_status(t);
                }
                R::Undef
            }
            moves::PSYCHUP => {
                let b = self.mon_at(t).boosts;
                self.mon_at_mut(user).boosts = b;
                self.row(Row::BoostReset { p: Some(who(user)), what: Reset::Copy, of: Some(who(t)), by: Fx::Move(am.id) });
                for c in [conds::DRAGONCHEER, conds::FOCUSENERGY] {
                    self.remove_volatile(user, c);
                }
                for c in [conds::DRAGONCHEER, conds::FOCUSENERGY] {
                    if let Some(v) = self.vol(t, c) {
                        self.add_volatile(user, c, Some(user), eff, false);
                        if c == conds::DRAGONCHEER {
                            if let Some(x) = self.vol_mut(user, c) {
                                x.data2 = v.data2;
                            }
                        }
                    }
                }
                R::Undef
            }
            moves::REFLECTTYPE => {
                let tt = self.get_types(t, true);
                let mut list: Vec<Type> = tt.as_slice().iter().copied().filter(|x| *x != Type::Typeless).collect();
                let added = self.mon_at(t).added_type;
                if list.is_empty() {
                    if added != Type::Typeless {
                        list = vec![Type::Normal];
                    } else {
                        return Some(R::False);
                    }
                }
                let m = self.mon_at_mut(user);
                m.set_types(&list);
                m.added_type = added;
                self.types_row(user, Fx::Move(am.id));
                R::Undef
            }
            moves::REST => {
                self.status_null = false;
                if !self.set_status(t, Status::Slp, Some(user), eff, false) {
                    self.no_fail_row = std::mem::take(&mut self.status_null);
                    return Some(R::False);
                }
                self.mon_at_mut(t).status_time = 3;
                let mx = self.mon_at(t).maxhp as f64;
                self.heal(mx, t, Some(user), eff);
                R::Undef
            }
            moves::SLEEPTALK => {
                let m = *self.mon_at(user);
                let mut names: [&'static str; 4] = [""; 4];
                let mut ids = [MoveId::NONE; 4];
                let mut n = 0;
                for ms in m.moves() {
                    let d = ms.id.data();
                    if d.has_flag(flag::NOSLEEPTALK) || d.has_flag(flag::CHARGE) {
                        continue;
                    }
                    names[n] = d.id;
                    ids[n] = ms.id;
                    n += 1;
                }
                if n == 0 {
                    return Some(R::False);
                }
                let site = Site { effect: Effect::One(EffectRef::Move(am.id)), ..Site::new(Key::Pick, Some(user.0 as u8)) };
                let k = if names[..n].iter().all(|x| *x == names[0]) { 0 } else { self.o.sample_str(&site, &names[..n]) };
                self.use_move(user, ids[k], Tgt::Unset, am.id, false, MoveId::NONE);
                R::Undef
            }
            moves::SPEEDSWAP => {
                let a = self.mon_at(t).stats[4];
                let b = self.mon_at(user).stats[4];
                self.mon_at_mut(t).stats[4] = b;
                self.mon_at_mut(user).stats[4] = a;
                self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(am.id), of: Some(who(t)) });
                R::Undef
            }
            moves::SPITE => {
                let lm = self.mon_at(t).last_move;
                if lm.is_none() {
                    return Some(R::False);
                }
                let lost = self.deduct_pp_amount(t, lm, 4);
                if lost == 0 {
                    return Some(R::False);
                }
                self.row(Row::Pp { p: who(t), mv: lm, lost, by: Fx::Move(am.id) });
                R::Undef
            }
            moves::STRENGTHSAP => {
                if self.mon_at(t).boosts[0] == -6 {
                    return Some(R::False);
                }
                let atk = self.stat_boosted(t, 0) as f64;
                let success = self.boost(&[(0, -1)], t, Some(user), Eff::None, false, true);
                let h = self.heal(atk, user, Some(t), eff);
                R::of_bool(h.truthy() || success.truthy())
            }
            moves::SUBSTITUTE => {
                let mx = self.mon_at(t).maxhp as f64;
                self.direct_damage(mx / 4.0, t, Some(user), eff);
                R::Undef
            }
            moves::SHEDTAIL => {
                let mx = self.mon_at(t).maxhp as f64;
                self.direct_damage((mx / 2.0).ceil(), t, Some(user), eff);
                R::Undef
            }
            moves::SWALLOW => {
                let layers = self.vol(user, conds::STOCKPILE).map_or(1, |v| v.data.max(1)) as usize;
                let m = [1024u32, 2048, 4096][layers.min(3) - 1];
                let a = super::calc::modify(self.mon_at(user).maxhp as u32, m) as f64;
                let ok = self.heal(a, user, Some(user), eff).truthy();
                if !ok {
                    self.row(Row::Fail { p: Some(who(user)), why: Fx::HpFull });
                }
                self.remove_volatile(user, conds::STOCKPILE);
                if ok {
                    R::True
                } else {
                    R::NotFail
                }
            }
            moves::TOPSYTURVY => {
                let m = self.mon_at_mut(t);
                let mut any = false;
                for b in m.boosts.iter_mut() {
                    if *b != 0 {
                        *b = -*b;
                        any = true;
                    }
                }
                if !any {
                    R::False
                } else {
                    self.row(Row::BoostReset { p: Some(who(t)), what: Reset::Invert, of: None, by: Fx::Move(am.id) });
                    R::Undef
                }
            }
            moves::TRANSFORM => R::of_bool(self.transform_into(user, t)),
            moves::MORNINGSUN | moves::MOONLIGHT | moves::SYNTHESIS => self.weather_heal(t, am),
            moves::DEFOG => self.defog(t, user, am),
            moves::TIDYUP => self.tidy_up(user),
            moves::STEELROLLER => {
                self.clear_terrain();
                R::Undef
            }
            moves::BUGBITE | moves::PLUCK => {
                let item = self.mon_at(t).item;
                if self.mon_at(user).hp > 0 && item.data().is_berry && self.take_item(t, Some(user)).item().is_some() {
                    self.row(Row::Item { p: who(t), item, what: ItemWhat::Removed, of: Some(who(user)), by: Fx::Move(am.id) });
                    // singleEvent('Eat') on the user: no UseItem, TryEatItem or Unnerve
                    self.item_eat(item, user, Some(user), eff);
                    self.eat_item_event(user, Some(user), eff, item);
                    self.mon_at_mut(user).ate_berry = true;
                }
                R::Undef
            }
            moves::CORROSIVEGAS => {
                let item = self.mon_at(t).item;
                if self.take_item(t, Some(user)).item().is_some() {
                    self.row(Row::Item { p: who(t), item, what: ItemWhat::Removed, of: Some(who(user)), by: Fx::Move(am.id) });
                } else {
                    self.row(Row::Fail { p: Some(who(t)), why: Fx::Move(am.id) });
                }
                R::Undef
            }
            moves::RECYCLE => {
                let m = *self.mon_at(t);
                if !m.item.is_none() || m.last_item.is_none() {
                    return Some(R::False);
                }
                self.mon_at_mut(t).last_item = ItemId::NONE;
                // the row comes before setItem: the item's Start may use it at once (Misty Seed in Misty Terrain)
                self.row(Row::Item { p: who(t), item: m.last_item, what: ItemWhat::Received, of: None, by: Fx::Move(am.id) });
                self.set_item(t, m.last_item, Some(user), eff);
                R::Undef
            }
            moves::STUFFCHEEKS => {
                let b = self.boost(&[(1, 2)], t, Some(t), eff, false, false);
                if !b.truthy() {
                    return Some(R::Null);
                }
                self.eat_item(t, true, Some(t), eff);
                R::Undef
            }
            moves::TRICK | moves::SWITCHEROO => self.trick(t, user, am),
            moves::FLING => {
                if am.fling_item.is_none() {
                    return None;
                }
                self.fling_on_hit(t, user, am.fling_item, am.id);
                R::Undef
            }
            _ => return None,
        };
        Some(r)
    }

    /// Trick and Switcheroo's onHit: the two items change hands, unless either cannot be taken or given
    fn trick(&mut self, t: Pk, user: Pk, am: &ActiveMove) -> R {
        use super::items::Taken;
        let yours = self.take_item(t, Some(user));
        let mine = self.take_item(user, None);
        let restore = |s: &mut Self| {
            if let Taken::Item(i) = yours {
                s.restore_item(t, i);
            }
            if let Taken::Item(i) = mine {
                s.restore_item(user, i);
            }
        };
        if yours == Taken::Blocked || mine == Taken::Blocked || (yours == Taken::NoItem && mine == Taken::NoItem) {
            restore(self);
            return R::False;
        }
        // singleEvent('TakeItem') of each item with its receiver as holder (a stone for another species)
        let blocked = mine.item().map_or(false, |i| !self.item_take_single(i, t)) || yours.item().map_or(false, |i| !self.item_take_single(i, user));
        if blocked {
            restore(self);
            return R::False;
        }
        let eff = Eff::Move(am.id);
        let by = Fx::Move(am.id);
        self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(moves::TRICK), of: Some(who(t)) });
        if let Taken::Item(i) = mine {
            self.set_item(t, i, Some(user), eff);
            self.row(Row::Item { p: who(t), item: i, what: ItemWhat::Received, of: Some(who(user)), by });
        }
        if let Taken::Item(i) = yours {
            self.set_item(user, i, Some(user), eff);
            self.row(Row::Item { p: who(user), item: i, what: ItemWhat::Received, of: Some(who(t)), by });
        }
        R::Undef
    }

    /// Fling's onPrepareHit: the item's power and effect; the item leaves at the next Update
    fn fling_prepare(&mut self, user: Pk, am: &mut ActiveMove) -> R {
        use super::items::FlingFx;
        if self.ignoring_item(user, true) {
            return R::False;
        }
        let item = self.mon_at(user).item;
        if !self.item_take_single(item, user) {
            return R::False;
        }
        let power = item.data().fling_power;
        if item.is_none() || power == 0 {
            return R::False;
        }
        am.base_power = power as u32;
        match super::items::fling_fx(item) {
            FlingFx::OnHit => {
                if item.data().is_berry && self.has_ability(user, abilities::CUDCHEW) {
                    // singleEvent('EatItem', Cud Chew) with Fling as the effect
                    let empty = self.s.queue.len == 0;
                    let m = self.mon_at_mut(user);
                    m.ab_data = item.0 as i16;
                    m.ab_count = if empty { 1 } else { 2 };
                }
                am.fling_item = item;
            }
            FlingFx::Status(st) => am.push_secondary(ExtraSec { chance: None, status: st, vol: Cond::NONE }),
            FlingFx::Volatile(v) => am.push_secondary(ExtraSec { chance: None, status: Status::None, vol: v }),
            FlingFx::None => {}
        }
        self.sync_active(am);
        self.add_volatile(user, conds::FLING, Some(user), Eff::Move(am.id), false);
        R::Undef
    }

    /// Teatime's onHitField: every active Pokémon the move reaches eats its berry
    fn teatime(&mut self, user: Pk, am: &mut ActiveMove) -> R {
        let mut targets = [None; SIDES];
        let mut n = 0;
        for side in 0..SIDES {
            let sd = &self.s.sides[side];
            if !sd.has_active || sd.active_mon().fainted {
                continue;
            }
            let p = (side, sd.active as usize);
            let mut ctx = Ctx::with_move(Some(p), Some(user), am);
            if self.run_event(Ev::Invulnerability, &mut ctx, R::True) == R::False {
                self.row(Row::Miss { p: who(p), by: who(user) });
                continue;
            }
            let mut ctx = Ctx::with_move(Some(p), Some(user), am);
            if self.run_event(Ev::TryHit, &mut ctx, R::True).truthy() && self.mon_at(p).item.data().is_berry {
                targets[n] = Some(p);
                n += 1;
            }
        }
        self.row(Row::Once { p: None, fx: Fx::Move(am.id), of: None });
        if n == 0 {
            self.row(Row::Fail { p: Some(who(user)), why: Fx::Move(am.id) });
            return R::NotFail;
        }
        for p in targets[..n].iter().flatten() {
            self.eat_item(*p, true, None, Eff::Move(am.id));
        }
        R::Undef
    }

    /// Instruct: the target uses its last move again, now (queue.prioritizeAction)
    fn instruct(&mut self, t: Pk, _user: Pk) -> R {
        let m = *self.mon_at(t);
        let lm = m.last_move;
        if lm.is_none() {
            return R::False;
        }
        let d = lm.data();
        let slot = m.moves().iter().position(|ms| ms.id == lm);
        if d.has_flag(flag::FAILINSTRUCT) || d.has_flag(flag::CHARGE) || d.has_flag(flag::RECHARGE)
            || self.has_vol(t, conds::BEAKBLAST) || self.has_vol(t, conds::FOCUSPUNCH)
            || slot.map_or(false, |i| m.moves[i].pp == 0)
        {
            return R::False;
        }
        let index = slot.map(|i| i as u8).unwrap_or(super::battle::NOSLOT);
        self.row(Row::Once { p: Some(who(t)), fx: Fx::Move(moves::INSTRUCT), of: Some(who(_user)) });
        self.prioritize_move(t, lm, index);
        R::Undef
    }

    /// pokemon.transformInto
    pub fn transform_into(&mut self, user: Pk, t: Pk) -> bool {
        let tm = *self.mon_at(t);
        let um = *self.mon_at(user);
        if tm.fainted || um.illusion || tm.illusion || self.has_vol(t, conds::SUBSTITUTE) || tm.transformed || um.transformed {
            return false;
        }
        let types = if self.has_vol(t, conds::ROOST) {
            let mut v = [Type::Typeless; 2];
            v[..tm.ntypes as usize].copy_from_slice(&tm.types[..tm.ntypes as usize]);
            (v, tm.ntypes)
        } else {
            let tt = self.get_types(t, true);
            let mut v = [Type::Typeless; 2];
            let n = (tt.n as usize).min(2);
            v[..n].copy_from_slice(&tt.t[..n]);
            (v, n as u8)
        };
        // setSpecies: types, weight, the stats of the user's set in that form, speed
        self.set_species(user.0, user.1, tm.species);
        let m = self.mon_at_mut(user);
        m.transformed = true;
        m.times_attacked = tm.times_attacked;
        m.weight_hg = tm.weight_hg;
        m.set_types(&types.0[..types.1 as usize]);
        m.added_type = tm.added_type;
        m.stats = tm.stats;
        m.base_moves = m.moves;
        let mut slots = [MoveSlot::default(); 4];
        for i in 0..tm.nmoves as usize {
            let id = tm.moves[i].id;
            let pp = id.data().pp.min(5);
            slots[i] = MoveSlot { id, pp, max_pp: pp, disabled: false, disabled_hidden: false, used: false };
        }
        m.moves = slots;
        m.nmoves = tm.nmoves;
        m.boosts = tm.boosts;
        for c in [conds::DRAGONCHEER, conds::FOCUSENERGY] {
            self.remove_volatile(user, c);
        }
        for c in [conds::DRAGONCHEER, conds::FOCUSENERGY] {
            if let Some(v) = self.vol(t, c) {
                self.add_volatile(user, c, Some(user), Eff::Move(moves::TRANSFORM), false);
                if let Some(x) = self.vol_mut(user, c) {
                    x.data2 = v.data2;
                }
            }
        }
        self.row(Row::Transform { p: who(user), into: who(t) });
        self.set_ability(user, tm.ability, Some(user), true, true);
        true
    }

    // ------------------------------------------------------------------------------------------ damage

    /// basePowerCallback: Some(Some(bp)), Some(None) for false, None when the move has none
    pub fn move_base_power_callback(&mut self, user: Pk, t: Pk, am: &mut ActiveMove) -> Option<Option<f64>> {
        if no_handlers(am.id) {
            return None;
        }
        let bp = am.base_power as f64;
        let um = *self.mon_at(user);
        let tm = *self.mon_at(t);
        let v = match am.id {
            moves::ASSURANCE => {
                if tm.hurt_this_turn.map_or(false, |h| h > 0) {
                    bp * 2.0
                } else {
                    bp
                }
            }
            moves::AVALANCHE => {
                let hit = um.attacks().iter().any(|a| a.source == pk_code(t) && a.damage > 0 && a.this_turn);
                if hit {
                    bp * 2.0
                } else {
                    bp
                }
            }
            moves::BEATUP => {
                if am.nallies == 0 {
                    return Some(Some(5.0));
                }
                let x = am.allies[0];
                am.allies.copy_within(1..TEAM, 0);
                am.nallies -= 1;
                5.0 + x as f64
            }
            moves::ELECTROBALL => {
                let a = self.speed_stat(user) as f64;
                let b = self.speed_stat(t) as f64;
                let mut ratio = (a / b).floor();
                if !ratio.is_finite() {
                    ratio = 0.0;
                }
                [40.0, 60.0, 80.0, 120.0, 150.0][(ratio as usize).min(4)]
            }
            moves::ERUPTION | moves::WATERSPOUT => bp * um.hp as f64 / um.maxhp as f64,
            moves::FLAIL | moves::REVERSAL => {
                let ratio = ((um.hp as u32 * 48 / um.maxhp as u32) as i32).max(1);
                if ratio < 2 {
                    200.0
                } else if ratio < 5 {
                    150.0
                } else if ratio < 10 {
                    100.0
                } else if ratio < 17 {
                    80.0
                } else if ratio < 33 {
                    40.0
                } else {
                    20.0
                }
            }
            moves::GRASSKNOT | moves::LOWKICK => {
                let w = self.get_weight(t);
                if w >= 2000 {
                    120.0
                } else if w >= 1000 {
                    100.0
                } else if w >= 500 {
                    80.0
                } else if w >= 250 {
                    60.0
                } else if w >= 100 {
                    40.0
                } else {
                    20.0
                }
            }
            moves::GYROBALL => {
                let a = self.speed_stat(t) as f64;
                let b = self.speed_stat(user) as f64;
                let mut p = (25.0 * a / b).floor() + 1.0;
                if !p.is_finite() {
                    p = 1.0;
                }
                p.min(150.0)
            }
            moves::HARDPRESS => {
                let x = (tm.hp as u64 * 4096 / tm.maxhp as u64) as u64;
                let v = ((100 * (100 * x) + 2048 - 1) / 4096) / 100;
                if v == 0 {
                    1.0
                } else {
                    v as f64
                }
            }
            moves::HEATCRASH | moves::HEAVYSLAM => {
                let tw = self.get_weight(t);
                let uw = self.get_weight(user);
                if uw >= tw * 5 {
                    120.0
                } else if uw >= tw * 4 {
                    100.0
                } else if uw >= tw * 3 {
                    80.0
                } else if uw >= tw * 2 {
                    60.0
                } else {
                    40.0
                }
            }
            moves::HEX | moves::INFERNALPARADE => {
                if tm.status != Status::None || self.has_ability(t, abilities::COMATOSE) {
                    bp * 2.0
                } else {
                    bp
                }
            }
            moves::LASTRESPECTS => 50.0 + 50.0 * self.s.sides[user.0].total_fainted as f64,
            moves::PAYBACK => {
                if tm.newly_switched || self.will_move(t).is_some() {
                    bp
                } else {
                    bp * 2.0
                }
            }
            moves::POWERTRIP | moves::STOREDPOWER => {
                let pos: i32 = um.boosts.iter().filter(|b| **b > 0).map(|b| *b as i32).sum();
                bp + 20.0 * pos as f64
            }
            moves::RAGEFIST => (50.0 + 50.0 * um.times_attacked as f64).min(350.0),
            moves::ROUND => {
                if am.source_effect == moves::ROUND {
                    bp * 2.0
                } else {
                    bp
                }
            }
            moves::SPITUP => {
                let l = self.vol(user, conds::STOCKPILE).map_or(0, |v| v.data);
                if l == 0 {
                    return Some(None);
                }
                l as f64 * 100.0
            }
            moves::STOMPINGTANTRUM | moves::TEMPERFLARE => {
                if um.move_last_turn_result == MoveResult::False {
                    bp * 2.0
                } else {
                    bp
                }
            }
            moves::TRIPLEAXEL => 20.0 * am.hit as f64,
            moves::RISINGVOLTAGE => {
                if self.terrain() == conds::ELECTRICTERRAIN && self.grounded(t) {
                    bp * 2.0
                } else {
                    bp
                }
            }
            moves::WATERSHURIKEN => bp,
            moves::ACROBATICS => {
                if um.item.is_none() {
                    bp * 2.0
                } else {
                    bp
                }
            }
            _ => return None,
        };
        Some(Some(v))
    }

    /// damageCallback: Some(Some(hp)), Some(None) for false, None when the move has none
    pub fn move_damage_callback(&mut self, user: Pk, t: Pk, am: &ActiveMove) -> Option<Option<f64>> {
        if no_handlers(am.id) {
            return None;
        }
        let um = *self.mon_at(user);
        let tm = *self.mon_at(t);
        let v = match am.id {
            moves::COMEUPPANCE | moves::METALBURST => match self.last_damaged_by(user) {
                Some(a) => {
                    let x = a.damage as f64 * 1.5;
                    if x == 0.0 {
                        1.0
                    } else {
                        x
                    }
                }
                None => 0.0,
            },
            moves::COUNTER | moves::MIRRORCOAT => {
                let c = if am.id == moves::COUNTER { conds::COUNTER } else { conds::MIRRORCOAT };
                match self.vol(user, c) {
                    None => 0.0,
                    Some(v) => {
                        if v.hp == 0 {
                            1.0
                        } else {
                            v.hp as f64
                        }
                    }
                }
            }
            moves::ENDEAVOR => tm.hp as f64 - um.hp as f64,
            moves::FINALGAMBIT => {
                let d = um.hp as f64;
                self.faint_by(user, None, false);
                d
            }
            moves::SUPERFANG => (tm.hp as f64 / 2.0).floor().max(1.0),
            _ => return None,
        };
        Some(Some(v))
    }

    /// the move's own onBasePower (a BasePower handler; chainModify on the event)
    pub fn move_on_base_power(&mut self, user: Pk, t: Pk, am: &ActiveMove, ctx: &mut Ctx) {
        if no_handlers(am.id) {
            return;
        }
        let tm = *self.mon_at(t);
        let um = *self.mon_at(user);
        match am.id {
            moves::BARBBARRAGE | moves::VENOSHOCK => {
                if matches!(tm.status, Status::Psn | Status::Tox) {
                    ctx.chain_modify(2, 1);
                }
            }
            moves::FACADE => {
                if um.status != Status::None && um.status != Status::Slp {
                    ctx.chain_modify(2, 1);
                }
            }
            moves::LASHOUT => {
                if um.stats_lowered_this_turn {
                    ctx.chain_modify(2, 1);
                }
            }
            moves::FICKLEBEAM => {
                let site = Site { effect: Effect::One(EffectRef::Move(am.id)), ..Site::new(Key::Proc, Some(user.0 as u8)) };
                if self.chance(site, 3, 10) {
                    self.row(Row::Once { p: Some(who(user)), fx: Fx::Move(am.id), of: None });
                    ctx.chain_modify(2, 1);
                }
            }
            moves::SOLARBEAM | moves::SOLARBLADE => {
                if weak_weather(self.eff_weather_move(user)) {
                    ctx.chain_modify(1, 2);
                }
            }
            moves::EXPANDINGFORCE => {
                if self.terrain() == conds::PSYCHICTERRAIN && self.grounded(user) {
                    ctx.chain_modify(3, 2);
                }
            }
            moves::MISTYEXPLOSION => {
                if self.terrain() == conds::MISTYTERRAIN && self.grounded(user) {
                    ctx.chain_modify(3, 2);
                }
            }
            moves::GRAVAPPLE => {
                if self.gravity() {
                    ctx.chain_modify(3, 2);
                }
            }
            moves::KNOCKOFF => {
                // singleEvent('TakeItem') of the item only: Sticky Hold does not stop the boost
                if !tm.item.is_none() && self.item_take_single(tm.item, t) {
                    ctx.chain_modify(3, 2);
                }
            }
            _ => {}
        }
    }

    /// singleEvent('Effectiveness') of the move for one defending type
    pub fn move_effectiveness(&self, am: &ActiveMove, ty: Type, base: i32) -> i32 {
        match am.id {
            moves::FREEZEDRY if ty == Type::Water => 1,
            moves::FLYINGPRESS => base + effectiveness(Type::Flying, ty),
            _ => base,
        }
    }

    // ------------------------------------------------------------------------------------------ after

    /// singleEvent('AfterMove') of the move
    pub fn move_after_move(&mut self, user: Pk, _target: Option<Pk>, am: &ActiveMove) {
        match am.id {
            moves::BEAKBLAST => {
                self.remove_volatile(user, conds::BEAKBLAST);
            }
            moves::SPITUP => {
                self.remove_volatile(user, conds::STOCKPILE);
            }
            moves::SPARKLINGARIA => {
                let foe = self.foe_of(user);
                if self.mon_at(user).fainted || am.hit_targets.is_none() || am.sheer_force {
                    for side in 0..SIDES {
                        if self.s.sides[side].has_active {
                            self.s.sides[side].delete_vol(conds::SPARKLINGARIA);
                        }
                    }
                    return;
                }
                if am.hit_targets == Some(true) && foe != user && self.is_active_pk(foe) && self.remove_volatile(foe, conds::SPARKLINGARIA)
                    && self.mon_at(foe).status == Status::Brn
                {
                    self.cure_status(foe);
                }
            }
            _ => {}
        }
    }

    /// singleEvent('MoveFail'): crash damage
    pub fn move_fail(&mut self, user: Pk, _t: Pk, am: &ActiveMove) {
        if no_handlers(am.id) {
            return;
        }
        let m = *self.mon_at(user);
        match am.id {
            moves::AXEKICK | moves::HIGHJUMPKICK | moves::SUPERCELLSLAM => {
                if self.logging() {
                    self.hp_cause = Some(Fx::Crash(am.id));
                }
                self.damage(m.base_maxhp as f64 / 2.0, user, Some(user), Eff::Cond(Cond::NONE));
                self.hp_cause = None;
            }
            // not when Parental Bond made it a multi-hit move
            moves::STEELBEAM if am.multihit.1 == 0 => {
                self.damage((m.maxhp as f64 / 2.0).round(), user, Some(user), Eff::Cond(Cond::NONE));
            }
            _ => {}
        }
    }

    /// singleEvent('AfterMoveSecondarySelf'): Fell Stinger
    pub fn move_after_move_secondary_self(&mut self, user: Pk, t: Pk, am: &ActiveMove) {
        if am.id == moves::FELLSTINGER {
            let tm = self.mon_at(t);
            if tm.fainted || tm.hp == 0 {
                self.boost(&[(0, 3)], user, Some(user), Eff::Move(am.id), false, false);
            }
        }
    }

    /// singleEvent('DisableMove') of a move slot (Champions Fake Out, First Impression)
    pub fn move_disable_move(&mut self, p: Pk, mv: MoveId) {
        if (mv == moves::FAKEOUT || mv == moves::FIRSTIMPRESSION) && self.mon_at(p).active_move_actions > 0 {
            self.disable_move(p, mv);
        }
    }

    /// beforeTurnCallback (Counter, Mirror Coat)
    pub fn move_before_turn(&mut self, p: Pk, mv: MoveId) {
        let c = if mv == moves::COUNTER { conds::COUNTER } else { conds::MIRRORCOAT };
        self.add_volatile(p, c, Some(p), Eff::Move(mv), false);
    }

    /// priorityChargeCallback (Focus Punch, Beak Blast)
    pub fn move_priority_charge(&mut self, p: Pk, mv: MoveId) {
        let c = match mv {
            moves::FOCUSPUNCH => conds::FOCUSPUNCH,
            moves::CHILLYRECEPTION => conds::CHILLYRECEPTION,
            _ => conds::BEAKBLAST,
        };
        self.add_volatile(p, c, Some(p), Eff::Move(mv), false);
    }

    /// singleEvent('HitSide') of the move (moves aimed at a side)
    pub fn move_hit_side(&mut self, _side: usize, user: Pk, am: &mut ActiveMove) -> Option<R> {
        match am.id {
            moves::QUICKGUARD | moves::WIDEGUARD => {
                self.add_volatile(user, conds::STALL, Some(user), Eff::Move(am.id), false);
                Some(R::Undef)
            }
            // the side's Pokémon with Plus or Minus: in singles only the user
            moves::MAGNETICFLUX => {
                if !(self.has_ability(user, abilities::PLUS) || self.has_ability(user, abilities::MINUS)) {
                    return Some(R::False);
                }
                let r = self.boost(&[(1, 1), (3, 1)], user, Some(user), Eff::Move(am.id), false, true);
                Some(R::of_bool(r.truthy()))
            }
            _ => None,
        }
    }

    /// singleEvent('AfterHit') of the move, after a hit that dealt damage
    pub fn move_after_hit(&mut self, t: Pk, user: Pk, am: &ActiveMove) {
        match am.id {
            moves::KNOCKOFF => {
                if let Some(it) = self.take_item(t, None).item() {
                    self.row(Row::Item { p: who(t), item: it, what: ItemWhat::Removed, of: Some(who(user)), by: Fx::Move(am.id) });
                }
                return;
            }
            moves::COVET | moves::THIEF => {
                let um = *self.mon_at(user);
                if !um.item.is_none() || self.has_vol(user, conds::GEM) {
                    return;
                }
                let Some(it) = self.take_item(t, Some(user)).item() else { return };
                if !self.item_take_single(it, user) || !self.set_item(user, it, Some(user), Eff::Move(am.id)) {
                    self.restore_item(t, it);
                } else {
                    self.row(Row::Item { p: who(user), item: it, what: ItemWhat::Received, of: Some(who(t)), by: Fx::Move(am.id) });
                }
                return;
            }
            _ => {}
        }
        if am.sheer_force {
            return;
        }
        match am.id {
            moves::RAPIDSPIN | moves::MORTALSPIN => self.spin_clear(user),
            moves::CEASELESSEDGE => {
                self.add_side_cond(t.0, conds::SPIKES, t, Eff::Move(am.id));
            }
            moves::STONEAXE => {
                self.add_side_cond(t.0, conds::STEALTHROCK, t, Eff::Move(am.id));
            }
            moves::ICESPINNER => {
                self.clear_terrain();
            }
            _ => {}
        }
    }

    /// singleEvent('AfterSubDamage') of the move, after a hit into a Substitute
    pub fn move_after_sub_damage(&mut self, t: Pk, user: Pk, am: &ActiveMove) {
        let alive = self.mon_at(user).hp > 0 && !am.sheer_force;
        match am.id {
            moves::RAPIDSPIN | moves::MORTALSPIN if alive => self.spin_clear(user),
            moves::CEASELESSEDGE if alive => {
                self.add_side_cond(t.0, conds::SPIKES, t, Eff::Move(am.id));
            }
            moves::STONEAXE if alive => {
                self.add_side_cond(t.0, conds::STEALTHROCK, t, Eff::Move(am.id));
            }
            moves::ICESPINNER if alive => {
                self.clear_terrain();
            }
            moves::STEELROLLER if !am.sheer_force => {
                self.clear_terrain();
            }
            _ => {}
        }
    }
}
