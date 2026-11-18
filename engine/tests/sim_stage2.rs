//! Stage-2 rules that random battles reach rarely, played through the public API with scripted outcomes:
//! the third-attempt thaw, Toxic's counter restarting on a switch-in, the Perish Song count, Destiny Bond,
//! both last Pokémon fainting on one action, and a pivot move into a replacement mid-turn.

use cg_engine::sim::data::*;
use cg_engine::sim::outcome::{EffectRef, Key, Outcomes, Site};
use cg_engine::sim::state::{State, Winner};
use cg_engine::sim::{Choice, Kind, MonSpec};

/// Accuracy always hits; no crits, no secondaries, no thaw, no full paralysis, no confusion self-hit;
/// rolls at their highest damage; ties go to the first named. Counts the draws by key.
#[derive(Default)]
struct Script {
    draws: Vec<Key>,
}

impl Outcomes for Script {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        self.draws.push(site.key);
        matches!(site.key, Key::Accuracy | Key::Protect)
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

/// Six-Pokémon teams where every member has `moves`; leads are slot 0.
fn battle(p1: (&str, &[&str]), p2: (&str, &[&str]), o: &mut Script) -> State {
    let a: Vec<MonSpec> = (0..6).map(|_| spec(p1.0, p1.1)).collect();
    let b: Vec<MonSpec> = (0..6).map(|_| spec(p2.0, p2.1)).collect();
    State::start([&a, &b], [[0, 1, 2], [0, 1, 2]], o).unwrap()
}

fn mv_choice(i: u8) -> Option<Choice> {
    Some(Choice::Move { index: i, mega: false })
}

#[test]
fn freeze_thaws_by_the_third_attempt() {
    let mut o = Script::default();
    let mut st = battle(("snorlax", &["swordsdance"]), ("snorlax", &["swordsdance"]), &mut o);
    st.sides[0].mons[0].status = Status::Frz;
    st.sides[0].mons[0].status_time = 3;
    for turn in 1..=3 {
        o.draws.clear();
        st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
        let m = &st.sides[0].mons[0];
        let thaw_draws = o.draws.iter().filter(|k| **k == Key::Thaw).count();
        if turn < 3 {
            assert_eq!(m.status, Status::Frz, "turn {}", turn);
            assert_eq!(thaw_draws, 1, "a 1/4 thaw draw on attempt {}", turn);
            assert_eq!(m.boosts[0], 0, "frozen: no move");
            assert_eq!(m.moves[0].pp, m.moves[0].max_pp, "no PP spent while frozen");
        } else {
            assert_eq!(m.status, Status::None, "thawed on the third attempt");
            assert_eq!(thaw_draws, 0, "the third attempt thaws without a draw");
            assert_eq!(m.boosts[0], 2, "and moves that turn");
            assert_eq!(m.moves[0].pp, m.moves[0].max_pp - 1);
        }
    }
}

#[test]
fn toxic_counter_restarts_on_switch_in() {
    let mut o = Script::default();
    let mut st = battle(("snorlax", &["swordsdance"]), ("snorlax", &["swordsdance"]), &mut o);
    st.sides[0].mons[0].status = Status::Tox;
    let per = (st.sides[0].mons[0].base_maxhp / 16) as u16;
    for stage in 1..=3u16 {
        let hp = st.sides[0].mons[0].hp;
        st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
        assert_eq!(st.sides[0].mons[0].tox_stage as u16, stage);
        assert_eq!(hp - st.sides[0].mons[0].hp, per * stage, "floor(maxhp/16) * stage");
    }
    st.step([Some(Choice::Switch(1)), mv_choice(0)], &mut o).unwrap();
    assert_eq!(st.sides[0].mons[0].tox_stage, 3, "the stage stays while benched");
    let hp = st.sides[0].mons[0].hp;
    st.step([Some(Choice::Switch(0)), mv_choice(0)], &mut o).unwrap();
    assert_eq!(st.sides[0].mons[0].tox_stage, 1, "back to 0 on switch-in, then one residual");
    assert_eq!(hp - st.sides[0].mons[0].hp, per);
}

#[test]
fn perish_song_counts_three_turns_then_faints() {
    let mut o = Script::default();
    let mut st = battle(("snorlax", &["perishsong", "swordsdance"]), ("snorlax", &["swordsdance"]), &mut o);
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    for left in [3i8, 2, 1] {
        for side in 0..2 {
            let v = st.sides[side].vol(conds::PERISHSONG).expect("perish song");
            assert_eq!(v.duration, left);
        }
        if left > 1 {
            st.step([mv_choice(1), mv_choice(0)], &mut o).unwrap();
        }
    }
    st.step([mv_choice(1), mv_choice(0)], &mut o).unwrap();
    assert!(st.sides[0].mons[0].fainted && st.sides[1].mons[0].fainted, "both faint at the fourth residual");
    let opts = st.options();
    assert_eq!((opts[0].kind, opts[1].kind), (Kind::Replace, Kind::Replace));
}

#[test]
fn destiny_bond_takes_the_attacker_down() {
    let mut o = Script::default();
    let mut st = battle(("gengar", &["destinybond"]), ("garchomp", &["dragonclaw"]), &mut o);
    st.sides[0].mons[0].stats[4] = 400;
    st.sides[0].mons[0].hp = 1;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    assert!(st.sides[0].mons[0].fainted, "the Destiny Bond user is knocked out");
    assert!(st.sides[1].mons[0].fainted, "and takes its attacker with it");
    // used again before its next move, Destiny Bond lasts: a second use fails (onPrepareHit)
    let mut o = Script::default();
    let mut st = battle(("gengar", &["destinybond", "swordsdance"]), ("garchomp", &["swordsdance"]), &mut o);
    st.sides[0].mons[0].stats[4] = 400;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    assert!(st.sides[0].has_vol(conds::DESTINYBOND), "lasts until the user's next move");
    st.step([mv_choice(1), mv_choice(0)], &mut o).unwrap();
    assert!(!st.sides[0].has_vol(conds::DESTINYBOND), "removed before the next move");
}

#[test]
fn both_last_pokemon_faint_on_one_action() {
    let mut o = Script::default();
    let mut st = battle(("snorlax", &["explosion"]), ("garchomp", &["swordsdance"]), &mut o);
    for side in 0..2 {
        for slot in 1..3 {
            st.sides[side].mons[slot].hp = 0;
            st.sides[side].mons[slot].fainted = true;
        }
        st.sides[side].pokemon_left = 1;
    }
    st.sides[1].mons[0].hp = 1;
    st.sides[0].mons[0].stats[4] = 400;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    assert!(st.ended);
    assert!(st.sides[0].mons[0].fainted && st.sides[1].mons[0].fainted);
    // the user faints first (selfdestruct "always"), the target last: the target's side wins
    assert_eq!(st.winner, Winner::Side(1));
}

#[test]
fn pivot_move_into_a_replacement() {
    let mut o = Script::default();
    let mut st = battle(("garchomp", &["uturn"]), ("snorlax", &["dragonclaw"]), &mut o);
    st.sides[0].mons[0].stats[4] = 400;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    let opts = st.options();
    assert_eq!((opts[0].kind, opts[1].kind), (Kind::Replace, Kind::Wait), "the turn stops for the pivot");
    assert!(st.mid_turn);
    let full = st.sides[0].mons[1].hp;
    assert_eq!(full, st.sides[0].mons[1].maxhp);
    st.step([Some(Choice::Switch(1)), None], &mut o).unwrap();
    assert_eq!(st.sides[0].active, 1);
    assert!(st.sides[0].mons[1].hp < full, "the waiting move hits the replacement");
    assert_eq!(st.sides[0].mons[0].hp, st.sides[0].mons[0].maxhp, "not the U-turn user");
    assert!(!st.mid_turn && st.turn == 2);
}

#[test]
fn recharge_turn_is_not_spent_by_a_replacement() {
    // N14: after Hyper Beam knocks the target out, the opponent's replacement is not the recharge turn
    let mut o = Script::default();
    let mut st = battle(("snorlax", &["hyperbeam", "swordsdance"]), ("garchomp", &["swordsdance"]), &mut o);
    st.sides[0].mons[0].stats[4] = 400;
    st.sides[1].mons[0].hp = 1;
    st.step([mv_choice(0), mv_choice(0)], &mut o).unwrap();
    let opts = st.options();
    assert_eq!((opts[0].kind, opts[1].kind), (Kind::Wait, Kind::Replace));
    st.step([None, Some(Choice::Switch(1))], &mut o).unwrap();
    let opts = st.options();
    assert_eq!(opts[0].forced, Some("recharge"), "the next turn is the recharge turn");
    assert!(opts[0].switches.iter().all(|x| !x), "and the recharging Pokémon cannot switch");
}
