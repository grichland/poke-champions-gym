//! Over many random Battles built through the builder, every Player returns a legal action and valid
//! picks, and never panics.

use battle::legal;
use players::gen::random_battle;
use players::{by_name, Rng, NAMES};

#[test]
fn every_player_is_always_legal() {
    let g = Rng::new(2026);
    let mut checked = 0usize;
    let mut empty = 0usize;
    for name in NAMES {
        let mut p = by_name(name).unwrap();
        let mut r = Rng::new(11);
        let mut g2 = g;
        for _ in 0..4000 {
            let (b, k) = random_battle(&mut g2, false);
            let l = legal(&b, k);
            let a = p.choose_move(&b, k, &mut r);
            if l.count() == 0 {
                empty += 1;
                continue;
            }
            assert!(l.mask[a], "{} chose {} outside {:?} ({:?})", name, a, l.mask, k);
            checked += 1;
        }
        for _ in 0..500 {
            let (b, _) = random_battle(&mut g2, true);
            let picks = p.teampreview(&b, &mut r);
            assert!(picks.iter().all(|&x| (x as usize) < b.n_own as usize), "{:?}", picks);
            assert!(picks[0] != picks[1] && picks[1] != picks[2] && picks[0] != picks[2], "{} {:?}", name, picks);
        }
    }
    assert!(checked > 37000, "{} checked, {} empty", checked, empty);
}
