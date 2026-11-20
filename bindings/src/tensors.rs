//! The observation for Python: writes into caller-provided writable buffers (numpy arrays, or
//! `memoryview`s over `bytearray`s), so nothing is copied per element. The Python layer
//! (`champions_gym.obs`) allocates them and names the result.

use env::observe::{ObsBatch, ObsOptions, ROWS_LEN, ROW_IDS_LEN};
use env::N_ALL_ACTIONS;
use obs::layout::{Feature, FEATURES, MOVE_BASE, MOVE_STRIDE};
use obs::{vocab, Block, FIELD, LAST, LAST_IDS, MASK, N_ROWS, ROW, ROW_IDS};
use pyo3::buffer::{Element, PyBuffer};
use pyo3::prelude::*;

use super::err;

/// A writable C-contiguous buffer of `T` with `len` elements.
pub fn writable<T: Element>(obj: &Bound<'_, PyAny>, name: &str, len: usize) -> PyResult<PyBuffer<T>> {
    let b = PyBuffer::<T>::get(obj).map_err(|e| err(format!("{}: {}", name, e)))?;
    if b.readonly() {
        return Err(err(format!("{}: buffer is read-only", name)));
    }
    if !b.is_c_contiguous() {
        return Err(err(format!("{}: buffer is not C-contiguous", name)));
    }
    if b.item_count() != len {
        return Err(err(format!("{}: {} elements, need {}", name, b.item_count(), len)));
    }
    Ok(b)
}

/// The buffer's elements as a mutable slice.
///
/// # Safety
/// The buffer must stay alive and unaliased while the slice is used.
pub unsafe fn slice_mut<'a, T: Element>(b: &'a PyBuffer<T>) -> &'a mut [T] {
    std::slice::from_raw_parts_mut(b.buf_ptr() as *mut T, b.item_count())
}

/// Refuses buffers that overlap (the same array passed twice).
pub fn disjoint(spans: &[(&str, usize, usize)]) -> PyResult<()> {
    for (i, a) in spans.iter().enumerate() {
        for b in &spans[i + 1..] {
            if a.2 > 0 && b.2 > 0 && a.1 < b.1 + b.2 && b.1 < a.1 + a.2 {
                return Err(err(format!("{} and {} share memory", a.0, b.0)));
            }
        }
    }
    Ok(())
}

pub fn span<T: Element>(name: &'static str, b: &PyBuffer<T>) -> (&'static str, usize, usize) {
    (name, b.buf_ptr() as usize, b.len_bytes())
}

/// The eleven batch buffers, checked.
pub struct BatchBufs {
    rows: PyBuffer<f32>,
    row_ids: PyBuffer<i32>,
    field: PyBuffer<f32>,
    last: PyBuffer<f32>,
    last_ids: PyBuffer<i32>,
    mask: PyBuffer<u8>,
    actions: PyBuffer<u8>,
    seat: PyBuffer<u8>,
    kind: PyBuffer<u8>,
    decision: PyBuffer<i64>,
    opp_prior: PyBuffer<f32>,
}

