//! CG's observation: the model's input tensors from a Battle (what one player knows), laid out as in
//! `features_proposed.csv` (copied here from `pcsa-former`; the layout is generated from it by
//! `tools/obs/gen_layout.py`). Depends only on the Battle and CG's static data, so it is the same in
//! training and live. Constant time; writes into caller-provided arrays without allocating.
//!
//! Rows: 0..6 our team, 6..12 the opponent's (`row_order`). Shapes and vocabularies: README.md.

pub mod layout;
pub mod learnsets;
pub mod vocab;
pub mod vols;

use battle::track::is_setup;
use battle::*;
use layout::*;
use players::estimate::{estimate, Hit};
use players::view::{own_form_stats, speed, Fighter, FieldView};
use cg_engine::sim::calc::stat_from_points;
use cg_engine::sim::data::*;

/// Which array a feature lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Row,
    RowIds,
    Field,
    Last,
    LastIds,
    Mask,
    /// the fixed learnset table (not part of each observation)
    Table,
}

pub const N_ROWS: usize = 12;
pub const ROW: usize = ROW_LEN;
pub const ROW_IDS: usize = ROWIDS_LEN;
pub const FIELD: usize = FIELD_LEN;
pub const LAST: usize = LAST_LEN;
pub const LAST_IDS: usize = LASTIDS_LEN;
pub const MASK: usize = MASK_LEN;
/// opponent actions seen from the observer: 4 moves of row 6, the same with Mega, switch to rows 7..12
pub const OPP_ACTIONS: usize = 13;

/// One observation as plain contiguous arrays (C layout, no padding between elements of an array).
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct ObsBuffers {
    /// [12, 300] f32
    pub rows: [[f32; ROW]; N_ROWS],
    /// [12, 10] i32
    pub row_ids: [[i32; ROW_IDS]; N_ROWS],
    /// [66] f32
    pub field: [f32; FIELD],
    /// [17] f32
    pub last: [f32; LAST],
    /// [4] i32
    pub last_ids: [i32; LAST_IDS],
    /// [11] u8, 1 = legal
    pub mask: [u8; MASK],
    /// [13] f32, how available each opponent action is (`opp_prior`)
    pub opp_prior: [f32; OPP_ACTIONS],
}

impl Default for ObsBuffers {
    fn default() -> Self {
        ObsBuffers {
            rows: [[0.0; ROW]; N_ROWS],
            row_ids: [[0; ROW_IDS]; N_ROWS],
            field: [0.0; FIELD],
            last: [0.0; LAST],
            last_ids: [0; LAST_IDS],
            mask: [0; MASK],
            opp_prior: [0.0; OPP_ACTIONS],
        }
    }
}

/// The same arrays as caller-provided flat slices (rows 12*300, row_ids 12*10, field 66, last 17,
/// last_ids 4, mask 11, opp_prior 13), e.g. one entry of a batch held by numpy.
pub struct ObsOut<'a> {
    pub rows: &'a mut [f32],
    pub row_ids: &'a mut [i32],
    pub field: &'a mut [f32],
    pub last: &'a mut [f32],
    pub last_ids: &'a mut [i32],
    pub mask: &'a mut [u8],
    pub opp_prior: &'a mut [f32],
}

impl ObsBuffers {
    pub fn out(&mut self) -> ObsOut<'_> {
        ObsOut {
            rows: self.rows.as_flattened_mut(),
            row_ids: self.row_ids.as_flattened_mut(),
            field: &mut self.field,
            last: &mut self.last,
            last_ids: &mut self.last_ids,
            mask: &mut self.mask,
            opp_prior: &mut self.opp_prior,
        }
    }
}

/// Fills `out` with the observation of `b` for a request of kind `kind`.
pub fn observe(b: &Battle, kind: ReqKind, out: &mut ObsBuffers) {
    observe_into(b, kind, out.out(), true);
}

/// `observe` into flat slices; `damage: false` leaves the damage features at 0 (for speed runs).
pub fn observe_into(b: &Battle, kind: ReqKind, o: ObsOut<'_>, damage: bool) {
    assert!(o.rows.len() == N_ROWS * ROW && o.row_ids.len() == N_ROWS * ROW_IDS && o.field.len() == FIELD);
    assert!(o.last.len() == LAST && o.last_ids.len() == LAST_IDS && o.mask.len() == MASK && o.opp_prior.len() == OPP_ACTIONS);
    o.rows.fill(0.0);
    o.row_ids.fill(0);
    o.field.fill(0.0);
    o.last.fill(0.0);
    o.last_ids.fill(0);
    let c = Ctx::new(b, kind);
    for (i, m) in c.legal.mask.iter().enumerate() {
        o.mask[i] = *m as u8;
    }
    let hits = if damage { c.damage() } else { Damage::default() };
    for r in 0..N_ROWS {
        let f = &mut o.rows[r * ROW..(r + 1) * ROW];
        let ids = &mut o.row_ids[r * ROW_IDS..(r + 1) * ROW_IDS];
        c.row(r, f, ids, &hits);
    }
    c.field(o.field);
    c.last(o.last, o.last_ids);
    opp_prior_into(b, &c.rows, o.opp_prior);
}

