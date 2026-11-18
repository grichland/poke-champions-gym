//! The team pool: a list of six-Pokémon teams in Showdown's set format (python/champions_gym/data/teams.json,
//! a copy of tools/rig/data/teams.json), read with a small JSON reader of our own.

use cg_engine::sim::data::{self as d, abilities, to_id, AbilityId, ItemId, MoveId, SpeciesId, STAGE3_MOVES};
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::MonSpec;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(v) => v.iter().find(|(x, _)| x == k).map(|(_, j)| j),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn parse(text: &str) -> Result<Json, String> {
        let mut p = Parser { b: text.as_bytes(), i: 0 };
        let v = p.value()?;
        p.ws();
        if p.i != p.b.len() {
            return Err(format!("json: trailing data at {}", p.i));
        }
        Ok(v)
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("json: {} at {}", what, self.i))
    }
    fn lit(&mut self, s: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            self.err("bad literal")
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'n') => self.lit("null", Json::Null),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(v));
                        }
                        _ => return self.err("expected , or ]"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(v));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return self.err("expected key");
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return self.err("expected :");
                    }
                    self.i += 1;
                    v.push((k, self.value()?));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(v));
                        }
                        _ => return self.err("expected , or }"),
                    }
                }
            }
            Some(_) => {
                let s = self.i;
                while self.i < self.b.len() && matches!(self.b[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[s..self.i])
                    .ok()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(Json::Num)
                    .ok_or_else(|| format!("json: bad number at {}", s))
            }
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else { return self.err("unterminated string") };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else { return self.err("bad escape") };
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let h = self.b.get(self.i..self.i + 4).and_then(|x| std::str::from_utf8(x).ok());
                            let cp = h.and_then(|x| u32::from_str_radix(x, 16).ok());
                            let Some(cp) = cp else { return self.err("bad \\u escape") };
                            self.i += 4;
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        x => out.push(x),
                    }
                }
                x => out.push(x),
            }
        }
        String::from_utf8(out).map_err(|_| "json: bad utf-8".into())
    }
}

/// An optional id field: None when absent or null, an error when not a string.
fn id_field(set: &Json, key: &str) -> Result<Option<String>, String> {
    match set.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => Ok(Some(to_id(s))),
        Some(_) => Err(format!("{} is not a string", key)),
    }
}

const STAT_KEYS: [&str; 6] = ["hp", "atk", "def", "spa", "spd", "spe"];

