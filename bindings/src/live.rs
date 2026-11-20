//! Live play for Python (`champions_gym.live`): `LiveCore` holds a `battle::live::Session` (the
//! controller's messages in, CG's answers out) and answers with a built-in Player in Rust, or hands the
//! Battle to a Python Player. Also two test helpers: a game played by CG's engine with seat 0's record and
//! its Battles at each request, and Battle equality.

use crate::view::PyBattle;
use battle::live::{AskKind, Session, Step};
use battle::{Battle, Body, Input, ReqKind, N_ACTIONS as B_ACTIONS};
use env::teams::{mix, read_file, TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind};
use env::builtin::by_name;
use players::{Player, Rng};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

const PREVIEW0: usize = env::batch::N_TURN_ACTIONS;
const N_ALL: usize = env::batch::N_TURN_ACTIONS + env::batch::N_PREVIEW_ACTIONS;

fn kind_code(k: Option<AskKind>) -> u8 {
    match k {
        Some(AskKind::Preview) => 0,
        Some(AskKind::Turn) => 1,
        Some(AskKind::Replace) => 2,
        None => 3,
    }
}

/// A read-only Battle for Players: `kind` and `mask` of the request (3 and all zero with none open).
fn py_battle(b: &Battle, open: Option<(i64, AskKind)>) -> PyBattle {
    let mut mask = vec![0u8; N_ALL];
    match open.map(|o| o.1) {
        Some(AskKind::Preview) => mask[PREVIEW0..].fill(1),
        Some(k) => {
            let l = battle::legal(b, k.req());
            for a in 0..B_ACTIONS {
                mask[a] = l.mask[a] as u8;
            }
        }
        None => {}
    }
    PyBattle { b: *b, game: 0, seat: 0, decision: open.map_or(-1, |o| o.0), kind: kind_code(open.map(|o| o.1)), mask }
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "unknown panic".into())
}

/// The live session's core. `handle` returns (step, reply, seq, kind): step 0 no answer, 1 `reply` is the
/// answer (JSON text), 2 a Player must choose for request `seq` of `kind` (0 preview, 1 turn, 2 replace),
/// then `answer`, `answer_picks` or `builtin` gives the reply.
/// Any thread may call it, one call at a time: no method releases the GIL, and the players sit behind a
/// lock only so the class is `Sync` (it is never contended).
#[pyclass(name = "LiveCore", module = "champions_gym._cg")]
pub struct PyLiveCore {
    s: Session,
    players: Mutex<HashMap<String, Box<dyn Player>>>,
    /// panics caught (the session may be off after one)
    panics: Vec<String>,
}

impl PyLiveCore {
    fn open(&self) -> Option<(i64, AskKind)> {
        self.s.open.map(|o| (o.seq, o.kind))
    }
    fn guard(&mut self, f: impl FnOnce(&mut Session) -> String) -> String {
        match catch_unwind(AssertUnwindSafe(|| f(&mut self.s))) {
            Ok(r) => r,
            Err(p) => {
                let t = panic_text(p);
                self.panics.push(t.clone());
                let mut o = battle::json::Json::obj();
                o.set("error", format!("internal error: {}", t));
                o.to_string()
            }
        }
    }
}

#[pymethods]
impl PyLiveCore {
    #[new]
    fn new() -> Self {
        PyLiveCore { s: Session::new(), players: Mutex::new(HashMap::new()), panics: vec![] }
    }

    fn handle(&mut self, text: &str) -> (u8, Option<String>, i64, u8) {
        let r = catch_unwind(AssertUnwindSafe(|| self.s.handle_text(text)));
        match r {
            Ok(Step::None) => (0, None, 0, 3),
            Ok(Step::Reply(j)) => (1, Some(j.to_string()), 0, 3),
            Ok(Step::Ask(o)) => (2, None, o.seq, kind_code(Some(o.kind))),
            Err(p) => {
                let t = panic_text(p);
                self.panics.push(t.clone());
                let mut o = battle::json::Json::obj();
                o.set("error", format!("internal error: {}", t));
                (1, Some(o.to_string()), 0, 3)
            }
        }
    }

