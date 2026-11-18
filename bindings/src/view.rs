//! A seat's Battle for Python Players: a read-only copy with poke-env's names, and CG's Rust Players by
//! name. Exposed (all read-only):
//! - the request: `game`, `seat`, `kind` (0 preview, 1 turn, 2 replace), `decision`, `mask` (71 bytes),
//!   `legal_actions()`; `tensors()` the observation
//! - progress: `turn`, `finished`, `won` (None until the end)
//! - `team`, `opponent_team`: lists of `Pokemon`; `active_pokemon`, `opponent_active_pokemon` (or None)
//! - `available_moves` (move ids by action 0..3), `available_switches` (species by action 8..10),
//!   `force_switch`, `can_mega_evolve`, `trapped`
//! - `weather`, `fields` (terrain and rooms), `side_conditions`, `opponent_side_conditions`: dicts of
//!   id -> turns since it began (hazards: layers)
//! - `opponent_preview`: the six species shown at team preview
//! `Pokemon`: `species`, `form`, `current_hp_fraction`, `fainted`, `status`, `boosts` (dict), `effects`
//! (volatile ids), `moves` (list of (id, PP left or None)), `item` ("" none, None unknown), `ability`
//! (None unknown), `types`, `active`, `revealed`.

use battle::{legal, Battle, ItemState, ReqKind, Side, Who, N_ACTIONS as B_ACTIONS, ROOMS, SIDE_CONDS};
use players::{by_name, Player, Rng};
use cg_engine::sim::data::BOOST_IDS;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Mutex;

#[pyclass(name = "Pokemon", module = "champions_gym._cg", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyPokemon {
    #[pyo3(get)]
    species: Option<String>,
    #[pyo3(get)]
    form: Option<String>,
    #[pyo3(get)]
    current_hp_fraction: f64,
    #[pyo3(get)]
    fainted: bool,
    #[pyo3(get)]
    status: String,
    boosts: [i8; 7],
    #[pyo3(get)]
    effects: Vec<String>,
    #[pyo3(get)]
    moves: Vec<(String, Option<u8>)>,
    #[pyo3(get)]
    item: Option<String>,
    #[pyo3(get)]
    ability: Option<String>,
    #[pyo3(get)]
    types: Vec<String>,
    #[pyo3(get)]
    active: bool,
    #[pyo3(get)]
    revealed: bool,
}

#[pymethods]
impl PyPokemon {
    #[getter]
    fn boosts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (i, k) in BOOST_IDS.iter().enumerate() {
            d.set_item(*k, self.boosts[i])?;
        }
        Ok(d)
    }
    fn __repr__(&self) -> String {
        format!("Pokemon({:?}, {:.2}, {})", self.species, self.current_hp_fraction, self.status)
    }
}

fn pokemon(b: &Battle, w: Who) -> PyPokemon {
    let m = b.mon(w);
    let active = b.active(w.side) == Some(w);
    let a = &b.side(w.side).active;
    let pp = b.pp_left(w);
    PyPokemon {
        species: m.species.or(m.preview).map(|s| s.id().to_string()),
        form: m.form.map(|s| s.id().to_string()),
        current_hp_fraction: m.hp as f64 / 100.0,
        fainted: m.fainted,
        status: m.status.id().to_string(),
        boosts: if active { a.boosts } else { [0; 7] },
        effects: if active { a.vols.iter().flatten().map(|v| v.cond.id().to_string()).collect() } else { vec![] },
        moves: (0..4).filter_map(|i| m.moves[i].map(|x| (x.id().to_string(), pp[i]))).collect(),
        item: match m.item {
            ItemState::Held(i) => Some(i.id().to_string()),
            ItemState::Gone(_) => Some(String::new()),
            ItemState::Unknown => None,
        },
        ability: m.ability.map(|x| x.id().to_string()),
        types: b.types(w).map(|t| t.iter().filter(|x| x.is_real()).map(|x| x.id().to_string()).collect()).unwrap_or_default(),
        active,
        revealed: w.side == Side::Own || m.seen,
    }
}

/// One seat's Battle at a request.
#[pyclass(name = "Battle", module = "champions_gym._cg", frozen, skip_from_py_object)]
pub struct PyBattle {
    pub b: Battle,
    #[pyo3(get)]
    pub game: usize,
    #[pyo3(get)]
    pub seat: u8,
    /// the pending decision's id (the key of its label); -1 when this seat is not the one asked
    #[pyo3(get)]
    pub decision: i64,
    #[pyo3(get)]
    pub kind: u8,
    pub mask: Vec<u8>,
}

