//! One-line text of an engine row, for traces, dumps and tests.

use cg_engine::sim::data::BOOST_IDS;
use cg_engine::sim::log::{Detail, Fx, How, ItemWhat, Reset, Row, SideWhat, Who};
use cg_engine::sim::state::Winner;

fn p(w: Who) -> String {
    format!("{}:{}", w.0, w.1)
}

fn op(w: Option<Who>) -> String {
    w.map(p).unwrap_or_else(|| "-".into())
}

pub fn fx(f: Fx) -> String {
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

fn stages(b: &[i8; 7]) -> String {
    let v: Vec<String> = (0..7).filter(|&i| b[i] != 0).map(|i| format!("{}{:+}", BOOST_IDS[i], b[i])).collect();
    v.join(",")
}

fn detail(d: Detail) -> String {
    match d {
        Detail::None => "-".into(),
        Detail::Move(x) => x.id().into(),
        Detail::Num(n) => n.to_string(),
        Detail::Type(t) => t.id().into(),
    }
}

pub fn how(h: How) -> &'static str {
    match h {
        How::Super => "super",
        How::Extreme => "extreme",
        How::Resisted => "resisted",
        How::Mostly => "mostly_ineffective",
        How::Immune => "immune",
        How::Ohko => "ohko",
    }
}

pub fn reset(r: Reset) -> &'static str {
    match r {
        Reset::Field => "field",
        Reset::All => "all",
        Reset::Negative => "negative",
        Reset::Positive => "positive",
        Reset::Copy => "copy",
        Reset::Swap => "swap",
        Reset::Invert => "invert",
    }
}

pub fn item_what(w: ItemWhat) -> &'static str {
    match w {
        ItemWhat::Activated => "activated",
        ItemWhat::Used => "used",
        ItemWhat::Removed => "removed",
        ItemWhat::Received => "received",
        ItemWhat::Found => "found",
    }
}

pub fn show(r: &Row) -> String {
    match *r {
        Row::Turn(n) => format!("turn {}", n),
        Row::Enter { p: w, forced } => format!("enter {}{}", p(w), if forced { " forced" } else { "" }),
        Row::Withdraw { p: w, pivot } => format!("withdraw {}{}", p(w), pivot.map(|m| format!(" pivot:{}", m.id())).unwrap_or_default()),
        Row::Move { p: w, mv, target, called_by, priority } => {
            format!("move {} {} at {} by {} pri {}", p(w), mv.id(), op(target), fx(called_by), priority)
        }
        Row::Prepare { p: w, mv } => format!("prepare {} {}", p(w), mv.id()),
        Row::Cant { p: w, why, mv } => format!("cant {} {} {}", p(w), fx(why), mv.id()),
        Row::Protect { p: w, by } => format!("protect {} {}", p(w), by.id()),
        Row::Miss { p: w, by } => format!("miss {} by {}", p(w), p(by)),
        Row::Fail { p: w, why } => format!("fail {} {}", op(w), fx(why)),
        Row::Result { how: h, p: w } => format!("result {} {}", op(w), how(h)),
        Row::Hits { n, p: w } => format!("hits {} {}", p(w), n),
        Row::Hp { p: w, before, after, cause, of } => format!("hp {} {}>{} {} of {}", p(w), before, after, fx(cause), op(of)),
        Row::Faint { p: w } => format!("faint {}", p(w)),
        Row::Status { p: w, st, by, of, rest } => format!("status {} {} {} of {}{}", p(w), st.id(), fx(by), op(of), if rest { " rest" } else { "" }),
        Row::Cure { p: w, st, by } => format!("cure {} {} {}", p(w), st.id(), fx(by)),
        Row::Volatile { p: w, c, detail: d, by, of } => format!("volatile {} {} {} {} of {}", p(w), c.id(), detail(d), fx(by), op(of)),
        Row::VolatileEnd { p: w, c, by } => format!("volatile_end {} {} {}", p(w), c.id(), fx(by)),
        Row::VolatileActed { p: w, c, detail: d, of } => format!("volatile_acted {} {} {} of {}", p(w), c.id(), detail(d), op(of)),
        Row::Boost { p: w, by_stat, by } => format!("boost {} {} {}", p(w), stages(&by_stat), fx(by)),
        Row::BoostLimit { p: w, stats, up, by } => {
            let n: Vec<&str> = (0..7).filter(|&i| stats[i]).map(|i| BOOST_IDS[i]).collect();
            format!("boost_limit {} {} {} {}", p(w), n.join(","), if up { "up" } else { "down" }, fx(by))
        }
        Row::BoostReset { p: w, what, of, by } => format!("boost_reset {} {} of {} {}", op(w), reset(what), op(of), fx(by)),
        Row::Blocked { p: w, by, what } => format!("blocked {} {} {}", op(w), fx(by), fx(what)),
        Row::Ability { p: w, ab, of } => format!("ability {} {} of {}", p(w), ab.id(), op(of)),
        Row::AbilityChange { p: w, from, to, by } => format!("ability_change {} {}>{} {}", p(w), from.id(), to.id(), fx(by)),
        Row::Item { p: w, item, what, of, by } => format!("item {} {} {} of {} {}", p(w), item.id(), item_what(what), op(of), fx(by)),
        Row::MegaStone { p: w, stone } => format!("mega_stone {} {}", p(w), stone.id()),
        Row::Mega { p: w, form } => format!("mega {} {}", p(w), form.id()),
        Row::Form { p: w, form, by } => format!("form {} {} {}", p(w), form.id(), fx(by)),
        Row::Transform { p: w, into } => format!("transform {} into {}", p(w), p(into)),
        Row::Types { p: w, types, lost, added, by } => format!("types {} {}/{} lost {} added {} {}", p(w), types[0].id(), types[1].id(), lost.id(), added.id(), fx(by)),
        Row::Weather { w, by, of } => format!("weather {} {} of {}", if w.is_none() { "none" } else { w.id() }, fx(by), op(of)),
        Row::Field { c, start, by, of } => format!("field {} {} {} of {}", c.id(), if start { "start" } else { "end" }, fx(by), op(of)),
        Row::Side { side, c, what, p: w, by } => {
            let s = match what {
                SideWhat::Start => "start",
                SideWhat::End => "end",
                SideWhat::Acted => "acted",
            };
            format!("side {} {} {} {} {}", side, c.id(), s, op(w), fx(by))
        }
        Row::Once { p: w, fx: f, of } => format!("once {} {} of {}", op(w), fx(f), op(of)),
        Row::Pp { p: w, mv, lost, by } => format!("pp {} {} {} {}", p(w), mv.id(), lost, fx(by)),
        Row::Revive { p: w } => format!("revive {}", p(w)),
        Row::End { winner } => format!(
            "end {}",
            match winner {
                Winner::Side(s) => s.to_string(),
                Winner::Tie => "tie".into(),
                Winner::None => "-".into(),
            }
        ),
    }
}
