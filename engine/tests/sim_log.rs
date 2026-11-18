//! The battle log's rows (sim/log.rs): scripted battles through the public API with the rows recorded, each
//! asserting the exact row sequence of a step. Every row kind appears in at least one test (listed per test).

use cg_engine::sim::data::*;
use cg_engine::sim::log::{Detail, Fx, How, ItemWhat, Reset, SideWhat};
use cg_engine::sim::outcome::{EffectRef, Key, Outcomes, Site};
use cg_engine::sim::state::{State, Winner};
use cg_engine::sim::{Choice, MonSpec, Row};

/// Accuracy hits unless `miss`; no crits; chance effects as `procs`; lowest rolls and hit counts; first of a
/// tie or sample.
#[derive(Default)]
struct Script {
    procs: bool,
    miss: bool,
}

impl Outcomes for Script {
    fn chance(&mut self, site: &Site, _: u32, _: u32) -> bool {
        match site.key {
            Key::Accuracy => !self.miss,
            Key::Protect => true,
            Key::Crit => false,
            _ => self.procs,
        }
    }
    fn range(&mut self, _: &Site, lo: i32, _: i32) -> i32 {
        lo
    }
    fn sample(&mut self, _: &Site, values: &[i32]) -> i32 {
        values[0]
    }
    fn tie(&mut self, _: &Site, a: u8, _: u8) -> u8 {
        a
    }
    fn tie_effects(&mut self, _: &Site, _: EffectRef, _: EffectRef) -> bool {
        true
    }
    fn sample_str(&mut self, _: &Site, _: &[&'static str]) -> usize {
        0
    }
}

/// species, ability, item, moves
type Set<'a> = (&'a str, &'a str, &'a str, &'a [&'a str]);

fn spec(s: &Set) -> MonSpec {
    let mut m = [MoveId::NONE; 4];
    for (i, x) in s.3.iter().enumerate() {
        m[i] = MoveId::from_id(x).unwrap_or_else(|| panic!("move {}", x));
    }
    MonSpec {
        species: SpeciesId::from_id(s.0).unwrap_or_else(|| panic!("species {}", s.0)),
        ability: AbilityId::from_id(s.1).unwrap_or_else(|| panic!("ability {}", s.1)),
        item: if s.2.is_empty() { ItemId::NONE } else { ItemId::from_id(s.2).unwrap_or_else(|| panic!("item {}", s.2)) },
        moves: m,
        nmoves: s.3.len() as u8,
        nature: nature_from_id("serious").unwrap(),
        evs: [0; 6],
        gender: 1,
    }
}

/// Teams of up to three, brought in that order (the rest of the six repeat slot 0 without its item); the
/// rows of the leads' entry.
fn battle(p1: &[Set], p2: &[Set], o: &mut Script) -> (State, Vec<Row>) {
    let team = |t: &[Set]| -> Vec<MonSpec> {
        let mut v: Vec<MonSpec> = t.iter().map(spec).collect();
        while v.len() < 6 {
            let mut x = spec(&t[0]);
            x.item = ItemId::NONE;
            v.push(x);
        }
        v
    };
    let mut rows = Vec::new();
    let st = State::start_logged([&team(p1), &team(p2)], [[0, 1, 2], [0, 1, 2]], o, &mut rows).unwrap();
    (st, rows)
}

fn step(st: &mut State, c: [Option<Choice>; 2], o: &mut Script) -> Vec<String> {
    let mut rows = Vec::new();
    st.step_logged(c, o, &mut rows).unwrap();
    rows.iter().map(show).collect()
}

fn m(i: u8) -> Option<Choice> {
    Some(Choice::Move { index: i, mega: false })
}

fn sw(s: u8) -> Option<Choice> {
    Some(Choice::Switch(s))
}

fn w(p: (u8, u8)) -> String {
    format!("{}:{}", p.0, p.1)
}

fn ow(p: Option<(u8, u8)>) -> String {
    p.map(w).unwrap_or_else(|| "-".into())
}

