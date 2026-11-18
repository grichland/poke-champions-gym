//! The shared damage estimate (spec 3C, PS-Q1): one integer damage calculation per (attacker, move,
//! defender) through `sim::calc::hit_damage`, with the roll fixed, no crit and no branching over
//! outcomes. Inputs come from `Fighter`s built from a Battle, so unknown opposing sets follow D7's fixed
//! rule. Modifiers are Showdown's (gen 9, Champions mod) for what the Battle can show.

use crate::view::{grounded, speed, vf, Fighter, FieldView};
use battle::{Battle, Side, Who};
use cg_engine::sim::calc::{accuracy_with_stage, apply_stage, chain, hit_damage, modify, HitInput};
use cg_engine::sim::data::*;

/// Roll indices of `HitInput::roll` (damage factor 100 - roll percent).
pub const ROLL_MAX: u32 = 0;
/// The fixed roll of the point estimate: 92%, the nearest of the 16 to the mean factor 92.5%.
pub const ROLL_MID: u32 = 8;
pub const ROLL_MIN: u32 = 15;
pub const LEVEL: u32 = 50;

/// One move's estimate against one defender.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Hit {
    /// HP points of one hit at the lowest, fixed and highest roll
    pub min: u32,
    pub mid: u32,
    pub max: u32,
    /// the defender's max HP as used (own: exact; opponent: the fixed rule)
    pub def_max_hp: u32,
    /// chance the move hits, 0..=1
    pub accuracy: f32,
    /// expected number of hits given the first hits (escalating moves fold their power steps in)
    pub hits: f32,
    /// most hits it can deal (escalating moves: the sum of their power steps)
    pub max_hits: f32,
}

impl Hit {
    /// One hit at the fixed roll, as a fraction of the defender's max HP.
    pub fn frac(&self) -> f32 {
        self.mid as f32 / self.def_max_hp.max(1) as f32
    }
    /// Expected fraction: fixed roll x accuracy x expected hits (tower's KO test, max damage's ranking).
    pub fn expected(&self) -> f32 {
        self.frac() * self.accuracy * self.hits
    }
    /// The most it can deal: highest roll x most hits, no accuracy (tower's "can it KO us").
    pub fn max_frac(&self) -> f32 {
        self.max as f32 * self.max_hits / self.def_max_hp.max(1) as f32
    }
    pub fn is_zero(&self) -> bool {
        self.max == 0
    }
}

fn zero(def: &Fighter) -> Hit {
    Hit { def_max_hp: def.max_hp(), ..Hit::default() }
}

fn fixed(def: &Fighter, hp: u32, accuracy: f32) -> Hit {
    Hit { min: hp, mid: hp, max: hp, def_max_hp: def.max_hp(), accuracy, hits: 1.0, max_hits: 1.0 }
}

const ATE: [(&str, Type); 5] =
    [("aerilate", Type::Flying), ("pixilate", Type::Fairy), ("refrigerate", Type::Ice), ("galvanize", Type::Electric), ("dragonize", Type::Dragon)];
const NO_TYPE_CHANGE: [&str; 6] = ["judgment", "multiattack", "naturalgift", "revelationdance", "terrainpulse", "weatherball"];

/// Type-boosting held items (gen 9: 4915/4096).
fn type_item(id: &str) -> Option<Type> {
    Some(match id {
        "silverpowder" => Type::Bug,
        "blackglasses" => Type::Dark,
        "dragonfang" => Type::Dragon,
        "magnet" => Type::Electric,
        "fairyfeather" => Type::Fairy,
        "blackbelt" => Type::Fighting,
        "charcoal" => Type::Fire,
        "sharpbeak" => Type::Flying,
        "spelltag" => Type::Ghost,
        "miracleseed" => Type::Grass,
        "softsand" => Type::Ground,
        "nevermeltice" => Type::Ice,
        "silkscarf" => Type::Normal,
        "poisonbarb" => Type::Poison,
        "twistedspoon" => Type::Psychic,
        "hardstone" => Type::Rock,
        "metalcoat" => Type::Steel,
        "mysticwater" => Type::Water,
        _ => return None,
    })
}

