//! Speed of the new resolver (`cg_engine::sim`): random legal choices, seeded outcomes, from team
//! preview to the end of the battle, single process.
//!
//! Usage: sim_play TEAMS_JSON BATTLES SEED [TIME_CAP_S] [--stage2 | --stage3 | --stage4 | --stage5 | --real] [--log]
//!
//! Battles are stage-1 ("plain") battles as the rig builds them: two teams from TEAMS_JSON (the rig's
//! teams.json) with No Ability, no item and four distinct moves drawn from the plain-move table, three
//! picks each. With --stage2 or --stage3 the moves come from that stage's pool instead; --stage4 draws from the
//! stage-3 pool and keeps each set's own ability when it is implemented. --stage5 keeps the abilities, gives
//! each Pokémon a random legal item (its Mega stone half the time when it has one; no item twice on a team) and
//! four moves from the whole legal pool. --real plays the sets exactly as written. Each step every deciding side
//! takes a uniformly random legal choice. Prints one JSON line.

use cg_engine::sim::data::{abilities, moves, ItemId, MoveId, ITEMS, MOVES, PLAIN_MOVES, STAGE2_MOVES, STAGE3_MOVES};
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::{State, Winner};
use cg_engine::sim::{Kind, MonSpec};
use rig::json::Json;
use std::time::Instant;

const MAX_STEPS: usize = 1000;

