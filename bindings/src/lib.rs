//! `champions_gym._cg`: the batched environment and per-game trees for Python. Arrays leave as `bytes`
//! (Python wraps them with `numpy.frombuffer` or `memoryview.cast`); actions arrive through the buffer
//! protocol (int32 or int64). Every engine call is guarded: an error or a panic becomes `CGError`.
//!
//! One action space of 71 for every decision:
//! - 0..4 move slot i; 4..8 move slot i with Mega Evolution; 8..11 switch to team slot 0..2 (turn, replace).
//!   When only Struggle or a forced move is possible, action 0.
//! - 11..71 team preview: `11 + lead * 10 + pair` (see `preview_picks`).

use battle::ReqKind;
use env::batch::{choice_of, each_turn_action, preview_action, preview_picks, turn_actions, Slot, N_PREVIEW_ACTIONS, N_TURN_ACTIONS};
use env::teams::{mix, read_file, TeamMode, TeamPool};
use env::{BatchEnv, EnumOptions, EnvConfig, Game, RequestKind, Rule};
use players::{by_name, Player, Rng, NAMES};
use cg_engine::sim::state::{Winner, SIDES, TEAM};
use cg_engine::sim::{Choice, Kind, SideOptions};
use pyo3::buffer::PyBuffer;
use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

mod live;
mod tensors;
mod view;

create_exception!(_cg, CGError, PyRuntimeError, "An error from the CG engine or environment.");

const N_TURN: usize = N_TURN_ACTIONS;
const PREVIEW0: usize = N_TURN_ACTIONS;
const N_ACTIONS: usize = N_TURN_ACTIONS + N_PREVIEW_ACTIONS;

const KIND_PREVIEW: u8 = 0;
const KIND_TURN: u8 = 1;
const KIND_REPLACE: u8 = 2;
/// a seat with no decision (Game only)
const KIND_NONE: i8 = -1;

fn err(e: impl std::fmt::Display) -> PyErr {
    CGError::new_err(e.to_string())
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".into()
    }
}

/// Runs `f`, turning a panic into an error string.
fn guarded<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(p) => Err(format!("engine panic: {}", panic_text(p))),
    }
}

/// An int buffer (int32 or int64, C-contiguous) as i64 values.
fn read_ints(py: Python<'_>, obj: &Bound<'_, PyAny>, out: &mut Vec<i64>) -> PyResult<()> {
    out.clear();
    if let Ok(b) = PyBuffer::<i32>::get(obj) {
        let s = b.as_slice(py).ok_or_else(|| err("actions: buffer is not C-contiguous"))?;
        out.extend(s.iter().map(|c| c.get() as i64));
        return Ok(());
    }
    if let Ok(b) = PyBuffer::<i64>::get(obj) {
        let s = b.as_slice(py).ok_or_else(|| err("actions: buffer is not C-contiguous"))?;
        out.extend(s.iter().map(|c| c.get()));
        return Ok(());
    }
    Err(err("actions: need a C-contiguous int32 or int64 buffer"))
}

/// The action (one action space) built-in player `p` takes at game `g`'s pending decision: what
/// `BuiltinPlayer` returns for the same Battle, with the player's source seeded
/// `mix(mix(seed, g), decision)`. -2 when the player gives picks that are not three distinct indices.
fn builtin_action(p: &mut dyn Player, g: usize, s: &Slot, seed: u64) -> i32 {
    let seat = s.pending.seat as usize;
    let b = &s.seats.builders[seat].battle;
    let mut rng = Rng::new(mix(mix(seed, g as u64), s.decision));
    match s.pending.kind {
        RequestKind::Preview => preview_action(p.teampreview(b, &mut rng)).map_or(-2, |a| a as i32 + PREVIEW0 as i32),
        k => {
            let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
            let a = p.choose_move(b, rk, &mut rng);
            // outside the request's mask: its first legal action, as BuiltinPlayer does
            let (mut first, mut ok) = (None, false);
            each_turn_action(&s.opts[seat], |x| {
                first.get_or_insert(x);
                ok |= x as usize == a;
            });
            if ok {
                a as i32
            } else {
                first.map_or(-2, |x| x as i32)
            }
        }
    }
}