/// Type-resist Berries: the type they halve when super effective (Chilan: Normal, any effectiveness).
fn resist_berry(id: &str) -> Option<Type> {
    Some(match id {
        "occaberry" => Type::Fire,
        "passhoberry" => Type::Water,
        "wacanberry" => Type::Electric,
        "rindoberry" => Type::Grass,
        "yacheberry" => Type::Ice,
        "chopleberry" => Type::Fighting,
        "kebiaberry" => Type::Poison,
        "shucaberry" => Type::Ground,
        "cobaberry" => Type::Flying,
        "payapaberry" => Type::Psychic,
        "tangaberry" => Type::Bug,
        "chartiberry" => Type::Rock,
        "kasibberry" => Type::Ghost,
        "habanberry" => Type::Dragon,
        "colburberry" => Type::Dark,
        "babiriberry" => Type::Steel,
        "roseliberry" => Type::Fairy,
        "chilanberry" => Type::Normal,
        _ => return None,
    })
}

/// The move's type after the attacker's type-changing ability and the field.
pub fn move_type(atk: &Fighter, mv: MoveId, field: &FieldView) -> (Type, bool) {
    let d = mv.data();
    let mut ty = d.ty;
    match d.id {
        "weatherball" => {
            ty = if field.sun() {
                Type::Fire
            } else if field.rain() {
                Type::Water
            } else if field.sand() {
                Type::Rock
            } else if field.snow() {
                Type::Ice
            } else {
                Type::Normal
            }
        }
        "terrainpulse" if grounded(atk, field) => {
            ty = match field.terrain.map(|c| c.id()) {
                Some("electricterrain") => Type::Electric,
                Some("grassyterrain") => Type::Grass,
                Some("mistyterrain") => Type::Fairy,
                Some("psychicterrain") => Type::Psychic,
                _ => ty,
            }
        }
        "revelationdance" => ty = atk.types[0],
        _ => {}
    }
    if ty == Type::Normal && !NO_TYPE_CHANGE.contains(&d.id) && d.category != Category::Status {
        if let Some(&(_, t)) = ATE.iter().find(|(a, _)| *a == atk.ab()) {
            return (t, true);
        }
        if atk.ab() == "normalize" {
            return (Type::Normal, true);
        }
    } else if atk.ab() == "normalize" && d.category != Category::Status {
        return (Type::Normal, true);
    }
    (ty, false)
}

/// Sum of effectiveness steps over the defender's types, None if a type does not affect it.
fn type_steps(atk: &Fighter, mv: MoveId, ty: Type, def: &Fighter, field: &FieldView) -> Option<i32> {
    let d = mv.data();
    let mut steps = 0;
    for &t in def.types.iter() {
        if !t.is_real() {
            continue;
        }
        let ghost_ok = t == Type::Ghost && matches!(ty, Type::Normal | Type::Fighting) && matches!(atk.ab(), "scrappy" | "mindseye");
        let ground_vs_flying = ty == Type::Ground && t == Type::Flying;
        if !type_affects(ty, t) && !ghost_ok && !ground_vs_flying {
            return None;
        }
        if ground_vs_flying {
            continue;
        }
        steps += if d.id == "freezedry" && t == Type::Water { 1 } else { effectiveness(ty, t) };
    }
    if ty == Type::Ground && !grounded(def, field) {
        return None;
    }
    Some(steps)
}

/// The defender's revealed ability makes the move do nothing.
fn ability_immune(d: &MoveData, ty: Type, steps: i32, def: &Fighter, priority: i8) -> bool {
    match def.ab() {
        "flashfire" | "wellbakedbody" => ty == Type::Fire,
        "waterabsorb" | "stormdrain" | "dryskin" => ty == Type::Water,
        "voltabsorb" | "lightningrod" | "motordrive" => ty == Type::Electric,
        "sapsipper" => ty == Type::Grass,
        "eartheater" => ty == Type::Ground,
        "bulletproof" => d.has_flag(flag::BULLET),
        "soundproof" => d.has_flag(flag::SOUND),
        "windrider" => d.has_flag(flag::WIND),
        "wonderguard" => steps <= 0,
        "dazzling" | "queenlymajesty" | "armortail" => priority > 0,
        _ => false,
    }
}