fn pick_distinct(rng: &mut Seeded, n: usize, k: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    for i in 0..k {
        let j = i + rng.below((n - i) as u32) as usize;
        v.swap(i, j);
    }
    v.truncate(k);
    v
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let stage2 = args.iter().any(|a| a == "--stage2");
    let stage4 = args.iter().any(|a| a == "--stage4");
    let stage5 = args.iter().any(|a| a == "--stage5");
    let real = args.iter().any(|a| a == "--real");
    // record the battle log's rows (State::step_logged), a buffer reused across steps
    let log = args.iter().any(|a| a == "--log");
    let stage3 = stage4 || args.iter().any(|a| a == "--stage3");
    args.retain(|a| a != "--stage2" && a != "--stage3" && a != "--stage4" && a != "--stage5" && a != "--real" && a != "--log");
    let full: Vec<MoveId> = (1..MOVES.len() as u16).map(MoveId).filter(|m| *m != moves::STRUGGLE).collect();
    let held: Vec<ItemId> = (1..ITEMS.len() as u16).map(ItemId).filter(|i| i.data().mega_stone.is_empty()).collect();
    let pool: &[MoveId] = if stage5 {
        &full
    } else if stage3 {
        &STAGE3_MOVES
    } else if stage2 {
        &STAGE2_MOVES
    } else {
        &PLAIN_MOVES
    };
    if args.len() < 4 || args.len() > 5 {
        eprintln!("usage: sim_play TEAMS_JSON BATTLES SEED [TIME_CAP_S] [--stage2 | --stage3 | --stage4 | --stage5 | --real]");
        std::process::exit(2);
    }
    let battles: usize = args[2].parse().expect("BATTLES must be an integer");
    let seed: u64 = args[3].parse().expect("SEED must be an integer");
    let cap: Option<f64> = args.get(4).map(|s| s.parse().expect("TIME_CAP_S must be a number"));
    let text = std::fs::read_to_string(&args[1]).unwrap_or_else(|e| panic!("cannot read {}: {}", args[1], e));
    let teams_json = Json::parse(&text).expect("teams json");
    let mut teams: Vec<Vec<MonSpec>> = Vec::new();
    for t in teams_json.as_arr().expect("a list of teams") {
        let team: Result<Vec<MonSpec>, String> = t.as_arr().unwrap().iter().map(rig::newsim::mon_spec).collect();
        match team {
            Ok(v) if v.len() == 6 => teams.push(v),
            _ => {}
        }
    }
    assert!(teams.len() >= 2, "need teams");

    let mut rng = Seeded::new(seed);
    let (mut s1, mut s2, mut ties, mut unfinished, mut steps, mut done) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut rows: Vec<cg_engine::sim::Row> = Vec::with_capacity(256);
    let mut nrows = 0usize;
    let mut unimpl = 0usize;
    let t0 = Instant::now();
    let mut setup = 0f64;
    let mut capped = false;
    for _ in 0..battles {
        if let Some(c) = cap {
            if t0.elapsed().as_secs_f64() > c {
                capped = true;
                break;
            }
        }
        let ts = Instant::now();
        let ti = pick_distinct(&mut rng, teams.len(), 2);
        let mut sides: [Vec<MonSpec>; 2] = [teams[ti[0]].clone(), teams[ti[1]].clone()];
        for team in sides.iter_mut() {
            if real {
                break;
            }
            let mut used: Vec<ItemId> = Vec::new();
            for m in team.iter_mut() {
                if !((stage4 || stage5) && cg_engine::sim::abilities::ability_implemented(m.ability)) {
                    m.ability = abilities::NOABILITY;
                }
                if stage5 {
                    let stone = ITEMS.iter().position(|d| d.mega_stone.iter().any(|(from, _)| *from == m.species)).map(|i| ItemId(i as u16));
                    m.item = match stone {
                        Some(s) if rng.below(2) == 0 && !used.contains(&s) => s,
                        _ => loop {
                            let x = held[rng.below(held.len() as u32) as usize];
                            if !used.contains(&x) {
                                break x;
                            }
                        },
                    };
                    used.push(m.item);
                } else {
                    m.item = ItemId::NONE;
                }
                let mv = pick_distinct(&mut rng, pool.len(), 4);
                m.moves = [pool[mv[0]], pool[mv[1]], pool[mv[2]], pool[mv[3]]];
                m.nmoves = 4;
            }
        }
        let mut picks = [[0u8; 3]; 2];
        for p in picks.iter_mut() {
            let v = pick_distinct(&mut rng, 6, 3);
            *p = [v[0] as u8, v[1] as u8, v[2] as u8];
        }
        setup += ts.elapsed().as_secs_f64();
        let mut st = if log {
            rows.clear();
            let st = State::start_logged([&sides[0], &sides[1]], picks, &mut rng, &mut rows).expect("start");
            nrows += rows.len();
            st
        } else {
            State::start([&sides[0], &sides[1]], picks, &mut rng).expect("start")
        };
        let mut k = 0;
        while !st.ended && k < MAX_STEPS {
            let opts = st.options();
            let mut ch = [None, None];
            for side in 0..2 {
                if matches!(opts[side].kind, Kind::Turn | Kind::Replace) {
                    let cs = opts[side].choices();
                    ch[side] = Some(cs[rng.below(cs.len() as u32) as usize]);
                }
            }
            if log {
                rows.clear();
                st.step_logged(ch, &mut rng, &mut rows).expect("step");
                nrows += rows.len();
            } else {
                st.step(ch, &mut rng).expect("step");
            }
            k += 1;
        }
        steps += k;
        done += 1;
        match st.winner {
            Winner::Side(0) => s1 += 1,
            Winner::Side(_) => s2 += 1,
            Winner::Tie => ties += 1,
            Winner::None => unfinished += 1,
        }
        unimpl += (st.unimpl.count > 0) as usize;
    }
    let wall = t0.elapsed().as_secs_f64();
    println!(
        "{{\"engine\": \"sim\", \"pool\": \"{}\", \"battles\": {}, \"seed\": {}, \"wall_s\": {:.3}, \"team_setup_s\": {:.3}, \
         \"battles_per_s\": {:.1}, \"steps\": {}, \"steps_per_s\": {:.1}, \"steps_per_battle\": {:.2}, \
         \"s1_wins\": {}, \"s2_wins\": {}, \"ties\": {}, \"unfinished\": {}, \"unimplemented_battles\": {}, \"state_bytes\": {}, \"time_capped\": {}, \
         \"log\": {}, \"rows\": {}}}",
        if real {
            "real"
        } else if stage5 {
            "stage5"
        } else if stage4 {
            "stage4"
        } else if stage3 {
            "stage3"
        } else if stage2 {
            "stage2"
        } else {
            "plain"
        },
        done,
        seed,
        wall,
        setup,
        done as f64 / wall,
        steps,
        steps as f64 / wall,
        steps as f64 / done.max(1) as f64,
        s1,
        s2,
        ties,
        unfinished,
        unimpl,
        std::mem::size_of::<State>(),
        capped,
        log,
        nrows
    );
}