/// `builtin_action` for every game on the environment's workers; with `skip_agent`, games waiting on
/// their agent seat get -1.
fn builtin_fill(env: &BatchEnv<Rule>, name: &str, seed: u64, skip_agent: bool, out: &mut [i32]) -> Result<(), String> {
    if by_name(name).is_none() {
        return Err(format!("no built-in player {:?} ({})", name, NAMES.join(", ")));
    }
    let agent = env.agent_seats();
    env.map_games(out, || by_name(name).expect("player"), |p, g, s| {
        if skip_agent && s.pending.seat == agent[g] {
            -1
        } else {
            builtin_action(p.as_mut(), g, s, seed)
        }
    });
    match out.iter().position(|&a| a == -2) {
        Some(g) => Err(format!("{}: no legal action at game {}", name, g)),
        None => Ok(()),
    }
}

fn kind_code(k: RequestKind) -> u8 {
    match k {
        RequestKind::Preview => KIND_PREVIEW,
        RequestKind::Turn => KIND_TURN,
        RequestKind::Replace => KIND_REPLACE,
    }
}

fn team_mode(s: &str) -> PyResult<TeamMode> {
    match s {
        "stage3" => Ok(TeamMode::Stage3),
        "real" | "as_given" => Ok(TeamMode::AsGiven),
        _ => Err(err(format!("team_mode: {:?} (real or stage3)", s))),
    }
}

/// The batched environment: N games, strict turn-taking, one pending decision per game.
#[pyclass(name = "BatchEnv", module = "champions_gym._cg")]
struct PyBatchEnv {
    env: BatchEnv<Rule>,
    acts: Vec<u16>,
    ints: Vec<i64>,
    /// the opponent's actions in `act(..., opponent=...)`
    opp: Vec<i32>,
    /// set after a panic: the environment's state can no longer be trusted
    broken: Option<String>,
}

impl PyBatchEnv {
    fn check(&self) -> PyResult<()> {
        match &self.broken {
            Some(e) => Err(err(format!("environment unusable after an earlier engine panic: {}", e))),
            None => Ok(()),
        }
    }

    fn game_index(&self, g: i64) -> PyResult<usize> {
        if g < 0 || g as usize >= self.env.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!("game {} of {}", g, self.env.len())));
        }
        Ok(g as usize)
    }

    fn game_of(&self, g: i64) -> PyResult<Game> {
        self.check()?;
        let g = self.game_index(g)?;
        let s = self.env.slot(g);
        if s.in_preview {
            return Err(err(format!("game {} is in team preview: no game yet", g)));
        }
        Ok(s.game.clone())
    }

    fn fill_masks(&self, out: &mut [u8]) {
        for g in 0..self.env.len() {
            self.env.action_mask(g, &mut out[g * N_ACTIONS..(g + 1) * N_ACTIONS]);
        }
    }
}