/// Priority as the move would be used now (Grassy Glide, Gale Wings, Prankster, Triage).
pub fn priority(atk: &Fighter, mv: MoveId, field: &FieldView) -> i8 {
    let d = mv.data();
    let mut p = d.priority;
    if d.id == "grassyglide" && field.terrain_is("grassyterrain") && grounded(atk, field) {
        p += 1;
    }
    match atk.ab() {
        "galewings" if d.ty == Type::Flying && atk.hp >= 100.0 => p += 1,
        "prankster" if d.category == Category::Status => p += 1,
        "triage" if d.has_flag(flag::HEAL) => p += 3,
        _ => {}
    }
    p
}

/// Base power after the move's own power rule (basePowerCallback and the like).
fn base_power(atk: &Fighter, mv: MoveId, def: &Fighter, field: &FieldView, b: Option<&Battle>) -> u32 {
    let d = mv.data();
    let bp = d.base_power as u32;
    let frac = |f: &Fighter| f.hp.clamp(0.0, 100.0) / 100.0;
    match d.id {
        "grassknot" | "lowkick" => match def.weight_hg() {
            w if w >= 2000 => 120,
            w if w >= 1000 => 100,
            w if w >= 500 => 80,
            w if w >= 250 => 60,
            w if w >= 100 => 40,
            _ => 20,
        },
        "heavyslam" | "heatcrash" => {
            let (a, t) = (atk.weight_hg(), def.weight_hg().max(1));
            if a >= t * 5 {
                120
            } else if a >= t * 4 {
                100
            } else if a >= t * 3 {
                80
            } else if a >= t * 2 {
                60
            } else {
                40
            }
        }
        "electroball" => {
            let t = speed(def, field).max(1);
            let r = speed(atk, field) / t;
            [40, 60, 80, 120, 150][r.min(4) as usize]
        }
        "gyroball" => {
            let a = speed(atk, field).max(1);
            (25 * speed(def, field) / a + 1).min(150)
        }
        "facade" if atk.status != Status::None => bp * 2,
        "hex" if def.status != Status::None || def.ab() == "comatose" => bp * 2,
        "venoshock" | "barbbarrage" if matches!(def.status, Status::Psn | Status::Tox) => bp * 2,
        "brine" if def.hp <= 50.0 => bp * 2,
        "acrobatics" if atk.item.is_none() && (atk.no_item || atk.side == Side::Own) => bp * 2,
        "storedpower" | "powertrip" => 20 + 20 * atk.boosts.iter().filter(|&&x| x > 0).map(|&x| x as u32).sum::<u32>(),
        "eruption" | "waterspout" | "dragonenergy" => ((bp as f32 * frac(atk)) as u32).max(1),
        "reversal" | "flail" => {
            let r = (48.0 * frac(atk)) as u32;
            match r {
                0..=1 => 200,
                2..=4 => 150,
                5..=9 => 100,
                10..=16 => 80,
                17..=32 => 40,
                _ => 20,
            }
        }
        "weatherball" if field.weather.is_some() => bp * 2,
        "risingvoltage" if field.terrain_is("electricterrain") && grounded(def, field) => bp * 2,
        "lastrespects" => {
            let fainted = b.map_or(0, |b| {
                let s = b.side(atk.side);
                let n = if atk.side == Side::Own { b.n_own as usize } else { 6 };
                s.team[..n].iter().filter(|m| m.fainted).count() as u32
            });
            (50 + 50 * fainted).min(5050)
        }
        _ => bp,
    }
}

