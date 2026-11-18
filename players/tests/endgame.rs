//! The 1v1 chain's arithmetic and the endgame player: E-1 (options against the worst unseen members),
//! E-2 (stay, switch, send; draw and tie fallbacks), E-3 (back-to-back), and disciplined outside it.

mod common;
use battle::*;
use common::*;
use players::chain::{chain_options, Attack, Attacks, Combatant, Matchups};
use players::endgame::{combinations, in_endgame, EndgameParams};
use players::*;

const P: &str = "endgame";

fn atk(id: u16, fraction: f32, priority: i8) -> Attack {
    Attack { id, fraction, priority, accuracy: 1.0, ko_fraction: fraction / 0.925, recover: false }
}
fn unit(hp: f32, speed: i32) -> Combatant {
    Combatant { hp, speed, can_switch_on_faint: false, ..Combatant::default() }
}
fn one(a: Attack) -> Attacks {
    let mut x = Attacks::default();
    x.push(a);
    x
}

#[test]
fn chain_faster_side_wins_the_race() {
    let mut m = Matchups::default();
    m.ours[0][0] = one(atk(1, 0.6, 0));
    m.theirs[0][0] = one(atk(2, 0.6, 0));
    let (r, n) = chain_options(&[unit(1.0, 100)], &[unit(1.0, 50)], &m, 0, 0);
    assert_eq!(n, 1);
    assert!(r[0].1.wins && (r[0].1.margin - 0.4).abs() < 1e-5, "{:?}", r[0]);
    assert_eq!(r[0].1.first.map(|a| a.id), Some(1));
    // equal speed: the tie goes to them
    let (r, _) = chain_options(&[unit(1.0, 50)], &[unit(1.0, 50)], &m, 0, 0);
    assert!(!r[0].1.wins && r[0].1.margin < 0.0);
    // priority beats speed
    m.ours[0][0] = one(atk(1, 0.6, 1));
    let (r, _) = chain_options(&[unit(1.0, 10)], &[unit(1.0, 50)], &m, 0, 0);
    assert!(r[0].1.wins);
}

#[test]
fn chain_no_damage_is_a_draw_and_recovery_outlasts() {
    let m = Matchups::default();
    let (r, _) = chain_options(&[unit(0.8, 10)], &[unit(0.5, 50)], &m, 0, 0);
    assert!(!r[0].1.wins && (r[0].1.margin - 0.3).abs() < 1e-5);
    // they hit for 0.2 a turn, we hit for 0.1 and recover half under 50%: we win the long race
    let mut m = Matchups::default();
    m.ours[0][0] = one(atk(1, 0.1, 0));
    m.theirs[0][0] = one(atk(2, 0.2, 0));
    let us = Combatant { recover: 0.5, ..unit(1.0, 10) };
    let (r, _) = chain_options(&[us], &[unit(1.0, 50)], &m, 0, 0);
    assert!(r[0].1.wins, "{:?}", r[0]);
    let (r, _) = chain_options(&[unit(1.0, 10)], &[unit(1.0, 50)], &m, 0, 0);
    assert!(!r[0].1.wins);
}

#[test]
fn chain_switching_to_the_counter() {
    // our 0 cannot touch their 0, our 1 wins; switching costs a turn
    let mut m = Matchups::default();
    m.theirs[0][0] = one(atk(2, 0.3, 0));
    m.theirs[1][0] = one(atk(2, 0.1, 0));
    m.ours[1][0] = one(atk(3, 0.5, 0));
    let (r, n) = chain_options(&[unit(1.0, 10), unit(1.0, 100)], &[unit(1.0, 50)], &m, 0, 0);
    assert_eq!(n, 2);
    assert_eq!(r[0].0, 0);
    assert_eq!(r[1].0, 1);
    assert!(r[1].1.wins && !r[1].1.first.is_some());
}

#[test]
fn combinations_in_order() {
    let mut v = vec![];
    combinations(4, 2, |c| v.push(c.to_vec()));
    assert_eq!(v, vec![vec![0, 1], vec![0, 2], vec![0, 3], vec![1, 2], vec![1, 3], vec![2, 3]]);
    let mut n = 0;
    combinations(3, 0, |_| n += 1);
    assert_eq!(n, 1);
    combinations(1, 2, |_| panic!());
}

fn ttar() -> SetMsg {
    set("tyranitar", None, "sandstream", &["crunch", "stoneedge", "earthquake", "protect"])
}
fn normal_lax() -> SetMsg {
    set("snorlax", None, "immunity", &["bodyslam", "protect", "rest", "curse"])
}
const PV: [&str; 6] = ["charizard", "gengar", "dragonite", "garchomp", "rotomwash", "corviknight"];

