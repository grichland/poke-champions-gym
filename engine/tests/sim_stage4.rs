//! Stage-4 rules that random battles reach rarely, played through the public API with scripted outcomes:
//! a hidden trap refused and revealed, Imprison's hidden disables, Intimidate against Defiant, Mirror Armor
//! and Guard Dog, Unaware ignoring stages, Mold Breaker through Levitate, Magic Guard against recoil, Sturdy,
//! Cloud Nine suppressing sand, weather abilities at the start, Skill Swap and Neutralizing-free ability
//! changes, Gastro Acid, Emergency Exit's switch request, Protean, Disguise and an unimplemented ability.

use cg_engine::sim::data::*;
use cg_engine::sim::outcome::{EffectRef, Key, Outcomes, Site};
use cg_engine::sim::state::State;
use cg_engine::sim::{Choice, Kind, MonSpec};

/// Accuracy always hits; no crits; chance effects as `procs` says; rolls at their highest damage; ties go
/// to the first named.
#[derive(Default)]
struct Script {
    procs: bool,
}

impl Outcomes for Script {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        match site.key {
            Key::Accuracy | Key::Protect => true,
            Key::Crit => false,
            _ => self.procs,
        }
    }
    fn range(&mut self, _: &Site, lo: i32, _: i32) -> i32 {
        lo
    }
    fn sample(&mut self, _: &Site, values: &[i32]) -> i32 {
        values[0]
    }
    fn tie(&mut self, _: &Site, a: u8, _: u8) -> u8 {
        a
    }
    fn tie_effects(&mut self, _: &Site, _: EffectRef, _: EffectRef) -> bool {
        true
    }
    fn sample_str(&mut self, _: &Site, _: &[&'static str]) -> usize {
        0
    }
}

fn mv(id: &str) -> MoveId {
    MoveId::from_id(id).unwrap_or_else(|| panic!("move {}", id))
}

fn ab(id: &str) -> AbilityId {
    AbilityId::from_id(id).unwrap_or_else(|| panic!("ability {}", id))
}

fn spec(species: &str, ability: &str, moves: &[&str]) -> MonSpec {
    let mut m = [MoveId::NONE; 4];
    for (i, x) in moves.iter().enumerate() {
        m[i] = mv(x);
    }
    MonSpec {
        species: SpeciesId::from_id(species).unwrap_or_else(|| panic!("species {}", species)),
        ability: ab(ability),
        item: ItemId::NONE,
        moves: m,
        nmoves: moves.len() as u8,
        nature: nature_from_id("serious").unwrap(),
        evs: [0; 6],
        gender: 1,
    }
}

