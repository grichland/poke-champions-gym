//! Strategy pilots (`research/pcsa-rl/rl2/opponents/pilots.py`): encore, weather, hazard, priority. Each is
//! tower plus one signature rule, taken with probability `rule_p` when it applies (checked first, whenever
//! both sides have a Pokémon on the field, the fainted or leaving one included). Preview: tower's picks
//! with the first two team members carrying the gimmick brought first (the first one leads).
//!
//! - encore: the foe is under Encore: a uniform random setup move that is not maxed. Else, holding Encore
//!   and the foe's last action of last turn was a status move or a recovery move: Encore.
//! - weather: our team's weathers (abilities and moves of all six). Our weather up: the boosted move with the
//!   highest estimate, if it does damage. Not up: a setter move, else a switch to a setter ability, both
//!   only when the foe cannot KO us.
//! - hazard: the foe cannot KO us: the first hazard move under its layer cap. Else a phaze when the foe is
//!   boosted or its side has 2+ hazard layers; else a pivot when it has a layer and can KO us.
//! - priority: on the first turn out, Fake Out if it can hit. Else the priority attack with the highest
//!   estimate (Sucker Punch only after a damaging move by the foe, first-turn moves only on the first turn):
//!   if it KOs, or if we are slower and the foe can KO us.

use crate::estimate::{priority, Estimator};
use crate::fired::Fired;
use crate::lists::{self, class};
use crate::max_damage::{action_move, move_actions, preview_picks};
use crate::threat::{can_ko, faster, type_mult};
use crate::tower;
use crate::view::{has_vol, Fighter, FieldView};
use crate::{Options, Player, Rng};
use battle::{legal, Act, Battle, Legal, ReqKind, Side, Who, N_ACTIONS, SIDE_CONDS};
use cg_engine::sim::data::*;

/// `pilots.py` constants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PilotParams {
    /// chance the signature rule is taken when it applies (RULE_P)
    pub rule_p: f64,
    /// gimmick members brought first at preview
    pub core: usize,
}

