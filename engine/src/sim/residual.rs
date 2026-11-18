//! The end-of-turn step (Showdown's fieldEvent('Residual')): the residual handlers of every effect in play
//! and every effect with a duration, ordered by Showdown's order table (each handler's onResidualOrder,
//! priority, the holder's speed, sub-order), with speed ties drawn through the outcome source. Durations
//! count down; an effect whose duration reaches 0 ends (its End handler) instead of acting.

use super::battle::Sim;
use super::data::*;
use super::hooks::*;
use super::outcome::*;
use super::state::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Holder {
    Weather,
    Terrain,
    Pseudo(Cond),
    SideCond(u8, Cond),
    SlotCond(u8, Cond),
    Status(u8),
    Volatile(u8, Cond),
    Ability(u8),
    Item(u8),
    /// a field condition's onResidual for one active Pokemon (Grassy Terrain's heal)
    FieldFor(u8),
}

#[derive(Clone, Copy, Debug)]
pub struct Handler {
    pub effect: EffectRef,
    pub holder: Holder,
    /// None sorts last (Showdown's `order: false`)
    pub order: Option<f32>,
    pub priority: f32,
    pub sub_order: f32,
    pub speed: i32,
    pub callback: bool,
    pub duration: bool,
}

fn num(nums: &[(&str, f32)], name: &str) -> Option<f32> {
    nums.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// The handler names of a holder kind: callback, Order, Priority, SubOrder.
const PLAIN: [&str; 4] = ["onResidual", "onResidualOrder", "onResidualPriority", "onResidualSubOrder"];
const SIDE: [&str; 4] = ["onSideResidual", "onSideResidualOrder", "onSideResidualPriority", "onSideResidualSubOrder"];
const FIELD: [&str; 4] = ["onFieldResidual", "onFieldResidualOrder", "onFieldResidualPriority", "onFieldResidualSubOrder"];

fn handler(effect: EffectRef, holder: Holder, names: &[&str; 4], default_sub: f32, speed: i32, duration: bool) -> Option<Handler> {
    let (handlers, nums) = match effect {
        EffectRef::Cond(c) => (c.data().handlers, c.data().nums),
        EffectRef::Ability(a) => (a.data().handlers, a.data().nums),
        EffectRef::Item(i) => (i.data().handlers, i.data().nums),
        EffectRef::Move(m) => (m.data().handlers, &[][..]),
    };
    if handlers.is_empty() && !duration {
        return None;
    }
    let callback = handlers.iter().any(|h| *h == names[0]);
    if !callback && !duration {
        return None;
    }
    let sub = num(nums, names[3]).unwrap_or(default_sub);
    Some(Handler {
        effect,
        holder,
        order: num(nums, names[1]),
        priority: num(nums, names[2]).unwrap_or(0.0),
        sub_order: if sub == 0.0 { default_sub } else { sub },
        speed,
        callback,
        duration,
    })
}

/// Every residual handler in play.
pub fn collect(s: &State) -> Vec<Handler> {
    let mut out = Vec::new();
    let f = &s.field;
    // sub-orders by effect type: a pseudo-weather 5 once its state has a target (2 before), a weather 5,
    // a terrain 0
    for c in f.pseudo[..f.npseudo as usize].iter() {
        let sub = if c.targeted { 5.0 } else { 2.0 };
        out.extend(handler(EffectRef::Cond(c.id), Holder::Pseudo(c.id), &FIELD, sub, 0, c.duration > 0));
    }
    if !f.weather.id.is_none() {
        out.extend(handler(EffectRef::Cond(f.weather.id), Holder::Weather, &FIELD, 5.0, 0, f.weather.duration > 0));
    }
    if !f.terrain.id.is_none() {
        out.extend(handler(EffectRef::Cond(f.terrain.id), Holder::Terrain, &FIELD, 0.0, 0, f.terrain.duration > 0));
    }
    for (si, side) in s.sides.iter().enumerate() {
        for c in side.conds().iter() {
            out.extend(handler(EffectRef::Cond(c.id), Holder::SideCond(si as u8, c.id), &SIDE, 4.0, 0, c.duration > 0));
        }
        if !side.has_active {
            continue;
        }
        let m = side.active_mon();
        let sp = m.speed;
        // a status is effectType Status: sub-order 0
        if let Some(c) = status_cond(m.status) {
            out.extend(handler(EffectRef::Cond(c), Holder::Status(si as u8), &PLAIN, 0.0, sp, false));
        }
        for v in side.volatiles() {
            out.extend(handler(EffectRef::Cond(v.id), Holder::Volatile(si as u8, v.id), &PLAIN, 2.0, sp, v.duration > 0));
        }
        out.extend(handler(EffectRef::Ability(m.ability), Holder::Ability(si as u8), &PLAIN, 7.0, sp, false));
        out.extend(handler(EffectRef::Item(m.item), Holder::Item(si as u8), &PLAIN, 8.0, sp, false));
        for c in side.slot_conds().iter() {
            out.extend(handler(EffectRef::Cond(c.id), Holder::SlotCond(si as u8, c.id), &PLAIN, 3.0, sp, c.duration > 0));
        }
        // findSideEventHandlers and findFieldEventHandlers with the active Pokemon as holder: plain
        // onResidual callbacks only, no duration
        for c in side.conds().iter() {
            if c.id.data().handlers.contains(&"onResidual") {
                out.extend(handler(EffectRef::Cond(c.id), Holder::FieldFor(si as u8), &PLAIN, 4.0, sp, false));
            }
        }
        for c in f.pseudo[..f.npseudo as usize].iter().chain([f.weather, f.terrain].iter()) {
            if !c.id.is_none() && c.id.data().handlers.contains(&"onResidual") {
                out.extend(handler(EffectRef::Cond(c.id), Holder::FieldFor(si as u8), &PLAIN, 0.0, sp, false));
            }
        }
    }
    out
}

fn compare(a: &Handler, b: &Handler) -> f64 {
    let oa = a.order.map(|x| x as f64).unwrap_or(4294967296.0);
    let ob = b.order.map(|x| x as f64).unwrap_or(4294967296.0);
    if oa != ob {
        return oa - ob;
    }
    if a.priority != b.priority {
        return (b.priority - a.priority) as f64;
    }
    if a.speed != b.speed {
        return (b.speed - a.speed) as f64;
    }
    (a.sub_order - b.sub_order) as f64
}

fn holder_side(h: &Holder) -> Option<u8> {
    match *h {
        Holder::SideCond(s, _) | Holder::SlotCond(s, _) | Holder::Status(s) | Holder::Volatile(s, _) => Some(s),
        Holder::Ability(s) | Holder::Item(s) | Holder::FieldFor(s) => Some(s),
        _ => None,
    }
}

/// speedSort with Showdown's selection sort; a tie of two handlers draws a `speedtie` with their ids.
pub fn sort<O: Outcomes>(sim: &mut Sim<'_, O>, hs: &mut [Handler]) {
    let n = hs.len();
    let mut sorted = 0;
    while sorted + 1 < n {
        let mut next = vec![sorted];
        for i in sorted + 1..n {
            let d = compare(&hs[next[0]], &hs[i]);
            if d < 0.0 {
                continue;
            }
            if d > 0.0 {
                next = vec![i];
            } else {
                next.push(i);
            }
        }
        for (k, &idx) in next.iter().enumerate() {
            if idx != sorted + k {
                hs.swap(sorted + k, idx);
            }
        }
        if next.len() == 2 {
            let (a, b) = (hs[sorted], hs[sorted + 1]);
            let site = Site { effect: Effect::Pair(a.effect, b.effect), ..Site::new(Key::SpeedTie, None) };
            let (sa, sb) = (holder_side(&a.holder), holder_side(&b.holder));
            let first = match (sa, sb) {
                (Some(x), Some(y)) if x != y => sim.o.tie(&site, x, y) == x,
                _ if a.effect != b.effect => {
                    let s1 = Site { side: sa.or(sb), ..site };
                    sim.o.tie_effects(&s1, a.effect, b.effect)
                }
                _ => {
                    sim.note(a.effect, Hook::Residual);
                    true
                }
            };
            if !first {
                hs.swap(sorted, sorted + 1);
            }
        } else if next.len() > 2 {
            // prng.shuffle: one draw per position (the oracle's raw shuffle events)
            let end = sorted + next.len();
            for i in sorted..end - 1 {
                let site = Site { effect: Effect::Pair(hs[i].effect, hs[i + 1].effect), args: Some((i as i32, end as i32)), ..Site::new(Key::Raw, None) };
                let r = sim.o.range(&site, i as i32, end as i32) as usize;
                if r != i {
                    hs.swap(i, r);
                }
            }
        }
        sorted += next.len();
    }
}

/// Runs one handler; returns true when the battle ended.
pub fn run<O: Outcomes>(sim: &mut Sim<'_, O>, h: &Handler) -> bool {
    if let Some(side) = holder_side(&h.holder) {
        let sd = &sim.s.sides[side as usize];
        if !matches!(h.holder, Holder::SideCond(..)) && sd.has_active && sd.active_mon().fainted
            && !matches!(h.holder, Holder::SlotCond(..))
        {
            return false;
        }
    }
    // the effect may have been removed by an earlier handler
    match h.holder {
        Holder::Volatile(s, c) => {
            if !sim.s.sides[s as usize].has_vol(c) {
                return false;
            }
        }
        Holder::Status(s) => {
            if status_cond(sim.s.active(s as usize).status) != Some(match h.effect {
                EffectRef::Cond(c) => c,
                _ => Cond::NONE,
            }) {
                return false;
            }
        }
        Holder::SideCond(s, c) => {
            if sim.side_cond(s as usize, c).is_none() {
                return false;
            }
        }
        Holder::SlotCond(s, c) => {
            if sim.slot_cond(s as usize, c).is_none() {
                return false;
            }
        }
        Holder::Pseudo(c) => {
            if !sim.has_pseudo(c) {
                return false;
            }
        }
        Holder::Weather => {
            if EffectRef::Cond(sim.s.field.weather.id) != h.effect {
                return false;
            }
        }
        Holder::Terrain => {
            if EffectRef::Cond(sim.s.field.terrain.id) != h.effect {
                return false;
            }
        }
        _ => {}
    }
    if h.duration {
        let f = &mut sim.s.field;
        let left = match h.holder {
            Holder::Weather => {
                f.weather.duration -= 1;
                f.weather.duration
            }
            Holder::Terrain => {
                f.terrain.duration -= 1;
                f.terrain.duration
            }
            Holder::Pseudo(c) => {
                let n = f.npseudo as usize;
                let x = f.pseudo[..n].iter_mut().find(|x| x.id == c).unwrap();
                x.duration -= 1;
                x.duration
            }
            Holder::SideCond(s, c) => {
                let sd = &mut sim.s.sides[s as usize];
                let n = sd.nconds as usize;
                let x = sd.conds[..n].iter_mut().find(|x| x.id == c).unwrap();
                x.duration -= 1;
                x.duration
            }
            Holder::SlotCond(s, c) => {
                let sd = &mut sim.s.sides[s as usize];
                let n = sd.nslot_conds as usize;
                let x = sd.slot_conds[..n].iter_mut().find(|x| x.id == c).unwrap();
                x.duration -= 1;
                x.duration
            }
            Holder::Volatile(s, c) => {
                let v = sim.s.sides[s as usize].vol_mut(c).unwrap();
                v.duration -= 1;
                v.duration
            }
            _ => 1,
        };
        if left <= 0 {
            match h.holder {
                Holder::Volatile(s, c) => {
                    let p = (s as usize, sim.s.sides[s as usize].active as usize);
                    sim.remove_volatile(p, c);
                }
                Holder::Weather => {
                    sim.clear_weather();
                }
                Holder::Terrain => {
                    sim.clear_terrain();
                }
                Holder::Pseudo(c) => {
                    sim.remove_pseudo(c);
                }
                Holder::SideCond(s, c) => {
                    sim.remove_side_cond(s as usize, c);
                }
                Holder::SlotCond(s, c) => {
                    sim.remove_slot_cond(s as usize, c);
                }
                _ => sim.note(h.effect, Hook::End),
            }
            return sim.s.ended;
        }
    }
    if h.callback {
        match (h.holder, h.effect) {
            (Holder::Volatile(s, c), EffectRef::Cond(_)) if implemented(EffectRef::Cond(c), Hook::Residual) => {
                let p = (s as usize, sim.s.sides[s as usize].active as usize);
                sim.cond_residual(p, c);
            }
            (Holder::Status(s), EffectRef::Cond(c)) if implemented(EffectRef::Cond(c), Hook::Residual) => {
                let p = (s as usize, sim.s.sides[s as usize].active as usize);
                sim.cond_residual(p, c);
            }
            (Holder::Weather, EffectRef::Cond(c)) if implemented(EffectRef::Cond(c), Hook::Residual) => {
                sim.weather_residual(c);
            }
            (Holder::SlotCond(s, c), _) if implemented(EffectRef::Cond(c), Hook::Residual) => {
                sim.slot_residual(s as usize, c);
            }
            (Holder::FieldFor(s), EffectRef::Cond(c)) if c == conds::GRASSYTERRAIN => {
                let p = (s as usize, sim.s.sides[s as usize].active as usize);
                sim.grassy_heal(p);
            }
            (Holder::Item(s), EffectRef::Item(i)) if super::items::item_implemented(i.data()) => {
                let p = (s as usize, sim.s.sides[s as usize].active as usize);
                sim.item_residual(p, i);
            }
            (Holder::Ability(s), EffectRef::Ability(a)) if super::abilities::ability_implemented(a) => {
                let p = (s as usize, sim.s.sides[s as usize].active as usize);
                if sim.s.sides[s as usize].active_mon().ability == a {
                    sim.ability_residual(p, a);
                }
            }
            _ => sim.note(h.effect, Hook::Residual),
        }
    }
    sim.resolve_faints(false, false, true);
    sim.s.ended
}