type Mon<'a> = (&'a str, &'a str, &'a [&'a str]);

/// Teams of three (slots 0, 1, 2 brought in that order); the rest of the six repeat slot 0.
fn battle(p1: [Mon; 3], p2: [Mon; 3], o: &mut Script) -> State {
    let team = |t: [Mon; 3]| -> Vec<MonSpec> {
        let mut v: Vec<MonSpec> = t.iter().map(|(s, a, m)| spec(s, a, m)).collect();
        while v.len() < 6 {
            v.push(spec(t[0].0, t[0].1, t[0].2));
        }
        v
    };
    State::start([&team(p1), &team(p2)], [[0, 1, 2], [0, 1, 2]], o).unwrap()
}

fn mvc(i: u8) -> Option<Choice> {
    Some(Choice::Move { index: i, mega: false })
}

const SPLASH: &[&str] = &["calmmind"];

#[test]
fn shadow_tag_trap_is_offered_then_refused() {
    let mut o = Script::default();
    let mut st = battle(
        [("garchomp", "roughskin", &["earthquake"]), ("gholdengo", "goodasgold", &["makeitrain"]), ("primarina", "torrent", &["moonblast"])],
        [("alakazam", "shadowtag", &["psychic"]), ("mamoswine", "thickfat", &["iceshard"]), ("cinderace", "libero", &["pyroball"])],
        &mut o,
    );
    let op = st.options();
    assert_eq!(op[0].kind, Kind::Turn);
    assert!(op[0].switch_list().is_empty(), "trapped: no legal switch");
    assert_eq!(op[0].hidden_switch_list(), vec![1, 2], "the request still offers the switches");
    // the refused choice changes nothing but the request
    let before = st;
    let r = st.refuse([Some(Choice::Switch(1)), mvc(0)]).expect("refused");
    assert_eq!(r[0], Some("trapped"));
    assert_eq!(r[1], None);
    assert_eq!(st.sides[0].mons, before.sides[0].mons);
    assert!(st.options()[0].hidden_switch_list().is_empty(), "revealed: no longer offered");
    // a new turn hides it again
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    if st.options()[0].kind == Kind::Turn {
        assert_eq!(st.options()[0].hidden_switch_list(), vec![1, 2]);
    }
}

#[test]
fn imprison_hides_shared_moves_and_struggle_when_all_are_shared() {
    let mut o = Script::default();
    let mut st = battle(
        [("alakazam", "synchronize", &["imprison", "psychic", "recover"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        [("slowbro", "oblivious", &["psychic", "imprison", "slackoff", "scald"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(3)], &mut o).unwrap();
    let op = st.options();
    // Slowbro's Psychic and Imprison are disabled but still offered; Slack Off and Scald are free
    assert_eq!(op[1].hidden_move_list(), vec![0, 1]);
    assert_eq!(op[1].move_list(), vec![2, 3]);
    let r = st.refuse([mvc(1), mvc(0)]).expect("refused");
    assert_eq!(r[1], Some("disabled"));
    // only the chosen move is revealed
    assert_eq!(st.options()[1].hidden_move_list(), vec![1]);
}

#[test]
fn intimidate_meets_defiant_mirror_armor_and_guard_dog() {
    let mut o = Script::default();
    let st = battle(
        [("gyarados", "intimidate", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        [("kingambit", "defiant", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    assert_eq!(st.sides[1].mons[0].boosts[0], 1, "Defiant: -1 then +2");
    let st = battle(
        [("gyarados", "intimidate", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        [("corviknight", "mirrorarmor", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    assert_eq!(st.sides[1].mons[0].boosts[0], 0);
    assert_eq!(st.sides[0].mons[0].boosts[0], -1, "Mirror Armor sends the drop back");
    let st = battle(
        [("gyarados", "intimidate", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        [("mabosstiff", "guarddog", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    assert_eq!(st.sides[1].mons[0].boosts[0], 1, "Guard Dog raises Attack instead");
}

#[test]
fn unaware_ignores_the_attackers_stages() {
    let dmg = |unaware: &str| {
        let mut o = Script::default();
        let mut st = battle(
            [("garchomp", "roughskin", &["swordsdance", "earthquake"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
            [("clefable", unaware, &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
            &mut o,
        );
        st.step([mvc(0), mvc(0)], &mut o).unwrap();
        let hp = st.sides[1].mons[0].hp;
        st.step([mvc(1), mvc(0)], &mut o).unwrap();
        hp - st.sides[1].mons[0].hp
    };
    let plain = dmg("magicguard");
    let unaware = dmg("unaware");
    assert!(unaware * 3 / 2 < plain + 2 && unaware < plain, "{} vs {}", unaware, plain);
}

#[test]
fn mold_breaker_hits_through_levitate() {
    let run = |a: &str| {
        let mut o = Script::default();
        let mut st = battle(
            [("excadrill", a, &["earthquake"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
            [("hydreigon", "levitate", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
            &mut o,
        );
        st.step([mvc(0), mvc(0)], &mut o).unwrap();
        st.sides[1].mons[0].hp < st.sides[1].mons[0].maxhp
    };
    assert!(!run("sandrush"), "Levitate avoids Earthquake");
    assert!(run("moldbreaker"), "Mold Breaker ignores Levitate");
}

#[test]
fn magic_guard_takes_no_recoil_and_sturdy_holds() {
    let mut o = Script::default();
    let mut st = battle(
        [("clefable", "magicguard", &["doubleedge"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("garchomp", "roughskin", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    let c = &st.sides[0].mons[0];
    assert_eq!(c.hp, c.maxhp, "no recoil and no Rough Skin damage with Magic Guard");
    let mut st = battle(
        [("garchomp", "roughskin", &["earthquake"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("aggron", "sturdy", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    assert_eq!(st.sides[1].mons[0].hp, 1, "Sturdy at full HP leaves 1");
}

#[test]
fn cloud_nine_suppresses_sand_set_by_sand_stream() {
    let mut o = Script::default();
    let mut st = battle(
        [("tyranitar", "sandstream", &["calmmind"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("altaria", "cloudnine", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    assert_eq!(st.field.weather.id, conds::SANDSTORM, "the weather is set");
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    let g = &st.sides[1].mons[0];
    assert_eq!(g.hp, g.maxhp, "no sand damage while Cloud Nine is out");
}

#[test]
fn skill_swap_and_gastro_acid() {
    let mut o = Script::default();
    let mut st = battle(
        [("alakazam", "magicguard", &["skillswap", "gastroacid"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("garchomp", "roughskin", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    assert_eq!(st.sides[0].mons[0].ability, abilities::ROUGHSKIN);
    assert_eq!(st.sides[1].mons[0].ability, abilities::MAGICGUARD);
    assert_eq!(st.sides[0].mons[0].base_ability, abilities::MAGICGUARD, "the base ability stays");
    st.step([mvc(1), mvc(0)], &mut o).unwrap();
    assert!(st.sides[1].has_vol(conds::GASTROACID));
}

#[test]
fn emergency_exit_asks_for_a_switch() {
    let mut o = Script::default();
    let mut st = battle(
        [("garchomp", "roughskin", &["outrage"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("golisopod", "emergencyexit", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.sides[1].mons[0].hp = st.sides[1].mons[0].maxhp / 2 + 30;
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    let g = &st.sides[1].mons[0];
    if g.hp > 0 && (g.hp as u32) * 2 <= g.maxhp as u32 {
        assert_eq!(st.options()[1].kind, Kind::Replace, "dropped below half: Emergency Exit");
    }
}

#[test]
fn protean_changes_type_once_per_entry() {
    let mut o = Script::default();
    let mut st = battle(
        [("greninja", "protean", &["icebeam", "darkpulse"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("corviknight", "pressure", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    let g = &st.sides[0].mons[0];
    assert_eq!(&g.types[..g.ntypes as usize], &[Type::Ice]);
    st.step([mvc(1), mvc(0)], &mut o).unwrap();
    let g = &st.sides[0].mons[0];
    assert_eq!(&g.types[..g.ntypes as usize], &[Type::Ice], "gen 9: once per switch-in");
}

#[test]
fn disguise_takes_the_first_hit() {
    let mut o = Script::default();
    let mut st = battle(
        [("garchomp", "roughskin", &["earthquake"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("mimikyu", "disguise", &["calmmind"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    let m = &st.sides[1].mons[0];
    assert_eq!(m.species.id(), "mimikyubusted");
    assert_eq!(m.hp, m.maxhp - m.maxhp / 8, "only the bust damage");
}

#[test]
fn unimplemented_ability_marks_the_battle() {
    let mut o = Script::default();
    let st = battle(
        [("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("garchomp", "neutralizinggas", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    assert!(st.unimpl.count > 0, "Neutralizing Gas is not implemented");
    let mut o = Script::default();
    let st = battle(
        [("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("garchomp", "roughskin", SPLASH), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    assert_eq!(st.unimpl.count, 0);
}

#[test]
fn destiny_bond_does_not_take_a_future_attacker() {
    let mut o = Script::default();
    let mut st = battle(
        [("alakazam", "synchronize", &["futuresight", "calmmind"]), ("gholdengo", "goodasgold", SPLASH), ("primarina", "torrent", SPLASH)],
        [("gengar", "cursedbody", &["calmmind", "destinybond"]), ("garchomp", "roughskin", SPLASH), ("gholdengo", "goodasgold", SPLASH)],
        &mut o,
    );
    st.step([mvc(0), mvc(0)], &mut o).unwrap();
    st.step([mvc(1), mvc(0)], &mut o).unwrap();
    st.sides[1].mons[0].hp = 1;
    st.step([mvc(1), mvc(1)], &mut o).unwrap();
    assert!(st.sides[1].mons[0].fainted, "Future Sight lands");
    assert!(!st.sides[0].mons[0].fainted, "Destiny Bond ignores future attacks");
}