fn fx(f: Fx) -> String {
    match f {
        Fx::None => "-".into(),
        Fx::Move(x) => format!("move:{}", x.id()),
        Fx::Ability(x) => format!("ability:{}", x.id()),
        Fx::Item(x) => format!("item:{}", x.id()),
        Fx::Cond(x) => format!("cond:{}", x.id()),
        Fx::Status(x) => format!("status:{}", x.id()),
        Fx::Recoil(x) => format!("recoil:{}", x.id()),
        Fx::Drain(x) => format!("drain:{}", x.id()),
        Fx::Crash(x) => format!("crash:{}", x.id()),
        Fx::Cost(x) => format!("cost:{}", x.id()),
        Fx::Confusion => "confusion".into(),
        Fx::HpFull => "hpfull".into(),
    }
}

fn stats(b: [i8; 7]) -> String {
    let mut v = Vec::new();
    for (i, x) in b.iter().enumerate() {
        if *x != 0 {
            v.push(format!("{}{:+}", BOOST_IDS[i], x));
        }
    }
    v.join(",")
}

fn det(d: Detail) -> String {
    match d {
        Detail::None => "-".into(),
        Detail::Move(x) => x.id().into(),
        Detail::Num(n) => n.to_string(),
        Detail::Type(t) => t.id().into(),
    }
}

/// A row in one line: kind, Pokémon, then what it says.
fn show(r: &Row) -> String {
    match *r {
        Row::Turn(n) => format!("turn {}", n),
        Row::Enter { p, forced } => format!("enter {}{}", w(p), if forced { " forced" } else { "" }),
        Row::Withdraw { p, pivot } => format!("withdraw {}{}", w(p), pivot.map(|x| format!(" pivot:{}", x.id())).unwrap_or_default()),
        Row::Move { p, mv, called_by, .. } => format!("move {} {}{}", w(p), mv.id(), if called_by == Fx::None { String::new() } else { format!(" by {}", fx(called_by)) }),
        Row::Prepare { p, mv } => format!("prepare {} {}", w(p), mv.id()),
        Row::Cant { p, why, mv } => format!("cant {} {}{}", w(p), fx(why), if mv.is_none() { String::new() } else { format!(" {}", mv.id()) }),
        Row::Protect { p, by } => format!("protect {} {}", w(p), by.id()),
        Row::Miss { p, by } => format!("miss {} by {}", w(p), w(by)),
        Row::Fail { p, why } => format!("fail {} {}", ow(p), fx(why)),
        Row::Result { how, p } => format!("result {} {}", ow(p), match how {
            How::Super => "super",
            How::Extreme => "extreme",
            How::Resisted => "resisted",
            How::Mostly => "mostly",
            How::Immune => "immune",
            How::Ohko => "ohko",
        }),
        Row::Hits { n, p } => format!("hits {} {}", w(p), n),
        Row::Hp { p, before, after, cause, of } => format!("hp {} {}>{} {} of {}", w(p), before, after, fx(cause), ow(of)),
        Row::Faint { p } => format!("faint {}", w(p)),
        Row::Status { p, st, by, rest, .. } => format!("status {} {} {}{}", w(p), st.id(), fx(by), if rest { " rest" } else { "" }),
        Row::Cure { p, st, by } => format!("cure {} {} {}", w(p), st.id(), fx(by)),
        Row::Volatile { p, c, detail, by, of } => format!("volatile {} {} {} {} of {}", w(p), c.id(), det(detail), fx(by), ow(of)),
        Row::VolatileEnd { p, c, by } => format!("volatile_end {} {} {}", w(p), c.id(), fx(by)),
        Row::VolatileActed { p, c, of, .. } => format!("volatile_acted {} {} of {}", w(p), c.id(), ow(of)),
        Row::Boost { p, by_stat, by } => format!("boost {} {} {}", w(p), stats(by_stat), fx(by)),
        Row::BoostLimit { p, stats: s, up, .. } => {
            let n: Vec<&str> = (0..7).filter(|&i| s[i]).map(|i| BOOST_IDS[i]).collect();
            format!("boost_limit {} {} {}", w(p), n.join(","), if up { "up" } else { "down" })
        }
        Row::BoostReset { p, what, .. } => format!("boost_reset {} {}", ow(p), match what {
            Reset::Field => "field",
            Reset::All => "all",
            Reset::Negative => "negative",
            Reset::Positive => "positive",
            Reset::Copy => "copy",
            Reset::Swap => "swap",
            Reset::Invert => "invert",
        }),
        Row::Blocked { p, by, what } => format!("blocked {} {} {}", ow(p), fx(by), fx(what)),
        Row::Ability { p, ab, .. } => format!("ability {} {}", w(p), ab.id()),
        Row::AbilityChange { p, from, to, by } => format!("ability_change {} {}>{} {}", w(p), from.id(), to.id(), fx(by)),
        Row::Item { p, item, what, of, .. } => format!("item {} {} {} of {}", w(p), item.id(), match what {
            ItemWhat::Activated => "activated",
            ItemWhat::Used => "used",
            ItemWhat::Removed => "removed",
            ItemWhat::Received => "received",
            ItemWhat::Found => "found",
        }, ow(of)),
        Row::MegaStone { p, stone } => format!("mega_stone {} {}", w(p), stone.id()),
        Row::Mega { p, form } => format!("mega {} {}", w(p), form.id()),
        Row::Form { p, form, .. } => format!("form {} {}", w(p), form.id()),
        Row::Transform { p, into } => format!("transform {} into {}", w(p), w(into)),
        Row::Types { p, types, lost, added, .. } => {
            let t: Vec<&str> = types.iter().filter(|t| t.is_real()).map(|t| t.id()).collect();
            format!("types {} {}{}{}", w(p), t.join("/"), if lost.is_real() { format!(" lost:{}", lost.id()) } else { String::new() },
                if added.is_real() { format!(" added:{}", added.id()) } else { String::new() })
        }
        Row::Weather { w: x, by, .. } => format!("weather {} {}", if x.is_none() { "none" } else { x.id() }, fx(by)),
        Row::Field { c, start, .. } => format!("field {} {}", c.id(), if start { "start" } else { "end" }),
        Row::Side { side, c, what, p, .. } => format!("side {} {} {}{}", side, c.id(), match what {
            SideWhat::Start => "start",
            SideWhat::End => "end",
            SideWhat::Acted => "acted",
        }, p.map(|x| format!(" {}", w(x))).unwrap_or_default()),
        Row::Once { p, fx: f, .. } => format!("once {} {}", ow(p), fx(f)),
        Row::Pp { p, mv, lost, .. } => format!("pp {} {} {}", w(p), mv.id(), lost),
        Row::Revive { p } => format!("revive {}", w(p)),
        Row::End { winner } => format!("end {}", match winner {
            Winner::Side(s) => s.to_string(),
            Winner::Tie => "tie".into(),
            Winner::None => "-".into(),
        }),
    }
}