impl BatchBufs {
    #[allow(clippy::too_many_arguments)]
    pub fn get(n: usize, a: [&Bound<'_, PyAny>; 11]) -> PyResult<BatchBufs> {
        let b = BatchBufs {
            rows: writable(a[0], "rows", n * ROWS_LEN)?,
            row_ids: writable(a[1], "row_ids", n * ROW_IDS_LEN)?,
            field: writable(a[2], "field", n * FIELD)?,
            last: writable(a[3], "last", n * LAST)?,
            last_ids: writable(a[4], "last_ids", n * LAST_IDS)?,
            mask: writable(a[5], "mask", n * MASK)?,
            actions: writable(a[6], "action_mask", n * N_ALL_ACTIONS)?,
            seat: writable(a[7], "seat", n)?,
            kind: writable(a[8], "kind", n)?,
            decision: writable(a[9], "decision", n)?,
            opp_prior: writable(a[10], "opp_prior", n * obs::OPP_ACTIONS)?,
        };
        disjoint(&[
            span("rows", &b.rows),
            span("row_ids", &b.row_ids),
            span("field", &b.field),
            span("last", &b.last),
            span("last_ids", &b.last_ids),
            span("mask", &b.mask),
            span("action_mask", &b.actions),
            span("seat", &b.seat),
            span("kind", &b.kind),
            span("decision", &b.decision),
            span("opp_prior", &b.opp_prior),
        ])?;
        Ok(b)
    }

    /// # Safety
    /// As `slice_mut`: the buffers are held by `self` and were checked disjoint.
    pub unsafe fn out(&self) -> ObsBatch<'_> {
        ObsBatch {
            rows: slice_mut(&self.rows),
            row_ids: slice_mut(&self.row_ids),
            field: slice_mut(&self.field),
            last: slice_mut(&self.last),
            last_ids: slice_mut(&self.last_ids),
            mask: slice_mut(&self.mask),
            actions: slice_mut(&self.actions),
            seat: slice_mut(&self.seat),
            kind: slice_mut(&self.kind),
            decision: slice_mut(&self.decision),
            opp_prior: slice_mut(&self.opp_prior),
        }
    }
}

pub fn options(threads: usize, damage: bool) -> ObsOptions {
    ObsOptions { threads: threads.max(1), damage }
}

fn block_name(b: Block) -> &'static str {
    match b {
        Block::Row => "rows",
        Block::RowIds => "row_ids",
        Block::Field => "field",
        Block::Last => "last",
        Block::LastIds => "last_ids",
        Block::Mask => "mask",
        Block::Table => "table",
    }
}

/// The vocabulary an id feature indexes.
fn vocab_of(f: &Feature) -> Option<&'static str> {
    match f.id {
        "F1-3" | "L2-2.1" | "L2-2.2" => Some("species"),
        "F2-6" => Some("items"),
        "F2-7" | "F8-10" => Some("abilities"),
        "F7-13" | "L2-1.1" | "L2-1.2" => Some("moves"),
        _ => None,
    }
}

type FeatureRow = (&'static str, &'static str, &'static str, usize, usize, usize, bool, &'static str, &'static str, f32, f32, Option<&'static str>);

/// Every feature of the layout: (id, name, block, offset, size, repeats, slot, encoding, scale, lo, hi,
/// vocabulary).
#[pyfunction]
pub fn features() -> Vec<FeatureRow> {
    FEATURES
        .iter()
        .map(|f| (f.id, f.name, block_name(f.block), f.offset, f.size, f.repeats, f.slot, f.encoding, f.scale, f.lo, f.hi, vocab_of(f)))
        .collect()
}

/// Sizes the layout needs: vocabularies, shapes, move-slot placement.
#[pyfunction]
pub fn layout_sizes() -> Vec<(&'static str, usize)> {
    vec![
        ("species", vocab::SPECIES_VOCAB),
        ("moves", vocab::MOVE_VOCAB),
        ("items", vocab::ITEM_VOCAB),
        ("abilities", vocab::ABILITY_VOCAB),
        ("n_rows", N_ROWS),
        ("row", ROW),
        ("row_ids", ROW_IDS),
        ("field", FIELD),
        ("last", LAST),
        ("last_ids", LAST_IDS),
        ("mask", MASK),
        ("actions", N_ALL_ACTIONS),
        ("opp_actions", env::N_OPP_ACTIONS),
        ("move_base", MOVE_BASE),
        ("move_stride", MOVE_STRIDE),
    ]
}

/// Fills a writable uint8 buffer of species x moves (1 = the species can learn the move).
#[pyfunction]
pub fn learnset_into(table: &Bound<'_, PyAny>) -> PyResult<()> {
    let b = writable::<u8>(table, "table", vocab::SPECIES_VOCAB * vocab::MOVE_VOCAB)?;
    obs::learnset_table(unsafe { slice_mut(&b) });
    Ok(())
}