#[pymethods]
impl PyBatchEnv {
    /// `teams`: a teams.json path. `team_mode`: "real" (the sets as written; "as_given" is the same) or "stage3" (No Ability, no item, stage-3 moves).
    /// `rule`: "battle" (what each seat may choose comes from its Battle; a seat locked into a move is not
    /// asked) or "state" (the State's options, every deciding seat asked, for comparison).
    /// `threads`: worker threads `act` and `observe_into` step the games on (results do not depend on it).
    /// `strict_teams`: a team that cannot be read raises ValueError instead of being dropped.
    /// `team_weights`: None (uniform), [w] for both seats or [w_agent, w_other], one weight per team in the
    /// file. `agent_seat`: one 0/1 per game, the seat that draws first, from the first weights (default 0).
    /// `replace_before_rows`: the live order, a replace request before the rows of its turn (False: after).
    #[new]
    #[pyo3(signature = (teams, games, seed = 0, team_mode = "real", max_steps = 1000, rule = "battle", threads = 1, strict_teams = false, team_weights = None, agent_seat = None, replace_before_rows = true))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        teams: &str,
        games: usize,
        seed: u64,
        team_mode: &str,
        max_steps: u32,
        rule: &str,
        threads: usize,
        strict_teams: bool,
        team_weights: Option<Vec<Vec<f64>>>,
        agent_seat: Option<Vec<u8>>,
        replace_before_rows: bool,
    ) -> PyResult<Self> {
        if games == 0 {
            return Err(err("games must be at least 1"));
        }
        let agent = agent_seat.unwrap_or_else(|| vec![0; games]);
        if agent.len() != games {
            return Err(PyValueError::new_err(format!("agent_seat: {} seats for {} games", agent.len(), games)));
        }
        if let Some(g) = agent.iter().position(|&x| x > 1) {
            return Err(PyValueError::new_err(format!("agent_seat: game {} has seat {} (0 or 1)", g, agent[g])));
        }
        let mode = self::team_mode(team_mode)?;
        let rule = match rule {
            "battle" => Rule::Battle,
            "state" => Rule::State,
            r => return Err(err(format!("rule: {:?} (battle or state)", r))),
        };
        let path = teams.to_string();
        let pool = py.detach(|| guarded(|| TeamPool::read(&read_file(&path)?))).map_err(err)?;
        if strict_teams && !pool.errors.is_empty() {
            return Err(PyValueError::new_err(format!("{}: {}", path, pool.dropped_message())));
        }
        let mut pool = pool.at_least_two().map_err(err)?;
        match team_weights.as_deref() {
            None => {}
            Some([w]) => pool.set_weights(w, w, true).map_err(PyValueError::new_err)?,
            Some([w0, w1]) => pool.set_weights(w0, w1, false).map_err(PyValueError::new_err)?,
            Some(v) => return Err(PyValueError::new_err(format!("team_weights: {} sequences (one, or two: agent seat, other seat)", v.len()))),
        }
        let pool = Arc::new(pool);
        let env = py
            .detach(move || {
                guarded(|| {
                    let cfg = EnvConfig { games, seed, team_mode: mode, max_steps, auto_locked: rule == Rule::Battle, replace_before_rows, ..EnvConfig::default() };
                    let mut env = BatchEnv::with_agent(pool, cfg, rule, agent);
                    env.set_threads(threads);
                    Ok(env)
                })
            })
            .map_err(err)?;
        Ok(PyBatchEnv { env, acts: vec![0; games], ints: Vec::with_capacity(games), opp: vec![0; games], broken: None })
    }

    /// Teams in the file that were not accepted: [(index in the file, reason)].
    #[getter]
    fn team_errors(&self) -> Vec<(usize, String)> {
        self.env.pool().errors.clone()
    }

    /// Teams accepted (the pool indices of `team_ids` are 0..team_count).
    #[getter]
    fn team_count(&self) -> usize {
        self.env.pool().teams.len()
    }

    /// Teams in the file, accepted or not.
    #[getter]
    fn team_file_count(&self) -> usize {
        self.env.pool().file_count
    }

    /// For each pool index, the team's index in the file.
    #[getter]
    fn team_file_index(&self) -> Vec<usize> {
        self.env.pool().file_index.clone()
    }

    /// Each game's agent seat (uint8[N]).
    fn agent_seat<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.env.agent_seats())
    }

    /// Writes into `out` (a writable C-contiguous int32 or int64 buffer of length N) the action, in the one
    /// action space, that built-in player `name` takes at every game's pending decision. The player's
    /// source for game g's decision d is seeded `mix_seed(mix_seed(seed, g), d)` (seed None: the
    /// environment's seed). Runs without the GIL on the environment's workers.
    #[pyo3(signature = (name, out, seed = None))]
    fn builtin_into(&self, py: Python<'_>, name: &str, out: &Bound<'_, PyAny>, seed: Option<u64>) -> PyResult<()> {
        self.check()?;
        let n = self.env.len();
        let seed = seed.unwrap_or(self.env.config().seed);
        let env = &self.env;
        let e32 = match tensors::writable::<i32>(out, "out", n) {
            Ok(b) => {
                // held by `b` for the whole call
                let o = unsafe { tensors::slice_mut(&b) };
                return py.detach(|| guarded(|| builtin_fill(env, name, seed, false, o))).map_err(err);
            }
            Err(e) => e,
        };
        let Ok(b) = tensors::writable::<i64>(out, "out", n) else { return Err(e32) };
        let mut tmp = vec![0i32; n];
        py.detach(|| guarded(|| builtin_fill(env, name, seed, false, &mut tmp))).map_err(err)?;
        let o = unsafe { tensors::slice_mut(&b) };
        for (x, y) in o.iter_mut().zip(&tmp) {
            *x = *y as i64;
        }
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.env.len()
    }

    #[getter]
    fn games(&self) -> usize {
        self.env.len()
    }

    /// worker threads used by `act` and `observe_into`
    #[getter]
    fn threads(&self) -> usize {
        self.env.threads()
    }

    #[setter]
    fn set_threads(&mut self, threads: usize) {
        self.env.set_threads(threads);
    }

    /// decisions taken since creation
    #[getter]
    fn decisions(&self) -> u64 {
        self.env.decisions
    }

    /// games finished since creation
    #[getter]
    fn games_done(&self) -> u64 {
        self.env.games_done
    }

    /// (seat uint8[N], kind uint8[N]); kind 0 preview, 1 turn, 2 replace.
    fn pending<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
        self.check()?;
        let n = self.env.len();
        let seat = PyBytes::new_with(py, n, |b| {
            for (g, x) in b.iter_mut().enumerate() {
                *x = self.env.pending(g).seat;
            }
            Ok(())
        })?;
        let kind = PyBytes::new_with(py, n, |b| {
            for (g, x) in b.iter_mut().enumerate() {
                *x = kind_code(self.env.pending(g).kind);
            }
            Ok(())
        })?;
        Ok((seat, kind))
    }

    /// bool[N, 71] as bytes (0 or 1), over the one action space.
    fn masks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.check()?;
        PyBytes::new_with(py, self.env.len() * N_ACTIONS, |b| {
            self.fill_masks(b);
            Ok(())
        })
    }

    /// pending() and masks() in one call.
    #[allow(clippy::type_complexity)]
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
        let (s, k) = self.pending(py)?;
        Ok((s, k, self.masks(py)?))
    }

    /// Every game's observation for its pending seat, written into eleven writable C-contiguous buffers:
    /// rows f32[N,12,300], row_ids i32[N,12,10], field f32[N,66], last f32[N,17], last_ids i32[N,4],
    /// mask u8[N,11], action_mask u8[N,71], seat u8[N], kind u8[N], decision i64[N], opp_prior f32[N,13].
    /// Runs without the GIL on the environment's workers (`threads=None`), or on that many threads;
    /// `damage=False` leaves the damage features at 0.
    #[pyo3(signature = (rows, row_ids, field, last, last_ids, mask, action_mask, seat, kind, decision, opp_prior, threads = None, damage = true))]
    #[allow(clippy::too_many_arguments)]
    fn observe_into(
        &self,
        py: Python<'_>,
        rows: &Bound<'_, PyAny>,
        row_ids: &Bound<'_, PyAny>,
        field: &Bound<'_, PyAny>,
        last: &Bound<'_, PyAny>,
        last_ids: &Bound<'_, PyAny>,
        mask: &Bound<'_, PyAny>,
        action_mask: &Bound<'_, PyAny>,
        seat: &Bound<'_, PyAny>,
        kind: &Bound<'_, PyAny>,
        decision: &Bound<'_, PyAny>,
        opp_prior: &Bound<'_, PyAny>,
        threads: Option<usize>,
        damage: bool,
    ) -> PyResult<()> {
        let threads = threads.unwrap_or_else(|| self.env.threads());
        self.check()?;
        let bufs = tensors::BatchBufs::get(self.env.len(), [rows, row_ids, field, last, last_ids, mask, action_mask, seat, kind, decision, opp_prior])?;
        // the buffers stay held by `bufs` for the whole call and were checked disjoint
        let out = unsafe { bufs.out() };
        let env = &self.env;
        let opt = tensors::options(threads, damage);
        py.detach(move || guarded(|| env.observe_batch_with(out, opt))).map_err(err)
    }

    /// One action per game (int32 or int64 buffer of length N). Every action is checked before any game
    /// moves; stepping runs without the GIL. With `opponent` (a built-in player's name), a game waiting on
    /// the seat that is not its agent seat takes that player's action (seeded as in `builtin_into`, seed
    /// `opponent_seed` or the environment's) and its entry in `actions` is ignored.
    #[pyo3(signature = (actions, opponent = None, opponent_seed = None))]
    fn act(&mut self, py: Python<'_>, actions: &Bound<'_, PyAny>, opponent: Option<&str>, opponent_seed: Option<u64>) -> PyResult<()> {
        self.check()?;
        let mut ints = std::mem::take(&mut self.ints);
        read_ints(py, actions, &mut ints)?;
        let n = self.env.len();
        if ints.len() != n {
            let m = format!("act: {} actions for {} games", ints.len(), n);
            self.ints = ints;
            return Err(err(m));
        }
        if let Some(name) = opponent {
            let seed = opponent_seed.unwrap_or(self.env.config().seed);
            let (env, opp) = (&self.env, &mut self.opp);
            if let Err(e) = py.detach(|| guarded(|| builtin_fill(env, name, seed, true, opp))) {
                self.ints = ints;
                return Err(err(e));
            }
            for (x, &o) in ints.iter_mut().zip(&self.opp) {
                if o >= 0 {
                    *x = o as i64;
                }
            }
        }
        for (g, &a) in ints.iter().enumerate() {
            let p = self.env.pending(g);
            let ok = match p.kind {
                RequestKind::Preview => (PREVIEW0 as i64..N_ACTIONS as i64).contains(&a),
                _ => (0..N_TURN as i64).contains(&a),
            };
            if !ok {
                self.ints = ints;
                return Err(err(format!("act: game {} seat {}: action {} is outside the {:?} actions", g, p.seat, a, p.kind)));
            }
            self.acts[g] = if p.kind == RequestKind::Preview { (a as usize - PREVIEW0) as u16 } else { a as u16 };
        }
        self.ints = ints;
        let env = &mut self.env;
        let acts = &self.acts;
        let r = py.detach(|| match catch_unwind(AssertUnwindSafe(|| env.act(acts))) {
            Ok(r) => r.map_err(|e| (e, false)),
            Err(p) => Err((format!("engine panic: {}", panic_text(p)), true)),
        });
        match r {
            Ok(()) => Ok(()),
            Err((e, panicked)) => {
                if panicked {
                    self.broken = Some(e.clone());
                }
                Err(err(e))
            }
        }
    }

    /// Games that ended in the last act: (done uint8[N], winner int8[N] (-1 tie or truncated),
    /// turns uint16[N], truncated uint8[N], episode uint64[N]), little-endian bytes.
    #[allow(clippy::type_complexity)]
    fn results<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
        self.check()?;
        let r = self.env.results();
        let n = r.len();
        let done = PyBytes::new_with(py, n, |b| {
            for (x, r) in b.iter_mut().zip(r) {
                *x = r.done as u8;
            }
            Ok(())
        })?;
        let winner = PyBytes::new_with(py, n, |b| {
            for (x, r) in b.iter_mut().zip(r) {
                *x = r.winner.map_or(-1i8, |w| w as i8) as u8;
            }
            Ok(())
        })?;
        let turns = PyBytes::new_with(py, n * 2, |b| {
            for (x, r) in b.chunks_exact_mut(2).zip(r) {
                x.copy_from_slice(&r.turns.to_le_bytes());
            }
            Ok(())
        })?;
        let trunc = PyBytes::new_with(py, n, |b| {
            for (x, r) in b.iter_mut().zip(r) {
                *x = r.truncated as u8;
            }
            Ok(())
        })?;
        let episode = PyBytes::new_with(py, n * 8, |b| {
            for (x, r) in b.chunks_exact_mut(8).zip(r) {
                x.copy_from_slice(&r.episode.to_le_bytes());
            }
            Ok(())
        })?;
        Ok((done, winner, turns, trunc, episode))
    }

    /// Labels of the decisions the last act completed, per game and seat, little-endian bytes:
    /// (valid u8[N,2], decision i64[N,2], refused u8[N,2], own_action i16[N,2], opp_action i8[N,2],
    /// opp_forced u8[N,2], opp_move i32[N,2], opp_shown u8[N,2], opp_prior f32[N,2,13], opp_picks i8[N,2,3],
    /// opp_true_brought u8[N,2,6], opp_true_moves i32[N,2,6,4], opp_true_item i32[N,2,6],
    /// opp_true_ability i32[N,2,6]).
    fn labels<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyBytes>>> {
        self.check()?;
        let l = self.env.labels();
        let all: Vec<&env::Label> = l.iter().flat_map(|x| x.iter()).collect();
        let col = |size: usize, f: &dyn Fn(&env::Label, &mut [u8])| {
            PyBytes::new_with(py, all.len() * size, |b| {
                for (x, l) in b.chunks_exact_mut(size).zip(&all) {
                    f(l, x);
                }
                Ok(())
            })
        };
        Ok(vec![
            col(1, &|l, x| x[0] = l.valid as u8)?,
            col(8, &|l, x| x.copy_from_slice(&(l.decision as i64).to_le_bytes()))?,
            col(1, &|l, x| x[0] = l.refused as u8)?,
            col(2, &|l, x| x.copy_from_slice(&l.own_action.to_le_bytes()))?,
            col(1, &|l, x| x[0] = l.opp_action as u8)?,
            col(1, &|l, x| x[0] = l.opp_forced)?,
            col(4, &|l, x| x.copy_from_slice(&l.opp_move.to_le_bytes()))?,
            col(1, &|l, x| x[0] = l.opp_shown)?,
            col(52, &|l, x| {
                for (c, v) in x.chunks_exact_mut(4).zip(l.opp_prior) {
                    c.copy_from_slice(&v.to_le_bytes());
                }
            })?,
            col(3, &|l, x| {
                for i in 0..3 {
                    x[i] = l.opp_picks[i] as u8;
                }
            })?,
            col(6, &|l, x| x.copy_from_slice(&l.opp_true_brought))?,
            col(96, &|l, x| {
                for (c, v) in x.chunks_exact_mut(4).zip(l.opp_true_moves.iter().flatten()) {
                    c.copy_from_slice(&v.to_le_bytes());
                }
            })?,
            col(24, &|l, x| {
                for (c, v) in x.chunks_exact_mut(4).zip(l.opp_true_item) {
                    c.copy_from_slice(&v.to_le_bytes());
                }
            })?,
            col(24, &|l, x| {
                for (c, v) in x.chunks_exact_mut(4).zip(l.opp_true_ability) {
                    c.copy_from_slice(&v.to_le_bytes());
                }
            })?,
        ])
    }

    /// Pool indices of each game's two teams: uint32[N, 2] as bytes.
    fn team_ids<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.check()?;
        PyBytes::new_with(py, self.env.len() * 8, |b| {
            for (g, x) in b.chunks_exact_mut(8).enumerate() {
                let t = self.env.slot(g).team_ids;
                x[..4].copy_from_slice(&(t[0] as u32).to_le_bytes());
                x[4..].copy_from_slice(&(t[1] as u32).to_le_bytes());
            }
            Ok(())
        })
    }

    /// The legal actions of game `g`'s pending decision, in the one action space.
    fn legal_actions(&self, g: i64) -> PyResult<Vec<usize>> {
        self.check()?;
        let g = self.game_index(g)?;
        let mut v = Vec::new();
        self.env.legal_actions(g, &mut v);
        let off = if self.env.pending(g).kind == RequestKind::Preview { PREVIEW0 } else { 0 };
        Ok(v.into_iter().map(|a| a as usize + off).collect())
    }

    /// What the pending seat of game `g` knows (or `seat`'s): its Battle, with the request and its mask.
    #[pyo3(signature = (g, seat = None))]
    fn battle(&self, g: i64, seat: Option<u8>) -> PyResult<view::PyBattle> {
        self.check()?;
        let gi = self.game_index(g)?;
        let p = self.env.pending(gi);
        let seat = seat.unwrap_or(p.seat);
        if seat > 1 {
            return Err(err("seat is 0 or 1"));
        }
        let mut mask = vec![0u8; N_ACTIONS];
        if seat == p.seat {
            let mut legal = Vec::with_capacity(16);
            self.env.legal_actions(gi, &mut legal);
            let off = if p.kind == RequestKind::Preview { PREVIEW0 } else { 0 };
            for a in legal {
                mask[off + a as usize] = 1;
            }
        }
        Ok(view::PyBattle { b: *self.env.battle(gi, seat as usize), game: gi, seat, decision: if seat == p.seat { self.env.slot(gi).decision as i64 } else { -1 }, kind: if seat == p.seat { kind_code(p.kind) } else { 3 }, mask })
    }

    /// Counts since creation: refusals (orders the State refused that the Battle allowed), Battles that
    /// allowed nothing, and seats not asked because they were locked into a move.
    fn counters(&self) -> (u64, u64, u64) {
        let c = &self.env.counters;
        (c.refusals, c.battle_empty + c.refusal_fallbacks, c.auto)
    }

    /// Replace requests since creation: (asked, asked before rows the seat had not been given, events
    /// held back at those).
    fn replace_counters(&self) -> (u64, u64, u64) {
        let c = &self.env.counters;
        (c.replace_asked, c.stale_replace, c.stale_events)
    }

    /// Whether a replace request comes before the rows of its turn (the live order).
    #[getter]
    fn replace_before_rows(&self) -> bool {
        self.env.config().replace_before_rows
    }

    /// An exact copy of game `g` (same seed state: the same continuation). Not during team preview.
    fn snapshot(&self, g: i64) -> PyResult<PyGame> {
        Ok(PyGame { game: self.game_of(g)? })
    }

    /// A copy of game `g` with its own seed: same State, independent continuation.
    fn fork(&self, g: i64, seed: u64) -> PyResult<PyGame> {
        Ok(PyGame { game: self.game_of(g)?.fork(seed) })
    }
}

