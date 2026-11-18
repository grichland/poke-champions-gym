//! Disciplined: D-0 to D-5, the phase filter and the back-to-back rule, each where it fires and where it
//! must not.

mod common;
use battle::*;
use common::*;
use players::threat::{best_hit, can_ko, hit_fraction};
use players::*;
use cg_engine::sim::data::*;

const P: &str = "disciplined";

fn team(first: SetMsg) -> Vec<SetMsg> {
    vec![
        first,
        set("tyranitar", None, "sandstream", &["stoneedge", "crunch", "earthquake", "protect"]),
        set("lucario", None, "innerfocus", &["closecombat", "meteormash", "protect", "swordsdance"]),
    ]
}

fn vs(first: SetMsg, opp: &[&str]) -> T {
    let mut pv: Vec<&str> = opp.to_vec();
    for p in PREVIEW {
        if !pv.contains(&p) && pv.len() < 6 {
            pv.push(p);
        }
    }
    T::new(team(first), &pv)
}

fn own_hp(t: &mut T, mon: &str, frac: f32) {
    let max = t.bt().sets[t.own().slot as usize].stats[0] as i32;
    t.e(&hp_own(mon, max, ((max as f32) * frac).round() as i32));
}

fn slot_of(t: &T, m: &str) -> usize {
    t.bt().sets[t.own().slot as usize].moves.iter().position(|x| x.map(|x| x.id()) == Some(m)).unwrap()
}

#[test]
fn d0_forced_is_max_damage() {
    let mut t = vs(set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "protect"]), &["charizard"]);
    t.e(r#"{"k":"faint","side":"own","mon":"garchomp"}"#);
    let (a, f) = t.fired(P, ReqKind::Replace);
    assert_eq!(a, t.md_kind(ReqKind::Replace));
    assert_eq!(f, vec!["forced"]);
}

fn chomp() -> SetMsg {
    set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "protect"])
}

#[test]
fn d1a_denier_switch_to_a_member_that_takes_little_and_kos() {
    let mut t = vs(set("snorlax", None, "thickfat", &["bodyslam", "protect", "rest", "curse"]), &["charizard"]);
    t.e(&boost("opp", "charizard", "atk", 1));
    t.e(&hp_opp("charizard", 100.0, 20.0));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!((a, f), (9, vec!["denier:switch"]));
    // no boost and no revealed setup: no denier
    let mut t = vs(set("snorlax", None, "thickfat", &["bodyslam", "protect", "rest", "curse"]), &["charizard"]);
    t.e(&hp_opp("charizard", 100.0, 20.0));
    assert!(!t.fired(P, ReqKind::Turn).1.iter().any(|r| r.starts_with("denier")));
}

#[test]
fn d1_a_revealed_setup_move_also_triggers_the_denier() {
    let mut t = vs(set("snorlax", None, "thickfat", &["bodyslam", "protect", "rest", "curse"]), &["charizard"]);
    t.e(&mv("opp", "charizard", "dragondance"));
    t.e(&hp_opp("charizard", 100.0, 20.0));
    t.turn(2);
    assert_eq!(t.fired(P, ReqKind::Turn).1, vec!["denier:switch"]);
}

#[test]
fn d1b_denier_ko_when_first_or_safe() {
    let mut t = vs(chomp(), &["charizard"]);
    t.e(&boost("opp", "charizard", "atk", 1));
    t.e(&hp_opp("charizard", 100.0, 20.0));
    t.e(&vol("own", "garchomp", "trapped"));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!(f, vec!["denier:ko"]);
    let b = t.bt();
    assert_eq!(Some(a), b.mon(t.own()).moves.iter().position(|m| *m == best_hit(b, &mut Estimator::new(b), t.own(), t.opp()).1));
    assert_eq!(a, slot_of(&t, "stoneedge"));
}

