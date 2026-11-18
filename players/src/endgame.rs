//! Endgame (`pcsa/players/endgame.py`, `config/players/endgame.yaml`): disciplined until both sides are
//! down to `max_members`, then the 1v1 chain (`chain`) takes the first step: stay and use the line's move,
//! switch to the line's first member, or send it after a faint.
//!
//! - E-1: in the endgame, each option we have (stay, a legal switch, a replacement) is scored against the
//!   worst set of their unseen members (preview species not yet seen, full HP).
//! - E-2: the best option (wins first, then HP margin; first on ties) is taken unless it is a draw that
//!   does not win (not a win and margin >= 0), or ties the next option within `TIE`; then disciplined.
//! - E-3: voluntary switches obey the back-to-back rule (they are left out of the options when blocked).
//!
//! Inputs from the Battle: HP, speed (negated under Trick Room), the shared estimate for known moves only
//! (our four, their revealed ones; an unseen member has none), recovery, known residuals, contact damage.

use crate::best_matchup::{blocked, switches};
use crate::chain::{chain_options, Attack, Attacks, ChainResult, Combatant, Matchups, MAX_MEMBERS};
use crate::disciplined::{self, heal_fraction, is_recovery, DisciplinedParams};
use crate::estimate::{priority, Estimator};
use crate::fired::Fired;
use crate::max_damage::{action_move, move_actions, preview_picks};
use crate::threat::{hp, known_moves};
use crate::view::{speed, Fighter};
use crate::{Options, Player, Rng};
use battle::{legal, Battle, Legal, ReqKind, Side, Who};
use cg_engine::sim::data::*;

/// `config/players/endgame.yaml`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndgameParams {
    /// members left on each side (theirs: 3 brought minus fainted) at which the chain takes over
    pub max_members: i32,
}

impl Default for EndgameParams {
    fn default() -> Self {
        EndgameParams { max_members: 2 }
    }
}

const TEAM_SIZE: i32 = 3;
const POISON: f32 = 1.0 / 8.0;
const BURN: f32 = 1.0 / 16.0;
const LEFTOVERS: f32 = 1.0 / 16.0;
const SAND: f32 = 1.0 / 16.0;
const HELMET: f32 = 1.0 / 6.0;
const BARBS: f32 = 1.0 / 8.0;
const TIE: f32 = 1e-6;

pub const RULES: &[&str] = &[
    "decisions",
    "forced",
    "denier:switch",
    "denier:ko",
    "denier:anti_setup",
    "denier:priority_ko",
    "denier:hardest_hit",
    "futile_switch",
    "trade_switch",
    "threatened_switch",
    "recover",
    "attack",
    "switch_blocked",
    "recover_blocked",
    "phase",
    "endgame:stay",
    "endgame:switch",
    "endgame:send",
    "endgame:fallback_draw",
    "endgame:fallback_tie",
    "endgame:fallback_no_line",
    "endgame:fallback_unseen",
];
const R_STAY: usize = 15;
const R_SWITCH: usize = 16;
const R_SEND: usize = 17;
const R_DRAW: usize = 18;
const R_TIE: usize = 19;
const R_NO_LINE: usize = 20;
const R_UNSEEN: usize = 21;

/// Known per-turn HP change: status, Leftovers / Black Sludge, sandstorm.
pub fn residual(b: &Battle, f: &Fighter) -> f32 {
    let (ab, it) = (f.ab(), f.it());
    let mut c = 0.0;
    match f.status {
        Status::Psn | Status::Tox => c += if ab == "poisonheal" { POISON } else { -POISON },
        Status::Brn => c -= BURN,
        _ => {}
    }
    if it == "leftovers" || (it == "blacksludge" && f.has_type(Type::Poison)) {
        c += LEFTOVERS;
    } else if it == "blacksludge" {
        c -= POISON;
    }
    let sand = b.field.weather.map_or(false, |w| w.cond == conds::SANDSTORM);
    let proof = f.has_type(Type::Rock) || f.has_type(Type::Ground) || f.has_type(Type::Steel);
    if sand && !(proof || matches!(ab, "overcoat" | "sandveil" | "sandrush" | "sandforce" | "magicguard") || it == "safetygoggles") {
        c -= SAND;
    }
    if ab == "magicguard" {
        c.max(0.0)
    } else {
        c
    }
}