/// One game outside the batch, for trees: both seats choose together, as the engine resolves a step.
#[pyclass(name = "Game", module = "champions_gym._cg", skip_from_py_object)]
#[derive(Clone)]
struct PyGame {
    game: Game,
}

impl PyGame {
    fn options(&self) -> [SideOptions; SIDES] {
        self.game.state.options()
    }

    fn choices(&self, a: [Option<i64>; SIDES]) -> Result<[Option<Choice>; SIDES], String> {
        if self.game.state.ended {
            return Err("the game has ended".into());
        }
        let o = self.options();
        let mut ch = [None; SIDES];
        for s in 0..SIDES {
            let deciding = matches!(o[s].kind, Kind::Turn | Kind::Replace);
            match (deciding, a[s]) {
                (true, Some(x)) => {
                    let c = if (0..N_TURN as i64).contains(&x) { choice_of(&o[s], x as u16) } else { None };
                    ch[s] = Some(c.ok_or_else(|| format!("seat {}: action {} not legal for {:?}", s, x, o[s].kind))?);
                }
                (true, None) => return Err(format!("seat {} must choose", s)),
                (false, Some(x)) => return Err(format!("seat {} has no decision (got action {})", s, x)),
                (false, None) => {}
            }
        }
        Ok(ch)
    }
}