/// A Showdown set to a MonSpec. Unknown ids, wrong types and impossible values are errors; an absent item,
/// nature or stat point means none, neutral, 0; keys CG does not read (level, ivs, name, ...) are ignored.
pub fn mon_spec(set: &Json) -> Result<MonSpec, String> {
    if !matches!(set, Json::Obj(_)) {
        return Err("set is not an object".into());
    }
    let sid = id_field(set, "species")?.unwrap_or_default();
    if sid.is_empty() {
        return Err("species missing".into());
    }
    let species = SpeciesId::from_id(&sid).ok_or_else(|| format!("unknown species {:?}", sid))?;
    let item = match id_field(set, "item")? {
        None => ItemId::NONE,
        Some(i) if i.is_empty() => ItemId::NONE,
        Some(i) => ItemId::from_id(&i).ok_or_else(|| format!("unknown item {:?}", i))?,
    };
    let aid = id_field(set, "ability")?.unwrap_or_default();
    if aid.is_empty() {
        return Err("ability missing".into());
    }
    let ability = AbilityId::from_id(&aid).ok_or_else(|| format!("unknown ability {:?}", aid))?;
    let list = match set.get("moves") {
        None | Some(Json::Null) => return Err("moves missing".into()),
        Some(Json::Arr(v)) => v.as_slice(),
        Some(_) => return Err("moves is not a list".into()),
    };
    let mut moves = [MoveId::NONE; 4];
    let mut n = 0;
    for m in list {
        // an empty or null entry is an empty slot (Showdown writes them)
        let id = match m {
            Json::Null => continue,
            Json::Str(s) => to_id(s),
            _ => return Err("a move is not a string".into()),
        };
        if id.is_empty() {
            continue;
        }
        if n >= 4 {
            return Err("more than four moves".into());
        }
        moves[n] = MoveId::from_id(&id).ok_or_else(|| format!("unknown move {:?}", id))?;
        n += 1;
    }
    if n == 0 {
        return Err("no moves".into());
    }
    let nid = id_field(set, "nature")?.unwrap_or_default();
    let nature = d::nature_from_id(if nid.is_empty() { "serious" } else { &nid }).ok_or_else(|| format!("unknown nature {:?}", nid))?;
    let mut evs = [0u8; 6];
    match set.get("evs") {
        None | Some(Json::Null) => {}
        Some(e @ Json::Obj(_)) => {
            for (i, k) in STAT_KEYS.iter().enumerate() {
                let v = match e.get(k) {
                    None | Some(Json::Null) => 0.0,
                    Some(Json::Num(x)) => *x,
                    Some(_) => return Err(format!("evs.{} is not a number", k)),
                };
                if !(0.0..=255.0).contains(&v) || v.fract() != 0.0 {
                    return Err(format!("evs.{} is {} (need a whole number 0..255)", k, v));
                }
                evs[i] = v as u8;
            }
        }
        Some(_) => return Err("evs is not an object".into()),
    }
    let g = match set.get("gender") {
        None | Some(Json::Null) => "",
        Some(Json::Str(s)) => s.as_str(),
        Some(_) => return Err("gender is not a string".into()),
    };
    let gender = match (g, species.data().gender) {
        // a species of one gender (or none) has it whatever the set says
        (_, "M") => 1,
        (_, "F") => 2,
        (_, "N") => 0,
        ("M", _) => 1,
        ("F", _) => 2,
        ("N", _) => return Err("gender N, but the species is male or female".into()),
        ("", _) => return Err("gender missing (the species can be M or F)".into()),
        (x, _) => return Err(format!("gender {:?} (need M or F)", x)),
    };
    Ok(MonSpec { species, ability, item, moves, nmoves: n as u8, nature, evs, gender })
}

pub type Team = [MonSpec; 6];

/// One team's sets, or why it cannot be used ("set 2 (Gyarados): unknown move ...").
pub fn read_team(t: &Json) -> Result<Team, String> {
    let sets = t.as_arr().ok_or("not a list of sets")?;
    if sets.len() != 6 {
        return Err(format!("{} sets, need 6", sets.len()));
    }
    let mut out = [MonSpec::default(); 6];
    for (i, s) in sets.iter().enumerate() {
        let name = s.get("species").and_then(Json::as_str).unwrap_or("?");
        out[i] = mon_spec(s).map_err(|e| format!("set {} ({}): {}", i, name, e))?;
    }
    Ok(out)
}

/// How a battle's two teams are made from the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamMode {
    /// the sets as written
    AsGiven,
    /// the resolver's current scope: No Ability, no item, four distinct moves drawn from the stage-3 pool
    Stage3,
}

/// One seat's weighted draw over pool indices: the weights and their running sum.
#[derive(Clone, Debug)]
pub struct DrawTable {
    w: Vec<f64>,
    cum: Vec<f64>,
}

impl DrawTable {
    pub fn new(w: Vec<f64>) -> DrawTable {
        let mut cum = Vec::with_capacity(w.len());
        let mut t = 0.0;
        for x in &w {
            t += x;
            cum.push(t);
        }
        DrawTable { w, cum }
    }

    fn support(&self) -> Vec<usize> {
        (0..self.w.len()).filter(|&i| self.w[i] > 0.0).collect()
    }

