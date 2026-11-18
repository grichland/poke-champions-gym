//! The 87 volatile bits of F3-5, in the order of pcsa's encoder (`rl_training/features_rows.py`, read to
//! learn the list), each matched against what the Battle holds for an active Pokémon.

use battle::{Active, Battle, Lock, VolDetail, Who};
use cg_engine::sim::data::*;

/// How a bit is read from the Battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// a volatile of this condition id
    Cond(&'static str),
    /// a volatile of this condition id with this number as its detail (no detail counts as `default`)
    Count(&'static str, i16, i16),
    /// a partial trap by this move
    Trap(&'static str),
    /// a locked move other than Uproar
    Locked,
    /// Uproar, as a volatile or a lock
    Uproar,
    /// a `types` row changed its types
    TypeChange,
    /// its current form is this species
    Form(&'static str),
    /// CG cannot know it: the move or effect is not in Champions, or the log has no row for it
    Never,
}

pub const N_VOLS: usize = 87;

pub static VOLS: [(&str, Rule); N_VOLS] = [
    ("confusion", Rule::Cond("confusion")),
    ("encore", Rule::Cond("encore")),
    ("taunt", Rule::Cond("taunt")),
    ("torment", Rule::Cond("torment")),
    ("disable", Rule::Cond("disable")),
    ("substitute", Rule::Cond("substitute")),
    ("leechseed", Rule::Cond("leechseed")),
    ("yawn", Rule::Cond("yawn")),
    ("perish0", Rule::Count("perishsong", 0, 3)),
    ("perish1", Rule::Count("perishsong", 1, 3)),
    ("perish2", Rule::Count("perishsong", 2, 3)),
    ("perish3", Rule::Count("perishsong", 3, 3)),
    ("attract", Rule::Cond("attract")),
    ("curse", Rule::Cond("curse")),
    ("nightmare", Rule::Never),
    ("destinybond", Rule::Cond("destinybond")),
    ("protect", Rule::Cond("protect")),
    ("endure", Rule::Cond("endure")),
    ("flinch", Rule::Cond("flinch")),
    ("focusenergy", Rule::Cond("focusenergy")),
    ("laserfocus", Rule::Never),
    ("charge", Rule::Cond("charge")),
    ("magnetrise", Rule::Cond("magnetrise")),
    ("telekinesis", Rule::Never),
    ("smackdown", Rule::Cond("smackdown")),
    ("ingrain", Rule::Cond("ingrain")),
    ("aquaring", Rule::Cond("aquaring")),
    ("healblock", Rule::Cond("healblock")),
    ("embargo", Rule::Never),
    ("gastroacid", Rule::Cond("gastroacid")),
    ("throatchop", Rule::Cond("throatchop")),
    ("saltcure", Rule::Cond("saltcure")),
    ("syrupbomb", Rule::Cond("syrupbomb")),
    ("tarshot", Rule::Never),
    ("octolock", Rule::Cond("octolock")),
    ("noretreat", Rule::Cond("noretreat")),
    ("trapped", Rule::Cond("trapped")),
    ("partiallytrapped", Rule::Cond("partiallytrapped")),
    ("bind", Rule::Trap("bind")),
    ("wrap", Rule::Trap("wrap")),
    ("firespin", Rule::Trap("firespin")),
    ("whirlpool", Rule::Trap("whirlpool")),
    ("sandtomb", Rule::Trap("sandtomb")),
    ("infestation", Rule::Trap("infestation")),
    ("magmastorm", Rule::Never),
    ("clamp", Rule::Never),
    ("snaptrap", Rule::Trap("snaptrap")),
    ("thundercage", Rule::Never),
    ("lockedmove", Rule::Locked),
    ("uproar", Rule::Uproar),
    ("stockpile1", Rule::Count("stockpile", 1, 1)),
    ("stockpile2", Rule::Count("stockpile", 2, 1)),
    ("stockpile3", Rule::Count("stockpile", 3, 1)),
    ("minimize", Rule::Cond("minimize")),
    ("defensecurl", Rule::Never),
    ("roost", Rule::Cond("roost")),
    ("powertrick", Rule::Cond("powertrick")),
    ("typechange", Rule::TypeChange),
    ("typeadd", Rule::Never),
    ("disguise", Rule::Form("mimikyubusted")),
    ("iceface", Rule::Form("eiscuenoice")),
    ("flashfire", Rule::Cond("flashfire")),
    ("protosynthesis", Rule::Cond("protosynthesis")),
    ("protosynthesisatk", Rule::Never),
    ("protosynthesisdef", Rule::Never),
    ("protosynthesisspa", Rule::Never),
    ("protosynthesisspd", Rule::Never),
    ("protosynthesisspe", Rule::Never),
    ("quarkdrive", Rule::Cond("quarkdrive")),
    ("quarkdriveatk", Rule::Never),
    ("quarkdrivedef", Rule::Never),
    ("quarkdrivespa", Rule::Never),
    ("quarkdrivespd", Rule::Never),
    ("quarkdrivespe", Rule::Never),
    ("slowstart", Rule::Never),
    ("illusion", Rule::Never),
    ("kingsshield", Rule::Cond("kingsshield")),
    ("spikyshield", Rule::Cond("spikyshield")),
    ("banefulbunker", Rule::Cond("banefulbunker")),
    ("silktrap", Rule::Never),
    ("burningbulwark", Rule::Never),
    ("obstruct", Rule::Never),
    ("glaiverush", Rule::Cond("glaiverush")),
    ("followme", Rule::Cond("followme")),
    ("ragepowder", Rule::Cond("ragepowder")),
    ("magiccoat", Rule::Never),
    ("imprison", Rule::Cond("imprison")),
];

/// Bit index per condition id of CG's table (`u8::MAX`: none), for the plain `Cond` rules.
struct Index {
    by_cond: [u8; CONDITIONS.len()],
    perish: Cond,
    stockpile: Cond,
    trap: Cond,
    /// (move, bit) for the partial-trap rules
    traps: [(MoveId, u8); 7],
    mimikyu: Option<SpeciesId>,
    eiscue: Option<SpeciesId>,
}

fn index() -> &'static Index {
    static I: std::sync::OnceLock<Index> = std::sync::OnceLock::new();
    I.get_or_init(|| {
        let mut x = Index {
            by_cond: [u8::MAX; CONDITIONS.len()],
            perish: Cond::from_id("perishsong").unwrap_or_default(),
            stockpile: Cond::from_id("stockpile").unwrap_or_default(),
            trap: Cond::from_id("partiallytrapped").unwrap_or_default(),
            traps: [(MoveId::NONE, 0); 7],
            mimikyu: SpeciesId::from_id("mimikyubusted"),
            eiscue: SpeciesId::from_id("eiscuenoice"),
        };
        let mut t = 0;
        for (i, (_, r)) in VOLS.iter().enumerate() {
            match r {
                Rule::Cond(id) => {
                    if let Some(c) = Cond::from_id(id) {
                        x.by_cond[c.0 as usize] = i as u8;
                    }
                }
                Rule::Trap(id) => {
                    if let Some(m) = MoveId::from_id(id) {
                        x.traps[t] = (m, i as u8);
                        t += 1;
                    }
                }
                _ => {}
            }
        }
        x
    })
}

#[cfg(test)]
fn bit_of(name: &str) -> usize {
    VOLS.iter().position(|(n, _)| *n == name).unwrap()
}

/// Sets the volatile bits of an active Pokémon into `out` (87 floats, already zero).
pub fn write(b: &Battle, w: Who, a: &Active, out: &mut [f32]) {
    let ix = index();
    for v in a.vols.iter().flatten() {
        let c = v.cond;
        let k = ix.by_cond[c.0 as usize];
        if k != u8::MAX {
            out[k as usize] = 1.0;
        }
        if c == ix.perish {
            let n = match v.detail {
                VolDetail::Num(n) => n.clamp(0, 3),
                _ => 3,
            };
            out[8 + n as usize] = 1.0;
        } else if c == ix.stockpile {
            let n = match v.detail {
                VolDetail::Num(n) => n.clamp(1, 3),
                _ => 1,
            };
            out[49 + n as usize] = 1.0;
        } else if c == ix.trap {
            if let VolDetail::Move(m) = v.detail {
                if let Some(&(_, k)) = ix.traps.iter().find(|(x, _)| *x == m && !x.is_none()) {
                    out[k as usize] = 1.0;
                }
            }
        } else if c.id() == "uproar" {
            out[49] = 1.0;
        } else if c.id() == "lockedmove" {
            out[48] = 1.0;
        }
    }
    if let Lock::Locked { mv, .. } = a.lock {
        out[if mv.id() == "uproar" { 49 } else { 48 }] = 1.0;
    }
    if a.types.is_some() {
        out[57] = 1.0;
    }
    let form = b.mon(w).form;
    if form.is_some() && form == ix.mimikyu {
        out[59] = 1.0;
    }
    if form.is_some() && form == ix.eiscue {
        out[60] = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_bits_match_names() {
        assert_eq!(bit_of("perish0"), 8);
        assert_eq!(bit_of("lockedmove"), 48);
        assert_eq!(bit_of("uproar"), 49);
        assert_eq!(bit_of("stockpile1"), 50);
        assert_eq!(bit_of("typechange"), 57);
        assert_eq!(bit_of("disguise"), 59);
        assert_eq!(bit_of("iceface"), 60);
        let mut seen = std::collections::HashSet::new();
        assert!(VOLS.iter().all(|(n, _)| seen.insert(*n)));
    }

    #[test]
    fn every_named_condition_and_move_exists() {
        for (n, r) in VOLS.iter() {
            match r {
                Rule::Cond(id) | Rule::Count(id, ..) => assert!(Cond::from_id(id).is_some(), "{} {}", n, id),
                Rule::Trap(id) => assert!(MoveId::from_id(id).is_some(), "{} {}", n, id),
                Rule::Form(id) => assert!(SpeciesId::from_id(id).is_some(), "{} {}", n, id),
                _ => {}
            }
        }
    }
}
