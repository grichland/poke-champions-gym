//! The heuristic's matchup score (`pcsa/players/heuristic.py` `HeuristicPlayer.matchup`) and the ranking
//! score it adds (`pcsa/players/tactics.py` `attack_score`), over Fighters built by CG's fixed rule.

use crate::estimate::{accuracy, Estimator};
use crate::threat::{known_moves, type_mult};
use crate::view::{speed, Fighter, FieldView};
use battle::{Battle, Side, Who, SIDE_CONDS};
use cg_engine::sim::data::*;

/// `config/players/heuristic.yaml` values the matchup reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeuristicParams {
    /// bonus for the faster base speed (and again when the effective order disagrees with it)
    pub speed_tier: f32,
    /// weight of each side's HP fraction
    pub hp_fraction: f32,
    /// STAB in the ranking score
    pub stab: f32,
}

impl Default for HeuristicParams {
    fn default() -> Self {
        HeuristicParams { speed_tier: 0.42, hp_fraction: 0.67, stab: 1.54 }
    }
}

/// Divides the ranking score in the matchup.
pub const SCORE_SCALE: f32 = 200.0;
/// Weight of the entry-hazard fraction.
pub const HAZARD_WEIGHT: f32 = 4.0;
/// Penalty when hazards would KO the member on entry.
pub const HAZARD_KO: f32 = 100.0;

fn stage_mult(f: &Fighter, s: Stat) -> f32 {
    let v = f.boosts[s.boost_index()] as f32;
    if v >= 0.0 {
        (2.0 + v) / 2.0
    } else {
        2.0 / (2.0 - v)
    }
}

/// poke-env's expected hits.
fn expected_hits(d: &MoveData) -> f32 {
    if matches!(d.id, "triplekick" | "tripleaxel") {
        return 1.0 + 2.0 * 0.9 + 3.0 * 0.81;
    }
    let (lo, hi) = d.multihit;
    if hi == 0 || lo == hi {
        return lo.max(1) as f32;
    }
    if (lo, hi) == (2, 5) {
        return 5.0 / 3.0 + 9.0 / 6.0;
    }
    (lo as f32 + hi as f32) / 2.0
}

/// `effective_power`: weight, speed-ratio and Facade powers; otherwise the table's.
fn power(atk: &Fighter, def: &Fighter, d: &MoveData) -> f32 {
    let kg = |f: &Fighter| f.form.data().weight_hg as f32 / 10.0;
    match d.id {
        "grassknot" | "lowkick" => {
            let w = kg(def);
            [(10.0, 20.0), (25.0, 40.0), (50.0, 60.0), (100.0, 80.0), (200.0, 100.0)].iter().find(|(l, _)| w < *l).map_or(120.0, |x| x.1)
        }
        "heavyslam" | "heatcrash" => {
            let r = kg(atk) / kg(def).max(0.1);
            if r >= 5.0 {
                120.0
            } else if r >= 4.0 {
                100.0
            } else if r >= 3.0 {
                80.0
            } else if r >= 2.0 {
                60.0
            } else {
                40.0
            }
        }
        "facade" if atk.status != Status::None => d.base_power as f32 * 2.0,
        "electroball" | "gyroball" => {
            let ours = atk.stats[Stat::Spe as usize] as f32;
            let theirs = (def.stats[Stat::Spe as usize] as f32).max(1.0);
            if d.id == "gyroball" {
                return (25.0 * theirs / ours.max(1.0) + 1.0).min(150.0);
            }
            let r = ours / theirs;
            if r >= 4.0 {
                150.0
            } else if r >= 3.0 {
                120.0
            } else if r >= 2.0 {
                80.0
            } else if r >= 1.0 {
                60.0
            } else {
                40.0
            }
        }
        _ => d.base_power as f32,
    }
}

