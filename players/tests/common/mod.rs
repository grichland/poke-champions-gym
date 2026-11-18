//! Hand-made Battles through the builder: a team with real stats, a preview, picks, and events as JSON.
#![allow(dead_code)]

use battle::*;
use players::*;
use cg_engine::sim::calc::stat_from_points;
use cg_engine::sim::data::*;

/// A set with neutral nature and the given stat points; stats computed by Champions' formula.
pub fn set_pts(species: &str, item: Option<&str>, ability: &str, moves: &[&str], points: [u8; 6]) -> SetMsg {
    let sp = SpeciesId::from_id(species).unwrap_or_else(|| panic!("species {}", species));
    let neutral = nature_from_id("serious").unwrap();
    let st = [Stat::Hp, Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe];
    let mut stats = [0i32; 6];
    for i in 0..6 {
        stats[i] = stat_from_points(sp.data().base_stats[i], points[i], st[i], neutral) as i32;
    }
    for m in moves {
        assert!(MoveId::from_id(m).is_some(), "move {}", m);
    }
    SetMsg {
        species: species.into(),
        item: item.map(Into::into),
        ability: ability.into(),
        moves: moves.iter().map(|m| m.to_string()).collect(),
        nature: Some("serious".into()),
        points: Some(points.map(|x| x as i32)),
        stats,
    }
}

pub fn set(species: &str, item: Option<&str>, ability: &str, moves: &[&str]) -> SetMsg {
    set_pts(species, item, ability, moves, [0; 6])
}

pub fn msg(body: Body) -> Message {
    Message { v: CONTRACT_VERSION.into(), battle: "players-test".into(), body }
}

pub struct T {
    pub b: Builder,
}