#[test]
fn d1c_anti_setup_when_we_cannot_ko() {
    let mut t = vs(set("toxapex", None, "regenerator", &["haze", "toxic", "recover", "sludgebomb"]), &["dragonite"]);
    t.e(&boost("opp", "dragonite", "atk", 2));
    t.e(&vol("own", "toxapex", "trapped"));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!((a, f), (0, vec!["denier:anti_setup"]));
    // Haze needs a boost to clear: a revealed setup move alone does not make it work
    let mut t = vs(set("toxapex", None, "regenerator", &["haze", "toxic", "recover", "sludgebomb"]), &["dragonite"]);
    t.e(&mv("opp", "dragonite", "dragondance"));
    t.e(&vol("own", "toxapex", "trapped"));
    t.turn(2);
    assert_ne!(t.fired(P, ReqKind::Turn).1, vec!["denier:anti_setup"]);
}

#[test]
fn d1d_priority_ko_when_slower_and_threatened() {
    let mut t = vs(set("scizor", None, "technician", &["bulletpunch", "uturn", "swordsdance", "roost"]), &["dragonite"]);
    t.e(&boost("opp", "dragonite", "atk", 1));
    t.e(&mv("opp", "dragonite", "flamethrower"));
    t.e(&hp_opp("dragonite", 100.0, 4.0));
    own_hp(&mut t, "scizor", 0.1);
    t.e(&vol("own", "scizor", "trapped"));
    t.turn(2);
    let b = t.bt();
    let mut est = Estimator::new(b);
    assert!(can_ko(b, &mut est, t.own(), t.opp()) && can_ko(b, &mut est, t.opp(), t.own()));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!((a, f), (0, vec!["denier:priority_ko"]));
}

#[test]
fn d1_hardest_hit_when_no_step_applies() {
    let mut t = vs(set("snorlax", None, "thickfat", &["bodyslam", "protect", "rest", "curse"]), &["dragonite"]);
    t.e(&boost("opp", "dragonite", "atk", 1));
    t.e(&vol("own", "snorlax", "trapped"));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!(f, vec!["denier:hardest_hit"]);
    assert_eq!(a, t.md());
}

#[test]
fn d2_futile_switch_ignores_the_back_to_back_rule() {
    let ground = || set("garchomp", None, "roughskin", &["earthquake", "bulldoze", "highhorsepower", "swordsdance"]);
    let t = vs(ground(), &["corviknight"]);
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!(f, vec!["futile_switch"]);
    let b = t.bt();
    let mut est = Estimator::new(b);
    let h = |s: u8, est: &mut Estimator| best_hit(b, est, Who { side: Side::Own, slot: s }, t.opp()).0;
    let want = if h(2, &mut est) > h(1, &mut est) { 10 } else { 9 };
    assert_eq!(a, want);
    // switched last turn, the opponent stayed: still switches
    let mut t = T::new(vec![set("snorlax", None, "thickfat", &["bodyslam", "protect", "rest", "curse"]), ground(), team(chomp())[1].clone()], &["corviknight", "gengar", "dragonite", "garchomp", "rotomwash", "ditto"]);
    t.switch("own", "snorlax", "garchomp");
    t.e(&mv("opp", "corviknight", "roost"));
    t.turn(2);
    assert!(players::best_matchup::blocked(t.bt(), 2).is_some());
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert!(a >= 8 && f == vec!["futile_switch"], "{} {:?}", a, f);
}

fn scizor_vs_zard() -> T {
    let mut t = vs(set("scizor", None, "swarm", &["bulletpunch", "swordsdance", "protect", "substitute"]), &["charizard"]);
    t.e(&mv("opp", "charizard", "flamethrower"));
    t.turn(2);
    t
}

#[test]
fn d3_bad_trade_switch() {
    let t = scizor_vs_zard();
    let b = t.bt();
    let mut est = Estimator::new(b);
    let mine = hit_fraction(b, &mut est, t.own(), MoveId::from_id("bulletpunch").unwrap(), t.opp());
    assert!(mine >= 0.05 && mine < 0.15, "{}", mine);
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!((a, f), (9, vec!["trade_switch"]));
}

#[test]
fn d3_not_when_unthreatened() {
    // the same weak attack, but the foe has shown nothing that hurts Scizor
    let mut t = vs(set("scizor", None, "swarm", &["bulletpunch", "swordsdance", "protect", "substitute"]), &["charizard"]);
    t.e(&mv("opp", "charizard", "roost"));
    t.turn(2);
    let f = t.fired(P, ReqKind::Turn).1;
    assert!(!f.contains(&"trade_switch"), "{:?}", f);
}