/// How available each opponent action is, from the Battle alone (model contract section 5): 0
/// impossible, 1 certain, between an estimate. Moves 0..4 of row 6: a revealed move 1 (0 if shown disabled
/// or out of PP, unless it is shown locked into or charging that move), the first empty slot 1, later empty slots 0. Mega 4..8: the plain move's value times 1
/// if its stone was shown, 0 if another item was shown, the species has no Mega or the side already Mega
/// Evolved, else 0.5. Switches 8..13 (rows 7..12): a seen unfainted Pokémon 1, fainted or known not
/// brought 0, unseen (3 - k) / (6 - k) with k of theirs seen; all 0 while their active is shown trapped.
/// All 0 when they have no Pokémon on the field (preview).
pub fn opp_prior(b: &Battle, out: &mut [f32; OPP_ACTIONS]) {
    opp_prior_into(b, &row_order(b), out);
}

fn opp_prior_into(b: &Battle, rows: &[Option<Who>; N_ROWS], out: &mut [f32]) {
    out.fill(0.0);
    let Some(w) = b.active(Side::Opp).filter(|w| !b.mon(*w).fainted) else { return };
    let a = &b.side(Side::Opp).active;
    let m = b.mon(w);
    let pp = b.pp_left(w);
    let mv = row_moves(b, w);
    let mut first_empty = true;
    for j in 0..4 {
        out[j] = match mv[j] {
            Some(x) => {
                let disabled = vol(a, conds::DISABLE).map_or(false, |v| v.detail == VolDetail::Move(x));
                // a move it is locked into or charging goes on without PP
                let locked = matches!(a.lock, Lock::Locked { mv, .. } if mv == x) || matches!(a.lock, Lock::Charging { mv: Some(m), .. } if m == x);
                (locked || (!disabled && pp[j] != Some(0))) as u8 as f32
            }
            None if first_empty => {
                first_empty = false;
                1.0
            }
            None => 0.0,
        };
    }
    let form = m.form.or(m.species).or(m.preview);
    let mega = if b.side(Side::Opp).mega_used || m.mega {
        0.0
    } else {
        match (m.item, form) {
            (ItemState::Held(it), Some(f)) => battle::items::mega_form(it, f).is_some() as u8 as f32,
            (ItemState::Unknown, Some(f)) if has_mega(f) => 0.5,
            _ => 0.0,
        }
    };
    for j in 0..4 {
        out[4 + j] = out[j] * mega;
    }
    if opp_trapped(b, w) {
        return;
    }
    let team = &b.sides[1].team;
    let k = team.iter().filter(|t| t.seen).count();
    let unseen = if k < 3 { (3 - k) as f32 / (6 - k) as f32 } else { 0.0 };
    for r in 7..N_ROWS {
        if let Some(x) = rows[r] {
            let t = b.mon(x);
            out[8 + r - 7] = if t.fainted {
                0.0
            } else if t.seen {
                1.0
            } else {
                unseen
            };
        }
    }
}

/// Whether some item gives this species a Mega form.
fn has_mega(s: SpeciesId) -> bool {
    static T: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
    let t = T.get_or_init(|| {
        let mut v = vec![false; SPECIES.len()];
        for i in 0..ITEMS.len() {
            for (base, _) in ItemId(i as u16).data().mega_stone {
                v[base.0 as usize] = true;
            }
        }
        v
    });
    t[s.0 as usize]
}

/// The Battle shows the opposing active cannot switch: a trap on it, with its item and ability known and
/// neither an escape, and it is not a Ghost.
fn opp_trapped(b: &Battle, w: Who) -> bool {
    let a = &b.side(Side::Opp).active;
    let m = b.mon(w);
    if b.types(w).map_or(true, |t| t.contains(&Type::Ghost)) {
        return false;
    }
    let item_ok = match m.item {
        ItemState::Unknown => false,
        ItemState::Held(i) => i.id() != "shedshell",
        ItemState::Gone(_) => true,
    };
    if !item_ok || m.ability.map_or(true, |x| x.id() == "runaway") {
        return false;
    }
    // a partial trap or Octolock holds only while its user is on the field
    let foe = b.active(Side::Own);
    let holds = |v: &Vol| match v.cond.id() {
        "trapped" | "noretreat" | "ingrain" => true,
        "partiallytrapped" | "octolock" => v.by.is_none() || v.by == foe,
        _ => false,
    };
    a.vols.iter().flatten().any(holds) || b.field.rooms[R_FAIRYLOCK].map_or(false, |f| b.turn <= f.start.saturating_add(1))
}

