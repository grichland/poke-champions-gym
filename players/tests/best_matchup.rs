//! Best-matchup max damage: BM-1 (switch out of a bad matchup), BM-2 (back-to-back rule), the fallback to
//! max damage, and the matchup score's parts.

mod common;
use battle::*;
use common::*;
use players::best_matchup::blocked;
use players::matchup::{attack_score, entry_hazard_fraction, matchup, HeuristicParams};
use players::*;

const P: &str = "best_matchup_max_damage";

fn scizor() -> SetMsg {
    set("scizor", None, "technician", &["bulletpunch", "uturn", "swordsdance", "roost"])
}
fn ttar() -> SetMsg {
    set("tyranitar", None, "sandstream", &["stoneedge", "crunch", "earthquake", "protect"])
}
fn lax() -> SetMsg {
    set("snorlax", None, "thickfat", &["bodyslam", "crunch", "protect", "rest"])
}
const PV: [&str; 6] = ["charizard", "gengar", "corviknight", "rotomwash", "dragonite", "garchomp"];
/// Gengar leads, so the opponent can switch into Charizard.
const PV2: [&str; 6] = ["gengar", "charizard", "corviknight", "rotomwash", "dragonite", "garchomp"];

fn score(t: &T, w: Who) -> f32 {
    let b = t.bt();
    matchup(b, &mut Estimator::new(b), w, t.opp(), &HeuristicParams::default())
}

#[test]
fn bm1_switches_out_of_a_really_bad_matchup() {
    let t = T::new(vec![scizor(), ttar(), lax()], &PV);
    let here = score(&t, t.own());
    let bench = score(&t, Who { side: Side::Own, slot: 1 });
    assert!(here < -2.0 && bench > 1.0, "{} {}", here, bench);
    let (a, fired) = t.fired(P, ReqKind::Turn);
    assert_eq!(a, 9, "{:?}", fired);
    assert_eq!(fired, vec!["switch"]);
    // max damage never switches here
    assert!(t.md() < 8);
}

#[test]
fn bm1_no_switch_in_a_fair_matchup() {
    let t = T::new(vec![ttar(), scizor(), lax()], &PV);
    assert!(score(&t, t.own()) >= -2.0);
    let (a, fired) = t.fired(P, ReqKind::Turn);
    assert_eq!(a, t.md());
    assert_eq!(fired, vec!["max_damage"]);
}

#[test]
fn bm1_no_switch_when_no_bench_member_is_good_enough() {
    // Scizor with two Steel-weak teammates of its own kind
    let t = T::new(vec![scizor(), set("kingambit", None, "defiant", &["ironhead", "kowtowcleave", "suckerpunch", "swordsdance"]), lax()], &PV);
    assert!(score(&t, t.own()) < -2.0);
    for s in 1..3 {
        assert!(score(&t, Who { side: Side::Own, slot: s }) <= 1.0);
    }
    assert!(t.play(P, 1) < 8);
}

#[test]
fn bm1_needs_a_legal_switch() {
    let mut t = T::new(vec![scizor(), ttar(), lax()], &PV);
    t.e(&vol("own", "scizor", "trapped"));
    assert!(t.play(P, 1) < 8);
}

/// Snorlax leads, then switches to Scizor on turn 1 (the opponent attacks): Scizor faces Charizard on turn 2.
fn after_own_switch(opp_switches: bool) -> T {
    let mut t = T::new(vec![lax(), ttar(), scizor()], if opp_switches { &PV2 } else { &PV });
    t.switch("own", "snorlax", "scizor");
    if opp_switches {
        t.switch("opp", "gengar", "charizard");
    } else {
        t.e(&mv("opp", "charizard", "airslash"));
    }
    t.turn(2);
    t
}

#[test]
fn bm2_back_to_back_blocked_when_the_opponent_stayed() {
    let t = after_own_switch(false);
    assert_eq!(blocked(t.bt(), 2), Some("switched last turn and the opponent did not"));
    let (a, fired) = t.fired(P, ReqKind::Turn);
    assert!(a < 8);
    assert_eq!(fired, vec!["switch_blocked", "max_damage"]);
}

#[test]
fn bm2_back_to_back_allowed_when_the_opponent_switched_too_but_not_a_third_time() {
    let mut t = T::new(vec![lax(), ttar(), scizor()], &PV2);
    // turn 1: both switch
    t.switch("own", "snorlax", "scizor");
    t.switch("opp", "gengar", "charizard");
    t.turn(2);
    // turn 2: both switch again
    t.switch("own", "scizor", "snorlax");
    t.switch("opp", "charizard", "gengar");
    t.turn(3);
    assert_eq!(t.bt().track.sides[0].switch_run, 2);
    assert_eq!(blocked(t.bt(), 2), Some("already switched twice in a row"));
    // one switch then the opponent's switch: allowed
    let t = after_own_switch(true);
    assert_eq!(t.bt().track.sides[0].switch_run, 1);
    assert_eq!(blocked(t.bt(), 2), None);
    assert_eq!(t.play(P, 1), 9);
}

#[test]
fn forced_switch_is_max_damage() {
    let mut t = T::new(vec![scizor(), ttar(), lax()], &PV);
    t.e(r#"{"k":"faint","side":"own","mon":"scizor"}"#);
    assert_eq!(t.play_kind(P, ReqKind::Replace, 1), t.md_kind(ReqKind::Replace));
}

#[test]
fn preview_is_max_damage() {
    let t = T::start(vec![scizor(), ttar(), lax(), set("gengar", None, "cursedbody", &["shadowball", "sludgebomb", "protect", "willowisp"])], &PV);
    let mut p = by_name(P).unwrap();
    assert_eq!(p.teampreview(t.bt(), &mut Rng::new(1)), max_damage::preview_picks(t.bt()));
}

#[test]
fn matchup_parts() {
    let t = T::new(vec![scizor(), ttar(), lax()], &PV);
    let b = t.bt();
    let mut est = Estimator::new(b);
    let me = est.fighter(b, t.own(), false);
    let foe = est.fighter(b, t.opp(), false);
    // Fire on Scizor x4, Steel on Charizard x0.5: the type terms
    assert_eq!(players::threat::type_mult(cg_engine::sim::data::Type::Fire, &me.types), 4.0);
    // Levitate or Air Balloon makes a Ground move score 0
    let eq = cg_engine::sim::data::MoveId::from_id("earthquake").unwrap();
    let rot = Fighter::species(cg_engine::sim::data::SpeciesId::from_id("garchomp").unwrap(), Side::Opp);
    assert!(attack_score(&est.field, &me, eq, &rot, 1.54) > 0.0);
    let mut lev = rot;
    lev.ability = cg_engine::sim::data::AbilityId::from_id("levitate");
    assert_eq!(attack_score(&est.field, &me, eq, &lev, 1.54), 0.0);
    let _ = foe;
    // Stealth Rock on our side: Scizor (neutral to Rock) takes 1/8, weighted x4 in the matchup
    let mut t2 = T::new(vec![scizor(), ttar(), lax()], &PV);
    t2.e(&side_cond("own", "stealthrock"));
    let b2 = t2.bt();
    let f = Fighter::of(b2, t2.own(), false);
    assert!((entry_hazard_fraction(b2, &f) - 0.125).abs() < 1e-6);
    let before = score(&t, t.own());
    let after = score(&t2, t2.own());
    assert!((before - after - 0.5).abs() < 1e-4, "{} {}", before, after);
}
