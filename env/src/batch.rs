//! `BatchEnv`: N games with strict turn-taking. Each game always shows exactly one pending decision
//! (game, seat, kind). When both seats choose in the same game turn seat 0 is asked first, then seat 1, and
//! the step resolves when both are in. A finished game records its result and restarts from the team pool.
//!
//! Every game keeps one Battle Builder per seat, fed after each step with the contract events that seat's
//! battle log would show (`emit`). In the live order (`EnvConfig::replace_before_rows`, the default) a seat's
//! rows reach its Builder at its next turn request or the game's end, so a replace request is answered
//! before the rows of its turn, as on the phone. What a seat may choose comes from its Battle (`BattleRule`); `StateRule`
//! reads the State instead, for comparison. A seat locked into a move, charging or recharging is not asked:
//! the game shows no action screen then. When the State refuses an action the Battle allowed (a trap or a
//! disabled move not yet shown, or something the Battle could not know), the State learns it as the game
//! does (`State::refuse`), the seat's Battle gets the `refused` event and the seat is asked again with
//! the refusal in its request.
//!
//! Games are independent: `act` steps them over a persistent worker pool (`set_threads`), each game on one
//! worker, with results identical to one thread for the same seed.
//!
//! Action layout, one `u16` per game:
//! - turn and replace: 0..4 move slot i; 4..8 move slot i with Mega Evolution; 8..11 switch to team slot
//!   0..2 (slots are the picks in pick order). When only Struggle or a forced move is possible, action 0.
//! - preview: `lead * 10 + pair` (60 actions): `lead` is a roster index 0..5; the other five indices in
//!   ascending order are numbered 0..4 and `pair` numbers their pairs (0,1) (0,2) (0,3) (0,4) (1,2) (1,3)
//!   (1,4) (2,3) (2,4) (3,4) as 0..9. Picks are [lead, first of pair, second of pair].

use crate::game::Game;
use crate::hidden::{self, Call};
use crate::labels::{self, Asked, Label, Took};
use crate::pool::{Chunks, Pool, Shards};
use crate::teams::{mix, Team, TeamMode, TeamPool};
use battle::{legal, order_for, Battle, Forced, ReqKind};
use emit::check::{self, Tally};
use emit::Seats;
use cg_engine::sim::battle::STRUGGLE;
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::{State, Winner, SIDES, TEAM};
use cg_engine::sim::{Choice, Kind, SideOptions};
use std::sync::{Arc, Mutex};

pub const N_TURN_ACTIONS: usize = 11;
pub const N_PREVIEW_ACTIONS: usize = 60;
/// the mask length: the larger of the two layouts
pub const N_ACTIONS: usize = N_PREVIEW_ACTIONS;
pub const SWITCH0: u16 = 8;
pub const MEGA0: u16 = 4;

const PAIRS: [(u8, u8); 10] = [(0, 1), (0, 2), (0, 3), (0, 4), (1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)];

/// Picks (lead first) for a preview action.
pub fn preview_picks(action: u16) -> Option<[u8; TEAM]> {
    if action as usize >= N_PREVIEW_ACTIONS {
        return None;
    }
    let lead = (action / 10) as u8;
    let (i, j) = PAIRS[(action % 10) as usize];
    let mut rest = [0u8; 5];
    let mut k = 0;
    for x in 0..6u8 {
        if x != lead {
            rest[k] = x;
            k += 1;
        }
    }
    Some([lead, rest[i as usize], rest[j as usize]])
}

/// The preview action for picks; the order of the two back picks does not matter.
pub fn preview_action(picks: [u8; TEAM]) -> Option<u16> {
    let rest: Vec<u8> = (0..6u8).filter(|&x| x != picks[0]).collect();
    let a = rest.iter().position(|&x| x == picks[1])? as u8;
    let b = rest.iter().position(|&x| x == picks[2])? as u8;
    let (a, b) = if a < b { (a, b) } else { (b, a) };
    let p = PAIRS.iter().position(|&x| x == (a, b))?;
    Some(picks[0] as u16 * 10 + p as u16)
}

/// The engine choice for a turn or replace action under these options, if legal.
pub fn choice_of(o: &SideOptions, action: u16) -> Option<Choice> {
    let c = match action {
        0..=3 if o.kind == Kind::Turn && (o.struggle || o.forced.is_some()) => {
            if action != 0 {
                return None;
            }
            Choice::Move { index: STRUGGLE, mega: false }
        }
        0..=3 => Choice::Move { index: action as u8, mega: false },
        4..=7 => Choice::Move { index: (action - MEGA0) as u8, mega: true },
        8..=10 => Choice::Switch((action - SWITCH0) as u8),
        _ => return None,
    };
    if o.allows(c) {
        Some(c)
    } else {
        None
    }
}

/// The legal actions under these options, in the layout above.
pub fn turn_actions(o: &SideOptions, out: &mut Vec<u16>) {
    each_turn_action(o, |a| out.push(a));
}

/// Calls `f` with each legal action under these options, in ascending order.
pub fn each_turn_action(o: &SideOptions, mut f: impl FnMut(u16)) {
    if o.kind == Kind::Turn {
        if o.struggle || o.forced.is_some() {
            f(0);
        } else {
            for i in 0..4u16 {
                if o.moves[i as usize] {
                    f(i);
                }
            }
            if o.mega {
                for i in 0..4u16 {
                    if o.moves[i as usize] {
                        f(MEGA0 + i);
                    }
                }
            }
        }
    }
    if matches!(o.kind, Kind::Turn | Kind::Replace) {
        for s in 0..TEAM as u16 {
            if o.switches[s as usize] {
                f(SWITCH0 + s);
            }
        }
    }
}

/// What a seat may choose. The kind of decision (turn, replace, none) is always the State's: whether the
/// game asks is not something a player decides. What may be chosen is the rule's.
pub trait LegalRule: Sync {
    fn options(&self, s: &State, battles: [&Battle; SIDES]) -> [SideOptions; SIDES];
    /// the rule as a value, for a Game's views
    fn as_rule(&self) -> Rule;
}

/// The State's truth (options.rs): kept for comparison.
#[derive(Clone, Copy, Debug, Default)]
pub struct StateRule;

impl LegalRule for StateRule {
    #[inline]
    fn options(&self, s: &State, _: [&Battle; SIDES]) -> [SideOptions; SIDES] {
        s.options()
    }
    fn as_rule(&self) -> Rule {
        Rule::State
    }
}

/// Each seat's Battle (battle::legal), the rule a player has in live play.
#[derive(Clone, Copy, Debug, Default)]
pub struct BattleRule;