/// `attack_score`: power x STAB x attack/defense ratio x accuracy x hits x type multiplier (unitless).
pub fn attack_score(field: &FieldView, atk: &Fighter, mv: MoveId, def: &Fighter, stab: f32) -> f32 {
    let d = mv.data();
    let physical = d.category == Category::Physical;
    let (mut a_stat, d_stat) = if physical { (Stat::Atk, Stat::Def) } else { (Stat::Spa, Stat::Spd) };
    if d.id == "bodypress" {
        a_stat = Stat::Def;
    }
    let foul = d.id == "foulplay";
    let source = if foul { def } else { atk };
    let attack = if foul { def.stats[Stat::Atk as usize] } else { atk.stats[a_stat as usize] } as f32;
    let ratio = attack * stage_mult(source, a_stat) / (def.stats[d_stat as usize] as f32 * stage_mult(def, d_stat)).max(1.0);
    let mut mult = type_mult(d.ty, &def.types);
    let immune = match d.ty {
        Type::Ground => def.ab() == "levitate" || def.it() == "airballoon",
        Type::Electric => matches!(def.ab(), "voltabsorb" | "lightningrod" | "motordrive"),
        Type::Water => matches!(def.ab(), "waterabsorb" | "stormdrain" | "dryskin"),
        _ => false,
    };
    if immune {
        mult = 0.0;
    }
    let s = if atk.has_type(d.ty) { stab } else { 1.0 };
    power(atk, def, d) * s * ratio * accuracy(field, atk, mv, def) * expected_hits(d) * mult
}

/// `entry_hazard_fraction`: Stealth Rock and Spikes on our side, for a member coming in.
pub fn entry_hazard_fraction(b: &Battle, mon: &Fighter) -> f32 {
    if mon.it() == "heavydutyboots" || mon.ab() == "magicguard" {
        return 0.0;
    }
    let cond = |id: &str| SIDE_CONDS.iter().position(|s| *s == id).and_then(|i| b.side(Side::Own).conds[i]);
    let mut dmg = if cond("stealthrock").is_some() { 0.125 * type_mult(Type::Rock, &mon.types) } else { 0.0 };
    let airborne = mon.has_type(Type::Flying) || mon.ab() == "levitate" || mon.it() == "airballoon";
    if let (false, Some(sp)) = (airborne, cond("spikes")) {
        let layers = sp.layers.clamp(1, 3) as usize;
        dmg += [1.0 / 8.0, 1.0 / 6.0, 1.0 / 4.0][layers - 1];
    }
    dmg
}

fn best_type_mult(atk: &Fighter, def: &Fighter) -> f32 {
    atk.types.iter().filter(|t| t.is_real()).map(|&t| type_mult(t, &def.types)).fold(f32::NEG_INFINITY, f32::max)
}

/// Best ranking score of a Pokémon's damaging known moves on a defender, if it has any.
fn best_score(b: &Battle, field: &FieldView, atk: Who, fa: &Fighter, fd: &Fighter, stab: f32) -> Option<f32> {
    known_moves(b, atk)
        .iter()
        .flatten()
        .filter(|m| m.data().category != Category::Status)
        .map(|&m| attack_score(field, fa, m, fd, stab))
        .fold(None, |acc: Option<f32>, s| Some(acc.map_or(s, |a| a.max(s))))
}

/// The matchup of our `mon` against their `opp` (higher is better for us).
pub fn matchup(b: &Battle, est: &mut Estimator, mon: Who, opp: Who, p: &HeuristicParams) -> f32 {
    let field = est.field;
    let m = est.fighter(b, mon, false);
    let o = est.fighter(b, opp, false);
    let mut score = best_type_mult(&m, &o) - best_type_mult(&o, &m);
    let (ms, os) = (m.form.data().base_stats[5], o.form.data().base_stats[5]);
    if ms != os {
        score += if ms > os { p.speed_tier } else { -p.speed_tier };
    }
    score += p.hp_fraction * (m.hp / 100.0 - o.hp / 100.0);
    let (ours, theirs) = (speed(&m, &field), speed(&o, &field));
    if (ours > theirs) != (ms > os) {
        score += p.speed_tier * if ours > theirs { 1.0 } else { -1.0 };
    }
    if let Some(s) = best_score(b, &field, mon, &m, &o, p.stab) {
        score += s / SCORE_SCALE;
    }
    if let Some(s) = best_score(b, &field, opp, &o, &m, p.stab) {
        score -= s / SCORE_SCALE;
    }
    let hz = entry_hazard_fraction(b, &m);
    score -= hz * HAZARD_WEIGHT;
    if hz >= m.hp / 100.0 {
        score -= HAZARD_KO;
    }
    score
}
