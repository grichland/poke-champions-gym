//! PS-Q4 / PS-B4 / PS-B6: a status move aimed at the foe that the Battle shows cannot work.

use crate::view::{grounded, vf, Fighter, FieldView};
use battle::Battle;
use cg_engine::sim::data::*;

pub fn targets_foe(t: Target) -> bool {
    matches!(t, Target::Normal | Target::Any | Target::AdjacentFoe | Target::AllAdjacentFoes | Target::AllAdjacent | Target::RandomNormal)
}

/// Why `mv` cannot work on `foe` (None: it may). `repeat`: also refuse a volatile the foe already has.
pub fn cannot_work(b: &Battle, field: &FieldView, user: &Fighter, mv: MoveId, foe: &Fighter, repeat: bool) -> Option<&'static str> {
    let d = mv.data();
    if d.category != Category::Status || !targets_foe(d.target) {
        return None;
    }
    let ab = foe.ab();
    let infiltrator = user.ab() == "infiltrator";
    if foe.vols & vf::SUBSTITUTE != 0 && !d.has_flag(flag::BYPASSSUB) && !infiltrator {
        return Some("Substitute");
    }
    if ab == "goodasgold" {
        return Some("Good as Gold");
    }
    if ab == "magicbounce" && d.has_flag(flag::REFLECTABLE) {
        return Some("Magic Bounce");
    }
    if user.ab() == "prankster" && foe.has_type(Type::Dark) {
        return Some("Prankster into Dark");
    }
    if d.has_flag(flag::POWDER) && (foe.has_type(Type::Grass) || ab == "overcoat" || foe.it() == "safetygoggles") {
        return Some("powder");
    }
    if !d.ignore_immunity && foe.types.iter().any(|&t| !type_affects(d.ty, t)) {
        return Some("type immunity");
    }
    let absorbed = match ab {
        "voltabsorb" | "lightningrod" | "motordrive" => d.ty == Type::Electric,
        "sapsipper" => d.ty == Type::Grass,
        "flashfire" | "wellbakedbody" => d.ty == Type::Fire,
        "waterabsorb" | "stormdrain" | "dryskin" => d.ty == Type::Water,
        _ => false,
    };
    if absorbed {
        return Some("absorbing ability");
    }
    let safeguard = field.screens[foe.side.idx()][4] && !infiltrator;
    let on_ground = grounded(foe, field);
    let v = d.volatile_status;
    let st = if v == conds::YAWN { Status::Slp } else { d.status };
    if st != Status::None {
        if foe.status != Status::None {
            return Some("already statused");
        }
        if v == conds::YAWN && foe.vols & vf::YAWN != 0 {
            return Some("already drowsy");
        }
        if matches!(ab, "comatose" | "purifyingsalt") || (ab == "leafguard" && field.sun()) {
            return Some("status-blocking ability");
        }
        if safeguard {
            return Some("Safeguard");
        }
        if on_ground && field.terrain_is("mistyterrain") {
            return Some("Misty Terrain");
        }
        let blocked = match st {
            Status::Slp => matches!(ab, "insomnia" | "vitalspirit" | "sweetveil") || (on_ground && field.terrain_is("electricterrain")),
            Status::Par => foe.has_type(Type::Electric) || ab == "limber",
            Status::Brn => foe.has_type(Type::Fire) || matches!(ab, "waterveil" | "waterbubble" | "thermalexchange"),
            Status::Psn | Status::Tox => {
                ((foe.has_type(Type::Poison) || foe.has_type(Type::Steel)) && user.ab() != "corrosion") || matches!(ab, "immunity" | "pastelveil")
            }
            Status::Frz => foe.has_type(Type::Ice) || ab == "magmaarmor",
            Status::None => false,
        };
        if blocked {
            return Some("status immunity");
        }
    }
    if !v.is_none() && v != conds::YAWN {
        let fa = &b.side(foe.side).active;
        if v == conds::CONFUSION && (ab == "owntempo" || safeguard || (on_ground && field.terrain_is("mistyterrain"))) {
            return Some("confusion blocked");
        }
        if (v == conds::ATTRACT || v == conds::TAUNT) && ab == "oblivious" {
            return Some("Oblivious");
        }
        if ab == "aromaveil" && [conds::ATTRACT, conds::TAUNT, conds::ENCORE, conds::DISABLE, conds::TORMENT, conds::HEALBLOCK].contains(&v) {
            return Some("Aroma Veil");
        }
        if (v == conds::ENCORE || v == conds::DISABLE) && foe.active && fa.last_move.is_none() {
            return Some("no move to lock");
        }
        if v == conds::LEECHSEED && foe.has_type(Type::Grass) {
            return Some("Grass type");
        }
        if repeat && foe.active && fa.vols.iter().flatten().any(|x| x.cond == v) {
            return Some("already has it");
        }
    }
    if let Some(bo) = d.boosts {
        let lowers: Vec<usize> = (0..7).filter(|&i| bo[i] < 0).collect();
        if !lowers.is_empty() && bo.iter().all(|&x| x <= 0) {
            if matches!(ab, "clearbody" | "whitesmoke" | "fullmetalbody") {
                return Some("stat-drop blocking ability");
            }
            if foe.active && lowers.iter().all(|&i| foe.boosts[i] <= -6) {
                return Some("stats at -6");
            }
        }
    }
    None
}