/// A seat's options from its Battle, for a decision of the State's kind.
pub fn battle_options(b: &Battle, kind: Kind) -> SideOptions {
    let rk = match kind {
        Kind::Turn => ReqKind::Turn,
        Kind::Replace => ReqKind::Replace,
        _ => return SideOptions { kind, ..SideOptions::default() },
    };
    let l = legal(b, rk);
    let mut o = SideOptions { kind, ..SideOptions::default() };
    match l.forced {
        Some(Forced::Struggle) => o.struggle = true,
        Some(Forced::Recharge) => o.forced = Some("recharge"),
        Some(Forced::Move(mv)) => o.forced = Some(mv.id()),
        None => {
            for i in 0..4 {
                o.moves[i] = l.mask[i];
            }
            o.mega = (4..8).any(|i| l.mask[i]);
        }
    }
    for i in 0..TEAM {
        o.switches[i] = l.mask[8 + i];
    }
    o
}

impl LegalRule for BattleRule {
    fn options(&self, s: &State, battles: [&Battle; SIDES]) -> [SideOptions; SIDES] {
        let st = s.options();
        let mut out = st;
        for side in 0..SIDES {
            if deciding(&st[side]) {
                out[side] = battle_options(battles[side], st[side].kind);
            }
        }
        out
    }
    fn as_rule(&self) -> Rule {
        Rule::Battle
    }
}

/// Either rule, chosen at run time (the Python environment's `rule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Rule {
    #[default]
    Battle,
    State,
}

impl LegalRule for Rule {
    fn options(&self, s: &State, battles: [&Battle; SIDES]) -> [SideOptions; SIDES] {
        match self {
            Rule::Battle => BattleRule.options(s, battles),
            Rule::State => StateRule.options(s, battles),
        }
    }
    fn as_rule(&self) -> Rule {
        *self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestKind {
    Preview,
    Turn,
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    pub seat: u8,
    pub kind: RequestKind,
}

/// A game that ended during the last `act`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameResult {
    pub done: bool,
    /// 0 or 1; None for a tie or a truncated game
    pub winner: Option<u8>,
    pub turns: u16,
    /// stopped at the step cap
    pub truncated: bool,
    /// the episode number that ended (0 based, per game)
    pub episode: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct EnvConfig {
    pub games: usize,
    pub seed: u64,
    pub team_mode: TeamMode,
    /// steps after which a game is stopped as truncated
    pub max_steps: u32,
    /// compare each asked seat's Battle with the State (checks a and b of the runner), check the labels
    /// and replay a sample of games with hidden information changed
    pub checks: bool,
    /// keep each game's full record per seat (the rebuild check; `last_records`)
    pub keep_records: bool,
    /// a seat locked into a move, charging or recharging is not asked (the game shows no action screen);
    /// false asks it anyway, as the environment did before the Battle existed
    pub auto_locked: bool,
    /// the live order (LIVE.md section 2, LV5Q-7): a seat's log rows reach its Battle only at its next
    /// `turn` request (or the game's end), so a `replace` request is answered before the rows of its turn;
    /// false delivers each step's rows at once
    pub replace_before_rows: bool,
}

impl Default for EnvConfig {
    fn default() -> Self {
        EnvConfig { games: 1, seed: 0, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: false, auto_locked: true, replace_before_rows: true }
    }
}

/// What the environment counted since creation.
#[derive(Clone, Debug, Default)]
pub struct Counters {
    /// decisions the seats were asked for (refused ones included)
    pub asked: u64,
    /// decisions taken for a seat that the game does not ask (locked, charging, recharging)
    pub auto: u64,
    /// orders the State refused although the Battle allowed them
    pub refusals: u64,
    /// decisions where the Battle allowed nothing and the State's options were used
    pub battle_empty: u64,
    /// decisions where the Battle kept offering a refused action and the State's options were used
    pub refusal_fallbacks: u64,
    /// decisions checked (a and b)
    pub checked: u64,
    /// of those, Battle and State legal sets equal
    pub legal_equal: u64,
    /// decisions where the Battle forbade something the State allows
    pub battle_forbids: u64,
    /// decisions where the Battle allowed something the State refuses
    pub battle_allows_more: u64,
    /// (a) disagreements by field
    pub agree: Tally,
    /// (b) legal-set differences by class
    pub legal: Tally,
    /// refusals by class
    pub refused: Tally,
    /// builder anomalies by kind
    pub anomalies: Tally,
    /// (d) games whose Battles were rebuilt from the record, and how many matched
    pub rebuilt: u64,
    pub rebuild_equal: u64,
    /// steps played and rows recorded
    pub steps: u64,
    pub rows: u64,
    /// refusals the engine gave a reason for: hidden trap, hidden disabled move
    pub refused_trapped: u64,
    pub refused_disabled: u64,
    /// games that ended in an engine error
    pub errors: u64,
    /// labels checked; opposing choices the label's opp_prior gave 0 (by class); truth labels that
    /// contradict the State or the decision's Battle (by field)
    pub labels_checked: u64,
    pub prior0: Tally,
    pub truth: Tally,
    /// games replayed with hidden information changed, steps compared, and what a seat was shown
    /// differently while nothing it can see differed (by kind)
    pub hidden_games: u64,
    pub hidden_steps: u64,
    pub hidden: Tally,
    /// replace requests asked (refused ones included); of the first asks, those asked before rows the
    /// seat had not been given (`replace_before_rows`), and the events held back at them
    pub replace_asked: u64,
    pub stale_replace: u64,
    pub stale_events: u64,
    /// stale replace requests whose Battle was checked (a) against the State at the seat's last `turn`
    /// request, and those with no such State yet (before the first `turn` request)
    pub stale_checked: u64,
    pub stale_unsnapped: u64,
    /// stale replace requests replayed from the step before them with other outcomes (sampled games): the
    /// replays that asked the same seat to replace, and those whose rows differed from the game's
    pub replace_twins: u64,
    pub replace_twins_varied: u64,
    /// nanoseconds by section when profiling (`set_profile`), named by `PROF_NAMES`
    pub prof: [u64; PROF_SECTIONS],
}

/// Sections of `Counters::prof`: engine, emitter, Builders, legal options, the asked record (row order and
/// opp_prior), step labels, requests and orders, game end and restart, the whole of each game's step.
pub const PROF_SECTIONS: usize = 9;
pub const PROF_NAMES: [&str; PROF_SECTIONS] = ["engine", "emit", "builders", "options", "asked", "labels", "requests", "restart", "act_total"];

impl Counters {
    pub fn merge(&mut self, b: &Counters) {
        self.asked += b.asked;
        self.auto += b.auto;
        self.refusals += b.refusals;
        self.battle_empty += b.battle_empty;
        self.refusal_fallbacks += b.refusal_fallbacks;
        self.checked += b.checked;
        self.legal_equal += b.legal_equal;
        self.battle_forbids += b.battle_forbids;
        self.battle_allows_more += b.battle_allows_more;
        self.agree.merge(&b.agree);
        self.legal.merge(&b.legal);
        self.refused.merge(&b.refused);
        self.anomalies.merge(&b.anomalies);
        self.rebuilt += b.rebuilt;
        self.rebuild_equal += b.rebuild_equal;
        self.steps += b.steps;
        self.rows += b.rows;
        self.refused_trapped += b.refused_trapped;
        self.refused_disabled += b.refused_disabled;
        self.errors += b.errors;
        self.labels_checked += b.labels_checked;
        self.prior0.merge(&b.prior0);
        self.truth.merge(&b.truth);
        self.hidden_games += b.hidden_games;
        self.hidden_steps += b.hidden_steps;
        self.hidden.merge(&b.hidden);
        self.replace_asked += b.replace_asked;
        self.stale_replace += b.stale_replace;
        self.stale_events += b.stale_events;
        self.stale_checked += b.stale_checked;
        self.stale_unsnapped += b.stale_unsnapped;
        self.replace_twins += b.replace_twins;
        self.replace_twins_varied += b.replace_twins_varied;
        for i in 0..PROF_SECTIONS {
            self.prof[i] += b.prof[i];
        }
    }
}

#[derive(Clone, Debug)]
pub struct Slot {
    /// the game; during preview it holds the previous game's end (or a default State) and is not used
    pub game: Game,
    pub in_preview: bool,
    pub rosters: [Team; 2],
    pub team_ids: [usize; 2],
    pub picks: [Option<[u8; TEAM]>; SIDES],
    pub chosen: [Option<Choice>; SIDES],
    /// what each seat may choose (the rule's)
    pub opts: [SideOptions; SIDES],
    /// the State's own options
    pub state_opts: [SideOptions; SIDES],
    pub episode: u64,
    pub steps: u32,
    pub episode_seed: u64,
    pub pending: Pending,
    /// both seats' Battles
    pub seats: Seats,
    /// the records of the game that ended last (when kept)
    pub last_records: Option<[Vec<battle::Input>; 2]>,
    /// the rows of the last step played
    pub last_rows: Vec<cg_engine::sim::Row>,
    /// actions the State refused at the pending decision
    pub refused_here: Vec<u16>,
    /// id of the pending decision (per game, from 1, never reset)
    pub decision: u64,
    /// the pending decision of each seat as asked (for its label)
    pub asked: [Asked; SIDES],
    /// what each seat did so far in the step under way
    pub took: [Took; SIDES],
    /// every call made on `seats` this game, for the hidden-information replay (sampled games, checks on)
    pub log: Option<Vec<Call>>,
    /// per seat, the State (and the opposing disguise) at its last `turn` request: what its Battle holds
    /// at a replace request asked before the rows (checks on, `replace_before_rows`)
    pub at_turn: [Option<(State, Option<(u8, u8)>)>; SIDES],
}

/// The settings a slot's Battles, options and team ids depend on: a save restores only into an
/// environment with the same ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settings {
    rule: Rule,
    team_mode: TeamMode,
    auto_locked: bool,
    replace_before_rows: bool,
    checks: bool,
    keep_records: bool,
}

/// A game of the environment at its pending decision, everything its slot holds for it (`BatchEnv::save`),
/// to be put back into a slot with `BatchEnv::restore`.
#[derive(Clone, Debug)]
pub struct SlotSave {
    pub game: Game,
    pub team_ids: [usize; 2],
    pub agent: u8,
    rosters: [Team; 2],
    picks: [Option<[u8; TEAM]>; SIDES],
    chosen: [Option<Choice>; SIDES],
    opts: [SideOptions; SIDES],
    state_opts: [SideOptions; SIDES],
    steps: u32,
    pending: Pending,
    seats: Seats,
    refused_here: Vec<u16>,
    decision: u64,
    asked: [Asked; SIDES],
    took: [Took; SIDES],
    at_turn: Option<Box<[Option<(State, Option<(u8, u8)>)>; SIDES]>>,
    settings: Settings,
    pool: Arc<TeamPool>,
}

impl SlotSave {
    pub fn pending(&self) -> Pending {
        self.pending
    }
}

/// A copy of a slot's Seats for a save: the last step's events are left out (the next step sets them).
fn seats_copy(s: &Seats) -> Seats {
    Seats {
        builders: s.builders.clone(),
        records: if s.keep { s.records.clone() } else { Default::default() },
        keep: s.keep,
        tag: s.tag.clone(),
        seq: s.seq,
        emitter: s.emitter.clone(),
        last: Default::default(),
        profile: false,
        prof: [0; 3],
        replace_before_rows: s.replace_before_rows,
        held: s.held.clone(),
        held_events: s.held_events,
    }
}

/// One unit of work's counts (a chunk of games in one `act`), merged in game order afterwards.
#[derive(Default)]
struct Local {
    counters: Counters,
    games_done: u64,
    err: Option<String>,
}

/// What every game's step reads; the counts it adds go to `c`.
struct Ctx<'a, R> {
    cfg: &'a EnvConfig,
    pool: &'a TeamPool,
    rule: &'a R,
    c: &'a mut Counters,
    games_done: &'a mut u64,
    prof: bool,
    /// each game's agent seat: the seat that draws first from weighted teams
    agent: &'a [u8],
}