/// The estimate for one (attacker, move, defender). `b` gives the counts some moves read (Last Respects).
pub fn estimate(field: &FieldView, atk: &Fighter, mv: MoveId, def: &Fighter, b: Option<&Battle>) -> Hit {
    let d = mv.data();
    if d.category == Category::Status || mv.is_none() {
        return zero(def);
    }
    let (ty, ate) = move_type(atk, mv, field);
    let Some(steps) = type_steps(atk, mv, ty, def, field) else { return zero(def) };
    let prio = priority(atk, mv, field);
    if ability_immune(d, ty, steps, def, prio) {
        return zero(def);
    }
    if prio > 0 && field.terrain_is("psychicterrain") && grounded(def, field) {
        return zero(def);
    }
    let acc = accuracy(field, atk, mv, def);

    // fixed damage
    if d.ohko != Ohko::No {
        if def.ab() == "sturdy" {
            return zero(def);
        }
        if let Ohko::Type(t) = d.ohko {
            if def.has_type(t) {
                return zero(def);
            }
        }
        let base = if d.id == "sheercold" && !atk.has_type(Type::Ice) { 20.0 } else { 30.0 };
        return fixed(def, def.max_hp(), if atk.ab() == "noguard" || def.ab() == "noguard" { 1.0 } else { base / 100.0 });
    }
    match d.damage {
        FixedDamage::Level => return fixed(def, LEVEL, acc),
        FixedDamage::Value(v) => return fixed(def, v as u32, acc),
        FixedDamage::None => {}
    }
    match d.id {
        "superfang" | "ruination" | "naturesmadness" => return fixed(def, (def.cur_hp() / 2).max(1), acc),
        "finalgambit" => return fixed(def, atk.cur_hp(), acc),
        "endeavor" => return fixed(def, def.cur_hp().saturating_sub(atk.cur_hp()), acc),
        "counter" | "mirrorcoat" | "metalburst" | "bide" | "comeuppance" => return zero(def),
        _ => {}
    }

    let mut bp = base_power(atk, mv, def, field, b);
    if bp == 0 {
        return zero(def);
    }
    let physical = d.category == Category::Physical;
    let contact = d.has_flag(flag::CONTACT) && atk.ab() != "longreach";

    // BasePower chain
    let mut m = 4096;
    let a_ab = atk.ab();
    let up = match a_ab {
        "technician" if bp <= 60 => 6144,
        "toughclaws" if contact => 5325,
        "ironfist" if d.has_flag(flag::PUNCH) => 4915,
        "strongjaw" if d.has_flag(flag::BITE) => 6144,
        "sharpness" if d.has_flag(flag::SLICING) => 6144,
        "megalauncher" if d.has_flag(flag::PULSE) => 6144,
        "punkrock" if d.has_flag(flag::SOUND) => 5325,
        "reckless" if d.recoil.0 > 0 || d.has_crash_damage => 4915,
        "sheerforce" if !d.secondaries.is_empty() => 5325,
        "sandforce" if field.sand() && matches!(ty, Type::Rock | Type::Ground | Type::Steel) => 5325,
        "toxicboost" if physical && matches!(atk.status, Status::Psn | Status::Tox) => 6144,
        "flareboost" if !physical && atk.status == Status::Brn => 6144,
        _ => 4096,
    };
    m = chain(m, up);
    if ate {
        m = chain(m, 4915);
    }
    if def.ab() == "dryskin" && ty == Type::Fire {
        m = chain(m, 5120);
    }
    if let Some(t) = type_item(atk.it()) {
        if t == ty {
            m = chain(m, 4915);
        }
    }
    match atk.it() {
        "muscleband" if physical => m = chain(m, 4505),
        "wiseglasses" if !physical => m = chain(m, 4505),
        "normalgem" if ty == Type::Normal => m = chain(m, 5325),
        _ => {}
    }
    if atk.vols & vf::CHARGE != 0 && ty == Type::Electric {
        m = chain(m, 8192);
    }
    if d.id == "knockoff" && def.item.is_some() {
        m = chain(m, 6144);
    }
    if matches!(d.id, "solarbeam" | "solarblade") && (field.rain() || field.sand() || field.snow()) {
        m = chain(m, 2048);
    }
    if d.id == "expandingforce" && field.terrain_is("psychicterrain") && grounded(atk, field) {
        m = chain(m, 6144);
    }
    if d.id == "psyblade" && field.terrain_is("electricterrain") {
        m = chain(m, 6144);
    }
    if grounded(atk, field) {
        let boosted = match field.terrain.map(|c| c.id()) {
            Some("electricterrain") => ty == Type::Electric,
            Some("grassyterrain") => ty == Type::Grass,
            Some("psychicterrain") => ty == Type::Psychic,
            _ => false,
        };
        if boosted {
            m = chain(m, 5325);
        }
    }
    if grounded(def, field) {
        if field.terrain_is("mistyterrain") && ty == Type::Dragon {
            m = chain(m, 2048);
        }
        if field.terrain_is("grassyterrain") && matches!(d.id, "earthquake" | "bulldoze" | "magnitude") {
            m = chain(m, 2048);
        }
    }
    bp = modify(bp, m).max(1);

    // attacking stat
    let src = if d.offensive_pokemon_target { def } else { atk };
    let a_stat = d.override_offensive_stat.unwrap_or(if physical { Stat::Atk } else { Stat::Spa });
    let d_stat = d.override_defensive_stat.unwrap_or(if physical { Stat::Def } else { Stat::Spd });
    let stored = |f: &Fighter, s: Stat| -> u32 {
        let s = match (field.wonder_room, s) {
            (true, Stat::Def) => Stat::Spd,
            (true, Stat::Spd) => Stat::Def,
            (_, s) => s,
        };
        f.stats[s as usize] as u32
    };
    let mut a_boost = src.boosts[a_stat.boost_index()];
    let mut d_boost = def.boosts[d_stat.boost_index()];
    if d.ignore_offensive || def.ab() == "unaware" {
        a_boost = 0;
    }
    if d.ignore_defensive || a_ab == "unaware" {
        d_boost = 0;
    }
    let mut attack = apply_stage(stored(src, a_stat), a_boost);
    if a_ab == "hustle" && physical {
        attack = modify(attack, 6144);
    }
    let mut am = 4096;
    let low_hp = atk.hp <= 100.0 / 3.0;
    let am_up = match a_ab {
        "hugepower" | "purepower" if a_stat == Stat::Atk => 8192,
        "guts" if physical && atk.status != Status::None => 6144,
        "solarpower" if !physical && field.sun() => 6144,
        "gorillatactics" if physical => 6144,
        "transistor" if ty == Type::Electric => 5325,
        "dragonsmaw" if ty == Type::Dragon => 6144,
        "rockypayload" if ty == Type::Rock => 6144,
        "steelworker" if ty == Type::Steel => 6144,
        "waterbubble" if ty == Type::Water => 8192,
        "blaze" if low_hp && ty == Type::Fire => 6144,
        "torrent" if low_hp && ty == Type::Water => 6144,
        "overgrow" if low_hp && ty == Type::Grass => 6144,
        "swarm" if low_hp && ty == Type::Bug => 6144,
        "defeatist" if atk.hp <= 50.0 => 2048,
        _ => 4096,
    };
    am = chain(am, am_up);
    if atk.vols & vf::FLASHFIRE != 0 && ty == Type::Fire {
        am = chain(am, 6144);
    }
    let am_down = match def.ab() {
        "thickfat" if matches!(ty, Type::Fire | Type::Ice) => 2048,
        "heatproof" | "waterbubble" if ty == Type::Fire => 2048,
        "purifyingsalt" if ty == Type::Ghost => 2048,
        _ => 4096,
    };
    am = chain(am, am_down);
    attack = modify(attack, am);

    // defending stat
    let mut defense = apply_stage(stored(def, d_stat), d_boost);
    let spd_used = matches!((field.wonder_room, d_stat), (false, Stat::Spd) | (true, Stat::Def));
    if field.sand() && def.has_type(Type::Rock) && spd_used {
        defense = modify(defense, 6144);
    }
    if field.snow() && def.has_type(Type::Ice) && !spd_used {
        defense = modify(defense, 6144);
    }
    if def.ab() == "furcoat" && !spd_used {
        defense = modify(defense, 8192);
    }

    // weather
    let sun = field.sun() || a_ab == "megasol";
    let weather = if sun && ty == Type::Fire || field.rain() && ty == Type::Water {
        6144
    } else if sun && ty == Type::Water || field.rain() && ty == Type::Fire {
        2048
    } else {
        4096
    };

    // STAB
    let stab = if ty == Type::Typeless {
        None
    } else if atk.has_type(ty) || d.force_stab {
        Some(if a_ab == "adaptability" { 8192 } else { 6144 })
    } else {
        Some(4096)
    };

    // final modifiers
    let mut f = 4096;
    let def_side = if def.side == Side::Own { 0 } else { 1 };
    let sc = field.screens[def_side];
    let screened = if physical { sc[0] || sc[2] } else { sc[1] || sc[2] };
    if screened && a_ab != "infiltrator" && !matches!(d.id, "brickbreak" | "psychicfangs" | "ragingbull") {
        f = chain(f, 2048);
    }
    if matches!(def.ab(), "multiscale" | "shadowshield") && def.hp >= 100.0 {
        f = chain(f, 2048);
    }
    if matches!(def.ab(), "filter" | "solidrock" | "prismarmor") && steps > 0 {
        f = chain(f, 3072);
    }
    if a_ab == "tintedlens" && steps < 0 {
        f = chain(f, 8192);
    }
    match def.ab() {
        "icescales" if !physical => f = chain(f, 2048),
        "fluffy" => {
            if contact {
                f = chain(f, 2048);
            }
            if ty == Type::Fire {
                f = chain(f, 8192);
            }
        }
        "punkrock" if d.has_flag(flag::SOUND) => f = chain(f, 2048),
        _ => {}
    }
    match atk.it() {
        "expertbelt" if steps > 0 => f = chain(f, 4915),
        "lifeorb" => f = chain(f, 5324),
        _ => {}
    }
    if let Some(t) = resist_berry(def.it()) {
        if t == ty && (steps > 0 || t == Type::Normal) {
            f = chain(f, 2048);
        }
    }

    let input = HitInput {
        level: LEVEL,
        base_power: bp,
        attack,
        defense,
        spread: false,
        parental_second: false,
        weather,
        crit: false,
        crit_modifier: 0,
        roll: ROLL_MID,
        stab,
        type_mod: steps,
        burn: atk.status == Status::Brn && physical && a_ab != "guts" && d.id != "facade",
        final_mod: f,
        quarter: false,
    };
    let mid = hit_damage(&input);
    let max = hit_damage(&HitInput { roll: ROLL_MAX, ..input });
    let min = hit_damage(&HitInput { roll: ROLL_MIN, ..input });
    let (hits, max_hits) = hit_counts(atk, d, acc);
    Hit { min, mid, max, def_max_hp: def.max_hp(), accuracy: acc, hits, max_hits }
}

