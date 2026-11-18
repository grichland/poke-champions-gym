//! Stage-5 rules that random battles reach rarely, played through the public API with scripted outcomes:
//! items (Focus Sash and Sturdy, Focus Band's draw, berries at Update, resist berries, Normal Gem, Life Orb and
//! Sheer Force, Leftovers before poison, Rocky Helmet, Air Balloon, Red Card, Eject Button, White Herb, Choice
//! Scarf, Quick Claw, the duration items, Klutz and Magic Room, Metronome, Mental Herb, seeds), the moves that
//! need items (Knock Off, Trick, Thief, Fling, Acrobatics, Belch, Bug Bite, Recycle), Mega Evolution (order on
//! the Mega turn, the new ability, once per side, not while locked) and the item abilities (Unburden, Harvest,
//! Cud Chew, Opportunist, Magician, Stalwart).

use cg_engine::sim::data::*;
use cg_engine::sim::outcome::{Effect, EffectRef, Key, Outcomes, Site};
use cg_engine::sim::state::{Request, State};
use cg_engine::sim::{Choice, Kind, MonSpec};

/// Accuracy always hits; no crits; chance effects as `procs` says; rolls at their highest damage; ties go
/// to the first named. Counts the chance draws of each item.
#[derive(Default)]
struct Script {
    procs: bool,
    item_draws: Vec<&'static str>,
}

impl Outcomes for Script {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        if let Effect::One(EffectRef::Item(i)) = site.effect {
            self.item_draws.push(i.id());
        }
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

fn it(id: &str) -> ItemId {
    if id.is_empty() {
        return ItemId::NONE;
    }
    ItemId::from_id(id).unwrap_or_else(|| panic!("item {}", id))
}

/// species, ability, item, moves
type Set<'a> = (&'a str, &'a str, &'a str, &'a [&'a str]);

fn spec(s: &Set) -> MonSpec {
    let mut m = [MoveId::NONE; 4];
    for (i, x) in s.3.iter().enumerate() {
        m[i] = mv(x);
    }
    MonSpec {
        species: SpeciesId::from_id(s.0).unwrap_or_else(|| panic!("species {}", s.0)),
        ability: AbilityId::from_id(s.1).unwrap_or_else(|| panic!("ability {}", s.1)),
        item: it(s.2),
        moves: m,
        nmoves: s.3.len() as u8,
        nature: nature_from_id("serious").unwrap(),
        evs: [0; 6],
        gender: 1,
    }
}

/// Teams of up to three (brought in that order); the rest of the six repeat slot 0 without its item.
fn battle(p1: &[Set], p2: &[Set], o: &mut Script) -> State {
    let team = |t: &[Set]| -> Vec<MonSpec> {
        let mut v: Vec<MonSpec> = t.iter().map(spec).collect();
        while v.len() < 6 {
            let mut x = spec(&t[0]);
            x.item = ItemId::NONE;
            v.push(x);
        }
        v
    };
    State::start([&team(p1), &team(p2)], [[0, 1, 2], [0, 1, 2]], o).unwrap()
}

fn m(i: u8) -> Option<Choice> {
    Some(Choice::Move { index: i, mega: false })
}

fn mega(i: u8) -> Option<Choice> {
    Some(Choice::Move { index: i, mega: true })
}

fn sw(s: u8) -> Option<Choice> {
    Some(Choice::Switch(s))
}

const REST: &[&str] = &["rest"];
/// a move that changes nothing the test looks at
const WAIT: &[&str] = &["swordsdance"];
const SLAM: &[&str] = &["bodyslam"];