pub struct BatchEnv<R: LegalRule = BattleRule> {
    cfg: EnvConfig,
    pool: Arc<TeamPool>,
    rule: R,
    slots: Vec<Slot>,
    results: Vec<GameResult>,
    labels: Vec<[Label; SIDES]>,
    /// totals since creation
    pub games_done: u64,
    pub decisions: u64,
    pub counters: Counters,
    workers: Option<Pool>,
    locals: Vec<Mutex<Local>>,
    profile: bool,
    /// each game's agent seat as configured: the seat that draws first
    agent: Vec<u8>,
    /// each game's agent seat now: a restored game brings its own until it ends
    agent_now: Vec<u8>,
    /// games whose restored game has an agent seat other than the configured one
    agent_moved: Vec<usize>,
}

/// Games per unit of work handed to a worker.
fn chunk_len(n: usize, threads: usize) -> usize {
    (n / (threads * 8)).clamp(8, 256)
}

impl BatchEnv<BattleRule> {
    pub fn new(pool: Arc<TeamPool>, cfg: EnvConfig) -> BatchEnv<BattleRule> {
        BatchEnv::with_rule(pool, cfg, BattleRule)
    }
}

impl<R: LegalRule> BatchEnv<R> {
    pub fn with_rule(pool: Arc<TeamPool>, cfg: EnvConfig, rule: R) -> BatchEnv<R> {
        let agent = vec![0; cfg.games];
        BatchEnv::with_agent(pool, cfg, rule, agent)
    }

