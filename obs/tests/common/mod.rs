//! Helpers for building Battles through the builder from event lists.
#![allow(dead_code)]

use battle::*;
use obs::{observe, ObsBuffers};
use cg_engine::sim::calc::stat_from_points;
use cg_engine::sim::data::*;

pub fn stats_of(species: &str, nature: &str, points: [u8; 6]) -> [i32; 6] {
    let d = SpeciesId::from_id(species).unwrap().data();
    let n = nature_from_id(nature).unwrap();
    let st = [Stat::Hp, Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe];
    let mut out = [0; 6];
    for i in 0..6 {
        out[i] = stat_from_points(d.base_stats[i], points[i], st[i], n) as i32;
    }
    out
}

pub fn set(species: &str, item: Option<&str>, ability: &str, moves: [&str; 4]) -> SetMsg {
    let points = [2, 32, 0, 0, 0, 32];
    SetMsg {
        species: species.into(),
        item: item.map(|x| x.into()),
        ability: ability.into(),
        moves: moves.iter().map(|m| m.to_string()).collect(),
        nature: Some("jolly".into()),
        points: Some(points.map(|x| x as i32)),
        stats: stats_of(species, "jolly", points),
    }
}

pub fn own(sp: &str) -> Mon {
    Mon::new(Side::Own, sp)
}
pub fn opp(sp: &str) -> Mon {
    Mon::new(Side::Opp, sp)
}

pub fn mv(who: Mon, m: &str) -> Event {
    Event::Move { who, mv: m.into(), priority: None, called_by: None }
}
pub fn enter(who: Mon) -> Event {
    Event::Enter { who, forced: false }
}
pub fn withdraw(who: Mon) -> Event {
    Event::Withdraw { who, pivot: false }
}
pub fn turn(n: u32) -> Event {
    Event::Turn { n }
}
pub fn hp_own(who: &str, before: i32, after: i32) -> Event {
    let dir = if after < before { HpDir::Lost } else { HpDir::Restored };
    Event::Hp { who: own(who), dir, hp: Hp::Own { before, after } }
}
pub fn hp_opp(who: &str, before: f64, after: f64) -> Event {
    let dir = if after < before { HpDir::Lost } else { HpDir::Restored };
    Event::Hp { who: opp(who), dir, hp: Hp::Opp { before_pct: before, after_pct: after } }
}
pub fn vol(who: Mon, effect: &str, detail: Option<Detail>) -> Event {
    Event::Volatile { who, effect: effect.into(), detail }
}
pub fn boost(who: Mon, stat: &str, by: i32) -> Event {
    Event::Boost { who, stats: vec![stat.into()], by }
}
pub fn side_start(side: Side, effect: &str) -> Event {
    Event::SideCond { side, effect: effect.into(), what: SideWhat::Start, mon: None }
}

fn msg(body: Body) -> Message {
    Message { v: CONTRACT_VERSION.into(), battle: "t".into(), body }
}

/// A Battle under construction, from one seat.
pub struct G {
    pub b: Builder,
}

impl G {
    pub fn new(team: Vec<SetMsg>, preview: &[&str]) -> G {
        let mut b = Builder::new();
        let preview = preview.iter().map(|p| if p.is_empty() { None } else { Some(p.to_string()) }).collect();
        b.apply(&Input::Msg(msg(Body::BattleStart { own_team: team, opp_preview: preview })));
        G { b }
    }
    pub fn picks(&mut self, p: [u8; 3]) -> &mut G {
        self.b.apply(&Input::Picks(Picks { picks: p.to_vec() }));
        self
    }
    pub fn ev(&mut self, e: Event) -> &mut G {
        self.b.apply(&Input::Msg(msg(Body::Events { events: vec![e], fallback: None })));
        self
    }
    pub fn evs(&mut self, es: Vec<Event>) -> &mut G {
        for e in es {
            self.ev(e);
        }
        self
    }
    pub fn obs(&self, kind: ReqKind) -> Box<ObsBuffers> {
        let mut o = Box::new(ObsBuffers::default());
        observe(&self.b.battle, kind, &mut o);
        o
    }
    pub fn battle(&self) -> &Battle {
        &self.b.battle
    }
}

/// Our usual six (gold team species) and a fixed opposing preview.
pub fn gold() -> Vec<SetMsg> {
    vec![
        set("gholdengo", Some("choicescarf"), "goodasgold", ["makeitrain", "shadowball", "nastyplot", "recover"]),
        set("garchomp", Some("garchompitez"), "roughskin", ["earthquake", "dragonclaw", "swordsdance", "protect"]),
        set("golisopod", None, "emergencyexit", ["uturn", "liquidation", "spikes", "protect"]),
        set("primarina", Some("leftovers"), "torrent", ["moonblast", "hydropump", "wish", "protect"]),
        set("mamoswine", Some("focussash"), "oblivious", ["earthquake", "iceshard", "stealthrock", "iciclecrash"]),
        set("cinderace", Some("lifeorb"), "blaze", ["pyroball", "uturn", "highjumpkick", "suckerpunch"]),
    ]
}

pub const PREVIEW: [&str; 6] = ["rotomwash", "snorlax", "glimmora", "dragonite", "tyranitar", "mamoswine"];

/// Rows of a buffer by feature: value of a row feature at an offset.
pub fn row(o: &ObsBuffers, r: usize, off: usize) -> f32 {
    o.rows[r][off]
}
pub fn slot(o: &ObsBuffers, r: usize, s: usize, off: usize) -> f32 {
    o.rows[r][obs::layout::MOVE_BASE + obs::layout::MOVE_STRIDE * s + off]
}

pub fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}