impl Default for PilotParams {
    fn default() -> Self {
        PilotParams { rule_p: 0.85, core: 2 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Encore,
    Weather,
    Hazard,
    Priority,
}

/// The pilots' recovery list (Rest in, Wish out), for the Encore rule.
const RECOVERY: [&str; 10] = ["recover", "roost", "slackoff", "softboiled", "milkdrink", "shoreup", "synthesis", "moonlight", "morningsun", "rest"];
const WEATHER_MOVES: [(&str, usize); 5] = [("sunnyday", 0), ("raindance", 1), ("sandstorm", 2), ("snowscape", 3), ("chillyreception", 3)];
const WEATHER_ABILITIES: [(&str, usize); 4] = [("drought", 0), ("drizzle", 1), ("sandstream", 2), ("snowwarning", 3)];
const WEATHER_OF: [(&str, usize); 7] =
    [("sunnyday", 0), ("desolateland", 0), ("raindance", 1), ("primordialsea", 1), ("sandstorm", 2), ("snowscape", 3), ("hail", 3)];
const BOOSTED: [&[&str]; 4] = [
    &["weatherball", "solarbeam", "solarblade", "flamethrower", "fireblast", "heatwave", "flareblitz", "overheat", "eruption", "mysticalfire", "fierydance"],
    &["weatherball", "thunder", "hurricane", "surf", "hydropump", "waterfall", "liquidation", "wavecrash", "scald", "muddywater", "aquajet"],
    &["weatherball", "stoneedge", "rockslide", "earthquake", "highhorsepower", "earthpower"],
    &["weatherball", "blizzard", "icebeam", "iciclecrash", "freezedry", "auroraveil"],
];
const HAZARDS: [(&str, &str, u8); 4] = [("stealthrock", "stealthrock", 1), ("spikes", "spikes", 3), ("toxicspikes", "toxicspikes", 2), ("stickyweb", "stickyweb", 1)];
const PIVOTS: [&str; 7] = ["uturn", "voltswitch", "flipturn", "partingshot", "teleport", "chillyreception", "shedtail"];
const PHAZES: [&str; 4] = ["roar", "whirlwind", "dragontail", "circlethrow"];
const FIRST_TURN: [&str; 2] = ["fakeout", "firstimpression"];

const ENCORE_RULES: &[&str] = &["decisions", "applies", "setup during encore", "encore"];
const WEATHER_RULES: &[&str] = &["decisions", "applies", "sun attack", "rain attack", "sand attack", "snow attack", "set weather", "switch to weather setter"];
const HAZARD_RULES: &[&str] = &["decisions", "applies", "set stealthrock", "set spikes", "set toxicspikes", "set stickyweb", "phaze", "pivot"];
const PRIORITY_RULES: &[&str] = &["decisions", "applies", "fake out", "priority ko", "priority when outsped"];
const R_DECISIONS: usize = 0;
const R_APPLIES: usize = 1;

pub struct PilotPlayer {
    pub kind: Kind,
    pub opts: Options,
    pub params: PilotParams,
    est: Estimator,
    pub fired: Fired,
}

impl PilotPlayer {
    pub fn new(kind: Kind, opts: Options, params: PilotParams) -> Self {
        let names = match kind {
            Kind::Encore => ENCORE_RULES,
            Kind::Weather => WEATHER_RULES,
            Kind::Hazard => HAZARD_RULES,
            Kind::Priority => PRIORITY_RULES,
        };
        PilotPlayer { kind, opts, params, est: Estimator::default(), fired: Fired::new(names) }
    }
    pub fn of(kind: Kind) -> Self {
        PilotPlayer::new(kind, Options::default(), PilotParams::default())
    }
}

fn idx(list: &[(&str, usize)], id: &str) -> Option<usize> {
    list.iter().find(|(x, _)| *x == id).map(|x| x.1)
}

fn side_cond(b: &Battle, side: Side, id: &str) -> Option<u8> {
    let i = SIDE_CONDS.iter().position(|s| *s == id)?;
    b.side(side).conds[i].map(|c| c.layers)
}

/// The foe's last action of last turn: Some(Some((move, is status))) for a move, Some(None) for a switch,
/// None for nothing seen.
pub fn foe_last(b: &Battle) -> Option<Option<(MoveId, bool)>> {
    let r = b.side(Side::Opp).last;
    // a faint was followed by a replacement, the latest thing the foe did
    if b.track.sides[1].faints_last > 0 || r.dragged || matches!(r.act, Act::Switch | Act::Pivot) {
        return Some(None);
    }
    match (r.act, r.mv) {
        (Act::Move, Some(mv)) => Some(Some((mv, mv.data().category == Category::Status))),
        _ => None,
    }
}

/// Gimmick test for one own team member at preview.
fn gimmick(kind: Kind, b: &Battle, i: usize) -> bool {
    let set = &b.sets[i];
    let mvs = set.moves.iter().flatten();
    match kind {
        Kind::Encore => set.moves.iter().flatten().any(|m| m.id() == "encore"),
        Kind::Weather => idx(&WEATHER_ABILITIES, set.ability.id()).is_some() || set.moves.iter().flatten().any(|m| idx(&WEATHER_MOVES, m.id()).is_some()),
        Kind::Hazard => set.moves.iter().flatten().any(|m| HAZARDS.iter().any(|h| h.0 == m.id())),
        Kind::Priority => {
            let f = Fighter::own(b, i, false);
            let field = FieldView::default();
            let mut any = false;
            for m in mvs {
                any |= m.data().category != Category::Status && priority(&f, *m, &field) > 0;
            }
            any
        }
    }
}

/// Tower's preview with the gimmick members swapped in.
pub fn preview(kind: Kind, b: &Battle, core: usize) -> [u8; 3] {
    let picks = preview_picks(b);
    let n = (b.n_own as usize).clamp(1, 6);
    let gm: Vec<u8> = (0..n).filter(|&i| gimmick(kind, b, i)).map(|i| i as u8).take(core).collect();
    if gm.is_empty() {
        return picks;
    }
    let mut out: Vec<u8> = gm.clone();
    out.extend(picks.iter().copied().filter(|p| !gm.contains(p)));
    [out[0], out[1], out[2]]
}

struct Ctx<'a> {
    b: &'a Battle,
    l: &'a Legal,
    me: Who,
    foe: Who,
    moves: [usize; 4],
    n: usize,
}

impl<'a> Ctx<'a> {
    fn moves(&self) -> &[usize] {
        &self.moves[..self.n]
    }
    fn mv(&self, a: usize) -> Option<MoveId> {
        action_move(self.b, self.l, self.me, a)
    }
    fn by_ids(&self, ids: &'static [&'static str]) -> impl Iterator<Item = usize> + '_ {
        self.moves().iter().copied().filter(move |&a| self.mv(a).map_or(false, |m| ids.contains(&m.id())))
    }
    /// Tower's estimate of a candidate action (its Mega variant scored as the Mega form).
    fn damage(&self, est: &mut Estimator, a: usize) -> f32 {
        match self.mv(a) {
            Some(mv) => est.hit(self.b, self.me, a >= 4, mv, self.foe).expected(),
            None => 0.0,
        }
    }
    /// The candidate with the highest estimate, first on ties.
    fn best(&self, est: &mut Estimator, xs: &[usize]) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for &a in xs {
            let d = self.damage(est, a);
            if best.map_or(true, |(_, x)| d > x) {
                best = Some((a, d));
            }
        }
        best
    }
    fn ko_threat(&self, est: &mut Estimator) -> bool {
        can_ko(self.b, est, self.foe, self.me)
    }
}