impl PyBattle {
    fn req(&self) -> ReqKind {
        if self.kind == 2 {
            ReqKind::Replace
        } else {
            ReqKind::Turn
        }
    }
    fn timed<'py>(&self, py: Python<'py>, side: Side) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (i, id) in SIDE_CONDS.iter().enumerate() {
            if let Some(c) = self.b.side(side).conds[i] {
                let hazard = matches!(*id, "spikes" | "toxicspikes" | "stealthrock" | "stickyweb");
                d.set_item(*id, if hazard { c.layers as u16 } else { self.b.elapsed(c.start) })?;
            }
        }
        Ok(d)
    }
}

#[pymethods]
impl PyBattle {
    #[getter]
    fn mask<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new(py, &self.mask)
    }
    fn legal_actions(&self) -> Vec<usize> {
        (0..self.mask.len()).filter(|&a| self.mask[a] != 0).collect()
    }
    #[getter]
    fn teampreview(&self) -> bool {
        self.kind == 0
    }
    #[getter]
    fn turn(&self) -> u16 {
        self.b.turn
    }
    #[getter]
    fn finished(&self) -> bool {
        self.b.result.is_some()
    }
    #[getter]
    fn won(&self) -> Option<bool> {
        self.b.result.map(|r| r == battle::Outcome::Win)
    }
    #[getter]
    fn team(&self) -> Vec<PyPokemon> {
        (0..self.b.n_own).map(|i| pokemon(&self.b, Who { side: Side::Own, slot: i })).collect()
    }
    #[getter]
    fn opponent_team(&self) -> Vec<PyPokemon> {
        (0..6).map(|i| pokemon(&self.b, Who { side: Side::Opp, slot: i })).collect()
    }
    #[getter]
    fn opponent_preview(&self) -> Vec<Option<String>> {
        self.b.sides[1].team.iter().map(|m| m.preview.map(|s| s.id().to_string())).collect()
    }
    #[getter]
    fn picks(&self) -> Vec<Option<u8>> {
        self.b.picks.to_vec()
    }
    #[getter]
    fn active_pokemon(&self) -> Option<PyPokemon> {
        self.b.active(Side::Own).map(|w| pokemon(&self.b, w))
    }
    #[getter]
    fn opponent_active_pokemon(&self) -> Option<PyPokemon> {
        self.b.active(Side::Opp).map(|w| pokemon(&self.b, w))
    }
    /// Move id per move action 0..3 (None where that action is not legal).
    #[getter]
    fn available_moves(&self) -> Vec<Option<String>> {
        let l = legal(&self.b, self.req());
        (0..4).map(|i| if l.mask[i] { battle::order_for(&self.b, &l, i).and_then(|o| match o { battle::Order::Move { mv, .. } => Some(mv), _ => None }) } else { None }).collect()
    }
    /// Species per switch action 8..10 (None where not legal).
    #[getter]
    fn available_switches(&self) -> Vec<Option<String>> {
        let l = legal(&self.b, self.req());
        (8..B_ACTIONS).map(|i| if l.mask[i] { battle::order_for(&self.b, &l, i).and_then(|o| match o { battle::Order::Switch { mon } => Some(mon), _ => None }) } else { None }).collect()
    }
    #[getter]
    fn force_switch(&self) -> bool {
        self.kind == 2
    }
    #[getter]
    fn can_mega_evolve(&self) -> bool {
        legal(&self.b, self.req()).can_mega
    }
    #[getter]
    fn trapped(&self) -> bool {
        legal(&self.b, self.req()).trapped
    }
    /// {weather id: turns since it began}
    #[getter]
    fn weather<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        if let Some(w) = self.b.field.weather {
            d.set_item(w.cond.id(), self.b.elapsed(w.start))?;
        }
        Ok(d)
    }
    /// {terrain or room id: turns since it began}
    #[getter]
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        if let Some(t) = self.b.field.terrain {
            d.set_item(t.cond.id(), self.b.elapsed(t.start))?;
        }
        for (i, r) in ROOMS.iter().enumerate() {
            if let Some(t) = self.b.field.rooms[i] {
                d.set_item(*r, self.b.elapsed(t.start))?;
            }
        }
        Ok(d)
    }
    #[getter]
    fn side_conditions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.timed(py, Side::Own)
    }
    #[getter]
    fn opponent_side_conditions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.timed(py, Side::Opp)
    }
    /// Builder anomalies and unread rows so far (non-zero means the Battle may be off).
    #[getter]
    fn anomalies(&self) -> u16 {
        self.b.anomalies
    }
    /// This Battle's observation (`champions_gym.Tensors`): the arrays of `BatchEnv.observe_tensors` for one
    /// game. A preview has an all-zero `mask`; a Battle not at its own request is observed as for a turn.
    #[pyo3(signature = (damage = true))]
    fn tensors<'py>(slf: &Bound<'py, Self>, damage: bool) -> PyResult<Bound<'py, PyAny>> {
        slf.py().import("champions_gym.obs")?.getattr("battle_tensors")?.call1((slf, damage))
    }
    /// Writes the observation into seven writable C-contiguous buffers (rows f32[12,300], row_ids
    /// i32[12,10], field f32[66], last f32[17], last_ids i32[4], mask u8[11], opp_prior f32[13]).
    #[pyo3(signature = (rows, row_ids, field, last, last_ids, mask, opp_prior, damage = true))]
    #[allow(clippy::too_many_arguments)]
    fn tensors_into(
        &self,
        rows: &Bound<'_, PyAny>,
        row_ids: &Bound<'_, PyAny>,
        field: &Bound<'_, PyAny>,
        last: &Bound<'_, PyAny>,
        last_ids: &Bound<'_, PyAny>,
        mask: &Bound<'_, PyAny>,
        opp_prior: &Bound<'_, PyAny>,
        damage: bool,
    ) -> PyResult<()> {
        use crate::tensors::{disjoint, slice_mut, writable};
        let r = writable::<f32>(rows, "rows", obs::N_ROWS * obs::ROW)?;
        let i = writable::<i32>(row_ids, "row_ids", obs::N_ROWS * obs::ROW_IDS)?;
        let f = writable::<f32>(field, "field", obs::FIELD)?;
        let l = writable::<f32>(last, "last", obs::LAST)?;
        let d = writable::<i32>(last_ids, "last_ids", obs::LAST_IDS)?;
        let m = writable::<u8>(mask, "mask", obs::MASK)?;
        let q = writable::<f32>(opp_prior, "opp_prior", obs::OPP_ACTIONS)?;
        let sp = |n: &'static str, p: *mut std::ffi::c_void, len: usize| (n, p as usize, len);
        disjoint(&[
            sp("rows", r.buf_ptr(), r.len_bytes()),
            sp("row_ids", i.buf_ptr(), i.len_bytes()),
            sp("field", f.buf_ptr(), f.len_bytes()),
            sp("last", l.buf_ptr(), l.len_bytes()),
            sp("last_ids", d.buf_ptr(), d.len_bytes()),
            sp("mask", m.buf_ptr(), m.len_bytes()),
            sp("opp_prior", q.buf_ptr(), q.len_bytes()),
        ])?;
        // the buffers are held above for the whole call and were checked disjoint
        let out = unsafe {
            obs::ObsOut { rows: slice_mut(&r), row_ids: slice_mut(&i), field: slice_mut(&f), last: slice_mut(&l), last_ids: slice_mut(&d), mask: slice_mut(&m), opp_prior: slice_mut(&q) }
        };
        obs::observe_into(&self.b, self.req(), out, damage);
        if self.kind == 0 {
            unsafe { slice_mut(&m) }.fill(0);
        }
        Ok(())
    }
}