    /// The reply to the open turn or replace request for action 0..10 (outside the legal set: the first
    /// legal one; none legal: a last-resort order, marked in `notes`).
    fn answer(&mut self, action: i64, player: &str) -> String {
        let p = player.to_string();
        self.guard(|s| s.answer(action, &p).to_string())
    }

    /// The reply to the open team preview: three indices into `own_team`, lead first.
    fn answer_picks(&mut self, picks: Vec<i64>, player: &str) -> String {
        let p = player.to_string();
        self.guard(|s| s.answer_picks(&picks, &p).to_string())
    }

    /// Answers the open request with built-in player `name`, its source seeded `mix_seed(seed, seq)`
    /// (preview: seq -1), so the same request gets the same answer.
    fn builtin(&mut self, name: &str, seed: u64) -> PyResult<String> {
        let players = self.players.get_mut().unwrap_or_else(|e| e.into_inner());
        if !players.contains_key(name) {
            let p = by_name(name).ok_or_else(|| super::err(format!("no built-in player {:?}", name)))?;
            players.insert(name.to_string(), p);
        }
        let Some((seq, kind)) = self.open() else {
            return Ok(self.answer(-1, name));
        };
        let mut rng = Rng::new(mix(seed, seq as u64));
        let b = self.s.builder.battle;
        let p = self.players.get_mut().unwrap_or_else(|e| e.into_inner()).get_mut(name).expect("player");
        let choice = catch_unwind(AssertUnwindSafe(|| match kind {
            AskKind::Preview => p.teampreview(&b, &mut rng).iter().map(|&x| x as i64).collect::<Vec<_>>(),
            k => vec![p.choose_move(&b, k.req(), &mut rng) as i64],
        }));
        Ok(match (choice, kind) {
            (Ok(c), AskKind::Preview) => self.answer_picks(c, name),
            (Ok(c), _) => self.answer(c[0], name),
            (Err(e), k) => {
                self.panics.push(format!("{}: {}", name, panic_text(e)));
                if k == AskKind::Preview {
                    self.answer_picks(vec![], name)
                } else {
                    self.answer(-1, name)
                }
            }
        })
    }

    /// The Battle with the open request's kind and mask (`kind` 3, empty mask, with none open).
    fn battle(&self) -> PyBattle {
        py_battle(&self.s.builder.battle, self.open())
    }

    /// The request the last message answered without asking the Player (replayed, `played`, given picks,
    /// or repeated from the session's record), or None: (Battle at that request with its kind and mask,
    /// action 0..10 or None, picks or None, how: "played", "repeated", "given" or "unknown").
    fn answered(&self) -> Option<(PyBattle, Option<usize>, Option<Vec<u8>>, String)> {
        self.s.answered.as_ref().map(|a| (py_battle(&a.battle, Some((a.seq, a.kind))), a.action, a.picks.clone(), a.how.to_string()))
    }

    /// A Game at the open turn or replace request with the opponent's guesses (`game_from_battle`), from
    /// the Session's Builder: (Game, rejected).
    #[pyo3(signature = (opponent, seat = 0, genders = None, seed = 0))]
    fn game(&self, opponent: &Bound<'_, pyo3::types::PyList>, seat: usize, genders: Option<Vec<Option<String>>>, seed: u64) -> PyResult<(crate::game::PyGame, Vec<(String, String, String)>)> {
        if seat > 1 {
            return Err(super::err("seat is 0 or 1"));
        }
        let Some((seq, k)) = self.open().filter(|o| o.1 != AskKind::Preview) else {
            return Err(super::err("no open turn or replace request"));
        };
        let (gs, gd, mut rej) = crate::game::guesses_of(opponent, genders)?;
        let tag = self.s.tag.clone().unwrap_or_default();
        let (game, t, r) = crate::guarded(|| env::from_battle::game_from_builder(&self.s.builder, k.req(), seq, seat, &gs, &gd, seed, &tag)).map_err(super::err)?;
        rej.extend(r);
        Ok((crate::game::PyGame::new(game, Some(t)), crate::game::rejected_list(rej)))
    }

