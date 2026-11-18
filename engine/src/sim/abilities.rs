//! Abilities (data/abilities.ts and data/mods/champions/abilities.ts): the handlers of every implemented
//! ability, the ability state, suppression (Gastro Acid, Neutralizing Gas, Mold Breaker), weather suppression
//! (Air Lock, Cloud Nine) and ability changes (pokemon.setAbility, battle.skillSwap). An ability not in
//! `IMPLEMENTED_ABILITIES` marks the battle unimplemented as soon as a Pokémon has it. Messages are left out.

use super::battle::Sim;
use super::data::*;
use super::events::*;
use super::hooks::Hook;
use super::moves::ActiveMove;
use super::outcome::*;
use super::state::*;
use super::status::Imm;
use super::log::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;

/// Test coverage: when on, every ability handler that changed something is counted (rig only; off in play).
pub static COVERAGE: AtomicBool = AtomicBool::new(false);
const ZERO: AtomicU32 = AtomicU32::new(0);
pub static TRIGGERS: [AtomicU32; 320] = [ZERO; 320];

/// Counts a handler run of `a` that changed the battle or the event.
#[inline]
pub fn count_trigger(a: AbilityId) {
    TRIGGERS[(a.0 as usize).min(319)].fetch_add(1, Ordering::Relaxed);
}

#[inline]
pub fn coverage_on() -> bool {
    COVERAGE.load(Ordering::Relaxed)
}

/// The abilities whose handlers are all implemented here (or that have none).
pub const IMPLEMENTED_ABILITIES: &[&str] = &[
    "noability",
    // no handlers, or behaviour implemented where Showdown checks hasAbility
    "levitate", "ballfetch", "honeygather", "runaway", "dancer", "illusion",
    // gold team and usage order
    "roughskin", "toughclaws", "torrent", "intimidate", "sandstream", "protean", "multiscale", "adaptability",
    "blaze", "stamina", "speedboost", "emergencyexit", "thermalexchange", "goodasgold", "aerilate", "auraguard",
    "pressure", "grassysurge", "justified", "regenerator", "drought", "innerfocus", "clearbody", "toxicdebris",
    "sharpness", "cursedbody", "snowwarning", "moldbreaker", "psychicsurge", "unaware", "flamebody", "prankster",
    "hugepower", "libero", "contrary", "noguard", "supremeoverlord", "megalauncher", "static", "technician",
    "ironfist", "drizzle", "unburden", "weakarmor", "pixilate", "thickfat", "swiftswim", "steadfast", "scrappy",
    "shadowtag", "sturdy", "overgrow", "cutecharm", "illuminate", "electromorphosis", "gooey", "frisk",
    "chlorophyll", "limber", "competitive", "defiant", "trace", "flashfire", "piercingdrill", "skilllink",
    "naturalcure", "sapsipper", "leafguard", "oblivious", "shellarmor", "mirrorarmor", "moxie", "waterbubble",
    "swarm", "rockhead", "innardsout", "bulletproof", "spicyspray", "compoundeyes", "unnerve", "moody", "harvest",
    "synchronize", "poisonpoint", "eelevate", "waterabsorb", "quickdraw", "keeneye", "sniper", "punkrock",
    "eartheater", "magicguard", "firemane", "refrigerate", "earlybird", "superluck", "infiltrator", "berserk",
    "sheerforce", "lightningrod", "dragonize", "dryskin", "sandrush", "purifyingsalt", "corrosion", "healer",
    "telepathy", "insomnia", "hypercutter", "seedsower", "filter", "heavymetal", "strongjaw", "hydration",
    "tangledfeet", "motordrive", "heatproof", "furcoat", "electricsurge", "stakeout", "snowcloak",
    "armortail", "cudchew", "fairyaura", "flowerveil", "purepower", "sandforce", "mummy", "sandspit", "symbiosis",
    "solarpower", "galewings", "effectspore", "magmaarmor", "aromaveil", "queenlymajesty", "marvelscale",
    "poisonheal", "damp", "soundproof", "shedskin", "poisontouch", "rivalry", "gluttony", "guts", "sweetveil",
    "icebody", "fluffy", "raindish", "sandveil", "solidrock", "liquidooze", "voltabsorb", "slushrush",
    "wanderingspirit", "surgesurfer", "cheekpouch", "analytic", "aftermath", "airlock", "angerpoint", "angershell",
    "anticipation", "arenatrap", "baddreams", "battery", "battlearmor", "beadsofruin", "beastboost", "bigpecks",
    "chillingneigh", "cloudnine", "colorchange", "costar", "cottondown", "curiousmedicine", "darkaura",
    "dauntlessshield", "dazzling", "defeatist", "download", "dragonsmaw", "flareboost", "forewarn", "friendguard",
    "fullmetalbody", "galvanize", "grasspelt", "grimneigh", "guarddog", "hadronengine", "hospitality", "hustle",
    "icescales", "immunity", "intrepidsword", "ironbarbs", "klutz", "lightmetal", "lingeringaroma", "liquidvoice",
    "longreach", "magician", "magnetpull", "merciless", "mindseye", "minus", "mistysurge", "myceliummight",
    "neuroforce", "normalize", "orichalcumpulse", "overcoat", "owntempo", "pastelveil", "perishbody", "pickpocket",
    "pickup", "plus", "powerspot", "prismarmor", "protosynthesis", "quarkdrive", "quickfeet",
    "rattled", "reckless", "rockypayload", "screencleaner", "serenegrace", "shadowshield", "shielddust", "simple",
    "slowstart", "soulheart", "stall", "steamengine", "steelworker", "steelyspirit", "stench", "stickyhold",
    "stormdrain", "suctioncups", "supersweetsyrup", "swordofruin", "tabletsofruin", "vesselofruin",
    "tanglinghair", "teravolt", "turboblaze", "tintedlens", "toxicboost", "toxicchain", "transistor", "triage",
    "unseenfist", "victorystar", "vitalspirit", "watercompaction", "waterveil", "wellbakedbody", "whitesmoke",
    "wimpout", "windpower", "windrider", "wonderguard", "wonderskin", "ripen", "gorillatactics", "truant",
    "mimicry", "magicbounce", "megasol", "parentalbond", "aurabreak", "asoneglastrier", "asonespectrier",
    "battlebond",
    // only act with an ally, with Terastallization or in doubles: nothing to do in Champions singles
    "embodyaspectcornerstone", "embodyaspecthearthflame", "embodyaspectteal", "embodyaspectwellspring",
    "receiver", "powerofalchemy", "teraformzero", "commander",
    // form changes
    "imposter", "stancechange", "zerotohero", "hungerswitch", "disguise", "forecast",
    "comatose", "poisonpuppeteer",
    // stage 5: the last abilities of legal species (Gulp Missile, Ice Face, Shields Down and Tera Shell act only
    // on species that cannot be brought; their handlers check the species)
    "opportunist", "stalwart", "gulpmissile", "iceface", "shieldsdown", "terashell",
];

fn implemented_table() -> &'static Vec<bool> {
    static T: OnceLock<Vec<bool>> = OnceLock::new();
    T.get_or_init(|| ABILITIES.iter().map(|a| a.id.is_empty() || IMPLEMENTED_ABILITIES.contains(&a.id)).collect())
}

#[inline]
pub fn ability_implemented(a: AbilityId) -> bool {
    implemented_table()[a.0 as usize]
}

/// Bits of `Mon::ab_flags` (the ability's effectState).
pub mod abf {
    /// Protean and Libero changed the type this switch-in
    pub const USED: u8 = 1;
    /// Air Lock, Cloud Nine and Neutralizing Gas are ending
    pub const ENDING: u8 = 2;
    /// Berserk and Anger Shell: checkedBerserk / checkedAngerShell is false (healing berries wait)
    pub const PENDING: u8 = 4;
    /// Trace looks for an ability to copy
    pub const SEEK: u8 = 8;
    pub const UNNERVED: u8 = 16;
    /// Disguise or Ice Face was hit
    pub const BUSTED: u8 = 32;
    /// Ripen: the berry eaten was a resist berry (berryWeaken)
    pub const BERRY_WEAKEN: u8 = 64;
}

/// Bits of `Mon::ab_battle`: once-per-battle flags kept on the Pokémon (swordBoost, shieldBoost, syrupTriggered).
pub mod abb {
    pub const SWORD: u8 = 1;
    pub const SHIELD: u8 = 2;
    pub const SYRUP: u8 = 4;
    pub const BOND: u8 = 8;
    /// Zero to Hero's message was shown (heroMessageDisplayed)
    pub const HERO: u8 = 16;
}

/// Effect Spore's draw: random(100) cut at 11, 21 and 30
static EFFECT_SPORE: [&str; 100] = {
    let mut a = [""; 100];
    let mut i = 0;
    while i < 30 {
        a[i] = if i < 11 { "slp" } else if i < 21 { "par" } else { "psn" };
        i += 1;
    }
    a
};

const NO_MODIFY_TYPE: [&str; 7] = ["judgment", "multiattack", "naturalgift", "revelationdance", "technoblast", "terrainpulse", "weatherball"];

fn flags(a: AbilityId) -> u16 {
    a.data().flags
}

impl<'a, O: Outcomes> Sim<'a, O> {
    // ------------------------------------------------------------------------------------------ state

    /// pokemon.ignoringAbility
    pub fn ignoring_ability(&self, p: Pk) -> bool {
        let m = self.mon_at(p);
        if !m.is_active {
            return true;
        }
        let f = flags(m.ability);
        if f & aflag::NOTRANSFORM != 0 && m.transformed {
            return true;
        }
        if f & aflag::CANTSUPPRESS != 0 {
            return false;
        }
        if self.s.sides[p.0].has_vol(conds::GASTROACID) && self.s.sides[p.0].active as usize == p.1 {
            return true;
        }
        if m.ability == abilities::NEUTRALIZINGGAS {
            return false;
        }
        for side in 0..SIDES {
            let sd = &self.s.sides[side];
            if !sd.has_active {
                continue;
            }
            let x = sd.active_mon();
            if x.is_active && x.ability == abilities::NEUTRALIZINGGAS && !sd.has_vol(conds::GASTROACID) && !x.transformed
                && x.ab_flags & abf::ENDING == 0
            {
                return true;
            }
        }
        false
    }

    /// pokemon.hasAbility
    #[inline]
    pub fn has_ability(&self, p: Pk, a: AbilityId) -> bool {
        self.mon_at(p).ability == a && !self.ignoring_ability(p)
    }

    /// battle.suppressingAbility(target): the move being used ignores abilities (Mold Breaker)
    pub fn suppressing_ability(&self, target: Pk) -> bool {
        let Some(am) = self.active else { return false };
        let Some(ap) = pk_of(self.s.active_pokemon) else { return false };
        am.ignore_ability && self.mon_at(ap).is_active && ap != target
    }

    /// The runEvent checks before an ability handler: Mold Breaker for breakable abilities, then Gastro Acid
    /// and Neutralizing Gas.
    pub fn ability_skipped(&self, pk: Pk, ab: AbilityId, e: Ev) -> bool {
        if flags(ab) & aflag::BREAKABLE != 0 && self.suppressing_ability(pk) {
            return true;
        }
        self.ignoring_ability(pk)
            && !matches!(e, Ev::SwitchIn) // never reached: the switch-in has its own check
    }

    /// field.suppressingWeather: an active Pokémon's Air Lock or Cloud Nine
    pub fn suppressing_weather(&self) -> bool {
        for side in 0..SIDES {
            let sd = &self.s.sides[side];
            if !sd.has_active {
                continue;
            }
            let m = sd.active_mon();
            if m.fainted || !m.ability.data().suppress_weather || m.ab_flags & abf::ENDING != 0 {
                continue;
            }
            if !self.ignoring_ability((side, sd.active as usize)) {
                return true;
            }
        }
        false
    }

    /// battle.suppressingSecondaries: Sheer Force on the move being used
    pub fn suppressing_secondaries(&self) -> bool {
        let Some(am) = self.active else { return false };
        let Some(ap) = pk_of(self.s.active_pokemon) else { return false };
        am.sheer_force && self.has_ability(ap, abilities::SHEERFORCE)
    }

    /// checkMoveMakesContact (no Protective Pads: items are not in play)
    #[inline]
    pub fn makes_contact(&self, am: &ActiveMove) -> bool {
        am.has_flag(flag::CONTACT)
    }

    /// Records an ability without an implementation: the battle is not exact from here.
    pub fn note_ability(&mut self, a: AbilityId) {
        if !ability_implemented(a) {
            self.s.unimpl.note(EffectRef::Ability(a), Hook::Start);
        }
    }

    /// a fresh abilityState (initEffectState with the Pokémon as target)
    pub fn init_ability_state(&mut self, p: Pk) {
        let active = self.mon_at(p).is_active;
        let order = if active {
            self.s.effect_order = self.s.effect_order.wrapping_add(1);
            self.s.effect_order
        } else {
            0
        };
        let m = self.mon_at_mut(p);
        m.ab_order = order;
        m.ab_flags = 0;
        m.ab_data = 0;
        m.ab_count = 0;
        if self.is_active_pk(p) {
            self.s.sides[p.0].opp_boosts = None;
        }
    }

    // ------------------------------------------------------------------------------------------ helpers

    fn foe_active(&self, p: Pk) -> Option<Pk> {
        let f = 1 - p.0;
        let sd = &self.s.sides[f];
        if sd.has_active && !sd.active_mon().fainted && sd.active_mon().hp > 0 {
            Some((f, sd.active as usize))
        } else {
            None
        }
    }

    fn hp_third(&self, p: Pk) -> bool {
        let m = self.mon_at(p);
        (m.hp as f64) <= m.maxhp as f64 / 3.0
    }

    fn eff_weather_is(&self, p: Pk, ws: &[Cond]) -> bool {
        ws.contains(&self.eff_weather(p))
    }

    fn weather_is(&self, ws: &[Cond]) -> bool {
        ws.contains(&self.weather())
    }

    fn ab_boost(&mut self, b: &[(usize, i8)], target: Pk, source: Option<Pk>, ab: AbilityId) -> R {
        self.boost(b, target, source, Eff::Ability(ab), false, false)
    }

    /// pokemon.formeChange (Champions): a new form; a permanent one also becomes the base form, with its
    /// maximum HP, and takes the form's ability unless Disguise or Ice Face caused it
    pub fn forme_change(&mut self, p: Pk, id: &str, permanent: bool, source: AbilityId) {
        let Some(sp) = SpeciesId::from_id(id) else { return };
        self.set_species(p.0, p.1, sp);
        // the row names its ability only for Forecast and Flower Gift (formeChange with the effect)
        let by = if !permanent && (source == abilities::FORECAST || source == abilities::FLOWERGIFT) { Fx::Ability(source) } else { Fx::None };
        self.row(Row::Form { p: who(p), form: sp, by });
        if permanent {
            self.mon_at_mut(p).base_species = sp;
            self.update_max_hp(p.0, p.1);
            if source != abilities::DISGUISE && source != abilities::ICEFACE {
                let ab = sp.data().abilities[0];
                self.set_ability(p, ab, None, true, false);
                self.mon_at_mut(p).base_ability = ab;
            }
        }
    }