/// One of CG's Rust Players (`players::NAMES`) with its own seeded source. Any thread may call it, one call
/// at a time (the lock only makes it `Sync`).
#[pyclass(name = "RustPlayer", module = "champions_gym._cg", skip_from_py_object)]
pub struct PyRustPlayer {
    p: Mutex<Box<dyn Player>>,
    rng: Rng,
    #[pyo3(get)]
    name: String,
}

#[pymethods]
impl PyRustPlayer {
    #[new]
    #[pyo3(signature = (name, seed = 0))]
    fn new(name: &str, seed: u64) -> PyResult<Self> {
        let p = by_name(name).ok_or_else(|| super::err(format!("no built-in player {:?} ({})", name, players::NAMES.join(", "))))?;
        Ok(PyRustPlayer { p: Mutex::new(p), rng: Rng::new(seed), name: name.to_string() })
    }
    /// Three team indices, lead first.
    fn teampreview(&mut self, battle: &PyBattle) -> (u8, u8, u8) {
        let p = self.p.get_mut().unwrap_or_else(|e| e.into_inner()).teampreview(&battle.b, &mut self.rng);
        (p[0], p[1], p[2])
    }
    /// A turn or replace action 0..10 inside the Battle's legal set.
    fn choose_move(&mut self, battle: &PyBattle) -> usize {
        self.p.get_mut().unwrap_or_else(|e| e.into_inner()).choose_move(&battle.b, battle.req(), &mut self.rng)
    }
}