    /// `with_rule` with each game's agent seat (0 or 1, one per game): the seat that draws its team first,
    /// from the first weights of a weighted pool (`TeamPool::set_weights`). Without weights it changes nothing.
    pub fn with_agent(pool: Arc<TeamPool>, cfg: EnvConfig, rule: R, agent: Vec<u8>) -> BatchEnv<R> {
        assert_eq!(agent.len(), cfg.games, "one agent seat per game");
        assert!(agent.iter().all(|&x| x < 2), "agent seats are 0 or 1");
        let mut env = BatchEnv {
            cfg,
            pool,
            rule,
            slots: Vec::with_capacity(cfg.games),
            results: vec![GameResult::default(); cfg.games],
            labels: vec![[Label::default(); SIDES]; cfg.games],
            games_done: 0,
            decisions: 0,
            counters: Counters::default(),
            workers: None,
            locals: Vec::new(),
            profile: false,
            agent_now: agent.clone(),
            agent,
            agent_moved: Vec::new(),
        };
        let mut done = 0;
        let mut cx = Ctx { cfg: &env.cfg, pool: &env.pool, rule: &env.rule, c: &mut env.counters, games_done: &mut done, prof: false, agent: &env.agent };
        for g in 0..cfg.games {
            let mut s = Slot {
                game: Game { state: State::default(), rng: Seeded::new(0) },
                in_preview: true,
                rosters: [cx.pool.teams[0], cx.pool.teams[1]],
                team_ids: [0, 1],
                picks: [None; SIDES],
                chosen: [None; SIDES],
                opts: [SideOptions::default(); SIDES],
                state_opts: [SideOptions::default(); SIDES],
                episode: 0,
                steps: 0,
                episode_seed: 0,
                pending: Pending { seat: 0, kind: RequestKind::Preview },
                seats: Seats::default(),
                last_records: None,
                last_rows: Vec::with_capacity(64),
                refused_here: Vec::new(),
                decision: 0,
                asked: [Asked::default(); SIDES],
                took: [Took::Nothing; SIDES],
                log: None,
                at_turn: [None; SIDES],
            };
            cx.reset_slot(&mut s, g);
            env.slots.push(s);
        }
        env
    }

    /// Worker threads `act` and `observe_batch` use (1: the calling thread only).
    pub fn set_threads(&mut self, threads: usize) {
        let t = threads.max(1);
        if t == self.threads() {
            return;
        }
        self.workers = (t > 1).then(|| Pool::new(t));
    }

    /// Time the sections of every step into `counters.prof` (a little slower).
    pub fn set_profile(&mut self, on: bool) {
        self.profile = on;
    }

    pub fn threads(&self) -> usize {
        self.workers.as_ref().map_or(1, |p| p.threads())
    }

