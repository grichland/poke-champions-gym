//! The resolver's rows as each seat's contract events: one event per row the phone's log writes, in the
//! phone's order, with what that seat may see.
//!
//! What this module decides, and nothing else:
//! - own / opp per seat; own HP in whole numbers, the opponent's in percent (`opp_pct`);
//! - the phone's order where it differs from the rows: an effectiveness row after the HP row of its hit (one
//!   for a multi-hit move), Focus Sash's rows after the hit, a resist Berry's second row after it, the cause
//!   row before its HP row except where the phone writes it after (Life Orb, a trapping move, Leech Seed);
//! - rows the phone writes for a cause the rows carry inside another row: the `Held Item` row of an item
//!   that changed HP, the `Ability` row of an ability that changed HP, set a weather or terrain, started a
//!   condition, blocked something or cured a status;
//! - rows the phone does not write: HP rows with no change (LG-1, unconfirmed), a protect or recharge row
//!   repeated (LG-7, unconfirmed), Roost's turn, Must Recharge's start, silent heals and cures on switching
//!   out (Regenerator, Natural Cure), the battle's end row;
//! - names: an opposing Pokémon under Illusion is named as the Pokémon it shows until the disguise ends
//!   (our own keeps its real name: unconfirmed, the phone's own-side rows under Illusion are not recorded).

use battle::{Detail, Eff, Event, Hp, HpDir, ItemWhat as EvItem, LimitDir, Mon, ResetWhat, SetMsg, Side, SideWhat as EvSide, StartEnd};
use cg_engine::sim::data::*;
use cg_engine::sim::log::{Detail as RDetail, Fx, How, ItemWhat, Reset, Row, SideWhat, Who};
use cg_engine::sim::options::build_mon;
use cg_engine::sim::state::{State, SIDES, TEAM};
use cg_engine::sim::MonSpec;

/// The opponent's HP as the game shows it: a whole percent, rounded up, 0 only when fainted and 100 only
/// when full (Showdown's rule for the other side's percent; unconfirmed for the phone: IN-1).
pub fn opp_pct(hp: u16, max: u16) -> f64 {
    if hp == 0 || max == 0 {
        return 0.0;
    }
    let p = ((hp as u64 * 100 + max as u64 - 1) / max as u64).max(1);
    if p >= 100 && hp < max {
        99.0
    } else {
        p.min(100) as f64
    }
}

/// Species ids as brought and max HP, by side and State slot.
#[derive(Clone, Copy, Debug)]
pub struct Names {
    pub ids: [[&'static str; TEAM]; SIDES],
    pub maxhp: [[u16; TEAM]; SIDES],
}

impl Names {
    pub fn of(s: &State) -> Names {
        let mut ids = [[""; TEAM]; SIDES];
        let mut maxhp = [[0; TEAM]; SIDES];
        for side in 0..SIDES {
            for i in 0..TEAM {
                ids[side][i] = s.sides[side].mons[i].set_species.id();
                maxhp[side][i] = s.sides[side].mons[i].maxhp;
            }
        }
        Names { ids, maxhp }
    }
}

/// One row of the phone's log, seat-neutral: the event as side 0 sees it (real names), plus what a seat
/// needs to finish it.
#[derive(Clone, Debug)]
struct Ph {
    ev: Event,
    /// an HP row in exact HP: the Pokémon, before, after, max
    hp: Option<(Who, u16, u16, u16)>,
    /// (real slot, shown slot) of a Pokémon under Illusion when the row was written, per side
    disguise: [Option<(u8, u8)>; SIDES],
    /// the row names the real Pokémon even under a disguise (Illusion wearing off)
    real: bool,
}

/// What the emitter carries from row to row and step to step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Emitter {
    /// an Illusion user on the field and the slot it shows itself as, per side
    pub disguise: [Option<(u8, u8)>; SIDES],
    /// the move of each Pokémon's partial trap
    pub traps: [[MoveId; TEAM]; SIDES],
    weather: Cond,
}

fn mon0(names: &Names, w: Who) -> Mon {
    Mon::new(if w.0 == 0 { Side::Own } else { Side::Opp }, names.ids[w.0 as usize][w.1 as usize])
}

fn side0(s: u8) -> Side {
    if s == 0 {
        Side::Own
    } else {
        Side::Opp
    }
}

fn cond_id(c: Cond) -> &'static str {
    c.id()
}

fn is_protect_cond(c: Cond) -> bool {
    matches!(c.id(), "protect" | "kingsshield" | "spikyshield" | "banefulbunker" | "silktrap" | "burningbulwark" | "obstruct" | "maxguard")
}

fn fx_id(f: Fx) -> Option<&'static str> {
    Some(match f {
        Fx::None => return None,
        Fx::Move(m) | Fx::Recoil(m) | Fx::Drain(m) | Fx::Crash(m) | Fx::Cost(m) => m.id(),
        Fx::Ability(a) => a.id(),
        Fx::Item(i) => i.id(),
        Fx::Cond(c) => c.id(),
        Fx::Status(s) => s.id(),
        Fx::Confusion => "confusion",
        Fx::HpFull => "hpfull",
    })
}

