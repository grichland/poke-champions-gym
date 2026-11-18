//! Best-matchup max damage (`pcsa/players/best_matchup.py`): max damage, plus a voluntary switch out of a
//! really bad matchup.
//!
//! - BM-1: a move and a switch legal, both actives on the field, our active's matchup below
//!   `switch_below`, and the best legal bench member's (first on ties) above `switch_in_above`: switch to it,
//!   unless the back-to-back rule blocks it (BM-2).
//! - BM-2: a voluntary switch the turn after our own is allowed only if the opponent switched (or pivoted)
//!   last turn too, and never a third in a row (`max_streak`).
//! - Otherwise max damage (its preview, moves and forced switches).

use crate::estimate::Estimator;
use crate::fired::Fired;
use crate::matchup::{matchup, HeuristicParams};
use crate::max_damage::{self, preview_picks, single};
use crate::{Options, Player, Rng};
use battle::{legal, Act, Battle, Legal, ReqKind, Side, Who, N_ACTIONS};

/// `config/players/best_matchup.yaml` and `best_matchup.py`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchParams {
    /// switch out when the active's matchup is below this...
    pub switch_below: f32,
    /// ...and the best bench member's is above this
    pub switch_in_above: f32,
    /// voluntary switches in a row (MAX_STREAK)
    pub max_streak: u8,
}

impl Default for SwitchParams {
    fn default() -> Self {
        SwitchParams { switch_below: -2.0, switch_in_above: 1.0, max_streak: 2 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BestMatchupParams {
    pub heuristic: HeuristicParams,
    pub switch: SwitchParams,
}

/// BM-2: why a voluntary switch is not allowed now (None: allowed). Read from the Battle's last turn.
pub fn blocked(b: &Battle, max_streak: u8) -> Option<&'static str> {
    if b.side(Side::Own).last.act != Act::Switch {
        return None;
    }
    if b.track.sides[0].switch_run >= max_streak {
        return Some("already switched twice in a row");
    }
    if !matches!(b.side(Side::Opp).last.act, Act::Switch | Act::Pivot) {
        return Some("switched last turn and the opponent did not");
    }
    None
}

pub const RULES: &[&str] = &["decisions", "switch", "switch_blocked", "max_damage"];
const R_DECISIONS: usize = 0;
const R_SWITCH: usize = 1;
const R_BLOCKED: usize = 2;
const R_MAX_DAMAGE: usize = 3;

pub struct BestMatchupPlayer {
    pub opts: Options,
    pub params: BestMatchupParams,
    est: Estimator,
    pub fired: Fired,
}

impl Default for BestMatchupPlayer {
    fn default() -> Self {
        BestMatchupPlayer::new(Options::default(), BestMatchupParams::default())
    }
}

impl BestMatchupPlayer {
    pub fn new(opts: Options, params: BestMatchupParams) -> Self {
        BestMatchupPlayer { opts, params, est: Estimator::default(), fired: Fired::new(RULES) }
    }
}

/// The legal switches (action, our member).
pub fn switches(b: &Battle, l: &Legal) -> ([(usize, Who); 3], usize) {
    let mut out = [(0, Who { side: Side::Own, slot: 0 }); 3];
    let mut n = 0;
    for a in 8..N_ACTIONS {
        if let (true, Some(slot)) = (l.mask[a], b.picks[a - 8]) {
            out[n] = (a, Who { side: Side::Own, slot });
            n += 1;
        }
    }
    (out, n)
}

/// BM-1: the switch out of a bad matchup, if one applies. `Err` when BM-2 blocked it.
pub fn voluntary_switch(b: &Battle, l: &Legal, est: &mut Estimator, p: &BestMatchupParams) -> Result<Option<usize>, ()> {
    let (Some(me), Some(foe)) = (b.active(Side::Own), b.active(Side::Opp)) else { return Ok(None) };
    if !l.mask[..8].iter().any(|x| *x) {
        return Ok(None);
    }
    let (sw, n) = switches(b, l);
    if n == 0 {
        return Ok(None);
    }
    if matchup(b, est, me, foe, &p.heuristic) >= p.switch.switch_below {
        return Ok(None);
    }
    let mut best = (f32::NEG_INFINITY, 0usize);
    for &(a, w) in &sw[..n] {
        let s = matchup(b, est, w, foe, &p.heuristic);
        if s > best.0 {
            best = (s, a);
        }
    }
    if best.0 <= p.switch.switch_in_above {
        return Ok(None);
    }
    if blocked(b, p.switch.max_streak).is_some() {
        return Err(());
    }
    Ok(Some(best.1))
}

impl Player for BestMatchupPlayer {
    fn name(&self) -> &'static str {
        "best_matchup_max_damage"
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        preview_picks(b)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, _rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        self.est.reset(b);
        self.fired.hit(R_DECISIONS);
        if l.count() == 0 {
            return 0;
        }
        if let Some(a) = single(&l) {
            return a;
        }
        match voluntary_switch(b, &l, &mut self.est, &self.params) {
            Ok(Some(a)) => {
                self.fired.hit(R_SWITCH);
                return a;
            }
            Err(()) => self.fired.hit(R_BLOCKED),
            Ok(None) => {}
        }
        self.fired.hit(R_MAX_DAMAGE);
        max_damage::choose(b, &l, &self.opts, &mut self.est)
    }
    fn fired(&self) -> Vec<(&'static str, u64)> {
        self.fired.list()
    }
}
