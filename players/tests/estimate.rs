//! The shared estimate against sim::calc for known stats (spec test list 51-56). Each case builds the
//! HitInput by hand from Showdown's modifier values and compares the three rolls.

mod common;
use battle::*;
use common::*;
use players::*;
use cg_engine::sim::calc::*;
use cg_engine::sim::data::*;

fn vs(team: Vec<SetMsg>, opp: &str) -> T {
    let mut pv = vec![opp];
    for p in PREVIEW {
        if p != opp && pv.len() < 6 {
            pv.push(p);
        }
    }
    T::new(fill(team), &pv)
}

/// The three rolls of a hand-built hit.
fn rolls(h: HitInput) -> (u32, u32, u32) {
    (hit_damage(&HitInput { roll: ROLL_MIN, ..h }), hit_damage(&HitInput { roll: ROLL_MID, ..h }), hit_damage(&HitInput { roll: ROLL_MAX, ..h }))
}

fn input(bp: u32, attack: u32, defense: u32, stab: u32, type_mod: i32) -> HitInput {
    HitInput {
        level: 50,
        base_power: bp,
        attack,
        defense,
        spread: false,
        parental_second: false,
        weather: 4096,
        crit: false,
        crit_modifier: 0,
        roll: 0,
        stab: Some(stab),
        type_mod,
        burn: false,
        final_mod: 4096,
        quarter: false,
    }
}

fn est(t: &T, slot: usize) -> Hit {
    let b = t.bt();
    let mv = b.mon(t.own()).moves[slot].unwrap();
    Estimator::new(b).hit(b, t.own(), false, mv, t.opp())
}

#[track_caller]
fn same(h: Hit, r: (u32, u32, u32)) {
    assert_eq!((h.min, h.mid, h.max), r);
}

fn neutral(sp: &str) -> [u16; 6] {
    players::view::neutral_stats(SpeciesId::from_id(sp).unwrap())
}

const PTS: [u8; 6] = [2, 32, 0, 0, 0, 32];

fn chomp(item: Option<&str>, ability: &str) -> SetMsg {
    set_pts("garchomp", item, ability, &["earthquake", "dragonclaw", "rockslide", "ironhead"], PTS)
}

