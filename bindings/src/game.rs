//! `Game`: one game outside the batch, for trees. Both seats choose together and the engine resolves the
//! step. A Game taken from the environment with views carries both seats' Battles (`env::views`), kept
//! lazily: the rows of the steps taken are emitted when a view is asked for.

use crate::tensors::{self, BatchBufs};
use crate::view::PyBattle;
use crate::{err, guarded, KIND_NONE, KIND_REPLACE, KIND_TURN, N_ACTIONS, N_TURN};
use env::views::KIND_NONE as VIEW_NONE;
use env::batch::{choice_of, turn_actions};
use env::observe::{ObsBatch, ROWS_LEN, ROW_IDS_LEN};
use env::views::{Source, Tracked, Views};
use env::{sample_step, EnumOptions, Enumeration, Game, SlotSave};
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::{Winner, SIDES, TEAM};
use cg_engine::sim::{Choice, Kind, SideOptions};
use obs::{FIELD, LAST, LAST_IDS, MASK, OPP_ACTIONS};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use battle::ReqKind;
use cg_engine::sim::data::{nature_from_id, to_id, AbilityId, ItemId, MoveId};
use env::from_battle::{OppGuess, Rejected};
use std::sync::Arc;

#[pyclass(name = "Game", module = "champions_gym._cg", skip_from_py_object)]
#[derive(Clone)]
pub struct PyGame {
    pub game: Game,
    pub views: Option<Tracked>,
    /// the slot it was taken from, for `BatchEnv.restore`; dropped once the Game is stepped
    pub save: Option<Arc<SlotSave>>,
}

/// An outcome list for Python: (probability or weight, path, leaves, Game).
type OutcomeList = Vec<(f64, Vec<u16>, u32, PyGame)>;

impl PyGame {
    pub fn new(game: Game, views: Option<Tracked>) -> PyGame {
        PyGame { game, views, save: None }
    }

    /// A new seeded source; the State and the views stay.
    pub fn reseed(&mut self, seed: u64) {
        self.game.rng = Seeded::new(seed);
    }

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

    /// The views now (built from the recorded steps on first use).
    fn views_now(&mut self) -> PyResult<&Views> {
        let st = self.game.state;
        match &mut self.views {
            None => Err(err("this Game has no views (take it with snapshot/fork(..., views=True), or from game_from_battle)")),
            Some(t) => t.views(&st).map_err(err),
        }
    }

    /// The views now, for a seat whose Battle is known.
    fn views_of(&mut self, seat: usize) -> PyResult<&Views> {
        let v = self.views_now()?;
        if !v.known[seat] {
            return Err(err(format!("seat {}'s Battle is not known: this Game was built from the other seat's live Battle", seat)));
        }
        Ok(v)
    }

    fn seat_of(seat: i64) -> PyResult<usize> {
        if !(0..SIDES as i64).contains(&seat) {
            return Err(err(format!("seat {} (0 or 1)", seat)));
        }
        Ok(seat as usize)
    }

    /// Children of an enumeration or a sample: each keeps this game's seed state and, with views, records
    /// its forced path.
    fn children(&self, e: Enumeration, ch: [Option<Choice>; SIDES]) -> OutcomeList {
        let rng = self.game.rng.clone();
        e.outcomes
            .into_iter()
            .map(|o| {
                let views = self.views.clone().map(|mut t| {
                    t.record(ch, Source::Path(Arc::new(o.path.clone())));
                    t
                });
                (o.p, o.path, o.leaves, PyGame { game: Game { state: o.state, rng: rng.clone() }, views, save: None })
            })
            .collect()
    }
}