fn check(got: Vec<String>, want: &[&str]) {
    let want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, want, "\ngot:\n{}\n", got.join("\n"));
}

/// The other two of a side fainted, so the battle ends with its lead.
fn last_one(st: &mut State, side: usize) {
    for slot in 1..3 {
        let m = &mut st.sides[side].mons[slot];
        m.hp = 0;
        m.fainted = true;
    }
    st.sides[side].pokemon_left = 1;
}

const WAIT: &[&str] = &["calmmind"];
const STRUGGLE_BOT: Set = ("snorlax", "noability", "", WAIT);

// ---------------------------------------------------------------------------------------------- tests

/// enter, turn (the leads), move, boost, result (extreme), hp, faint, end
#[test]
fn enter_move_effectiveness_faint_end() {
    let mut o = Script::default();
    let (mut st, rows) = battle(&[("primarina", "noability", "", &["icebeam"])], &[("garchomp", "noability", "", WAIT), STRUGGLE_BOT], &mut o);
    check(rows.iter().map(show).collect(), &["enter 0:0", "enter 1:0", "turn 1"]);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "move 0:0 icebeam",
        "result 1:0 extreme",
        "hp 1:0 183>31 move:icebeam of 0:0",
        "turn 2",
    ]);
    last_one(&mut st, 1);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "move 0:0 icebeam",
        "result 1:0 extreme",
        "hp 1:0 31>0 move:icebeam of 0:0",
        "faint 1:0",
        "end 0",
    ]);
}

/// withdraw (a chosen switch, a pivot move's), enter, result (resisted)
#[test]
fn withdraw_on_switch_and_pivot() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("greninja", "noability", "", &["uturn"]), ("snorlax", "noability", "", WAIT)], &[("snorlax", "noability", "", WAIT), ("garchomp", "noability", "", WAIT)], &mut o);
    check(step(&mut st, [m(0), sw(1)], &mut o), &["withdraw 1:0", "enter 1:1", "move 0:0 uturn", "hp 1:1 183>151 move:uturn of 0:0"]);
    check(step(&mut st, [sw(1), None], &mut o), &["withdraw 0:0 pivot:uturn", "enter 0:1", "turn 2"]);
}

