//! Stage-3 rules that random battles reach rarely, played through the public API with scripted outcomes:
//! both sides switching into hazards at once, Toxic Spikes absorbed by a Poison type, a critical hit
//! through a screen, Future Sight landing on a replacement, Wish healing a switch-in, a weather set on the
//! turn the old one would end, Trick Room's order and its speed ties, and Defog blocked by Protect.

use cg_engine::sim::data::*;
use cg_engine::sim::outcome::{EffectRef, Key, Outcomes, Site};
use cg_engine::sim::state::{SideCond, State, Winner};
use cg_engine::sim::{Choice, Kind, MonSpec};

/// Accuracy always hits; crits as `crit` says; no secondaries or other chances; rolls at their highest
/// damage; ties go to the first named. Counts the draws by key.
#[derive(Default)]
struct Script {
    crit: bool,
    draws: Vec<Key>,
}

impl Outcomes for Script {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        self.draws.push(site.key);
        match site.key {
            Key::Accuracy | Key::Protect => true,
            Key::Crit => self.crit,
            _ => false,
        }
    }
    fn range(&mut self, site: &Site, lo: i32, _: i32) -> i32 {
        self.draws.push(site.key);
        lo
    }
    fn sample(&mut self, site: &Site, values: &[i32]) -> i32 {
        self.draws.push(site.key);
        values[0]
    }
    fn tie(&mut self, site: &Site, a: u8, _: u8) -> u8 {
        self.draws.push(site.key);
        a
    }
    fn tie_effects(&mut self, site: &Site, _: EffectRef, _: EffectRef) -> bool {
        self.draws.push(site.key);
        true
    }
    fn sample_str(&mut self, site: &Site, _: &[&'static str]) -> usize {
        self.draws.push(site.key);
        0
    }
}

fn mv(id: &str) -> MoveId {
    MoveId::from_id(id).unwrap_or_else(|| panic!("move {}", id))
}

fn spec(species: &str, moves: &[&str]) -> MonSpec {
    let mut m = [MoveId::NONE; 4];
    for (i, x) in moves.iter().enumerate() {
        m[i] = mv(x);
    }
    MonSpec {
        species: SpeciesId::from_id(species).unwrap_or_else(|| panic!("species {}", species)),
        ability: abilities::NOABILITY,
        item: ItemId::NONE,
        moves: m,
        nmoves: moves.len() as u8,
        nature: nature_from_id("serious").unwrap(),
        evs: [0; 6],
        gender: 1,
    }
}

/// Teams of three kinds of Pokémon (slots 0, 1, 2 brought in that order); the rest of the six repeat slot 0.
fn battle(p1: [(&str, &[&str]); 3], p2: [(&str, &[&str]); 3], o: &mut Script) -> State {
    let team = |t: [(&str, &[&str]); 3]| -> Vec<MonSpec> {
        let mut v: Vec<MonSpec> = t.iter().map(|(s, m)| spec(s, m)).collect();
        while v.len() < 6 {
            v.push(spec(t[0].0, t[0].1));
        }
        v
    };
    State::start([&team(p1), &team(p2)], [[0, 1, 2], [0, 1, 2]], o).unwrap()
}

fn mv_choice(i: u8) -> Option<Choice> {
    Some(Choice::Move { index: i, mega: false })
}

fn add_side(st: &mut State, side: usize, c: Cond, layers: u8) {
    let sd = &mut st.sides[side];
    st.effect_order += 1;
    sd.conds[sd.nconds as usize] = SideCond { id: c, layers, duration: 0, order: st.effect_order, ..Default::default() };
    sd.nconds += 1;
}

fn has_side(st: &State, side: usize, c: Cond) -> bool {
    st.sides[side].conds().iter().any(|x| x.id == c)
}