impl T {
    /// battle_start with `team` and `preview`, no picks yet.
    pub fn start(team: Vec<SetMsg>, preview: &[&str]) -> T {
        let mut t = T { b: Builder::new() };
        let pv = preview.iter().map(|s| if s.is_empty() { None } else { Some(s.to_string()) }).collect();
        t.b.apply(&Input::Msg(msg(Body::BattleStart { own_team: team, opp_preview: pv })));
        t
    }
    /// battle_start, picks [0, 1, 2], own team[0] and the opponent's preview[0] entering, turn 1.
    pub fn new(team: Vec<SetMsg>, preview: &[&str]) -> T {
        let own_lead = team[0].species.clone();
        let opp_lead = preview[0].to_string();
        let mut t = T::start(team, preview);
        t.b.apply(&Input::Picks(Picks { picks: vec![0, 1, 2] }));
        t.e(&format!(r#"{{"k":"enter","side":"own","mon":"{}"}}"#, own_lead));
        t.e(&format!(r#"{{"k":"enter","side":"opp","mon":"{}"}}"#, opp_lead));
        t.turn(1);
        t
    }
    pub fn e(&mut self, json: &str) {
        let ev = Event::from_json(&json::Json::parse(json).expect("json")).expect("event");
        self.b.apply(&Input::Msg(msg(Body::Events { events: vec![ev], fallback: None })));
    }
    pub fn es(&mut self, list: &[String]) {
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
    pub fn md(&self) -> usize {
        MaxDamagePlayer::default().choose_move(self.bt(), ReqKind::Turn, &mut Rng::new(1))
    }
    pub fn md_kind(&self, k: ReqKind) -> usize {
        MaxDamagePlayer::default().choose_move(self.bt(), k, &mut Rng::new(1))
    }
    pub fn tower(&self, seed: u64) -> usize {
        TowerPlayer::default().choose_move(self.bt(), ReqKind::Turn, &mut Rng::new(seed))
    }
    pub fn tower_kind(&self, k: ReqKind, seed: u64) -> usize {
        TowerPlayer::default().choose_move(self.bt(), k, &mut Rng::new(seed))
    }
    /// Tower's choices over many seeds, as counts per action.
    pub fn tower_counts(&self, n: u64) -> [u32; N_ACTIONS] {
        let mut c = [0; N_ACTIONS];
        let mut p = TowerPlayer::default();
        for s in 0..n {
            c[p.choose_move(self.bt(), ReqKind::Turn, &mut Rng::new(s))] += 1;
        }
        c
    }
    /// A named Player's action at this turn request.
    pub fn play(&self, name: &str, seed: u64) -> usize {
        self.play_kind(name, ReqKind::Turn, seed)
    }
    pub fn play_kind(&self, name: &str, k: ReqKind, seed: u64) -> usize {
        by_name(name).unwrap().choose_move(self.bt(), k, &mut Rng::new(seed))
    }
    /// A named Player's choices over many seeds, as counts per action, and its rule firings.
    pub fn counts(&self, name: &str, n: u64) -> ([u32; N_ACTIONS], Vec<(&'static str, u64)>) {
        let mut c = [0; N_ACTIONS];
        let mut p = by_name(name).unwrap();
        for s in 0..n {
            c[p.choose_move(self.bt(), ReqKind::Turn, &mut Rng::new(s))] += 1;
        }
        (c, p.fired())
    }
    /// The rule firings of one decision.
    pub fn fired(&self, name: &str, k: ReqKind) -> (usize, Vec<&'static str>) {
        let mut p = by_name(name).unwrap();
        let a = p.choose_move(self.bt(), k, &mut Rng::new(1));
        (a, p.fired().into_iter().filter(|(n, c)| *c > 0 && *n != "decisions").map(|x| x.0).collect())
    }
    /// Our or their active is withdrawn and `to` comes in (a switch by choice).
    pub fn switch(&mut self, side: &str, from: &str, to: &str) {
        self.e(&format!(r#"{{"k":"withdraw","side":"{}","mon":"{}"}}"#, side, from));
        self.e(&format!(r#"{{"k":"enter","side":"{}","mon":"{}"}}"#, side, to));
    }
    pub fn sorted(&self) -> tower::Sorted {
        tower::inspect(self.bt(), ReqKind::Turn, &Options::default()).expect("both actives")
    }
    pub fn own(&self) -> Who {
        self.bt().active(Side::Own).unwrap()
    }
    pub fn opp(&self) -> Who {
        self.bt().active(Side::Opp).unwrap()
    }
    /// Expected fraction of own active's move slot `slot` on the foe.
    pub fn exp(&self, slot: usize) -> f32 {
        let b = self.bt();
        let mv = b.mon(self.own()).moves[slot].unwrap();
        Estimator::new(b).hit(b, self.own(), false, mv, self.opp()).expected()
    }
}

pub fn mv(side: &str, mon: &str, m: &str) -> String {
    format!(r#"{{"k":"move","side":"{}","mon":"{}","move":"{}"}}"#, side, mon, m)
}
pub fn hp_own(mon: &str, before: i32, after: i32) -> String {
    let dir = if after < before { "lost" } else { "restored" };
    format!(r#"{{"k":"hp","side":"own","mon":"{}","dir":"{}","before":{},"after":{}}}"#, mon, dir, before, after)
}
pub fn hp_opp(mon: &str, before: f64, after: f64) -> String {
    let dir = if after < before { "lost" } else { "restored" };
    format!(r#"{{"k":"hp","side":"opp","mon":"{}","dir":"{}","before_pct":{},"after_pct":{}}}"#, mon, dir, before, after)
}
pub fn vol(side: &str, mon: &str, effect: &str) -> String {
    format!(r#"{{"k":"volatile","side":"{}","mon":"{}","effect":"{}"}}"#, side, mon, effect)
}
pub fn boost(side: &str, mon: &str, stat: &str, by: i32) -> String {
    format!(r#"{{"k":"boost","side":"{}","mon":"{}","stats":["{}"],"by":{}}}"#, side, mon, stat, by)
}
pub fn status(side: &str, mon: &str, s: &str) -> String {
    format!(r#"{{"k":"status","side":"{}","mon":"{}","status":"{}"}}"#, side, mon, s)
}
pub fn ability(side: &str, mon: &str, a: &str) -> String {
    format!(r#"{{"k":"ability","side":"{}","mon":"{}","ability":"{}"}}"#, side, mon, a)
}
pub fn item(side: &str, mon: &str, it: &str) -> String {
    format!(r#"{{"k":"item","side":"{}","mon":"{}","item":"{}","what":"activated"}}"#, side, mon, it)
}
pub fn weather(w: &str) -> String {
    format!(r#"{{"k":"weather","weather":"{}","what":"start"}}"#, w)
}
pub fn field(f: &str) -> String {
    format!(r#"{{"k":"field","effect":"{}","what":"start"}}"#, f)
}
pub fn side_cond(side: &str, e: &str) -> String {
    format!(r#"{{"k":"side","side":"{}","effect":"{}","what":"start"}}"#, side, e)
}
pub fn s(x: &str) -> String {
    x.to_string()
}

/// Two filler team members so picks [0, 1, 2] are valid.
pub fn fill(mut team: Vec<SetMsg>) -> Vec<SetMsg> {
    let fillers = [
        set("snorlax", None, "thickfat", &["bodyslam", "crunch", "protect", "rest"]),
        set("lucario", None, "innerfocus", &["closecombat", "crunch", "protect", "swordsdance"]),
        set("sylveon", None, "pixilate", &["hypervoice", "moonblast", "protect", "calmmind"]),
    ];
    let mut i = 0;
    while team.len() < 3 {
        team.push(fillers[i].clone());
        i += 1;
    }
    team
}

pub const PREVIEW: [&str; 6] = ["garchomp", "gengar", "corviknight", "rotomwash", "dragonite", "ditto"];