/// Both leads faint on turn 1: we send `send`, they send Gengar, which has shown Shadow Ball.
fn two_each(team: Vec<SetMsg>, send: &str) -> T {
    let lead = team[0].species.clone();
    let mut t = T::new(team, &PV);
    t.e(&format!(r#"{{"k":"faint","side":"own","mon":"{}"}}"#, lead));
    t.e(r#"{"k":"faint","side":"opp","mon":"charizard"}"#);
    t.e(&format!(r#"{{"k":"enter","side":"own","mon":"{}"}}"#, send));
    t.e(r#"{"k":"enter","side":"opp","mon":"gengar"}"#);
    t.e(&mv("opp", "gengar", "shadowball"));
    t.turn(2);
    t
}

fn chomp() -> SetMsg {
    set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "protect"])
}

#[test]
fn not_in_the_endgame_plays_disciplined() {
    let t = T::new(vec![chomp(), ttar(), normal_lax()], &PV);
    assert!(!in_endgame(t.bt(), &EndgameParams::default()));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!(a, t.fired("disciplined", ReqKind::Turn).0);
    assert!(!f.iter().any(|r| r.starts_with("endgame")), "{:?}", f);
}

#[test]
fn e2_stay_with_the_line_move() {
    let t = two_each(vec![chomp(), ttar(), normal_lax()], "tyranitar");
    assert!(in_endgame(t.bt(), &EndgameParams::default()));
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!(f, vec!["endgame:stay"]);
    assert_eq!(a, 0, "Crunch");
}

#[test]
fn e2_switch_to_the_member_that_wins() {
    // Snorlax's Normal moves cannot touch Gengar; Tyranitar's Crunch can
    let t = two_each(vec![chomp(), normal_lax(), ttar()], "snorlax");
    let (a, f) = t.fired(P, ReqKind::Turn);
    assert_eq!((a, f), (10, vec!["endgame:switch"]));
}

#[test]
fn e3_back_to_back_takes_the_switch_out_of_the_options() {
    let mut t = two_each(vec![chomp(), ttar(), normal_lax()], "tyranitar");
    // turn 2: Tyranitar switches to Snorlax, Gengar attacks; now the winning switch back is blocked and
    // staying has no move that does anything
    t.switch("own", "tyranitar", "snorlax");
    t.e(&mv("opp", "gengar", "shadowball"));
    t.turn(3);
    let f = t.fired(P, ReqKind::Turn).1;
    assert!(f.contains(&"endgame:fallback_no_line") && !f.contains(&"endgame:switch"), "{:?}", f);
    // the opponent switched too: the switch is an option again
    let mut t = two_each(vec![chomp(), ttar(), normal_lax()], "tyranitar");
    t.switch("own", "tyranitar", "snorlax");
    t.switch("opp", "gengar", "dragonite");
    t.switch("opp", "dragonite", "gengar");
    t.turn(3);
    assert!(players::best_matchup::blocked(t.bt(), 2).is_none());
}

#[test]
fn e2_send_after_a_faint() {
    let mut t = T::new(vec![chomp(), ttar(), normal_lax()], &PV);
    t.e(r#"{"k":"faint","side":"opp","mon":"charizard"}"#);
    t.e(r#"{"k":"enter","side":"opp","mon":"gengar"}"#);
    t.e(&mv("opp", "gengar", "shadowball"));
    t.turn(2);
    t.e(r#"{"k":"faint","side":"own","mon":"garchomp"}"#);
    assert!(in_endgame(t.bt(), &EndgameParams::default()));
    let (a, f) = t.fired(P, ReqKind::Replace);
    assert_eq!((a, f), (9, vec!["endgame:send"]));
}

#[test]
fn e2_draw_falls_back_to_disciplined() {
    // nobody can hurt anybody: Snorlax against a Gengar that has shown nothing damaging
    let lead = "garchomp";
    let mut t = T::new(vec![chomp(), normal_lax(), set("dragonite", None, "innerfocus", &["extremespeed", "protect", "dragondance", "substitute"])], &PV);
    t.e(&format!(r#"{{"k":"faint","side":"own","mon":"{}"}}"#, lead));
    t.e(r#"{"k":"faint","side":"opp","mon":"charizard"}"#);
    t.e(r#"{"k":"enter","side":"own","mon":"snorlax"}"#);
    t.e(r#"{"k":"enter","side":"opp","mon":"gengar"}"#);
    t.turn(2);
    let f = t.fired(P, ReqKind::Turn).1;
    assert!(f.contains(&"endgame:fallback_draw") || f.contains(&"endgame:fallback_tie"), "{:?}", f);
}