    pub(crate) fn workers(&self) -> Option<&Pool> {
        self.workers.as_ref()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn config(&self) -> &EnvConfig {
        &self.cfg
    }

    pub fn pool(&self) -> &TeamPool {
        &self.pool
    }

    /// Each game's agent seat (`with_agent`; 0 by default; a restored game's own until it ends).
    pub fn agent_seats(&self) -> &[u8] {
        &self.agent_now
    }

    fn settings(&self) -> Settings {
        let c = &self.cfg;
        Settings { rule: self.rule.as_rule(), team_mode: c.team_mode, auto_locked: c.auto_locked, replace_before_rows: c.replace_before_rows, checks: c.checks, keep_records: c.keep_records }
    }

    /// Game `g` as its slot holds it at the pending decision; None during team preview.
    pub fn save(&self, g: usize) -> Option<SlotSave> {
        let s = &self.slots[g];
        if s.in_preview {
            return None;
        }
        Some(SlotSave {
            game: s.game.clone(),
            team_ids: s.team_ids,
            agent: self.agent_now[g],
            rosters: s.rosters,
            picks: s.picks,
            chosen: s.chosen,
            opts: s.opts,
            state_opts: s.state_opts,
            steps: s.steps,
            pending: s.pending,
            seats: seats_copy(&s.seats),
            refused_here: s.refused_here.clone(),
            decision: s.decision,
            asked: s.asked,
            took: s.took,
            at_turn: self.cfg.checks.then(|| Box::new(s.at_turn)),
            settings: self.settings(),
            pool: self.pool.clone(),
        })
    }

    /// Puts a saved game into slot `g`, its luck from then on drawn from `seed` alone (as
    /// `Game::fork(seed)`), waiting on the save's pending decision. Both seats' Battles, the decision ids,
    /// the step count, the team ids and the agent seat come with it; the slot's episode number does not
    /// change and the game ends as any other (its result counted once), then the slot restarts as usual.
    /// Only into a slot whose game has not started (team preview with no picks given: a game just
    /// ended, or a new environment), of an environment with the same settings and teams.
    pub fn restore(&mut self, g: usize, save: &SlotSave, seed: u64) -> Result<(), String> {
        if g >= self.slots.len() {
            return Err(format!("restore: game {} of {}", g, self.slots.len()));
        }
        let mine = self.settings();
        if save.settings != mine {
            return Err(format!("restore: the save comes from an environment with other settings ({:?}; this one {:?})", save.settings, mine));
        }
        if !Arc::ptr_eq(&save.pool, &self.pool) && save.pool.teams != self.pool.teams {
            return Err("restore: the save comes from an environment with other teams".into());
        }
        let s = &mut self.slots[g];
        if !s.in_preview || s.picks.iter().any(|p| p.is_some()) {
            let at = if s.in_preview { "in team preview with picks given".to_string() } else { format!("at turn {}", s.game.state.turn) };
            return Err(format!("restore: game {} is being played ({}); restore into a slot whose game just ended, before acting on it", g, at));
        }
        s.game = Game { state: save.game.state, rng: Seeded::new(seed) };
        s.in_preview = false;
        s.rosters = save.rosters;
        s.team_ids = save.team_ids;
        s.picks = save.picks;
        s.chosen = save.chosen;
        s.opts = save.opts;
        s.state_opts = save.state_opts;
        s.steps = save.steps;
        s.pending = save.pending;
        s.seats = seats_copy(&save.seats);
        s.refused_here.clone_from(&save.refused_here);
        s.decision = save.decision;
        s.asked = save.asked;
        s.took = save.took;
        // the hidden-information replay starts from the game's first step: not for a restored game
        s.log = None;
        s.at_turn = save.at_turn.as_deref().copied().unwrap_or([None; SIDES]);
        self.agent_now[g] = save.agent;
        if save.agent != self.agent[g] && !self.agent_moved.contains(&g) {
            self.agent_moved.push(g);
        }
        Ok(())
    }

    /// After an act: a restored game that ended gives its slot back its configured agent seat.
    fn settle_agents(&mut self) {
        let (results, agent, now) = (&self.results, &self.agent, &mut self.agent_now);
        self.agent_moved.retain(|&g| {
            if results[g].done {
                now[g] = agent[g];
                false
            } else {
                true
            }
        });
    }

    /// `out[g] = f(&mut state, g, slot)` for every game, on the worker pool in chunks of games, `init`
    /// making one state per chunk. The games are only read; when `f` depends only on its game and not on
    /// what an earlier call left in `state`, the result does not depend on the thread count.
    pub fn map_games<S, T: Send>(&self, out: &mut [T], init: impl Fn() -> S + Sync, f: impl Fn(&mut S, usize, &Slot) -> T + Sync) {
        let n = self.slots.len();
        assert_eq!(out.len(), n, "map_games: one output per game");
        let threads = self.threads();
        if threads == 1 || n < 32 {
            let mut st = init();
            for (g, o) in out.iter_mut().enumerate() {
                *o = f(&mut st, g, &self.slots[g]);
            }
            return;
        }
        let ch = chunk_len(n, threads);
        let chunks = Chunks::new(n.div_ceil(ch));
        let outs = Shards::new(out);
        let job = |_: usize| {
            while let Some(c) = chunks.take() {
                let (a, b) = (c * ch, ((c + 1) * ch).min(n));
                // each chunk is handed out once: its outputs are this worker's alone
                let os = unsafe { outs.part(a, b) };
                let mut st = init();
                for (i, o) in os.iter_mut().enumerate() {
                    *o = f(&mut st, a + i, &self.slots[a + i]);
                }
            }
        };
        self.workers.as_ref().expect("pool").run(&job);
    }

    /// The one decision each game waits for.
    pub fn pending(&self, g: usize) -> Pending {
        self.slots[g].pending
    }

    pub fn slot(&self, g: usize) -> &Slot {
        &self.slots[g]
    }

    /// Game `g` with both seats' views as they are now (`views`); None during team preview.
    pub fn tracked(&self, g: usize) -> Option<(Game, crate::views::Tracked)> {
        let s = &self.slots[g];
        if s.in_preview {
            return None;
        }
        let v = crate::views::Views::of_slot(s, self.rule.as_rule(), self.cfg.auto_locked);
        Some((s.game.clone(), crate::views::Tracked::new(s.game.state, v)))
    }

    /// The Battle of a seat of game `g`: what that seat knows.
    pub fn battle(&self, g: usize, seat: usize) -> &Battle {
        &self.slots[g].seats.builders[seat].battle
    }

    /// Results of the last `act`, one per game (`done` false where nothing ended).
    pub fn results(&self) -> &[GameResult] {
        &self.results
    }

    /// Labels of the decisions the last `act` completed, per game and seat (`labels` module).
    pub fn labels(&self) -> &[[Label; SIDES]] {
        &self.labels
    }

    /// The legal actions of game `g`'s pending decision.
    pub fn legal_actions(&self, g: usize, out: &mut Vec<u16>) {
        out.clear();
        let s = &self.slots[g];
        match s.pending.kind {
            RequestKind::Preview => out.extend(0..N_PREVIEW_ACTIONS as u16),
            _ => turn_actions(&s.opts[s.pending.seat as usize], out),
        }
    }

    /// The legal-action mask of game `g` (the first 11 entries mean something for a turn or replace).
    pub fn mask(&self, g: usize) -> [bool; N_ACTIONS] {
        let mut m = [false; N_ACTIONS];
        let s = &self.slots[g];
        match s.pending.kind {
            RequestKind::Preview => m.fill(true),
            _ => each_turn_action(&s.opts[s.pending.seat as usize], |a| m[a as usize] = true),
        }
        m
    }

    /// One action per game for its pending decision. Every action is checked before any game moves; the
    /// games are then stepped on the worker pool.
    pub fn act(&mut self, actions: &[u16]) -> Result<(), String> {
        let n = self.slots.len();
        if actions.len() != n {
            return Err(format!("act: {} actions for {} games", actions.len(), n));
        }
        for (g, &a) in actions.iter().enumerate() {
            let s = &self.slots[g];
            let ok = match s.pending.kind {
                RequestKind::Preview => (a as usize) < N_PREVIEW_ACTIONS,
                _ => choice_of(&s.opts[s.pending.seat as usize], a).is_some(),
            };
            if !ok {
                return Err(format!("act: game {} seat {}: action {} not legal for {:?}", g, s.pending.seat, a, s.pending.kind));
            }
        }
        let r = self.step_all(actions);
        if !self.agent_moved.is_empty() {
            self.settle_agents();
        }
        r
    }

    /// `act` once its actions are checked.
    fn step_all(&mut self, actions: &[u16]) -> Result<(), String> {
        let n = self.slots.len();
        self.decisions += n as u64;
        let threads = self.threads();
        if threads == 1 || n < 32 {
            let mut done = 0;
            let mut cx = Ctx { cfg: &self.cfg, pool: &self.pool, rule: &self.rule, c: &mut self.counters, games_done: &mut done, prof: self.profile, agent: &self.agent };
            let mut err = None;
            for g in 0..n {
                if let Err(e) = cx.step_game(&mut self.slots[g], g, actions[g], &mut self.results[g], &mut self.labels[g]) {
                    err.get_or_insert(format!("act: game {}: {}", g, e));
                }
            }
            self.games_done += done;
            return err.map_or(Ok(()), Err);
        }
        let ch = chunk_len(n, threads);
        let nc = n.div_ceil(ch);
        while self.locals.len() < nc {
            self.locals.push(Mutex::new(Local::default()));
        }
        let slots = Shards::new(&mut self.slots);
        let results = Shards::new(&mut self.results);
        let labs = Shards::new(&mut self.labels);
        let chunks = Chunks::new(nc);
        let (cfg, pool, rule, locals, prof, agent) = (&self.cfg, &*self.pool, &self.rule, &self.locals, self.profile, &self.agent[..]);
        let job = |_: usize| {
            while let Some(c) = chunks.take() {
                let (a, b) = (c * ch, ((c + 1) * ch).min(n));
                // each chunk is handed out once: its games are this worker's alone
                let (ss, rs, ls) = unsafe { (slots.part(a, b), results.part(a, b), labs.part(a, b)) };
                let mut loc = locals[c].lock().unwrap();
                let Local { counters, games_done, err } = &mut *loc;
                let mut cx = Ctx { cfg, pool, rule, c: counters, games_done, prof, agent };
                for i in 0..b - a {
                    if let Err(e) = cx.step_game(&mut ss[i], a + i, actions[a + i], &mut rs[i], &mut ls[i]) {
                        err.get_or_insert(format!("act: game {}: {}", a + i, e));
                    }
                }
            }
        };
        self.workers.as_ref().expect("pool").run(&job);
        let mut err = None;
        for l in &self.locals[..nc] {
            let mut l = l.lock().unwrap();
            if self.cfg.checks || l.counters.asked > 0 {
                self.counters.merge(&l.counters);
            }
            self.games_done += l.games_done;
            if let Some(e) = l.err.take() {
                err.get_or_insert(e);
            }
            l.games_done = 0;
            l.counters = Counters::default();
        }
        err.map_or(Ok(()), Err)
    }
}

impl<R: LegalRule> Ctx<'_, R> {
    /// One game's action for its pending decision.
    fn step_game(&mut self, s: &mut Slot, g: usize, a: u16, res: &mut GameResult, lab: &mut [Label; SIDES]) -> Result<(), String> {
        let t = self.now();
        *res = GameResult::default();
        *lab = [Label::default(); SIDES];
        s.seats.profile = self.prof;
        let r = self.act_one(s, g, a, res, lab);
        if self.prof {
            for i in 0..3 {
                self.c.prof[i] += s.seats.prof[i];
            }
            s.seats.prof = [0; 3];
        }
        self.lap(8, t);
        r
    }

    #[inline]
    fn now(&self) -> Option<std::time::Instant> {
        self.prof.then(std::time::Instant::now)
    }