fn encore_rule(c: &Ctx, est: &mut Estimator, rng: &mut Rng) -> Option<(usize, usize)> {
    let _ = est;
    if has_vol(c.b, Side::Opp, conds::ENCORE).is_some() {
        let a = &c.b.side(Side::Own).active;
        let setup: Vec<usize> = c
            .moves()
            .iter()
            .copied()
            .filter(|&x| {
                c.mv(x).map_or(false, |mv| {
                    let r = lists::raised(mv);
                    lists::is(mv, class::SETUP) && !((0..7).any(|i| r[i]) && (0..7).filter(|&i| r[i]).all(|i| a.boosts[i] >= 6))
                })
            })
            .collect();
        return if setup.is_empty() { None } else { Some((rng.pick(&setup), 2)) };
    }
    let enc = c.by_ids(&["encore"]).next()?;
    let (mv, status) = foe_last(c.b)??;
    if status || RECOVERY.contains(&mv.id()) {
        return Some((enc, 3));
    }
    None
}

fn weather_rule(c: &Ctx, est: &mut Estimator, l: &Legal) -> Option<(usize, usize)> {
    let b = c.b;
    let mut ours = [false; 4];
    for i in 0..(b.n_own as usize).min(6) {
        let s = &b.sets[i];
        if let Some(k) = idx(&WEATHER_ABILITIES, s.ability.id()) {
            ours[k] = true;
        }
        for m in s.moves.iter().flatten() {
            if let Some(k) = idx(&WEATHER_MOVES, m.id()) {
                ours[k] = true;
            }
        }
    }
    if !ours.iter().any(|x| *x) {
        return None;
    }
    let up = b.field.weather.and_then(|w| idx(&WEATHER_OF, w.cond.id())).filter(|&k| ours[k]);
    if let Some(k) = up {
        let boosted: Vec<usize> = c.by_ids(BOOSTED[k]).collect();
        if let Some((a, d)) = c.best(est, &boosted) {
            if d > 0.0 {
                return Some((a, 2 + k));
            }
        }
        return None;
    }
    let setter = c.moves().iter().copied().find(|&a| c.mv(a).and_then(|m| idx(&WEATHER_MOVES, m.id())).map_or(false, |k| ours[k]));
    if let Some(a) = setter {
        if !c.ko_threat(est) {
            return Some((a, 6));
        }
    }
    let switcher = (8..N_ACTIONS).find(|&a| {
        l.mask[a] && b.picks[a - 8].map_or(false, |s| idx(&WEATHER_ABILITIES, b.sets[s as usize].ability.id()).map_or(false, |k| ours[k]))
    });
    if let Some(a) = switcher {
        if !c.ko_threat(est) {
            return Some((a, 7));
        }
    }
    None
}