/// The Pokémon in each row. Ours (0..6) in pick order: rows 0..3 the picks (lead first, so switch action
/// 8 + p is row p), then the others in team order; before the picks (preview) all six in team order. The
/// active is marked by F3-9 only. Theirs: 6 their active, then the seen, then the unseen, each in preview
/// order. `None`: an empty row.
pub fn row_order(b: &Battle) -> [Option<Who>; N_ROWS] {
    let mut out = [None; N_ROWS];
    let n = (b.n_own as usize).min(6);
    let mut k = 0;
    for p in b.picks.iter().flatten() {
        if (*p as usize) < n && !out[..k].contains(&Some(Who { side: Side::Own, slot: *p })) {
            out[k] = Some(Who { side: Side::Own, slot: *p });
            k += 1;
        }
    }
    for i in 0..n as u8 {
        let w = Some(Who { side: Side::Own, slot: i });
        if !out[..k].contains(&w) {
            out[k] = w;
            k += 1;
        }
    }
    let team = &b.side(Side::Opp).team;
    let first = b.side(Side::Opp).active.slot.map(|x| x as usize).filter(|&x| x < 6);
    let mut k = 6;
    let mut put = |i: usize, out: &mut [Option<Who>; N_ROWS]| {
        out[k] = Some(Who { side: Side::Opp, slot: i as u8 });
        k += 1;
    };
    if let Some(a) = first {
        put(a, &mut out);
    }
    for seen in [true, false] {
        for i in 0..6 {
            if Some(i) != first && team[i].seen == seen {
                put(i, &mut out);
            }
        }
    }
    out
}

/// Fills a [SPECIES_VOCAB, MOVE_VOCAB] u8 table (M2-1): 1 where the species can learn the move.
pub fn learnset_table(out: &mut [u8]) {
    assert_eq!(out.len(), vocab::SPECIES_VOCAB * vocab::MOVE_VOCAB);
    out.fill(0);
    for (s, bits) in learnsets::LEARNSET.iter().enumerate() {
        let row = &mut out[(s + 1) * vocab::MOVE_VOCAB..(s + 2) * vocab::MOVE_VOCAB];
        for (m, x) in row.iter_mut().enumerate() {
            *x = ((bits[m / 64] >> (m % 64)) & 1) as u8;
        }
    }
}

pub fn can_learn(s: SpeciesId, m: MoveId) -> bool {
    let bits = &learnsets::LEARNSET[s.0 as usize];
    (bits[m.0 as usize / 64] >> (m.0 % 64)) & 1 == 1
}

// ------------------------------------------------------------------ move classes

pub fn is_pivot(mv: MoveId) -> bool {
    mv.data().self_switch != SelfSwitch::No
}

pub fn is_heal_or_drain(mv: MoveId) -> bool {
    let d = mv.data();
    d.has_flag(flag::HEAL) || d.drain.0 > 0 || d.heal.0 > 0
}

/// Expected hits for the power column: 2-5 hits count 3.1, other ranges their middle, Triple Axel and
/// Triple Kick the sum of their power steps, Population Bomb all ten.
pub fn dex_hits(mv: MoveId) -> f32 {
    let d = mv.data();
    let (lo, hi) = d.multihit;
    if hi == 0 {
        return 1.0;
    }
    if matches!(d.id, "tripleaxel" | "triplekick") {
        return (1..=hi as u32).sum::<u32>() as f32;
    }
    if d.multiaccuracy || lo == hi {
        return hi as f32;
    }
    if (lo, hi) == (2, 5) {
        return 3.1;
    }
    (lo as f32 + hi as f32) / 2.0
}

/// Type-chart multiplier of a move type against a pair of types (0 when immune).
pub fn type_multiplier(ty: Type, types: [Type; 2]) -> f32 {
    let mut m = 1.0;
    for t in types {
        if !t.is_real() || !ty.is_real() {
            continue;
        }
        if !type_affects(ty, t) {
            return 0.0;
        }
        m *= match effectiveness(ty, t) {
            1 => 2.0,
            -1 => 0.5,
            _ => 1.0,
        };
    }
    m
}

// ------------------------------------------------------------------ helpers

#[inline]
fn clip(x: f32, lo: f32, hi: f32) -> f32 {
    if x.is_finite() {
        x.clamp(lo, hi)
    } else {
        0.0
    }
}