/// Writes one Game's view of `seat` into entry `i` of `out`.
fn observe(v: &Views, seat: usize, out: &mut ObsBatch<'_>, i: usize, damage: bool) {
    v.observe(seat, out, i, damage)
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

    /// Whether this Game carries the seats' views.
    #[getter]
    fn has_views(&self) -> bool {
        self.views.is_some()
    }

    /// Whether `BatchEnv.restore` takes this Game (an unstepped `BatchEnv.snapshot` or a copy of one).
    #[getter]
    fn restorable(&self) -> bool {
        self.save.is_some()
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
        self.clone()
    }

    /// A copy with its own seed.
    fn fork(&self, seed: u64) -> PyGame {
        let mut g = self.clone();
        g.reseed(seed);
        g
    }

    /// One step with the game's seeded source. `a0`, `a1`: each seat's action, None for a seat with no
    /// decision.
    #[pyo3(signature = (a0, a1))]
    fn step(&mut self, py: Python<'_>, a0: Option<i64>, a1: Option<i64>) -> PyResult<()> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        let before = self.views.as_ref().map(|_| self.game.rng.clone());
        let g = &mut self.game;
        py.detach(|| guarded(|| g.step(ch))).map_err(err)?;
        self.save = None;
        if let (Some(t), Some(r)) = (&mut self.views, before) {
            t.record(ch, Source::Seeded(r));
        }
        Ok(())
    }

    /// One step with every random event forced from `path` (an outcome's path from `outcomes`).
    #[pyo3(signature = (a0, a1, path))]
    fn step_forced(&mut self, py: Python<'_>, a0: Option<i64>, a1: Option<i64>, path: Vec<u16>) -> PyResult<()> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        let g = &mut self.game;
        let p = &path;
        py.detach(|| guarded(|| g.step_forced(ch, p))).map_err(err)?;
        self.save = None;
        if let Some(t) = &mut self.views {
            t.record(ch, Source::Path(Arc::new(path)));
        }
        Ok(())
    }

    /// Every outcome of the next step with these actions: (outcomes, covered, capped, leaves), where each
    /// outcome is (probability, path, leaves merged, Game). Each Game keeps this game's seed state.
    /// `merge` joins outcomes with equal States; `max_branches` caps the replays (then `capped` is true
    /// and `covered` < 1).
    #[pyo3(signature = (a0, a1, merge = true, max_branches = 100_000))]
    fn outcomes(&self, py: Python<'_>, a0: Option<i64>, a1: Option<i64>, merge: bool, max_branches: usize) -> PyResult<(OutcomeList, f64, bool, usize)> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        let opts = EnumOptions { merge, max_branches: max_branches.max(1), keep_events: false };
        let g = &self.game;
        let e = py.detach(|| guarded(|| g.outcomes(ch, &opts))).map_err(err)?;
        let (covered, capped, leaves) = (e.covered, e.capped, e.leaves);
        Ok((self.children(e, ch), covered, capped, leaves))
    }

    /// `n` outcomes of the next step drawn with the engine's probabilities (draw k seeded
    /// `mix_seed(seed, k)`): (outcomes, 1.0, False, n), each outcome (weight, path, draws, Game). Each
    /// draw has weight 1/n; with `merge`, draws with equal States are one outcome with their summed weight.
    /// `path` forces the outcome again with `step_forced`.
    #[pyo3(signature = (a0, a1, n, seed = 0, merge = true))]
    fn sample(&self, py: Python<'_>, a0: Option<i64>, a1: Option<i64>, n: usize, seed: u64, merge: bool) -> PyResult<(OutcomeList, f64, bool, usize)> {
        let ch = self.choices([a0, a1]).map_err(err)?;
        if n == 0 {
            return Err(err("sample: n must be at least 1"));
        }
        let st = &self.game.state;
        let e = py.detach(|| guarded(|| sample_step(st, ch, n, seed, merge))).map_err(err)?;
        let leaves = e.leaves;
        Ok((self.children(e, ch), 1.0, false, leaves))
    }

    /// Whether the two games' States are equal (the seed state is not compared).
    fn same_state(&self, other: &PyGame) -> bool {
        self.game.state == other.game.state
    }

    /// What `seat` knows now: its Battle, with its open request's kind, decision id and mask (kind 3,
    /// decision -1 and an empty mask with none open).
    fn battle(&mut self, seat: i64) -> PyResult<PyBattle> {
        let seat = Self::seat_of(seat)?;
        let v = self.views_of(seat)?;
        let mut mask = vec![0u8; N_ACTIONS];
        v.action_mask(seat, &mut mask);
        let (decision, kind) = match v.request(seat) {
            Some((d, k)) => (d as i64, k),
            None => (-1, VIEW_NONE),
        };
        Ok(PyBattle { b: *v.battle(seat), game: 0, seat: seat as u8, decision, kind, mask })
    }

    /// Writes `seat`'s observation into eight writable C-contiguous buffers (rows f32[12,300], row_ids
    /// i32[12,10], field f32[66], last f32[17], last_ids i32[4], mask u8[11], action_mask u8[71], opp_prior
    /// f32[13]) and returns (seat, kind, decision): what `BatchEnv.observe_into` writes for this seat.
    #[pyo3(signature = (seat, rows, row_ids, field, last, last_ids, mask, action_mask, opp_prior, damage = true))]
    #[allow(clippy::too_many_arguments)]
    fn tensors_into(
        &mut self,
        seat: i64,
        rows: &Bound<'_, PyAny>,
        row_ids: &Bound<'_, PyAny>,
        field: &Bound<'_, PyAny>,
        last: &Bound<'_, PyAny>,
        last_ids: &Bound<'_, PyAny>,
        mask: &Bound<'_, PyAny>,
        action_mask: &Bound<'_, PyAny>,
        opp_prior: &Bound<'_, PyAny>,
        damage: bool,
    ) -> PyResult<(u8, u8, i64)> {
        let seat = Self::seat_of(seat)?;
        let (mut s, mut k, mut d) = ([0u8; 1], [0u8; 1], [0i64; 1]);
        let r = tensors::writable::<f32>(rows, "rows", ROWS_LEN)?;
        let i = tensors::writable::<i32>(row_ids, "row_ids", ROW_IDS_LEN)?;
        let f = tensors::writable::<f32>(field, "field", FIELD)?;
        let l = tensors::writable::<f32>(last, "last", LAST)?;
        let li = tensors::writable::<i32>(last_ids, "last_ids", LAST_IDS)?;
        let m = tensors::writable::<u8>(mask, "mask", MASK)?;
        let a = tensors::writable::<u8>(action_mask, "action_mask", N_ACTIONS)?;
        let q = tensors::writable::<f32>(opp_prior, "opp_prior", OPP_ACTIONS)?;
        tensors::disjoint(&[
            tensors::span("rows", &r),
            tensors::span("row_ids", &i),
            tensors::span("field", &f),
            tensors::span("last", &l),
            tensors::span("last_ids", &li),
            tensors::span("mask", &m),
            tensors::span("action_mask", &a),
            tensors::span("opp_prior", &q),
        ])?;
        let v = self.views_of(seat)?;
        // the buffers are held above for the whole call and were checked disjoint
        let mut out = unsafe {
            ObsBatch {
                rows: tensors::slice_mut(&r),
                row_ids: tensors::slice_mut(&i),
                field: tensors::slice_mut(&f),
                last: tensors::slice_mut(&l),
                last_ids: tensors::slice_mut(&li),
                mask: tensors::slice_mut(&m),
                actions: tensors::slice_mut(&a),
                seat: &mut s,
                kind: &mut k,
                decision: &mut d,
                opp_prior: tensors::slice_mut(&q),
            }
        };
        guarded(|| {
            observe(v, seat, &mut out, 0, damage);
            Ok(())
        })
        .map_err(err)?;
        Ok((s[0], k[0], d[0]))
    }

    /// Both sides' Pokémon from the State, in team-slot order (the picks, lead first; switch action 8 + i
    /// is slot i): (hp fraction f32[2,3], fainted u8[2,3], status u8[2,3] (0 none, 1 brn, 2 par, 3 slp,
    /// 4 frz, 5 psn, 6 tox), active i8[2] (the slot on the field, -1 none)), little-endian bytes.
    #[allow(clippy::type_complexity)]
    fn readout<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
        let st = &self.game.state;
        let hp = PyBytes::new_with(py, SIDES * TEAM * 4, |b| {
            for (k, x) in b.chunks_exact_mut(4).enumerate() {
                let m = &st.sides[k / TEAM].mons[k % TEAM];
                x.copy_from_slice(&(m.hp as f32 / m.maxhp.max(1) as f32).to_le_bytes());
            }
            Ok(())
        })?;
        let fainted = PyBytes::new_with(py, SIDES * TEAM, |b| {
            for (k, x) in b.iter_mut().enumerate() {
                *x = st.sides[k / TEAM].mons[k % TEAM].fainted as u8;
            }
            Ok(())
        })?;
        let status = PyBytes::new_with(py, SIDES * TEAM, |b| {
            for (k, x) in b.iter_mut().enumerate() {
                *x = st.sides[k / TEAM].mons[k % TEAM].status as u8;
            }
            Ok(())
        })?;
        let active = PyBytes::new_with(py, SIDES, |b| {
            for (k, x) in b.iter_mut().enumerate() {
                let sd = &st.sides[k];
                *x = if sd.has_active && !sd.active_mon().fainted { sd.active } else { 0xFF };
            }
            Ok(())
        })?;
        Ok((hp, fainted, status, active))
    }
}

