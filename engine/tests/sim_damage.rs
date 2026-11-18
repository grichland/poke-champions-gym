//! The new resolver's damage for one hit against Showdown's getDamage (fixture from
//! tools/data/damage_fixture.js): stats from stat points, stages, crit, roll, STAB, type effectiveness,
//! immunity, burn, fixed damage.

use cg_engine::sim::battle::Sim;
use cg_engine::sim::data::*;
use cg_engine::sim::moves::{ActiveMove, Dmg};
use cg_engine::sim::options::{build_mon, MonSpec};
use cg_engine::sim::outcome::{EffectRef, Outcomes, Site};
use cg_engine::sim::state::State;
use std::convert::TryInto;

/// Answers the crit and the roll of the fixture row; any other draw is a test failure.
struct Fixed {
    crit: bool,
    roll: i32,
}

impl Outcomes for Fixed {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        assert_eq!(site.key.name(), "crit");
        self.crit
    }
    fn range(&mut self, site: &Site, lo: i32, hi: i32) -> i32 {
        assert_eq!((site.key.name(), lo, hi), ("roll", 0, 16));
        self.roll
    }
    fn sample(&mut self, _: &Site, _: &[i32]) -> i32 {
        panic!("unexpected sample")
    }
    fn tie(&mut self, _: &Site, _: u8, _: u8) -> u8 {
        panic!("unexpected tie")
    }
    fn tie_effects(&mut self, _: &Site, _: EffectRef, _: EffectRef) -> bool {
        panic!("unexpected tie")
    }
    fn sample_str(&mut self, _: &Site, _: &[&'static str]) -> usize {
        panic!("unexpected sample")
    }
}

fn nums<const N: usize>(s: &str) -> [i64; N] {
    let v: Vec<i64> = s.split(',').map(|x| x.parse().unwrap()).collect();
    v.try_into().unwrap()
}

fn mon(species: &str, nature: &str, evs: &str, boosts: &str) -> cg_engine::sim::state::Mon {
    let ev = nums::<6>(evs);
    let spec = MonSpec {
        species: SpeciesId::from_id(species).unwrap_or_else(|| panic!("species {}", species)),
        ability: abilities::NOABILITY,
        item: ItemId::NONE,
        moves: [MoveId::NONE; 4],
        nmoves: 0,
        nature: nature_from_id(nature).unwrap(),
        evs: ev.map(|x| x as u8),
        gender: 1,
    };
    let mut m = build_mon(&spec);
    let b = nums::<5>(boosts);
    for i in 0..5 {
        m.boosts[i] = b[i] as i8;
    }
    m.is_active = true;
    m
}

#[test]
fn damage_matches_showdown() {
    let text = include_str!("data/sim_damage.tsv");
    let mut n = 0;
    let mut bad = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let mut s = State::default();
        s.sides[0].mons[0] = mon(f[0], f[1], f[2], f[3]);
        if f[4] == "brn" {
            s.sides[0].mons[0].status = Status::Brn;
        }
        s.sides[1].mons[0] = mon(f[5], f[6], f[7], f[8]);
        for side in &mut s.sides {
            side.has_active = true;
            side.active = 0;
        }
        let mv = MoveId::from_id(f[9]).unwrap();
        let mut am = ActiveMove::new(mv);
        am.hit = 1;
        let mut o = Fixed { crit: f[10] == "1", roll: f[11].parse().unwrap() };
        let got = Sim::new(&mut s, &mut o).get_damage(0, 0, 1, 0, &mut am);
        let got = match got {
            Dmg::Fail => "fail".to_string(),
            Dmg::None => "none".to_string(),
            Dmg::Hp(x) => x.to_string(),
        };
        n += 1;
        if got != f[12] {
            bad.push(format!("{} -> engine {}", line, got));
        }
    }
    assert!(n >= 4000, "fixture has {} cases", n);
    assert!(bad.is_empty(), "{} of {} differ, first: {:#?}", bad.len(), n, &bad[..bad.len().min(10)]);
}