    /// Index i with probability w[i] / sum, drawn over the weights without `skip` (never returned). One
    /// 53-bit uniform and a binary search. Needs a positive weight outside `skip`.
    pub fn sample(&self, rng: &mut Seeded, skip: Option<usize>) -> usize {
        let total = *self.cum.last().unwrap_or(&0.0);
        let ws = skip.map_or(0.0, |k| self.w[k]);
        let u = (rng.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        let mut x = u * (total - ws);
        if let Some(k) = skip {
            let before = if k == 0 { 0.0 } else { self.cum[k - 1] };
            if x >= before {
                x += ws;
            }
        }
        let i = self.cum.partition_point(|&c| c <= x);
        if i < self.w.len() && self.w[i] > 0.0 && Some(i) != skip {
            return i;
        }
        // x rounded onto the end of the table: the last eligible index
        (0..self.w.len()).rev().find(|&j| self.w[j] > 0.0 && Some(j) != skip).expect("a positive weight")
    }
}

#[derive(Clone, Debug, Default)]
pub struct TeamPool {
    /// the accepted teams; a pool index is an index into this
    pub teams: Vec<Team>,
    /// the index in the file of each accepted team
    pub file_index: Vec<usize>,
    /// the teams in the file that were not accepted: (index in the file, reason)
    pub errors: Vec<(usize, String)>,
    /// teams in the file, accepted or not
    pub file_count: usize,
    /// weighted draws: the first seat's table and the other's; None draws uniformly
    weights: Option<[DrawTable; 2]>,
}

/// The first `k` reasons, for a message.
pub fn first_errors(errors: &[(usize, String)], k: usize) -> String {
    let v: Vec<String> = errors.iter().take(k).map(|(i, e)| format!("team {}: {}", i, e)).collect();
    v.join("; ")
}

impl TeamPool {
    /// Reads a teams.json, keeping every team that is six valid sets and the reason for each one that is
    /// not. No minimum count (`from_json` needs two).
    pub fn read(text: &str) -> Result<TeamPool, String> {
        let j = Json::parse(text)?;
        let list = j.as_arr().ok_or("teams: not a list")?;
        let mut p = TeamPool { file_count: list.len(), ..TeamPool::default() };
        for (i, t) in list.iter().enumerate() {
            match read_team(t) {
                Ok(team) => {
                    p.teams.push(team);
                    p.file_index.push(i);
                }
                Err(e) => p.errors.push((i, e)),
            }
        }
        Ok(p)
    }

    /// Reads a teams.json; teams that are not six valid sets are dropped and listed in `errors`. Needs at
    /// least two valid teams.
    pub fn from_json(text: &str) -> Result<TeamPool, String> {
        TeamPool::read(text)?.at_least_two()
    }

    /// `from_json`, refusing the file when any team is not accepted.
    pub fn from_json_strict(text: &str) -> Result<TeamPool, String> {
        let p = TeamPool::read(text)?;
        if !p.errors.is_empty() {
            return Err(p.dropped_message());
        }
        p.at_least_two()
    }

    /// Errs when fewer than two teams were accepted.
    pub fn at_least_two(self) -> Result<TeamPool, String> {
        if self.teams.len() < 2 {
            let why = if self.errors.is_empty() { String::new() } else { format!(" ({})", first_errors(&self.errors, 3)) };
            return Err(format!("teams: need at least two valid teams, {} of {} accepted{}", self.teams.len(), self.file_count, why));
        }
        Ok(self)
    }

    pub fn from_file(path: &str) -> Result<TeamPool, String> {
        TeamPool::from_json(&read_file(path)?)
    }

    /// `from_file` for command-line tools: one line on stderr when teams were dropped.
    pub fn open(path: &str) -> Result<TeamPool, String> {
        let p = TeamPool::from_file(path)?;
        if !p.errors.is_empty() {
            eprintln!("warning: {}: {}", path, p.dropped_message());
        }
        Ok(p)
    }

    /// "k of n teams not accepted: team i: reason; ..." with the first three reasons.
    pub fn dropped_message(&self) -> String {
        format!("{} of {} teams not accepted: {}", self.errors.len(), self.file_count, first_errors(&self.errors, 3))
    }