/// cant (sleep), a move called by another (Sleep Talk)
#[test]
fn sleep_talk_calls_a_move() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("snorlax", "noability", "", &["sleeptalk", "bodyslam"])], &[("garchomp", "noability", "", WAIT)], &mut o);
    st.sides[0].mons[0].status = Status::Slp;
    st.sides[0].mons[0].status_time = 3;
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "cant 0:0 status:slp",
        "move 0:0 sleeptalk",
        "move 0:0 bodyslam by move:sleeptalk",
        "hp 1:0 183>117 move:bodyslam of 0:0",
        "turn 2",
    ]);
}

/// cant (flinch), prepare (the charge turn)
#[test]
fn charge_turn_and_flinch() {
    let mut o = Script { procs: true, ..Script::default() };
    let (mut st, _) = battle(&[("venusaur", "noability", "", &["solarbeam"])], &[("incineroar", "noability", "", &["fakeout", "calmmind"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 1:0 fakeout", "hp 0:0 155>130 move:fakeout of 1:0", "cant 0:0 cond:flinch", "turn 2"]);
    o.procs = false;
    check(step(&mut st, [m(0), m(1)], &mut o), &[
        "move 0:0 solarbeam",
        "prepare 0:0 solarbeam",
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "turn 3",
    ]);
}

/// once (Protect's start), protect (it blocked), miss
#[test]
fn protect_and_miss() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("garchomp", "noability", "", &["earthquake", "stoneedge"])], &[("toxapex", "noability", "", &["protect", "calmmind"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 1:0 protect", "once 1:0 cond:protect", "move 0:0 earthquake", "protect 1:0 protect", "turn 2"]);
    o.miss = true;
    check(step(&mut st, [m(1), m(1)], &mut o), &["move 0:0 stoneedge", "miss 1:0 by 0:0", "move 1:0 calmmind", "boost 1:0 spa+1,spd+1 move:calmmind", "turn 3"]);
}

/// fail (HP is full), boost_limit
#[test]
fn failures_and_stat_limits() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("snorlax", "noability", "", &["rest", "swordsdance"])], &[("garchomp", "noability", "", &["swordsdance", "fakeout"])], &mut o);
    st.sides[1].mons[0].boosts[0] = 6;
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 1:0 swordsdance", "boost_limit 1:0 atk up", "move 0:0 rest", "fail 0:0 hpfull", "turn 2"]);
}

/// hits
#[test]
fn hit_count() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("rillaboom", "noability", "", &["bulletseed"])], &[("snorlax", "noability", "", WAIT)], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 bulletseed",
        "hp 1:0 235>205 move:bulletseed of 0:0",
        "hp 1:0 205>175 move:bulletseed of 0:0",
        "hits 1:0 2",
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "turn 2",
    ]);
}

/// status (a move's, Rest's), cure (by an item), item (a berry eaten), fail (HP is full)
#[test]
fn statuses_rest_and_lum() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("gengar", "noability", "", &["willowisp"])], &[("snorlax", "noability", "lumberry", &["rest"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 willowisp",
        "status 1:0 brn move:willowisp",
        "item 1:0 lumberry used of -",
        "cure 1:0 brn item:lumberry",
        "move 1:0 rest",
        "fail 1:0 hpfull",
        "turn 2",
    ]);
    st.sides[1].mons[0].hp = 100;
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 willowisp",
        "status 1:0 brn move:willowisp",
        "move 1:0 rest",
        "status 1:0 slp move:rest rest",
        "hp 1:0 100>235 move:rest of -",
        "turn 3",
    ]);
}

/// volatile (Substitute), hp (its cost), volatile_end (it broke)
#[test]
fn substitute_starts_and_breaks() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("gengar", "noability", "", &["substitute"])], &[("garchomp", "noability", "", &["dragonclaw", "earthquake"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 substitute",
        "volatile 0:0 substitute - move:substitute of -",
        "hp 0:0 135>102 cost:substitute of -",
        "move 1:0 dragonclaw",
        "volatile_end 0:0 substitute -",
        "turn 2",
    ]);
}