    /// Disguise's checks before a hit: Mimikyu in its disguise, not behind a Substitute, not immune
    fn disguise_blocks(&mut self, t: Pk, am: &ActiveMove) -> bool {
        if self.mon_at(t).species.id() != "mimikyu" {
            return false;
        }
        let hit_sub = self.has_vol(t, conds::SUBSTITUTE) && !am.has_flag(flag::BYPASSSUB) && !am.infiltrates;
        !hit_sub && self.run_immunity(t, am)
    }

    /// pokemon.getBestStat(unboosted, unmodified)
    pub fn best_stat(&mut self, p: Pk, unboosted: bool, unmodified: bool) -> usize {
        let mut best = 0;
        let mut v = 0;
        for (i, st) in [Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe].iter().enumerate() {
            let x = self.get_stat(p.0, p.1, *st, unboosted, unmodified);
            if x > v {
                v = x;
                best = i;
            }
        }
        best
    }

    // ------------------------------------------------------------------------------------------ events

    /// One ability handler of a runEvent.
    pub fn ability_event(&mut self, e: Ev, h: &H, ctx: &mut Ctx, relay: R) -> R {
        let ab = h.ab;
        let me = h.pk;
        use abilities as A;
        match e {
            Ev::BasePower => self.ab_base_power(ab, h.pre, me, ctx, relay),
            Ev::ModifyAtk | Ev::ModifySpA => self.ab_modify_offense(e, ab, h.pre, me, ctx, relay),
            Ev::ModifyDef | Ev::ModifySpD => self.ab_modify_defense(e, ab, h.pre, me, ctx),
            Ev::ModifyDamage => self.ab_modify_damage(ab, h.pre, me, ctx),
            Ev::ModifySTAB => {
                if ab == A::ADAPTABILITY {
                    let am = ctx.am.unwrap();
                    let user = ctx.target.unwrap();
                    if am.data().force_stab || self.has_type_pk(user, am.ty) {
                        return R::Num(if relay == R::Num(8192) { 9216 } else { 8192 });
                    }
                }
                R::Undef
            }
            Ev::ModifyType => {
                let Some(am) = ctx.am.as_mut() else { return R::Undef };
                let to = match ab {
                    A::AERILATE => Type::Flying,
                    A::PIXILATE => Type::Fairy,
                    A::REFRIGERATE => Type::Ice,
                    A::GALVANIZE => Type::Electric,
                    A::DRAGONIZE => Type::Dragon,
                    A::NORMALIZE => {
                        let no = ["hiddenpower", "judgment", "multiattack", "naturalgift", "revelationdance", "struggle", "technoblast", "terrainpulse", "weatherball"];
                        if !no.contains(&am.id.id()) {
                            am.ty = Type::Normal;
                            am.type_changer = ab;
                        }
                        return R::Undef;
                    }
                    A::LIQUIDVOICE => {
                        if am.has_flag(flag::SOUND) {
                            am.ty = Type::Water;
                        }
                        return R::Undef;
                    }
                    _ => return R::Undef,
                };
                if am.ty == Type::Normal && !NO_MODIFY_TYPE.contains(&am.id.id()) {
                    am.ty = to;
                    am.type_changer = ab;
                }
                R::Undef
            }
            Ev::ModifyMove => self.ab_modify_move(ab, me, ctx),
            Ev::ModifyPriority => {
                let R::Num(pr) = relay else { return R::Undef };
                let Some(am) = ctx.am.as_mut() else { return R::Undef };
                match ab {
                    A::PRANKSTER if am.category == Category::Status => {
                        am.prankster = true;
                        R::Num(pr + 1)
                    }
                    A::GALEWINGS if am.ty == Type::Flying && self.mon_at(me).hp == self.mon_at(me).maxhp => R::Num(pr + 1),
                    A::TRIAGE if am.has_flag(flag::HEAL) => R::Num(pr + 3),
                    _ => R::Undef,
                }
            }
            Ev::FractionalPriority => {
                let am = ctx.am.unwrap();
                match ab {
                    A::QUICKDRAW => {
                        if am.category != Category::Status {
                            let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Proc, Some(me.0 as u8)) };
                            if self.chance(site, 3, 10) {
                                self.ab_row(me, ab);
                                return R::Num(1);
                            }
                        }
                        R::Undef
                    }
                    A::MYCELIUMMIGHT if am.category == Category::Status => R::Num(-1),
                    A::STALL => R::Num(-1),
                    _ => R::Undef,
                }
            }
            Ev::TryHit => self.ab_try_hit(ab, me, ctx),
            Ev::TryHitSide => {
                // onAllyTryHitSide: Magic Bounce reflects hazards and other side moves
                if ab == A::MAGICBOUNCE && h.pre == Pre::Ally {
                    let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                    if t.0 == src.0 || am.has_bounced || !am.has_flag(flag::REFLECTABLE) || self.semi_invulnerable(t) {
                        return R::Undef;
                    }
                    self.bounce_next = true;
                    self.use_move(me, am.id, super::moves::Tgt::Some(src), MoveId::NONE, false, MoveId::NONE);
                    return R::Null;
                }
                // onAllyTryHitSide: Soundproof only shows its immunity (Heal Bell)
                if ab == A::SOUNDPROOF && h.pre == Pre::Ally && ctx.am.map_or(false, |am| am.has_flag(flag::SOUND)) {
                    self.ab_immune(me, ab);
                }
                R::Undef
            }
            Ev::TryMove => {
                // onFoeTryMove (Dazzling, Queenly Majesty, Armor Tail), onAnyTryMove (Damp)
                let am = ctx.am.unwrap();
                match ab {
                    A::DAZZLING | A::QUEENLYMAJESTY | A::ARMORTAIL => {
                        if am.target == Target::FoeSide || (am.target == Target::All && !["perishsong", "flowershield", "rototiller"].contains(&am.id.id())) {
                            return R::Undef;
                        }
                        // onFoeTryMove(target, source, move): `source` is the move's target
                        let Some(src) = ctx.source else { return R::Undef };
                        if (src.0 == me.0 || am.target == Target::All) && am.priority > 0 {
                            // Showdown's row names the holder
                            self.row(Row::Cant { p: who(me), why: Fx::Ability(ab), mv: am.id });
                            return R::False;
                        }
                        R::Undef
                    }
                    A::DAMP => {
                        if ["explosion", "mindblown", "mistyexplosion", "selfdestruct"].contains(&am.id.id()) {
                            self.row(Row::Cant { p: who(me), why: Fx::Ability(ab), mv: am.id });
                            return R::False;
                        }
                        R::Undef
                    }
                    _ => R::Undef,
                }
            }
            Ev::DamagingHit => self.ab_damaging_hit(ab, h.pre, me, ctx),
            Ev::Damage => self.ab_damage(ab, h.pre, me, ctx, relay),
            Ev::SetStatus => self.ab_set_status(ab, h.pre, me, ctx),
            Ev::AfterSetStatus => {
                // Poison Puppeteer: onAnyAfterSetStatus, Pecharunt's poison also confuses
                if ab == A::POISONPUPPETEER && h.pre == Pre::Any {
                    let (Some(t), Some(src)) = (ctx.target, ctx.source) else { return R::Undef };
                    if src != me || self.mon_at(src).base_species.data().name != "Pecharunt" || !ctx.eff.is_move() {
                        return R::Undef;
                    }
                    let st = status_of(ctx.arg);
                    if t != src && (st == Status::Psn || st == Status::Tox) {
                        self.add_volatile(t, conds::CONFUSION, Some(src), Eff::Ability(ab), false);
                    }
                    return R::Undef;
                }
                if ab == A::SYNCHRONIZE {
                    let (Some(t), Some(src)) = (ctx.target, ctx.source) else { return R::Undef };
                    if src == t || ctx.eff == Eff::Cond(conds::TOXICSPIKES) {
                        return R::Undef;
                    }
                    let st = status_of(ctx.arg);
                    if st == Status::Slp || st == Status::Frz {
                        return R::Undef;
                    }
                    self.ab_row(t, ab);
                    // a statused attacker: Showdown's setStatus failure rows for Synchronize's effect
                    let cur = self.mon_at(src).status;
                    if cur != Status::None && self.mon_at(src).hp > 0 && self.mon_at(src).is_active {
                        let p = if cur == st { src } else { t };
                        let why = if cur == st { Fx::Status(st) } else { Fx::None };
                        self.row(Row::Fail { p: Some(who(p)), why });
                    }
                    self.try_set_status(src, st, Some(t), Eff::Ability(ab));
                }
                R::Undef
            }
            Ev::TryAddVolatile => self.ab_try_add_volatile(ab, h.pre, me, ctx),
            Ev::Immunity => {
                let code = ctx.arg;
                let imm = |i: Imm| super::field::imm_code(i) == code;
                let sand = imm(Imm::Cond(conds::SANDSTORM));
                let hail = imm(Imm::Cond(conds::HAIL));
                match ab {
                    A::SANDVEIL | A::SANDRUSH | A::SANDFORCE if sand => R::False,
                    A::ICEBODY | A::SNOWCLOAK if hail => R::False,
                    A::OVERCOAT if sand || hail || imm(Imm::Powder) => R::False,
                    A::MAGMAARMOR if imm(Imm::Status(Status::Frz)) => R::False,
                    A::OBLIVIOUS if imm(Imm::Cond(conds::ATTRACT)) => R::False,
                    _ => R::Undef,
                }
            }
            Ev::Update => {
                self.ab_update(ab, me);
                R::Undef
            }
            Ev::TakeItem => {
                let Some(t) = ctx.target else { return R::Undef };
                match ab {
                    A::STICKYHOLD => {
                        let src = ctx.source;
                        if self.mon_at(t).hp == 0 {
                            return R::Undef;
                        }
                        if (src.is_some() && src != Some(t)) || self.s.active_move == moves::KNOCKOFF {
                            self.ab_row(t, ab);
                            return R::False;
                        }
                        R::Undef
                    }
                    A::UNBURDEN => {
                        self.add_volatile(t, conds::UNBURDEN, Some(t), Eff::Ability(ab), false);
                        R::Undef
                    }
                    _ => R::Undef,
                }
            }
            Ev::TryEatItem => {
                // onFoeTryEatItem: no berry for the foe of an Unnerve holder
                if matches!(ab, A::UNNERVE | A::ASONEGLASTRIER | A::ASONESPECTRIER) && h.pre == Pre::Foe {
                    return R::of_bool(self.mon_at(me).ab_flags & abf::UNNERVED == 0);
                }
                // Berserk and Anger Shell: healing berries wait until the HP check after the move
                if matches!(ab, A::BERSERK | A::ANGERSHELL) && h.pre == Pre::Own {
                    let heal = matches!(
                        ItemId(ctx.arg).data().id,
                        "aguavberry" | "enigmaberry" | "figyberry" | "iapapaberry" | "magoberry" | "sitrusberry" | "wikiberry" | "oranberry" | "berryjuice"
                    );
                    if heal && self.mon_at(me).ab_flags & abf::PENDING != 0 {
                        return R::False;
                    }
                    return R::Undef;
                }
                // Ripen announces itself for every berry it is about to double
                if ab == A::RIPEN && h.pre == Pre::Own {
                    self.ab_row(me, ab);
                }
                R::Undef
            }
            Ev::EatItem => {
                let Some(t) = ctx.target else { return R::Undef };
                let item = ItemId(ctx.arg);
                match ab {
                    A::CHEEKPOUCH => {
                        let a = self.mon_at(t).base_maxhp as f64 / 3.0;
                        self.heal(a, t, ctx.source, Eff::Ability(ab));
                    }
                    A::CUDCHEW => {
                        let by_bite = ctx.eff == Eff::Move(moves::BUGBITE) || ctx.eff == Eff::Move(moves::PLUCK);
                        if item.data().is_berry && !by_bite {
                            let empty = self.s.queue.len == 0;
                            let m = self.mon_at_mut(me);
                            m.ab_data = item.0 as i16;
                            m.ab_count = if empty { 1 } else { 2 };
                        }
                    }
                    A::RIPEN => {
                        let m = self.mon_at_mut(t);
                        if super::items::is_resist_berry(item) {
                            m.ab_flags |= abf::BERRY_WEAKEN;
                        } else {
                            m.ab_flags &= !abf::BERRY_WEAKEN;
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::AfterUseItem => {
                if ab == A::UNBURDEN && h.pre == Pre::Own {
                    self.add_volatile(me, conds::UNBURDEN, Some(me), Eff::Ability(ab), false);
                }
                R::Undef
            }
            Ev::AfterMoveSecondarySelf => {
                if ab == A::MAGICIAN {
                    self.magician(ctx);
                }
                R::Undef
            }
            Ev::AfterMega => {
                if ab == A::OPPORTUNIST && h.pre == Pre::Any {
                    self.opportunist_copy(me);
                }
                R::Undef
            }
            Ev::AfterMove => {
                if ab == A::OPPORTUNIST && h.pre == Pre::Any {
                    let src = ctx.source;
                    self.opportunist_boost(me, src);
                }
                R::Undef
            }
            Ev::ChangeBoost => {
                match ab {
                    A::RIPEN => {
                        if let Eff::Item(i) = ctx.eff {
                            if i.data().is_berry {
                                let mut b = ctx.boosts;
                                for x in b.v[..b.n as usize].iter_mut() {
                                    x.1 = x.1.saturating_mul(2);
                                }
                                ctx.boosts = b;
                            }
                        }
                    }
                    A::CONTRARY | A::SIMPLE => {
                        let mut b = ctx.boosts;
                        for x in b.v[..b.n as usize].iter_mut() {
                            x.1 = if ab == A::CONTRARY { -x.1 } else { x.1.saturating_mul(2) };
                        }
                        ctx.boosts = b;
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::TryBoost => {
                self.ab_try_boost(ab, h.pre, me, ctx);
                R::Undef
            }
            Ev::AfterEachBoost => {
                if ab == A::DEFIANT || ab == A::COMPETITIVE {
                    let (Some(t), Some(src)) = (ctx.target, ctx.source) else { return R::Undef };
                    if t.0 == src.0 {
                        return R::Undef;
                    }
                    if ctx.boosts.as_slice().iter().any(|x| x.1 < 0) {
                        let st = if ab == A::DEFIANT { 0 } else { 2 };
                        self.boost(&[(st, 2)], t, Some(t), Eff::Ability(ab), false, true);
                    }
                }
                R::Undef
            }
            Ev::AfterBoost => {
                // Opportunist: onFoeAfterBoost records the foe's raised stages
                if ab == A::OPPORTUNIST && h.pre == Pre::Foe {
                    if ctx.eff == Eff::Ability(A::OPPORTUNIST) {
                        return R::Undef;
                    }
                    let mut t = self.s.sides[me.0].opp_boosts.unwrap_or_default();
                    for &(i, v) in ctx.boosts.as_slice() {
                        if v > 0 {
                            let cur = t.get(i as usize).unwrap_or(0);
                            t.set(i as usize, cur.saturating_add(v));
                        }
                    }
                    self.s.sides[me.0].opp_boosts = Some(t);
                    return R::Undef;
                }
                if ab == A::RATTLED {
                    if ctx.eff == Eff::Ability(A::INTIMIDATE) && ctx.boosts.get(0).map_or(false, |v| v != 0) {
                        let t = ctx.target.unwrap();
                        self.ab_boost(&[(4, 1)], t, ctx.source, ab);
                    }
                }
                R::Undef
            }
            Ev::ModifySpe => {
                let m = *self.mon_at(me);
                let w = self.eff_weather(me);
                let x2 = match ab {
                    A::SWIFTSWIM => w == conds::RAINDANCE || w == conds::PRIMORDIALSEA,
                    A::CHLOROPHYLL => w == conds::SUNNYDAY || w == conds::DESOLATELAND,
                    A::SANDRUSH => self.weather() == conds::SANDSTORM,
                    A::SLUSHRUSH => self.weather_is(&[conds::HAIL, conds::SNOWSCAPE]),
                    A::SURGESURFER => self.terrain() == conds::ELECTRICTERRAIN,
                    _ => false,
                };
                if x2 {
                    ctx.chain_modify(2, 1);
                } else if ab == A::QUICKFEET && m.status != Status::None {
                    ctx.chain_modify(3, 2);
                } else if ab == A::SLOWSTART && m.ab_data != 0 {
                    ctx.chain_modify(1, 2);
                }
                R::Undef
            }
            Ev::ModifyCritRatio => {
                let R::Num(cr) = relay else { return R::Undef };
                match ab {
                    A::SUPERLUCK => R::Num(cr + 1),
                    A::MERCILESS => {
                        let t = ctx.source;
                        if t.map_or(false, |t| matches!(self.mon_at(t).status, Status::Psn | Status::Tox)) {
                            R::Num(5)
                        } else {
                            R::Undef
                        }
                    }
                    _ => R::Undef,
                }
            }
            Ev::CriticalHit => match ab {
                A::SHELLARMOR | A::BATTLEARMOR => R::False,
                A::DISGUISE => {
                    let (Some(t), Some(am)) = (ctx.target, ctx.am) else { return R::Undef };
                    if self.disguise_blocks(t, &am) {
                        return R::False;
                    }
                    R::Undef
                }
                _ => R::Undef,
            },
            Ev::ModifyAccuracy => {
                let R::Num(_) = relay else { return R::Undef };
                let am = ctx.am.unwrap();
                match (ab, h.pre) {
                    (A::COMPOUNDEYES, Pre::Source) => ctx.chain_modify(5325, 4096),
                    (A::HUSTLE, Pre::Source) if am.category == Category::Physical => ctx.chain_modify(3277, 4096),
                    (A::VICTORYSTAR, Pre::Any) => {
                        if ctx.source.map_or(false, |s| s.0 == me.0) {
                            ctx.chain_modify(4506, 4096);
                        }
                    }
                    (A::SANDVEIL, Pre::Own) if self.weather() == conds::SANDSTORM => ctx.chain_modify(3277, 4096),
                    (A::SNOWCLOAK, Pre::Own) if self.weather_is(&[conds::HAIL, conds::SNOWSCAPE]) => ctx.chain_modify(3277, 4096),
                    (A::TANGLEDFEET, Pre::Own) => {
                        if ctx.target.map_or(false, |t| self.has_vol(t, conds::CONFUSION)) {
                            ctx.chain_modify(1, 2);
                        }
                    }
                    (A::WONDERSKIN, Pre::Own) if am.category == Category::Status => return R::Num(50),
                    _ => {}
                }
                R::Undef
            }
            Ev::Accuracy => {
                if ab == A::NOGUARD && h.pre == Pre::Any && ctx.am.is_some() {
                    if ctx.source == Some(me) || ctx.target == Some(me) {
                        return R::True;
                    }
                    return relay;
                }
                R::Undef
            }
            Ev::Invulnerability => {
                if ab == A::NOGUARD && h.pre == Pre::Any && ctx.am.is_some() && (ctx.source == Some(me) || ctx.target == Some(me)) {
                    return R::Num(0);
                }
                R::Undef
            }
            Ev::PrepareHit => {
                let Some(mut am) = ctx.am else { return R::Undef };
                let user = ctx.target.unwrap();
                match ab {
                    A::PROTEAN | A::LIBERO => {
                        if self.mon_at(me).ab_flags & abf::USED != 0 {
                            return R::Undef;
                        }
                        if am.has_bounced || am.has_flag(flag::FUTUREMOVE) || am.data().calls_move {
                            return R::Undef;
                        }
                        let ty = am.ty;
                        if ty != Type::Typeless {
                            let types = self.get_types(user, false);
                            if types.as_slice() != [ty] {
                                self.mon_at_mut(user).set_types(&[ty]);
                                self.mon_at_mut(me).ab_flags |= abf::USED;
                                self.types_row(user, Fx::Ability(ab));
                            }
                        }
                    }
                    A::PARENTALBOND => {
                        if am.category == Category::Status || am.multihit.1 > 0 || am.has_flag(flag::NOPARENTALBOND)
                            || am.has_flag(flag::CHARGE) || am.has_flag(flag::FUTUREMOVE)
                        {
                            return R::Undef;
                        }
                        am.multihit = (2, 2);
                        am.parental_bond = true;
                        ctx.am = Some(am);
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::DeductPP => {
                if ab == A::PRESSURE {
                    let (Some(t), Some(src)) = (ctx.target, ctx.source) else { return R::Undef };
                    if t.0 == src.0 {
                        return R::Undef;
                    }
                    return R::Num(1);
                }
                R::Undef
            }
            Ev::SwitchOut => {
                self.ab_switch_out(ab, me);
                R::Undef
            }
            Ev::AfterFaint => {
                // onSourceAfterFaint(length, target, source, effect)
                if h.pre != Pre::Source || ctx.arg >> 8 == 0 {
                    return R::Undef;
                }
                let R::Num(len) = relay else { return R::Undef };
                let len = len as i8;
                let src = me;
                match ab {
                    A::MOXIE | A::CHILLINGNEIGH => {
                        self.boost(&[(0, len)], src, ctx.source, Eff::Ability(ab), false, false);
                    }
                    A::GRIMNEIGH => {
                        self.boost(&[(2, len)], src, ctx.source, Eff::Ability(ab), false, false);
                    }
                    A::BEASTBOOST | A::EELEVATE => {
                        let st = self.best_stat(src, true, true);
                        self.boost(&[(st, len)], src, ctx.source, Eff::Ability(ab), false, false);
                    }
                    A::ASONEGLASTRIER => {
                        self.boost(&[(0, len)], src, Some(src), Eff::Ability(A::CHILLINGNEIGH), false, false);
                    }
                    A::ASONESPECTRIER => {
                        self.boost(&[(2, len)], src, Some(src), Eff::Ability(A::GRIMNEIGH), false, false);
                    }
                    A::BATTLEBOND => {
                        let m = *self.mon_at(src);
                        if m.ab_battle & abb::BOND == 0 && m.species.id() == "greninjabond" && m.hp > 0 && !m.transformed
                            && self.s.sides[1 - src.0].pokemon_left > 0
                        {
                            self.boost(&[(0, 1), (2, 1), (4, 1)], src, Some(src), Eff::Ability(ab), false, false);
                            self.mon_at_mut(src).ab_battle |= abb::BOND;
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::Faint => {
                if ab == A::SOULHEART && h.pre == Pre::Any {
                    self.boost(&[(2, 1)], me, ctx.source, Eff::Ability(ab), false, false);
                }
                R::Undef
            }
            Ev::TrapPokemon => {
                let Some(t) = ctx.target else { return R::Undef };
                match (ab, h.pre) {
                    (A::SHADOWTAG, Pre::Foe) => {
                        if !self.has_ability(t, A::SHADOWTAG) {
                            self.try_trap_hidden(t);
                        }
                    }
                    (A::ARENATRAP, Pre::Foe) => {
                        if self.grounded(t) {
                            self.try_trap_hidden(t);
                        }
                    }
                    (A::MAGNETPULL, Pre::Foe) => {
                        if self.has_type_pk(t, Type::Steel) {
                            self.try_trap_hidden(t);
                        }
                    }
                    (A::RUNAWAY, Pre::Own) => {
                        let m = self.mon_at_mut(t);
                        m.trapped = false;
                        m.trap_hidden = false;
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::DragOut => match ab {
                A::SUCTIONCUPS | A::GUARDDOG => {
                    self.ab_row(me, ab);
                    R::Null
                }
                _ => R::Undef,
            },
            Ev::Flinch => {
                if ab == A::STEADFAST {
                    let t = ctx.target.unwrap();
                    self.ab_boost(&[(4, 1)], t, ctx.source, ab);
                }
                R::Undef
            }
            Ev::EmergencyExit => {
                if ab == A::EMERGENCYEXIT || ab == A::WIMPOUT {
                    let R::Num(orig) = relay else { return R::Undef };
                    let t = ctx.target.unwrap();
                    let m = *self.mon_at(t);
                    let half = m.maxhp as f64 / 2.0;
                    if m.hp == 0 || m.hp as f64 > half || orig as f64 <= half {
                        return R::Undef;
                    }
                    if !self.can_switch(t.0) || m.force_switch_flag || m.switch_flag {
                        return R::Undef;
                    }
                    let mm = self.mon_at_mut(t);
                    mm.switch_flag = true;
                    mm.switch_flag_move = MoveId::NONE;
                    self.ab_row(t, ab);
                }
                R::Undef
            }
            Ev::AfterMoveSecondary => {
                let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
                match ab {
                    A::PICKPOCKET => {
                        if src == t || !am.has_flag(flag::CONTACT) {
                            return R::Undef;
                        }
                        let (mt, ms) = (*self.mon_at(t), *self.mon_at(src));
                        if !mt.item.is_none() || mt.switch_flag || mt.force_switch_flag || (ms.switch_flag && ms.switch_flag_move.is_none()) {
                            return R::Undef;
                        }
                        if let super::items::Taken::Item(it) = self.take_item(src, Some(t)) {
                            if !self.set_item(t, it, Some(src), Eff::Ability(ab)) {
                                self.restore_item(src, it);
                            } else {
                                self.row(Row::Item { p: who(t), item: it, what: ItemWhat::Received, of: Some(who(src)), by: Fx::Ability(ab) });
                            }
                        }
                        R::Undef
                    }
                    A::BERSERK | A::ANGERSHELL => {
                        self.mon_at_mut(me).ab_flags &= !abf::PENDING;
                        if src == t || self.mon_at(t).hp == 0 || am.total_damage == 0 {
                            return R::Undef;
                        }
                        let Some(last) = self.mon_at(t).attacks().last().copied() else { return R::Undef };
                        let dmg = if am.multihit.1 > 0 { am.total_damage } else { last.damage as u32 };
                        let m = self.mon_at(t);
                        let half = m.maxhp as f64 / 2.0;
                        if (m.hp as f64) <= half && (m.hp as u32 + dmg) as f64 > half {
                            if ab == A::BERSERK {
                                self.ab_boost(&[(2, 1)], t, Some(t), ab);
                            } else {
                                self.ab_boost(&[(0, 1), (2, 1), (4, 1), (1, -1), (3, -1)], t, Some(t), ab);
                            }
                        }
                        R::Undef
                    }
                    A::COLORCHANGE => {
                        if self.mon_at(t).hp == 0 {
                            return R::Undef;
                        }
                        if self.mon_at(t).is_active && am.category != Category::Status && am.ty != Type::Typeless && !self.has_type_pk(t, am.ty) {
                            self.mon_at_mut(t).set_types(&[am.ty]);
                            self.types_row(t, Fx::Ability(ab));
                        }
                        R::Undef
                    }
                    _ => R::Undef,
                }
            }
            Ev::ModifySecondaries => R::Undef,
            Ev::Weather => {
                let Some(t) = ctx.target else { return R::Undef };
                let Eff::Cond(w) = ctx.eff else { return R::Undef };
                let bm = self.mon_at(t).base_maxhp as f64;
                match ab {
                    A::DRYSKIN => {
                        if self.eff_weather(t) != w {
                            return R::Undef;
                        }
                        if w == conds::RAINDANCE || w == conds::PRIMORDIALSEA {
                            self.heal(bm / 8.0, t, None, Eff::Ability(ab));
                        } else if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
                            self.damage(bm / 8.0, t, Some(t), Eff::Ability(ab));
                        }
                    }
                    A::SOLARPOWER => {
                        if self.eff_weather(t) == w && (w == conds::SUNNYDAY || w == conds::DESOLATELAND) {
                            self.damage(bm / 8.0, t, Some(t), Eff::Ability(ab));
                        }
                    }
                    A::RAINDISH => {
                        if self.eff_weather(t) == w && (w == conds::RAINDANCE || w == conds::PRIMORDIALSEA) {
                            self.heal(bm / 16.0, t, None, Eff::Ability(ab));
                        }
                    }
                    A::ICEBODY => {
                        if w == conds::HAIL || w == conds::SNOWSCAPE {
                            self.heal(bm / 16.0, t, None, Eff::Ability(ab));
                        }
                    }
                    _ => {}
                }
                R::Undef
            }
            Ev::TryHeal => {
                if ab == A::RIPEN && h.pre == Pre::Own {
                    if let Eff::Item(i) = ctx.eff {
                        if i == items::LEFTOVERS {
                            self.ab_row(me, ab);
                        }
                        if i.data().is_berry {
                            ctx.chain_modify(2, 1);
                        }
                    }
                    return R::Undef;
                }
                if ab == A::LIQUIDOOZE && h.pre == Pre::Source {
                    let ooze = matches!(ctx.eff, Eff::Drain | Eff::Cond(conds::LEECHSEED)) || ctx.eff == Eff::Move(moves::STRENGTHSAP);
                    if ooze {
                        let R::Num(d) = relay else { return R::Undef };
                        if let Some(t) = ctx.target {
                            self.damage(d as f64, t, ctx.source, Eff::Ability(ab));
                        }
                        return R::Num(0);
                    }
                }
                R::Undef
            }
            Ev::HitProtect => {
                if ab == A::PIERCINGDRILL || ab == A::UNSEENFIST {
                    if let Some(am) = ctx.am.as_mut() {
                        if am.has_flag(flag::CONTACT) {
                            am.bypass_protect = true;
                            return R::False;
                        }
                    }
                }
                R::Undef
            }
            Ev::ModifyWeight => {
                let R::Num(w) = relay else { return R::Undef };
                match ab {
                    A::HEAVYMETAL => R::Num(w * 2),
                    A::LIGHTMETAL => R::Num(w / 2),
                    _ => R::Undef,
                }
            }
            Ev::WeatherChange => {
                let t = ctx.target.unwrap_or(me);
                if ab == A::PROTOSYNTHESIS {
                    self.protosynthesis_change(t);
                } else if ab == A::FORECAST {
                    self.forecast_change(t);
                }
                R::Undef
            }
            Ev::TerrainChange => {
                let t = ctx.target.unwrap_or(me);
                if ab == A::QUARKDRIVE {
                    self.quarkdrive_change(t);
                } else if ab == A::MIMICRY {
                    self.mimicry_change(t);
                }
                R::Undef
            }
            Ev::BeforeMove => {
                match ab {
                    A::TRUANT => {
                        let p = ctx.target.unwrap();
                        if self.remove_volatile(p, conds::TRUANT) {
                            self.row(Row::Cant { p: who(p), why: Fx::Ability(ab), mv: MoveId::NONE });
                            return R::False;
                        }
                        self.add_volatile(p, conds::TRUANT, Some(p), Eff::Ability(ab), false);
                        R::Undef
                    }
                    A::GORILLATACTICS => {
                        let am = ctx.am.unwrap();
                        let lock = MoveId(self.mon_at(me).ab_data as u16);
                        if am.id == moves::STRUGGLE {
                            return R::Undef;
                        }
                        if !lock.is_none() && lock != am.id {
                            self.row(Row::Move { p: who(me), mv: am.id, target: None, called_by: Fx::None, priority: am.priority });
                            self.row(Row::Fail { p: Some(who(me)), why: Fx::None });
                            return R::False;
                        }
                        R::Undef
                    }
                    _ => R::Undef,
                }
            }
            Ev::DisableMove => {
                if ab == A::GORILLATACTICS {
                    let lock = MoveId(self.mon_at(me).ab_data as u16);
                    if !lock.is_none() {
                        let m = *self.mon_at(me);
                        for ms in m.moves() {
                            if ms.id != lock {
                                self.disable_move(me, ms.id);
                            }
                        }
                    }
                }
                R::Undef
            }
            Ev::Hit => {
                if ab == A::OWNTEMPO {
                    if ctx.am.map_or(false, |am| am.data().volatile_status == conds::CONFUSION) {
                        let t = ctx.target.unwrap();
                        self.ab_immune(t, ab);
                    }
                    return R::Undef;
                }
                if ab == A::ANGERPOINT {
                    let t = ctx.target.unwrap();
                    if self.mon_at(t).hp == 0 {
                        return R::Undef;
                    }
                    if ctx.am.map_or(false, |am| am.crit) {
                        self.boost(&[(0, 12)], t, Some(t), Eff::Ability(ab), false, false);
                    }
                }
                R::Undef
            }
            Ev::ModifyBoost => {
                // Unaware: onAnyModifyBoost(boosts, pokemon)
                if ab == A::UNAWARE && h.pre == Pre::Any {
                    let Some(p) = ctx.target else { return R::Undef };
                    if p == me {
                        return R::Undef;
                    }
                    let ap = pk_of(self.s.active_pokemon);
                    let at = pk_of(self.s.active_target);
                    let mut b = ctx.boosts;
                    if ap == Some(me) && at == Some(p) {
                        for i in [1, 3, B_EVA] {
                            b.set(i, 0);
                        }
                    }
                    if ap == Some(p) && at == Some(me) {
                        for i in [0, 1, 2, B_ACC] {
                            b.set(i, 0);
                        }
                    }
                    ctx.boosts = b;
                }
                R::Undef
            }
            Ev::TryPrimaryHit => {
                // Aura Break: onAnyTryPrimaryHit(target, source, move)
                if ab == A::AURABREAK && h.pre == Pre::Any {
                    if let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am.as_mut()) {
                        if t != src && am.category != Category::Status {
                            am.aura_break = true;
                        }
                    }
                }
                R::Undef
            }
            Ev::Effectiveness => {
                if ab == A::DISGUISE {
                    let (Some(t), Some(am)) = (ctx.target, ctx.am) else { return R::Undef };
                    if am.category != Category::Status && self.disguise_blocks(t, &am) {
                        return R::Num(0);
                    }
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    fn ab_base_power(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx, relay: R) -> R {
        use abilities as A;
        let (Some(att), Some(def), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
        match pre {
            Pre::Own => match ab {
                A::TOUGHCLAWS if am.has_flag(flag::CONTACT) => ctx.chain_modify(5325, 4096),
                A::SHARPNESS if am.has_flag(flag::SLICING) => ctx.chain_modify(3, 2),
                A::MEGALAUNCHER if am.has_flag(flag::PULSE) => ctx.chain_modify(3, 2),
                A::STRONGJAW if am.has_flag(flag::BITE) => ctx.chain_modify(3, 2),
                A::IRONFIST if am.has_flag(flag::PUNCH) => ctx.chain_modify(4915, 4096),
                A::TECHNICIAN => {
                    // the base power with the modifiers so far
                    let R::Num(bp) = relay else { return R::Undef };
                    if super::calc::modify(bp.max(0) as u32, ctx.modifier) <= 60 {
                        ctx.chain_modify(3, 2);
                    }
                }
                A::AERILATE | A::PIXILATE | A::REFRIGERATE | A::GALVANIZE | A::DRAGONIZE | A::NORMALIZE => {
                    if am.type_changer == ab {
                        ctx.chain_modify(4915, 4096);
                    }
                }
                A::SUPREMEOVERLORD => {
                    let f = self.mon_at(me).ab_data as usize;
                    if f > 0 {
                        let pm = [4096, 4506, 4915, 5325, 5734, 6144];
                        ctx.chain_modify(pm[f.min(5)], 4096);
                    }
                }
                // Electro Shot is hasSheerForceBoost: boosted, its secondary kept
                A::SHEERFORCE if am.sheer_force || am.id == moves::ELECTROSHOT => ctx.chain_modify(5325, 4096),
                A::PUNKROCK if am.has_flag(flag::SOUND) => ctx.chain_modify(5325, 4096),
                A::RECKLESS if am.data().recoil.1 > 0 || am.data().has_crash_damage => ctx.chain_modify(4915, 4096),
                A::SANDFORCE => {
                    if self.weather() == conds::SANDSTORM && matches!(am.ty, Type::Rock | Type::Ground | Type::Steel) {
                        ctx.chain_modify(5325, 4096);
                    }
                }
                A::RIVALRY => {
                    let (ga, gd) = (self.mon_at(att).gender, self.mon_at(def).gender);
                    if ga != 0 && gd != 0 {
                        if ga == gd {
                            ctx.chain_modify(5, 4);
                        } else {
                            ctx.chain_modify(3, 4);
                        }
                    }
                }
                A::ANALYTIC => {
                    let foe = (1 - att.0, self.s.sides[1 - att.0].active as usize);
                    let boosted = !(self.s.sides[foe.0].has_active && self.mon_at(foe).is_active && self.will_move(foe).is_some());
                    if boosted {
                        ctx.chain_modify(5325, 4096);
                    }
                }
                A::FLAREBOOST if self.mon_at(att).status == Status::Brn && am.category == Category::Special => ctx.chain_modify(3, 2),
                A::TOXICBOOST => {
                    if matches!(self.mon_at(att).status, Status::Psn | Status::Tox) && am.category == Category::Physical {
                        ctx.chain_modify(3, 2);
                    }
                }
                _ => {}
            },
            Pre::Source => {
                if ab == A::DRYSKIN && am.ty == Type::Fire {
                    ctx.chain_modify(5, 4);
                }
            }
            Pre::Ally => match ab {
                A::STEELYSPIRIT if am.ty == Type::Steel => ctx.chain_modify(3, 2),
                A::BATTERY if att != me && am.category == Category::Special => ctx.chain_modify(5325, 4096),
                A::POWERSPOT if att != me => ctx.chain_modify(5325, 4096),
                _ => {}
            },
            Pre::Any => {
                if ab == A::FAIRYAURA || ab == A::DARKAURA {
                    let ty = if ab == A::FAIRYAURA { Type::Fairy } else { Type::Dark };
                    if def == att || am.category == Category::Status || am.ty != ty {
                        return R::Undef;
                    }
                    // auraBooster: the first aura holder in the handler order boosts
                    let mut a2 = am;
                    let booster = pk_of(a2.aura_booster);
                    if booster.map_or(true, |b| !self.has_ability(b, ab)) {
                        a2.aura_booster = pk_code(me);
                        ctx.am = Some(a2);
                    }
                    if a2.aura_booster != pk_code(me) {
                        return R::Undef;
                    }
                    ctx.chain_modify(if a2.aura_break { 3072 } else { 5448 }, 4096);
                }
            }
            Pre::Foe => {}
        }
        R::Undef
    }

    fn ab_modify_offense(&mut self, e: Ev, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx, relay: R) -> R {
        use abilities as A;
        let spa = e == Ev::ModifySpA;
        let (Some(att), def, Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
        match pre {
            Pre::Own => {
                let pinch = |t: Type| am.ty == t && self.hp_third(att);
                match ab {
                    A::TORRENT if pinch(Type::Water) => ctx.chain_modify(3, 2),
                    A::BLAZE if pinch(Type::Fire) => ctx.chain_modify(3, 2),
                    A::OVERGROW if pinch(Type::Grass) => ctx.chain_modify(3, 2),
                    A::SWARM if pinch(Type::Bug) => ctx.chain_modify(3, 2),
                    A::HUGEPOWER | A::PUREPOWER if !spa => ctx.chain_modify(2, 1),
                    A::FIREMANE if am.ty == Type::Fire => ctx.chain_modify(3, 2),
                    A::WATERBUBBLE if am.ty == Type::Water => ctx.chain_modify(2, 1),
                    A::GUTS if !spa && self.mon_at(att).status != Status::None => ctx.chain_modify(3, 2),
                    A::DEFEATIST => {
                        let m = self.mon_at(att);
                        if (m.hp as f64) <= m.maxhp as f64 / 2.0 {
                            ctx.chain_modify(1, 2);
                        }
                    }
                    A::STAKEOUT => {
                        if def.map_or(false, |d| self.mon_at(d).active_turns == 0) {
                            ctx.chain_modify(2, 1);
                        }
                    }
                    A::STEELWORKER if am.ty == Type::Steel => ctx.chain_modify(3, 2),
                    A::TRANSISTOR if am.ty == Type::Electric => ctx.chain_modify(5325, 4096),
                    A::DRAGONSMAW if am.ty == Type::Dragon => ctx.chain_modify(3, 2),
                    A::ROCKYPAYLOAD if am.ty == Type::Rock => ctx.chain_modify(3, 2),
                    A::SOLARPOWER if spa => {
                        let w = self.eff_weather(att);
                        if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
                            ctx.chain_modify(3, 2);
                        }
                    }
                    A::ORICHALCUMPULSE if !spa => {
                        let w = self.eff_weather(att);
                        if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
                            ctx.chain_modify(5461, 4096);
                        }
                    }
                    A::HADRONENGINE if spa && self.terrain() == conds::ELECTRICTERRAIN => ctx.chain_modify(5461, 4096),
                    A::HUSTLE if !spa => {
                        if let R::Num(x) = relay {
                            return R::Num(super::calc::modify(x.max(0) as u32, 6144) as i32);
                        }
                    }
                    A::GORILLATACTICS if !spa => ctx.chain_modify(3, 2),
                    A::SLOWSTART if !spa && self.mon_at(me).ab_data != 0 => ctx.chain_modify(1, 2),
                    // Plus and Minus need an ally
                    _ => {}
                }
            }
            Pre::Source => {
                let t = am.ty;
                match ab {
                    A::THICKFAT if t == Type::Ice || t == Type::Fire => ctx.chain_modify(1, 2),
                    A::WATERBUBBLE | A::HEATPROOF if t == Type::Fire => ctx.chain_modify(1, 2),
                    A::PURIFYINGSALT if t == Type::Ghost => ctx.chain_modify(1, 2),
                    _ => {}
                }
            }
            Pre::Any => {
                // Tablets of Ruin, Vessel of Ruin: onAnyModifyAtk(atk, source, target, move)
                let ruin = if spa { A::VESSELOFRUIN } else { A::TABLETSOFRUIN };
                if ab == ruin {
                    if self.has_ability(att, ruin) {
                        return R::Undef;
                    }
                    let mut a2 = am;
                    let slot = if spa { &mut a2.ruined[2] } else { &mut a2.ruined[0] };
                    if *slot == NO_MON {
                        *slot = pk_code(me);
                    }
                    let holder = *slot;
                    ctx.am = Some(a2);
                    if holder != pk_code(me) {
                        return R::Undef;
                    }
                    ctx.chain_modify(3, 4);
                }
            }
            _ => {}
        }
        R::Undef
    }

    fn ab_modify_defense(&mut self, e: Ev, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let spd = e == Ev::ModifySpD;
        let Some(defender) = ctx.target else { return R::Undef };
        match pre {
            Pre::Own => match ab {
                A::FURCOAT if !spd => ctx.chain_modify(2, 1),
                A::MARVELSCALE if !spd && self.mon_at(defender).status != Status::None => ctx.chain_modify(3, 2),
                A::GRASSPELT if !spd && self.terrain() == conds::GRASSYTERRAIN => ctx.chain_modify(3, 2),
                _ => {}
            },
            Pre::Any => {
                let ruin = if spd { A::BEADSOFRUIN } else { A::SWORDOFRUIN };
                if ab == ruin {
                    if self.has_ability(defender, ruin) {
                        return R::Undef;
                    }
                    let Some(mut a2) = ctx.am else { return R::Undef };
                    let i = if spd { 3 } else { 1 };
                    if pk_of(a2.ruined[i]).map_or(true, |b| !self.has_ability(b, ruin)) {
                        a2.ruined[i] = pk_code(me);
                    }
                    ctx.am = Some(a2);
                    if a2.ruined[i] != pk_code(me) {
                        return R::Undef;
                    }
                    ctx.chain_modify(3, 4);
                }
            }
            _ => {}
        }
        R::Undef
    }

    fn ab_modify_damage(&mut self, ab: AbilityId, pre: Pre, _me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let (Some(_att), Some(def), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
        match pre {
            Pre::Own => match ab {
                A::SNIPER if am.crit => ctx.chain_modify(3, 2),
                A::TINTEDLENS if am.type_mod < 0 => ctx.chain_modify(2, 1),
                A::NEUROFORCE if am.type_mod > 0 => ctx.chain_modify(5120, 4096),
                _ => {}
            },
            Pre::Source => match ab {
                A::MULTISCALE | A::SHADOWSHIELD => {
                    let m = self.mon_at(def);
                    if m.hp >= m.maxhp {
                        ctx.chain_modify(1, 2);
                    }
                }
                A::FILTER | A::SOLIDROCK | A::PRISMARMOR if am.type_mod > 0 => ctx.chain_modify(3, 4),
                A::ICESCALES if am.category == Category::Special => ctx.chain_modify(1, 2),
                A::PUNKROCK if am.has_flag(flag::SOUND) => ctx.chain_modify(1, 2),
                A::AURAGUARD if am.has_flag(flag::CONTACT) => ctx.chain_modify(1, 2),
                A::RIPEN => {
                    let m = self.mon_at_mut(def);
                    if m.ab_flags & abf::BERRY_WEAKEN != 0 {
                        m.ab_flags &= !abf::BERRY_WEAKEN;
                        ctx.chain_modify(1, 2);
                    }
                }
                A::FLUFFY => {
                    // chainModify(mod) with mod 1, 2, 0.5 or 1
                    let fire = am.ty == Type::Fire;
                    let contact = am.has_flag(flag::CONTACT);
                    match (fire, contact) {
                        (true, false) => ctx.chain_modify(2, 1),
                        (false, true) => ctx.chain_modify(1, 2),
                        _ => ctx.chain_modify(1, 1),
                    }
                }
                _ => {}
            },
            _ => {}
        }
        R::Undef
    }

    fn ab_modify_move(&mut self, ab: AbilityId, me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let Some(am) = ctx.am.as_mut() else { return R::Undef };
        match ab {
            A::MOLDBREAKER | A::TERAVOLT | A::TURBOBLAZE => am.ignore_ability = true,
            A::MYCELIUMMIGHT if am.category == Category::Status => am.ignore_ability = true,
            A::SCRAPPY | A::MINDSEYE => {
                if !am.ignore_immunity {
                    am.scrappy = true;
                }
                if ab == A::MINDSEYE {
                    am.ignore_evasion = true;
                }
            }
            A::KEENEYE | A::ILLUMINATE => am.ignore_evasion = true,
            A::SKILLLINK => {
                if am.multihit.1 > 0 && am.multihit.0 != am.multihit.1 {
                    am.multihit = (am.multihit.1, am.multihit.1);
                }
                am.multiaccuracy = false;
            }
            A::SERENEGRACE => am.sec_double = true,
            A::SHEERFORCE => {
                if (!am.data().secondaries.is_empty() || am.nextra > 0) && am.id != moves::ELECTROSHOT {
                    am.sheer_force = true;
                }
            }
            A::INFILTRATOR => am.infiltrates = true,
            A::LONGREACH => am.flags &= !flag::CONTACT,
            A::UNSEENFIST => {
                // Champions: onModifyMove removed; the HitProtect handler does the work
            }
            A::STENCH => {
                if am.category != Category::Status && !am.has_flinch_secondary() {
                    am.push_secondary(super::moves::ExtraSec { chance: Some(10), status: Status::None, vol: conds::FLINCH });
                }
            }
            A::GORILLATACTICS => {
                if self.mon_at(me).ab_data == 0 && am.id != moves::STRUGGLE {
                    self.mon_at_mut(me).ab_data = am.id.0 as i16;
                }
            }
            A::STANCECHANGE => {
                let m = *self.mon_at(me);
                let base_ok = m.species.data().base_species == SpeciesId::from_id("aegislash").unwrap_or_default();
                if !base_ok || m.transformed {
                    return R::Undef;
                }
                if am.category == Category::Status && am.id != moves::KINGSSHIELD {
                    return R::Undef;
                }
                let to = if am.id == moves::KINGSSHIELD { "aegislash" } else { "aegislashblade" };
                if m.species.id() != to {
                    self.forme_change(me, to, false, ab);
                }
            }
            _ => {}
        }
        R::Undef
    }

    fn ab_try_hit(&mut self, ab: AbilityId, me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
        let other = t != src;
        let bm = self.mon_at(t).base_maxhp as f64;
        let eff = Eff::Ability(ab);
        match ab {
            A::VOLTABSORB | A::WATERABSORB | A::DRYSKIN | A::EARTHEATER => {
                let ty = match ab {
                    A::VOLTABSORB => Type::Electric,
                    A::EARTHEATER => Type::Ground,
                    _ => Type::Water,
                };
                if other && am.ty == ty {
                    if !self.heal(bm / 4.0, t, Some(src), eff).truthy() {
                        self.ab_immune(t, ab);
                    }
                    return R::Null;
                }
                R::Undef
            }
            A::FLASHFIRE => {
                if other && am.ty == Type::Fire {
                    if !self.add_volatile(t, conds::FLASHFIRE, Some(src), eff, false).truthy() {
                        self.ab_immune(t, ab);
                    }
                    return R::Null;
                }
                R::Undef
            }
            A::SAPSIPPER | A::LIGHTNINGROD | A::STORMDRAIN | A::MOTORDRIVE | A::WELLBAKEDBODY => {
                let (ty, b) = match ab {
                    A::SAPSIPPER => (Type::Grass, (0, 1)),
                    A::LIGHTNINGROD => (Type::Electric, (2, 1)),
                    A::STORMDRAIN => (Type::Water, (2, 1)),
                    A::MOTORDRIVE => (Type::Electric, (4, 1)),
                    _ => (Type::Fire, (1, 2)),
                };
                if other && am.ty == ty {
                    if !self.boost(&[b], t, Some(src), eff, false, false).truthy() {
                        self.ab_immune(t, ab);
                    }
                    return R::Null;
                }
                R::Undef
            }
            A::WINDRIDER => {
                if other && am.has_flag(flag::WIND) {
                    if !self.boost(&[(0, 1)], t, Some(t), eff, false, false).truthy() {
                        self.ab_immune(t, ab);
                    }
                    return R::Null;
                }
                R::Undef
            }
            A::GOODASGOLD if am.category == Category::Status && other => {
                self.ab_immune(t, ab);
                R::Null
            }
            A::BULLETPROOF if am.has_flag(flag::BULLET) => {
                self.ab_immune(t, ab);
                R::Null
            }
            A::SOUNDPROOF if other && am.has_flag(flag::SOUND) => {
                self.ab_immune(t, ab);
                R::Null
            }
            A::OVERCOAT => {
                if am.has_flag(flag::POWDER) && other && self.type_immunity(t, Imm::Powder) {
                    self.ab_immune(t, ab);
                    return R::Null;
                }
                R::Undef
            }
            A::STURDY if am.data().ohko != Ohko::No => {
                self.ab_immune(t, ab);
                R::Null
            }
            A::OBLIVIOUS if am.id == moves::ATTRACT || am.id == moves::TAUNT => {
                self.ab_immune(t, ab);
                R::Null
            }
            A::TELEPATHY => R::Undef,
            A::MAGICBOUNCE => {
                if !other || am.has_bounced || !am.has_flag(flag::REFLECTABLE) || self.semi_invulnerable(t) {
                    return R::Undef;
                }
                self.bounce_next = true;
                self.use_move(t, am.id, super::moves::Tgt::Some(src), MoveId::NONE, false, MoveId::NONE);
                R::Null
            }
            A::WONDERGUARD => {
                if !other || am.category == Category::Status || am.id == moves::STRUGGLE {
                    return R::Undef;
                }
                if self.run_effectiveness(t, &am) <= 0 || !self.run_immunity(t, &am) {
                    self.ab_immune(t, ab);
                    return R::Null;
                }
                R::Undef
            }
            _ => {
                let _ = me;
                R::Undef
            }
        }
    }

    fn ab_damaging_hit(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let (Some(t), Some(src), Some(am)) = (ctx.target, ctx.source, ctx.am) else { return R::Undef };
        let eff = Eff::Ability(ab);
        let contact = self.makes_contact(&am);
        if pre == Pre::Source {
            // onSourceDamagingHit(damage, target, source, move): the attacker's ability
            match ab {
                A::POISONTOUCH => {
                    if self.has_ability(t, A::SHIELDDUST) {
                        return R::Undef;
                    }
                    if contact {
                        let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Proc, Some(me.0 as u8)) };
                        if self.chance(site, 3, 10) {
                            self.try_set_status(t, Status::Psn, Some(src), eff);
                        }
                    }
                }
                A::TOXICCHAIN => {
                    if self.has_ability(t, A::SHIELDDUST) {
                        return R::Undef;
                    }
                    let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Proc, Some(me.0 as u8)) };
                    if self.chance(site, 3, 10) {
                        self.try_set_status(t, Status::Tox, Some(src), eff);
                    }
                }
                _ => {}
            }
            return R::Undef;
        }
        if pre != Pre::Own {
            return R::Undef;
        }
        let proc = |s: &mut Self| {
            let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Proc, Some(me.0 as u8)) };
            s.chance(site, 3, 10)
        };
        match ab {
            A::ROUGHSKIN | A::IRONBARBS => {
                if contact {
                    let a = self.mon_at(src).base_maxhp as f64 / 8.0;
                    self.damage(a, src, Some(t), eff);
                }
            }
            A::STATIC | A::FLAMEBODY | A::POISONPOINT => {
                if contact && proc(self) {
                    let st = match ab {
                        A::STATIC => Status::Par,
                        A::FLAMEBODY => Status::Brn,
                        _ => Status::Psn,
                    };
                    self.try_set_status(src, st, Some(t), eff);
                }
            }
            A::CUTECHARM => {
                if contact && proc(self) {
                    self.add_volatile(src, conds::ATTRACT, Some(t), eff, false);
                }
            }
            A::EFFECTSPORE => {
                if contact && self.run_status_immunity(src, Imm::Powder) {
                    let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Pick, Some(me.0 as u8)) };
                    // random(100) in buckets: 11 sleep, 10 paralysis, 9 poison, 70 nothing
                    let v = self.o.sample_str(&site, &EFFECT_SPORE);
                    let st = match EFFECT_SPORE[v] {
                        "slp" => Status::Slp,
                        "par" => Status::Par,
                        "psn" => Status::Psn,
                        _ => Status::None,
                    };
                    if st != Status::None {
                        self.try_set_status(src, st, Some(t), eff);
                    }
                }
            }
            A::GOOEY | A::TANGLINGHAIR => {
                if contact {
                    self.ab_row(t, ab);
                    self.boost(&[(4, -1)], src, Some(t), eff, true, false);
                }
            }
            A::STAMINA => {
                self.boost(&[(1, 1)], t, Some(src), eff, false, false);
            }
            A::WEAKARMOR => {
                if am.category == Category::Physical {
                    self.boost(&[(1, -1), (4, 2)], t, Some(t), eff, false, false);
                }
            }
            A::JUSTIFIED if am.ty == Type::Dark => {
                self.boost(&[(0, 1)], t, Some(src), eff, false, false);
            }
            A::RATTLED if matches!(am.ty, Type::Dark | Type::Bug | Type::Ghost) => {
                self.boost(&[(4, 1)], t, Some(src), eff, false, false);
            }
            A::WATERCOMPACTION if am.ty == Type::Water => {
                self.boost(&[(1, 2)], t, Some(src), eff, false, false);
            }
            A::STEAMENGINE if am.ty == Type::Water || am.ty == Type::Fire => {
                self.boost(&[(4, 6)], t, Some(src), eff, false, false);
            }
            A::THERMALEXCHANGE if am.ty == Type::Fire => {
                self.boost(&[(0, 1)], t, Some(src), eff, false, false);
            }
            A::ELECTROMORPHOSIS => {
                self.add_volatile(t, conds::CHARGE, Some(src), eff, false);
            }
            A::WINDPOWER => {
                if am.has_flag(flag::WIND) {
                    self.add_volatile(t, conds::CHARGE, Some(src), eff, false);
                }
            }
            A::CURSEDBODY => {
                if self.has_vol(src, conds::DISABLE) {
                    return R::Undef;
                }
                if !am.has_flag(flag::FUTUREMOVE) && am.id != moves::STRUGGLE && proc(self) {
                    self.add_volatile(src, conds::DISABLE, Some(t), eff, false);
                }
            }
            A::TOXICDEBRIS => {
                let side = if src.0 == t.0 { 1 - src.0 } else { src.0 };
                let layers = self.side_cond(side, conds::TOXICSPIKES).map_or(0, |x| x.layers);
                if am.category == Category::Physical && layers < 2 {
                    self.ab_row(t, ab);
                    self.add_side_cond(side, conds::TOXICSPIKES, t, eff);
                }
            }
            A::INNARDSOUT => {
                if self.mon_at(t).hp == 0 {
                    let d = ctx.arg as u32 + am.total_damage;
                    self.damage(d as f64, src, Some(t), eff);
                }
            }
            A::AFTERMATH => {
                if self.mon_at(t).hp == 0 && contact {
                    let a = self.mon_at(src).base_maxhp as f64 / 4.0;
                    self.damage(a, src, Some(t), eff);
                }
            }
            A::PERISHBODY => {
                if !contact || self.has_vol(src, conds::PERISHSONG) {
                    return R::Undef;
                }
                self.ab_row(t, ab);
                self.add_volatile(src, conds::PERISHSONG, Some(t), eff, false);
                self.add_volatile(t, conds::PERISHSONG, Some(t), eff, false);
            }
            A::MUMMY | A::LINGERINGAROMA => {
                let sa = self.mon_at(src).ability;
                if flags(sa) & aflag::CANTSUPPRESS != 0 || sa == ab {
                    return R::Undef;
                }
                if contact {
                    self.set_ability_by(src, ab, Some(t), Fx::Ability(ab));
                }
            }
            A::WANDERINGSPIRIT => {
                if contact {
                    self.skill_swap(src, t);
                }
            }
            A::COTTONDOWN => {
                let mut shown = false;
                for side in 0..SIDES {
                    let sd = &self.s.sides[side];
                    if !sd.has_active {
                        continue;
                    }
                    let p = (side, sd.active as usize);
                    if p == t || self.mon_at(p).fainted {
                        continue;
                    }
                    if !shown {
                        self.ab_row(t, ab);
                        shown = true;
                    }
                    self.boost(&[(4, -1)], p, Some(t), eff, true, false);
                }
            }
            A::SANDSPIT => {
                self.set_weather_by(conds::SANDSTORM, t, eff);
            }
            A::SEEDSOWER => {
                self.set_terrain_by(conds::GRASSYTERRAIN, t, eff);
            }
            A::SPICYSPRAY => {
                self.try_set_status(src, Status::Brn, Some(t), eff);
            }
            A::ILLUSION => {
                if self.mon_at(t).illusion {
                    self.ability_end(t, ab);
                }
            }
            _ => {}
        }
        R::Undef
    }

    fn ab_damage(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx, relay: R) -> R {
        use abilities as A;
        let Some(t) = ctx.target else { return R::Undef };
        if pre == Pre::Any {
            if ab == A::DAMP && ctx.eff == Eff::Ability(A::AFTERMATH) {
                return R::False;
            }
            return R::Undef;
        }
        if pre != Pre::Own {
            return R::Undef;
        }
        let is_move = ctx.eff.is_move();
        match ab {
            A::DISGUISE => {
                if is_move && self.mon_at(t).species.id() == "mimikyu" {
                    self.ab_row(t, ab);
                    self.mon_at_mut(me).ab_flags |= abf::BUSTED;
                    return R::Num(0);
                }
                R::Undef
            }
            A::MAGICGUARD if !is_move => {
                // an ability's damage shows that ability on its holder
                if let (Eff::Ability(a), Some(src)) = (ctx.eff, ctx.source) {
                    self.ab_row(src, a);
                }
                R::False
            }
            A::ROCKHEAD => {
                if ctx.eff == Eff::Recoil && self.active.map_or(true, |a| a.id != moves::STRUGGLE) {
                    return R::Null;
                }
                R::Undef
            }
            A::STURDY => {
                let m = self.mon_at(t);
                if let R::Num(d) = relay {
                    if m.hp == m.maxhp && d >= m.hp as i32 && is_move {
                        let hp = m.hp as i32;
                        self.ab_row(t, ab);
                        return R::Num(hp - 1);
                    }
                }
                R::Undef
            }
            A::POISONHEAL => {
                if ctx.eff == Eff::Cond(conds::PSN) || ctx.eff == Eff::Cond(conds::TOX) {
                    let a = self.mon_at(t).base_maxhp as f64 / 8.0;
                    self.heal(a, t, ctx.source, Eff::Ability(ab));
                    return R::False;
                }
                R::Undef
            }
            A::HEATPROOF => {
                if ctx.eff == Eff::Cond(conds::BRN) {
                    if let R::Num(d) = relay {
                        // damage / 2: the event's number may be fractional; spreadDamage clamps to at least 1
                        return R::Num(((d as f64) / 2.0).max(1.0) as i32 + if d % 2 == 1 && d > 1 { 0 } else { 0 });
                    }
                }
                R::Undef
            }
            A::BERSERK | A::ANGERSHELL => {
                // Champions: no suppressingSecondaries term
                // confusion's self-hit is a move without multihit
                let multi = matches!(ctx.eff, Eff::Move(_)) && self.active.map_or(false, |a| a.multihit.1 > 0);
                let m = self.mon_at_mut(me);
                if is_move && !multi {
                    m.ab_flags |= abf::PENDING;
                } else {
                    m.ab_flags &= !abf::PENDING;
                }
                R::Undef
            }
            A::GLUTTONY => R::Undef,
            A::ICEFACE => {
                // onDamage (priority 1): a physical move breaks the ice instead
                let phys = self.active.map_or(false, |a| a.category == Category::Physical);
                if is_move && phys && self.mon_at(t).species.id() == "eiscue" {
                    self.ab_row(t, ab);
                    self.mon_at_mut(me).ab_flags |= abf::BUSTED;
                    return R::Num(0);
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    fn ab_set_status(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) -> R {
        let r = self.ab_set_status_inner(ab, pre, me, ctx);
        if (r == R::False || r == R::Null) && self.logging() {
            let t = ctx.target.unwrap();
            let own_status = super::status::effect_status(ctx.eff);
            // Flower Veil shows only for a move without secondaries (Synchronize's effect has no name, so not for
            // it); Pastel Veil only for a status move
            let flower_shown = matches!(ctx.eff, Eff::Move(m) if m.data().secondaries.is_empty());
            let block = match ab {
                abilities::SWEETVEIL => Some(true),
                abilities::FLOWERVEIL => Some(flower_shown),
                abilities::PASTELVEIL if pre == Pre::Ally => Some(own_status),
                _ => None,
            };
            if let Some(shown) = block {
                if shown {
                    self.row(Row::Blocked { p: Some(who(t)), by: Fx::Ability(ab), what: Fx::Status(status_of(ctx.arg)) });
                }
            } else if own_status {
                self.ab_immune(t, ab);
            }
        }
        r
    }

    fn ab_set_status_inner(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let st = status_of(ctx.arg);
        let Some(t) = ctx.target else { return R::Undef };
        match pre {
            Pre::Own => match ab {
                A::LIMBER if st == Status::Par => R::False,
                A::INSOMNIA | A::VITALSPIRIT if st == Status::Slp => R::False,
                A::IMMUNITY if st == Status::Psn || st == Status::Tox => R::False,
                A::WATERVEIL | A::WATERBUBBLE | A::THERMALEXCHANGE if st == Status::Brn => R::False,
                A::LEAFGUARD if self.eff_weather_is(t, &[conds::SUNNYDAY, conds::DESOLATELAND]) => R::False,
                A::PURIFYINGSALT | A::COMATOSE => R::False,
                A::SHIELDSDOWN if self.mon_at(t).species.id() == "miniormeteor" && !self.mon_at(t).transformed => R::False,
                A::PASTELVEIL if st == Status::Psn || st == Status::Tox => R::False,
                A::SWEETVEIL if st == Status::Slp => R::Null,
                A::FLOWERVEIL => self.flower_veil_status(t, ctx),
                _ => R::Undef,
            },
            Pre::Ally => match ab {
                A::PASTELVEIL if st == Status::Psn || st == Status::Tox => R::False,
                A::SWEETVEIL if st == Status::Slp => R::Null,
                A::FLOWERVEIL => self.flower_veil_status(t, ctx),
                _ => R::Undef,
            },
            _ => {
                let _ = me;
                R::Undef
            }
        }
    }

    fn flower_veil_status(&mut self, t: Pk, ctx: &Ctx) -> R {
        let Some(src) = ctx.source else { return R::Undef };
        if self.has_type_pk(t, Type::Grass) && t != src && ctx.eff != Eff::None && !matches!(ctx.eff, Eff::Cond(c) if c == conds::YAWN)
            && ctx.eff != Eff::Move(moves::YAWN)
        {
            return R::Null;
        }
        R::Undef
    }

    fn ab_try_add_volatile(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) -> R {
        let r = self.ab_try_add_volatile_inner(ab, pre, me, ctx);
        if r == R::Null && self.logging() {
            let t = ctx.target.unwrap();
            let c = Cond(ctx.arg);
            match ab {
                abilities::INNERFOCUS | abilities::OWNTEMPO => {}
                abilities::SWEETVEIL | abilities::FLOWERVEIL => {
                    self.row(Row::Blocked { p: Some(who(t)), by: Fx::Ability(ab), what: Fx::Cond(c) });
                }
                abilities::AROMAVEIL => {
                    if ctx.eff.is_move() {
                        self.row(Row::Blocked { p: Some(who(t)), by: Fx::Ability(ab), what: Fx::Cond(c) });
                    }
                }
                _ => self.ab_immune(t, ab),
            }
        }
        r
    }

    fn ab_try_add_volatile_inner(&mut self, ab: AbilityId, pre: Pre, _me: Pk, ctx: &mut Ctx) -> R {
        use abilities as A;
        let c = Cond(ctx.arg);
        let Some(t) = ctx.target else { return R::Undef };
        let own = pre == Pre::Own;
        let ally = pre == Pre::Own || pre == Pre::Ally;
        match ab {
            A::INNERFOCUS if own && c == conds::FLINCH => R::Null,
            A::SHIELDSDOWN if own && c == conds::YAWN && self.mon_at(t).species.id() == "miniormeteor" && !self.mon_at(t).transformed => R::Null,
            A::OWNTEMPO if own && c == conds::CONFUSION => R::Null,
            A::LEAFGUARD if own && c == conds::YAWN && self.eff_weather_is(t, &[conds::SUNNYDAY, conds::DESOLATELAND]) => R::Null,
            A::PURIFYINGSALT | A::INSOMNIA | A::VITALSPIRIT if own && c == conds::YAWN => R::Null,
            A::SWEETVEIL if ally && c == conds::YAWN => R::Null,
            A::FLOWERVEIL if ally && c == conds::YAWN && self.has_type_pk(t, Type::Grass) => R::Null,
            A::AROMAVEIL if ally => {
                if [conds::ATTRACT, conds::DISABLE, conds::ENCORE, conds::HEALBLOCK, conds::TAUNT, conds::TORMENT].contains(&c) {
                    return R::Null;
                }
                R::Undef
            }
            _ => R::Undef,
        }
    }

    fn ab_try_boost(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) {
        let before = ctx.boosts;
        self.ab_try_boost_inner(ab, pre, me, ctx);
        if !self.logging() || ab == abilities::MIRRORARMOR || ab == abilities::GUARDDOG {
            return;
        }
        // a stage the ability removed from the table: "X's stats were not lowered!"
        if ctx.boosts.n < before.n {
            let t = ctx.target.unwrap();
            let intimidate = ctx.eff == Eff::Ability(abilities::INTIMIDATE);
            let by_move_sec = matches!(ctx.eff, Eff::Move(m) if !m.data().secondaries.is_empty()) || ctx.eff == Eff::Move(moves::OCTOLOCK);
            let shown = match ab {
                abilities::INNERFOCUS | abilities::OBLIVIOUS | abilities::OWNTEMPO | abilities::SCRAPPY => intimidate,
                _ => !by_move_sec,
            };
            if shown {
                self.row(Row::Blocked { p: Some(who(t)), by: Fx::Ability(ab), what: Fx::None });
            }
        }
    }

    fn ab_try_boost_inner(&mut self, ab: AbilityId, pre: Pre, me: Pk, ctx: &mut Ctx) {
        use abilities as A;
        let t = ctx.target.unwrap();
        let src = ctx.source;
        let self_inflicted = src == Some(t);
        let intimidate = ctx.eff == Eff::Ability(A::INTIMIDATE);
        let mut b = ctx.boosts;
        if pre == Pre::Ally {
            if ab == A::FLOWERVEIL && !self_inflicted && self.has_type_pk(t, Type::Grass) {
                for i in 0..7 {
                    if b.get(i).map_or(false, |v| v < 0) {
                        b.delete(i);
                    }
                }
                ctx.boosts = b;
            }
            return;
        }
        if pre != Pre::Own {
            return;
        }
        let drop_neg = |b: &mut Boosts, i: usize| {
            if b.get(i).map_or(false, |v| v < 0) {
                b.delete(i);
            }
        };
        match ab {
            A::CLEARBODY | A::WHITESMOKE | A::FULLMETALBODY => {
                if !self_inflicted || src.is_none() {
                    if src.is_some() || true {
                        for i in 0..7 {
                            drop_neg(&mut b, i);
                        }
                    }
                }
            }
            A::FLOWERVEIL => {
                if !self_inflicted && self.has_type_pk(t, Type::Grass) {
                    for i in 0..7 {
                        drop_neg(&mut b, i);
                    }
                }
            }
            A::HYPERCUTTER if !self_inflicted => drop_neg(&mut b, 0),
            A::BIGPECKS if !self_inflicted => drop_neg(&mut b, 1),
            A::KEENEYE | A::ILLUMINATE | A::MINDSEYE if !self_inflicted => drop_neg(&mut b, B_ACC),
            A::INNERFOCUS | A::OBLIVIOUS | A::OWNTEMPO | A::SCRAPPY => {
                if intimidate && b.get(0).map_or(false, |v| v != 0) {
                    b.delete(0);
                }
            }
            A::GUARDDOG => {
                if intimidate && b.get(0).map_or(false, |v| v != 0) {
                    b.delete(0);
                    ctx.boosts = b;
                    self.boost(&[(0, 1)], t, Some(t), Eff::Ability(ab), false, true);
                    return;
                }
            }
            A::MIRRORARMOR => {
                let Some(s) = src else { return };
                if s == t || ctx.eff == Eff::Ability(A::MIRRORARMOR) {
                    return;
                }
                let list: Vec<(u8, i8)> = b.as_slice().to_vec();
                for (i, v) in list {
                    if v < 0 {
                        if self.mon_at(t).boosts[i as usize] == -6 {
                            continue;
                        }
                        b.delete(i as usize);
                        ctx.boosts = b;
                        if self.mon_at(s).hp > 0 {
                            self.ab_row(t, ab);
                            self.boost(&[(i as usize, v)], s, Some(t), Eff::Ability(ab), true, false);
                        }
                    }
                }
            }
            _ => {
                let _ = me;
            }
        }
        ctx.boosts = b;
    }

    // ------------------------------------------------------------------------------------------ single events

    /// The ability's onUpdate, run from the Update event.
    fn ab_update(&mut self, ab: AbilityId, me: Pk) {
        use abilities as A;
        let st = self.mon_at(me).status;
        let cure = match ab {
            A::LIMBER => st == Status::Par,
            A::INSOMNIA | A::VITALSPIRIT => st == Status::Slp,
            A::IMMUNITY | A::PASTELVEIL => st == Status::Psn || st == Status::Tox,
            A::WATERVEIL | A::WATERBUBBLE | A::THERMALEXCHANGE => st == Status::Brn,
            A::MAGMAARMOR => st == Status::Frz,
            _ => false,
        };
        if cure {
            self.ab_row(me, ab);
            self.cure_status(me);
            return;
        }
        match ab {
            A::OBLIVIOUS => {
                if self.has_vol(me, conds::ATTRACT) {
                    self.ab_row(me, ab);
                    self.remove_volatile(me, conds::ATTRACT);
                    self.row(Row::VolatileEnd { p: who(me), c: conds::ATTRACT, by: Fx::Ability(ab) });
                }
                if self.has_vol(me, conds::TAUNT) {
                    self.ab_row(me, ab);
                    self.remove_volatile(me, conds::TAUNT);
                }
            }
            A::OWNTEMPO => {
                if self.has_vol(me, conds::CONFUSION) {
                    self.ab_row(me, ab);
                    self.remove_volatile(me, conds::CONFUSION);
                }
            }
            A::TRACE => self.trace_update(me),
            A::ICEFACE => {
                if self.mon_at(me).species.id() == "eiscue" && self.mon_at(me).ab_flags & abf::BUSTED != 0 {
                    self.forme_change(me, "eiscuenoice", true, ab);
                }
            }
            A::DISGUISE => {
                if self.mon_at(me).species.id() == "mimikyu" && self.mon_at(me).ab_flags & abf::BUSTED != 0 {
                    self.forme_change(me, "mimikyubusted", true, ab);
                    let a = self.mon_at(me).base_maxhp as f64 / 8.0;
                    // the effect is the species: not a move
                    self.damage(a, me, Some(me), Eff::None);
                }
            }
            _ => {}
        }
    }

    /// Trace's onUpdate: copy the foe's ability
    fn trace_update(&mut self, me: Pk) {
        if self.mon_at(me).ab_flags & abf::SEEK == 0 {
            return;
        }
        let Some(foe) = self.foe_active(me) else { return };
        let fa = self.mon_at(foe).ability;
        if flags(fa) & aflag::NOTRACE != 0 || fa == abilities::NOABILITY {
            return;
        }
        self.set_ability_by(me, fa, Some(foe), Fx::Ability(abilities::TRACE));
    }

    /// The ability's onStart (singleEvent 'Start'); also run at switch-in when the ability has no onSwitchIn.
    pub fn ability_start(&mut self, p: Pk, ab: AbilityId) {
        if coverage_on() {
            let before = *self.s;
            self.ability_start_inner(p, ab);
            if *self.s != before {
                count_trigger(ab);
            }
            return;
        }
        self.ability_start_inner(p, ab);
    }

    fn ability_start_inner(&mut self, p: Pk, ab: AbilityId) {
        use abilities as A;
        if self.ignoring_ability(p) {
            return;
        }
        let eff = Eff::Ability(ab);
        match ab {
            A::INTIMIDATE => {
                if let Some(foe) = self.foe_active(p) {
                    self.ab_row(p, ab);
                    if !self.has_vol(foe, conds::SUBSTITUTE) {
                        self.boost(&[(0, -1)], foe, Some(p), eff, true, false);
                    } else {
                        self.row(Row::Result { how: How::Immune, p: Some(who(foe)) });
                    }
                }
            }
            A::SUPERSWEETSYRUP => {
                if self.mon_at(p).ab_battle & abb::SYRUP != 0 {
                    return;
                }
                self.mon_at_mut(p).ab_battle |= abb::SYRUP;
                self.ab_row(p, ab);
                if let Some(foe) = self.foe_active(p) {
                    if !self.has_vol(foe, conds::SUBSTITUTE) {
                        self.boost(&[(B_EVA, -1)], foe, Some(p), eff, true, false);
                    } else {
                        self.row(Row::Result { how: How::Immune, p: Some(who(foe)) });
                    }
                }
            }
            A::SANDSTREAM => {
                self.set_weather_by(conds::SANDSTORM, p, eff);
            }
            A::DROUGHT => {
                self.set_weather_by(conds::SUNNYDAY, p, eff);
            }
            A::ORICHALCUMPULSE => {
                if self.set_weather_by(conds::SUNNYDAY, p, eff).truthy() || self.weather() == conds::SUNNYDAY {
                    self.ab_row(p, ab);
                }
            }
            A::DRIZZLE => {
                self.set_weather_by(conds::RAINDANCE, p, eff);
            }
            A::SNOWWARNING => {
                self.set_weather_by(conds::SNOWSCAPE, p, eff);
            }
            A::GRASSYSURGE => {
                self.set_terrain_by(conds::GRASSYTERRAIN, p, eff);
            }
            A::PSYCHICSURGE => {
                self.set_terrain_by(conds::PSYCHICTERRAIN, p, eff);
            }
            A::ELECTRICSURGE => {
                self.set_terrain_by(conds::ELECTRICTERRAIN, p, eff);
            }
            A::HADRONENGINE => {
                if !self.set_terrain_by(conds::ELECTRICTERRAIN, p, eff).truthy() && self.terrain() == conds::ELECTRICTERRAIN {
                    self.ab_row(p, ab);
                }
            }
            A::MISTYSURGE => {
                self.set_terrain_by(conds::MISTYTERRAIN, p, eff);
            }
            A::AIRLOCK | A::CLOUDNINE => {
                self.mon_at_mut(p).ab_flags &= !abf::ENDING;
                self.each_weather_change(eff);
            }
            A::SUPREMEOVERLORD => {
                let tf = self.s.sides[p.0].total_fainted;
                if tf > 0 {
                    self.ab_row(p, ab);
                    self.mon_at_mut(p).ab_data = tf.min(5) as i16;
                }
            }
            A::TRACE => {
                let foe = self.foe_active(p);
                let seek = !foe.map_or(false, |f| self.mon_at(f).ability == A::NOABILITY);
                if seek {
                    self.mon_at_mut(p).ab_flags |= abf::SEEK;
                    self.trace_update(p);
                } else {
                    self.mon_at_mut(p).ab_flags &= !abf::SEEK;
                }
            }
            A::DOWNLOAD => {
                let mut tdef = 0u32;
                let mut tspd = 0u32;
                if let Some(foe) = self.foe_active(p) {
                    tdef += self.get_stat(foe.0, foe.1, Stat::Def, false, true);
                    tspd += self.get_stat(foe.0, foe.1, Stat::Spd, false, true);
                }
                if tdef > 0 && tdef >= tspd {
                    self.boost(&[(2, 1)], p, None, eff, false, false);
                } else if tspd > 0 {
                    self.boost(&[(0, 1)], p, None, eff, false, false);
                }
            }
            A::INTREPIDSWORD => {
                if self.mon_at(p).ab_battle & abb::SWORD != 0 {
                    return;
                }
                self.mon_at_mut(p).ab_battle |= abb::SWORD;
                self.boost(&[(0, 1)], p, None, eff, false, false);
            }
            A::DAUNTLESSSHIELD => {
                if self.mon_at(p).ab_battle & abb::SHIELD != 0 {
                    return;
                }
                self.mon_at_mut(p).ab_battle |= abb::SHIELD;
                self.boost(&[(1, 1)], p, None, eff, false, false);
            }
            A::SCREENCLEANER => {
                let mut shown = false;
                for c in [conds::REFLECT, conds::LIGHTSCREEN, conds::AURORAVEIL] {
                    for side in [p.0, 1 - p.0] {
                        if self.side_cond(side, c).is_some() {
                            if !shown {
                                self.ab_row(p, ab);
                                shown = true;
                            }
                            self.remove_side_cond(side, c);
                        }
                    }
                }
            }
            A::UNNERVE | A::ASONEGLASTRIER | A::ASONESPECTRIER => {
                if self.mon_at(p).ab_flags & abf::UNNERVED == 0 {
                    self.ab_row(p, ab);
                    if ab != A::UNNERVE {
                        self.ab_row(p, A::UNNERVE);
                    }
                }
                self.mon_at_mut(p).ab_flags |= abf::UNNERVED;
            }
            A::FOREWARN => self.forewarn(p),
            A::SLOWSTART => {
                self.ab_row(p, ab);
                self.mon_at_mut(p).ab_data = 5;
            }
            A::GORILLATACTICS => {
                self.mon_at_mut(p).ab_data = 0;
            }
            A::PROTOSYNTHESIS => self.protosynthesis_change(p),
            A::FORECAST => self.forecast_change(p),
            A::QUARKDRIVE => self.quarkdrive_change(p),
            A::MIMICRY => self.mimicry_change(p),
            A::PASTELVEIL => {
                if matches!(self.mon_at(p).status, Status::Psn | Status::Tox) {
                    self.ab_row(p, ab);
                    self.cure_status(p);
                }
            }
            A::TRUANT => {
                self.remove_volatile(p, conds::TRUANT);
                let m = *self.mon_at(p);
                if m.active_turns > 0 && (m.move_this_turn_result != MoveResult::Undefined || self.will_move(p).is_none()) {
                    self.add_volatile(p, conds::TRUANT, Some(p), eff, false);
                }
            }
            A::WINDRIDER => {
                if self.side_cond(p.0, conds::TAILWIND).is_some() {
                    self.boost(&[(0, 1)], p, Some(p), eff, false, false);
                }
            }
            A::SHIELDSDOWN => self.shields_down(p),
            A::ICEFACE => {
                if self.weather_is(&[conds::HAIL, conds::SNOWSCAPE]) && self.mon_at(p).species.id() == "eiscuenoice" {
                    self.ab_row(p, ab);
                    self.mon_at_mut(p).ab_flags &= !abf::BUSTED;
                    self.forme_change(p, "eiscue", true, ab);
                }
            }
            A::PRESSURE | A::MOLDBREAKER | A::TURBOBLAZE | A::TERAVOLT | A::COMATOSE | A::AURABREAK => {
                self.ab_row(p, ab);
            }
            // not announced when a Mold Breaker move brings the holder in
            A::BEADSOFRUIN | A::SWORDOFRUIN | A::TABLETSOFRUIN | A::VESSELOFRUIN | A::FAIRYAURA | A::DARKAURA => {
                if !self.suppressing_ability(p) {
                    self.ab_row(p, ab);
                }
            }
            A::ANTICIPATION => {
                if self.anticipates(p) {
                    self.ab_row(p, ab);
                }
            }
            A::FRISK => {
                if let Some(foe) = self.foe_active(p) {
                    let item = self.mon_at(foe).item;
                    if !item.is_none() {
                        self.row(Row::Item { p: who(foe), item, what: ItemWhat::Found, of: Some(who(p)), by: Fx::Ability(ab) });
                    }
                }
            }
            // the ally-only ones in singles (Costar, Curious Medicine, Hospitality)
            _ => {}
        }
    }

    /// Anticipation's check: a foe's move that is super effective on the holder, or an OHKO move
    fn anticipates(&self, p: Pk) -> bool {
        let Some(foe) = self.foe_active(p) else { return false };
        let types = self.get_types(p, false);
        for ms in self.mon_at(foe).moves() {
            let d = ms.id.data();
            if d.category == Category::Status {
                continue;
            }
            let immune = types.as_slice().iter().any(|&t| !type_affects(d.ty, t));
            let eff: i32 = types.as_slice().iter().map(|&t| effectiveness(d.ty, t)).sum();
            if (!immune && eff > 0) || d.ohko != Ohko::No {
                return true;
            }
        }
        false
    }

    /// Forewarn's onStart: one draw among the foe's strongest moves
    fn forewarn(&mut self, p: Pk) {
        let Some(foe) = self.foe_active(p) else { return };
        let m = *self.mon_at(foe);
        let mut best: Vec<&'static str> = Vec::new();
        let mut bp_best = 1;
        for ms in m.moves() {
            let d = ms.id.data();
            let mut bp = d.base_power as i32;
            if d.ohko != Ohko::No {
                bp = 150;
            }
            if ms.id == moves::COUNTER || ms.id == moves::METALBURST || ms.id == moves::MIRRORCOAT {
                bp = 120;
            }
            if bp == 1 {
                bp = 80;
            }
            if bp == 0 && d.category != Category::Status {
                bp = 80;
            }
            if bp > bp_best {
                best.clear();
                best.push(d.id);
                bp_best = bp;
            } else if bp == bp_best {
                best.push(d.id);
            }
        }
        if best.len() > 1 {
            // the oracle reports the index of the [move, target] pair drawn
            let idx: Vec<i32> = (0..best.len() as i32).collect();
            let site = Site { effect: Effect::One(EffectRef::Ability(abilities::FOREWARN)), ..Site::new(Key::Pick, Some(p.0 as u8)) };
            self.o.sample(&site, &idx);
        }
        if !best.is_empty() {
            self.row(Row::Ability { p: who(p), ab: abilities::FOREWARN, of: Some(who(foe)) });
        }
    }

    /// The ability's switch-in handler (fieldEvent 'SwitchIn'): onSwitchIn, or onStart when it has none.
    pub fn ability_switch_in(&mut self, p: Pk) {
        let ab = self.mon_at(p).ability;
        if ab == abilities::IMPOSTER && coverage_on() {
            let before = *self.s;
            self.ability_switch_in_inner(p, ab);
            if *self.s != before {
                count_trigger(ab);
            }
            return;
        }
        self.ability_switch_in_inner(p, ab);
    }

    fn ability_switch_in_inner(&mut self, p: Pk, ab: AbilityId) {
        if flags(ab) & aflag::BREAKABLE != 0 && self.suppressing_ability(p) {
            return;
        }
        if self.ignoring_ability(p) {
            return;
        }
        match ab {
            abilities::AIRLOCK | abilities::CLOUDNINE => {
                self.ab_row(p, ab);
                self.ability_start(p, ab)
            }
            abilities::IMPOSTER => {
                let foe = (1 - p.0, self.s.sides[1 - p.0].active as usize);
                if self.s.sides[foe.0].has_active {
                    let mark = self.row_mark();
                    self.ab_row(p, ab);
                    if !self.transform_into(p, foe) {
                        self.row_truncate(mark);
                    }
                }
            }
            abilities::NEUTRALIZINGGAS => self.ab_row(p, ab),
            abilities::ZEROTOHERO => {
                if self.mon_at(p).species.id() == "palafinhero" && self.mon_at(p).ab_battle & abb::HERO == 0 {
                    self.mon_at_mut(p).ab_battle |= abb::HERO;
                    self.ab_row(p, ab);
                }
            }
            _ => {
                if ab.data().handlers.contains(&"onStart") {
                    self.ability_start(p, ab);
                }
            }
        }
    }

    /// The ability's onEnd (singleEvent 'End'): switch-out, faint, ability change.
    pub fn ability_end(&mut self, p: Pk, ab: AbilityId) {
        use abilities as A;
        match ab {
            A::AIRLOCK | A::CLOUDNINE => {
                self.mon_at_mut(p).ab_flags |= abf::ENDING;
                self.each_weather_change(Eff::Ability(ab));
            }
            A::FLASHFIRE => {
                self.remove_volatile(p, conds::FLASHFIRE);
            }
            A::ILLUSION => {
                if !self.mon_at(p).being_called_back {
                    // a fainting Pokémon loses its disguise without a row (Illusion's onFaint)
                    if self.mon_at(p).illusion && !self.fainting {
                        self.row(Row::VolatileEnd { p: who(p), c: Cond::NONE, by: Fx::Ability(ab) });
                    }
                    self.mon_at_mut(p).illusion = false;
                }
            }
            A::UNBURDEN => {
                self.remove_volatile(p, conds::UNBURDEN);
            }
            A::PROTOSYNTHESIS | A::QUARKDRIVE => {
                let c = if ab == A::PROTOSYNTHESIS { conds::PROTOSYNTHESIS } else { conds::QUARKDRIVE };
                if self.is_active_pk(p) {
                    self.s.sides[p.0].delete_vol(c);
                }
            }
            A::UNNERVE | A::ASONEGLASTRIER | A::ASONESPECTRIER => self.mon_at_mut(p).ab_flags &= !abf::UNNERVED,
            A::GORILLATACTICS => self.mon_at_mut(p).ab_data = 0,
            A::OPPORTUNIST => {
                if self.is_active_pk(p) {
                    self.s.sides[p.0].opp_boosts = None;
                }
            }
            _ => {}
        }
    }

    /// Opportunist: the recorded stages are copied (onAnySwitchIn, onAnyAfterMega, onResidual)
    pub fn opportunist_copy(&mut self, p: Pk) {
        self.opportunist_boost(p, None);
    }

    /// Opportunist's copy with the event's source (onAnyAfterMove)
    fn opportunist_boost(&mut self, p: Pk, source: Option<Pk>) {
        if !self.is_active_pk(p) {
            return;
        }
        let Some(b) = self.s.sides[p.0].opp_boosts.take() else { return };
        let list: Vec<(usize, i8)> = b.as_slice().iter().map(|&(i, v)| (i as usize, v)).collect();
        self.boost(&list, p, source, Eff::Ability(abilities::OPPORTUNIST), false, false);
    }

    /// Shields Down's onStart and onResidual: Minior's form follows its HP
    fn shields_down(&mut self, p: Pk) {
        let m = *self.mon_at(p);
        if m.base_species.data().base_species.id() != "minior" || m.transformed || m.hp == 0 {
            return;
        }
        let meteor = m.species.data().forme == "Meteor";
        if (m.hp as f64) > m.maxhp as f64 / 2.0 {
            if !meteor {
                self.forme_change(p, "miniormeteor", false, abilities::SHIELDSDOWN);
            }
        } else if meteor {
            let set = m.set_species.id();
            self.forme_change(p, set, false, abilities::SHIELDSDOWN);
        }
    }

    /// Magician's onAfterMoveSecondarySelf: the user steals the item of the Pokémon it hit
    fn magician(&mut self, ctx: &Ctx) {
        let (Some(src), Some(am)) = (ctx.target, ctx.am) else { return };
        let m = *self.mon_at(src);
        if (m.switch_flag && m.switch_flag_move.is_none()) || am.hit_targets.is_none() || !m.item.is_none()
            || self.has_vol(src, conds::GEM) || am.id == moves::FLING || am.category == Category::Status
        {
            return;
        }
        if am.hit_targets != Some(true) {
            return;
        }
        let Some(t) = ctx.source else { return };
        if t == src {
            return;
        }
        if let super::items::Taken::Item(it) = self.take_item(t, Some(src)) {
            if !self.set_item(src, it, Some(t), Eff::Ability(abilities::MAGICIAN)) {
                self.restore_item(t, it);
            } else {
                self.row(Row::Item { p: who(src), item: it, what: ItemWhat::Received, of: Some(who(t)), by: Fx::Ability(abilities::MAGICIAN) });
            }
        }
    }

    /// runEvent('SwitchOut') for the ability (Regenerator, Natural Cure)
    fn ab_switch_out(&mut self, ab: AbilityId, me: Pk) {
        use abilities as A;
        match ab {
            A::REGENERATOR => {
                // pokemon.heal: no TryHeal event
                let m = self.mon_at_mut(me);
                if m.hp > 0 && m.hp < m.maxhp {
                    let a = (m.base_maxhp as f64 / 3.0).trunc().max(1.0) as u32;
                    m.hp = (m.hp as u32 + a).min(m.maxhp as u32) as u16;
                }
            }
            A::NATURALCURE => {
                if self.mon_at(me).status != Status::None {
                    self.cure_status_by(me, Fx::Ability(ab), false);
                }
            }
            A::ZEROTOHERO => {
                let m = self.mon_at(me);
                if m.base_species.data().base_species == SpeciesId::from_id("palafin").unwrap_or_default()
                    && m.species.id() != "palafinhero"
                {
                    self.forme_change(me, "palafinhero", true, ab);
                }
            }
            _ => {}
        }
    }

    /// The ability's onResidual.
    pub fn ability_residual(&mut self, p: Pk, ab: AbilityId) {
        if coverage_on() {
            let before = *self.s;
            self.ability_residual_inner(p, ab);
            if *self.s != before {
                count_trigger(ab);
            }
            return;
        }
        self.ability_residual_inner(p, ab);
    }

    fn ability_residual_inner(&mut self, p: Pk, ab: AbilityId) {
        use abilities as A;
        if self.ignoring_ability(p) {
            return;
        }
        let eff = Eff::Ability(ab);
        match ab {
            A::SPEEDBOOST => {
                if self.mon_at(p).active_turns > 0 {
                    self.boost(&[(4, 1)], p, Some(p), eff, false, false);
                }
            }
            A::MOODY => {
                let b = self.mon_at(p).boosts;
                let names = ["atk", "def", "spa", "spd", "spe"];
                let up: Vec<usize> = (0..5).filter(|&i| b[i] < 6).collect();
                let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Pick, Some(p.0 as u8)) };
                let plus = if up.is_empty() {
                    None
                } else if up.len() == 1 {
                    Some(up[0])
                } else {
                    let v: Vec<&'static str> = up.iter().map(|&i| names[i]).collect();
                    Some(up[self.o.sample_str(&site, &v)])
                };
                let down: Vec<usize> = (0..5).filter(|&i| b[i] > -6 && Some(i) != plus).collect();
                let minus = if down.is_empty() {
                    None
                } else if down.len() == 1 {
                    Some(down[0])
                } else {
                    let v: Vec<&'static str> = down.iter().map(|&i| names[i]).collect();
                    Some(down[self.o.sample_str(&site, &v)])
                };
                let mut list = Vec::new();
                if let Some(i) = plus {
                    list.push((i, 2));
                }
                if let Some(i) = minus {
                    list.push((i, -1));
                }
                self.boost(&list, p, Some(p), eff, false, false);
            }
            A::HARVEST => {
                let go = self.weather_is(&[conds::SUNNYDAY, conds::DESOLATELAND]) || {
                    let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Proc, Some(p.0 as u8)) };
                    self.chance(site, 1, 2)
                };
                let m = *self.mon_at(p);
                if go && m.hp > 0 && m.item.is_none() && m.last_item.data().is_berry {
                    self.set_item(p, m.last_item, None, eff);
                    self.mon_at_mut(p).last_item = ItemId::NONE;
                    self.row(Row::Item { p: who(p), item: m.last_item, what: ItemWhat::Received, of: None, by: Fx::Ability(ab) });
                }
            }
            A::PICKUP => {
                if !self.mon_at(p).item.is_none() {
                    return;
                }
                // the adjacent active Pokémon that used an item this turn: in singles the foe
                let Some(foe) = self.foe_active(p) else { return };
                let fm = *self.mon_at(foe);
                if self.mon_at(p).fainted || fm.last_item.is_none() || !fm.used_item_this_turn {
                    return;
                }
                let item = fm.last_item;
                self.mon_at_mut(foe).last_item = ItemId::NONE;
                // the row before setItem, as for Recycle
                self.row(Row::Item { p: who(p), item, what: ItemWhat::Received, of: None, by: Fx::Ability(ab) });
                self.set_item(p, item, None, eff);
            }
            A::CUDCHEW => {
                let m = *self.mon_at(p);
                if m.ab_data == 0 || m.hp == 0 {
                    return;
                }
                let c = m.ab_count - 1;
                self.mon_at_mut(p).ab_count = c;
                if c <= 0 {
                    let item = ItemId(m.ab_data as u16);
                    self.ab_row(p, ab);
                    self.row(Row::Item { p: who(p), item, what: ItemWhat::Used, of: None, by: Fx::None });
                    self.item_eat(item, p, None, Eff::None);
                    self.eat_item_event(p, None, Eff::None, item);
                    let mm = self.mon_at_mut(p);
                    mm.ate_berry = true;
                    mm.ab_data = 0;
                    mm.ab_count = 0;
                }
            }
            A::OPPORTUNIST => self.opportunist_copy(p),
            A::SHIELDSDOWN => self.shields_down(p),
            A::SHEDSKIN => {
                let m = self.mon_at(p);
                if m.hp > 0 && m.status != Status::None {
                    let site = Site { effect: Effect::One(EffectRef::Ability(ab)), ..Site::new(Key::Proc, Some(p.0 as u8)) };
                    if self.chance(site, 33, 100) {
                        self.ab_row(p, ab);
                        self.cure_status(p);
                    }
                }
            }
            A::HYDRATION => {
                if self.mon_at(p).status != Status::None && self.eff_weather_is(p, &[conds::RAINDANCE, conds::PRIMORDIALSEA]) {
                    self.ab_row(p, ab);
                    self.cure_status(p);
                }
            }
            A::BADDREAMS => {
                if self.mon_at(p).hp == 0 {
                    return;
                }
                if let Some(foe) = self.foe_active(p) {
                    if self.mon_at(foe).status == Status::Slp || self.has_ability(foe, A::COMATOSE) {
                        let a = self.mon_at(foe).base_maxhp as f64 / 8.0;
                        self.damage(a, foe, Some(p), eff);
                    }
                }
            }
            A::SLOWSTART => {
                let m = self.mon_at_mut(p);
                if m.active_turns > 0 && m.ab_data > 0 {
                    m.ab_data -= 1;
                    if m.ab_data == 0 {
                        self.row(Row::Once { p: Some(who(p)), fx: Fx::Ability(ab), of: None });
                    }
                }
            }
            A::HUNGERSWITCH => {
                let sid = self.mon_at(p).species.id();
                if sid == "morpeko" {
                    self.forme_change(p, "morpekohangry", false, ab);
                } else if sid == "morpekohangry" {
                    self.forme_change(p, "morpeko", false, ab);
                }
            }
            // Healer: no ally in singles
            _ => {}
        }
    }

    // ------------------------------------------------------------------------------------------ form, type, weather

    /// Protosynthesis' onWeatherChange
    fn protosynthesis_change(&mut self, p: Pk) {
        if !self.is_active_pk(p) || self.mon_at(p).ability != abilities::PROTOSYNTHESIS || self.ignoring_ability(p) {
            return;
        }
        if self.weather() == conds::SUNNYDAY {
            self.add_volatile(p, conds::PROTOSYNTHESIS, Some(p), Eff::Ability(abilities::PROTOSYNTHESIS), false);
        } else {
            self.remove_volatile(p, conds::PROTOSYNTHESIS);
        }
    }

    /// Forecast's onWeatherChange: Castform's form follows the weather
    fn forecast_change(&mut self, p: Pk) {
        let m = *self.mon_at(p);
        let castform = SpeciesId::from_id("castform").unwrap_or_default();
        if m.ability != abilities::FORECAST || self.ignoring_ability(p) || m.base_species.data().base_species != castform || m.transformed {
            return;
        }
        let w = self.eff_weather(p);
        let to = if w == conds::SUNNYDAY || w == conds::DESOLATELAND {
            "castformsunny"
        } else if w == conds::RAINDANCE || w == conds::PRIMORDIALSEA {
            "castformrainy"
        } else if w == conds::HAIL || w == conds::SNOWSCAPE {
            "castformsnowy"
        } else {
            "castform"
        };
        if m.is_active && m.species.id() != to {
            self.forme_change(p, to, false, abilities::FORECAST);
        }
    }

    /// Quark Drive's onTerrainChange
    fn quarkdrive_change(&mut self, p: Pk) {
        if !self.is_active_pk(p) || self.mon_at(p).ability != abilities::QUARKDRIVE || self.ignoring_ability(p) {
            return;
        }
        if self.terrain() == conds::ELECTRICTERRAIN {
            self.add_volatile(p, conds::QUARKDRIVE, Some(p), Eff::Ability(abilities::QUARKDRIVE), false);
        } else {
            self.remove_volatile(p, conds::QUARKDRIVE);
        }
    }

    /// Mimicry's onTerrainChange
    fn mimicry_change(&mut self, p: Pk) {
        if !self.is_active_pk(p) || self.mon_at(p).ability != abilities::MIMICRY || self.ignoring_ability(p) {
            return;
        }
        let tr = self.s.field.terrain.id;
        let ty = if tr == conds::ELECTRICTERRAIN {
            Some(Type::Electric)
        } else if tr == conds::GRASSYTERRAIN {
            Some(Type::Grass)
        } else if tr == conds::MISTYTERRAIN {
            Some(Type::Fairy)
        } else if tr == conds::PSYCHICTERRAIN {
            Some(Type::Psychic)
        } else {
            None
        };
        let base = self.mon_at(p).base_species.data().types;
        let (list, n): ([Type; 2], usize) = match ty {
            Some(t) => ([t, Type::Typeless], 1),
            None => (base, if base[1] == Type::Typeless { 1 } else { 2 }),
        };
        let old = self.get_types(p, false);
        if old.as_slice() == &list[..n] {
            return;
        }
        self.mon_at_mut(p).set_types(&list[..n]);
        if ty.is_some() || self.mon_at(p).transformed {
            self.types_row(p, Fx::Ability(abilities::MIMICRY));
        } else {
            self.ab_row(p, abilities::MIMICRY);
        }
    }

    /// eachEvent('WeatherChange', effect)
    pub fn each_weather_change(&mut self, _eff: Eff) {
        let (order, n) = self.speed_order();
        for &side in &order[..n] {
            let sd = &self.s.sides[side];
            if !sd.has_active {
                continue;
            }
            let p = (side, sd.active as usize);
            if self.any_handler(Ev::WeatherChange) {
                let mut ctx = Ctx::new(Some(p), None, Eff::None);
                self.run_event(Ev::WeatherChange, &mut ctx, R::True);
            }
        }
    }

    /// pokemon.tryTrap(true): a trap the request does not show
    pub fn try_trap_hidden(&mut self, p: Pk) {
        if !self.run_status_immunity(p, Imm::Cond(conds::TRAPPED)) {
            return;
        }
        let m = self.mon_at_mut(p);
        if m.trapped {
            return;
        }
        m.trapped = true;
        m.trap_hidden = true;
    }

    /// pokemon.setAbility. `forme`: from a form change (no checks, no SetAbility event); `transform`: from
    /// Transform (Start only when the ability differs). Returns false when the change fails.
    pub fn set_ability(&mut self, p: Pk, ab: AbilityId, source: Option<Pk>, forme: bool, transform: bool) -> bool {
        self.set_ability_with(p, ab, source, forme, transform, Fx::None)
    }

    /// pokemon.setAbility by an effect (Trace, Mummy, a move): its row names the effect
    pub fn set_ability_by(&mut self, p: Pk, ab: AbilityId, source: Option<Pk>, by: Fx) -> bool {
        self.set_ability_with(p, ab, source, false, false, by)
    }

    fn set_ability_with(&mut self, p: Pk, ab: AbilityId, source: Option<Pk>, forme: bool, transform: bool, by: Fx) -> bool {
        if self.mon_at(p).hp == 0 {
            return false;
        }
        let old = self.mon_at(p).ability;
        if !forme && (flags(ab) & aflag::CANTSUPPRESS != 0 || flags(old) & aflag::CANTSUPPRESS != 0) {
            return false;
        }
        self.note_ability(ab);
        self.s.ab_union |= ability_evmask(ab);
        // singleEvent('End', old): suppressed while the Pokémon ignores its ability, except End itself
        self.ability_end(p, old);
        self.mon_at_mut(p).ability = ab;
        self.s.no_holder_fx &= super::hooks::masks().abilities_unimpl[ab.0 as usize] == 0;
        self.init_ability_state(p);
        if !forme && !transform && by != Fx::None && self.logging() {
            if matches!(by, Fx::Ability(a) if a == abilities::MUMMY || a == abilities::LINGERINGAROMA) {
                if let (Some(src), Fx::Ability(a)) = (source, by) {
                    self.row(Row::Ability { p: who(src), ab: a, of: Some(who(p)) });
                }
            }
            self.row(Row::AbilityChange { p: who(p), from: old, to: ab, by });
        }
        if ab != AbilityId::NONE && (!transform || old != ab) {
            self.ability_start(p, ab);
        }
        true
    }

    /// battle.skillSwap
    pub fn skill_swap(&mut self, source: Pk, target: Pk) -> bool {
        if self.mon_at(source).fainted || self.mon_at(target).fainted {
            return false;
        }
        let sa = self.mon_at(source).ability;
        let ta = self.mon_at(target).ability;
        if flags(sa) & aflag::FAILSKILLSWAP != 0 || flags(ta) & aflag::FAILSKILLSWAP != 0 {
            return false;
        }
        self.note_ability(sa);
        self.note_ability(ta);
        self.s.ab_union |= ability_evmask(sa) | ability_evmask(ta);
        self.row(Row::Once { p: Some(who(source)), fx: Fx::Move(moves::SKILLSWAP), of: Some(who(target)) });
        self.ability_end(source, sa);
        self.ability_end(target, ta);
        self.mon_at_mut(source).ability = ta;
        self.mon_at_mut(target).ability = sa;
        self.init_ability_state(source);
        self.init_ability_state(target);
        self.ability_start(target, sa);
        self.ability_start(source, ta);
        true
    }
}

/// The status of a SetStatus or AfterSetStatus event's argument.
pub fn status_of(arg: u16) -> Status {
    match arg {
        1 => Status::Brn,
        2 => Status::Par,
        3 => Status::Slp,
        4 => Status::Frz,
        5 => Status::Psn,
        6 => Status::Tox,
        _ => Status::None,
    }
}
