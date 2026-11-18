//! Checks of a seat's Battle against the State: (a) everything the seat can know agrees, (b) the legal
//! sets of the Battle rule and the State rule, by class of difference.

use crate::view::opp_pct;
use cg_engine::sim::log::{vol_start_show, VolShow};
use battle::{legal, Battle, ItemState, ReqKind, Side, Who, ROOMS, SIDE_CONDS};
use cg_engine::sim::state::Side as Side_;
use cg_engine::sim::data::*;
use cg_engine::sim::state::*;
use cg_engine::sim::{Kind, SideOptions};
use std::collections::BTreeMap;

/// Disagreements of one field: a count and the first few examples.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub counts: BTreeMap<String, u64>,
    pub examples: BTreeMap<String, Vec<String>>,
}

impl Tally {
    pub fn add(&mut self, key: &str, example: impl FnOnce() -> String) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
        let ex = self.examples.entry(key.to_string()).or_default();
        if ex.len() < 3 {
            ex.push(example());
        }
    }
    pub fn merge(&mut self, o: &Tally) {
        for (k, v) in &o.counts {
            *self.counts.entry(k.clone()).or_insert(0) += v;
        }
        for (k, v) in &o.examples {
            let ex = self.examples.entry(k.clone()).or_default();
            for x in v {
                if ex.len() < 3 {
                    ex.push(x.clone());
                }
            }
        }
    }
    pub fn total(&self) -> u64 {
        self.counts.values().sum()
    }
    /// Counts other than the `explained:` keys.
    pub fn unexplained(&self) -> u64 {
        self.counts.iter().filter(|(k, _)| !k.starts_with("explained")).map(|(_, v)| v).sum()
    }
}

/// The announced volatiles of the active Pokémon of a side (what the log shows a start row for).
fn state_vols(s: &State, side: usize) -> Vec<Cond> {
    vols_of(&s.sides[side])
}

fn vols_of(sd: &Side_) -> Vec<Cond> {
    // No Retreat's start row is written only when its trap holds (not on a Ghost)
    let trapped = sd.has_vol(conds::TRAPPED);
    let mut v: Vec<Cond> = sd.volatiles().iter().filter(|v| announced(v.id) && (v.id != conds::NORETREAT || trapped)).map(|v| v.id).collect();
    v.sort();
    v
}

/// A volatile whose start the phone writes as a lasting condition (Endure's bracing included; Must
/// Recharge's start is not shown).
fn announced(c: Cond) -> bool {
    (vol_start_show(c) == VolShow::Start && c != conds::MUSTRECHARGE) || c == conds::ENDURE || c == conds::NORETREAT
}

fn battle_vols(b: &Battle, side: Side) -> Vec<Cond> {
    let mut v: Vec<Cond> = b.side(side).active.vols.iter().flatten().map(|v| v.cond).filter(|c| announced(*c)).collect();
    v.sort();
    v
}

/// Turns since a timed effect began, from the State: its full duration less what is left. The duration
/// is the condition's own, or the longer one an item gives (Light Clay, a weather rock, Terrain
/// Extender): the State does not keep which, so both are returned.
fn state_elapsed(c: Cond, left: i8) -> Option<[i32; 2]> {
    let d = c.data().duration as i32;
    if d == 0 || left <= 0 {
        return None;
    }
    let long = match c.id() {
        "reflect" | "lightscreen" | "auroraveil" => 8,
        _ if c.data().kind & (ckind::WEATHER | ckind::TERRAIN) != 0 => 8,
        _ => d,
    };
    Some([d - left as i32, long - left as i32])
}