#[test]
fn focus_sash_saves_once_and_sturdy_keeps_it() {
    let mut o = Script::default();
    let mut st = battle(&[("garchomp", "roughskin", "", &["earthquake"])], &[("alakazam", "synchronize", "focussash", &["calmmind"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let a = st.active(1);
    assert_eq!(a.hp, 1, "the sash leaves 1 HP");
    assert_eq!(a.item, ItemId::NONE, "and is used up");
    assert_eq!(a.last_item, it("focussash"));
    // Sturdy acts first (priority -30 before -40): the sash stays
    let mut st = battle(&[("garchomp", "roughskin", "", &["earthquake"])], &[("aggron", "sturdy", "focussash", &["irondefense"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).hp, 1);
    assert_eq!(st.active(1).item, it("focussash"));
}

#[test]
fn focus_band_rolls_on_every_damage_event() {
    let mut o = Script::default();
    let mut st = battle(&[("toxapex", "regenerator", "", &["toxic", "recover"])], &[("snorlax", "thickfat", "focusband", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    // Rest heals at full HP? no: Snorlax is at full HP, Rest fails; the poison tick at the end of the turn draws
    assert!(o.item_draws.contains(&"focusband"), "a draw for the poison damage, which was never lethal");
    assert!(st.active(1).hp < st.active(1).maxhp);
}

#[test]
fn sitrus_at_half_and_lum_at_once() {
    let mut o = Script::default();
    // Gholdengo's 162 HP: Belly Drum leaves exactly half, and the berry is eaten at the next Update
    let mut st = battle(&[("snorlax", "thickfat", "", WAIT)], &[("gholdengo", "goodasgold", "sitrusberry", &["bellydrum"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let a = *st.active(1);
    assert_eq!(a.item, ItemId::NONE);
    assert!(a.ate_berry);
    assert_eq!(a.hp, a.maxhp - a.maxhp / 2 + a.maxhp / 4);
    let mut st = battle(&[("gengar", "cursedbody", "", &["thunderwave"])], &[("primarina", "torrent", "lumberry", WAIT)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).status, Status::None, "Lum Berry cures on AfterSetStatus");
    assert_eq!(st.active(1).item, ItemId::NONE);
}

#[test]
fn resist_berry_only_on_a_damaging_super_effective_hit() {
    let mut o = Script::default();
    let p2: &[Set] = &[("rillaboom", "overgrow", "occaberry", WAIT)];
    let mut st = battle(&[("charizard", "blaze", "", &["willowisp", "seismictoss", "flamecharge"])], p2, &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).item, it("occaberry"), "a status move leaves it");
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).item, it("occaberry"), "fixed damage leaves it");
    // the burn's tick is part of each turn's loss
    let tick = st.active(1).maxhp / 16;
    let before = st.active(1).hp;
    st.step([m(2), m(0)], &mut o).unwrap();
    let with_berry = before - st.active(1).hp - tick;
    assert_eq!(st.active(1).item, ItemId::NONE);
    let mut st2 = battle(&[("charizard", "blaze", "", &["flamecharge"])], &[("rillaboom", "overgrow", "", WAIT)], &mut o);
    let full = st2.active(1).hp;
    st2.step([m(0), m(0)], &mut o).unwrap();
    let without = full - st2.active(1).hp;
    assert_eq!((with_berry, without), (33, 66), "halved by the berry");
}

#[test]
fn normal_gem_is_used_on_the_hit_only() {
    let mut o = Script::default();
    let mut st = battle(
        &[("kangaskhan", "innerfocus", "normalgem", &["swordsdance", "bodyslam"])],
        &[("gengar", "cursedbody", "", &["calmmind"]), ("snorlax", "thickfat", "", REST)],
        &mut o,
    );
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("normalgem"), "a Ghost is immune: the gem stays");
    st.step([m(1), sw(1)], &mut o).unwrap();
    assert_eq!(st.active(0).item, ItemId::NONE);
    assert_eq!(st.active(0).last_item, it("normalgem"));
    assert!(!st.sides[0].has_vol(conds::GEM), "the gem's volatile ends with the turn");
}

#[test]
fn life_orb_recoil_and_sheer_force() {
    let mut o = Script::default();
    let mut st = battle(&[("garchomp", "roughskin", "lifeorb", &["dragonclaw", "firefang"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let g = st.active(0);
    assert_eq!(g.hp, g.maxhp - g.maxhp / 10);
    let mut st = battle(&[("feraligatr", "sheerforce", "lifeorb", &["icefang"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).hp, st.active(0).maxhp, "no recoil for a move whose secondaries Sheer Force removed");
}

#[test]
fn leftovers_heal_before_poison() {
    let mut o = Script::default();
    let mut st = battle(&[("toxapex", "regenerator", "", &["toxic", "recover"])], &[("garchomp", "roughskin", "leftovers", &["swordsdance"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    // full HP: Leftovers heals nothing, then 1/16 poison
    let g = *st.active(1);
    assert_eq!(g.hp, g.maxhp - g.maxhp / 16);
    st.step([m(1), m(0)], &mut o).unwrap();
    // Leftovers back to full first, then 2/16
    let g = *st.active(1);
    assert_eq!(g.hp, g.maxhp - 2 * (g.maxhp / 16));
}

#[test]
fn rocky_helmet_per_contact_hit_not_on_magic_guard() {
    let mut o = Script::default();
    let p2: &[Set] = &[("skarmory", "sturdy", "rockyhelmet", &["roost"])];
    let mut st = battle(&[("cinderace", "libero", "", &["doublehit"]), ("clefable", "magicguard", "", &["doublehit"])], p2, &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let c = st.active(0);
    assert_eq!(c.hp, c.maxhp - 2 * (c.maxhp / 6), "two contact hits, two helmets");
    st.step([sw(1), m(0)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).hp, st.active(0).maxhp);
}

#[test]
fn air_balloon_floats_until_hit() {
    let mut o = Script::default();
    let mut st = battle(&[("garchomp", "roughskin", "", &["earthquake", "dragonclaw"])], &[("gholdengo", "goodasgold", "airballoon", &["nastyplot"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).hp, st.active(1).maxhp, "Ground moves miss a balloon holder");
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).item, ItemId::NONE);
    assert_eq!(st.active(1).last_item, ItemId::NONE, "a popped balloon is not 'used'");
}

#[test]
fn red_card_and_eject_button() {
    let mut o = Script::default();
    let p1: &[Set] = &[("garchomp", "roughskin", "", &["dragonclaw"]), ("gholdengo", "goodasgold", "", &["makeitrain"])];
    let mut st = battle(p1, &[("snorlax", "thickfat", "redcard", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_ne!(st.sides[0].active, 0, "the attacker was dragged out");
    assert_eq!(st.active(1).item, ItemId::NONE);
    let mut st = battle(p1, &[("snorlax", "thickfat", "ejectbutton", REST), ("garchomp", "roughskin", "", &["dragonclaw"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let op = st.options();
    assert_eq!(op[1].kind, Kind::Replace, "the holder must switch");
    assert_eq!(st.request, Request::Switch);
}

#[test]
fn white_herb_after_a_self_drop() {
    let mut o = Script::default();
    let mut st = battle(&[("lucario", "innerfocus", "whiteherb", &["closecombat"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).boosts[1], 0);
    assert_eq!(st.active(0).boosts[3], 0);
    assert_eq!(st.active(0).item, ItemId::NONE);
}

#[test]
fn choice_scarf_locks_after_the_move() {
    let mut o = Script::default();
    let mut st = battle(&[("alakazam", "magicguard", "choicescarf", &["psychic", "calmmind"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    assert_eq!(st.options()[0].move_list(), vec![0, 1]);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert!(st.sides[0].has_vol(conds::CHOICELOCK));
    assert_eq!(st.options()[0].move_list(), vec![0], "locked into Psychic");
}

#[test]
fn quick_claw_rolls_per_move_action() {
    let mut o = Script::default();
    let mut st = battle(&[("snorlax", "thickfat", "quickclaw", &["swordsdance"])], &[("mamoswine", "oblivious", "", &["iceshard"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(o.item_draws.iter().filter(|x| **x == "quickclaw").count(), 1);
}

#[test]
fn duration_items() {
    let mut o = Script::default();
    let st = battle(&[("torkoal", "drought", "heatrock", REST)], &[("snorlax", "thickfat", "", REST)], &mut o);
    assert_eq!(st.field.weather.id, conds::SUNNYDAY);
    assert_eq!(st.field.weather.duration, 8, "Heat Rock: eight turns");
    let mut st = battle(&[("klefki", "prankster", "lightclay", &["reflect"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.sides[0].conds().iter().find(|c| c.id == conds::REFLECT).unwrap().duration, 7);
}

#[test]
fn knock_off_sticky_hold_and_stones() {
    let mut o = Script::default();
    let p2: &[Set] = &[("swalot", "stickyhold", "leftovers", REST), ("garchomp", "roughskin", "garchompitez", &["swordsdance"]), ("snorlax", "thickfat", "sitrusberry", REST)];
    let mut st = battle(&[("kingambit", "defiant", "", &["knockoff"])], p2, &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).item, it("leftovers"), "Sticky Hold keeps it");
    st.step([m(0), sw(1)], &mut o).unwrap();
    assert_eq!(st.active(1).item, it("garchompitez"), "a stone of the holder's species stays");
    st.step([m(0), sw(2)], &mut o).unwrap();
    assert_eq!(st.active(1).item, ItemId::NONE);
    assert_eq!(st.active(1).last_item, ItemId::NONE, "Knock Off leaves nothing to recycle");
}

#[test]
fn trick_swaps_but_not_a_stone() {
    let mut o = Script::default();
    let mut st = battle(&[("alakazam", "synchronize", "choicescarf", &["trick"])], &[("snorlax", "thickfat", "leftovers", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("leftovers"));
    assert_eq!(st.active(1).item, it("choicescarf"));
    let mut st = battle(&[("alakazam", "synchronize", "alakazite", &["trick"])], &[("snorlax", "thickfat", "leftovers", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("alakazite"), "the user's own stone cannot leave");
    assert_eq!(st.active(1).item, it("leftovers"));
}

#[test]
fn thief_steals_into_an_empty_hand() {
    let mut o = Script::default();
    let mut st = battle(&[("liepard", "prankster", "", &["thief"])], &[("snorlax", "thickfat", "leftovers", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("leftovers"));
    assert_eq!(st.active(1).item, ItemId::NONE);
}

#[test]
fn fling_loses_the_item_even_into_protect() {
    let mut o = Script::default();
    let mut st = battle(&[("kingambit", "defiant", "ironball", &["fling"])], &[("snorlax", "thickfat", "", &["protect"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).hp, st.active(1).maxhp);
    assert_eq!(st.active(0).item, ItemId::NONE);
    assert_eq!(st.active(0).last_item, it("ironball"));
    assert!(!st.sides[0].has_vol(conds::FLING));
}

#[test]
fn acrobatics_belch_bug_bite_recycle() {
    let mut o = Script::default();
    // Acrobatics without an item hits harder
    let dmg = |item: &str, o: &mut Script| {
        let mut st = battle(&[("hawlucha", "limber", item, &["acrobatics"])], &[("snorlax", "thickfat", "", WAIT)], o);
        st.step([m(0), m(0)], o).unwrap();
        st.active(1).maxhp - st.active(1).hp
    };
    assert!(dmg("", &mut o) > dmg("leftovers", &mut o) * 3 / 2);
    // Belch before any berry: fails and spends PP; Bug Bite eats the foe's berry, then Belch works
    let mut st = battle(&[("hawlucha", "limber", "", &["belch", "bugbite"])], &[("snorlax", "thickfat", "oranberry", WAIT)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).hp, st.active(1).maxhp);
    assert_eq!(st.active(0).moves[0].pp, st.active(0).moves[0].max_pp - 1);
    st.step([m(1), m(0)], &mut o).unwrap();
    assert!(st.active(0).ate_berry);
    assert_eq!(st.active(1).item, ItemId::NONE);
    let hp = st.active(1).hp;
    st.step([m(0), m(0)], &mut o).unwrap();
    assert!(st.active(1).hp < hp, "Belch hits once a berry was eaten");
    // Recycle brings back an eaten berry
    let mut st = battle(&[("gholdengo", "goodasgold", "sitrusberry", &["bellydrum", "recycle"])], &[("kingambit", "defiant", "", WAIT)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, ItemId::NONE);
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("sitrusberry"));
    assert_eq!(st.active(0).last_item, ItemId::NONE);
}

#[test]
fn mega_garchomp_z_moves_with_its_new_speed_and_levitates() {
    let mut o = Script::default();
    // Garchomp 102 base speed against a 120 base speed foe: slower before, faster after the evolution
    let mut st = battle(&[("garchomp", "roughskin", "garchompitez", &["dracometeor", "earthpower"])], &[("froslass", "cursedbody", "", &["earthpower", "icebeam"])], &mut o);
    assert!(st.options()[0].mega);
    st.step([mega(0), m(0)], &mut o).unwrap();
    let g = *st.active(0);
    assert_eq!(g.species, SpeciesId::from_id("garchompmegaz").unwrap());
    assert_eq!(g.ability, AbilityId::from_id("levitate").unwrap());
    assert_eq!(g.base_ability, g.ability);
    assert_eq!(g.hp, g.maxhp, "Levitate: Earth Power misses the Mega");
    assert_eq!(g.item, it("garchompitez"), "the stone stays held");
    assert!(st.sides[0].mega_used());
    assert!(!st.options()[0].mega, "once per side");
}

#[test]
fn mega_golisopod_and_first_impression() {
    let mut o = Script::default();
    let mut st = battle(&[("golisopod", "emergencyexit", "golisopite", &["firstimpression"])], &[("snorlax", "thickfat", "", WAIT)], &mut o);
    st.step([mega(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).ability, AbilityId::from_id("toughclaws").unwrap());
    assert!(st.active(1).hp < st.active(1).maxhp, "First Impression still works on the Mega turn");
}

#[test]
fn mega_not_offered_while_locked() {
    let mut o = Script::default();
    let mut st = battle(&[("salamence", "intimidate", "salamencite", &["outrage", "roost"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert!(st.sides[0].has_vol(conds::LOCKEDMOVE));
    assert!(!st.options()[0].mega, "no Mega while locked into Outrage");
}

#[test]
fn stalwart_hits_the_stored_target_of_a_copied_locked_move() {
    let mut o = Script::default();
    // Garchomp outspeeds; Archaludon's Copycat copies Outrage and stores its own position as the target
    let p1: &[Set] = &[("archaludon", "stalwart", "", &["copycat", "irondefense"])];
    let mut st = battle(p1, &[("garchomp", "roughskin", "", &["outrage"]), ("garchomp", "roughskin", "", &["outrage"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert!(st.sides[0].has_vol(conds::LOCKEDMOVE));
    if st.options()[1].kind == Kind::Replace {
        let r = st.step([None, sw(1)], &mut o);
        assert!(r.is_ok(), "{:?} {:?}", r, st.options());
    }
    let hp = st.active(0).hp;
    // the locked move is not one of its slots: any move choice is it
    assert_eq!(st.options()[0].forced, Some("outrage"));
    st.step([m(cg_engine::sim::battle::STRUGGLE), m(0)], &mut o).unwrap();
    // its own locked Outrage hit itself (and Garchomp's)
    assert!(st.active(0).hp < hp || st.active(0).fainted);
}

#[test]
fn unburden_harvest_cud_chew() {
    let mut o = Script::default();
    // Fling uses the item up (AfterUseItem): Unburden's volatile
    let mut st = battle(&[("sceptile", "unburden", "sitrusberry", &["fling"])], &[("snorlax", "thickfat", "", WAIT)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert!(st.sides[0].has_vol(conds::UNBURDEN));
    // Harvest in sun: always restored
    let mut st = battle(&[("trevenant", "harvest", "sitrusberry", &["bellydrum", "sunnyday"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(1), m(0)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("sitrusberry"), "harvested at the end of the turn");
    // Cud Chew eats it again the turn after
    let mut st = battle(&[("taurospaldeacombat", "cudchew", "sitrusberry", &["bellydrum", "bulkup"])], &[("snorlax", "thickfat", "", WAIT)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let hp = st.active(0).hp;
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).hp, hp + st.active(0).maxhp / 4);
}

#[test]
fn opportunist_and_magician() {
    let mut o = Script::default();
    let mut st = battle(&[("espathra", "opportunist", "", &["calmmind"])], &[("garchomp", "roughskin", "", &["swordsdance"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).boosts[0], 2, "Swords Dance copied");
    let mut st = battle(&[("delphox", "magician", "", &["mysticalfire"])], &[("snorlax", "thickfat", "leftovers", SLAM)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).item, it("leftovers"));
}

#[test]
fn klutz_and_magic_room_stop_leftovers() {
    let mut o = Script::default();
    let mut st = battle(&[("lopunny", "klutz", "leftovers", &["bellydrum", "magicroom"])], &[("snorlax", "thickfat", "leftovers", &["bellydrum"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    let (a, b) = (*st.active(0), *st.active(1));
    assert_eq!(a.hp, a.maxhp - a.maxhp / 2, "Klutz: no heal");
    assert_eq!(b.hp, b.maxhp - b.maxhp / 2 + b.maxhp / 16);
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(1).hp, b.hp, "Magic Room: no heal");
}

#[test]
fn metronome_counts_consecutive_uses_and_mental_herb_cures() {
    let mut o = Script::default();
    let mut st = battle(&[("garchomp", "roughskin", "metronome", &["dragonclaw"])], &[("snorlax", "thickfat", "", REST)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.sides[0].vol(conds::METRONOME).unwrap().data, 1);
    let mut st = battle(&[("grimmsnarl", "prankster", "", &["taunt"])], &[("snorlax", "thickfat", "mentalherb", &["curse"])], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    assert!(!st.sides[1].has_vol(conds::TAUNT));
    assert_eq!(st.active(1).item, ItemId::NONE);
}

#[test]
fn terrain_seed_on_entry() {
    let mut o = Script::default();
    let st = battle(&[("rillaboom", "grassysurge", "", &["woodhammer"])], &[("snorlax", "thickfat", "grassyseed", REST)], &mut o);
    assert_eq!(st.active(1).item, ItemId::NONE);
    assert_eq!(st.active(1).boosts[1], 1);
}

#[test]
fn future_sight_life_orb_and_explosion_gem() {
    let mut o = Script::default();
    // Future Sight's condition calls the user's Life Orb when the attack lands
    let mut st = battle(&[("alakazam", "synchronize", "lifeorb", &["futuresight", "calmmind"])], &[("snorlax", "thickfat", "", WAIT)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).hp, st.active(0).maxhp);
    st.step([m(1), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).hp, st.active(0).maxhp - st.active(0).maxhp / 10);
    // Explosion: the user faints before the hit, the gem's condition still applies
    let dmg = |item: &str, o: &mut Script| {
        let mut st = battle(&[("snorlax", "thickfat", item, &["explosion"]), ("garchomp", "roughskin", "", &["earthquake"])], &[("corviknight", "pressure", "", &["roost"])], o);
        let hp = st.active(1).hp;
        st.step([m(0), m(0)], o).unwrap();
        hp - st.active(1).hp
    };
    let (with, without) = (dmg("normalgem", &mut o), dmg("", &mut o));
    assert!(with > without, "{} {}", with, without);
}

#[test]
fn unimplemented_ability_still_marks_the_battle() {
    let mut o = Script::default();
    let st = battle(&[("garchomp", "neutralizinggas", "", &["earthquake"])], &[("snorlax", "thickfat", "leftovers", REST)], &mut o);
    assert!(st.unimpl.count > 0);
    // every legal item is implemented
    for i in ITEMS.iter().skip(1) {
        assert!(cg_engine::sim::items::item_implemented(i), "{}", i.id);
    }
}

const TOSS: &[&str] = &["seismictoss", "calmmind"];

#[test]
fn berserk_and_anger_shell_check_before_healing_berries() {
    let mut o = Script::default();
    // Drampa 153 HP; two Seismic Tosses: 103, then 53 (crosses half): the boost, then the berry
    let mut st = battle(&[("drampa", "berserk", "sitrusberry", &["irondefense"])], &[("corviknight", "pressure", "", TOSS)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(st.active(0).boosts[2], 1, "Berserk saw 53 HP");
    assert_eq!((st.active(0).hp, st.active(0).item), (91, ItemId::NONE), "then Sitrus healed above half");
    let mut st = battle(&[("drampa", "berserk", "oranberry", &["irondefense"])], &[("corviknight", "pressure", "", TOSS)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!((st.active(0).boosts[2], st.active(0).hp), (1, 63));
    let mut st = battle(&[("drampa", "angershell", "sitrusberry", &["calmmind"])], &[("corviknight", "pressure", "", TOSS)], &mut o);
    st.step([m(0), m(0)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!(&st.active(0).boosts[..5], &[1, -1, 3, 1, 1], "Anger Shell, then Calm Mind");
    assert_eq!(st.active(0).hp, 91);
    // behind a Substitute: the hit on the doll changes nothing; the next one crosses half
    let mut st = battle(&[("drampa", "berserk", "sitrusberry", &["irondefense", "substitute"])], &[("corviknight", "pressure", "", TOSS)], &mut o);
    st.step([m(1), m(1)], &mut o).unwrap();
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!((st.active(0).hp, st.active(0).boosts[2]), (115, 0), "the doll took the hit");
    st.step([m(0), m(0)], &mut o).unwrap();
    assert_eq!((st.active(0).hp, st.active(0).boosts[2]), (103, 1));
    // a Sheer Force attacker: the berry still waits (Champions)
    let mut st = battle(&[("drampa", "berserk", "sitrusberry", &["irondefense"])], &[("corviknight", "sheerforce", "", &["crunch"])], &mut o);
    for _ in 0..3 {
        let before = st.active(0).hp;
        st.step([m(0), m(0)], &mut o).unwrap();
        if st.active(0).item.is_none() {
            assert!(before > 76, "crossed half on this hit");
            assert_eq!(st.active(0).boosts[2], 1);
            break;
        }
    }
    assert!(st.active(0).item.is_none());
}

#[test]
fn berserk_multi_hit_and_confusion() {
    let mut o = Script::default();
    // Super Fang to 77, then Double Hit: the berry is eaten between the hits; the boost only if the last hit
    // left it at half or below
    for berry in ["sitrusberry", "oranberry"] {
        let mut st = battle(&[("drampa", "berserk", berry, &["irondefense"])], &[("corviknight", "pressure", "", &["superfang", "doublehit"])], &mut o);
        st.step([m(0), m(0)], &mut o).unwrap();
        assert_eq!(st.active(0).hp, 77);
        st.step([m(0), m(1)], &mut o).unwrap();
        let a = st.active(0);
        assert!(a.item.is_none(), "{} eaten", berry);
        assert_eq!(a.boosts[2], (a.hp <= 76) as i8, "{}: {} HP", berry, a.hp);
    }
    // a confusion self-hit is a move: the berry waits, also while the chosen move is a multi-hit one
    for chosen in [0u8, 2] {
        let mut st = battle(&[("drampa", "berserk", "sitrusberry", &["calmmind", "bellydrum", "doublehit"])], &[("corviknight", "pressure", "", &["calmmind", "confuseray"])], &mut o);
        st.step([m(1), m(0)], &mut o).unwrap();
        assert_eq!(st.active(0).hp, 77);
        o.procs = true;
        st.step([m(chosen), m(1)], &mut o).unwrap();
        let hp = st.active(0).hp;
        assert!(hp <= 76, "hit itself below half");
        assert_eq!(st.active(0).item, it("sitrusberry"), "the berry waits");
        assert_eq!(st.active(0).boosts[2], 0);
        o.procs = false;
        st.step([m(0), m(0)], &mut o).unwrap();
        assert!(st.active(0).item.is_none(), "eaten after its own move");
        assert_eq!(st.active(0).hp, hp + 38);
    }
}