#[pymethods]
impl PyGame {
    #[getter]
    fn turn(&self) -> u16 {
        self.game.state.turn
    }

    #[getter]
    fn ended(&self) -> bool {
        self.game.state.ended
    }

    /// 0 or 1 once ended; None while playing or for a tie
    #[getter]
    fn winner(&self) -> Option<u8> {
        match self.game.state.winner {
            Winner::Side(x) if self.game.state.ended => Some(x),
            _ => None,
        }
    }

    /// Each seat's decision kind: 1 turn, 2 replace, -1 none.
    fn kinds(&self) -> (i8, i8) {
        let o = self.options();
        let k = |o: &SideOptions| match o.kind {
            Kind::Turn => KIND_TURN as i8,
            Kind::Replace => KIND_REPLACE as i8,
            _ => KIND_NONE,
        };
        (k(&o[0]), k(&o[1]))
    }

    /// bool[2, 11] as bytes: each seat's legal turn/replace actions.
    fn masks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let o = self.options();
        PyBytes::new_with(py, SIDES * N_TURN, |b| {
            let mut v = Vec::with_capacity(16);
            for s in 0..SIDES {
                v.clear();
                turn_actions(&o[s], &mut v);
                for &a in &v {
                    b[s * N_TURN + a as usize] = 1;
                }
            }
            Ok(())
        })
    }

    /// An exact copy (same seed state).
    fn snapshot(&self) -> PyGame {
        PyGame { game: self.game.snapshot() }
    }

    /// A copy with its own seed.
    fn fork(&self, seed: u64) -> PyGame {
        PyGame { game: self.game.fork(seed) }
    }

    /// One step with the game's seeded source. `a0`, `a1`: each seat's action, None for a seat with no
    /// decision.
    #[pyo3(signature = (a0, a1))]
    fn step(&mut self, py: Python<'_>, a0: Option<i64>, a1: Option<i64>) -> PyResult<()> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        let g = &mut self.game;
        py.detach(|| guarded(|| g.step(ch))).map_err(err)
    }

    /// One step with every random event forced from `path` (an outcome's path from `outcomes`).
    #[pyo3(signature = (a0, a1, path))]
    fn step_forced(&mut self, py: Python<'_>, a0: Option<i64>, a1: Option<i64>, path: Vec<u16>) -> PyResult<()> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        let g = &mut self.game;
        py.detach(|| guarded(|| g.step_forced(ch, &path))).map_err(err)
    }

    /// Every outcome of the next step with these actions: (outcomes, covered, capped, leaves), where each
    /// outcome is (probability, path, Game). Each Game keeps this game's seed state. `merge` joins outcomes
    /// with equal States; `max_branches` caps the replays (then `capped` is true and `covered` < 1).
    #[pyo3(signature = (a0, a1, merge = true, max_branches = 100_000))]
    #[allow(clippy::type_complexity)]
    fn outcomes(
        &self,
        py: Python<'_>,
        a0: Option<i64>,
        a1: Option<i64>,
        merge: bool,
        max_branches: usize,
    ) -> PyResult<(Vec<(f64, Vec<u16>, PyGame)>, f64, bool, usize)> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        let opts = EnumOptions { merge, max_branches: max_branches.max(1), keep_events: false };
        let g = &self.game;
        let e = py.detach(|| guarded(|| g.outcomes(ch, &opts))).map_err(err)?;
        let rng = self.game.rng.clone();
        let list = e.outcomes.into_iter().map(|o| (o.p, o.path, PyGame { game: Game { state: o.state, rng: rng.clone() } })).collect();
        Ok((list, e.covered, e.capped, e.leaves))
    }

    /// Whether the two games' States are equal (the seed state is not compared).
    fn same_state(&self, other: &PyGame) -> bool {
        self.game.state == other.game.state
    }
}