/// (a): every field the seat can know. `disguise`: the opposing Illusion user on the field and the slot it
/// shows itself as (State slots), from the emitter. `ctx` names the game and decision for the examples.
/// Differences a seat cannot avoid are counted under `explained:` keys (not disagreements).
pub fn agree(b: &Battle, s: &State, seat: usize, disguise: Option<(u8, u8)>, ctx: &str, t: &mut Tally) {
    let own = &s.sides[seat];
    let opp = &s.sides[1 - seat];
    let team_of = |slot: usize| b.picks.get(slot).copied().flatten();
    // after the end of the turn and before the next `turn` row, effects have one more turn behind them
    let res_done = s.mid_turn && !s.queue.as_slice().iter().any(|a| a.kind == ActionKind::Residual);
    let elapsed = |start: u16| (b.turn as i32 + res_done as i32 - start as i32).max(0);
    // own active: a side waiting to send in a replacement has none on the field; a Pokémon that must
    // switch out (a pivot move, Baton Pass, Shed Tail, Emergency Exit, an Eject item) may still be on it
    // until its withdraw or enter row
    let leaving = own.has_active && !own.active_mon().fainted && own.active_mon().switch_flag;
    let st_active = if own.has_active && !own.active_mon().fainted && (own.request != Request::Switch || leaving) { team_of(own.active as usize) } else { None };
    let bt_active = b.active(Side::Own).map(|w| w.slot);
    if st_active != bt_active && !(leaving && bt_active.is_none()) {
        t.add("own_active", || format!("{} state {:?} battle {:?}", ctx, st_active, bt_active));
    }
    for i in 0..TEAM {
        let Some(ti) = team_of(i) else {
            t.add("own_picks", || format!("{} no pick for slot {}", ctx, i));
            continue;
        };
        let m = &own.mons[i];
        let w = Who { side: Side::Own, slot: ti };
        let bm = b.mon(w);
        let pct = m.hp as f32 * 100.0 / m.maxhp.max(1) as f32;
        if (bm.hp - pct).abs() > 0.01 {
            t.add("own_hp", || format!("{} {} state {:.2} battle {:.2}", ctx, m.set_species.id(), pct, bm.hp));
        }
        if bm.fainted != m.fainted {
            t.add("own_fainted", || format!("{} {} state {} battle {}", ctx, m.set_species.id(), m.fainted, bm.fainted));
        }
        if !m.fainted && bm.status != m.status {
            t.add("own_status", || format!("{} {} state {:?} battle {:?}", ctx, m.set_species.id(), m.status, bm.status));
        }
        if !m.transformed {
            let left = b.pp_left(w);
            for k in 0..m.nmoves as usize {
                if left[k] != Some(m.moves[k].pp) {
                    t.add("own_pp", || format!("{} {} {} state {} battle {:?}", ctx, m.set_species.id(), m.moves[k].id.id(), m.moves[k].pp, left[k]));
                }
            }
        }
        item_agrees(bm.item, m, false, ctx, "own_item", t);
        if !m.fainted && !m.transformed && bm.mega != m.species.data().is_mega {
            t.add("own_mega", || format!("{} {} state {} battle {}", ctx, m.set_species.id(), m.species.id(), bm.mega));
        }
        // a Pokémon switching out (Eject Button, a pivot) has its ability reverted by the Battle at the row
        // that announced it, by the State at the switch itself
        let going = leaving && own.active as usize == i;
        if !m.fainted && bm.ability != Some(m.ability) && !(m.transformed || !m.is_active || going) {
            t.add("own_ability", || format!("{} {} state {} battle {:?}", ctx, m.set_species.id(), m.ability.id(), bm.ability.map(|a| a.id())));
        }
    }
    if st_active.is_some() && st_active == bt_active && own.active_mon().transformed {
        // the copied moves and their PP (from the action screen) and the copied ability
        let m = own.active_mon();
        let a = &b.side(Side::Own).active;
        let w = Who { side: Side::Own, slot: bt_active.unwrap() };
        let left = b.pp_left(w);
        for k in 0..m.nmoves as usize {
            if a.t_moves[k] != Some(m.moves[k].id) || left[k] != Some(m.moves[k].pp) {
                // leaving before any action screen since it transformed (Emergency Exit, Eject Button
                // right after an Imposter entry): the copied moves were never shown
                let this_turn = a.entered == b.turn || b.side(Side::Own).now.mv == Some(moves::TRANSFORM);
                let key = if leaving && this_turn { "explained:own_transform_moves_unshown" } else { "own_transform_moves" };
                t.add(key, || format!("{} {} state {} {} battle {:?} {:?}", ctx, m.set_species.id(), m.moves[k].id.id(), m.moves[k].pp, a.t_moves[k].map(|x| x.id()), left[k]));
            }
        }
    }
    if st_active.is_some() && st_active == bt_active {
        let m = own.active_mon();
        if b.side(Side::Own).active.boosts != m.boosts {
            t.add("own_boosts", || format!("{} state {:?} battle {:?}", ctx, m.boosts, b.side(Side::Own).active.boosts));
        }
        let (sv, bv) = (state_vols(s, seat), battle_vols(b, Side::Own));
        if sv != bv {
            t.add("own_volatiles", || format!("{} state {:?} battle {:?}", ctx, ids(&sv), ids(&bv)));
        }
    }
    side_agrees(b, opp, 1 - seat, s, disguise, ctx, t);
    let _ = elapsed;
    field_agrees(b, s, seat, &elapsed, ctx, t);
}