    #[inline]
    fn lap(&mut self, i: usize, t: Option<std::time::Instant>) {
        if let Some(t) = t {
            self.c.prof[i] += t.elapsed().as_nanos() as u64;
        }
    }

    fn act_one(&mut self, s: &mut Slot, g: usize, a: u16, res: &mut GameResult, lab: &mut [Label; SIDES]) -> Result<(), String> {
        let seat = s.pending.seat as usize;
        if s.in_preview {
            let picks = preview_picks(a).ok_or("bad preview action")?;
            let at = labels::asked(&s.seats.builders[seat].battle, s.decision, false);
            s.took[seat] = Took::Asked(Asked { action: labels::preview_action_code(a), ..at });
            s.picks[seat] = Some(picks);
            s.seats.picks(seat, picks);
            log(s, Call::Picks(seat, picks));
            if seat == 0 {
                s.pending = Pending { seat: 1, kind: RequestKind::Preview };
                s.decision += 1;
                return Ok(());
            }
            labels::preview_labels(&s.took, &s.picks, [&s.rosters[0], &s.rosters[1]], lab);
            if self.cfg.checks {
                self.check_labels(s, lab);
            }
            s.took = [Took::Nothing; SIDES];
            let picks = [s.picks[0].unwrap(), s.picks[1].unwrap()];
            s.last_rows.clear();
            s.game = Game::start_logged([&s.rosters[0], &s.rosters[1]], picks, mix(s.episode_seed, 1), &mut s.last_rows)?;
            s.seats.leads(&s.last_rows, &s.game.state);
            if s.log.is_some() {
                let rows = s.last_rows.clone();
                log(s, Call::Leads(rows));
            }
            s.in_preview = false;
            s.steps = 0;
            return self.advance(s, g, res, lab);
        }
        let c = match choice_of(&s.state_opts[seat], a) {
            Some(c) => c,
            None => return self.refuse(s, g, seat, a, lab),
        };
        s.chosen[seat] = Some(c);
        s.took[seat] = Took::Asked(Asked { action: a, ..s.asked[seat] });
        s.refused_here.clear();
        let t = self.now();
        let order = own_order(&s.game.state, seat, c);
        if s.log.is_some() {
            log(s, Call::Order(seat, order.clone()));
        }
        s.seats.order(seat, order);
        self.lap(6, t);
        if let Some(next) = (seat + 1..SIDES).find(|&x| deciding(&s.state_opts[x]) && s.chosen[x].is_none()) {
            self.ask(s, next);
            return Ok(());
        }
        self.resolve(s, g, res, lab)
    }

    /// The State refused what the Battle allowed: the game's refusal, and the seat is asked again.
    fn refuse(&mut self, s: &mut Slot, g: usize, seat: usize, a: u16, lab: &mut [Label; SIDES]) -> Result<(), String> {
        lab[seat] = labels::refused_label(s.decision, a);
        let kind = if s.state_opts[seat].kind == Kind::Replace { ReqKind::Replace } else { ReqKind::Turn };
        let b = s.seats.builders[seat].battle;
        let l = legal(&b, kind);
        self.c.refusals += 1;
        let class = emit::check::legal_classes(&b, &s.game.state, seat, kind, &s.state_opts[seat]);
        let key = class.iter().find(|c| c.starts_with("battle_only") || c.starts_with("explained:battle_only")).cloned().unwrap_or_else(|| "other".into());
        let (seed, ep, turn) = (self.cfg.seed, s.episode, s.game.state.turn);
        self.c.refused.add(&key, || format!("seed {} game {} ep {} turn {} seat {} action {}", seed, g, ep, turn, seat, a));
        // a choice the game offers and refuses: the State learns what it revealed
        if let Some(c) = choice_of(&s.opts[seat], a) {
            let mut ch = [None; SIDES];
            ch[seat] = Some(c);
            if let Some(why) = s.game.state.refuse(ch) {
                match why[seat] {
                    Some("trapped") => self.c.refused_trapped += 1,
                    Some(_) => self.c.refused_disabled += 1,
                    None => {}
                }
                s.state_opts[seat] = s.game.state.options()[seat];
            }
            if s.log.is_some() {
                let act = [s.game.state.sides[0].active, s.game.state.sides[1].active];
                log(s, Call::Chose(ch, act));
                log(s, Call::Refuse(ch));
            }
        }
        let order = match order_for(&b, &l, a as usize) {
            Some(o) => o,
            None => return Err(format!("seat {}: refused action {} has no order", seat, a)),
        };
        if s.log.is_some() {
            log(s, Call::Refused(seat, order.clone()));
        }
        s.seats.refused(seat, order.clone());
        s.refused_here.push(a);
        self.ask_with(s, seat, Some(order));
        // the Battle still offers what was refused: the seat gets the State's options
        if s.refused_here.iter().any(|&x| choice_of(&s.opts[seat], x).is_some()) {
            self.c.refusal_fallbacks += 1;
            s.opts[seat] = s.state_opts[seat];
        }
        Ok(())
    }

    /// Asks a seat: its request goes to its Battle, then its options are read from it.
    fn ask(&mut self, s: &mut Slot, seat: usize) {
        self.ask_with(s, seat, None)
    }

    fn ask_with(&mut self, s: &mut Slot, seat: usize, refused: Option<battle::Order>) {
        let st = s.state_opts[seat];
        let kind = if st.kind == Kind::Replace { ReqKind::Replace } else { ReqKind::Turn };
        let t = self.now();
        let offer = if kind == ReqKind::Turn { emit::screen_offer(&s.game.state, seat) } else { None };
        if s.log.is_some() {
            log(s, Call::Request(seat, kind, refused.clone()));
        }
        if kind == ReqKind::Replace {
            self.c.replace_asked += 1;
            if refused.is_none() && s.seats.held_events[seat] > 0 {
                self.c.stale_replace += 1;
                self.c.stale_events += s.seats.held_events[seat] as u64;
            }
        }
        s.seats.request(seat, kind, refused, offer);
        if kind == ReqKind::Turn && self.cfg.checks && self.cfg.replace_before_rows {
            s.at_turn[seat] = Some((s.game.state, s.seats.emitter.disguise[1 - seat]));
        }
        self.lap(6, t);
        let t = self.now();
        let b = [&s.seats.builders[0].battle, &s.seats.builders[1].battle];
        let o = self.rule.options(&s.game.state, b)[seat];
        let mut any = false;
        each_turn_action(&o, |_| any = true);
        s.opts[seat] = if !any {
            self.c.battle_empty += 1;
            st
        } else {
            o
        };
        self.lap(3, t);
        self.c.asked += 1;
        s.decision += 1;
        let t = self.now();
        s.asked[seat] = labels::asked(&s.seats.builders[seat].battle, s.decision, kind == ReqKind::Replace);
        self.lap(4, t);
        if self.cfg.checks {
            self.check(s, seat, kind);
        }
        s.pending = Pending { seat: seat as u8, kind: if kind == ReqKind::Replace { RequestKind::Replace } else { RequestKind::Turn } };
    }