#[test]
fn t51_modifiers_against_calc() {
    let atk = |t: &T| t.bt().sets[0].stats[1] as u32;
    // STAB and type: Earthquake on Excadrill (Ground/Steel: Ground x2), STAB
    let t = vs(vec![chomp(None, "roughskin")], "excadrill");
    let d = neutral("excadrill");
    same(est(&t, 0), rolls(input(100, atk(&t), d[2] as u32, 6144, 1)));
    // not STAB: Iron Head on Dragonite (Steel neutral)
    let t = vs(vec![chomp(None, "roughskin")], "dragonite");
    let d = neutral("dragonite");
    same(est(&t, 3), rolls(input(80, atk(&t), d[2] as u32, 4096, 0)));
    // weather: Water move in rain and in sun
    let prim = || set_pts("primarina", None, "torrent", &["hydropump", "moonblast", "protect", "rest"], [0, 0, 0, 32, 0, 0]);
    let d = neutral("garchomp");
    for (w, m) in [("raindance", 6144), ("sunnyday", 2048)] {
        let mut t = vs(vec![prim()], "garchomp");
        t.e(&weather(w));
        let spa = t.bt().sets[0].stats[3] as u32;
        same(est(&t, 0), rolls(HitInput { weather: m, ..input(110, spa, d[4] as u32, 6144, 0) }));
    }
    // burn on a physical move
    let mut t = vs(vec![chomp(None, "roughskin")], "dragonite");
    t.e(&status("own", "garchomp", "brn"));
    same(est(&t, 3), rolls(HitInput { burn: true, ..input(80, atk(&t), neutral("dragonite")[2] as u32, 4096, 0) }));
    // Reflect on the defender's side
    let mut t = vs(vec![chomp(None, "roughskin")], "dragonite");
    t.e(&side_cond("opp", "reflect"));
    same(est(&t, 3), rolls(HitInput { final_mod: 2048, ..input(80, atk(&t), neutral("dragonite")[2] as u32, 4096, 0) }));
    // Life Orb
    let t = vs(vec![chomp(Some("lifeorb"), "roughskin")], "dragonite");
    same(est(&t, 3), rolls(HitInput { final_mod: 5324, ..input(80, atk(&t), neutral("dragonite")[2] as u32, 4096, 0) }));
    // Technician: Bullet Seed 25 -> 37 per hit (Grass on Dragon/Flying: two resistances)
    let t = vs(vec![set_pts("garchomp", None, "technician", &["bulletseed", "protect", "rest", "swordsdance"], PTS)], "dragonite");
    same(est(&t, 0), rolls(input(modify(25, 6144), atk(&t), neutral("dragonite")[2] as u32, 4096, -2)));
    // Adaptability: STAB 2
    let t = vs(vec![chomp(None, "adaptability")], "dragonite");
    same(est(&t, 1), rolls(input(80, atk(&t), neutral("dragonite")[2] as u32, 8192, 1)));
    // Electric Terrain boosts a grounded attacker's Electric move
    let mut t = vs(vec![set_pts("rotomwash", None, "motordrive", &["thunderbolt", "protect", "rest", "hydropump"], [0, 0, 0, 32, 0, 0])], "garchomp");
    t.e(&field("electricterrain"));
    // Rotom-Wash here has Motor Drive (not Levitate), so it is grounded; Garchomp is immune to Electric
    assert!(est(&t, 0).is_zero());
    let mut t = vs(vec![set_pts("rotomwash", None, "motordrive", &["thunderbolt", "protect", "rest", "hydropump"], [0, 0, 0, 32, 0, 0])], "dragonite");
    t.e(&field("electricterrain"));
    let spa = t.bt().sets[0].stats[3] as u32;
    same(est(&t, 0), rolls(input(modify(90, 5325), spa, neutral("dragonite")[4] as u32, 6144, 0)));
    // Choice Band is not a legal item in Champions
    assert!(ItemId::from_id("choiceband").is_none());
}

#[test]
fn t52_unknown_opposing_stats_neutral_no_points_no_item() {
    let t = vs(vec![chomp(None, "roughskin")], "corviknight");
    let b = t.bt();
    let f = Fighter::of(b, t.opp(), false);
    let base = SpeciesId::from_id("corviknight").unwrap().data().base_stats;
    assert_eq!(f.stats[0], base[0] + 75);
    for i in 1..6 {
        assert_eq!(f.stats[i], base[i] + 20);
    }
    assert!(f.item.is_none() && f.ability.is_none());
    // Iron Head on Corviknight (Steel/Flying): Steel resisted by Steel
    let h = est(&t, 3);
    same(h, rolls(input(80, b.sets[0].stats[1] as u32, (base[2] + 20) as u32, 4096, -1)));
    assert_eq!(h.def_max_hp, (base[0] + 75) as u32);
}