/// An item as the Battle holds it never contradicts the State. `opp`: unknown is allowed.
fn item_agrees(bi: ItemState, m: &Mon, opp: bool, ctx: &str, key: &str, t: &mut Tally) {
    let ok = match bi {
        ItemState::Unknown => opp,
        ItemState::Held(i) => m.item == i,
        ItemState::Gone(_) => m.item.is_none(),
    };
    if !ok {
        t.add(key, || format!("{} {} state {} battle {:?}", ctx, m.set_species.id(), m.item.id(), bi));
    }
}

/// The opponent: the active Pokémon, and of every Pokémon seen its HP, status, Mega state, and the
/// ability, item and moves shown, which never contradict the State.
fn side_agrees(b: &Battle, opp: &Side_, oside: usize, s: &State, disguise: Option<(u8, u8)>, ctx: &str, t: &mut Tally) {
    let leaving = opp.has_active && !opp.active_mon().fainted && opp.active_mon().switch_flag;
    let o_active = if opp.has_active && !opp.active_mon().fainted && (opp.request != Request::Switch || leaving) { Some(opp.active as usize) } else { None };
    let bo = b.active(Side::Opp);
    let bo_sp = bo.and_then(|w| b.mon(w).species);
    // under Illusion the seat sees the Pokémon shown
    let shown = |slot: usize| match disguise {
        Some((real, sh)) if real as usize == slot => sh as usize,
        _ => slot,
    };
    let expect = o_active.map(|x| opp.mons[shown(x)].set_species);
    if expect != bo_sp && !(leaving && bo.is_none()) {
        t.add("opp_active", || format!("{} state {:?} battle {:?}", ctx, expect.map(|m| m.id()), bo_sp.map(|x| x.id())));
    }
    let _ = (oside, s);
    for i in 0..TEAM {
        let m = &opp.mons[i];
        // only Pokémon the seat has seen; an Illusion user and the Pokémon it showed are what the seat
        // was shown, not the State (explained)
        let Some(slot) = (0..6).find(|&k| b.sides[1].team[k].species == Some(m.set_species)) else { continue };
        let bm = &b.sides[1].team[slot];
        if !bm.seen {
            continue;
        }
        // an opponent with Illusion makes what was shown about any of its Pokémon unreliable
        let illusion = opp.mons.iter().any(|x| x.base_ability == abilities::ILLUSION);
        let mut bad = Vec::new();
        let pct = opp_pct(m.hp, m.maxhp) as f32;
        if (bm.hp - pct).abs() > 0.01 {
            // Regenerator's silent heal is a third of a max HP the seat does not know: an estimate
            // the estimate adds a third to a whole percent: off by up to one percent plus a third's rounding
            let tol = 1.0 + 100.0 / m.maxhp.max(1) as f32 + 0.01;
            let regen = m.ability == abilities::REGENERATOR && (bm.hp - pct).abs() <= tol;
            // Regenerator never shown: the heal is silent
            let can_regen = m.set_species.data().abilities.contains(&abilities::REGENERATOR) || m.ability == abilities::REGENERATOR;
            // (it has Regenerator in truth, and the Battle did not know it when it left)
            let has_regen = m.ability == abilities::REGENERATOR || m.base_ability == abilities::REGENERATOR || (m.species.data().is_mega && m.set_species.data().abilities.contains(&abilities::REGENERATOR));
            let hidden = can_regen && has_regen && (bm.base_ability.map_or(true, |a| a == abilities::REGENERATOR) || bm.mega) && pct > bm.hp;
            // a silent heal the Battle estimated (an HP row gives whole percents)
            let estimate = bm.hp_estimated && (bm.hp - pct).abs() <= tol;
            let k = if regen || estimate {
                "explained:regenerator_estimate"
            } else if hidden {
                "explained:hidden_regenerator"
            } else {
                "opp_hp"
            };
            bad.push((k, format!("{} {} state {} battle {}", ctx, m.set_species.id(), pct, bm.hp)));
        }
        if bm.fainted != m.fainted {
            bad.push(("opp_fainted", format!("{} {}", ctx, m.set_species.id())));
        }
        if !m.fainted && bm.status != m.status {
            // Natural Cure on switching out, with the ability never shown
            // (a Mega no longer holds the ability it had when it left)
            let nc = m.ability == abilities::NATURALCURE || m.base_ability == abilities::NATURALCURE || (m.species.data().is_mega && m.set_species.data().abilities.contains(&abilities::NATURALCURE));
            let k = if nc && (bm.base_ability.map_or(true, |a| a == abilities::NATURALCURE) || bm.mega) && m.status == Status::None { "explained:hidden_natural_cure" } else { "opp_status" };
            bad.push((k, format!("{} {} state {:?} battle {:?}", ctx, m.set_species.id(), m.status, bm.status)));
        }
        if !m.fainted && !m.transformed && bm.mega != m.species.data().is_mega {
            bad.push(("opp_mega", format!("{} {} state {} battle {}", ctx, m.set_species.id(), m.species.id(), bm.mega)));
        }
        if let Some(a) = bm.ability {
            if !m.fainted && a != m.ability && !m.transformed && !(leaving && opp.active as usize == i) {
                bad.push(("opp_ability", format!("{} {} state {} battle {}", ctx, m.set_species.id(), m.ability.id(), a.id())));
            }
        }
        if let Some(a) = bm.base_ability {
            // a Mega's ability replaces the State's base ability
            if a != m.base_ability && !m.species.data().is_mega {
                bad.push(("opp_base_ability", format!("{} {} state {} battle {}", ctx, m.set_species.id(), m.base_ability.id(), a.id())));
            }
        }
        let item_ok = match bm.item {
            ItemState::Unknown => true,
            ItemState::Held(it) => m.item == it,
            ItemState::Gone(_) => m.item.is_none(),
        };
        if !item_ok {
            bad.push(("opp_item", format!("{} {} state {} battle {:?}", ctx, m.set_species.id(), m.item.id(), bm.item)));
        }
        for mv in bm.moves.iter().flatten() {
            // its own moves (a transformed Pokémon's count is the copy's)
            if !m.base_moves.iter().any(|x| x.id == *mv && !x.id.is_none()) {
                bad.push(("opp_moves", format!("{} {} {} not among its moves", ctx, m.set_species.id(), mv.id())));
            }
        }
        if !m.transformed && !(opp.has_active && opp.active as usize == i && opp.active_mon().transformed) {
            let w = Who { side: Side::Opp, slot: slot as u8 };
            let left = b.pp_left(w);
            for k in 0..4 {
                if let (Some(mv), Some(l)) = (bm.moves[k], left[k]) {
                    if let Some(ms) = m.moves[..m.nmoves as usize].iter().find(|x| x.id == mv) {
                        if ms.pp != l {
                            // a lock that ended while the Pokémon was already confused shows no fatigue row: its
                            // next use looks like the lock going on
                            let lock = matches!(b.side(Side::Opp).active.lock, battle::Lock::Locked { mv: x, .. } if x == mv);
                            let unseen = (lock && opp.active as usize == i && opp.has_vol(conds::CONFUSION)) || bm.pp_uncertain;
                            let k = if unseen { "explained:lock_end_unseen" } else { "opp_pp" };
                            bad.push((k, format!("{} {} {} state {} battle {}", ctx, m.set_species.id(), mv.id(), ms.pp, l)));
                        }
                    }
                }
            }
        }
        for (k, x) in bad {
            t.add(if illusion && !k.starts_with("explained") { "explained:illusion" } else { k }, || x);
        }
    }
    if let (Some(x), Some(_)) = (o_active, bo) {
        let m = &opp.mons[x];
        if Some(opp.mons[shown(x)].set_species) == bo_sp {
            if b.side(Side::Opp).active.boosts != m.boosts {
                t.add("opp_boosts", || format!("{} state {:?} battle {:?}", ctx, m.boosts, b.side(Side::Opp).active.boosts));
            }
            let (sv, bv) = (vols_of(opp), battle_vols(b, Side::Opp));
            if sv != bv {
                // Charge ends silently when the move it could not use was Electric: the opponent's choice
                // is not shown
                let extra: Vec<Cond> = bv.iter().copied().filter(|c| !sv.contains(c)).collect();
                let maybe = b.side(Side::Opp).active.vols.iter().flatten().any(|v| v.cond == conds::CHARGE && v.maybe_gone);
                let aborted = extra == [conds::CHARGE] && sv.iter().all(|c| bv.contains(c)) && maybe;
                let k = if aborted {
                    "explained:charge_aborted_hidden"
                } else {
                    "opp_volatiles"
                };
                t.add(k, || format!("{} state {:?} battle {:?}", ctx, ids(&sv), ids(&bv)));
            }
        }
    }
}

