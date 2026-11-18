//! Replays recorded battles (JSON lines of contract messages, one battle per file) through the Battle
//! builder and prints one JSON summary line per battle: events per kind, anomalies, HP agreement, and at
//! every decision whether the Order taken was inside `legal(&Battle, kind)`.
//!
//! Usage: replay FILE... | replay @LIST (a file of paths, one per line)
//! A first line {"meta":{...,"row_ids":[...]}} maps row events to recorded row ids. Stream shapes that
//! differ from the contract are adapted here and counted under `adapt`.

use battle::json::Json;
use battle::legal::{move_blocked, trapped};
use battle::*;
use cg_engine::sim::data::*;
use std::collections::BTreeMap;

const EXAMPLES: usize = 3;

fn is_id(t: &str) -> bool {
    !t.is_empty()
        && (SpeciesId::from_id(t).is_some()
            || MoveId::from_id(t).map_or(false, |m| !m.is_none())
            || ItemId::from_id(t).map_or(false, |i| i.0 != 0)
            || AbilityId::from_id(t).map_or(false, |a| a.0 != 0)
            || Cond::from_id(t).map_or(false, |c| !c.is_none()))
}

/// Anomaly text with ids and numbers taken out, as its kind.
fn kind_of(what: &str) -> String {
    what.split(' ')
        .map(|w| {
            let core = w.trim_matches(|c: char| c == ',' || c == ':' || c == '(' || c == ')' || c == '"');
            if core.chars().any(|c| c.is_ascii_digit()) {
                "N".to_string()
            } else if is_id(core) || core.starts_with("Some") {
                "X".to_string()
            } else {
                w.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Default)]
struct Kinds {
    n: BTreeMap<String, (u64, Vec<Json>)>,
}

impl Kinds {
    fn add(&mut self, k: &str, ex: impl FnOnce() -> Json) {
        let e = self.n.entry(k.to_string()).or_insert((0, vec![]));
        e.0 += 1;
        if e.1.len() < EXAMPLES {
            e.1.push(ex());
        }
    }
    fn json(&self) -> Json {
        let mut o = Json::obj();
        for (k, (n, ex)) in &self.n {
            let mut x = Json::obj();
            x.set("n", *n as i64);
            x.set("examples", ex.clone());
            o.set(k, x);
        }
        o
    }
}

/// Contract shapes the stream does not yet follow, adapted before parsing.
fn adapt(o: &mut Json, ad: &mut BTreeMap<&'static str, i64>) {
    let Json::Obj(m) = o else { return };
    if m.get("v").and_then(|v| v.as_str()) == Some("0.2") {
        m.insert("v".into(), Json::from(CONTRACT_VERSION));
        *ad.entry("S1 v 0.2").or_default() += 1;
    }
    if let Some(Json::Obj(f)) = m.get("fallback") {
        if !f.contains_key("kind") && !f.contains_key("order") {
            m.remove("fallback");
            *ad.entry("S3 fallback with no order").or_default() += 1;
        }
    }
    if let Some(Json::Arr(evs)) = m.get_mut("events") {
        for e in evs.iter_mut() {
            let Json::Obj(e) = e else { continue };
            let k = e.get("k").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if (k == "boost" || k == "boost_limit") && e.contains_key("stat") && !e.contains_key("stats") {
                let s = e.remove("stat").unwrap();
                e.insert("stats".into(), Json::Arr(vec![s]));
                *ad.entry("S2 single stat as `stat`").or_default() += 1;
            }
            if k == "blocked" && e.contains_key("side") && !e.contains_key("mon") {
                e.remove("side");
                *ad.entry("S4 blocked with side and no mon").or_default() += 1;
            }
            if k == "volatile_hit" && e.get("effect").and_then(|x| x.as_str()) == Some("confusion_self_hit") {
                *ad.entry("S7 volatile_hit effect confusion_self_hit").or_default() += 1;
            }
        }
    }
}

fn lookup(b: &Battle, m: &Mon) -> Option<Who> {
    let sp = SpeciesId::from_id(&m.mon)?;
    let t = &b.side(m.side).team;
    (0..6).find(|&i| t[i].species == Some(sp) || t[i].form == Some(sp)).map(|i| Who { side: m.side, slot: i as u8 })
}

fn lock_name(l: Lock) -> String {
    match l {
        Lock::None => "none".into(),
        Lock::Locked { mv, uses, .. } => format!("locked {} x{}", mv.id(), uses),
        Lock::Charging { mv, .. } => format!("charging {}", mv.map_or("?", |m| m.id())),
        Lock::Recharge { .. } => "recharge".into(),
    }
}

/// Why an Order is not in the legal set.
fn why_not(b: &Battle, l: &Legal, kind: ReqKind, o: &Order) -> String {
    let own = b.side(Side::Own);
    match o {
        Order::Move { mv, mega } => {
            if kind == ReqKind::Replace {
                return "move at a replace request".into();
            }
            let Some(w) = b.active(Side::Own) else { return "move with no own Pokémon active".into() };
            if let Some(f) = l.forced {
                return format!("forced {:?}, lock {}", f, lock_name(own.active.lock));
            }
            let moves = if own.active.transformed.is_some() { own.active.t_moves } else { b.mon(w).moves };
            let Some(id) = MoveId::from_id(mv) else { return format!("unknown move {}", mv) };
            let Some(i) = moves.iter().position(|m| *m == Some(id)) else {
                return "move not among the active Pokémon's moves".into();
            };
            if let Some(r) = move_blocked(b, i) {
                return format!("move blocked: {}", r);
            }
            if l.mask[..4].iter().filter(|x| **x).count() == 1 && !l.mask[i] {
                return format!("lock {}", lock_name(own.active.lock));
            }
            if *mega && !l.can_mega {
                let mon = b.mon(w);
                return format!(
                    "Mega not allowed (mega_used {}, mega {}, item {:?})",
                    own.mega_used,
                    mon.mega,
                    match mon.item {
                        ItemState::Held(i) => i.id(),
                        ItemState::Gone(_) => "gone",
                        ItemState::Unknown => "unknown",
                    }
                );
            }
            "move: other".into()
        }
        Order::Switch { mon } => {
            let Some(sp) = SpeciesId::from_id(mon) else { return format!("unknown species {}", mon) };
            let Some(slot) = (0..6).find(|&i| own.team[i].species == Some(sp)) else { return "switch target not in the team".into() };
            let Some(p) = b.picks.iter().position(|x| *x == Some(slot as u8)) else { return "switch target not picked".into() };
            if own.team[slot].fainted {
                return "switch target fainted".into();
            }
            if own.active.slot == Some(slot as u8) && !own.active.out {
                return "switch target is the active Pokémon".into();
            }
            if own.active.slot == Some(slot as u8) {
                return "switch target is the Pokémon that just left".into();
            }
            if b.refused[8 + p] {
                return "switch refused earlier this request".into();
            }
            if kind == ReqKind::Turn && trapped(b) {
                let vols: Vec<&str> = own.active.vols.iter().flatten().map(|v| v.cond.id()).collect();
                let foe = b.active(Side::Opp).and_then(|f| b.mon(f).ability).map_or("?", |a| a.id());
                return format!("trapped (vols {:?}, foe ability {})", vols, foe);
            }
            if kind == ReqKind::Turn && l.forced.is_some() {
                return format!("forced {:?}", l.forced);
            }
            "switch: other".into()
        }
    }
}

struct Pending {
    seq: i64,
    kind: ReqKind,
    order: Order,
    inside: bool,
    why: String,
    turn: u16,
    incomplete: bool,
    legal: Vec<Json>,
}

fn legal_orders(b: &Battle, l: &Legal) -> Vec<Order> {
    (0..N_ACTIONS).filter_map(|a| order_for(b, l, a)).collect()
}

fn run(path: &str) -> Json {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut bld = Builder::new();
    let mut meta = Json::Null;
    let mut ad: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut kinds: BTreeMap<String, i64> = BTreeMap::new();
    let mut anomalies = Kinds::default();
    let mut anomalies_after_gap = Kinds::default();
    let mut parse = Kinds::default();
    let mut decisions = Kinds::default();
    let mut misses = Kinds::default();
    let mut refusals = Kinds::default();
    let mut hp = Kinds::default();
    let mut hp_rows = 0i64;
    let mut row_i = 0usize;
    let mut req: Option<(i64, ReqKind, Legal)> = None;
    let mut pending: Option<Pending> = None;
    let mut fallback_seqs: Vec<i64> = vec![];
    // row events and turn events since the last answer
    let mut since_rows = 0i64;
    let mut since_turn = false;
    // per turn: had a turn request, row events, lock at its start, own moves
    let mut turn_info: Vec<(u32, bool, i64, String, Vec<String>, bool)> = vec![];
    let mut gaps_turn = 0i64;
    let mut gaps_rows = 0i64;
    let mut last_turn_gap: Option<u32> = None;
    let mut max_turn = 0u32;

    let settle = |p: Pending, outcome: &str, decisions: &mut Kinds, misses: &mut Kinds, refusals: &mut Kinds| {
        let key = format!("{} {} {}", p.kind.id(), outcome, if p.inside { "inside" } else { "outside" });
        let ex = || {
            let mut x = Json::obj();
            x.set("seq", p.seq);
            x.set("turn", p.turn as i64);
            x.set("order", p.order.to_json());
            x.set("why", p.why.as_str());
            x.set("incomplete", p.incomplete);
            x.set("legal", p.legal.clone());
            x
        };
        decisions.add(&key, ex);
        if outcome == "accepted" && !p.inside {
            misses.add(&kind_of(&p.why), ex);
        }
        if outcome.starts_with("refused") || outcome == "re-asked" {
            let k = format!("{}: {}", outcome, if p.inside { "not predicted" } else { "predicted" });
            refusals.add(&k, ex);
        }
    };

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut j = match Json::parse(line) {
            Ok(j) => j,
            Err(e) => {
                parse.add("bad json", || Json::from(e.as_str()));
                continue;
            }
        };
        if let Some(m) = j.get("meta") {
            meta = m.clone();
            continue;
        }
        if j.get("type").is_none() && j.get("picks").is_some() {
            match Picks::from_json(&j) {
                Ok(p) => bld.apply(&Input::Picks(p)),
                Err(e) => parse.add(&format!("picks: {}", e), || j.clone()),
            }
            continue;
        }
        if j.get("type").is_none() && j.get("order").is_some() {
            // CG's answer
            match Answer::from_json(&j) {
                Ok(a) => {
                    let (inside, why, legal) = match &req {
                        Some((seq, kind, l)) if *seq == a.seq => {
                            let lo = legal_orders(&bld.battle, l);
                            let inside = lo.contains(&a.order);
                            let why = if inside { String::new() } else { why_not(&bld.battle, l, *kind, &a.order) };
                            (inside, why, lo.iter().map(|o| o.to_json()).collect())
                        }
                        _ => (false, "answer with no open request".to_string(), vec![]),
                    };
                    if let Some(p) = pending.take() {
                        settle(p, "superseded", &mut decisions, &mut misses, &mut refusals);
                    }
                    let kind = req.as_ref().map_or(ReqKind::Turn, |r| r.1);
                    since_rows = 0;
                    since_turn = false;
                    pending = Some(Pending { seq: a.seq, kind, order: a.order, inside, why, turn: bld.battle.turn, incomplete: bld.battle.incomplete, legal });
                }
                Err(e) => parse.add(&format!("answer: {}", e), || j.clone()),
            }
            continue;
        }
        if let Ok(Some(fs)) = Fallback::list(j.get("fallback").filter(|x| !x.is_null())) {
            fallback_seqs.extend(fs.iter().filter_map(|f| f.seq));
        }
        adapt(&mut j, &mut ad);
        let msg = match Message::from_json(&j) {
            Ok(m) => m,
            Err(e) => {
                // try the events one by one so one bad event does not drop the message
                if j.get("type").and_then(|t| t.as_str()) == Some("events") {
                    parse.add(&format!("events message: {}", e), || Json::from(e.as_str()));
                    let evs = j.get("events").and_then(|x| x.as_arr()).cloned().unwrap_or_default();
                    let mut ok = vec![];
                    for ev in evs {
                        match Event::from_json(&ev) {
                            Ok(x) => ok.push(x),
                            Err(e2) => parse.add(&format!("event: {}", e2), || ev.clone()),
                        }
                    }
                    Message { v: CONTRACT_VERSION.into(), battle: String::new(), body: Body::Events { events: ok, fallback: None } }
                } else {
                    parse.add(&format!("message: {}", e), || Json::from(e.as_str()));
                    continue;
                }
            }
        };
        match &msg.body {
            Body::DecisionRequest { seq, kind, refused, .. } => {
                if let Some(p) = pending.take() {
                    // asked again in the same turn with nothing read between: the game did not take it
                    let outcome = if refused.as_ref() == Some(&p.order) {
                        "refused"
                    } else if refused.is_some() {
                        "refused (another order named)"
                    } else if *kind == p.kind && since_rows == 0 && !since_turn {
                        "re-asked"
                    } else if fallback_seqs.contains(&p.seq) {
                        "fallback"
                    } else {
                        "accepted"
                    };
                    settle(p, outcome, &mut decisions, &mut misses, &mut refusals);
                }
                if *kind == ReqKind::Turn {
                    if let Some(t) = turn_info.last_mut() {
                        t.1 = true;
                    }
                }
                bld.apply(&Input::Msg(msg.clone()));
                req = Some((*seq, *kind, legal(&bld.battle, *kind)));
            }
            Body::Events { events, .. } => {
                for e in events {
                    *kinds.entry(e.kind().to_string()).or_default() += 1;
                    let is_row = !matches!(e, Event::Turn { .. } | Event::Refused { .. } | Event::End { .. } | Event::Gap { .. });
                    let row = if is_row {
                        let r = meta.get("row_ids").and_then(|r| r.as_arr()).and_then(|r| r.get(row_i)).cloned().unwrap_or(Json::Null);
                        row_i += 1;
                        r
                    } else {
                        Json::Null
                    };
                    if is_row {
                        since_rows += 1;
                    }
                    match e {
                        Event::Turn { n } => {
                            since_turn = true;
                            max_turn = max_turn.max(*n);
                        }
                        Event::Gap { what, n } => {
                            if *what == GapWhat::Turn {
                                gaps_turn += 1;
                                last_turn_gap = *n;
                                if let Some(t) = turn_info.last_mut() {
                                    t.5 = true;
                                }
                            } else {
                                gaps_rows += 1;
                            }
                        }
                        Event::Hp { who, hp: h, .. } => {
                            hp_rows += 1;
                            if let Some(w) = lookup(&bld.battle, who) {
                                let held = bld.battle.mon(w).hp;
                                let before = match h {
                                    Hp::Own { before, .. } => {
                                        let max = bld.battle.sets[w.slot as usize].stats[0].max(1) as f32;
                                        *before as f32 * 100.0 / max
                                    }
                                    Hp::Opp { before_pct, .. } => *before_pct as f32,
                                };
                                let tol = if who.side == Side::Own { 0.01 } else { 1.0 };
                                let d = (held - before).abs();
                                if d > tol {
                                    let band = if d <= 5.0 { "<=5" } else if d <= 20.0 { "<=20" } else { ">20" };
                                    let k = format!("{} {} {}", who.side.id(), band, if bld.battle.incomplete { "after a gap" } else { "no gap before" });
                                    hp.add(&k, || {
                                        let mut x = Json::obj();
                                        x.set("row", row.clone());
                                        x.set("held", held as f64);
                                        x.set("row_before", before as f64);
                                        x
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                    let before = bld.anomalies.len();
                    let was_incomplete = bld.battle.incomplete;
                    bld.apply_event(e);
                    if let Event::Turn { n } = e {
                        let a = &bld.battle.side(Side::Own).active;
                        turn_info.push((*n, false, 0, lock_name(a.lock), vec![], false));
                    } else if is_row {
                        if let Some(t) = turn_info.last_mut() {
                            t.2 += 1;
                            if let Event::Move { who, mv, .. } = e {
                                if who.side == Side::Own {
                                    t.4.push(mv.clone());
                                }
                            }
                        }
                    }
                    for a in bld.anomalies[before..].to_vec() {
                        let k = kind_of(&a.what);
                        let target = if was_incomplete { &mut anomalies_after_gap } else { &mut anomalies };
                        target.add(&k, || {
                            let mut x = Json::obj();
                            x.set("what", a.what.as_str());
                            x.set("row", row.clone());
                            x.set("turn", bld.battle.turn as i64);
                            x.set("event", e.to_json());
                            x
                        });
                    }
                }
            }
            _ => {
                let before = bld.anomalies.len();
                bld.apply(&Input::Msg(msg.clone()));
                // the three brought, when the controller gives them (S8-1)
                if let (Body::BattleStart { .. }, Ok(Some(p))) = (&msg.body, Picks::field(&j, "picks")) {
                    bld.apply(&Input::Picks(p));
                }
                for a in bld.anomalies[before..].to_vec() {
                    anomalies.add(&kind_of(&a.what), || Json::from(a.what.as_str()));
                }
            }
        }
    }
    if let Some(p) = pending.take() {
        let outcome = if fallback_seqs.contains(&p.seq) {
            "fallback"
        } else if since_rows == 0 {
            "unconfirmed (no row after it)"
        } else {
            "accepted"
        };
        settle(p, outcome, &mut decisions, &mut misses, &mut refusals);
    }

    let mut out = Json::obj();
    out.set("file", path);
    for k in ["run", "folder", "battle"] {
        if let Some(v) = meta.get(k) {
            out.set(k, v.clone());
        }
    }
    let mut kj = Json::obj();
    for (k, n) in &kinds {
        kj.set(k, *n);
    }
    out.set("events", kj);
    out.set("rows", row_i as i64);
    out.set("unread", bld.battle.unread as i64);
    out.set("turns", max_turn as i64);
    out.set("gaps_turn", gaps_turn);
    out.set("gaps_rows", gaps_rows);
    // a gap other than the final turn's
    let mid = gaps_rows > 0 || gaps_turn > 1 || (gaps_turn == 1 && last_turn_gap != Some(max_turn));
    out.set("gap_before_end", mid);
    out.set("anomalies", anomalies.json());
    out.set("anomalies_after_gap", anomalies_after_gap.json());
    out.set("parse", parse.json());
    out.set("decisions", decisions.json());
    out.set("misses", misses.json());
    out.set("refusals", refusals.json());
    out.set("hp_rows", hp_rows);
    out.set("hp_disagree", hp.json());
    let mut aj = Json::obj();
    for (k, n) in &ad {
        aj.set(k, *n);
    }
    out.set("adapt", aj);
    let nr: Vec<Json> = turn_info
        .iter()
        .filter(|t| !t.1 && t.2 > 0)
        .map(|t| {
            let mut x = Json::obj();
            x.set("n", t.0 as i64);
            x.set("rows", t.2);
            x.set("lock", t.3.as_str());
            x.set("own_moves", t.4.iter().map(|m| Json::from(m.as_str())).collect::<Vec<_>>());
            x
        })
        .collect();
    out.set("turns_without_request", nr);
    let skipped: Vec<Json> = turn_info.iter().filter(|t| !t.1 && t.2 == 0 && t.0 != max_turn).map(|t| {
        let mut x = Json::obj();
        x.set("n", t.0 as i64);
        x.set("lock", t.3.as_str());
        x.set("gap", t.5);
        x
    }).collect();
    out.set("turns_empty_without_request", skipped);
    out.set("result", match bld.battle.result {
        Some(r) => Json::from(format!("{:?}", r)),
        None => Json::Null,
    });
    out
}

fn main() {
    let mut files = vec![];
    for a in std::env::args().skip(1) {
        if let Some(list) = a.strip_prefix('@') {
            for l in std::fs::read_to_string(list).unwrap_or_default().lines() {
                if !l.trim().is_empty() {
                    files.push(l.trim().to_string());
                }
            }
        } else {
            files.push(a);
        }
    }
    for f in files {
        println!("{}", run(&f).to_string());
    }
}