/// volatile (confusion), volatile_acted ("is confused", the self-hit), hp (confusion)
#[test]
fn confusion_and_its_self_hit() {
    let mut o = Script { procs: true, ..Script::default() };
    let (mut st, _) = battle(&[("gengar", "noability", "", &["confuseray"])], &[("snorlax", "noability", "", &["bodyslam"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 confuseray",
        "volatile 1:0 confusion - move:confuseray of 0:0",
        "volatile_acted 1:0 confusion of -",
        "hp 1:0 235>207 confusion of -",
        "turn 2",
    ]);
}

/// boost (a row per change of a multi-stat move), boost_reset (Haze)
#[test]
fn shell_smash_and_haze() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("primarina", "noability", "", &["shellsmash"])], &[("toxapex", "noability", "", &["haze"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 shellsmash",
        "boost 0:0 atk+2,spa+2,spe+2 move:shellsmash",
        "boost 0:0 def-1,spd-1 move:shellsmash",
        "move 1:0 haze",
        "boost_reset - field",
        "turn 2",
    ]);
}

/// ability (an announced ability), a boost by an ability, blocked (Clear Body)
#[test]
fn intimidate_and_clear_body() {
    let mut o = Script::default();
    let (_st, rows) = battle(&[("gyarados", "intimidate", "", WAIT)], &[("metagross", "clearbody", "", WAIT)], &mut o);
    check(rows.iter().map(show).collect(), &["enter 0:0", "enter 1:0", "ability 0:0 intimidate", "blocked 1:0 ability:clearbody -", "turn 1"]);
    let (_st, rows) = battle(&[("gyarados", "intimidate", "", WAIT)], &[("garchomp", "noability", "", WAIT)], &mut o);
    check(rows.iter().map(show).collect(), &["enter 0:0", "enter 1:0", "ability 0:0 intimidate", "boost 1:0 atk-1 ability:intimidate", "turn 1"]);
}

/// weather (by an ability), ability_change (by a move), field (a room)
#[test]
fn worry_seed_drought_trick_room() {
    let mut o = Script::default();
    let (mut st, rows) = battle(&[("torkoal", "drought", "", &["trickroom"])], &[("venusaur", "noability", "", &["worryseed"])], &mut o);
    check(rows.iter().map(show).collect(), &["enter 0:0", "enter 1:0", "weather sunnyday ability:drought", "turn 1"]);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 1:0 worryseed",
        "ability_change 0:0 drought>insomnia move:worryseed",
        "move 0:0 trickroom",
        "field trickroom start",
        "turn 2",
    ]);
}

/// item (Air Balloon shown and popped, an item knocked off)
#[test]
fn items_shown_and_removed() {
    let mut o = Script::default();
    let (mut st, rows) = battle(&[("weavile", "noability", "", &["knockoff"])], &[("snorlax", "noability", "airballoon", WAIT)], &mut o);
    check(rows.iter().map(show).collect(), &["enter 0:0", "enter 1:0", "item 1:0 airballoon found of -", "turn 1"]);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 knockoff",
        "hp 1:0 235>127 move:knockoff of 0:0",
        "item 1:0 airballoon removed of -",
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "turn 2",
    ]);
    let (mut st, _) = battle(&[("weavile", "noability", "", &["knockoff"])], &[("snorlax", "noability", "leftovers", WAIT)], &mut o);
    let rows = step(&mut st, [m(0), m(0)], &mut o);
    assert_eq!(rows[2], "item 1:0 leftovers removed of 0:0");
}

/// hp caused by an item (Leftovers)
#[test]
fn leftovers_heal_names_its_item() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("snorlax", "noability", "leftovers", WAIT)], &[("garchomp", "noability", "", &["dragonclaw"])], &mut o);
    let rows = step(&mut st, [m(0), m(0)], &mut o);
    assert!(rows.iter().any(|r| r.starts_with("hp 0:0") && r.contains("item:leftovers")), "{:?}", rows);
}

/// mega_stone, mega
#[test]
fn mega_evolution() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("garchomp", "roughskin", "garchompite", &["swordsdance"])], &[("snorlax", "noability", "", WAIT)], &mut o);
    check(step(&mut st, [Some(Choice::Move { index: 0, mega: true }), m(0)], &mut o), &[
        "mega_stone 0:0 garchompite",
        "mega 0:0 garchompmega",
        "move 0:0 swordsdance",
        "boost 0:0 atk+2 move:swordsdance",
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "turn 2",
    ]);
}