#[test]
fn t53_revealed_mega_changes_base_stats_and_ability() {
    let mut t = vs(vec![set_pts("lucario", None, "innerfocus", &["closecombat", "earthquake", "protect", "rest"], PTS)], "garchomp");
    let before = est(&t, 1);
    t.e(r#"{"k":"mega","side":"opp","mon":"garchomp","form":"garchompmegaz"}"#);
    let b = t.bt();
    let f = Fighter::of(b, t.opp(), false);
    assert_eq!(f.stats, neutral("garchompmegaz"));
    assert_eq!(f.ability, SpeciesId::from_id("garchompmegaz").unwrap().data().abilities.first().copied());
    // Mega Garchomp Z has Levitate: Earthquake now does nothing
    assert!(before.max > 0);
    assert!(est(&t, 1).is_zero());
    // own Mega: scored as the Mega form when the Mega variant is asked for
    let t = vs(vec![set_pts("garchomp", Some("garchompitez"), "roughskin", &["dragonclaw", "earthquake", "protect", "rest"], PTS)], "dragonite");
    let b = t.bt();
    let plain = Fighter::of(b, t.own(), false);
    let mega = Fighter::of(b, t.own(), true);
    assert_eq!(mega.form.id(), "garchompmegaz");
    let set = &b.sets[0];
    let neutral_n = nature_from_id("serious").unwrap();
    assert_eq!(mega.stats[1], stat_from_points(130, PTS[1], Stat::Atk, neutral_n));
    assert_eq!(mega.stats[0], set.stats[0]);
    assert_eq!(plain.stats, set.stats);
}

#[test]
fn t53b_own_mega_stats_without_points() {
    // stats read without nature or points: shown stat plus the change in base stat
    let mut s = set_pts("garchomp", Some("garchompitez"), "roughskin", &["dragonclaw", "earthquake", "protect", "rest"], PTS);
    s.nature = None;
    s.points = None;
    let t = vs(vec![s.clone()], "dragonite");
    let mega = Fighter::of(t.bt(), t.own(), true);
    let mb = SpeciesId::from_id("garchompmegaz").unwrap().data().base_stats;
    let bb = SpeciesId::from_id("garchomp").unwrap().data().base_stats;
    for i in 1..6 {
        assert_eq!(mega.stats[i] as i32, s.stats[i] + mb[i] as i32 - bb[i] as i32);
    }
}

#[test]
fn t54_used_up_item_has_no_effect() {
    let prim = || set_pts("primarina", None, "torrent", &["moonblast", "hydropump", "protect", "rest"], [0, 0, 0, 32, 0, 0]);
    // Moonblast on Garchomp: Fairy x2; a shown Roseli Berry halves it, a used one does not
    let base = est(&vs(vec![prim()], "garchomp"), 0);
    let mut t = vs(vec![prim()], "garchomp");
    t.e(r#"{"k":"item","side":"opp","mon":"garchomp","item":"roseliberry","what":"found"}"#);
    let b = t.bt();
    assert!(matches!(b.mon(t.opp()).item, ItemState::Held(_)));
    assert!(est(&t, 0).mid < base.mid);
    t.e(r#"{"k":"item","side":"opp","mon":"garchomp","item":"roseliberry","what":"activated"}"#);
    assert!(matches!(t.bt().mon(t.opp()).item, ItemState::Gone(_)));
    assert_eq!(est(&t, 0), base);
}

#[test]
fn t55_status_and_immune_moves_are_zero() {
    let t = vs(vec![set_pts("garchomp", None, "roughskin", &["swordsdance", "earthquake", "dragonclaw", "rest"], PTS)], "corviknight");
    assert!(est(&t, 0).is_zero());
    assert!(est(&t, 1).is_zero());
    assert!(!est(&t, 2).is_zero());
    assert_eq!(est(&t, 0).expected(), 0.0);
}

#[test]
fn t56_estimate_takes_only_battle_derived_inputs() {
    // the signature: field and fighters built from a Battle, never a State
    let _f: fn(&FieldView, &Fighter, MoveId, &Fighter, Option<&Battle>) -> Hit = estimate;
    let _g: fn(&Battle) -> Estimator = Estimator::new;
}

#[test]
fn cache_is_per_decision_and_consistent() {
    let t = vs(vec![chomp(None, "roughskin")], "dragonite");
    let b = t.bt();
    let mut e = Estimator::new(b);
    let mv = MoveId::from_id("dragonclaw").unwrap();
    let h1 = e.hit(b, t.own(), false, mv, t.opp());
    let h2 = e.hit(b, t.own(), false, mv, t.opp());
    assert_eq!(h1, h2);
    assert_eq!(e.cached(), 1);
    e.reset(b);
    assert_eq!(e.cached(), 0);
}

#[test]
fn foe_attacking_us_uses_our_exact_stats_and_hp() {
    let mut t = vs(vec![chomp(None, "roughskin")], "dragonite");
    t.e(&mv("opp", "dragonite", "dragonclaw"));
    let b = t.bt();
    let h = Estimator::new(b).hit(b, t.opp(), false, MoveId::from_id("dragonclaw").unwrap(), t.own());
    let atk = neutral("dragonite")[1] as u32;
    same(h, rolls(input(80, atk, b.sets[0].stats[2] as u32, 6144, 1)));
    assert_eq!(h.def_max_hp, b.sets[0].stats[0] as u32);
}
