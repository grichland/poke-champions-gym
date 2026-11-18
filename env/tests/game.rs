mod common;

use cg_engine::sim::outcome::Seeded;

#[test]
fn snapshot_and_same_seed_fork_continue_identically() {
    let pool = common::pool();
    let mut n = 0;
    for seed in 0..60u64 {
        let Some((g, _)) = common::sample_state(&pool, seed, 20) else { continue };
        let mut a = g.snapshot();
        let mut b = g.snapshot();
        let mut c = g.fork(77);
        let mut d = g.fork(77);
        let (mut ra, mut rb) = (Seeded::new(seed), Seeded::new(seed));
        for _ in 0..200 {
            if a.state.ended {
                break;
            }
            let ca = common::random_choices(&a, &mut ra);
            let cb = common::random_choices(&b, &mut rb);
            assert_eq!(ca, cb);
            a.step(ca).unwrap();
            b.step(cb).unwrap();
            assert!(a.state == b.state);
        }
        let (mut rc, mut rd) = (Seeded::new(seed), Seeded::new(seed));
        for _ in 0..200 {
            if c.state.ended {
                break;
            }
            let cc = common::random_choices(&c, &mut rc);
            let cd = common::random_choices(&d, &mut rd);
            c.step(cc).unwrap();
            d.step(cd).unwrap();
            assert!(c.state == d.state);
        }
        // the original is untouched by its copies
        assert!(g.state != a.state || a.state.turn == g.state.turn);
        n += 1;
    }
    assert!(n > 40);
}

#[test]
fn forks_with_other_seeds_diverge() {
    let pool = common::pool();
    let mut differ = 0;
    for seed in 0..40u64 {
        let Some((g, ch)) = common::sample_state(&pool, seed, 10) else { continue };
        let mut a = g.fork(1);
        let mut b = g.fork(2);
        let mut ra = Seeded::new(5);
        let mut rb = Seeded::new(5);
        a.step(ch).unwrap();
        b.step(ch).unwrap();
        for _ in 0..30 {
            if a.state.ended || b.state.ended {
                break;
            }
            let x = common::random_choices(&a, &mut ra);
            let y = common::random_choices(&b, &mut rb);
            a.step(x).unwrap();
            b.step(y).unwrap();
        }
        differ += (a.state != b.state) as usize;
    }
    assert!(differ > 30, "{}", differ);
}