/// Expected and maximum hits. Multi-accuracy moves (Triple Axel, Population Bomb) check each hit;
/// escalating ones count hit k as k times the first.
fn hit_counts(atk: &Fighter, d: &MoveData, acc: f32) -> (f32, f32) {
    let (lo, hi) = d.multihit;
    if hi == 0 {
        if atk.ab() == "parentalbond" && !d.has_flag(flag::NOPARENTALBOND) {
            return (1.25, 1.25);
        }
        return (1.0, 1.0);
    }
    let escalating = matches!(d.id, "tripleaxel" | "triplekick");
    if d.multiaccuracy {
        let n = hi as i32;
        let skill = atk.ab() == "skilllink";
        let mut e = 0.0;
        let mut mx = 0.0;
        for k in 1..=n {
            let w = if escalating { k as f32 } else { 1.0 };
            let p = if skill { 1.0 } else { acc.max(1e-6).powi(k - 1) };
            e += w * p;
            mx += w;
        }
        return (e, mx);
    }
    if lo == hi {
        return (lo as f32, hi as f32);
    }
    if atk.ab() == "skilllink" {
        return (hi as f32, hi as f32);
    }
    // 2-5 hits: 35%, 35%, 15%, 15%
    if (lo, hi) == (2, 5) {
        return (3.1, 5.0);
    }
    ((lo as f32 + hi as f32) / 2.0, hi as f32)
}