fn lax_vs_zard(hp: f32) -> T {
    let mut t = vs(set("snorlax", None, "immunity", &["bodyslam", "protect", "rest", "curse"]), &["charizard"]);
    t.e(&mv("opp", "charizard", "flamethrower"));
    own_hp(&mut t, "snorlax", hp);
    t.turn(2);
    t
}

#[test]
fn d4_threatened_switch_to_a_safe_threatening_member() {
    let t = lax_vs_zard(0.2);
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!((a, f), (9, vec!["threatened_switch"]));
}

#[test]
fn d4_not_when_we_can_ko_or_are_not_threatened() {
    let mut t = lax_vs_zard(0.2);
    t.e(&hp_opp("charizard", 100.0, 5.0));
    assert_eq!(t.fired(P, ReqKind::Turn).1, vec!["attack"]);
    let t = lax_vs_zard(1.0);
    assert_eq!(t.fired(P, ReqKind::Turn).1, vec!["attack"]);
}

#[test]
fn back_to_back_blocks_the_threatened_switch() {
    let mut t = T::new(
        vec![set("gengar", None, "cursedbody", &["shadowball", "protect", "sludgebomb", "willowisp"]), team(chomp())[1].clone(), set("snorlax", None, "immunity", &["bodyslam", "protect", "rest", "curse"])],
        &["charizard", "gengar", "dragonite", "garchomp", "rotomwash", "ditto"],
    );
    t.switch("own", "gengar", "snorlax");
    t.e(&mv("opp", "charizard", "flamethrower"));
    own_hp(&mut t, "snorlax", 0.2);
    t.turn(2);
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert!(a < 8);
    assert_eq!(f, vec!["attack", "switch_blocked"]);
}

fn ghold(hp: f32, ahead: bool) -> T {
    let mut t = vs(set("gholdengo", None, "goodasgold", &["recover", "makeitrain", "shadowball", "nastyplot"]), &["garchomp", "dragonite"]);
    if ahead {
        t.e(r#"{"k":"faint","side":"opp","mon":"garchomp"}"#);
        t.e(r#"{"k":"enter","side":"opp","mon":"dragonite","forced":false}"#);
    } else {
        t.switch("opp", "garchomp", "dragonite");
    }
    t.e(&mv("opp", "dragonite", "extremespeed"));
    own_hp(&mut t, "gholdengo", hp);
    t.turn(2);
    t
}

#[test]
fn d5_recover_below_the_threshold() {
    let t = ghold(0.5, false);
    assert_eq!(t.fired(P, ReqKind::Turn), (0, vec!["recover"]));
    let t = ghold(0.7, false);
    assert_eq!(t.fired(P, ReqKind::Turn).1, vec!["attack"]);
}

#[test]
fn d5_recover_blocked_when_ahead() {
    let t = ghold(0.5, true);
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert!(a != 0);
    assert!(f.contains(&"recover_blocked") && f.contains(&"attack"), "{:?}", f);
}

#[test]
fn phase_drops_status_moves_when_ahead() {
    let ground = || set("garchomp", None, "roughskin", &["protect", "earthquake", "bulldoze", "highhorsepower"]);
    let mk = |ahead: bool| {
        let mut t = vs(ground(), &["garchomp", "corviknight"]);
        if ahead {
            t.e(r#"{"k":"faint","side":"opp","mon":"garchomp"}"#);
            t.e(r#"{"k":"enter","side":"opp","mon":"corviknight","forced":false}"#);
        } else {
            t.switch("opp", "garchomp", "corviknight");
        }
        t.e(&vol("own", "garchomp", "trapped"));
        t.turn(2);
        t
    };
    let t = mk(false);
    assert_eq!(t.fired(P, ReqKind::Turn), (0, vec!["attack"]));
    let t = mk(true);
    assert_eq!(t.fired(P, ReqKind::Turn), (1, vec!["attack", "phase"]));
}

#[test]
fn mega_variants_when_offered() {
    let t = vs(set("garchomp", Some("garchompitez"), "roughskin", &["dragonclaw", "earthquake", "stoneedge", "protect"]), &["gengar"]);
    assert!(t.play(P, 1) >= 4 && t.play(P, 1) < 8);
}
