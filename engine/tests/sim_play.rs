//! The new resolver through its public API: seeded random plain battles end with a winner and keep the
//! state consistent; the same seed plays the same battle; a copied state forks; a recorded tape replays
//! through the forced source with no event missing or left over.

use cg_engine::sim::data::*;
use cg_engine::sim::outcome::{Forced, Recording, Seeded, TapeEvent, TapeValue, Value};
use cg_engine::sim::state::{State, Winner};
use cg_engine::sim::{Choice, Kind, MonSpec};

fn team(rng: &mut Seeded) -> Vec<MonSpec> {
    let legal: Vec<usize> = (0..SPECIES.len()).filter(|&i| !SPECIES[i].is_mega).collect();
    (0..6)
        .map(|_| {
            let sp = SpeciesId(legal[rng.below(legal.len() as u32) as usize] as u16);
            let mut moves = [MoveId::NONE; 4];
            let mut n = 0;
            while n < 4 {
                let m = PLAIN_MOVES[rng.below(PLAIN_MOVES.len() as u32) as usize];
                if !moves[..n].contains(&m) {
                    moves[n] = m;
                    n += 1;
                }
            }
            let mut evs = [0u8; 6];
            for e in evs.iter_mut() {
                *e = rng.below(33) as u8;
            }
            MonSpec {
                species: sp,
                ability: abilities::NOABILITY,
                item: ItemId::NONE,
                moves,
                nmoves: 4,
                nature: rng.below(25) as u8,
                evs,
                gender: 1,
            }
        })
        .collect()
}

fn choose(st: &State, rng: &mut Seeded) -> [Option<Choice>; 2] {
    let opts = st.options();
    let mut ch = [None, None];
    for side in 0..2 {
        if matches!(opts[side].kind, Kind::Turn | Kind::Replace) {
            let cs = opts[side].choices();
            assert!(!cs.is_empty());
            ch[side] = Some(cs[rng.below(cs.len() as u32) as usize]);
        }
    }
    ch
}

fn battle(seed: u64) -> (State, usize) {
    let mut rng = Seeded::new(seed);
    let (a, b) = (team(&mut rng), team(&mut rng));
    let mut st = State::start([&a, &b], [[0, 1, 2], [3, 4, 5]], &mut rng).unwrap();
    let mut steps = 0;
    while !st.ended {
        let ch = choose(&st, &mut rng);
        st.step(ch, &mut rng).unwrap();
        steps += 1;
        assert!(steps < 2000, "battle {} does not end", seed);
        for side in &st.sides {
            for m in &side.mons {
                assert!(m.hp <= m.maxhp);
                assert_eq!(m.fainted, m.hp == 0);
            }
        }
    }
    (st, steps)
}

#[test]
fn seeded_battles_end_with_a_winner() {
    for seed in 0..300 {
        let (st, _) = battle(seed);
        let left = |s: usize| st.sides[s].mons.iter().filter(|m| !m.fainted).count();
        match st.winner {
            Winner::Side(w) => assert_eq!(left(1 - w as usize), 0, "seed {}", seed),
            _ => panic!("seed {}: no winner", seed),
        }
        assert_eq!(st.unimpl.count, 0, "plain battles meet no unimplemented effect");
    }
}

#[test]
fn same_seed_same_battle_and_forks() {
    assert_eq!(battle(7), battle(7));
    let mut rng = Seeded::new(99);
    let (a, b) = (team(&mut rng), team(&mut rng));
    let mut st = State::start([&a, &b], [[0, 1, 2], [0, 1, 2]], &mut rng).unwrap();
    let ch = choose(&st, &mut rng);
    st.step(ch, &mut rng).unwrap();
    let fork = st;
    let (mut r1, mut r2) = (rng.clone(), rng.clone());
    let (mut x, mut y) = (st, fork);
    while !x.ended {
        let c = choose(&x, &mut r1);
        x.step(c, &mut r1).unwrap();
        let c = choose(&y, &mut r2);
        y.step(c, &mut r2).unwrap();
    }
    assert_eq!(x, y);
}

#[test]
fn recorded_tape_replays_forced() {
    let mut rng = Seeded::new(5);
    let (a, b) = (team(&mut rng), team(&mut rng));
    let mut rec = Recording::new(rng.clone());
    let mut st = State::start([&a, &b], [[0, 1, 2], [2, 1, 0]], &mut rec).unwrap();
    let mut replay = st;
    let mut choices_rng = Seeded::new(6);
    while !st.ended {
        let ch = choose(&st, &mut choices_rng);
        let mut r = Recording::new(rec.inner.clone());
        st.step(ch, &mut r).unwrap();
        rec.inner = r.inner.clone();
        let tape: Vec<TapeEvent> = r
            .events
            .iter()
            .map(|e| TapeEvent {
                key: e.site.key,
                side: e.site.side,
                mv: e.site.mv.map(|m| m.id().to_string()).or(e.site.mv_name.map(String::from)),
                hit: e.site.hit.map(|h| h as i64),
                index: cg_engine::sim::outcome::index_string(&e.site.index),
                effect: cg_engine::sim::outcome::effect_string(&e.site.effect),
                value: match e.value {
                    Value::Bool(b) => TapeValue::Bool(b),
                    Value::Int(i) => TapeValue::Int(i as i64),
                    Value::Side(s) => TapeValue::Str(["p1", "p2"][s as usize].to_string()),
                    Value::Effect(x) => TapeValue::Str(x.id().to_string()),
                    Value::Str(x) => TapeValue::Str(x.to_string()),
                },
            })
            .collect();
        let mut f = Forced::new(tape);
        replay.step(ch, &mut f).unwrap();
        f.finish().unwrap();
        assert_eq!(st, replay);
    }
}

#[test]
fn struggle_when_no_pp() {
    let mut rng = Seeded::new(3);
    let (a, b) = (team(&mut rng), team(&mut rng));
    let mut st = State::start([&a, &b], [[0, 1, 2], [0, 1, 2]], &mut rng).unwrap();
    for ms in st.sides[0].mons[0].moves.iter_mut() {
        ms.pp = 0;
    }
    let o = st.options();
    assert!(o[0].struggle && !o[0].mega && o[0].move_list().is_empty());
    assert!(st.disabled_flags(0, 0).iter().all(|x| *x));
    st.step([Some(Choice::Move { index: 0, mega: false }), o[1].choices().first().copied()], &mut rng).unwrap();
    assert_eq!(st.sides[0].mons[0].last_move, moves::STRUGGLE);
    assert!(st.sides[0].mons[0].hp < st.sides[0].mons[0].maxhp || st.sides[0].mons[0].fainted);
}