fn hazard_rule(c: &Ctx, est: &mut Estimator) -> Option<(usize, usize)> {
    let b = c.b;
    if !c.ko_threat(est) {
        for &a in c.moves() {
            let Some(mv) = c.mv(a) else { continue };
            if let Some(h) = HAZARDS.iter().position(|h| h.0 == mv.id()) {
                if side_cond(b, Side::Opp, HAZARDS[h].1).unwrap_or(0) < HAZARDS[h].2 {
                    return Some((a, 2 + h));
                }
            }
        }
    }
    let layers: u32 = HAZARDS
        .iter()
        .filter_map(|h| side_cond(b, Side::Opp, h.1).map(|n| if matches!(h.1, "spikes" | "toxicspikes") { n as u32 } else { 1 }))
        .sum();
    let boosted = b.side(Side::Opp).active.boosts.iter().map(|&x| x.max(0) as i32).sum::<i32>() > 0;
    if let Some(a) = c.by_ids(&PHAZES).next() {
        if boosted || layers >= 2 {
            return Some((a, 6));
        }
    }
    if let Some(a) = c.by_ids(&PIVOTS).next() {
        if layers >= 1 && c.ko_threat(est) {
            return Some((a, 7));
        }
    }
    None
}

fn priority_rule(c: &Ctx, est: &mut Estimator) -> Option<(usize, usize)> {
    let b = c.b;
    let user = est.fighter(b, c.me, false);
    let field = est.field;
    let first_turn = b.side(Side::Own).active.move_actions == 0;
    if first_turn {
        if let Some(a) = c.by_ids(&["fakeout"]).next() {
            let ty = moves::FAKEOUT.data().ty;
            let hits = b.types(c.foe).map_or(true, |t| type_mult(ty, &t) > 0.0);
            if hits {
                return Some((a, 2));
            }
        }
    }
    let last = foe_last(b);
    let prio: Vec<usize> = c
        .moves()
        .iter()
        .copied()
        .filter(|&a| {
            let Some(mv) = c.mv(a) else { return false };
            if mv.data().category == Category::Status || priority(&user, mv, &field) <= 0 {
                return false;
            }
            if mv.id() == "suckerpunch" && !matches!(last, Some(Some((_, false)))) {
                return false;
            }
            !FIRST_TURN.contains(&mv.id()) || first_turn
        })
        .collect();
    let (a, d) = c.best(est, &prio)?;
    if d >= b.mon(c.foe).hp / 100.0 && d > 0.0 {
        return Some((a, 3));
    }
    if !faster(b, est, c.me, c.foe) && c.ko_threat(est) && d > 0.0 {
        return Some((a, 4));
    }
    None
}

impl Player for PilotPlayer {
    fn name(&self) -> &'static str {
        match self.kind {
            Kind::Encore => "pilot_encore",
            Kind::Weather => "pilot_weather",
            Kind::Hazard => "pilot_hazard",
            Kind::Priority => "pilot_priority",
        }
    }
    fn teampreview(&mut self, b: &Battle, _rng: &mut Rng) -> [u8; 3] {
        preview(self.kind, b, self.params.core)
    }
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, rng: &mut Rng) -> usize {
        let l = legal(b, kind);
        self.est.reset(b);
        // at a replacement asked before its rows our Pokémon on the field is the one leaving: no rule
        let me = b.side(Side::Own).active.slot.map(|slot| Who { side: Side::Own, slot }).filter(|_| !battle::legal::replace_unread(b, &l));
        let foe = b.side(Side::Opp).active.slot.map(|slot| Who { side: Side::Opp, slot });
        if let (Some(me), Some(foe), true) = (me, foe, l.count() > 0) {
            self.fired.hit(R_DECISIONS);
            let (moves, n) = move_actions(&l, &self.opts);
            let c = Ctx { b, l: &l, me, foe, moves, n };
            let hit = match self.kind {
                Kind::Encore => encore_rule(&c, &mut self.est, rng),
                Kind::Weather => weather_rule(&c, &mut self.est, &l),
                Kind::Hazard => hazard_rule(&c, &mut self.est),
                Kind::Priority => priority_rule(&c, &mut self.est),
            };
            if let Some((a, rule)) = hit {
                self.fired.hit(R_APPLIES);
                if rng.unit() < self.params.rule_p {
                    self.fired.hit(rule);
                    return a;
                }
            }
        }
        tower::choose(b, &l, &self.opts, &mut self.est, rng)
    }
    fn fired(&self) -> Vec<(&'static str, u64)> {
        self.fired.list()
    }
}