/// Raw pointers handed to worker threads; the objects stay borrowed by the caller for the whole call.
struct Ptr<T>(*const T);
unsafe impl<T> Send for Ptr<T> {}
unsafe impl<T> Sync for Ptr<T> {}

/// Writes the observation of `seats[i]` in `games[i]` into entry i of eleven writable C-contiguous buffers
/// (the arrays of `BatchEnv.observe_into` with N = len(games)). The same Game may appear more than once.
/// The views are built under the GIL; the observations are written without it, on `threads` threads.
#[pyfunction]
#[pyo3(signature = (games, seats, rows, row_ids, field, last, last_ids, mask, action_mask, seat, kind, decision, opp_prior, threads = 1, damage = true))]
#[allow(clippy::too_many_arguments)]
pub fn games_tensors_into(
    py: Python<'_>,
    games: &Bound<'_, PyList>,
    seats: Vec<i64>,
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
    threads: usize,
    damage: bool,
) -> PyResult<()> {
    let n = games.len();
    if seats.len() != n {
        return Err(err(format!("games_tensors: {} seats for {} games", seats.len(), n)));
    }
    let bufs = BatchBufs::get(n, [rows, row_ids, field, last, last_ids, mask, action_mask, seat, kind, decision, opp_prior])?;
    // each distinct Game borrowed once, its views built
    let mut held: Vec<PyRefMut<'_, PyGame>> = Vec::new();
    let mut ptrs: Vec<*mut pyo3::ffi::PyObject> = Vec::new();
    let mut which = Vec::with_capacity(n);
    for (i, g) in games.iter().enumerate() {
        let p = g.as_ptr();
        let at = match ptrs.iter().position(|&x| x == p) {
            Some(j) => j,
            None => {
                let mut r: PyRefMut<'_, PyGame> = g.extract().map_err(|_| err(format!("games[{}] is not a Game", i)))?;
                r.views_now()?;
                held.push(r);
                ptrs.push(p);
                held.len() - 1
            }
        };
        which.push(at);
        let st = PyGame::seat_of(seats[i])?;
        held[at].views_of(st)?;
    }
    let views: Vec<Ptr<Views>> = held.iter().map(|g| Ptr(g.views.as_ref().and_then(|t| t.built()).expect("built") as *const Views)).collect();
    // the buffers stay held by `bufs` and were checked disjoint; the Games stay borrowed by `held`
    let out = unsafe { bufs.out() };
    let work = |a: usize, mut part: ObsBatch<'_>| {
        for k in 0..part.seat.len() {
            let v = unsafe { &*views[which[a + k]].0 };
            observe(v, seats[a + k] as usize, &mut part, k, damage);
        }
    };
    py.detach(|| {
        guarded(|| {
            let t = threads.clamp(1, n.max(1));
            if t == 1 {
                work(0, out);
                return Ok(());
            }
            let per = n.div_ceil(t);
            std::thread::scope(|sc| {
                let mut rest = out;
                let mut a = 0;
                while a < n {
                    let k = per.min(n - a);
                    let (part, tail) = split(rest, k);
                    rest = tail;
                    let w = &work;
                    sc.spawn(move || w(a, part));
                    a += k;
                }
            });
            Ok(())
        })
    })
    .map_err(err)?;
    drop(held);
    Ok(())
}