/// Picks (lead first, as roster indices 0..5) of a preview action in the one action space (11..71).
#[pyfunction(name = "preview_picks")]
fn py_preview_picks(action: i64) -> PyResult<(u8, u8, u8)> {
    if !(PREVIEW0 as i64..N_ACTIONS as i64).contains(&action) {
        return Err(err(format!("preview action {} outside 11..71", action)));
    }
    let p = preview_picks((action as usize - PREVIEW0) as u16).ok_or_else(|| err("bad preview action"))?;
    Ok((p[0], p[1], p[2]))
}

/// The preview action (11..71) for picks (lead first); the order of the two back picks does not matter.
#[pyfunction(name = "preview_action")]
fn py_preview_action(picks: (u8, u8, u8)) -> PyResult<usize> {
    let p: [u8; TEAM] = [picks.0, picks.1, picks.2];
    if p.iter().any(|&x| x >= 6) || p[0] == p[1] || p[0] == p[2] || p[1] == p[2] {
        return Err(err(format!("picks {:?}: need three distinct roster indices 0..5", p)));
    }
    preview_action(p).map(|a| a as usize + PREVIEW0).ok_or_else(|| err("bad picks"))
}

/// Reads a teams file (a path, or the JSON text itself when it starts with `[`) without building an
/// environment: (teams accepted, [(index in the file, reason) for each team not accepted]).
#[pyfunction(name = "check_teams")]
fn py_check_teams(py: Python<'_>, path_or_text: &str) -> PyResult<(usize, Vec<(usize, String)>)> {
    let src = path_or_text.to_string();
    let p = py
        .detach(move || {
            let text = if src.trim_start().starts_with('[') { src } else { read_file(&src)? };
            TeamPool::read(&text)
        })
        .map_err(err)?;
    Ok((p.teams.len(), p.errors))
}

