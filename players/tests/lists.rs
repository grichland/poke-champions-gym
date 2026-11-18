//! The generated move classes against the spec's gen9 lists. Run with --nocapture to see the differences.

use players::lists::{class, members};
use cg_engine::sim::data::MoveId;

const SPEC_SETUP: &str = "acidarmor agility amnesia autotomize barrier bulkup calmmind charge clangoroussoul coil cosmicpower cottonguard defendorder defensecurl doubleteam dragondance extremeevoboost filletaway geomancy growth harden honeclaws irondefense meditate minimize nastyplot noretreat quiverdance rockpolish sharpen shellsmash shelter shiftgear swordsdance tailglow victorydance withdraw workup bellydrum";
const SPEC_RECOVERY: &str = "recover roost slackoff softboiled milkdrink shoreup wish synthesis moonlight morningsun rest";
const SPEC_SPEED_DOWN: &str = "icywind electroweb rocktomb bulldoze mudshot scaryface stringshot cottonspore glaciate lowsweep thunderwave nuzzle";
const SPEC_INFLICT: &str = "thunderwave willowisp toxic poisonpowder sleeppowder spore hypnosis glare stunspore yawn confuseray swagger flatter supersonic sweetkiss teeterdance attract nuzzle infestation whirlpool firespin sandtomb magmastorm";
const SPEC_SCREENS: &str = "reflect lightscreen auroraveil";
const SPEC_FIRST_TURN: &str = "fakeout firstimpression matblock";
const SPEC_HAZARDS: &str = "stealthrock spikes toxicspikes stickyweb";

fn diff(name: &str, spec: &str, c: u16) -> (Vec<String>, Vec<String>, Vec<String>) {
    let gen = members(c);
    let spec: Vec<&str> = spec.split_whitespace().collect();
    let not_legal: Vec<String> = spec.iter().filter(|m| MoveId::from_id(m).is_none()).map(|s| s.to_string()).collect();
    let dropped: Vec<String> = spec.iter().filter(|m| MoveId::from_id(m).is_some() && !gen.contains(m)).map(|s| s.to_string()).collect();
    let added: Vec<String> = gen.iter().filter(|m| !spec.contains(m)).map(|s| s.to_string()).collect();
    println!("{}: generated {} | spec not legal in Champions: {:?} | legal but not generated: {:?} | added: {:?}", name, gen.len(), not_legal, dropped, added);
    (not_legal, dropped, added)
}

#[test]
fn generated_lists_against_the_spec() {
    let (_, dropped, _) = diff("SETUP", SPEC_SETUP, class::SETUP);
    assert!(dropped.is_empty(), "setup moves the rule misses: {:?}", dropped);
    let (_, dropped, _) = diff("RECOVERY", SPEC_RECOVERY, class::RECOVERY);
    assert!(dropped.is_empty(), "recovery moves the rule misses: {:?}", dropped);
    diff("SPEED_DOWN", SPEC_SPEED_DOWN, class::SPEED_DOWN);
    diff("INFLICT", SPEC_INFLICT, class::INFLICT);
    let (_, dropped, added) = diff("SCREENS", SPEC_SCREENS, class::SCREEN);
    assert!(dropped.is_empty() && added.is_empty());
    diff("FIRST_TURN", SPEC_FIRST_TURN, class::FIRST_TURN);
    diff("HAZARDS", SPEC_HAZARDS, class::HAZARD);
    // every class is non-empty and the core members are where the rules say
    for (m, c) in [("swordsdance", class::SETUP), ("recover", class::RECOVERY), ("thunderwave", class::SPEED_DOWN), ("icywind", class::SPEED_DOWN), ("willowisp", class::INFLICT), ("reflect", class::SCREEN), ("fakeout", class::FIRST_TURN), ("stealthrock", class::HAZARD)] {
        assert!(players::lists::is(MoveId::from_id(m).unwrap(), c), "{} not in its class", m);
    }
    // damaging moves are never INFLICT (PS-B5)
    assert!(!players::lists::is(MoveId::from_id("nuzzle").unwrap(), class::INFLICT));
}

#[test]
fn print_generated_lists() {
    for (n, c) in [("SETUP", class::SETUP), ("RECOVERY", class::RECOVERY), ("SPEED_DOWN", class::SPEED_DOWN), ("INFLICT", class::INFLICT), ("FIRST_TURN", class::FIRST_TURN), ("ALWAYS_HIGH", class::ALWAYS_HIGH)] {
        println!("{} = {}", n, members(c).join(" "));
    }
}