    /// The open request's legal actions in the one action space (preview: 11..70).
    fn legal_actions(&self) -> Vec<usize> {
        match self.open() {
            Some((_, AskKind::Preview)) => (PREVIEW0..N_ALL).collect(),
            Some(_) => self.s.legal().map(|l| battle::live::actions(&l)).unwrap_or_default(),
            None => vec![],
        }
    }

    /// [(action, order JSON)] of the open turn or replace request's legal set (`battle::order_for`).
    fn legal_orders(&self) -> Vec<(usize, String)> {
        match self.open() {
            Some((_, k)) if k != AskKind::Preview => {
                let b = &self.s.builder.battle;
                let l = battle::legal(b, k.req());
                (0..B_ACTIONS).filter_map(|a| battle::order_for(b, &l, a).map(|o| (a, o.to_json().to_string()))).collect()
            }
            _ => vec![],
        }
    }

    /// (seq, kind) of the open request, or None.
    fn open_request(&self) -> Option<(i64, u8)> {
        self.open().map(|(q, k)| (q, kind_code(Some(k))))
    }

    #[getter]
    fn tag(&self) -> Option<String> {
        self.s.tag.clone()
    }
    #[getter]
    fn ended(&self) -> bool {
        self.s.ended
    }
    #[getter]
    fn restarts(&self) -> u32 {
        self.s.restarts
    }
    #[getter]
    fn messages(&self) -> usize {
        self.s.messages
    }

    /// [(source, at, what)]: "builder" (the Battle's, at = events applied), "session" (the protocol's,
    /// at = messages handled) and "internal" (caught panics).
    fn anomalies(&self) -> Vec<(String, usize, String)> {
        let mut v: Vec<(String, usize, String)> = self.s.builder.anomalies.iter().map(|a| ("builder".to_string(), a.at, a.what.clone())).collect();
        v.extend(self.s.notes.iter().map(|n| ("session".to_string(), n.at, n.what.clone())));
        v.extend(self.panics.iter().map(|p| ("internal".to_string(), 0, p.clone())));
        v
    }

    /// [(at, what)]: what the log put right (the picks, rule 32; our recorded choice, rule 31).
    fn corrections(&self) -> Vec<(usize, String)> {
        self.s.builder.corrections.iter().map(|a| (a.at, a.what.clone())).collect()
    }

    /// (anomalies, corrections): the lengths of `anomalies()` and `corrections()`, without building them.
    fn counts(&self) -> (usize, usize) {
        (self.s.builder.anomalies.len() + self.s.notes.len() + self.panics.len(), self.s.builder.corrections.len())
    }
}

/// Whether `name` is a built-in Player.
#[pyfunction(name = "live_has_player")]
fn has_player(name: &str) -> bool {
    by_name(name).is_some()
}

/// Whether two Battles are equal (everything the Builder holds).
#[pyfunction(name = "live_battle_equal")]
fn battle_equal(a: &PyBattle, b: &PyBattle) -> bool {
    a.b == b.b
}

/// The first line where two Battles' debug forms differ, for test messages ("" when equal).
#[pyfunction(name = "live_battle_diff")]
fn battle_diff(a: &PyBattle, b: &PyBattle) -> String {
    let x = format!("{:#?}", a.b);
    let y = format!("{:#?}", b.b);
    for (i, (l, r)) in x.lines().zip(y.lines()).enumerate() {
        if l != r {
            return format!("line {}: {} | {}", i, l.trim(), r.trim());
        }
    }
    String::new()
}

