//! Showdown's integer arithmetic (docs/investigation/audit/mechanics-formulas.md part A): stats from stat
//! points, stat stages, the 4096-based modifiers, and the damage formula for one hit with a given roll.

use super::data::*;

/// Showdown's trunc: `x >>> 0`, a wrap to 32 bits.
#[inline]
pub fn tr(x: u64) -> u64 {
    x & 0xFFFF_FFFF
}

/// A modifier in 4096ths from num/den (Showdown: trunc(num * 4096 / den)).
#[inline]
pub fn modifier(num: u32, den: u32) -> u32 {
    (num as u64 * 4096 / den as u64) as u32
}

/// modify(value, m/4096): multiply and round half down.
#[inline]
pub fn modify(value: u32, m4096: u32) -> u32 {
    tr((tr(value as u64 * m4096 as u64) + 2047) / 4096) as u32
}

/// Two chained modifiers (chainModify): round half up.
#[inline]
pub fn chain(prev: u32, next: u32) -> u32 {
    ((prev as u64 * next as u64 + 2048) >> 12) as u32
}

/// Champions stat from base stat and stat points (data/mods/champions/scripts.ts statModify, no level clause).
pub fn stat_from_points(base: u16, points: u8, stat: Stat, nature: u8) -> u16 {
    let b = base as u32;
    let sp = points as u32;
    if stat == Stat::Hp {
        return (b + sp + 75) as u16;
    }
    let mut v = b + sp + 20;
    let n = &NATURES[nature as usize];
    if n.plus == Some(stat) {
        v = ((v * 110) & 0xFFFF) / 100;
    } else if n.minus == Some(stat) {
        v = ((v * 90) & 0xFFFF) / 100;
    }
    v as u16
}

/// Stage multiplier: floor(stat * [1, 1.5, ..., 4][b]) or floor(stat / that) for negative stages.
#[inline]
pub fn apply_stage(stat: u32, boost: i8) -> u32 {
    let b = boost.clamp(-6, 6) as i32;
    if b >= 0 {
        stat * (2 + b as u32) / 2
    } else {
        stat * 2 / (2 + (-b) as u32)
    }
}

/// Accuracy after the combined accuracy/evasion stage (hitStepAccuracy).
#[inline]
pub fn accuracy_with_stage(acc: u32, stage: i32) -> u32 {
    if stage > 0 {
        acc * (3 + stage as u32) / 3
    } else if stage < 0 {
        acc * 3 / (3 + (-stage) as u32)
    } else {
        acc
    }
}

/// Inputs to one hit's damage after the attacking and defending stats are known.
#[derive(Clone, Copy, Debug)]
pub struct HitInput {
    pub level: u32,
    pub base_power: u32,
    pub attack: u32,
    pub defense: u32,
    pub spread: bool,
    pub parental_second: bool,
    /// WeatherModifyDamage chain, 4096ths
    pub weather: u32,
    pub crit: bool,
    /// 0 = Showdown's default 1.5
    pub crit_modifier: u16,
    /// 0..15: the damage factor is 100 - roll percent
    pub roll: u32,
    /// None for a typeless move (no STAB step); else the STAB modifier in 4096ths
    pub stab: Option<u32>,
    pub type_mod: i32,
    pub burn: bool,
    /// ModifyDamage chain, 4096ths
    pub final_mod: u32,
    /// moveHitData bypassProtect: a quarter after the final modifier
    pub quarter: bool,
}

/// tr(tr(tr(22 * BP * A) / D) / 50) at level 50 (getDamage).
#[inline]
pub fn base_damage(level: u32, bp: u32, atk: u32, def: u32) -> u32 {
    let l = (2 * level / 5 + 2) as u64;
    let x = tr(l * bp as u64 * atk as u64);
    tr(tr(x / def.max(1) as u64) / 50) as u32
}

/// modifyDamage (data/mods/champions/scripts.ts): the steps after the base damage, each with its rounding.
pub fn hit_damage(h: &HitInput) -> u32 {
    let mut d = base_damage(h.level, h.base_power, h.attack, h.defense) as u64 + 2;
    if h.spread {
        d = modify(d as u32, 3072) as u64;
    } else if h.parental_second {
        d = modify(d as u32, 1024) as u64;
    }
    d = modify(d as u32, h.weather) as u64;
    if h.crit {
        d = if h.crit_modifier == 0 { tr(d * 3 / 2) } else { tr(d * h.crit_modifier as u64 / 4096) };
    }
    d = tr(tr(d * (100 - h.roll as u64)) / 100);
    if let Some(stab) = h.stab {
        d = modify(d as u32, stab) as u64;
    }
    let tm = h.type_mod.clamp(-6, 6);
    if tm > 0 {
        for _ in 0..tm {
            d *= 2;
        }
    } else {
        for _ in 0..(-tm) {
            d = tr(d / 2);
        }
    }
    if h.burn {
        d = modify(d as u32, 2048) as u64;
    }
    d = modify(d as u32, h.final_mod) as u64;
    if h.quarter {
        d = modify(d as u32, 1024) as u64;
    }
    if d == 0 {
        return 1;
    }
    (tr(d) % 65536) as u32
}

/// Math.round(a * n / d) for non-negative integers.
#[inline]
pub fn round_frac(a: u32, n: u32, d: u32) -> u32 {
    ((2 * a as u64 * n as u64 + d as u64) / (2 * d as u64)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers() {
        assert_eq!(modifier(3, 2), 6144);
        assert_eq!(modifier(13, 10), 5324);
        assert_eq!(modify(100, 6144), 150);
        assert_eq!(modify(101, 6144), 151); // 151.5 rounds half down
        assert_eq!(modify(3, 2048), 1);
        assert_eq!(chain(6144, 6144), 9216);
        assert_eq!(round_frac(3, 1, 2), 2);
        assert_eq!(round_frac(50, 33, 100), 17);
        assert_eq!(round_frac(49, 33, 100), 16);
    }

    #[test]
    fn stages() {
        assert_eq!(apply_stage(100, 1), 150);
        assert_eq!(apply_stage(100, -1), 66);
        assert_eq!(apply_stage(100, 6), 400);
        assert_eq!(apply_stage(101, -6), 25);
        assert_eq!(accuracy_with_stage(100, -1), 75);
        assert_eq!(accuracy_with_stage(90, 1), 120);
    }

    #[test]
    fn stats() {
        let neutral = super::super::data::nature_from_id("serious").unwrap();
        let adamant = super::super::data::nature_from_id("adamant").unwrap();
        assert_eq!(stat_from_points(108, 32, Stat::Hp, neutral), 215);
        assert_eq!(stat_from_points(130, 32, Stat::Atk, adamant), 200);
        assert_eq!(stat_from_points(80, 0, Stat::Spa, adamant), 90);
    }
}