/// Field, weather, terrain, rooms and both sides' conditions, with the turns since each began.
fn field_agrees(b: &Battle, s: &State, seat: usize, elapsed: &dyn Fn(u16) -> i32, ctx: &str, t: &mut Tally) {
    // field
    let w = s.field.weather;
    let bw = b.field.weather;
    if (!w.id.is_none()).then_some(w.id) != bw.map(|x| x.cond) {
        t.add("weather", || format!("{} state {} battle {:?}", ctx, w.id.id(), bw.map(|x| x.cond.id())));
    } else if let (Some(e), Some(x)) = (state_elapsed(w.id, w.duration), bw) {
        if !e.contains(&elapsed(x.start)) {
            t.add("weather_turns", || format!("{} {} state {:?} (left {}) battle {} (start {} now {})", ctx, w.id.id(), e, w.duration, b.elapsed(x.start), x.start, b.turn));
        }
    }
    let tr = s.field.terrain;
    let bt = b.field.terrain;
    if (!tr.id.is_none()).then_some(tr.id) != bt.map(|x| x.cond) {
        t.add("terrain", || format!("{} state {} battle {:?}", ctx, tr.id.id(), bt.map(|x| x.cond.id())));
    } else if let (Some(e), Some(x)) = (state_elapsed(tr.id, tr.duration), bt) {
        if !e.contains(&elapsed(x.start)) {
            t.add("terrain_turns", || format!("{} {} state {:?} battle {}", ctx, tr.id.id(), e, b.elapsed(x.start)));
        }
    }
    for (i, r) in ROOMS.iter().enumerate() {
        let st = s.field.pseudo[..s.field.npseudo as usize].iter().find(|c| c.id.id() == *r);
        let bt = b.field.rooms[i];
        if st.is_some() != bt.is_some() {
            t.add("rooms", || format!("{} {} state {} battle {}", ctx, r, st.is_some(), bt.is_some()));
        } else if let (Some(c), Some(x)) = (st, bt) {
            if let Some(e) = state_elapsed(c.id, c.duration) {
                if !e.contains(&elapsed(x.start)) {
                    t.add("room_turns", || format!("{} {} state {:?} battle {}", ctx, r, e, b.elapsed(x.start)));
                }
            }
        }
    }
    for (side, bs) in [(seat, Side::Own), (1 - seat, Side::Opp)] {
        for (i, id) in SIDE_CONDS.iter().enumerate() {
            let st = s.sides[side].conds().iter().find(|c| c.id.id() == *id);
            let bc = b.side(bs).conds[i];
            if st.is_some() != bc.is_some() {
                t.add("side_conds", || format!("{} {} {} state {} battle {}", ctx, bs.id(), id, st.is_some(), bc.is_some()));
                continue;
            }
            if let (Some(c), Some(x)) = (st, bc) {
                let hazard = matches!(*id, "spikes" | "toxicspikes" | "stealthrock" | "stickyweb");
                if hazard && c.layers.max(1) != x.layers {
                    t.add("side_layers", || format!("{} {} {} state {} battle {}", ctx, bs.id(), id, c.layers, x.layers));
                }
                if let Some(e) = state_elapsed(c.id, c.duration) {
                    if !hazard && !e.contains(&elapsed(x.start)) {
                        t.add("side_turns", || format!("{} {} {} state {:?} battle {}", ctx, bs.id(), id, e, b.elapsed(x.start)));
                    }
                }
            }
        }
    }
}