fn record_line(i: &Input) -> String {
    match i {
        Input::Msg(m) => m.to_json().to_string(),
        Input::Picks(p) => p.to_json().to_string(),
        Input::Order(o) => {
            let mut j = battle::json::Json::obj();
            j.set("order", o.to_json());
            j.to_string()
        }
    }
}

/// Test helper: one game of built-in players `a` (seat 0) and `b` on the teams file, played by CG's engine.
/// Returns seat 0's record (JSON lines: messages, its picks `{"picks"}` and orders `{"order"}`) and, for
/// each of seat 0's requests, (seq, the Battle the engine side built, the action taken); preview has seq -1
/// and an action 11..70.
#[pyfunction(name = "live_selfplay")]
#[pyo3(signature = (teams, seed, a = "tower", b = "max_damage", replace_before_rows = true))]
fn selfplay(py: Python<'_>, teams: &str, seed: u64, a: &str, b: &str, replace_before_rows: bool) -> PyResult<(Vec<String>, Vec<(i64, PyBattle, i64)>)> {
    let path = teams.to_string();
    let pool = py.detach(|| TeamPool::read(&read_file(&path)?)).map_err(super::err)?;
    let pool = Arc::new(pool.at_least_two().map_err(super::err)?);
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: true, auto_locked: true, replace_before_rows };
    let mut env = BatchEnv::new(pool, cfg);
    let mut ps = [by_name(a).ok_or_else(|| super::err("player a"))?, by_name(b).ok_or_else(|| super::err("player b"))?];
    let mut rng = [Rng::new(seed), Rng::new(seed ^ 1)];
    let mut legal = Vec::new();
    let mut out = vec![];
    while env.games_done < 1 {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let bt = *env.battle(0, seat);
        let act = match p.kind {
            RequestKind::Preview => env::batch::preview_action(ps[seat].teampreview(&bt, &mut rng[seat])).ok_or_else(|| super::err("picks"))?,
            k => ps[seat].choose_move(&bt, if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn }, &mut rng[seat]) as u16,
        };
        env.legal_actions(0, &mut legal);
        let act = if legal.contains(&act) { act } else { legal[0] };
        if seat == 0 {
            let (seq, kind, action) = match p.kind {
                RequestKind::Preview => (-1, AskKind::Preview, act as i64 + PREVIEW0 as i64),
                k => {
                    let seq = env.slot(0).seats.records[0]
                        .iter()
                        .rev()
                        .find_map(|i| match i {
                            Input::Msg(m) => match m.body {
                                Body::DecisionRequest { seq, .. } => Some(seq),
                                _ => None,
                            },
                            _ => None,
                        })
                        .unwrap_or(-1);
                    (seq, if k == RequestKind::Replace { AskKind::Replace } else { AskKind::Turn }, act as i64)
                }
            };
            out.push((seq, py_battle(&bt, Some((seq, kind))), action));
        }
        env.act(&[act]).map_err(super::err)?;
    }
    let rec = env.slot(0).last_records.as_ref().map(|r| r[0].iter().map(record_line).collect()).unwrap_or_default();
    Ok((rec, out))
}

/// The opposing set as a guess dict (the format of `Session.game`).
fn guess_dict<'py>(py: Python<'py>, slot: u8, m: &cg_engine::sim::MonSpec) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let d = pyo3::types::PyDict::new(py);
    d.set_item("slot", slot)?;
    d.set_item("moves", m.moves[..m.nmoves as usize].iter().map(|x| x.id()).collect::<Vec<_>>())?;
    d.set_item("item", m.item.id())?;
    d.set_item("ability", m.ability.id())?;
    d.set_item("nature", cg_engine::sim::data::NATURES[m.nature as usize].id)?;
    d.set_item("points", m.evs.to_vec())?;
    d.set_item(
        "gender",
        match m.gender {
            1 => Some("M"),
            2 => Some("F"),
            _ => None,
        },
    )?;
    Ok(d)
}

fn gender_name(g: u8) -> Option<&'static str> {
    match g {
        1 => Some("M"),
        2 => Some("F"),
        _ => None,
    }
}