/// What an attacker loses per hit on this member: Rocky Helmet, Rough Skin / Iron Barbs.
pub fn passive_on_hit(f: &Fighter) -> f32 {
    (if f.it() == "rockyhelmet" { HELMET } else { 0.0 }) + (if matches!(f.ab(), "roughskin" | "ironbarbs") { BARBS } else { 0.0 })
}

fn alive(b: &Battle, w: Who) -> bool {
    !b.mon(w).fainted && b.mon(w).hp > 0.0
}

/// Our Pokémon being replaced at a replacement asked before its rows: counted out, as a fainted one.
fn leaving(b: &Battle, l: &Legal) -> Option<u8> {
    if battle::legal::replace_unread(b, l) {
        battle::legal::on_field(b)
    } else {
        None
    }
}

/// Both sides are down to `max_members` or fewer (and have some left).
pub fn in_endgame(b: &Battle, e: &EndgameParams) -> bool {
    in_endgame_without(b, e, None)
}

fn in_endgame_without(b: &Battle, e: &EndgameParams, gone: Option<u8>) -> bool {
    let ours = b.picks.iter().flatten().filter(|&&s| Some(s) != gone && alive(b, Who { side: Side::Own, slot: s })).count() as i32;
    let theirs = TEAM_SIZE - b.side(Side::Opp).team.iter().filter(|m| m.fainted).count() as i32;
    0 < ours && ours <= e.max_members && 0 < theirs && theirs <= e.max_members
}

/// A side's members for the chain: alive ones, plus None for a fainted (or absent) active; the active's
/// index. `gone`: our Pokémon being replaced (counted as the fainted active).
fn side_members(b: &Battle, side: Side, team: &[Who], gone: Option<u8>) -> ([Option<Who>; MAX_MEMBERS], usize, usize) {
    let mut out = [None; MAX_MEMBERS];
    let mut n = 0;
    let act = if gone.is_some() { None } else { b.side(side).active.slot };
    let mut i = None;
    for &w in team {
        if alive(b, w) && Some(w.slot) != gone && n < MAX_MEMBERS {
            if act == Some(w.slot) {
                i = Some(n);
            }
            out[n] = Some(w);
            n += 1;
        }
    }
    let i = match i {
        Some(i) => i,
        None => {
            out[n] = None;
            n += 1;
            n - 1
        }
    };
    (out, n, i)
}

struct Inputs<'a> {
    b: &'a Battle,
    /// our active's legal moves when it has any (its other known moves are left out)
    legal_moves: Option<[Option<MoveId>; 4]>,
    pivot: Option<Who>,
}

impl<'a> Inputs<'a> {
    fn moves(&self, w: Who) -> [Option<MoveId>; 4] {
        let mut m = known_moves(self.b, w);
        if let (Some(lm), Side::Own) = (self.legal_moves, w.side) {
            if self.b.active(Side::Own) == Some(w) {
                for x in m.iter_mut() {
                    if !x.map_or(false, |mv| lm.contains(&Some(mv))) {
                        *x = None;
                    }
                }
            }
        }
        m
    }

    fn unit(&self, est: &mut Estimator, w: Option<Who>, fresh: bool) -> Combatant {
        let Some(w) = w else { return Combatant { hp: 0.0, speed: 0, can_switch_on_faint: false, ..Combatant::default() } };
        let b = self.b;
        let f = est.fighter(b, w, false);
        let field = est.field;
        let mut sp = speed(&f, &field) as i32;
        if field.trick_room {
            sp = -sp;
        }
        let recover = self.moves(w).iter().flatten().filter(|&&m| is_recovery(m)).map(|&m| heal_fraction(b, m)).fold(0.0, f32::max);
        Combatant {
            hp: if fresh { 1.0 } else { hp(b, w) },
            speed: sp,
            recover,
            residual: residual(b, &f),
            passive_on_hit: passive_on_hit(&f),
            can_switch_on_faint: self.pivot == Some(w),
        }
    }