/// ability (Disguise), hp with no change (the busted hit), form, the disguise's cost
#[test]
fn disguise_busts() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("garchomp", "noability", "", &["earthquake"])], &[("mimikyu", "disguise", "", WAIT)], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 earthquake",
        "ability 1:0 disguise",
        "hp 1:0 130>130 move:earthquake of 0:0",
        "form 1:0 mimikyubusted",
        "hp 1:0 130>114 - of -",
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "turn 2",
    ]);
}

/// transform (Imposter), types (Soak)
#[test]
fn imposter_and_soak() {
    let mut o = Script::default();
    let (_st, rows) = battle(&[("ditto", "imposter", "", WAIT)], &[("politoed", "noability", "", &["soak"])], &mut o);
    check(rows.iter().map(show).collect(), &["enter 0:0", "enter 1:0", "ability 0:0 imposter", "transform 0:0 into 1:0", "turn 1"]);
    let (mut st, _) = battle(&[("garchomp", "noability", "", WAIT)], &[("politoed", "noability", "", &["soak"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 0:0 calmmind", "boost 0:0 spa+1,spd+1 move:calmmind", "move 1:0 soak", "types 0:0 water", "turn 2"]);
}

/// side (start, and acting on a switch-in), hp (a hazard's), weather (a move's)
#[test]
fn hazards_and_weather() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("skarmory", "noability", "", &["stealthrock", "stickyweb"])], &[("snorlax", "noability", "", &["raindance"]), ("garchomp", "noability", "", WAIT)], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 0:0 stealthrock", "side 1 stealthrock start", "move 1:0 raindance", "weather raindance -", "turn 2"]);
    check(step(&mut st, [m(1), sw(1)], &mut o), &[
        "withdraw 1:0",
        "enter 1:1",
        "hp 1:1 183>172 cond:stealthrock of -",
        "move 0:0 stickyweb",
        "side 1 stickyweb start",
        "turn 3",
    ]);
    let rows = step(&mut st, [m(0), sw(0)], &mut o);
    check(rows[..5].to_vec(), &["withdraw 1:1", "enter 1:0", "hp 1:0 235>206 cond:stealthrock of -", "side 1 stickyweb acted 1:0", "boost 1:0 spe-1 move:stickyweb"]);
}

/// once (Perish Song's song), volatile (the Perish count)
#[test]
fn perish_song_counts() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("gengar", "noability", "", &["perishsong"])], &[("snorlax", "noability", "", WAIT)], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &[
        "move 0:0 perishsong",
        "once - move:perishsong",
        "move 1:0 calmmind",
        "boost 1:0 spa+1,spd+1 move:calmmind",
        "volatile 0:0 perishsong 3 - of -",
        "volatile 1:0 perishsong 3 - of -",
        "turn 2",
    ]);
}

/// pp (Spite), fail (no last move), result (immune)
#[test]
fn spite_takes_pp() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("gengar", "noability", "", &["spite"])], &[("snorlax", "noability", "", &["bodyslam"])], &mut o);
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 0:0 spite", "fail 0:0 -", "move 1:0 bodyslam", "result 0:0 immune", "turn 2"]);
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 0:0 spite", "pp 1:0 bodyslam 4", "move 1:0 bodyslam", "result 0:0 immune", "turn 3"]);
}

/// revive (Revival Blessing)
#[test]
fn revival_blessing() {
    let mut o = Script::default();
    let (mut st, _) = battle(&[("pawmot", "noability", "", &["revivalblessing"]), ("snorlax", "noability", "", WAIT)], &[("snorlax", "noability", "", WAIT)], &mut o);
    st.sides[0].mons[1].hp = 0;
    st.sides[0].mons[1].fainted = true;
    st.sides[0].pokemon_left = 2;
    check(step(&mut st, [m(0), m(0)], &mut o), &["move 0:0 revivalblessing"]);
    check(step(&mut st, [sw(1), None], &mut o), &["revive 0:1", "move 1:0 calmmind", "boost 1:0 spa+1,spd+1 move:calmmind", "turn 2"]);
}