/// The first `g` entries of a batch and the rest.
fn split(o: ObsBatch<'_>, g: usize) -> (ObsBatch<'_>, ObsBatch<'_>) {
    let (r0, r1) = o.rows.split_at_mut(g * ROWS_LEN);
    let (i0, i1) = o.row_ids.split_at_mut(g * ROW_IDS_LEN);
    let (f0, f1) = o.field.split_at_mut(g * FIELD);
    let (l0, l1) = o.last.split_at_mut(g * LAST);
    let (d0, d1) = o.last_ids.split_at_mut(g * LAST_IDS);
    let (m0, m1) = o.mask.split_at_mut(g * MASK);
    let (a0, a1) = o.actions.split_at_mut(g * N_ACTIONS);
    let (s0, s1) = o.seat.split_at_mut(g);
    let (k0, k1) = o.kind.split_at_mut(g);
    let (e0, e1) = o.decision.split_at_mut(g);
    let (q0, q1) = o.opp_prior.split_at_mut(g * OPP_ACTIONS);
    (
        ObsBatch { rows: r0, row_ids: i0, field: f0, last: l0, last_ids: d0, mask: m0, actions: a0, seat: s0, kind: k0, decision: e0, opp_prior: q0 },
        ObsBatch { rows: r1, row_ids: i1, field: f1, last: l1, last_ids: d1, mask: m1, actions: a1, seat: s1, kind: k1, decision: e1, opp_prior: q1 },
    )
}