    fn attacks(&self, est: &mut Estimator, atk: Option<Who>, def: Option<Who>) -> Attacks {
        let mut out = Attacks::default();
        let (Some(a), Some(d)) = (atk, def) else { return out };
        let user = est.fighter(self.b, a, false);
        for mv in self.moves(a).iter().flatten() {
            if mv.data().category == Category::Status {
                continue;
            }
            let h = est.hit(self.b, a, false, *mv, d);
            let hit = h.expected();
            if hit <= 0.0 || h.accuracy <= 0.0 {
                continue;
            }
            let field = est.field;
            out.push(Attack {
                id: mv.0,
                fraction: hit / h.accuracy,
                priority: priority(&user, *mv, &field),
                accuracy: h.accuracy,
                ko_fraction: h.max as f32 * h.hits / h.def_max_hp.max(1) as f32,
                recover: false,
            });
        }
        out
    }
}

/// Combinations of `k` of the first `n` indices, in lexicographic order.
pub fn combinations(n: usize, k: usize, mut f: impl FnMut(&[usize])) {
    if k > n {
        return;
    }
    let mut idx = [0usize; MAX_MEMBERS];
    for (x, v) in idx.iter_mut().enumerate().take(k) {
        *v = x;
    }
    loop {
        f(&idx[..k]);
        let mut i = k;
        while i > 0 && idx[i - 1] == i - 1 + n - k {
            i -= 1;
        }
        if i == 0 {
            return;
        }
        idx[i - 1] += 1;
        for x in i..k {
            idx[x] = idx[x - 1] + 1;
        }
    }
}

fn key(r: &ChainResult) -> (bool, f32) {
    (r.wins, r.margin)
}

fn lower(a: (bool, f32), b: (bool, f32)) -> bool {
    (!a.0 && b.0) || (a.0 == b.0 && a.1 < b.1)
}