/// Chance the move hits (hitStepAccuracy from what is known), 0..=1.
pub fn accuracy(field: &FieldView, atk: &Fighter, mv: MoveId, def: &Fighter) -> f32 {
    let d = mv.data();
    if atk.ab() == "noguard" || def.ab() == "noguard" {
        return 1.0;
    }
    let mut acc = d.accuracy as u32;
    match d.id {
        "thunder" | "hurricane" if field.rain() => return 1.0,
        "thunder" | "hurricane" if field.sun() => acc = 50,
        "blizzard" if field.snow() => return 1.0,
        _ => {}
    }
    if d.never_misses() {
        return 1.0;
    }
    let a = if def.ab() == "unaware" { 0 } else { atk.boosts[B_ACC] as i32 };
    let e = if d.ignore_evasion || atk.ab() == "keeneye" || atk.ab() == "mindseye" || atk.ab() == "unaware" { 0 } else { def.boosts[B_EVA] as i32 };
    let mut v = accuracy_with_stage(acc, (a - e).clamp(-6, 6));
    let mut m = 4096;
    if field.gravity {
        m = chain(m, 6840);
    }
    match atk.ab() {
        "compoundeyes" => m = chain(m, 5325),
        "hustle" if d.category == Category::Physical => m = chain(m, 3277),
        "victorystar" => m = chain(m, 4506),
        _ => {}
    }
    match def.ab() {
        "sandveil" if field.sand() => m = chain(m, 3277),
        "snowcloak" if field.snow() => m = chain(m, 3277),
        _ => {}
    }
    if atk.it() == "widelens" {
        m = chain(m, 4505);
    }
    if atk.it() == "zoomlens" {
        // moving after the target: unknown here, left out
    }
    if def.it() == "brightpowder" {
        m = chain(m, 3686);
    }
    v = modify(v, m);
    (v as f32 / 100.0).min(1.0)
}

