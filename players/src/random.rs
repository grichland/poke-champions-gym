//! Uniform over the legal actions; three distinct random picks at preview.

use crate::{Player, Rng};
use battle::{legal, Battle, ReqKind, N_ACTIONS};

#[derive(Default, Clone, Copy, Debug)]
pub struct RandomPlayer;

/// Three distinct random team indices (fewer team members: repeats the last valid one).
pub fn random_picks(n: usize, rng: &mut Rng) -> [u8; 3] {
    let mut idx: [u8; 6] = [0, 1, 2, 3, 4, 5];
    let n = n.clamp(1, 6);
    for i in 0..3.min(n) {
        let j = i + rng.below(n - i);
        idx.swap(i, j);
    }
    let mut out = [idx[0], idx[1.min(n - 1)], idx[2.min(n - 1)]];
    if n < 3 {
        out = [0, 1.min(n as u8 - 1), 2.min(n as u8 - 1)];
    }
    out
}

impl Player for RandomPlayer {
    fn name(&self) -> &'static str {
        "random"
    }
    fn teampreview(&mut self, b: &Battle, rng: &mut Rng) -> [u8; 3] {
        random_picks(b.n_own as usize, rng)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        let mut opts = [0usize; N_ACTIONS];
        let mut n = 0;
        for i in 0..N_ACTIONS {
            if l.mask[i] {
                opts[n] = i;
                n += 1;
            }
        }
        if n == 0 {
            return 0;
        }
        opts[rng.below(n)]
    }
}