fn ids(v: &[Cond]) -> Vec<&'static str> {
    v.iter().map(|c| c.id()).collect()
}

/// The State rule's actions in the Battle's layout.
pub fn state_mask(o: &SideOptions) -> [bool; battle::N_ACTIONS] {
    let mut m = [false; battle::N_ACTIONS];
    if o.kind == Kind::Turn {
        if o.struggle || o.forced.is_some() {
            m[0] = true;
        } else {
            for i in 0..4 {
                m[i] = o.moves[i];
                m[4 + i] = o.moves[i] && o.mega;
            }
        }
    }
    if matches!(o.kind, Kind::Turn | Kind::Replace) {
        for i in 0..TEAM {
            m[8 + i] = o.switches[i];
        }
    }
    m
}

/// Why the State forbids a move slot.
fn state_reason(s: &State, side: usize, o: &SideOptions, i: usize) -> &'static str {
    let sd = &s.sides[side];
    let m = sd.active_mon();
    if o.struggle {
        return "struggle";
    }
    if o.forced.is_some() || s.locked_move(side).is_some() {
        return "locked";
    }
    if i >= m.nmoves as usize {
        return "no_move";
    }
    if m.moves[i].pp == 0 {
        return "no_pp";
    }
    let mv = m.moves[i].id;
    for v in sd.volatiles() {
        let hit = match v.id.id() {
            "disable" => v.mv == mv,
            "encore" => v.mv != mv,
            "taunt" => mv.data().category == Category::Status,
            "torment" => m.last_move == mv,
            "healblock" => mv.data().has_flag(flag::HEAL),
            "throatchop" => mv.data().has_flag(flag::SOUND),
            _ => false,
        };
        if hit {
            return v.id.id();
        }
    }
    if mv.data().has_flag(flag::CANTUSETWICE) && m.last_move == mv {
        return "cant_use_twice";
    }
    if (mv == moves::FAKEOUT || mv == moves::FIRSTIMPRESSION) && m.active_move_actions > 0 {
        return "first_turn_only";
    }
    "disabled_other"
}

