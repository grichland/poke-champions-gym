//! Prints the engine's battle-log rows of random games on the real teams, one row per line, for reading
//! the record and finding row patterns.
//!
//! Usage: row_dump [--games 20] [--seed 1] [--teams tools/rig/data/teams.json] [--grep TEXT]
//! With `--grep`, prints only steps holding a row that contains TEXT.

use emit::fmt::show;
use env::teams::{mix, TeamMode, TeamPool};
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::State;
use cg_engine::sim::Row;

fn main() {
    let v: Vec<String> = std::env::args().collect();
    let get = |k: &str| v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned();
    let games: u64 = get("--games").and_then(|x| x.parse().ok()).unwrap_or(20);
    let seed: u64 = get("--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let grep = get("--grep");
    let pool = TeamPool::open(&get("--teams").unwrap_or_else(|| "tools/rig/data/teams.json".into())).expect("teams");
    for g in 0..games {
        let mut rng = Seeded::new(mix(seed, g));
        let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
        let pick = |rng: &mut Seeded| {
            let mut p = [0u8; 3];
            let mut k = 0;
            while k < 3 {
                let x = rng.below(6) as u8;
                if !p[..k].contains(&x) {
                    p[k] = x;
                    k += 1;
                }
            }
            p
        };
        let picks = [pick(&mut rng), pick(&mut rng)];
        let mut rows: Vec<Row> = Vec::new();
        let mut st = match State::start_logged([&teams[0], &teams[1]], picks, &mut rng, &mut rows) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("game {}: {}", g, e);
                continue;
            }
        };
        let names = |st: &State| -> Vec<String> {
            (0..2).flat_map(|s| (0..3).map(move |i| (s, i))).map(|(s, i)| format!("{}:{}={}", s, i, st.sides[s].mons[i].set_species.id())).collect()
        };
        let print = |label: &str, rows: &[Row]| {
            let lines: Vec<String> = rows.iter().map(show).collect();
            if grep.as_ref().map_or(true, |t| lines.iter().any(|l| l.contains(t.as_str()))) {
                println!("== game {} {}", g, label);
                for l in lines {
                    println!("  {}", l);
                }
            }
        };
        println!("#### game {} {}", g, names(&st).join(" "));
        print("start", &rows);
        let mut steps = 0;
        while !st.ended && steps < 500 {
            let o = st.options();
            let mut ch = [None; 2];
            for side in 0..2 {
                let c = o[side].choices();
                if !c.is_empty() {
                    ch[side] = Some(c[rng.below(c.len() as u32) as usize]);
                }
            }
            rows.clear();
            if let Err(e) = st.step_logged(ch, &mut rng, &mut rows) {
                eprintln!("game {} step {}: {}", g, steps, e);
                break;
            }
            print(&format!("step {} turn {}", steps, st.turn), &rows);
            steps += 1;
        }
    }
}
