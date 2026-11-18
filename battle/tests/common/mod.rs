//! Shared helpers: a fixed own team, a preview, and a builder fed one JSON event at a time.
#![allow(dead_code)]

use battle::*;

pub fn set(species: &str, item: Option<&str>, ability: &str, moves: [&str; 4], hp: i32) -> SetMsg {
    SetMsg {
        species: species.into(),
        item: item.map(Into::into),
        ability: ability.into(),
        moves: moves.iter().map(|m| m.to_string()).collect(),
        nature: None,
        points: None,
        stats: [hp, 100, 100, 100, 100, 100],
    }
}

/// Own team (team-select order) used by most tests.
pub fn team() -> Vec<SetMsg> {
    vec![
        set("gholdengo", Some("choicescarf"), "goodasgold", ["makeitrain", "shadowball", "nastyplot", "recover"], 162),
        set("garchomp", Some("garchompitez"), "roughskin", ["earthquake", "outrage", "swordsdance", "fakeout"], 183),
        set("golisopod", Some("golisopite"), "emergencyexit", ["firstimpression", "liquidation", "knockoff", "uturn"], 150),
        set("primarina", Some("leftovers"), "torrent", ["moonblast", "hypervoice", "hyperbeam", "solarbeam"], 155),
        set("mamoswine", Some("shedshell"), "thickfat", ["earthquake", "iceshard", "gigatonhammer", "rest"], 185),
        set("cinderace", Some("leppaberry"), "blaze", ["pyroball", "uturn", "sleeptalk", "protect"], 155),
    ]
}

pub fn preview() -> Vec<Option<String>> {
    ["garchomp", "gengar", "corviknight", "rotomwash", "dragonite", "ditto"].iter().map(|s| Some(s.to_string())).collect()
}

pub fn msg(body: Body) -> Message {
    Message { v: CONTRACT_VERSION.into(), battle: "b-test".into(), body }
}

pub struct T {
    pub b: Builder,
    pub rec: Vec<Input>,
}

impl T {
    /// battle_start with `team()`, picks, then each lead entering.
    pub fn with_picks(picks: [u8; 3], own_lead: &str, opp_lead: &str) -> T {
        T::from_team(team(), picks, own_lead, opp_lead)
    }
    pub fn from_team(own_team: Vec<SetMsg>, picks: [u8; 3], own_lead: &str, opp_lead: &str) -> T {
        let mut t = T { b: Builder::new(), rec: vec![] };
        t.input(Input::Msg(msg(Body::BattleStart { own_team, opp_preview: preview() })));
        t.input(Input::Picks(Picks { picks: picks.to_vec() }));
        t.e(&format!(r#"{{"k":"enter","side":"own","mon":"{}"}}"#, own_lead));
        t.e(&format!(r#"{{"k":"enter","side":"opp","mon":"{}"}}"#, opp_lead));
        t
    }
    pub fn new(own_lead: &str, opp_lead: &str) -> T {
        let picks = match own_lead {
            "gholdengo" => [0, 1, 2],
            "garchomp" => [1, 0, 2],
            "golisopod" => [2, 0, 1],
            "primarina" => [3, 0, 1],
            "mamoswine" => [4, 0, 1],
            _ => [5, 0, 1],
        };
        T::with_picks(picks, own_lead, opp_lead)
    }
    pub fn input(&mut self, i: Input) {
        self.b.apply(&i);
        self.rec.push(i);
    }
    /// One event, as JSON text, in its own `events` message.
    pub fn e(&mut self, json: &str) {
        let ev = Event::from_json(&json::Json::parse(json).expect("json")).expect("event");
        self.input(Input::Msg(msg(Body::Events { events: vec![ev], fallback: None })));
    }
    pub fn es(&mut self, list: &[&str]) {
        for j in list {
            self.e(j);
        }
    }
    pub fn turn(&mut self, n: u32) {
        self.e(&format!(r#"{{"k":"turn","n":{}}}"#, n));
    }
    pub fn bt(&self) -> &Battle {
        &self.b.battle
    }
    pub fn legal(&self) -> Legal {
        legal(&self.b.battle, ReqKind::Turn)
    }
    pub fn anomalies(&self) -> Vec<String> {
        self.b.anomalies.iter().map(|a| a.what.clone()).collect()
    }
    pub fn assert_clean(&self) {
        assert!(self.b.anomalies.is_empty(), "anomalies: {:?}", self.anomalies());
    }
    pub fn own(&self, slot: u8) -> &MonState {
        self.bt().mon(Who { side: Side::Own, slot })
    }
    pub fn opp(&self, slot: u8) -> &MonState {
        self.bt().mon(Who { side: Side::Opp, slot })
    }
    pub fn own_active(&self) -> &Active {
        &self.bt().side(Side::Own).active
    }
    pub fn opp_active(&self) -> &Active {
        &self.bt().side(Side::Opp).active
    }
}

pub fn mv(side: &str, mon: &str, m: &str) -> String {
    format!(r#"{{"k":"move","side":"{}","mon":"{}","move":"{}"}}"#, side, mon, m)
}
pub fn hp_own(mon: &str, before: i32, after: i32) -> String {
    let dir = if after < before { "lost" } else { "restored" };
    format!(r#"{{"k":"hp","side":"own","mon":"{}","dir":"{}","before":{},"after":{}}}"#, mon, dir, before, after)
}
pub fn hp_opp(mon: &str, before: i32, after: i32) -> String {
    let dir = if after < before { "lost" } else { "restored" };
    format!(r#"{{"k":"hp","side":"opp","mon":"{}","dir":"{}","before_pct":{},"after_pct":{}}}"#, mon, dir, before, after)
}
pub fn vol(side: &str, mon: &str, effect: &str) -> String {
    format!(r#"{{"k":"volatile","side":"{}","mon":"{}","effect":"{}"}}"#, side, mon, effect)
}