/// Why a `replace` request asked before the rows of its turn offers a switch the State refuses: the
/// Pokémon fainted in the unread rows, the request is a Revival Blessing choice of a fainted one, the
/// Battle offered a fainted one because our order was Revival Blessing and this is a plain replacement, or
/// the Pokémon is the one on the field.
fn stale_class(b: &Battle, s: &State, seat: usize, p: usize) -> Option<&'static str> {
    let sd = &s.sides[seat];
    let known_fainted = b.picks[p].map_or(false, |x| b.sides[0].team[x as usize].fainted);
    if known_fainted && b.ctx.own_choice == Some(moves::REVIVALBLESSING) {
        Some("stale_revival_guess")
    } else if sd.slot_conds().iter().any(|c| c.id == conds::REVIVALBLESSING) {
        Some("stale_revival")
    } else if sd.mons[p].fainted {
        Some("stale_fainted")
    } else if sd.has_active && sd.active as usize == p {
        // it is the one on the field: dragged in (Red Card, Roar, Dragon Tail) in the unread rows
        Some("stale_on_field")
    } else {
        None
    }
}

/// (b): the differences of the two legal sets at one decision, as classes.
pub fn legal_classes(b: &Battle, s: &State, seat: usize, kind: ReqKind, o: &SideOptions) -> Vec<String> {
    let bl = legal(b, kind);
    let sm = state_mask(o);
    let mut out = Vec::new();
    for i in 0..battle::N_ACTIONS {
        if bl.mask[i] == sm[i] {
            continue;
        }
        let what = if i < 4 { "move" } else if i < 8 { "mega" } else { "switch" };
        let stale = if i >= 8 && bl.mask[i] && kind == ReqKind::Replace && b.ctx.unread { stale_class(b, s, seat, i - 8) } else { None };
        let k = if let Some(r) = stale {
            format!("explained:battle_only:switch:{}", r)
        } else if bl.mask[i] {
            // the Battle allows, the State refuses
            let r = if i < 8 {
                if i >= 4 && !o.mega {
                    "no_mega"
                } else {
                    state_reason(s, seat, o, i % 4)
                }
            } else if o.kind == Kind::Turn && o.switches.iter().all(|x| !x) {
                "state_trapped"
            } else {
                "switch_target"
            };
            // the imprisoner's moves are known only as far as shown; a trapping ability that writes no row
            // is known once the game refuses a switch
            let foe = &s.sides[1 - seat];
            let trap_ab = foe.has_active && matches!(foe.active_mon().ability.id(), "shadowtag" | "arenatrap" | "magnetpull");
            let foe_ab = b.active(Side::Opp).and_then(|w| b.mon(w).ability);
            let hidden_trap = r == "state_trapped" && trap_ab && foe_ab != Some(foe.active_mon().ability);
            let hidden = (i < 8 && b.side(Side::Opp).active.vols.iter().flatten().any(|v| v.cond == conds::IMPRISON)) || hidden_trap;
            format!("{}battle_only:{}:{}", if hidden { "explained:" } else { "" }, what, r)
        } else {
            let r = if i < 8 {
                if bl.forced.is_some() {
                    "battle_forced"
                } else if i >= 4 && !bl.can_mega {
                    "battle_no_mega"
                } else {
                    battle::legal::move_blocked(b, i % 4).unwrap_or("other")
                }
            } else if bl.trapped {
                "battle_trapped"
            } else if b.refused[i] {
                "refused"
            } else {
                "switch_target"
            };
            // the State's Struggle under an Imprison of moves the Battle has not seen (the same cause
            // as the battle_only moves above)
            let hidden = i == 0 && o.struggle && b.side(Side::Opp).active.vols.iter().flatten().any(|v| v.cond == conds::IMPRISON);
            // Throat Chop assumed to hold, from a user whose Sheer Force was never shown
            let foe = &s.sides[1 - seat];
            let sheer = r == "Throat Chop" && foe.has_active && foe.active_mon().ability.id() == "sheerforce" && b.active(Side::Opp).and_then(|w| b.mon(w).ability) != Some(foe.active_mon().ability);
            let hidden = hidden || sheer;
            // a replace request before the rows: the Pokémon the Battle takes to be on the field was dragged
            // out (Red Card, Roar, Dragon Tail) in the unread rows
            let dragged = i >= 8 && kind == ReqKind::Replace && b.ctx.unread && b.picks[i - 8].is_some() && b.picks[i - 8] == battle::legal::on_field(b);
            let r = if dragged { "stale_dragged" } else { r };
            format!("{}state_only:{}:{}", if hidden || dragged { "explained:" } else { "" }, what, r)
        };
        out.push(k);
    }
    out
}