#[test]
fn both_sides_switch_into_hazards_at_once() {
    let splash: &[&str] = &["swordsdance"];
    let mk = |o: &mut Script| {
        let mut st = battle(
            [("snorlax", &["explosion"]), ("blastoise", splash), ("blastoise", splash)],
            [("snorlax", splash), ("blastoise", splash), ("blastoise", splash)],
            o,
        );
        add_side(&mut st, 0, conds::STEALTHROCK, 1);
        add_side(&mut st, 1, conds::STEALTHROCK, 1);
        st.sides[0].mons[0].stats[4] = 400;
        st.sides[1].mons[0].hp = 1;
        st
    };
    // both leads fall to Explosion; both replacements enter together and both take Stealth Rock
    let mut o = Script::default();
    let mut st = mk(&mut o);
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    let opts = st.options();
    assert_eq!((opts[0].kind, opts[1].kind), (Kind::Replace, Kind::Replace));
    st.step([Some(Choice::Switch(1)), Some(Choice::Switch(1))], &mut o).unwrap();
    for side in 0..2 {
        let m = &st.sides[side].mons[1];
        // Blastoise (Water): Rock is neutral, maxhp / 8
        assert_eq!(m.maxhp - m.hp, m.maxhp / 8, "side {}", side);
    }
    // both at 1 HP, last Pokémon each: the faster one's Stealth Rock comes first, it faints, and the battle
    // ends before the slower one is hit
    let mut o = Script::default();
    let mut st = mk(&mut o);
    for side in 0..2 {
        st.sides[side].mons[1].hp = 1;
        st.sides[side].mons[2].hp = 0;
        st.sides[side].mons[2].fainted = true;
        st.sides[side].pokemon_left = 2;
    }
    st.sides[0].mons[1].stats[4] = 300;
    st.sides[0].mons[1].speed = 300;
    st.sides[1].mons[1].stats[4] = 100;
    st.sides[1].mons[1].speed = 100;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    st.step([Some(Choice::Switch(1)), Some(Choice::Switch(1))], &mut o).unwrap();
    assert!(st.ended);
    assert_eq!(st.winner, Winner::Side(1));
    assert!(st.sides[0].mons[1].fainted);
    assert_eq!(st.sides[1].mons[1].hp, 1, "the slower switch-in was never hit");
}

#[test]
fn toxic_spikes_absorbed_by_a_grounded_poison_type() {
    let mut o = Script::default();
    let splash: &[&str] = &["swordsdance"];
    let mut st = battle(
        [("snorlax", splash), ("snorlax", splash), ("snorlax", splash)],
        [("snorlax", splash), ("toxapex", splash), ("garchomp", splash)],
        &mut o,
    );
    add_side(&mut st, 1, conds::TOXICSPIKES, 2);
    st.step([mv_choice(0), Some(Choice::Switch(2))], &mut o).unwrap();
    assert_eq!(st.sides[1].mons[2].status, Status::Tox, "two layers: bad poison");
    assert!(has_side(&st, 1, conds::TOXICSPIKES));
    st.step([mv_choice(0), Some(Choice::Switch(1))], &mut o).unwrap();
    assert_eq!(st.sides[1].mons[1].status, Status::None, "a grounded Poison type is not poisoned");
    assert!(!has_side(&st, 1, conds::TOXICSPIKES), "and takes the spikes away");
}

#[test]
fn critical_hit_ignores_a_screen() {
    let hit = |reflect: bool, crit: bool| -> u16 {
        let mut o = Script { crit, ..Default::default() };
        let mut st = battle(
            [("garchomp", &["earthquake"]), ("garchomp", &["earthquake"]), ("garchomp", &["earthquake"])],
            [("snorlax", &["swordsdance"]), ("snorlax", &["swordsdance"]), ("snorlax", &["swordsdance"])],
            &mut o,
        );
        if reflect {
            add_side(&mut st, 1, conds::REFLECT, 1);
            st.sides[1].conds[0].duration = 5;
        }
        let hp = st.sides[1].mons[0].hp;
        st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
        hp - st.sides[1].mons[0].hp
    };
    let (plain, screened) = (hit(false, false), hit(true, false));
    assert!(screened < plain && screened >= plain / 2 - 1 && screened <= plain / 2 + 1, "{} {}", plain, screened);
    assert_eq!(hit(true, true), hit(false, true), "a critical hit goes through Reflect");
    assert!(hit(true, true) > plain);
}