/// The rows of a step in the phone's order (indices into `rows`).
fn phone_order(rows: &[Row]) -> Vec<usize> {
    let mut v: Vec<usize> = (0..rows.len()).collect();
    let hp_of = |r: &Row, t: Who| matches!(r, Row::Hp { p, cause: Fx::Move(_), .. } if *p == t) || matches!(r, Row::Hp { p, cause: Fx::Cond(c), .. } if *p == t && *c == conds::FUTUREMOVE);
    // effectiveness after the HP row of its hit, and a held item's rows that come between them
    let mut i = 0;
    while i < v.len() {
        if let Row::Result { how, p: Some(t) } = rows[v[i]] {
            if how != How::Immune {
                let mut j = i + 1;
                while j < v.len() && matches!(rows[v[j]], Row::Item { p, .. } if p == t) {
                    j += 1;
                }
                if j < v.len() && hp_of(&rows[v[j]], t) {
                    let r = v.remove(i);
                    v.insert(j, r);
                    i = j + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    // Focus Sash's rows and a resist Berry's second row go after the hit (and its effectiveness row)
    let mut i = 0;
    while i < v.len() {
        let late = match rows[v[i]] {
            Row::Item { p, item, what: ItemWhat::Used, .. } if item.id() == "focussash" => Some(p),
            Row::Item { p, item, what: ItemWhat::Activated, .. } if i > 0 && matches!(rows[v[i - 1]], Row::Item { p: q, item: x, what: ItemWhat::Used, .. } if q == p && x == item) => Some(p),
            _ => None,
        };
        if let Some(t) = late {
            let mut j = i + 1;
            if j < v.len() && hp_of(&rows[v[j]], t) {
                j += 1;
                if j < v.len() && matches!(rows[v[j]], Row::Result { p: Some(q), .. } if q == t) {
                    j += 1;
                }
                let r = v.remove(i);
                v.insert(j - 1, r);
                i = j;
                continue;
            }
        }
        i += 1;
    }
    // Endure's and Sturdy's rows after the hit they survived (and its effectiveness row)
    let mut i = 0;
    while i < v.len() {
        if let Row::VolatileActed { p: t, c, .. } = rows[v[i]] {
            if c == conds::ENDURE && i + 1 < v.len() && hp_of(&rows[v[i + 1]], t) {
                let start = if i > 0 && matches!(rows[v[i - 1]], Row::Ability { p, ab, .. } if p == t && ab == abilities::STURDY) { i - 1 } else { i };
                let mut j = i + 2;
                if j < v.len() && matches!(rows[v[j]], Row::Result { p: Some(q), .. } if q == t) {
                    j += 1;
                }
                let block: Vec<usize> = v.drain(start..=i).collect();
                let at = j - block.len();
                for (k, r) in block.into_iter().enumerate() {
                    v.insert(at + k, r);
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    // Substitute's HP cost before its row; Pain Split's gain before its loss; an item acting at once
    // (an Air Balloon's announcement, a White Herb) after the row that gave it (Trick)
    for i in 0..v.len().saturating_sub(1) {
        let (a, b) = (rows[v[i]], rows[v[i + 1]]);
        let swap = match (a, b) {
            (Row::Item { p, item, what, .. }, Row::Item { p: q, item: j, what: ItemWhat::Received, .. }) => {
                p == q && item == j && matches!(what, ItemWhat::Found | ItemWhat::Used | ItemWhat::Activated)
            }
            (Row::Volatile { p, c, .. }, Row::Hp { p: q, cause: Fx::Cost(_), .. }) => c == conds::SUBSTITUTE && p == q,
            (Row::Hp { before: b0, after: a0, cause: Fx::Move(m0), .. }, Row::Hp { before: b1, after: a1, cause: Fx::Move(m1), .. }) => {
                m0 == moves::PAINSPLIT && m1 == moves::PAINSPLIT && a0 < b0 && a1 > b1
            }
            _ => false,
        };
        if swap {
            v.swap(i, i + 1);
        }
    }
    // Skill Swap's own row (and the abilities it names) right after its move row, before what the new
    // abilities do (Trace taking effect)
    let mut i = 0;
    while i < v.len() {
        if matches!(rows[v[i]], Row::Once { fx: Fx::Move(m), .. } if m == moves::SKILLSWAP) {
            if let Some(mut j) = (0..i).rev().find(|&j| matches!(rows[v[j]], Row::Move { mv, .. } if mv == moves::SKILLSWAP)) {
                // Protean or Libero acts as the move is used, before the swap
                while j + 1 < i && matches!(rows[v[j + 1]], Row::Types { by: Fx::Ability(a), .. } if a.id() == "protean" || a.id() == "libero") {
                    j += 1;
                }
                if j + 1 < i {
                    let r = v.remove(i);
                    v.insert(j + 1, r);
                }
            }
        }
        i += 1;
    }
    // one effectiveness row for a multi-hit move: the last
    let mut out = Vec::with_capacity(v.len());
    for (k, &x) in v.iter().enumerate() {
        if let Row::Result { how, p: Some(t) } = rows[x] {
            let again = v.get(k + 1).map_or(false, |&y| hp_of(&rows[y], t)) && v.get(k + 2).map_or(false, |&y| rows[y] == Row::Result { how, p: Some(t) });
            if again {
                continue;
            }
        }
        out.push(x);
    }
    out
}

/// The Mega form shown, or None when the species has more than one Mega (the row does not say which).
fn mega_shown(base: SpeciesId, form: SpeciesId) -> Option<String> {
    let n = ITEMS.iter().flat_map(|i| i.mega_stone.iter()).filter(|(x, _)| *x == base).count();
    if n > 1 {
        None
    } else {
        Some(form.id().to_string())
    }
}

/// Items whose use the phone writes as a second row after the `Held Item` row.
fn used_sentence(item: ItemId) -> bool {
    matches!(item.id(), "focussash" | "redcard" | "ejectbutton" | "ejectpack" | "normalgem")
        || item.data().handlers.iter().any(|h| *h == "onSourceTryPrimaryHit") && item.id().ends_with("gem")
}

struct Ctx<'a> {
    names: &'a Names,
    s0: &'a State,
    s1: &'a State,
    out: Vec<Ph>,
    disguise: [Option<(u8, u8)>; SIDES],
    fainted: [[bool; TEAM]; SIDES],
    /// each Pokémon's ability as the rows so far left it
    ab: [[AbilityId; TEAM]; SIDES],
    /// each Pokémon's HP as the rows so far left it
    hp: [[u16; TEAM]; SIDES],
    /// Pain Split's row was written for the move under way
    painsplit: bool,
    /// the item a Fling under way throws and its thrower (its effects act on the target)
    flung: Option<(ItemId, Who)>,
    /// a Berry Bug Bite or Pluck just took and ate: eater, Berry, holder
    eaten: Option<(Who, ItemId, Who)>,
}

impl Ctx<'_> {
    fn m(&self, w: Who) -> Mon {
        mon0(self.names, w)
    }
    fn push(&mut self, ev: Event) {
        // LG-7: a protect or recharge row repeated is shown once (unconfirmed)
        if let Some(last) = self.out.last() {
            let dup = matches!(&ev, Event::Protect { .. }) || matches!(&ev, Event::Cant { reason, .. } if reason == "recharge");
            if dup && last.ev == ev {
                return;
            }
        }
        self.out.push(Ph { ev, hp: None, disguise: self.disguise, real: false });
    }
    fn push_real(&mut self, ev: Event) {
        self.out.push(Ph { ev, hp: None, disguise: self.disguise, real: true });
    }
    fn push_hp(&mut self, w: Who, before: u16, after: u16) {
        let max = self.names.maxhp[w.0 as usize][w.1 as usize];
        let dir = if after < before { HpDir::Lost } else { HpDir::Restored };
        let ev = Event::Hp { who: self.m(w), dir, hp: Hp::Own { before: before as i32, after: after as i32 } };
        self.out.push(Ph { ev, hp: Some((w, before, after, max)), disguise: self.disguise, real: false });
    }
    fn cause(&mut self, effect: &str, w: Option<Who>) {
        let who = w.map(|w| self.m(w));
        self.push(Event::Cause { effect: effect.into(), who, of: None });
    }
    /// The `Ability` row a cause implies, unless the row before is that same row.
    fn ability(&mut self, w: Who, ab: AbilityId) {
        let ev = Event::Ability { who: self.m(w), ability: ab.id().into() };
        if self.out.last().map_or(false, |l| l.ev == ev) {
            return;
        }
        self.push(ev);
    }
    /// The `Held Item` row a cause implies, unless the row before is that same row.
    fn held(&mut self, w: Who, it: ItemId) {
        // a Berry just taken from another Pokémon and eaten (Bug Bite, Pluck): its holder is named
        let of = stolen_from(self, w, it);
        let ev = Event::Item { who: Some(self.m(w)), item: it.id().into(), what: EvItem::Activated, of };
        if self.out.last().map_or(false, |l| l.ev == ev) {
            return;
        }
        self.push(ev);
    }
    fn implied(&mut self, by: Fx, holder: Who) {
        match by {
            Fx::Ability(a) => self.ability(holder, a),
            Fx::Item(i) => self.held(holder, i),
            _ => {}
        }
    }
}

impl Emitter {
    pub fn new(s: &State) -> Emitter {
        let mut e = Emitter::default();
        e.sync(s);
        e
    }

    fn sync(&mut self, s: &State) {
        self.weather = s.field.weather.id;
        for side in 0..SIDES {
            if let Some(v) = s.sides[side].vol(conds::PARTIALLYTRAPPED) {
                if s.sides[side].has_active && !v.mv.is_none() {
                    self.traps[side][s.sides[side].active as usize] = v.mv;
                }
            }
        }
    }

    /// The events of one step's rows for each seat. `s0` is the State before the step, `s1` after it.
    /// `start`: the leads' rows, where the phone writes the turn heading first and the opponent's lead
    /// before ours.
    pub fn events(&mut self, rows: &[Row], s0: &State, s1: &State, names: &Names, start: bool) -> [Vec<Event>; SIDES] {
        self.sync(s0);
        let mut fainted = [[false; TEAM]; SIDES];
        for side in 0..SIDES {
            for i in 0..TEAM {
                fainted[side][i] = s0.sides[side].mons[i].fainted;
            }
        }
        let mut ab = [[AbilityId::NONE; TEAM]; SIDES];
        for side in 0..SIDES {
            for i in 0..TEAM {
                ab[side][i] = s0.sides[side].mons[i].ability;
            }
        }
        let mut hp = [[0u16; TEAM]; SIDES];
        for side in 0..SIDES {
            for i in 0..TEAM {
                hp[side][i] = s0.sides[side].mons[i].hp;
            }
        }
        let mut cx = Ctx { names, s0, s1, out: Vec::with_capacity(rows.len() + 4), disguise: self.disguise, fainted, ab, hp, painsplit: false, flung: None, eaten: None };
        let order = phone_order(rows);
        let mut turn_first = None;
        if start {
            if let Some(&Row::Turn(n)) = rows.last() {
                turn_first = Some(n);
                cx.push(Event::Turn { n: n as u32 });
            }
        }
        for (k, &i) in order.iter().enumerate() {
            let r = &rows[i];
            if turn_first.is_some() && k + 1 == order.len() && matches!(r, Row::Turn(_)) {
                break;
            }
            self.row(&mut cx, rows, i);
        }
        self.disguise = cx.disguise;
        let out = cx.out;
        let mut seats: [Vec<Event>; SIDES] = [Vec::with_capacity(out.len()), Vec::with_capacity(out.len())];
        for (seat, evs) in seats.iter_mut().enumerate() {
            for ph in &out {
                evs.push(for_seat(ph, seat as u8, names));
            }
            if start {
                // the opponent's lead first
                let own = evs.iter().position(|e| matches!(e, Event::Enter { who, .. } if who.side == Side::Own));
                let opp = evs.iter().position(|e| matches!(e, Event::Enter { who, .. } if who.side == Side::Opp));
                if let (Some(a), Some(b)) = (own, opp) {
                    if a < b {
                        evs.swap(a, b);
                    }
                }
            }
        }
        seats
    }

    fn row(&mut self, cx: &mut Ctx, rows: &[Row], i: usize) {
        let r = rows[i];
        match r {
            Row::Turn(n) => cx.push(Event::Turn { n: n as u32 }),
            Row::Enter { p, forced } => {
                let side = p.0 as usize;
                cx.disguise[side] = self.disguise_of(cx, p);
                cx.push(Event::Enter { who: cx.m(p), forced });
            }
            Row::Withdraw { p, pivot } => {
                let pv = pivot.map_or(false, |m| m != moves::SHEDTAIL);
                cx.push(Event::Withdraw { who: cx.m(p), pivot: pv });
                cx.disguise[p.0 as usize] = None;
            }
            Row::Move { p, mv, called_by, priority, .. } => {
                cx.flung = None;
                if mv == moves::FLING {
                    // the item it throws: named by the row that removes it, later in the move
                    cx.flung = rows[i + 1..]
                        .iter()
                        .take_while(|r| !matches!(r, Row::Move { .. }))
                        .find_map(|r| match *r {
                            Row::Item { p: q, item, what: ItemWhat::Removed, by: Fx::Move(m), .. } if q == p && m == moves::FLING => Some((item, p)),
                            _ => None,
                        });
                }
                cx.painsplit = false;
                cx.eaten = None;
                // the phone names no caller (LV5-8): the Builder takes it from the row before, the calling
                // move's or the ability's (Magic Bounce, Dancer), whose Ability row comes first
                if let Fx::Ability(a) = called_by {
                    cx.ability(p, a);
                }
                cx.push(Event::Move { who: cx.m(p), mv: mv.id().into(), priority: if priority != 0 { Some(priority as i32) } else { None }, called_by: None });
            }
            Row::Prepare { p, .. } => cx.push(Event::Prepare { who: cx.m(p), mv: None }),
            Row::Cant { p, why, mv } => {
                // an ability that stops a move (Damp, Dazzling, ...): the holder's Ability row, then the row of
                // the Pokémon that could not use it, naming the move ("X cannot use MOVE!")
                if let (Fx::Ability(a), false) = (why, mv.is_none()) {
                    cx.ability(p, a);
                    let user = blocked_user(cx, p, mv);
                    cx.push(Event::Cant { who: Some(cx.m(user)), reason: mv.id().into() });
                    return;
                }
                // no PP left: the move row, then a nameless row
                if why == Fx::None && !mv.is_none() {
                    cx.push(Event::Move { who: cx.m(p), mv: mv.id().into(), priority: None, called_by: None });
                    cx.push(Event::Cant { who: None, reason: "nopp".into() });
                    return;
                }
                let reason = match why {
                    Fx::Status(s) => s.id().to_string(),
                    Fx::Cond(c) if c == conds::MUSTRECHARGE => "recharge".into(),
                    Fx::Cond(c) => c.id().to_string(),
                    Fx::Ability(a) => {
                        cx.ability(p, a);
                        a.id().to_string()
                    }
                    Fx::Move(m) => m.id().to_string(),
                    _ if !mv.is_none() => "nopp".into(),
                    _ => "other".into(),
                };
                cx.push(Event::Cant { who: Some(cx.m(p)), reason });
            }
            Row::Protect { p, .. } => cx.push(Event::Protect { who: cx.m(p) }),
            Row::Miss { p, .. } => cx.push(Event::Miss { who: cx.m(p) }),
            Row::Fail { p, why } => {
                // a failure the row explains names its Pokémon; "But it failed!" names none
                let who = if why == Fx::None { None } else { p.map(|w| cx.m(w)) };
                if let (Fx::Ability(a), Some(w)) = (why, p) {
                    cx.ability(w, a);
                }
                cx.push(Event::Fail { who, reason: fx_id(why).map(str::to_string) });
            }
            Row::Result { how, p } => {
                let how = match how {
                    How::Super => Eff::Super,
                    How::Extreme => Eff::Extreme,
                    How::Resisted => Eff::Resisted,
                    How::Mostly => Eff::MostlyIneffective,
                    How::Immune => Eff::Immune,
                    How::Ohko => Eff::Ohko,
                };
                let who = if how == Eff::Immune { p.map(|w| cx.m(w)) } else { None };
                cx.push(Event::Result { how, who });
            }
            Row::Hits { n, .. } => cx.push(Event::Hits { n: n as u32 }),
            Row::Hp { p, before, after, cause, of } => {
                cx.hp[p.0 as usize][p.1 as usize] = after;
                self.hp(cx, p, before, after, cause, of)
            }
            Row::Faint { p } => {
                cx.fainted[p.0 as usize][p.1 as usize] = true;
                cx.push(Event::Faint { who: cx.m(p) });
                if cx.disguise[p.0 as usize].map_or(false, |d| d.0 == p.1) {
                    cx.disguise[p.0 as usize] = None;
                }
            }
            Row::Status { p, st, by, of, rest } => {
                if let Fx::Ability(_) | Fx::Item(_) = by {
                    cx.implied(by, of.unwrap_or(p));
                }
                cx.push(Event::Status { who: cx.m(p), status: st.id().into(), effect: if rest { Some("slept_and_restored".into()) } else { None } });
            }
            Row::Cure { p, st, by } => {
                let item = match by {
                    Fx::Item(it) => {
                        let theirs = stolen_from(cx, p, it).is_some() || eaten(cx, p, it);
                        cx.held(p, it);
                        (!theirs).then(|| it.id().to_string())
                    }
                    Fx::Ability(a) if a == abilities::NATURALCURE && self.leaving(cx, p) => return,
                    Fx::Ability(a) => {
                        cx.ability(p, a);
                        None
                    }
                    _ => None,
                };
                cx.push(Event::Cure { who: cx.m(p), status: st.id().into(), item });
            }
            Row::Volatile { p, c, detail, by, of } => self.volatile(cx, p, c, detail, by, of),
            Row::VolatileEnd { p, c, by } => {
                if c.is_none() {
                    if by == Fx::Ability(abilities::ILLUSION) {
                        cx.disguise[p.0 as usize] = None;
                        cx.push_real(Event::VolatileEnd { who: cx.m(p), effect: "illusion".into() });
                    }
                    return;
                }
                let effect = if c == conds::PARTIALLYTRAPPED { self.trap_id(p) } else { cond_id(c).to_string() };
                cx.push(Event::VolatileEnd { who: cx.m(p), effect });
            }
            Row::VolatileActed { p, c, of, .. } => {
                let effect = match c.id() {
                    "endure" => "endured_hit",
                    "destinybond" => {
                        cx.push(Event::Once { effect: "took_attacker_down".into(), who: Some(cx.m(p)) });
                        // the phone writes the attacker's HP falling to 0 before its faint
                        if let Some(a) = of {
                            let before = cx.hp[a.0 as usize][a.1 as usize];
                            if before > 0 {
                                cx.push_hp(a, before, 0);
                                cx.hp[a.0 as usize][a.1 as usize] = 0;
                            }
                        }
                        return;
                    }
                    x => x,
                };
                let _ = of;
                cx.push(Event::VolatileHit { effect: effect.into(), who: Some(cx.m(p)) });
            }
            Row::Boost { p, by_stat, by } => {
                if let Fx::Item(it) = by {
                    cx.held(p, it);
                }
                let mut seen = [false; 7];
                for k in 0..7 {
                    let d = by_stat[k];
                    if d == 0 || seen[k] {
                        continue;
                    }
                    let stats: Vec<String> = (k..7).filter(|&j| by_stat[j] == d).map(|j| BOOST_IDS[j].to_string()).collect();
                    for j in k..7 {
                        if by_stat[j] == d {
                            seen[j] = true;
                        }
                    }
                    cx.push(Event::Boost { who: cx.m(p), stats, by: d as i32 });
                }
            }
            Row::BoostLimit { p, stats, up, .. } => {
                let s: Vec<String> = (0..7).filter(|&k| stats[k]).map(|k| BOOST_IDS[k].to_string()).collect();
                cx.push(Event::BoostLimit { who: cx.m(p), stats: s, dir: if up { LimitDir::Up } else { LimitDir::Down } });
            }
            Row::BoostReset { p, what, of, by } => {
                if let (Fx::Item(it), Some(w)) = (by, p) {
                    cx.held(w, it);
                }
                let (who, what) = match what {
                    Reset::Field => (None, ResetWhat::All),
                    Reset::All => (p.map(|w| cx.m(w)), ResetWhat::All),
                    Reset::Negative => (p.map(|w| cx.m(w)), ResetWhat::Negative),
                    Reset::Positive => (p.map(|w| cx.m(w)), ResetWhat::Positive),
                    Reset::Copy => (p.map(|w| cx.m(w)), ResetWhat::Copy),
                    Reset::Swap => (p.map(|w| cx.m(w)), ResetWhat::Swap),
                    Reset::Invert => (p.map(|w| cx.m(w)), ResetWhat::Invert),
                };
                let stats = match (what, by) {
                    (ResetWhat::Swap, Fx::Move(m)) => match m.id() {
                        "powerswap" => Some(vec!["atk".to_string(), "spa".to_string()]),
                        "guardswap" => Some(vec!["def".to_string(), "spd".to_string()]),
                        _ => None,
                    },
                    _ => None,
                };
                cx.push(Event::BoostReset { who, what, of: of.map(|w| cx.m(w)), stats });
            }
            Row::Blocked { p, by, what } => {
                let effect = match by {
                    Fx::Ability(a) => {
                        if let Some(w) = p {
                            cx.ability(w, a);
                        }
                        if a == abilities::UNNERVE {
                            "unnerve".to_string()
                        } else if matches!(what, Fx::None) {
                            "stat_drop".to_string()
                        } else {
                            a.id().to_string()
                        }
                    }
                    f => fx_id(f).unwrap_or("other").to_string(),
                };
                // a terrain's block names no move
                let what_id = match (by, what) {
                    (Fx::Ability(a), _) if a == abilities::UNNERVE => Some("berries".to_string()),
                    (Fx::Cond(_), _) => None,
                    (_, w) => fx_id(w).map(str::to_string),
                };
                let who = if by == Fx::Ability(abilities::UNNERVE) { None } else { p.map(|w| cx.m(w)) };
                cx.push(Event::Blocked { effect, who, what: what_id });
            }
            Row::Ability { p, ab, .. } => cx.ability(p, ab),
            Row::AbilityChange { p, from, to, .. } => {
                cx.push(Event::AbilityChange { who: cx.m(p), from: from.id().into(), to: to.id().into() });
                cx.ab[p.0 as usize][p.1 as usize] = to;
            }
            Row::Item { p, item, what, of, by } => self.item(cx, rows, i, p, item, what, of, by),
            Row::MegaStone { p, stone } => cx.push(Event::MegaStone { who: cx.m(p), stone: stone.id().into() }),
            Row::Mega { p, form } => {
                let base = cx.s0.sides[p.0 as usize].mons[p.1 as usize].base_species;
                cx.push(Event::Mega { who: cx.m(p), form: mega_shown(base, form) });
                // a Mega has its form's one ability from now on (a Skill Swap later names it)
                let ab = form.data().abilities[0];
                if ab.0 != 0 {
                    cx.ab[p.0 as usize][p.1 as usize] = ab;
                }
            }
            Row::Form { p, form, by } => {
                if let Fx::Ability(a) = by {
                    cx.ability(p, a);
                }
                cx.push(Event::Form { who: cx.m(p), form: form.id().into() });
            }
            Row::Transform { p, into } => {
                let sp = cx.s0.sides[into.0 as usize].mons[into.1 as usize].species;
                cx.push(Event::Transform { who: cx.m(p), into: sp.id().into() });
                cx.ab[p.0 as usize][p.1 as usize] = cx.ab[into.0 as usize][into.1 as usize];
            }
            Row::Types { p, types, lost, added, by } => {
                if let Fx::Ability(a) = by {
                    cx.ability(p, a);
                }
                let ev = if lost.is_real() {
                    Event::Types { who: cx.m(p), ty: None, lose: Some(lost.id().into()), add: None }
                } else if added.is_real() {
                    Event::Types { who: cx.m(p), ty: None, lose: None, add: Some(added.id().into()) }
                } else {
                    Event::Types { who: cx.m(p), ty: if types[0].is_real() { Some(types[0].id().into()) } else { None }, lose: None, add: None }
                };
                cx.push(ev);
            }
            Row::Weather { w, by, of } => {
                if let Some(h) = of {
                    cx.implied(by, h);
                }
                if w.is_none() {
                    if !self.weather.is_none() {
                        cx.push(Event::Weather { weather: self.weather.id().into(), what: StartEnd::End });
                    }
                } else {
                    cx.push(Event::Weather { weather: w.id().into(), what: StartEnd::Start });
                }
                self.weather = w;
            }
            Row::Field { c, start, by, of } => {
                if let Some(h) = of {
                    cx.implied(by, h);
                }
                cx.push(Event::Field { effect: c.id().into(), what: if start { StartEnd::Start } else { StartEnd::End } });
            }
            Row::Side { side, c, what, p, .. } => {
                let what = match what {
                    SideWhat::Start => EvSide::Start,
                    SideWhat::End => EvSide::End,
                    SideWhat::Acted => EvSide::Acted,
                };
                let mon = if what == EvSide::Acted { p.map(|w| cx.names.ids[w.0 as usize][w.1 as usize].to_string()) } else { None };
                cx.push(Event::SideCond { side: side0(side), effect: c.id().into(), what, mon });
            }
            Row::Once { p, fx, of } => match fx {
                Fx::Cond(c) if is_protect_cond(c) => {
                    if let Some(w) = p {
                        cx.push(Event::Protect { who: cx.m(w) });
                    }
                }
                Fx::Cond(c) if c == conds::ROOST => {}
                Fx::Cond(c) if c == conds::ENDURE => {
                    if let Some(w) = p {
                        cx.push(Event::Volatile { who: cx.m(w), effect: "endure".into(), detail: None });
                    }
                }
                Fx::Move(m) if m == moves::STRUGGLE => cx.push(Event::Once { effect: "no_moves_left".into(), who: p.map(|w| cx.m(w)) }),
                Fx::Move(m) if m == moves::PERISHSONG => cx.push(Event::Once { effect: "perishsong".into(), who: None }),
                // Future Sight: the row right after its user's move row is the use; any other is the landing
                Fx::Move(m) if m == moves::FUTURESIGHT => {
                    let used = p.map_or(false, |w| {
                        rows[..i]
                            .iter()
                            .rev()
                            .take_while(|r| !matches!(r, Row::Turn(_)) && !matches!(r, Row::Once { fx: Fx::Move(x), .. } if *x == moves::FUTURESIGHT))
                            .find(|r| matches!(r, Row::Move { .. }))
                            .map_or(false, |r| matches!(*r, Row::Move { p: q, mv, .. } if q == w && mv == moves::FUTURESIGHT))
                    });
                    let effect = if used { "futuresight_start" } else { "futuresight" };
                    cx.push(Event::Once { effect: effect.into(), who: p.map(|w| cx.m(w)) });
                }
                // Skill Swap between opponents names both abilities
                Fx::Move(m) if m == moves::SKILLSWAP => {
                    cx.push(Event::Once { effect: "skillswap".into(), who: p.map(|w| cx.m(w)) });
                    if let (Some(u), Some(t)) = (p, of) {
                        let au = cx.ab[u.0 as usize][u.1 as usize];
                        let at = cx.ab[t.0 as usize][t.1 as usize];
                        cx.ab[u.0 as usize][u.1 as usize] = at;
                        cx.ab[t.0 as usize][t.1 as usize] = au;
                        cx.push(Event::AbilityChange { who: cx.m(u), from: au.id().into(), to: at.id().into() });
                        cx.push(Event::AbilityChange { who: cx.m(t), from: at.id().into(), to: au.id().into() });
                    }
                }
                f => {
                    // the ability is its holder's: `of` when the row names one (Piercing Drill through a
                    // protection names the protected Pokémon)
                    if let (Fx::Ability(a), Some(w)) = (f, of.or(p)) {
                        cx.ability(w, a);
                    }
                    if let Some(id) = fx_id(f) {
                        cx.push(Event::Once { effect: id.into(), who: p.map(|w| cx.m(w)) });
                    }
                }
            },
            Row::Pp { p, mv, lost, .. } => cx.push(Event::Pp { who: cx.m(p), mv: mv.id().into(), lost: lost as u32 }),
            Row::Revive { p } => {
                cx.fainted[p.0 as usize][p.1 as usize] = false;
                cx.push(Event::Revive { who: cx.m(p) });
            }
            Row::End { .. } => {}
        }
    }

    /// The Pokémon an Illusion user entering shows itself as: the last one behind it that has not fainted.
    fn disguise_of(&self, cx: &Ctx, p: Who) -> Option<(u8, u8)> {
        let side = p.0 as usize;
        let sd = &cx.s1.sides[side];
        let m = &sd.mons[p.1 as usize];
        if m.base_ability != abilities::ILLUSION && cx.s0.sides[side].mons[p.1 as usize].ability != abilities::ILLUSION {
            return None;
        }
        let rest: Vec<u8> = sd.order.iter().copied().filter(|&x| x != p.1).collect();
        rest.iter().rev().copied().find(|&x| !cx.fainted[side][x as usize]).map(|x| (p.1, x))
    }

    fn trap_id(&self, p: Who) -> String {
        let m = self.traps[p.0 as usize][p.1 as usize];
        if m.is_none() {
            "partiallytrapped".into()
        } else {
            m.id().into()
        }
    }

    /// The Pokémon is leaving the field in this step (a cure or heal on switching out is silent).
    fn leaving(&self, cx: &Ctx, p: Who) -> bool {
        let sd = &cx.s1.sides[p.0 as usize];
        !(sd.has_active && sd.active == p.1) || sd.mons[p.1 as usize].being_called_back
    }

    #[allow(clippy::too_many_arguments)]
    fn hp(&mut self, cx: &mut Ctx, p: Who, before: u16, after: u16, cause: Fx, of: Option<Who>) {
        // LG-1: no change, no HP row (unconfirmed); Healing Wish's own row still shows, as it also cures
        if before == after {
            if matches!(cause, Fx::Cond(c) if c.id() == "healingwish" || c.id() == "lunardance") {
                cx.cause("healingwish", Some(p));
            }
            return;
        }
        let loss = after < before;
        match cause {
            Fx::Status(Status::Brn) => cx.cause("brn", Some(p)),
            Fx::Status(Status::Psn) | Fx::Status(Status::Tox) => cx.cause("psn", Some(p)),
            Fx::Cond(c) => match c.id() {
                "sandstorm" => cx.cause("sandstorm", Some(p)),
                "stealthrock" => cx.cause("stealthrock", Some(p)),
                "spikes" => cx.cause("spikes", Some(p)),
                "curse" => cx.cause("curse", Some(p)),
                "saltcure" => cx.cause("saltcure", Some(p)),
                "healingwish" | "lunardance" => cx.cause("healingwish", Some(p)),
                "wish" => cx.cause("wish", of.or(Some(p))),
                "grassyterrain" | "aquaring" | "ingrain" => cx.cause("hp_restored", Some(p)),
                "spikyshield" | "banefulbunker" | "burningbulwark" => cx.cause("was_hurt", Some(p)),
                // the landing's own row ("took the Future Sight attack!") is the cause row
                "futuremove" if matches!(cx.out.last().map(|x| &x.ev), Some(Event::Once { effect, .. }) if effect == "futuresight") => {}
                "futuremove" => cx.cause("futuresight", Some(p)),
                _ => {}
            },
            Fx::Recoil(_) => cx.cause("recoil", Some(p)),
            Fx::Crash(_) => cx.cause("crashed", Some(p)),
            Fx::Drain(_) => cx.cause("energy_drained", of.or(Some(p))),
            Fx::Confusion => cx.push(Event::VolatileHit { effect: "confusion_self_hit".into(), who: None }),
            Fx::Move(m) if m == moves::PAINSPLIT => {
                // one row for the pair of HP rows
                let first = !matches!(cx.out.last().map(|x| &x.ev), Some(Event::Hp { .. })) || !cx.painsplit;
                if first {
                    cx.cause("pain_split", None);
                    cx.painsplit = true;
                }
            }
            // an item or ability that hurt the Pokemon is the other one's (Rocky Helmet, Rough Skin)
            // a Berry another Pokémon held, eaten by this one (Fling, Bug Bite, Pluck): its row names both
            Fx::Item(it) if !loss && ((it.id().ends_with("berry") && of.map_or(false, |o| o != p)) || stolen_from(cx, p, it).is_some() || eaten(cx, p, it)) => {
                // a Berry eaten by Bug Bite or Pluck: its Held Item row names the eater only
                let holder = match of.filter(|o| *o != p && it.id().ends_with("berry")) {
                    _ if eaten(cx, p, it) => None,
                    Some(o) => Some(cx.m(o)),
                    None => stolen_from(cx, p, it),
                };
                cx.push(Event::Item { who: Some(cx.m(p)), item: it.id().into(), what: EvItem::Activated, of: holder });
            }
            Fx::Item(it) => cx.held(if loss { of.unwrap_or(p) } else { p }, it),
            Fx::Ability(a) if a == abilities::REGENERATOR => return,
            Fx::Ability(a) => cx.ability(if loss { of.unwrap_or(p) } else { p }, a),
            _ => {}
        }
        cx.push_hp(p, before, after);
        match cause {
            Fx::Item(it) if it.id() == "lifeorb" => cx.cause("lost_some_hp", Some(p)),
            Fx::Cond(c) if c == conds::PARTIALLYTRAPPED => cx.cause(&self.trap_id(p), Some(p)),
            Fx::Cond(c) if c == conds::LEECHSEED && loss => cx.cause("leechseed", Some(p)),
            Fx::Ability(a) if a == abilities::INNARDSOUT => cx.cause("was_hurt", Some(p)),
            _ => {}
        }
    }

    fn volatile(&mut self, cx: &mut Ctx, p: Who, c: Cond, detail: RDetail, by: Fx, of: Option<Who>) {
        if c == conds::MUSTRECHARGE {
            return;
        }
        if let Fx::Ability(_) | Fx::Item(_) = by {
            cx.implied(by, of.unwrap_or(p));
        }
        let det = match detail {
            RDetail::None => None,
            RDetail::Move(m) => Some(Detail::Id(m.id().into())),
            RDetail::Num(n) => Some(Detail::Num(n as i64)),
            RDetail::Type(t) => Some(Detail::Id(t.id().into())),
        };
        let (effect, det) = match c.id() {
            "partiallytrapped" => {
                if let RDetail::Move(m) = detail {
                    self.traps[p.0 as usize][p.1 as usize] = m;
                }
                (self.trap_id(p), None)
            }
            "flashfire" => ("type_power_rose".to_string(), Some(Detail::Id("fire".into()))),
            id => (id.to_string(), det),
        };
        cx.push(Event::Volatile { who: cx.m(p), effect, detail: det });
    }

    #[allow(clippy::too_many_arguments)]
    fn item(&mut self, cx: &mut Ctx, rows: &[Row], i: usize, p: Who, item: ItemId, what: ItemWhat, of: Option<Who>, by: Fx) {
        let ev = |cx: &Ctx, what: EvItem, of: Option<Who>| Event::Item { who: Some(cx.m(p)), item: item.id().into(), what, of: of.map(|w| cx.m(w)) };
        match what {
            ItemWhat::Activated => {
                cx.push(ev(cx, EvItem::Activated, None));
                if item.id() == "quickclaw" {
                    cx.push(ev(cx, EvItem::Activated, None));
                }
            }
            ItemWhat::Used => {
                // a resist Berry's second row ("weakened the damage") is the Activated row after this one
                cx.held(p, item);
                if item.id() == "whiteherb" {
                    cx.push(Event::BoostReset { who: Some(cx.m(p)), what: ResetWhat::Negative, of: None, stats: None });
                } else if used_sentence(item) {
                    if item.id().ends_with("gem") {
                        cx.push(Event::Item { who: None, item: item.id().into(), what: EvItem::Used, of: None });
                    } else {
                        cx.push(ev(cx, EvItem::Used, of));
                    }
                }
            }
            ItemWhat::Removed => {
                if let Fx::Ability(a) = by {
                    cx.ability(of.unwrap_or(p), a);
                }
                // "X stole and ate its target's ITEM!" names only the eater
                if let (Fx::Move(m), Some(e)) = (by, of) {
                    if m == moves::BUGBITE || m == moves::PLUCK {
                        cx.push(Event::Item { who: None, item: item.id().into(), what: EvItem::Removed, of: Some(cx.m(e)) });
                        cx.eaten = Some((e, item, p));
                        return;
                    }
                }
                cx.push(ev(cx, EvItem::Removed, of))
            }
            ItemWhat::Received => {
                // an ability that took it (Pickpocket, Magician) has its row first
                if let Fx::Ability(a) = by {
                    cx.ability(p, a);
                }
                // Trick's "X obtained ITEM." names no giver (the `once` row before names the user)
                let of = if matches!(by, Fx::Move(m) if m == moves::TRICK || m == moves::SWITCHEROO) { None } else { of };
                cx.push(ev(cx, EvItem::Received, of))
            }
            ItemWhat::Found => {
                if let Fx::Move(_) = by {
                    // shown by a move (Poltergeist)
                    cx.push(ev(cx, EvItem::Found, None));
                } else if let Some(f) = of {
                    // Frisk: the frisker found the item
                    cx.push(ev(cx, EvItem::Found, Some(f)));
                } else {
                    // shown on entry (Air Balloon): the Held Item row, then the sentence
                    cx.push(ev(cx, EvItem::Activated, None));
                    cx.push(ev(cx, EvItem::Activated, None));
                }
            }
        }
        let _ = (rows, i);
    }
}

/// The Pokémon whose `item` the row just before took away to `p` (Bug Bite, Pluck eat it at once).
fn stolen_from(cx: &Ctx, p: Who, it: ItemId) -> Option<Mon> {
    // a flung item acting on its target is the thrower's
    if let Some((x, f)) = cx.flung {
        if x == it && f != p {
            return Some(cx.m(f));
        }
    }
    match cx.out.last().map(|x| &x.ev) {
        Some(Event::Item { who: Some(h), item, what: EvItem::Removed, of: Some(o) }) if item == it.id() && *o == cx.m(p) && *h != cx.m(p) => Some(h.clone()),
        _ => None,
    }
}

/// The Berry Bug Bite or Pluck just took is acting on its eater `p`.
fn eaten(cx: &Ctx, p: Who, it: ItemId) -> bool {
    cx.eaten.map_or(false, |(e, x, _)| e == p && x == it)
}

/// The Pokémon whose move an ability of `holder` stopped: the opposing active Pokémon, unless only the holder
/// has the move (Damp stops its holder's own Explosion too).
fn blocked_user(cx: &Ctx, holder: Who, mv: MoveId) -> Who {
    let has = |side: usize, slot: u8| {
        let m = &cx.s0.sides[side].mons[slot as usize];
        m.moves[..m.nmoves as usize].iter().any(|x| x.id == mv)
    };
    let fs = 1 - holder.0 as usize;
    let sd = &cx.s0.sides[fs];
    if !sd.has_active {
        return holder;
    }
    let foe = (fs as u8, sd.active);
    if has(holder.0 as usize, holder.1) && !has(fs, foe.1) {
        holder
    } else {
        foe
    }
}

/// A seat-neutral row as one seat's event.
fn for_seat(ph: &Ph, seat: u8, names: &Names) -> Event {
    let mut e = ph.ev.clone();
    if let Some((w, before, after, max)) = ph.hp {
        if w.0 == seat {
            if let Event::Hp { hp, .. } = &mut e {
                *hp = Hp::Own { before: before as i32, after: after as i32 };
            }
        } else if let Event::Hp { hp, .. } = &mut e {
            *hp = Hp::Opp { before_pct: opp_pct(before, max), after_pct: opp_pct(after, max) };
        }
    }
    if seat == 1 {
        battle::event::each_side(&mut e, |s| *s = s.other());
    }
    // the opponent under Illusion is named as the Pokémon it shows
    let o = 1 - seat as usize;
    if let (Some((real, shown)), false) = (ph.disguise[o], ph.real) {
        let (real, shown) = (names.ids[o][real as usize], names.ids[o][shown as usize]);
        battle::event::each_mon(&mut e, |m| {
            if m.side == Side::Opp && m.mon == real {
                m.mon = shown.to_string();
            }
        });
    }
    e
}

/// A brought or unbrought set as `battle_start` lists it: the stats the game shows, nature and points.
pub fn set_msg(spec: &MonSpec) -> SetMsg {
    let m = build_mon(spec);
    SetMsg {
        species: spec.species.id().into(),
        item: if spec.item == ItemId::NONE { None } else { Some(spec.item.id().to_string()) },
        ability: spec.ability.id().into(),
        moves: spec.moves[..spec.nmoves as usize].iter().map(|x| x.id().to_string()).collect(),
        nature: NATURES.get(spec.nature as usize).map(|n| n.id.to_string()),
        points: Some(spec.evs.map(|x| x as i32)),
        stats: [m.maxhp as i32, m.stats[0] as i32, m.stats[1] as i32, m.stats[2] as i32, m.stats[3] as i32, m.stats[4] as i32],
    }
}

/// `battle_start` for one seat: its six sets, and the opponent's six species as preview shows them.
pub fn battle_start(own: &[MonSpec], opp: &[MonSpec]) -> battle::Body {
    battle::Body::BattleStart {
        own_team: own.iter().map(set_msg).collect(),
        opp_preview: opp.iter().map(|s| Some(s.species.id().to_string())).collect(),
    }
}