    fn check(&mut self, s: &Slot, seat: usize, kind: ReqKind) {
        let b = &s.seats.builders[seat].battle;
        let st = &s.game.state;
        let ctx = format!("seed {} ep {} turn {} seat {}", self.cfg.seed, s.episode, st.turn, seat);
        if kind == ReqKind::Replace && s.seats.held_events[seat] > 0 {
            // the Battle is deliberately behind: it must agree with the State it was last given rows for
            match &s.at_turn[seat] {
                Some((st0, dis)) => {
                    check::agree(b, st0, seat, *dis, &format!("{} (at its last turn request)", ctx), &mut self.c.agree);
                    self.c.stale_checked += 1;
                }
                None => self.c.stale_unsnapped += 1,
            }
        } else {
            check::agree(b, st, seat, s.seats.emitter.disguise[1 - seat], &ctx, &mut self.c.agree);
        }
        let classes = check::legal_classes(b, st, seat, kind, &s.state_opts[seat]);
        self.c.checked += 1;
        if classes.is_empty() {
            self.c.legal_equal += 1;
        }
        if classes.iter().any(|c| c.starts_with("state_only")) {
            self.c.battle_forbids += 1;
        }
        if classes.iter().any(|c| c.starts_with("battle_only")) {
            self.c.battle_allows_more += 1;
        }
        for c in classes {
            self.c.legal.add(&c, || ctx.clone());
        }
    }

    /// The labels just made against what was played: the opposing choice never had prior 0, and the
    /// truth agrees with the State's sets and never contradicts the decision's Battle.
    fn check_labels(&mut self, s: &Slot, lab: &[Label; SIDES]) {
        for seat in 0..SIDES {
            let l = &lab[seat];
            let Took::Asked(a) = s.took[seat] else { continue };
            if !l.valid || l.refused {
                continue;
            }
            self.c.labels_checked += 1;
            let ctx = || format!("seed {} ep {} turn {} seat {} decision {}", self.cfg.seed, s.episode, s.game.state.turn, seat, l.decision);
            if l.opp_action >= 0 && l.opp_prior[l.opp_action as usize] == 0.0 {
                let class = if l.opp_action < 4 { "move" } else if l.opp_action < 8 { "mega" } else { "switch" };
                let all0 = l.opp_prior.iter().all(|x| *x == 0.0);
                // an opponent that brought Illusion shows Pokémon doing what another did
                let illusion = s.picks[1 - seat].map_or(false, |p| p.iter().any(|&i| s.rosters[1 - seat][i as usize].ability == cg_engine::sim::data::abilities::ILLUSION));
                let pre = if illusion { "explained:illusion " } else { "" };
                self.c.prior0.add(&format!("{}prior0:{}{}", pre, class, if all0 { ":all_zero" } else { "" }), || format!("{} action {} prior {:?}", ctx(), l.opp_action, l.opp_prior));
            }
            let o = 1 - seat;
            for i in 0..6 {
                let Ok(t) = usize::try_from(a.row_slots[i]) else { continue };
                let spec = &s.rosters[o][t];
                let brought = s.picks[o].map_or(false, |p| p.contains(&(t as u8)));
                let mut bad = |k: &str| self.c.truth.add(k, || format!("{} row {}", ctx(), 6 + i));
                if (l.opp_true_brought[i] == 1) != brought {
                    bad("truth:brought");
                }
                let set: Vec<i32> = spec.moves[..spec.nmoves as usize].iter().map(|m| obs::vocab::mv(Some(*m))).collect();
                let mut tm: Vec<i32> = l.opp_true_moves[i].iter().copied().filter(|x| *x != 0).collect();
                let mut sm = set.clone();
                tm.sort();
                sm.sort();
                if tm != sm {
                    bad("truth:moves");
                }
                for (j, m) in a.revealed[t].iter().enumerate() {
                    if let Some(m) = m {
                        let id = obs::vocab::mv(Some(*m));
                        if set.contains(&id) && l.opp_true_moves[i][j] != id {
                            bad("truth:move_slot");
                        }
                    }
                }
                if l.opp_true_item[i] != obs::vocab::item(battle::ItemState::Held(spec.item)) {
                    bad("truth:item");
                }
                if l.opp_true_ability[i] != obs::vocab::ability(Some(spec.ability)) {
                    bad("truth:ability");
                }
            }
        }
    }

    fn resolve(&mut self, s: &mut Slot, g: usize, res: &mut GameResult, lab: &mut [Label; SIDES]) -> Result<(), String> {
        let ch = s.chosen;
        s.chosen = [None; SIDES];
        let s0 = s.game.state;
        // sampled games: the step is replayed with other outcomes when it ends at a stale replace request
        let pre = (self.cfg.checks && self.cfg.replace_before_rows && s.log.is_some()).then(|| s.seats.view_copy());
        let Game { state, rng } = &mut s.game;
        s.seats.step_into(state, rng, ch, &mut s.last_rows)?;
        if s.log.is_some() {
            let rows = s.last_rows.clone();
            log(s, Call::Step(ch, [s0.sides[0].active, s0.sides[1].active], rows));
        }
        self.c.steps += 1;
        self.c.rows += s.last_rows.len() as u64;
        let t = self.now();
        let shown = &s.seats.last;
        labels::step_labels(&s0, &s.took, &ch, &s.picks, [&s.rosters[0], &s.rosters[1]], [&shown[0], &shown[1]], lab);
        self.lap(5, t);
        if self.cfg.checks {
            self.check_labels(s, lab);
        }
        s.took = [Took::Nothing; SIDES];
        s.steps += 1;
        let steps = s.steps;
        let r = self.advance(s, g, res, lab);
        if let Some(pre) = pre {
            if r.is_ok() && !s.in_preview && s.steps == steps && s.pending.kind == RequestKind::Replace {
                self.replace_twin(s, &s0, pre, ch);
            }
        }
        r
    }

    /// The step that ended at a stale replace request, replayed from the State before it with other
    /// outcomes: where the same seat must replace again, its Battle (and so its observation and mask) must
    /// be the same, whatever the unread rows were.
    fn replace_twin(&mut self, s: &Slot, s0: &State, pre: Seats, ch: [Option<Choice>; SIDES]) {
        let seat = s.pending.seat as usize;
        if s.seats.held_events[seat] == 0 {
            return;
        }
        let want = &s.seats.builders[seat].battle;
        let mut want_obs = obs::ObsBuffers::default();
        obs::observe(want, ReqKind::Replace, &mut want_obs);
        let mut rows = Vec::with_capacity(64);
        for k in 0..3u64 {
            let mut st = *s0;
            let mut seats = pre.clone();
            let mut rng = Seeded::new(mix(s.episode_seed, 0x7e91 + (s.steps as u64) * 8 + k));
            if seats.step_into(&mut st, &mut rng, ch, &mut rows).is_err() || st.ended || st.options()[seat].kind != Kind::Replace {
                continue;
            }
            self.c.replace_twins += 1;
            let same_rows = rows.len() == s.last_rows.len() && rows.iter().zip(&s.last_rows).all(|(a, b)| emit::fmt::show(a) == emit::fmt::show(b));
            if !same_rows {
                self.c.replace_twins_varied += 1;
            }
            seats.request(seat, ReqKind::Replace, None, None);
            let got = &seats.builders[seat].battle;
            let mut got_obs = obs::ObsBuffers::default();
            obs::observe(got, ReqKind::Replace, &mut got_obs);
            if got != want || got_obs != want_obs {
                let (seed, ep, turn) = (self.cfg.seed, s.episode, s.game.state.turn);
                self.c.hidden.add("hidden:replace_depends_on_unread_rows", || format!("seed {} ep {} turn {} seat {} replay {}", seed, ep, turn, seat, k));
            }
        }
    }