#[inline]
fn b01(x: bool) -> f32 {
    x as u8 as f32
}

fn boost_scalar(s: i8, acc_eva: bool) -> f32 {
    let s = s.clamp(-6, 6) as f32;
    let d = if acc_eva { 3.0 } else { 2.0 };
    let m = if s >= 0.0 { (d + s) / d } else { d / (d - s) };
    clip(m.log2() / 2.0, -1.0, 1.0)
}

fn vol(a: &Active, c: Cond) -> Option<&Vol> {
    a.vols.iter().flatten().find(|v| v.cond == c)
}

/// Natures for the speed bounds: (minus speed, plus speed).
fn speed_natures() -> (u8, u8) {
    static N: std::sync::OnceLock<(u8, u8)> = std::sync::OnceLock::new();
    *N.get_or_init(|| {
        let lo = NATURES.iter().position(|n| n.minus == Some(Stat::Spe)).unwrap_or(0) as u8;
        let hi = NATURES.iter().position(|n| n.plus == Some(Stat::Spe)).unwrap_or(0) as u8;
        (lo, hi)
    })
}

/// Floats of a move slot that depend on the move alone (F7-1 to F7-6), one entry per MoveId.
const FACTS: usize = F7_6_3 + 1;

fn move_facts(mv: MoveId) -> &'static [f32; FACTS] {
    static T: std::sync::OnceLock<Vec<[f32; FACTS]>> = std::sync::OnceLock::new();
    let t = T.get_or_init(|| {
        (0..MOVES.len())
            .map(|i| {
                let mv = MoveId(i as u16);
                let d = mv.data();
                let mut s = [0.0; FACTS];
                if i == 0 {
                    return s;
                }
                if d.ty.is_real() {
                    s[F7_1 + d.ty as usize] = 1.0;
                }
                s[F7_2 + d.category as usize] = 1.0;
                s[F7_3] = clip(d.base_power as f32 * dex_hits(mv) / 150.0, 0.0, 2.0);
                s[F7_4] = if d.never_misses() { 1.0 } else { clip(d.accuracy as f32 / 100.0, 0.0, 1.0) };
                s[F7_5] = clip(d.priority as f32 / 5.0, -1.5, 1.5);
                s[F7_6_1] = b01(is_pivot(mv));
                s[F7_6_2] = b01(is_setup(mv));
                s[F7_6_3] = b01(is_heal_or_drain(mv));
                s
            })
            .collect()
    });
    &t[mv.0 as usize]
}

/// Index of a Pokémon in the per-Pokémon arrays (own 0..6, opponent 6..12).
#[inline]
fn key(w: Who) -> usize {
    w.side.idx() * 6 + w.slot as usize % 6
}

/// Damage estimates for one call: each row's moves against the opposing active, and the opposing
/// active's best hit on each row.
#[derive(Clone, Copy, Default)]
struct Damage {
    /// by row: per move slot
    out: [[Option<Hit>; 4]; N_ROWS],
    /// by row: (best expected share, best max share) of the row's current HP
    incoming: [(f32, f32); N_ROWS],
}

struct Ctx<'a> {
    b: &'a Battle,
    kind: ReqKind,
    legal: Legal,
    field: FieldView,
    rows: [Option<Who>; N_ROWS],
    act: [Option<Who>; 2],
    /// by `key`
    fighters: [Option<Fighter>; 12],
    /// by `key`: (lowest, highest) effective speed
    speeds: [Option<(f32, f32)>; 12],
}