    /// Weighted draws. `first` holds the weights of the seat that draws first (the agent's), `other` those
    /// of the other seat; each has one weight per team in the file (weights of teams not accepted are
    /// ignored). The two teams of a game stay distinct: the other seat draws from its weights without the
    /// first seat's team. `same` says both are one sequence (for the messages).
    pub fn set_weights(&mut self, first: &[f64], other: &[f64], same: bool) -> Result<(), String> {
        let mut tables = Vec::with_capacity(2);
        for (k, w) in [first, other].iter().enumerate() {
            let who = if same { "team_weights".to_string() } else { format!("team_weights[{}]", k) };
            if w.len() != self.file_count {
                return Err(format!("{}: {} weights for {} teams in the file", who, w.len(), self.file_count));
            }
            if let Some(i) = w.iter().position(|x| !x.is_finite()) {
                return Err(format!("{}: the weight of team {} is {} (need a finite number)", who, i, w[i]));
            }
            if let Some(i) = w.iter().position(|x| *x < 0.0) {
                return Err(format!("{}: the weight of team {} is negative ({})", who, i, w[i]));
            }
            let t = DrawTable::new(self.file_index.iter().map(|&f| w[f]).collect());
            if t.support().is_empty() {
                return Err(format!("{}: no accepted team has a positive weight", who));
            }
            tables.push(t);
        }
        let (a, b) = (tables[0].support(), tables[1].support());
        if b.len() == 1 && a.contains(&b[0]) {
            let f = self.file_index[b[0]];
            return Err(if same {
                format!("team_weights: only team {} has a positive weight; a game needs two different teams", f)
            } else {
                format!("team_weights: the second seat can draw only team {}, which the first seat can draw too; a game needs two different teams", f)
            });
        }
        let t1 = tables.pop().unwrap();
        let t0 = tables.pop().unwrap();
        self.weights = Some([t0, t1]);
        Ok(())
    }

    pub fn weighted(&self) -> bool {
        self.weights.is_some()
    }

    /// Two distinct teams for one battle, drawn with `rng`, made per `mode`. Returns the pool indices too.
    pub fn draw(&self, rng: &mut Seeded, mode: TeamMode) -> ([usize; 2], [Team; 2]) {
        self.draw_for(rng, mode, 0)
    }

    /// `draw` where seat `first` draws first, from the first weights. Without weights the draw is uniform
    /// (seat 0 first) and `first` is not used.
    pub fn draw_for(&self, rng: &mut Seeded, mode: TeamMode, first: usize) -> ([usize; 2], [Team; 2]) {
        let (a, b) = match &self.weights {
            None => {
                let n = self.teams.len() as u32;
                let a = rng.below(n) as usize;
                let mut b = rng.below(n - 1) as usize;
                if b >= a {
                    b += 1;
                }
                (a, b)
            }
            Some([w0, w1]) => {
                let x = w0.sample(rng, None);
                let y = w1.sample(rng, Some(x));
                if first == 0 {
                    (x, y)
                } else {
                    (y, x)
                }
            }
        };
        let mut out = [self.teams[a], self.teams[b]];
        if mode == TeamMode::Stage3 {
            for team in out.iter_mut() {
                for m in team.iter_mut() {
                    m.ability = abilities::NOABILITY;
                    m.item = ItemId::NONE;
                    let mv = distinct(rng, STAGE3_MOVES.len(), 4);
                    m.moves = [STAGE3_MOVES[mv[0]], STAGE3_MOVES[mv[1]], STAGE3_MOVES[mv[2]], STAGE3_MOVES[mv[3]]];
                    m.nmoves = 4;
                }
            }
        }
        ([a, b], out)
    }
}

/// A teams file's text.
pub fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))
}

/// k distinct indices of 0..n (partial Fisher-Yates)
pub fn distinct(rng: &mut Seeded, n: usize, k: usize) -> [usize; 4] {
    debug_assert!(k <= 4 && k <= n);
    let mut out = [0usize; 4];
    for i in 0..k {
        // rejection over the few already drawn: fine for k <= 4
        loop {
            let x = rng.below(n as u32) as usize;
            if !out[..i].contains(&x) {
                out[i] = x;
                break;
            }
        }
    }
    out
}

/// SplitMix64 finaliser: derives independent seeds from a master seed.
pub fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x632B_E59B_D9B4_E019);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