    /// After a step: finish and restart the game, or set the next pending decision.
    fn advance(&mut self, s: &mut Slot, g: usize, res: &mut GameResult, lab: &mut [Label; SIDES]) -> Result<(), String> {
        let st = s.game.state;
        if st.ended || s.steps >= self.cfg.max_steps {
            let winner = match st.winner {
                Winner::Side(x) if st.ended => Some(x),
                _ => None,
            };
            *res = GameResult { done: true, winner, turns: st.turn, truncated: !st.ended, episode: s.episode };
            *self.games_done += 1;
            let t = self.now();
            self.finish(s);
            s.episode += 1;
            self.reset_slot(s, g);
            self.lap(7, t);
            return Ok(());
        }
        s.state_opts = st.options();
        s.opts = s.state_opts;
        // a seat the game shows no action screen to takes its locked move
        for side in 0..SIDES {
            let o = &s.state_opts[side];
            if self.cfg.auto_locked && o.kind == Kind::Turn && st.locked_move(side).is_some() {
                let c = match o.forced {
                    Some(_) => Choice::Move { index: STRUGGLE, mega: false },
                    None => Choice::Move { index: o.moves.iter().position(|x| *x).unwrap_or(0) as u8, mega: false },
                };
                s.chosen[side] = Some(c);
                s.took[side] = Took::Locked;
                self.c.auto += 1;
            }
        }
        if let Some(first) = (0..SIDES).find(|&x| deciding(&s.state_opts[x]) && s.chosen[x].is_none()) {
            self.ask(s, first);
            return Ok(());
        }
        if s.chosen.iter().any(|c| c.is_some()) {
            return self.resolve(s, g, res, lab);
        }
        Err("no decision and not ended".into())
    }

    /// The end of a game: anomalies, the rebuild check and the hidden-information replay.
    fn finish(&mut self, s: &mut Slot) {
        // a truncated game's held rows still complete its Battles and records
        s.seats.flush_all();
        for seat in 0..SIDES {
            // an opponent that brought Illusion makes rows name Pokémon that did not do what they say
            let illusion = s.picks[1 - seat].map_or(false, |p| p.iter().any(|&i| s.rosters[1 - seat][i as usize].ability == cg_engine::sim::data::abilities::ILLUSION));
            for a in &s.seats.builders[seat].anomalies {
                let mut key: String = a.what.split_whitespace().filter(|w| !w.chars().any(|c| c.is_ascii_digit())).take(4).collect::<Vec<_>>().join(" ");
                if illusion && a.what.starts_with("opp ") {
                    key = format!("explained:illusion {}", key);
                }
                self.c.anomalies.add(&key, || format!("seed {} ep {} seat {}: {}", self.cfg.seed, s.episode, seat, a.what));
            }
        }
        if s.seats.keep {
            for seat in 0..SIDES {
                let r = battle::Builder::rebuild(&s.seats.records[seat]);
                self.c.rebuilt += 1;
                let b = &s.seats.builders[seat];
                if r.battle == b.battle && r.anomalies == b.anomalies {
                    self.c.rebuild_equal += 1;
                }
            }
        }
        if let Some(lg) = s.log.take() {
            if s.seats.keep && s.picks[0].is_some() && s.picks[1].is_some() {
                let ctx = format!("seed {} ep {}", self.cfg.seed, s.episode);
                let picks = [s.picks[0].unwrap(), s.picks[1].unwrap()];
                let steps = hidden::twin_check(&s.rosters, picks, s.episode_seed, &s.seats.tag, &lg, &s.seats.records, &s.game.state, s.episode % 8 == 0, self.cfg.replace_before_rows, &ctx, &mut self.c.hidden);
                self.c.hidden_games += 1;
                self.c.hidden_steps += steps;
            }
        }
        if s.seats.keep {
            s.last_records = Some(std::mem::take(&mut s.seats.records));
        }
    }

    fn reset_slot(&mut self, s: &mut Slot, g: usize) {
        s.episode_seed = mix(mix(self.cfg.seed, g as u64), s.episode);
        let mut rng = Seeded::new(s.episode_seed);
        let (ids, teams) = self.pool.draw_for(&mut rng, self.cfg.team_mode, self.agent[g] as usize);
        s.rosters = teams;
        s.team_ids = ids;
        s.in_preview = true;
        s.picks = [None; SIDES];
        s.chosen = [None; SIDES];
        s.opts = [SideOptions::default(); SIDES];
        s.state_opts = [SideOptions::default(); SIDES];
        s.steps = 0;
        s.pending = Pending { seat: 0, kind: RequestKind::Preview };
        s.refused_here.clear();
        s.decision += 1;
        s.took = [Took::Nothing; SIDES];
        s.at_turn = [None; SIDES];
        s.seats.reset(g, s.episode, self.cfg.keep_records);
        s.seats.replace_before_rows = self.cfg.replace_before_rows;
        // every fourth game is replayed with hidden information changed (checks on, records kept)
        s.log = (self.cfg.checks && self.cfg.keep_records && s.episode % 4 == 0).then(|| Vec::with_capacity(256));
        s.seats.start([&s.rosters[0], &s.rosters[1]]);
    }
}

/// Logs a call on `seats` with the record lengths after it (calls are logged after they are made).
fn log(s: &mut Slot, c: Call) {
    if let Some(l) = &mut s.log {
        l.push(c);
    }
}

/// The order a seat's choice stands for, as CG sends it.
pub fn own_order(st: &State, seat: usize, c: Choice) -> battle::Order {
    match c {
        Choice::Move { index, mega } => {
            let mv = if index == STRUGGLE { st.locked_move(seat).filter(|m| !m.is_none()).unwrap_or(cg_engine::sim::data::moves::STRUGGLE) } else { st.active(seat).moves[index as usize].id };
            battle::Order::Move { mv: mv.id().to_string(), mega }
        }
        Choice::Switch(p) => battle::Order::Switch { mon: st.sides[seat].mons[p as usize % TEAM].set_species.id().to_string() },
    }
}

#[inline]
fn deciding(o: &SideOptions) -> bool {
    matches!(o.kind, Kind::Turn | Kind::Replace)
}
