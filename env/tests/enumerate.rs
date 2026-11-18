mod common;

use env::enumerate::fingerprint;
use env::{enumerate_start, tape_of, EnumOptions};
use cg_engine::sim::outcome::Seeded;
use std::collections::HashMap;

const CAP: usize = 200_000;

#[test]
fn probabilities_sum_to_one_merged_and_not() {
    let pool = common::pool();
    let (mut n, mut capped, mut multi) = (0, 0, 0);
    for seed in 0..400u64 {
        let Some((g, ch)) = common::sample_state(&pool, seed, 30) else { continue };
        let raw = g.outcomes(ch, &EnumOptions { merge: false, max_branches: CAP, keep_events: false }).unwrap();
        if raw.capped {
            capped += 1;
            assert!(raw.covered < 1.0 + 1e-9);
            continue;
        }
        let merged = g.outcomes(ch, &EnumOptions { merge: true, max_branches: CAP, keep_events: false }).unwrap();
        let s1: f64 = raw.outcomes.iter().map(|o| o.p).sum();
        let s2: f64 = merged.outcomes.iter().map(|o| o.p).sum();
        assert!((s1 - 1.0).abs() < 1e-9, "seed {}: raw sum {}", seed, s1);
        assert!((s2 - 1.0).abs() < 1e-9, "seed {}: merged sum {}", seed, s2);
        assert!((raw.covered - 1.0).abs() < 1e-9);
        assert_eq!(raw.leaves, raw.outcomes.len());
        assert_eq!(merged.leaves, raw.leaves);
        assert_eq!(merged.outcomes.iter().map(|o| o.leaves as usize).sum::<usize>(), raw.leaves);
        assert!(raw.outcomes.iter().all(|o| o.p > 0.0));
        // merged outcomes are pairwise distinct
        for i in 0..merged.outcomes.len().min(300) {
            for j in i + 1..merged.outcomes.len().min(300) {
                assert!(merged.outcomes[i].state != merged.outcomes[j].state);
            }
        }
        multi += (merged.outcomes.len() > 1) as usize;
        n += 1;
    }
    eprintln!("states {} uncapped, {} capped, {} with more than one outcome", n, capped, multi);
    assert!(n >= 300 && multi >= 150);
}

#[test]
fn cap_is_reported() {
    let pool = common::pool();
    let mut seen = 0;
    for seed in 0..200u64 {
        let Some((g, ch)) = common::sample_state(&pool, seed, 30) else { continue };
        let e = g.outcomes(ch, &EnumOptions { merge: false, max_branches: 3, keep_events: false }).unwrap();
        let full = g.outcomes(ch, &EnumOptions { merge: false, max_branches: CAP, keep_events: false }).unwrap();
        if full.leaves > 3 {
            assert!(e.capped && e.leaves == 3 && e.covered < 1.0);
            seen += 1;
        } else {
            assert!(!e.capped && e.leaves == full.leaves);
        }
    }
    assert!(seen > 50);
}

/// chi-square critical value at upper tail alpha = 1e-4 (Wilson-Hilferty)
fn chi2_crit(df: usize) -> f64 {
    let k = df as f64;
    let z = 3.719;
    k * (1.0 - 2.0 / (9.0 * k) + z * (2.0 / (9.0 * k)).sqrt()).powi(3)
}

#[test]
fn seeded_sampling_matches_enumeration() {
    const SAMPLES: usize = 200_000;
    let pool = common::pool();
    let mut tested = 0;
    let mut sizes = Vec::new();
    for seed in 1000..3000u64 {
        if tested >= 10 {
            break;
        }
        let Some((g, ch)) = common::sample_state(&pool, seed, 30) else { continue };
        let e = g.outcomes(ch, &EnumOptions { merge: true, max_branches: CAP, keep_events: false }).unwrap();
        // states with a spread worth testing
        if e.capped || e.outcomes.len() < 3 || e.outcomes.len() > 4000 {
            continue;
        }
        let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
        for (i, o) in e.outcomes.iter().enumerate() {
            index.entry(fingerprint(&o.state)).or_default().push(i);
        }
        let mut counts = vec![0usize; e.outcomes.len()];
        let mut rng = Seeded::new(seed * 7919);
        for _ in 0..SAMPLES {
            let mut s = g.state;
            s.step(ch, &mut rng).unwrap();
            let i = index
                .get(&fingerprint(&s))
                .and_then(|v| v.iter().copied().find(|&i| e.outcomes[i].state == s))
                .unwrap_or_else(|| panic!("seed {}: a sampled outcome is not in the enumeration", seed));
            counts[i] += 1;
        }
        // bins with expected count >= 5; the rest pooled
        let (mut stat, mut bins, mut pool_e, mut pool_o) = (0.0, 0usize, 0.0, 0usize);
        for (i, o) in e.outcomes.iter().enumerate() {
            let ex = o.p * SAMPLES as f64;
            if ex >= 5.0 {
                stat += (counts[i] as f64 - ex).powi(2) / ex;
                bins += 1;
            } else {
                pool_e += ex;
                pool_o += counts[i];
            }
        }
        if pool_e > 0.0 {
            if pool_e >= 5.0 {
                stat += (pool_o as f64 - pool_e).powi(2) / pool_e;
                bins += 1;
            } else {
                assert!((pool_o as f64) < pool_e + 6.0 * pool_e.sqrt() + 10.0, "seed {}: rare outcomes {} vs {}", seed, pool_o, pool_e);
            }
        }
        if bins < 2 {
            continue;
        }
        let crit = chi2_crit(bins - 1);
        eprintln!("seed {}: {} outcomes, {} bins, chi2 {:.1}, critical {:.1}", seed, e.outcomes.len(), bins, stat, crit);
        assert!(stat < crit, "seed {}: chi2 {} >= {} over {} bins", seed, stat, crit, bins);
        sizes.push(e.outcomes.len());
        tested += 1;
    }
    assert_eq!(tested, 10, "sizes {:?}", sizes);
}

#[test]
fn forcing_an_outcome_reproduces_it() {
    let pool = common::pool();
    let mut checked = 0;
    for seed in 0..120u64 {
        let Some((g, ch)) = common::sample_state(&pool, seed, 30) else { continue };
        let e = g.outcomes(ch, &EnumOptions { merge: false, max_branches: 20_000, keep_events: true }).unwrap();
        for o in e.outcomes.iter().step_by(1 + e.outcomes.len() / 40) {
            let mut f = g.fork(12345);
            f.step_forced(ch, &o.path).unwrap();
            assert!(f.state == o.state, "seed {}: forced path differs", seed);
            let mut t = g.fork(999);
            t.step_tape(ch, tape_of(&o.events)).unwrap();
            assert!(t.state == o.state, "seed {}: forced tape differs", seed);
            let pe: f64 = o.events.iter().map(|x| x.p).product();
            assert!((pe - o.p).abs() < 1e-12);
            checked += 1;
        }
    }
    assert!(checked > 500, "{}", checked);
}

#[test]
fn start_outcomes_sum_to_one() {
    let pool = common::pool();
    let t = &pool.teams;
    let e = enumerate_start([&t[0], &t[1]], [[0, 1, 2], [3, 4, 5]], &EnumOptions::default()).unwrap();
    let s: f64 = e.outcomes.iter().map(|o| o.p).sum();
    assert!(!e.capped && (s - 1.0).abs() < 1e-9);
}