impl<'a> Ctx<'a> {
    fn new(b: &'a Battle, kind: ReqKind) -> Ctx<'a> {
        let rows = row_order(b);
        let mut fighters = [None; 12];
        for w in rows.iter().flatten() {
            if known_form(b, *w).is_some() {
                fighters[key(*w)] = Some(Fighter::of(b, *w, false));
            }
        }
        let mut c = Ctx {
            b,
            kind,
            legal: legal(b, kind),
            field: FieldView::of(b),
            rows,
            act: [b.active(Side::Own), b.active(Side::Opp)],
            fighters,
            speeds: [None; 12],
        };
        for w in rows.iter().flatten() {
            c.speeds[key(*w)] = c.speed_range(*w);
        }
        c
    }

    fn fighter(&self, w: Who) -> Option<&Fighter> {
        self.fighters[key(w)].as_ref()
    }

    /// The Pokémon on the field facing a side's Pokémon.
    fn foe(&self, s: Side) -> Option<Who> {
        self.act[s.other().idx()].filter(|w| !self.b.mon(*w).fainted)
    }

    fn damage(&self) -> Damage {
        let b = self.b;
        let mut d = Damage::default();
        for (r, row) in self.rows.iter().enumerate() {
            let Some(w) = *row else { continue };
            let Some(foe) = self.foe(w.side) else { continue };
            let (Some(a), Some(t)) = (self.fighter(w), self.fighter(foe)) else { continue };
            if b.mon(w).fainted {
                continue;
            }
            for (j, mv) in row_moves(b, w).iter().enumerate() {
                if let Some(mv) = mv {
                    d.out[r][j] = Some(estimate(&self.field, a, *mv, t, Some(b)));
                }
            }
        }
        for (r, row) in self.rows.iter().enumerate() {
            let Some(w) = *row else { continue };
            let Some(foe) = self.foe(w.side) else { continue };
            let hp = b.mon(w).hp / 100.0;
            if hp <= 0.0 {
                continue;
            }
            let (Some(a), Some(t)) = (self.fighter(foe), self.fighter(w)) else { continue };
            // the foe's own row already holds its hits on the Pokémon facing it
            let cached = self.act[w.side.idx()] == Some(w);
            let fr = self.rows.iter().position(|x| *x == Some(foe));
            let mut best = (0.0f32, 0.0f32);
            for (j, mv) in row_moves(b, foe).iter().enumerate() {
                let Some(mv) = mv else { continue };
                let h = match (cached, fr) {
                    (true, Some(fr)) => d.out[fr][j].unwrap_or_default(),
                    _ => estimate(&self.field, a, *mv, t, Some(b)),
                };
                best.0 = best.0.max(h.expected() / hp);
                best.1 = best.1.max(h.max_frac() / hp);
            }
            d.incoming[r] = (clip(best.0, 0.0, 2.0), clip(best.1, 0.0, 2.0));
        }
        d
    }

    /// (lowest, highest) effective speed: ours exact; theirs from no points and a lowering nature to all
    /// points and a raising nature, the top x1.5 while no item is known.
    fn speed_range(&self, w: Who) -> Option<(f32, f32)> {
        let f = self.fighter(w)?;
        if w.side == Side::Own {
            let s = speed(f, &self.field) as f32;
            return Some((s, s));
        }
        let base = f.form.data().base_stats[5];
        let (minus, plus) = speed_natures();
        let mut lo = *f;
        lo.stats[5] = stat_from_points(base, 0, Stat::Spe, minus);
        let mut hi = *f;
        hi.stats[5] = stat_from_points(base, 32, Stat::Spe, plus);
        let mut top = speed(&hi, &self.field) as f32;
        if f.item.is_none() && !f.no_item {
            top *= 1.5;
        }
        Some((speed(&lo, &self.field) as f32, top))
    }

    fn row(&self, r: usize, f: &mut [f32], ids: &mut [i32], dmg: &Damage) {
        let b = self.b;
        let Some(w) = self.rows[r] else { return };
        let m = b.mon(w);
        let own = w.side == Side::Own;
        let side = b.side(w.side);
        let a = &side.active;
        let active = self.act[w.side.idx()] == Some(w);
        f[F2_5] = b01(own);
        f[F3_1] = clip(m.hp / 100.0, 0.0, 1.0);
        let seen_opp = b.sides[1].team.iter().filter(|t| t.seen).count();
        if !own {
            f[F4_2] = b01(!m.seen && seen_opp >= 3);
        }
        let Some(form) = known_form(b, w) else { return };

        // identity
        let types = if active { b.types(w).unwrap_or(form.data().types) } else { form.data().types };
        for t in types {
            if t.is_real() {
                f[F1_1 + t as usize] = 1.0;
            }
        }
        let base = form.data().base_stats;
        for i in 0..6 {
            f[F1_2_1 + i] = clip(base[i] as f32 / 255.0, 0.0, 1.0);
        }
        ids[F1_3] = vocab::species(m.form.or(m.species).or(m.preview));

        // build
        f[F2_1] = b01(own || m.item != ItemState::Unknown);
        f[F2_2] = b01(matches!(m.item, ItemState::Gone(_)));
        f[F2_3] = b01(own || m.ability.is_some());
        if own {
            let slot = w.slot as usize;
            let st = own_form_stats(b, slot, m.form.unwrap_or(b.sets[slot].species));
            for i in 0..6 {
                f[F2_4_1 + i] = clip(st[i] as f32 / 400.0, 0.0, 2.0);
            }
        }
        ids[F2_6] = vocab::item(m.item);
        ids[F2_7] = vocab::ability(m.ability);

        // condition
        let st = match m.status {
            Status::Brn => Some(0),
            Status::Frz => Some(1),
            Status::Par => Some(2),
            Status::Psn => Some(3),
            Status::Slp => Some(4),
            Status::Tox => Some(5),
            Status::None => None,
        };
        if let Some(i) = st {
            f[F3_2 + i] = 1.0;
        }
        f[F3_3] = match m.status {
            Status::Slp => clip(m.status_count as f32 / 5.0, 0.0, 1.0),
            Status::Tox if active => clip(b.turn.saturating_sub(m.status_start.max(a.entered)) as f32 / 5.0, 0.0, 1.0),
            _ => 0.0,
        };
        if active {
            for i in 0..N_BOOSTS {
                f[F3_4_1 + i] = boost_scalar(a.boosts[i], i >= 5);
            }
            vols::write(b, w, a, &mut f[F3_5..F3_5 + vols::N_VOLS]);
            f[F3_6_1] = b01(a.move_actions == 0);
            f[F3_6_2] = b01(matches!(a.lock, Lock::Charging { .. }));
            f[F3_6_3] = b01(matches!(a.lock, Lock::Recharge { .. }));
            f[F3_7] = clip(b.track.sides[w.side.idx()].stall_streak as f32 / 3.0, 0.0, 1.0);
            f[F3_8] = clip(b.turn.saturating_sub(a.entered) as f32 / 10.0, 0.0, 1.0);
            if let Some(v) = vol(a, conds::ENCORE) {
                f[F9_1_1] = clip(b.elapsed(v.start) as f32 / 3.0, 0.0, 1.0);
            }
            if let Some(v) = vol(a, conds::TAUNT) {
                f[F9_1_2] = clip(b.elapsed(v.start) as f32 / 3.0, 0.0, 1.0);
            }
        }
        f[F3_9] = b01(active);
        f[F3_11] = b01(m.mega);

        // knowledge
        f[F4_1] = b01(if own { b.picks.contains(&Some(w.slot)) } else { m.seen });
        f[F4_4] = b01(m.was_lead);

        // matchup
        if let Some(foe) = self.foe(w.side) {
            if !m.fainted {
                if let (Some(mine), Some(theirs)) = (self.speeds[key(w)], self.speeds[key(foe)]) {
                    let (faster, slower) = (mine.0 > theirs.1, mine.1 < theirs.0);
                    let (x, y) = if self.field.trick_room { (slower, faster) } else { (faster, slower) };
                    f[F5_1_1] = b01(x);
                    f[F5_1_2] = b01(y);
                }
            }
        }
        f[F5_3_1] = dmg.incoming[r].0;
        f[F5_3_2] = dmg.incoming[r].1;

        // evidence
        let t = &b.track.sides[w.side.idx()];
        f[F6_1] = clip(m.biggest_hit / 100.0, 0.0, 1.0);
        f[F6_2_1] = clip(t.moved[w.slot as usize % 6][0] as f32 / 5.0, 0.0, 1.0);
        f[F6_2_2] = clip(t.moved[w.slot as usize % 6][1] as f32 / 5.0, 0.0, 1.0);
        f[F6_3] = clip(m.entries as f32 / 4.0, 0.0, 1.0);

        // moves
        let moves = row_moves(b, w);
        let pp = b.pp_left(w);
        let transformed = active && a.transformed.is_some();
        let foe_types = self.foe(w.side).and_then(|x| b.types(x));
        let foe_hp = self.foe(w.side).map_or(0.0, |x| b.mon(x).hp / 100.0);
        for (j, mv) in moves.iter().enumerate() {
            let Some(mv) = *mv else { continue };
            let d = mv.data();
            let s = &mut f[MOVE_BASE + MOVE_STRIDE * j..MOVE_BASE + MOVE_STRIDE * (j + 1)];
            s[..FACTS].copy_from_slice(move_facts(mv));
            let max = if transformed { 5 } else { d.max_pp };
            s[F7_7] = match pp[j] {
                Some(left) if max > 0 => clip(left as f32 / max as f32, 0.0, 1.0),
                _ => 0.0,
            };
            if active {
                s[F7_8_1] = b01(vol(a, conds::DISABLE).map_or(false, |v| v.detail == VolDetail::Move(mv)));
                s[F7_8_2] = b01(locked_into(b, w, mv));
            }
            s[F7_8_3] = 1.0;
            if let (Some(ft), true) = (foe_types, d.category != Category::Status) {
                s[F7_9] = clip(type_multiplier(d.ty, ft) / 4.0, 0.0, 1.0);
            }
            if let Some(h) = dmg.out[r][j] {
                let (mean, top) = (h.expected(), h.max_frac());
                s[F7_10_1] = clip(mean, 0.0, 2.0);
                s[F7_10_2] = clip(top, 0.0, 2.0);
                s[F7_11_1] = b01(foe_hp > 0.0 && mean >= foe_hp);
                s[F7_11_2] = b01(foe_hp > 0.0 && top >= foe_hp);
            }
            ids[F7_13 + j] = vocab::mv(Some(mv));
        }

        // possible abilities: ours the held one; theirs the form's list
        if own {
            ids[F8_10] = vocab::ability(m.ability);
        } else {
            let mut k = 0;
            for ab in form.data().abilities {
                if ab.0 != 0 && k < 3 && !ids[F8_10..F8_10 + k].contains(&vocab::ability(Some(ab))) {
                    ids[F8_10 + k] = vocab::ability(Some(ab));
                    k += 1;
                }
            }
        }
    }

    fn field(&self, f: &mut [f32]) {
        let b = self.b;
        f[G1_1] = clip(b.turn as f32 / 30.0, 0.0, 1.0);
        f[G1_2_1] = b01(self.kind == ReqKind::Replace);
        f[G1_2_2] = b01(self.legal.trapped);
        let fainted = |s: usize| b.sides[s].team.iter().filter(|t| t.fainted).count() as f32;
        f[G1_4_1] = clip((3.0 - fainted(0)) / 3.0, 0.0, 1.0);
        f[G1_4_2] = clip((3.0 - fainted(1)) / 3.0, 0.0, 1.0);
        f[G1_5] = clip(b.sides[1].team.iter().filter(|t| t.seen).count() as f32 / 3.0, 0.0, 1.0);
        let mask = &self.legal.mask;
        f[G1_7_1] = mask[..4].iter().filter(|x| **x).count() as f32 / 4.0;
        f[G1_7_2] = clip(mask[8..].iter().filter(|x| **x).count() as f32 / 2.0, 0.0, 1.0);
        if let Some(w) = b.field.weather {
            let k = match w.cond.id() {
                "sunnyday" | "desolateland" => Some(0),
                "raindance" | "primordialsea" => Some(1),
                "sandstorm" => Some(2),
                "snowscape" | "hail" => Some(3),
                _ => None,
            };
            if let Some(k) = k {
                f[G2_1 + k] = 1.0;
            }
            f[G2_2] = b01(matches!(w.cond.id(), "desolateland" | "primordialsea" | "deltastream"));
            f[G9_1] = clip(b.elapsed(w.start) as f32 / 8.0, 0.0, 1.0);
        }
        if let Some(t) = b.field.terrain {
            let k = match t.cond.id() {
                "electricterrain" => Some(0),
                "grassyterrain" => Some(1),
                "mistyterrain" => Some(2),
                "psychicterrain" => Some(3),
                _ => None,
            };
            if let Some(k) = k {
                f[G2_4 + k] = 1.0;
            }
            f[G2_5] = clip(b.elapsed(t.start) as f32 / 8.0, 0.0, 1.0);
        }
        // ROOMS order: trickroom, magicroom, wonderroom, gravity, fairylock
        let rooms = &b.field.rooms;
        f[G2_6_1] = b01(rooms[0].is_some());
        f[G2_6_2] = b01(rooms[R_GRAVITY].is_some());
        f[G2_6_3] = b01(rooms[R_MAGICROOM].is_some());
        f[G2_6_4] = b01(rooms[2].is_some());
        if let Some(t) = rooms[0] {
            f[G2_7] = clip(b.elapsed(t.start) as f32 / 5.0, 0.0, 1.0);
        }
        // SIDE_CONDS order: reflect, lightscreen, auroraveil, tailwind, safeguard, spikes, toxicspikes,
        // stealthrock, stickyweb, quickguard; Mist is not in Champions
        for (s, (flags, timed)) in [(G3_1_1, G3_2_1), (G3_3_1, G3_4_1)].into_iter().enumerate() {
            let c = &b.sides[s].conds;
            for i in 0..5 {
                f[flags + i] = b01(c[i].is_some());
            }
            f[flags + 6] = b01(c[7].is_some());
            f[flags + 7] = c[5].map_or(0.0, |x| clip(x.layers as f32 / 3.0, 0.0, 1.0));
            f[flags + 8] = c[6].map_or(0.0, |x| clip(x.layers as f32 / 2.0, 0.0, 1.0));
            f[flags + 9] = b01(c[8].is_some());
            for (i, d) in [8.0, 8.0, 8.0, 4.0, 5.0].into_iter().enumerate() {
                if let Some(x) = c[i] {
                    f[timed + i] = clip(b.elapsed(x.start) as f32 / d, 0.0, 1.0);
                }
            }
        }
        for (s, at) in [(0, G6_1_1), (1, G6_2_1)] {
            let t = &b.track.sides[s];
            let n = t.turns.max(1) as f32;
            f[at] = clip(t.switches as f32 / n, 0.0, 1.0);
            f[at + 1] = clip(t.protects as f32 / n, 0.0, 1.0);
            f[at + 2] = clip(t.setups as f32 / n, 0.0, 1.0);
            f[at + 3] = clip(t.attacks as f32 / n, 0.0, 1.0);
        }
        // pending on a side: its own Wish; the other side's Future Sight
        for (s, (wish, fs)) in [(0, (G9_2_1, G9_2_2)), (1, (G9_3_1, G9_3_2))] {
            let t = &b.track.sides[s];
            f[wish] = b01(t.wish_turn.map_or(false, |x| x.saturating_add(1) == b.turn));
            if let Some(x) = b.track.sides[1 - s].future_turn {
                let lands = x.saturating_add(2);
                if lands >= b.turn {
                    f[fs] = clip((lands - b.turn + 1) as f32 / 2.0, 0.0, 1.0);
                }
            }
        }
    }

    fn last(&self, f: &mut [f32], ids: &mut [i32]) {
        let b = self.b;
        for (s, (act, first, taken)) in [(0, (L1_1, L1_2_2, L1_3_1)), (1, (L1_4, L1_5_2, L1_6_1))] {
            let r = &b.sides[s].last;
            let o = &b.sides[1 - s].last;
            let k = match r.act {
                Act::Move | Act::Pivot => 0,
                Act::Switch => 1,
                Act::None | Act::Cant => 2,
            };
            f[act + k] = 1.0;
            let moving = |x: &TurnRecord| matches!(x.act, Act::Move | Act::Pivot) && x.order > 0;
            f[first] = b01(moving(r) && moving(o) && r.order < o.order);
            f[taken] = clip(o.damage / 100.0, 0.0, 2.0);
            let t = &b.track.sides[s];
            f[taken + 1] = clip(t.faints_last as f32 / 3.0, 0.0, 1.0);
            f[taken + 2] = b01(t.item_last);
            f[taken + 3] = b01(t.ability_last);
            let mv = if matches!(r.act, Act::Move | Act::Pivot) { r.mv } else { None };
            let (mi, si) = if s == 0 { (L2_1_1, L2_2_1) } else { (L2_1_2, L2_2_2) };
            ids[mi] = vocab::mv(mv);
            if let Some(to) = r.to {
                let side = if s == 0 { Side::Own } else { Side::Opp };
                let t = b.mon(Who { side, slot: to });
                ids[si] = vocab::species(t.form.or(t.species));
            }
        }
        f[L3_2] = b01(b.turn <= 1);
    }
}

/// Current form for the row's facts: the species it transformed into while active, else its form, else
/// its species, else the preview species.
fn known_form(b: &Battle, w: Who) -> Option<SpeciesId> {
    let m = b.mon(w);
    let a = &b.side(w.side).active;
    if a.slot == Some(w.slot) && !a.out {
        if let Some(t) = a.transformed {
            return Some(t);
        }
    }
    if w.side == Side::Own && w.slot >= b.n_own {
        return None;
    }
    m.form.or(m.species).or(m.preview)
}

/// Move slots of a Pokémon: its copied moves while transformed on the field, else its own (the
/// opponent's: those revealed, in order of first use).
fn row_moves(b: &Battle, w: Who) -> [Option<MoveId>; 4] {
    let a = &b.side(w.side).active;
    if a.slot == Some(w.slot) && !a.out && a.transformed.is_some() {
        a.t_moves
    } else {
        b.mon(w).moves
    }
}

/// The active Pokémon must use `mv` (Encore, a locked or charging move, or a choice lock shown by a
/// known Choice item or Gorilla Tactics).
fn locked_into(b: &Battle, w: Who, mv: MoveId) -> bool {
    let a = &b.side(w.side).active;
    if vol(a, conds::ENCORE).map_or(false, |v| v.detail == VolDetail::Move(mv)) {
        return true;
    }
    match a.lock {
        Lock::Locked { mv: m, .. } if m == mv => return true,
        Lock::Charging { mv: Some(m), .. } if m == mv => return true,
        _ => {}
    }
    if a.choice_lock == Some(mv) {
        let m = b.mon(w);
        let suppressed = b.field.rooms[R_MAGICROOM].is_some() || m.ability.map(|x| x.id()) == Some("klutz");
        let choice = matches!(m.item, ItemState::Held(i) if i.data().is_choice) && !suppressed;
        let gorilla = m.ability.map(|x| x.id()) == Some("gorillatactics");
        return choice || gorilla;
    }
    false
}