/// Test helper: `live_selfplay` with the truth at each of seat 0's turn and replace requests:
/// {seq: (true Game without views, the opponent's true sets as `Session.game` guesses (brought, lead
/// first), our genders by team index, request kind 1 turn or 2 replace)}.
#[pyfunction(name = "live_selfplay_truth")]
#[pyo3(signature = (teams, seed, a = "tower", b = "max_damage", replace_before_rows = true))]
#[allow(clippy::type_complexity)]
fn selfplay_truth<'py>(py: Python<'py>, teams: &str, seed: u64, a: &str, b: &str, replace_before_rows: bool) -> PyResult<(Vec<String>, Vec<(i64, PyBattle, i64)>, Bound<'py, pyo3::types::PyDict>)> {
    let path = teams.to_string();
    let pool = py.detach(|| TeamPool::read(&read_file(&path)?)).map_err(super::err)?;
    let pool = Arc::new(pool.at_least_two().map_err(super::err)?);
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: false, keep_records: true, auto_locked: true, replace_before_rows };
    let mut env = BatchEnv::new(pool, cfg);
    let mut ps = [by_name(a).ok_or_else(|| super::err("player a"))?, by_name(b).ok_or_else(|| super::err("player b"))?];
    let mut rng = [Rng::new(seed), Rng::new(seed ^ 1)];
    let mut legal = Vec::new();
    let mut out = vec![];
    let truth = pyo3::types::PyDict::new(py);
    while env.games_done < 1 {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let bt = *env.battle(0, seat);
        let act = match p.kind {
            RequestKind::Preview => env::batch::preview_action(ps[seat].teampreview(&bt, &mut rng[seat])).ok_or_else(|| super::err("picks"))?,
            k => ps[seat].choose_move(&bt, if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn }, &mut rng[seat]) as u16,
        };
        env.legal_actions(0, &mut legal);
        let act = if legal.contains(&act) { act } else { legal[0] };
        if seat == 0 {
            let (seq, kind, action) = match p.kind {
                RequestKind::Preview => (-1, AskKind::Preview, act as i64 + PREVIEW0 as i64),
                k => (last_seq(&env.slot(0).seats.records[0]), if k == RequestKind::Replace { AskKind::Replace } else { AskKind::Turn }, act as i64),
            };
            if kind != AskKind::Preview {
                let s = env.slot(0);
                let picks = s.picks[1].expect("picks");
                let gs = picks.iter().map(|&i| guess_dict(py, i, &s.rosters[1][i as usize])).collect::<PyResult<Vec<_>>>()?;
                let gd: Vec<Option<&str>> = s.rosters[0].iter().map(|m| gender_name(m.gender)).collect();
                truth.set_item(seq, (crate::game::PyGame::new(s.game.clone(), None), gs, gd, kind_code(Some(kind))))?;
            }
            out.push((seq, py_battle(&bt, Some((seq, kind))), action));
        }
        env.act(&[act]).map_err(super::err)?;
    }
    let rec = env.slot(0).last_records.as_ref().map(|r| r[0].iter().map(record_line).collect()).unwrap_or_default();
    Ok((rec, out, truth))
}

fn last_seq(rec: &[Input]) -> i64 {
    rec.iter()
        .rev()
        .find_map(|i| match i {
            Input::Msg(m) => match m.body {
                Body::DecisionRequest { seq, .. } => Some(seq),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or(-1)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(selfplay_truth, m)?)?;
    m.add_class::<PyLiveCore>()?;
    m.add("CONTRACT_VERSION", battle::CONTRACT_VERSION)?;
    m.add_function(wrap_pyfunction!(has_player, m)?)?;
    m.add_function(wrap_pyfunction!(battle_equal, m)?)?;
    m.add_function(wrap_pyfunction!(battle_diff, m)?)?;
    m.add_function(wrap_pyfunction!(selfplay, m)?)?;
    Ok(())
}