/// Key of a cached estimate: attacker, its Mega variant, move, defender.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    atk: (Side, u8),
    mega: bool,
    mv: MoveId,
    def: (Side, u8),
}

/// Estimates over one Battle, cached for one decision. Make a new one (or `reset`) per decision.
#[derive(Default)]
pub struct Estimator {
    pub field: FieldView,
    cache: Vec<(Key, Hit)>,
    fighters: Vec<((Side, u8, bool), Fighter)>,
}

impl Estimator {
    pub fn new(b: &Battle) -> Estimator {
        let mut e = Estimator::default();
        e.reset(b);
        e
    }
    pub fn reset(&mut self, b: &Battle) {
        self.field = FieldView::of(b);
        self.cache.clear();
        self.fighters.clear();
    }
    pub fn fighter(&mut self, b: &Battle, w: Who, mega: bool) -> Fighter {
        let k = (w.side, w.slot, mega);
        if let Some((_, f)) = self.fighters.iter().find(|(x, _)| *x == k) {
            return *f;
        }
        let f = Fighter::of(b, w, mega);
        self.fighters.push((k, f));
        f
    }
    /// The estimate of `atk` (as its Mega form when `mega`) using `mv` on `def`.
    pub fn hit(&mut self, b: &Battle, atk: Who, mega: bool, mv: MoveId, def: Who) -> Hit {
        let key = Key { atk: (atk.side, atk.slot), mega, mv, def: (def.side, def.slot) };
        if let Some((_, h)) = self.cache.iter().find(|(k, _)| *k == key) {
            return *h;
        }
        let a = self.fighter(b, atk, mega);
        let d = self.fighter(b, def, false);
        let h = estimate(&self.field, &a, mv, &d, Some(b));
        self.cache.push((key, h));
        h
    }
    pub fn cached(&self) -> usize {
        self.cache.len()
    }
}