/// One opposing guess from a dict: slot, moves, item, ability, nature, points, gender. Unknown ids are
/// rejected (the fallback is used).
fn guess_of(d: &Bound<'_, PyDict>, rej: &mut Vec<Rejected>) -> PyResult<OppGuess> {
    let slot: u8 = d.get_item("slot")?.ok_or_else(|| err("opponent entry without 'slot' (its preview index 0..5)"))?.extract()?;
    let who = format!("preview slot {}", slot);
    let mut g = OppGuess { slot, ..OppGuess::default() };
    let mut bad = |field: &'static str, reason: String| rej.push(Rejected { mon: who.clone(), field, reason });
    if let Some(ms) = d.get_item("moves")? {
        if !ms.is_none() {
            for m in ms.extract::<Vec<String>>()? {
                match MoveId::from_id(&to_id(&m)).filter(|x| !x.is_none()) {
                    Some(x) => g.moves.push(x),
                    None => bad("moves", format!("unknown move {:?}", m)),
                }
            }
        }
    }
    if let Some(x) = d.get_item("item")? {
        if !x.is_none() {
            let i: String = x.extract()?;
            if to_id(&i).is_empty() {
                g.item = Some(ItemId::NONE);
            } else {
                match ItemId::from_id(&to_id(&i)) {
                    Some(it) => g.item = Some(it),
                    None => bad("item", format!("unknown item {:?}", i)),
                }
            }
        }
    }
    if let Some(x) = d.get_item("ability")? {
        if !x.is_none() {
            let a: String = x.extract()?;
            match AbilityId::from_id(&to_id(&a)) {
                Some(ab) => g.ability = Some(ab),
                None => bad("ability", format!("unknown ability {:?}", a)),
            }
        }
    }
    if let Some(x) = d.get_item("nature")? {
        if !x.is_none() {
            let n: String = x.extract()?;
            match nature_from_id(&to_id(&n)) {
                Some(k) => g.nature = Some(k),
                None => bad("nature", format!("unknown nature {:?}", n)),
            }
        }
    }
    if let Some(x) = d.get_item("points")? {
        if !x.is_none() {
            let p: Vec<i64> = if let Ok(m) = x.cast::<PyDict>() {
                ["hp", "atk", "def", "spa", "spd", "spe"].iter().map(|k| m.get_item(*k).ok().flatten().and_then(|v| v.extract::<i64>().ok()).unwrap_or(0)).collect()
            } else {
                x.extract()?
            };
            if p.len() != 6 || p.iter().any(|&v| !(0..=255).contains(&v)) {
                bad("points", format!("{:?}: need six numbers 0..255 (hp, atk, def, spa, spd, spe)", p));
            } else {
                g.points = Some([p[0] as u8, p[1] as u8, p[2] as u8, p[3] as u8, p[4] as u8, p[5] as u8]);
            }
        }
    }
    if let Some(x) = d.get_item("gender")? {
        if !x.is_none() {
            let s: String = x.extract()?;
            g.gender = match s.as_str() {
                "M" => Some(1),
                "F" => Some(2),
                _ => {
                    bad("gender", format!("{:?} (M or F)", s));
                    None
                }
            };
        }
    }
    Ok(g)
}