/// A derived seed (SplitMix64), the same one the environment uses.
#[pyfunction(name = "mix_seed")]
fn py_mix(a: u64, b: u64) -> u64 {
    mix(a, b)
}

#[pymodule]
fn _cg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBatchEnv>()?;
    m.add_class::<PyGame>()?;
    m.add_class::<view::PyBattle>()?;
    m.add_class::<view::PyPokemon>()?;
    m.add_class::<view::PyRustPlayer>()?;
    m.add("BUILTIN_PLAYERS", NAMES.to_vec())?;
    m.add_function(wrap_pyfunction!(py_preview_picks, m)?)?;
    m.add_function(wrap_pyfunction!(py_preview_action, m)?)?;
    m.add_function(wrap_pyfunction!(py_mix, m)?)?;
    m.add_function(wrap_pyfunction!(py_check_teams, m)?)?;
    m.add_function(wrap_pyfunction!(tensors::features, m)?)?;
    m.add_function(wrap_pyfunction!(tensors::layout_sizes, m)?)?;
    m.add_function(wrap_pyfunction!(tensors::learnset_into, m)?)?;
    live::register(m)?;
    m.add("CGError", m.py().get_type::<CGError>())?;
    m.add("N_ACTIONS", N_ACTIONS)?;
    m.add("N_TURN_ACTIONS", N_TURN)?;
    m.add("PREVIEW0", PREVIEW0)?;
    m.add("KIND_PREVIEW", KIND_PREVIEW)?;
    m.add("KIND_TURN", KIND_TURN)?;
    m.add("KIND_REPLACE", KIND_REPLACE)?;
    Ok(())
}
