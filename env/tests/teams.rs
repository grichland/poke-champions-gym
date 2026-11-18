//! The team pool: reasons for teams not accepted, the unweighted draw's random stream, and weighted draws.

mod common;

use env::teams::{TeamMode, TeamPool};
use cg_engine::sim::outcome::Seeded;

fn shipped_text() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../python/champions_gym/data/teams.json")).unwrap()
}

#[test]
fn shipped_pool_is_all_accepted() {
    let p = TeamPool::from_json_strict(&shipped_text()).unwrap();
    assert_eq!((p.teams.len(), p.file_count, p.errors.len()), (430, 430, 0));
    assert_eq!(p.file_index, (0..430).collect::<Vec<_>>());
}

#[test]
fn dropped_teams_keep_index_and_reason() {
    // the file has one team per line
    let text = shipped_text();
    let teams: Vec<&str> = text.lines().filter(|l| l.starts_with("[{")).map(|l| l.trim_end_matches(',')).collect();
    assert_eq!(teams.len(), 430);
    let t = |i: usize| teams[i].to_string();
    let mut five = t(1);
    // drop the last set: "[a,b,c,d,e,f]" -> "[a,b,c,d,e]"
    let cut = five.rfind(",{").unwrap();
    five.truncate(cut);
    five.push(']');
    let bad_move = t(2).replacen("\"moves\":[\"", "\"moves\":[\"Not A Move\",\"", 1);
    let file = format!("[{},{},{},{},{}]", t(0), five, bad_move, "{\"x\":1}", t(3));
    let p = TeamPool::from_json(&file).unwrap();
    assert_eq!(p.teams.len(), 2);
    assert_eq!(p.file_index, vec![0, 4]);
    assert_eq!(p.file_count, 5);
    let idx: Vec<usize> = p.errors.iter().map(|e| e.0).collect();
    assert_eq!(idx, vec![1, 2, 3]);
    assert!(p.errors[0].1.contains("5 sets, need 6"), "{}", p.errors[0].1);
    assert!(p.errors[1].1.starts_with("set 0 (") && p.errors[1].1.contains("unknown move \"notamove\""), "{}", p.errors[1].1);
    assert!(p.errors[2].1.contains("not a list of sets"), "{}", p.errors[2].1);
    assert!(TeamPool::from_json_strict(&file).unwrap_err().contains("3 of 5 teams not accepted"));
    assert!(p.dropped_message().starts_with("3 of 5 teams not accepted: team 1: 5 sets"));
}

/// The draw as it was before weights: it must stay the same stream.
fn old_draw(n: u32, rng: &mut Seeded) -> [usize; 2] {
    let a = rng.below(n) as usize;
    let mut b = rng.below(n - 1) as usize;
    if b >= a {
        b += 1;
    }
    [a, b]
}

#[test]
fn unweighted_draw_stream_is_unchanged() {
    let p = common::pool();
    for seed in 0..20_000u64 {
        let (mut r1, mut r2) = (Seeded::new(seed), Seeded::new(seed));
        let (ids, teams) = p.draw_for(&mut r1, TeamMode::AsGiven, (seed % 2) as usize);
        assert_eq!(ids, old_draw(430, &mut r2));
        assert_eq!(teams, [p.teams[ids[0]], p.teams[ids[1]]]);
        // the source is left where the old draw left it
        assert_eq!(r1.next_u64(), r2.next_u64());
    }
}

/// Upper tail of chi-square with k degrees of freedom (Wilson-Hilferty), as a z score.
fn chi2_z(x: f64, k: f64) -> f64 {
    ((x / k).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * k))) / (2.0 / (9.0 * k)).sqrt()
}

#[test]
fn weighted_draws_match_the_weights() {
    let mut p = common::pool();
    let n = p.file_count;
    // agent seat: a 60% team, a few heavier ones, some never drawn; other seat: uniform
    let mut w = vec![1.0; n];
    for (i, x) in w.iter_mut().enumerate() {
        if i % 7 == 0 {
            *x = 0.0;
        } else if i % 11 == 0 {
            *x = 3.5;
        }
    }
    let rest: f64 = w.iter().enumerate().filter(|(i, _)| *i != 1).map(|(_, x)| x).sum();
    w[1] = rest * 1.5; // team 1 has 60% of the weight
    let other = vec![1.0; n];
    p.set_weights(&w, &other, false).unwrap();
    let draws = 200_000usize;
    let mut first = vec![0u64; n];
    let mut second = vec![0u64; n];
    let mut rng = Seeded::new(42);
    for d in 0..draws {
        let seat = d % 2;
        let (ids, _) = p.draw_for(&mut rng, TeamMode::AsGiven, seat);
        assert_ne!(ids[0], ids[1]);
        first[ids[seat]] += 1;
        second[ids[1 - seat]] += 1;
    }
    let total: f64 = w.iter().sum();
    let (mut chi, mut k) = (0.0, -1.0);
    for i in 0..n {
        let e = draws as f64 * w[i] / total;
        if e == 0.0 {
            assert_eq!(first[i], 0, "team {} has weight 0", i);
            continue;
        }
        chi += (first[i] as f64 - e).powi(2) / e;
        k += 1.0;
    }
    let z = chi2_z(chi, k);
    assert!(z.abs() < 4.0, "agent seat: chi2 {} on {} df, z {}", chi, k, z);
    let share = first[1] as f64 / draws as f64;
    assert!((share - 0.6).abs() < 0.005, "share {}", share);
    // the other seat: uniform over the teams that are not the agent's
    let (mut chi, k) = (0.0, n as f64 - 1.0);
    for i in 0..n {
        let e = draws as f64 * (1.0 - w[i] / total) / (n as f64 - 1.0);
        chi += (second[i] as f64 - e).powi(2) / e;
    }
    let z = chi2_z(chi, k);
    assert!(z.abs() < 4.0, "other seat: chi2 {} on {} df, z {}", chi, k, z);
}

#[test]
fn weights_are_validated() {
    let mut p = common::pool();
    let n = p.file_count;
    let one = |i: usize| {
        let mut v = vec![0.0; n];
        v[i] = 1.0;
        v
    };
    let e = |p: &mut TeamPool, a: &[f64], b: &[f64], same: bool| p.set_weights(a, b, same).unwrap_err();
    assert!(e(&mut p, &[1.0; 3], &[1.0; 3], true).contains("3 weights for 430 teams"));
    let mut neg = vec![1.0; n];
    neg[9] = -0.5;
    assert!(e(&mut p, &neg, &neg, true).contains("team 9 is negative"));
    let mut nan = vec![1.0; n];
    nan[4] = f64::NAN;
    assert!(e(&mut p, &nan, &nan, true).contains("team 4 is NaN"));
    assert!(e(&mut p, &vec![0.0; n], &vec![0.0; n], true).contains("no accepted team"));
    assert!(e(&mut p, &one(8), &one(8), true).contains("only team 8"));
    assert!(e(&mut p, &vec![1.0; n], &one(8), false).contains("second seat can draw only team 8"));
    assert!(!p.weighted());
    // a single team for the agent against any other team works
    let mut rest = vec![1.0; n];
    rest[8] = 0.0;
    p.set_weights(&one(8), &rest, false).unwrap();
    let mut rng = Seeded::new(1);
    for s in 0..1000 {
        let (ids, _) = p.draw_for(&mut rng, TeamMode::AsGiven, s % 2);
        assert_eq!(ids[s % 2], 8);
        assert_ne!(ids[1 - s % 2], 8);
    }
}