/// The opponent's guesses (dicts: the three brought, lead first) and our genders by team index.
#[allow(clippy::type_complexity)]
pub fn guesses_of(opponent: &Bound<'_, PyList>, genders: Option<Vec<Option<String>>>) -> PyResult<(Vec<OppGuess>, [Option<u8>; 6], Vec<Rejected>)> {
    let mut rej = Vec::new();
    let mut gs = Vec::new();
    for o in opponent.iter() {
        let d = o.cast::<PyDict>().map_err(|_| err("opponent: a list of dicts, one per opposing Pokémon brought"))?;
        gs.push(guess_of(d, &mut rej)?);
    }
    let mut gd = [None; 6];
    for (i, g) in genders.unwrap_or_default().iter().enumerate().take(6) {
        gd[i] = match g.as_deref() {
            Some("M") => Some(1),
            Some("F") => Some(2),
            _ => None,
        };
    }
    Ok((gs, gd, rej))
}

/// (mon, field, reason) tuples.
pub fn rejected_list(r: Vec<Rejected>) -> Vec<(String, String, String)> {
    r.into_iter().map(|x| (x.mon, x.field.to_string(), x.reason)).collect()
}

/// A Game from a Battle (`Session.battle`) at its open turn or replace request, with the opponent's
/// guesses: (Game, rejected). The Session's own `LiveCore.game` is preferred (the whole Builder).
#[pyfunction]
#[pyo3(signature = (battle, opponent, seat = 0, genders = None, seed = 0))]
pub fn game_from_battle(battle: &PyBattle, opponent: &Bound<'_, PyList>, seat: usize, genders: Option<Vec<Option<String>>>, seed: u64) -> PyResult<(PyGame, Vec<(String, String, String)>)> {
    if seat > 1 {
        return Err(err("seat is 0 or 1"));
    }
    let kind = match battle.kind {
        1 => ReqKind::Turn,
        2 => ReqKind::Replace,
        _ => return Err(err("the Battle has no open turn or replace request")),
    };
    let (gs, gd, mut rej) = guesses_of(opponent, genders)?;
    let mut b = battle::Builder::new();
    b.battle = battle.b;
    b.request = Some((battle.decision, kind));
    let (game, t, r) = guarded(|| env::from_battle::game_from_builder(&b, kind, battle.decision, seat, &gs, &gd, seed, "live")).map_err(err)?;
    rej.extend(r);
    Ok((PyGame::new(game, Some(t)), rejected_list(rej)))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(games_tensors_into, m)?)?;
    m.add_function(wrap_pyfunction!(game_from_battle, m)?)?;
    m.add("KIND_NO_REQUEST", VIEW_NONE)?;
    m.add("GAME_UNKNOWN", env::from_battle::UNKNOWN.to_vec())?;
    Ok(())
}