#[test]
fn future_sight_lands_on_a_replacement() {
    let mut o = Script::default();
    let splash: &[&str] = &["swordsdance"];
    let mut st = battle(
        [("alakazam", &["futuresight", "swordsdance"]), ("alakazam", splash), ("alakazam", splash)],
        [("snorlax", splash), ("garchomp", splash), ("garchomp", splash)],
        &mut o,
    );
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    assert!(st.sides[1].slot_conds().iter().any(|c| c.id == conds::FUTUREMOVE), "stored on the target's slot");
    let full = st.sides[1].mons[1].maxhp;
    st.step([mv_choice(1), Some(Choice::Switch(1))], &mut o).unwrap();
    assert_eq!(st.sides[1].mons[1].hp, full, "not yet");
    assert_eq!(st.sides[1].mons[0].hp, st.sides[1].mons[0].maxhp);
    st.step([mv_choice(1), mv_choice(0)], &mut o).unwrap();
    assert!(st.sides[1].mons[1].hp < full, "the replacement in the slot takes the hit at the end of the second turn");
    assert_eq!(st.sides[1].mons[0].hp, st.sides[1].mons[0].maxhp, "not the Pokémon it was aimed at");
    assert!(st.sides[1].slot_conds().is_empty());
}

#[test]
fn wish_heals_the_pokemon_that_switched_in() {
    let mut o = Script::default();
    let splash: &[&str] = &["swordsdance"];
    let mut st = battle(
        [("snorlax", &["wish", "swordsdance"]), ("garchomp", splash), ("garchomp", splash)],
        [("snorlax", splash), ("snorlax", splash), ("snorlax", splash)],
        &mut o,
    );
    let wisher_half = st.sides[0].mons[0].maxhp / 2;
    st.sides[0].mons[1].hp = 10;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    assert_eq!(st.sides[0].mons[1].hp, 10);
    st.step([Some(Choice::Switch(1)), mv_choice(0)], &mut o).unwrap();
    let m = &st.sides[0].mons[1];
    assert_eq!(m.hp, (10 + wisher_half).min(m.maxhp), "half of the wisher's max HP, to whoever is in the slot");
    assert!(st.sides[0].slot_conds().is_empty());
}

#[test]
fn weather_set_on_the_turn_the_old_one_ends() {
    let mut o = Script::default();
    let s: &[&str] = &["raindance", "sunnyday", "swordsdance"];
    let mut st = battle([("snorlax", s), ("snorlax", s), ("snorlax", s)], [("snorlax", s), ("snorlax", s), ("snorlax", s)], &mut o);
    st.sides[0].mons[0].stats[4] = 400;
    st.step([mv_choice(0), mv_choice(2)], &mut o).unwrap();
    assert_eq!((st.field.weather.id, st.field.weather.duration), (conds::RAINDANCE, 4));
    for left in [3, 2, 1] {
        st.step([mv_choice(2), mv_choice(2)], &mut o).unwrap();
        assert_eq!(st.field.weather.duration, left);
    }
    // the rain's last turn: Sunny Day replaces it; the sun counts its own five turns from here
    st.step([mv_choice(1), mv_choice(2)], &mut o).unwrap();
    assert_eq!((st.field.weather.id, st.field.weather.duration), (conds::SUNNYDAY, 4));
    // without a new weather, the last turn ends it
    let mut o = Script::default();
    let mut st = battle([("snorlax", s), ("snorlax", s), ("snorlax", s)], [("snorlax", s), ("snorlax", s), ("snorlax", s)], &mut o);
    st.step([mv_choice(0), mv_choice(2)], &mut o).unwrap();
    for _ in 0..4 {
        st.step([mv_choice(2), mv_choice(2)], &mut o).unwrap();
    }
    assert!(st.field.weather.id.is_none());
}

