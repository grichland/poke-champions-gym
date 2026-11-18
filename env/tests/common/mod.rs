#![allow(dead_code)]

use env::teams::{distinct, TeamMode, TeamPool};
use env::Game;
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::SIDES;
use cg_engine::sim::{Choice, Kind};

pub fn pool() -> TeamPool {
    TeamPool::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/../python/champions_gym/data/teams.json")).expect("teams")
}

pub fn random_choices(g: &Game, rng: &mut Seeded) -> [Option<Choice>; SIDES] {
    let opts = g.state.options();
    let mut ch = [None; SIDES];
    for side in 0..SIDES {
        if matches!(opts[side].kind, Kind::Turn | Kind::Replace) {
            let cs = opts[side].choices();
            ch[side] = Some(cs[rng.below(cs.len() as u32) as usize]);
        }
    }
    ch
}

/// A stage-3 game a random number of random steps in (0..max_steps), and choices for its next step.
pub fn sample_state(pool: &TeamPool, seed: u64, max_steps: u32) -> Option<(Game, [Option<Choice>; SIDES])> {
    let mut rng = Seeded::new(seed);
    let (_, teams) = pool.draw(&mut rng, TeamMode::Stage3);
    let p0 = distinct(&mut rng, 6, 3);
    let p1 = distinct(&mut rng, 6, 3);
    let picks = [[p0[0] as u8, p0[1] as u8, p0[2] as u8], [p1[0] as u8, p1[1] as u8, p1[2] as u8]];
    let mut g = Game::start([&teams[0], &teams[1]], picks, seed ^ 0xABCD).ok()?;
    let k = rng.below(max_steps);
    for _ in 0..k {
        if g.state.ended {
            break;
        }
        let ch = random_choices(&g, &mut rng);
        g.step(ch).ok()?;
    }
    if g.state.ended {
        return None;
    }
    let ch = random_choices(&g, &mut rng);
    Some((g, ch))
}