/// E-1..E-3: the chain's first step, or None (not the endgame, or a fallback to disciplined).
pub fn pick(b: &Battle, l: &Legal, opts: &Options, p: &DisciplinedParams, e: &EndgameParams, est: &mut Estimator, fired: &mut Fired) -> Option<usize> {
    let gone = leaving(b, l);
    if !in_endgame_without(b, e, gone) {
        return None;
    }
    b.side(Side::Opp).active.slot?;
    let own_slot = b.side(Side::Own).active.slot?;
    let me = Who { side: Side::Own, slot: own_slot };
    let (cands, n_moves) = move_actions(l, opts);
    let moves = &cands[..n_moves];
    let mut legal_ids = [None; 4];
    for (k, &a) in moves.iter().enumerate() {
        legal_ids[k] = action_move(b, l, me, a);
    }
    let own_team: Vec<Who> = b.picks.iter().flatten().map(|&s| Who { side: Side::Own, slot: s }).collect();
    let (ours, no, i) = side_members(b, Side::Own, &own_team, gone);
    let pivot = if n_moves > 0 { None } else { ours[i] };
    let inputs = Inputs { b, legal_moves: if n_moves > 0 { Some(legal_ids) } else { None }, pivot };
    let seen: Vec<Who> = (0..6u8).map(|s| Who { side: Side::Opp, slot: s }).filter(|&w| b.mon(w).seen).collect();
    let (theirs, nt, j) = side_members(b, Side::Opp, &seen, None);
    let fainted = b.side(Side::Opp).team.iter().filter(|m| m.fainted).count() as i32;
    let unseen_n = (TEAM_SIZE - fainted - theirs[..nt].iter().filter(|x| x.is_some()).count() as i32).max(0) as usize;
    let cand_slots: Vec<Who> = (0..6u8)
        .map(|s| Who { side: Side::Opp, slot: s })
        .filter(|&w| {
            let m = b.mon(w);
            m.preview.is_some() && !m.seen
        })
        .collect();
    if unseen_n > cand_slots.len() || nt + unseen_n > MAX_MEMBERS {
        fired.hit(R_UNSEEN);
        return None;
    }
    // our units and attacks do not depend on the candidates
    let mut our_units = [Combatant::default(); MAX_MEMBERS];
    for k in 0..no {
        our_units[k] = inputs.unit(est, ours[k], false);
    }
    let mut worst: [Option<ChainResult>; MAX_MEMBERS] = [None; MAX_MEMBERS];
    let mut order: [usize; MAX_MEMBERS] = [0; MAX_MEMBERS];
    let mut n_order = 0;
    let mut m = Matchups::default();
    let mut their_units = [Combatant::default(); MAX_MEMBERS];
    for jj in 0..nt {
        their_units[jj] = inputs.unit(est, theirs[jj], false);
        for k in 0..no {
            m.ours[k][jj] = inputs.attacks(est, ours[k], theirs[jj]);
            m.theirs[k][jj] = inputs.attacks(est, theirs[jj], ours[k]);
        }
    }
    combinations(cand_slots.len(), unseen_n, |pick| {
        let nn = nt + pick.len();
        for (x, &ci) in pick.iter().enumerate() {
            let w = Some(cand_slots[ci]);
            their_units[nt + x] = inputs.unit(est, w, true);
            for k in 0..no {
                m.ours[k][nt + x] = inputs.attacks(est, ours[k], w);
                m.theirs[k][nt + x] = inputs.attacks(est, w, ours[k]);
            }
        }
        let (opts_k, n) = chain_options(&our_units[..no], &their_units[..nn], &m, i, j);
        for &(k, r) in &opts_k[..n] {
            match worst[k] {
                None => {
                    worst[k] = Some(r);
                    order[n_order] = k;
                    n_order += 1;
                }
                Some(w) if lower(key(&r), key(&w)) => worst[k] = Some(r),
                _ => {}
            }
        }
    });
    // options that map to a legal action
    let (sw, n_sw) = switches(b, l);
    let voluntary = n_moves > 0 && ours[i].is_some();
    let block = voluntary && blocked(b, p.max_streak).is_some();
    let mut picks: Vec<(usize, ChainResult, bool)> = Vec::with_capacity(MAX_MEMBERS);
    for &k in &order[..n_order] {
        let r = worst[k].unwrap();
        if k == i && ours[i].is_some() {
            let a = match r.first {
                Some(f) if f.recover => moves.iter().copied().find(|&a| action_move(b, l, me, a).map_or(false, is_recovery)),
                Some(f) => moves.iter().copied().find(|&a| action_move(b, l, me, a).map(|x| x.0) == Some(f.id)),
                None => None,
            };
            if let Some(a) = a {
                picks.push((a, r, true));
            }
        } else if block {
            continue;
        } else if let Some(w) = ours[k] {
            if let Some(&(a, _)) = sw[..n_sw].iter().find(|(_, x)| *x == w) {
                picks.push((a, r, false));
            }
        }
    }
    if picks.is_empty() {
        fired.hit(R_NO_LINE);
        return None;
    }
    // stable sort, best first
    picks.sort_by(|x, y| {
        let (a, c) = (key(&x.1), key(&y.1));
        if lower(c, a) {
            std::cmp::Ordering::Less
        } else if lower(a, c) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    let best = picks[0].1;
    if !best.wins && best.margin >= 0.0 {
        fired.hit(R_DRAW);
        return None;
    }
    if picks.len() > 1 && (best.margin - picks[1].1.margin).abs() < TIE && best.wins == picks[1].1.wins {
        fired.hit(R_TIE);
        return None;
    }
    let (a, _, stay) = picks[0];
    let forced = n_moves == 0 || ours[i].is_none();
    fired.hit(if stay {
        R_STAY
    } else if forced {
        R_SEND
    } else {
        R_SWITCH
    });
    Some(a)
}

pub struct EndgamePlayer {
    pub opts: Options,
    pub params: DisciplinedParams,
    pub endgame: EndgameParams,
    est: Estimator,
    pub fired: Fired,
}

impl Default for EndgamePlayer {
    fn default() -> Self {
        EndgamePlayer::new(Options::default(), DisciplinedParams::default(), EndgameParams::default())
    }
}

impl EndgamePlayer {
    pub fn new(opts: Options, params: DisciplinedParams, endgame: EndgameParams) -> Self {
        EndgamePlayer { opts, params, endgame, est: Estimator::default(), fired: Fired::new(RULES) }
    }
}

impl Player for EndgamePlayer {
    fn name(&self) -> &'static str {
        "endgame"
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        preview_picks(b)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, _rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        self.est.reset(b);
        disciplined::choose(b, &l, &self.opts, &self.params, Some(&self.endgame), &mut self.est, &mut self.fired)
    }
    fn fired(&self) -> Vec<(&'static str, u64)> {
        self.fired.list()
    }
}