#[test]
fn trick_room_reverses_order_and_keeps_ties() {
    let mut o = Script::default();
    let s: &[&str] = &["trickroom", "bodyslam"];
    let mut st = battle([("snorlax", s), ("snorlax", s), ("snorlax", s)], [("snorlax", s), ("snorlax", s), ("snorlax", s)], &mut o);
    st.sides[0].mons[0].stats[4] = 50;
    st.sides[1].mons[0].stats[4] = 200;
    st.step([mv_choice(0), Some(Choice::Move { index: 1, mega: false })], &mut o).unwrap();
    assert!(st.field.pseudo[..st.field.npseudo as usize].iter().any(|c| c.id == conds::TRICKROOM));
    // the slower Snorlax now moves first: its Tackle lands before the faster one's
    let h1 = st.sides[0].mons[0].hp;
    st.sides[1].mons[0].hp = 1;
    st.step([mv_choice(1), mv_choice(1)], &mut o).unwrap();
    assert!(st.sides[1].mons[0].fainted, "the slower side hit first");
    assert_eq!(st.sides[0].mons[0].hp, h1, "the faster side never moved");
    // equal speeds under Trick Room still tie: the order is drawn
    let mut o = Script::default();
    let mut st = battle([("snorlax", s), ("snorlax", s), ("snorlax", s)], [("snorlax", s), ("snorlax", s), ("snorlax", s)], &mut o);
    st.step([mv_choice(0), mv_choice(1)], &mut o).unwrap();
    o.draws.clear();
    st.step([mv_choice(1), mv_choice(1)], &mut o).unwrap();
    assert!(o.draws.iter().any(|k| *k == Key::SpeedTie), "a tie is drawn");
}

#[test]
fn defog_blocked_by_protect_removes_nothing() {
    let mut o = Script::default();
    let mut st = battle(
        [("corviknight", &["defog"]), ("corviknight", &["defog"]), ("corviknight", &["defog"])],
        [("snorlax", &["protect", "swordsdance"]), ("snorlax", &["swordsdance"]), ("snorlax", &["swordsdance"])],
        &mut o,
    );
    add_side(&mut st, 0, conds::STEALTHROCK, 1);
    add_side(&mut st, 1, conds::SPIKES, 1);
    add_side(&mut st, 1, conds::REFLECT, 1);
    st.sides[1].conds[1].duration = 5;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    assert!(has_side(&st, 0, conds::STEALTHROCK) && has_side(&st, 1, conds::SPIKES) && has_side(&st, 1, conds::REFLECT));
    assert_eq!(st.sides[1].mons[0].boosts[B_EVA], 0);
    // not protected: the target's hazards and screen and the user's hazards go
    st.step([mv_choice(0), mv_choice(1)], &mut o).unwrap();
    assert!(!has_side(&st, 0, conds::STEALTHROCK) && !has_side(&st, 1, conds::SPIKES) && !has_side(&st, 1, conds::REFLECT));
    assert_eq!(st.sides[1].mons[0].boosts[B_EVA], -1);
}

#[test]
fn gravity_grounds_and_disables() {
    let mut o = Script::default();
    let s1: &[&str] = &["gravity", "earthquake"];
    let s2: &[&str] = &["fly", "swordsdance"];
    let mut st = battle([("snorlax", s1), ("snorlax", s1), ("snorlax", s1)], [("corviknight", s2), ("corviknight", s2), ("corviknight", s2)], &mut o);
    st.sides[0].mons[0].stats[4] = 400;
    st.step([mv_choice(1), mv_choice(1)], &mut o).unwrap();
    assert_eq!(st.sides[1].mons[0].hp, st.sides[1].mons[0].maxhp, "a Flying type is not hit by Earthquake");
    st.step([mv_choice(0), mv_choice(1)], &mut o).unwrap();
    assert!(!st.options()[1].moves[0], "Fly cannot be chosen under Gravity");
    st.step([mv_choice(1), mv_choice(1)], &mut o).unwrap();
    assert!(st.sides[1].mons[0].hp < st.sides[1].mons[0].maxhp, "grounded: Earthquake hits");
}
