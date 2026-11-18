//! The messages and events of contracts/LIVE.md (sections 2 to 4), with JSON reading and writing.
//! Ids stay strings here; the builder resolves them against the game data.

use crate::json::Json;

pub const CONTRACT_VERSION: &str = "0.9";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    Own,
    Opp,
}

impl Side {
    pub fn id(self) -> &'static str {
        match self {
            Side::Own => "own",
            Side::Opp => "opp",
        }
    }
    pub fn from_id(s: &str) -> Option<Side> {
        match s {
            "own" => Some(Side::Own),
            "opp" => Some(Side::Opp),
            _ => None,
        }
    }
    pub fn idx(self) -> usize {
        self as usize
    }
    pub fn other(self) -> Side {
        match self {
            Side::Own => Side::Opp,
            Side::Opp => Side::Own,
        }
    }
}

/// A Pokémon as the contract names it: side and species id as brought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mon {
    pub side: Side,
    pub mon: String,
}

impl Mon {
    pub fn new(side: Side, mon: &str) -> Mon {
        Mon { side, mon: mon.to_string() }
    }
}

macro_rules! str_enum {
    ($name:ident { $($v:ident = $s:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name { $($v),+ }
        impl $name {
            pub fn id(self) -> &'static str {
                match self { $($name::$v => $s),+ }
            }
            pub fn from_id(s: &str) -> Option<$name> {
                match s { $($s => Some($name::$v),)+ _ => None }
            }
        }
    };
}

str_enum!(Eff { Super = "super", Extreme = "extreme", Resisted = "resisted", MostlyIneffective = "mostly_ineffective", Immune = "immune", Ohko = "ohko" });
str_enum!(HpDir { Lost = "lost", Restored = "restored" });
str_enum!(StartEnd { Start = "start", End = "end" });
str_enum!(SideWhat { Start = "start", End = "end", Acted = "acted" });
str_enum!(LimitDir { Up = "up", Down = "down" });
str_enum!(ResetWhat { All = "all", Negative = "negative", Copy = "copy", Positive = "positive", Swap = "swap", Invert = "invert" });
str_enum!(ItemWhat { Activated = "activated", Used = "used", Removed = "removed", Received = "received", Found = "found" });
str_enum!(ReqKind { Turn = "turn", Replace = "replace" });
str_enum!(GapWhat { Turn = "turn", Rows = "rows" });

/// HP as the row shows it: whole numbers for our own Pokémon, percent for the opponent's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hp {
    Own { before: i32, after: i32 },
    Opp { before_pct: f64, after_pct: f64 },
}

/// `volatile.detail`: a move id or a number.
#[derive(Clone, Debug, PartialEq)]
pub enum Detail {
    Id(String),
    Num(i64),
}

/// A choice (LIVE.md section 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Order {
    Move { mv: String, mega: bool },
    Switch { mon: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Turn { n: u32 },
    Refused { order: Order },
    End { result: String },
    Enter { who: Mon, forced: bool },
    Withdraw { who: Mon, pivot: bool },
    Move { who: Mon, mv: String, priority: Option<i32>, called_by: Option<String> },
    Prepare { who: Mon, mv: Option<String> },
    /// `who` is absent for the nameless "no PP left" row, which follows the move row it is about
    Cant { who: Option<Mon>, reason: String },
    Protect { who: Mon },
    Miss { who: Mon },
    /// `reason` (extension): what the row says failed (a status, move or condition id, `hpfull`)
    Fail { who: Option<Mon>, reason: Option<String> },
    Result { how: Eff, who: Option<Mon> },
    Hits { n: u32 },
    Hp { who: Mon, dir: HpDir, hp: Hp },
    Cause { effect: String, who: Option<Mon>, of: Option<Mon> },
    Faint { who: Mon },
    /// `effect`: "slept_and_restored" for Rest's row
    Status { who: Mon, status: String, effect: Option<String> },
    Cure { who: Mon, status: String, item: Option<String> },
    Volatile { who: Mon, effect: String, detail: Option<Detail> },
    VolatileEnd { who: Mon, effect: String },
    VolatileHit { effect: String, who: Option<Mon> },
    /// one row can name several stats
    Boost { who: Mon, stats: Vec<String>, by: i32 },
    BoostLimit { who: Mon, stats: Vec<String>, dir: LimitDir },
    /// `who` is absent for Haze, which names no Pokémon. `stats` (extension): the stats a swap exchanged,
    /// when not all of them
    BoostReset { who: Option<Mon>, what: ResetWhat, of: Option<Mon>, stats: Option<Vec<String>> },
    Blocked { effect: String, who: Option<Mon>, what: Option<String> },
    Ability { who: Mon, ability: String },
    AbilityChange { who: Mon, from: String, to: String },
    /// `who` is absent for a Gem, whose row names only a move
    Item { who: Option<Mon>, item: String, what: ItemWhat, of: Option<Mon> },
    MegaStone { who: Mon, stone: String },
    /// `form` is absent when the species has more than one Mega
    Mega { who: Mon, form: Option<String> },
    Form { who: Mon, form: String },
    Transform { who: Mon, into: String },
    /// `lose`: a type burned out or used up; `add` (extension): a type added (Forest's Curse, Trick-or-Treat)
    Types { who: Mon, ty: Option<String>, lose: Option<String>, add: Option<String> },
    Weather { weather: String, what: StartEnd },
    Field { effect: String, what: StartEnd },
    /// `mon`: the Pokémon a side effect acted on
    SideCond { side: Side, effect: String, what: SideWhat, mon: Option<String> },
    Once { effect: String, who: Option<Mon> },
    Revive { who: Mon },
    /// "X lost N PP from MOVE!" (Spite, Eerie Spell, Grudge)
    Pp { who: Mon, mv: String, lost: u32 },
    Unread { text: String, category: Option<String> },
    /// rows were not read: a whole turn (with its number) or part of a page
    Gap { what: GapWhat, n: Option<u32> },
}

/// One set of `battle_start.own_team`.
#[derive(Clone, Debug, PartialEq)]
pub struct SetMsg {
    pub species: String,
    /// absent: holds no item
    pub item: Option<String>,
    pub ability: String,
    pub moves: Vec<String>,
    pub nature: Option<String>,
    /// hp, atk, def, spa, spd, spe
    pub points: Option<[i32; 6]>,
    pub stats: [i32; 6],
}

/// `events.fallback`: what the controller did when CG did not answer in time, or did otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Fallback {
    /// the request it answered; absent in streams that sent a bare order
    pub seq: Option<i64>,
    /// absent when the game chose for us and the controller could not read what (LV5Q-1)
    pub order: Option<Order>,
    pub reason: Option<String>,
    /// a preview fallback: the three brought, indices into `own_team`, lead first (S8-1)
    pub picks: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    BattleStart { own_team: Vec<SetMsg>, opp_preview: Vec<Option<String>> },
    PreviewRequest { deadline_ms: Option<i64> },
    /// `fallback`: one or more, applied in order before the events
    Events { events: Vec<Event>, fallback: Option<Vec<Fallback>> },
    /// `retry`: the controller repeats a tap that did not register; not a refusal
    DecisionRequest {
        seq: i64,
        kind: ReqKind,
        deadline_ms: Option<i64>,
        refused: Option<Order>,
        retry: bool,
        screen_offer: Option<Json>,
    },
    BattleEnd { result: String },
}

/// A message from the controller to CG.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub v: String,
    pub battle: String,
    pub body: Body,
}

/// CG's answer to `preview_request`: indices into `own_team`, lead first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picks {
    pub picks: Vec<u8>,
}

/// CG's answer to `decision_request`.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub seq: i64,
    pub order: Order,
    pub notes: Option<Json>,
}

// ---------------------------------------------------------------- reading

type R<T> = Result<T, String>;

fn field<'a>(o: &'a Json, k: &str) -> R<&'a Json> {
    match o.get(k) {
        Some(v) if !v.is_null() => Ok(v),
        _ => Err(format!("missing '{}'", k)),
    }
}
fn opt<'a>(o: &'a Json, k: &str) -> Option<&'a Json> {
    o.get(k).filter(|v| !v.is_null())
}
fn s(o: &Json, k: &str) -> R<String> {
    field(o, k)?.as_str().map(str::to_string).ok_or_else(|| format!("'{}' is not a string", k))
}
fn os(o: &Json, k: &str) -> R<Option<String>> {
    match opt(o, k) {
        None => Ok(None),
        Some(v) => v.as_str().map(|x| Some(x.to_string())).ok_or_else(|| format!("'{}' is not a string", k)),
    }
}
fn int(o: &Json, k: &str) -> R<i64> {
    field(o, k)?.as_i64().ok_or_else(|| format!("'{}' is not an integer", k))
}
fn oint(o: &Json, k: &str) -> R<Option<i64>> {
    match opt(o, k) {
        None => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or_else(|| format!("'{}' is not an integer", k)),
    }
}
fn num(o: &Json, k: &str) -> R<f64> {
    field(o, k)?.as_f64().ok_or_else(|| format!("'{}' is not a number", k))
}
fn flag(o: &Json, k: &str) -> R<bool> {
    match opt(o, k) {
        None => Ok(false),
        Some(v) => v.as_bool().ok_or_else(|| format!("'{}' is not a boolean", k)),
    }
}
fn strs(o: &Json, k: &str) -> R<Vec<String>> {
    let v = field(o, k)?.as_arr().ok_or_else(|| format!("'{}' is not an array", k))?;
    if v.is_empty() {
        return Err(format!("'{}' is empty", k));
    }
    v.iter().map(|x| x.as_str().map(str::to_string).ok_or_else(|| format!("'{}' holds a non-string", k))).collect()
}
fn en<T>(o: &Json, k: &str, f: fn(&str) -> Option<T>) -> R<T> {
    let v = s(o, k)?;
    f(&v).ok_or_else(|| format!("bad '{}': {}", k, v))
}
fn side(o: &Json) -> R<Side> {
    en(o, "side", Side::from_id)
}
fn mon(o: &Json) -> R<Mon> {
    Ok(Mon { side: side(o)?, mon: s(o, "mon")? })
}
/// `side` and `mon` together, or neither.
fn omon(o: &Json) -> R<Option<Mon>> {
    match (opt(o, "side"), opt(o, "mon")) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => mon(o).map(Some),
        _ => Err("'side' and 'mon' must come together".into()),
    }
}
/// `of`: an object with side and mon.
fn of(o: &Json) -> R<Option<Mon>> {
    match opt(o, "of") {
        None => Ok(None),
        Some(v) => mon(v).map(Some),
    }
}

impl Order {
    pub fn from_json(o: &Json) -> R<Order> {
        match s(o, "kind")?.as_str() {
            "move" => Ok(Order::Move { mv: s(o, "move")?, mega: flag(o, "mega")? }),
            "switch" => Ok(Order::Switch { mon: s(o, "mon")? }),
            k => Err(format!("bad order kind: {}", k)),
        }
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        match self {
            Order::Move { mv, mega } => {
                o.set("kind", "move");
                o.set("move", mv.as_str());
                o.set("mega", *mega);
            }
            Order::Switch { mon } => {
                o.set("kind", "switch");
                o.set("mon", mon.as_str());
            }
        }
        o
    }
}

impl Event {
    pub fn kind(&self) -> &'static str {
        match self {
            Event::Turn { .. } => "turn",
            Event::Refused { .. } => "refused",
            Event::End { .. } => "end",
            Event::Enter { .. } => "enter",
            Event::Withdraw { .. } => "withdraw",
            Event::Move { .. } => "move",
            Event::Prepare { .. } => "prepare",
            Event::Cant { .. } => "cant",
            Event::Protect { .. } => "protect",
            Event::Miss { .. } => "miss",
            Event::Fail { .. } => "fail",
            Event::Result { .. } => "result",
            Event::Hits { .. } => "hits",
            Event::Hp { .. } => "hp",
            Event::Cause { .. } => "cause",
            Event::Faint { .. } => "faint",
            Event::Status { .. } => "status",
            Event::Cure { .. } => "cure",
            Event::Volatile { .. } => "volatile",
            Event::VolatileEnd { .. } => "volatile_end",
            Event::VolatileHit { .. } => "volatile_hit",
            Event::Boost { .. } => "boost",
            Event::BoostLimit { .. } => "boost_limit",
            Event::BoostReset { .. } => "boost_reset",
            Event::Blocked { .. } => "blocked",
            Event::Ability { .. } => "ability",
            Event::AbilityChange { .. } => "ability_change",
            Event::Item { .. } => "item",
            Event::MegaStone { .. } => "mega_stone",
            Event::Mega { .. } => "mega",
            Event::Form { .. } => "form",
            Event::Transform { .. } => "transform",
            Event::Types { .. } => "types",
            Event::Weather { .. } => "weather",
            Event::Field { .. } => "field",
            Event::SideCond { .. } => "side",
            Event::Once { .. } => "once",
            Event::Revive { .. } => "revive",
            Event::Pp { .. } => "pp",
            Event::Unread { .. } => "unread",
            Event::Gap { .. } => "gap",
        }
    }

    /// The Pokémon the row names first, if any.
    pub fn who(&self) -> Option<&Mon> {
        match self {
            Event::Enter { who, .. }
            | Event::Withdraw { who, .. }
            | Event::Move { who, .. }
            | Event::Prepare { who, .. }
            | Event::Protect { who }
            | Event::Miss { who }
            | Event::Hp { who, .. }
            | Event::Faint { who }
            | Event::Status { who, .. }
            | Event::Cure { who, .. }
            | Event::Volatile { who, .. }
            | Event::VolatileEnd { who, .. }
            | Event::Boost { who, .. }
            | Event::BoostLimit { who, .. }
            | Event::Ability { who, .. }
            | Event::AbilityChange { who, .. }
            | Event::MegaStone { who, .. }
            | Event::Mega { who, .. }
            | Event::Form { who, .. }
            | Event::Transform { who, .. }
            | Event::Types { who, .. }
            | Event::Revive { who }
            | Event::Pp { who, .. } => Some(who),
            Event::Cant { who, .. }
            | Event::Fail { who, .. }
            | Event::Result { who, .. }
            | Event::Cause { who, .. }
            | Event::VolatileHit { who, .. }
            | Event::BoostReset { who, .. }
            | Event::Blocked { who, .. }
            | Event::Item { who, .. }
            | Event::Once { who, .. } => who.as_ref(),
            _ => None,
        }
    }

    pub fn from_json(o: &Json) -> R<Event> {
        let k = s(o, "k")?;
        Ok(match k.as_str() {
            "turn" => Event::Turn { n: int(o, "n")? as u32 },
            "refused" => Event::Refused { order: Order::from_json(field(o, "order")?)? },
            "end" => Event::End { result: s(o, "result")? },
            "enter" => Event::Enter { who: mon(o)?, forced: flag(o, "forced")? },
            "withdraw" => Event::Withdraw { who: mon(o)?, pivot: flag(o, "pivot")? },
            "move" => Event::Move {
                who: mon(o)?,
                mv: s(o, "move")?,
                priority: oint(o, "priority")?.map(|p| p as i32),
                called_by: os(o, "called_by")?,
            },
            "prepare" => Event::Prepare { who: mon(o)?, mv: os(o, "move")? },
            "cant" => Event::Cant { who: omon(o)?, reason: s(o, "reason")? },
            "protect" => Event::Protect { who: mon(o)? },
            "miss" => Event::Miss { who: mon(o)? },
            "fail" => Event::Fail { who: omon(o)?, reason: os(o, "reason")? },
            "result" => Event::Result { how: en(o, "how", Eff::from_id)?, who: omon(o)? },
            "hits" => Event::Hits { n: int(o, "n")? as u32 },
            "hp" => {
                let who = mon(o)?;
                let hp = match who.side {
                    Side::Own => Hp::Own { before: int(o, "before")? as i32, after: int(o, "after")? as i32 },
                    Side::Opp => Hp::Opp { before_pct: num(o, "before_pct")?, after_pct: num(o, "after_pct")? },
                };
                Event::Hp { who, dir: en(o, "dir", HpDir::from_id)?, hp }
            }
            "cause" => Event::Cause { effect: s(o, "effect")?, who: omon(o)?, of: of(o)? },
            "faint" => Event::Faint { who: mon(o)? },
            "status" => Event::Status { who: mon(o)?, status: s(o, "status")?, effect: os(o, "effect")? },
            "cure" => Event::Cure { who: mon(o)?, status: s(o, "status")?, item: os(o, "item")? },
            "volatile" => Event::Volatile {
                who: mon(o)?,
                effect: s(o, "effect")?,
                detail: match opt(o, "detail") {
                    None => None,
                    Some(Json::Str(x)) => Some(Detail::Id(x.clone())),
                    Some(v) => Some(Detail::Num(v.as_i64().ok_or("'detail' is not a string or integer")?)),
                },
            },
            "volatile_end" => Event::VolatileEnd { who: mon(o)?, effect: s(o, "effect")? },
            "volatile_hit" => Event::VolatileHit { effect: s(o, "effect")?, who: omon(o)? },
            "boost" => Event::Boost { who: mon(o)?, stats: strs(o, "stats")?, by: int(o, "by")? as i32 },
            "boost_limit" => Event::BoostLimit { who: mon(o)?, stats: strs(o, "stats")?, dir: en(o, "dir", LimitDir::from_id)? },
            "boost_reset" => Event::BoostReset {
                who: omon(o)?,
                what: en(o, "what", ResetWhat::from_id)?,
                of: of(o)?,
                stats: if opt(o, "stats").is_some() { Some(strs(o, "stats")?) } else { None },
            },
            // Unnerve's row names a side's team, not a Pokémon: a `side` with no `mon` is read as none
            "blocked" => Event::Blocked { effect: s(o, "effect")?, who: if opt(o, "mon").is_none() { None } else { omon(o)? }, what: os(o, "what")? },
            "ability" => Event::Ability { who: mon(o)?, ability: s(o, "ability")? },
            "ability_change" => Event::AbilityChange { who: mon(o)?, from: s(o, "from")?, to: s(o, "to")? },
            "item" => Event::Item { who: omon(o)?, item: s(o, "item")?, what: en(o, "what", ItemWhat::from_id)?, of: of(o)? },
            "mega_stone" => Event::MegaStone { who: mon(o)?, stone: s(o, "stone")? },
            "mega" => Event::Mega { who: mon(o)?, form: os(o, "form")? },
            "form" => Event::Form { who: mon(o)?, form: s(o, "form")? },
            "transform" => Event::Transform { who: mon(o)?, into: s(o, "into")? },
            "types" => Event::Types { who: mon(o)?, ty: os(o, "type")?, lose: os(o, "lose")?, add: os(o, "add")? },
            "weather" => Event::Weather { weather: s(o, "weather")?, what: en(o, "what", StartEnd::from_id)? },
            "field" => Event::Field { effect: s(o, "effect")?, what: en(o, "what", StartEnd::from_id)? },
            "side" => Event::SideCond { side: side(o)?, effect: s(o, "effect")?, what: en(o, "what", SideWhat::from_id)?, mon: os(o, "mon")? },
            "once" => Event::Once { effect: s(o, "effect")?, who: omon(o)? },
            "revive" => Event::Revive { who: mon(o)? },
            "pp" => Event::Pp { who: mon(o)?, mv: s(o, "move")?, lost: int(o, "lost")?.clamp(0, u32::MAX as i64) as u32 },
            "unread" => Event::Unread { text: s(o, "text")?, category: os(o, "category")? },
            "gap" => Event::Gap { what: en(o, "what", GapWhat::from_id)?, n: oint(o, "n")?.map(|x| x.clamp(0, u32::MAX as i64) as u32) },
            k => return Err(format!("unknown event kind: {}", k)),
        })
    }

    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.set("k", self.kind());
        fn put_mon(o: &mut Json, m: &Mon) {
            o.set("side", m.side.id());
            o.set("mon", m.mon.as_str());
        }
        fn put_omon(o: &mut Json, m: &Option<Mon>) {
            if let Some(m) = m {
                put_mon(o, m);
            }
        }
        fn put_of(o: &mut Json, m: &Option<Mon>) {
            if let Some(m) = m {
                let mut x = Json::obj();
                put_mon(&mut x, m);
                o.set("of", x);
            }
        }
        fn put_os(o: &mut Json, k: &str, v: &Option<String>) {
            if let Some(v) = v {
                o.set(k, v.as_str());
            }
        }
        match self {
            Event::Turn { n } => o.set("n", *n as i64),
            Event::Refused { order } => o.set("order", order.to_json()),
            Event::End { result } => o.set("result", result.as_str()),
            Event::Enter { who, forced } => {
                put_mon(&mut o, who);
                if *forced {
                    o.set("forced", true);
                }
            }
            Event::Withdraw { who, pivot } => {
                put_mon(&mut o, who);
                if *pivot {
                    o.set("pivot", true);
                }
            }
            Event::Move { who, mv, priority, called_by } => {
                put_mon(&mut o, who);
                o.set("move", mv.as_str());
                if let Some(p) = priority {
                    o.set("priority", *p);
                }
                put_os(&mut o, "called_by", called_by);
            }
            Event::Prepare { who, mv } => {
                put_mon(&mut o, who);
                put_os(&mut o, "move", mv);
            }
            Event::Cant { who, reason } => {
                put_omon(&mut o, who);
                o.set("reason", reason.as_str());
            }
            Event::Protect { who } | Event::Miss { who } | Event::Faint { who } | Event::Revive { who } => put_mon(&mut o, who),
            Event::Fail { who, reason } => {
                put_omon(&mut o, who);
                put_os(&mut o, "reason", reason);
            }
            Event::Result { how, who } => {
                o.set("how", how.id());
                put_omon(&mut o, who);
            }
            Event::Hits { n } => o.set("n", *n as i64),
            Event::Hp { who, dir, hp } => {
                put_mon(&mut o, who);
                o.set("dir", dir.id());
                match hp {
                    Hp::Own { before, after } => {
                        o.set("before", *before);
                        o.set("after", *after);
                    }
                    Hp::Opp { before_pct, after_pct } => {
                        o.set("before_pct", *before_pct);
                        o.set("after_pct", *after_pct);
                    }
                }
            }
            Event::Cause { effect, who, of: f } => {
                o.set("effect", effect.as_str());
                put_omon(&mut o, who);
                put_of(&mut o, f);
            }
            Event::Status { who, status, effect } => {
                put_mon(&mut o, who);
                o.set("status", status.as_str());
                put_os(&mut o, "effect", effect);
            }
            Event::Cure { who, status, item } => {
                put_mon(&mut o, who);
                o.set("status", status.as_str());
                put_os(&mut o, "item", item);
            }
            Event::Volatile { who, effect, detail } => {
                put_mon(&mut o, who);
                o.set("effect", effect.as_str());
                match detail {
                    None => {}
                    Some(Detail::Id(x)) => o.set("detail", x.as_str()),
                    Some(Detail::Num(n)) => o.set("detail", *n),
                }
            }
            Event::VolatileEnd { who, effect } => {
                put_mon(&mut o, who);
                o.set("effect", effect.as_str());
            }
            Event::VolatileHit { effect, who } | Event::Once { effect, who } => {
                o.set("effect", effect.as_str());
                put_omon(&mut o, who);
            }
            Event::Boost { who, stats, by } => {
                put_mon(&mut o, who);
                o.set("stats", stats.iter().map(|x| Json::from(x.as_str())).collect::<Vec<_>>());
                o.set("by", *by);
            }
            Event::BoostLimit { who, stats, dir } => {
                put_mon(&mut o, who);
                o.set("stats", stats.iter().map(|x| Json::from(x.as_str())).collect::<Vec<_>>());
                o.set("dir", dir.id());
            }
            Event::BoostReset { who, what, of: f, stats } => {
                put_omon(&mut o, who);
                o.set("what", what.id());
                put_of(&mut o, f);
                if let Some(st) = stats {
                    o.set("stats", st.iter().map(|x| Json::from(x.as_str())).collect::<Vec<_>>());
                }
            }
            Event::Blocked { effect, who, what } => {
                o.set("effect", effect.as_str());
                put_omon(&mut o, who);
                put_os(&mut o, "what", what);
            }
            Event::Ability { who, ability } => {
                put_mon(&mut o, who);
                o.set("ability", ability.as_str());
            }
            Event::AbilityChange { who, from, to } => {
                put_mon(&mut o, who);
                o.set("from", from.as_str());
                o.set("to", to.as_str());
            }
            Event::Item { who, item, what, of: f } => {
                put_omon(&mut o, who);
                o.set("item", item.as_str());
                o.set("what", what.id());
                put_of(&mut o, f);
            }
            Event::MegaStone { who, stone } => {
                put_mon(&mut o, who);
                o.set("stone", stone.as_str());
            }
            Event::Mega { who, form } => {
                put_mon(&mut o, who);
                put_os(&mut o, "form", form);
            }
            Event::Form { who, form } => {
                put_mon(&mut o, who);
                o.set("form", form.as_str());
            }
            Event::Transform { who, into } => {
                put_mon(&mut o, who);
                o.set("into", into.as_str());
            }
            Event::Types { who, ty, lose, add } => {
                put_mon(&mut o, who);
                put_os(&mut o, "type", ty);
                put_os(&mut o, "lose", lose);
                put_os(&mut o, "add", add);
            }
            Event::Pp { who, mv, lost } => {
                put_mon(&mut o, who);
                o.set("move", mv.as_str());
                o.set("lost", *lost as i64);
            }
            Event::Weather { weather, what } => {
                o.set("weather", weather.as_str());
                o.set("what", what.id());
            }
            Event::Field { effect, what } => {
                o.set("effect", effect.as_str());
                o.set("what", what.id());
            }
            Event::SideCond { side, effect, what, mon } => {
                o.set("side", side.id());
                o.set("effect", effect.as_str());
                o.set("what", what.id());
                put_os(&mut o, "mon", mon);
            }
            Event::Unread { text, category } => {
                o.set("text", text.as_str());
                put_os(&mut o, "category", category);
            }
            Event::Gap { what, n } => {
                o.set("what", what.id());
                if let Some(n) = n {
                    o.set("n", *n as i64);
                }
            }
        }
        o
    }
}

/// Calls `f` on every side an event names (its Pokémon, `of`, a side condition's side).
pub fn each_side(e: &mut Event, mut f: impl FnMut(&mut Side)) {
    if let Event::SideCond { side, .. } = e {
        f(side);
    }
    each_mon(e, |m| f(&mut m.side));
}

/// Calls `f` on every Pokémon an event names.
pub fn each_mon(e: &mut Event, mut f: impl FnMut(&mut Mon)) {
    match e {
        Event::Enter { who, .. }
        | Event::Withdraw { who, .. }
        | Event::Move { who, .. }
        | Event::Prepare { who, .. }
        | Event::Protect { who }
        | Event::Miss { who }
        | Event::Hp { who, .. }
        | Event::Faint { who }
        | Event::Status { who, .. }
        | Event::Cure { who, .. }
        | Event::Volatile { who, .. }
        | Event::VolatileEnd { who, .. }
        | Event::Boost { who, .. }
        | Event::BoostLimit { who, .. }
        | Event::Ability { who, .. }
        | Event::AbilityChange { who, .. }
        | Event::MegaStone { who, .. }
        | Event::Mega { who, .. }
        | Event::Form { who, .. }
        | Event::Transform { who, .. }
        | Event::Types { who, .. }
        | Event::Revive { who }
        | Event::Pp { who, .. } => f(who),
        Event::Cant { who, .. } | Event::Fail { who, .. } | Event::Result { who, .. } | Event::VolatileHit { who, .. } | Event::Blocked { who, .. } | Event::Once { who, .. } => {
            if let Some(m) = who {
                f(m)
            }
        }
        Event::Cause { who, of, .. } | Event::BoostReset { who, of, .. } | Event::Item { who, of, .. } => {
            if let Some(m) = who {
                f(m)
            }
            if let Some(m) = of {
                f(m)
            }
        }
        Event::Turn { .. } | Event::Refused { .. } | Event::End { .. } | Event::Hits { .. } | Event::Weather { .. } | Event::Field { .. } | Event::SideCond { .. } | Event::Unread { .. } | Event::Gap { .. } => {}
    }
}

const STAT_KEYS: [&str; 6] = ["hp", "atk", "def", "spa", "spd", "spe"];

fn stats6(o: &Json, k: &str) -> R<[i32; 6]> {
    let x = field(o, k)?;
    let mut out = [0; 6];
    for (i, sk) in STAT_KEYS.iter().enumerate() {
        out[i] = int(x, sk)? as i32;
    }
    Ok(out)
}
fn stats6_json(v: &[i32; 6]) -> Json {
    let mut o = Json::obj();
    for (i, sk) in STAT_KEYS.iter().enumerate() {
        o.set(sk, v[i]);
    }
    o
}

impl SetMsg {
    pub fn from_json(o: &Json) -> R<SetMsg> {
        let moves = field(o, "moves")?
            .as_arr()
            .ok_or("'moves' is not an array")?
            .iter()
            .map(|m| m.as_str().map(str::to_string).ok_or_else(|| "move is not a string".to_string()))
            .collect::<R<Vec<_>>>()?;
        Ok(SetMsg {
            species: s(o, "species")?,
            item: os(o, "item")?.filter(|x| !x.is_empty()),
            ability: s(o, "ability")?,
            moves,
            nature: os(o, "nature")?,
            points: if opt(o, "points").is_some() { Some(stats6(o, "points")?) } else { None },
            stats: stats6(o, "stats")?,
        })
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.set("species", self.species.as_str());
        if let Some(i) = &self.item {
            o.set("item", i.as_str());
        }
        o.set("ability", self.ability.as_str());
        o.set("moves", self.moves.iter().map(|m| Json::from(m.as_str())).collect::<Vec<_>>());
        if let Some(n) = &self.nature {
            o.set("nature", n.as_str());
        }
        if let Some(p) = &self.points {
            o.set("points", stats6_json(p));
        }
        o.set("stats", stats6_json(&self.stats));
        o
    }
}

impl Message {
    pub fn parse(text: &str) -> R<Message> {
        Message::from_json(&Json::parse(text)?)
    }
    pub fn from_json(o: &Json) -> R<Message> {
        let body = match s(o, "type")?.as_str() {
            "battle_start" => Body::BattleStart {
                own_team: field(o, "own_team")?
                    .as_arr()
                    .ok_or("'own_team' is not an array")?
                    .iter()
                    .map(SetMsg::from_json)
                    .collect::<R<Vec<_>>>()?,
                opp_preview: field(o, "opp_preview")?
                    .as_arr()
                    .ok_or("'opp_preview' is not an array")?
                    .iter()
                    .map(|v| match v {
                        Json::Null => Ok(None),
                        Json::Str(x) => Ok(Some(x.clone())),
                        _ => Err("preview entry is not a string or null".to_string()),
                    })
                    .collect::<R<Vec<_>>>()?,
            },
            "preview_request" => Body::PreviewRequest { deadline_ms: oint(o, "deadline_ms")? },
            "events" => Body::Events {
                events: field(o, "events")?
                    .as_arr()
                    .ok_or("'events' is not an array")?
                    .iter()
                    .map(Event::from_json)
                    .collect::<R<Vec<_>>>()?,
                fallback: Fallback::list(opt(o, "fallback"))?,
            },
            "decision_request" => Body::DecisionRequest {
                seq: int(o, "seq")?,
                kind: en(o, "kind", ReqKind::from_id)?,
                deadline_ms: oint(o, "deadline_ms")?,
                refused: match opt(o, "refused") {
                    None => None,
                    Some(v) => Some(Order::from_json(field(v, "order")?)?),
                },
                retry: flag(o, "retry")?,
                screen_offer: opt(o, "screen_offer").cloned(),
            },
            "battle_end" => Body::BattleEnd { result: s(o, "result")? },
            t => return Err(format!("unknown message type: {}", t)),
        };
        Ok(Message { v: s(o, "v")?, battle: s(o, "battle")?, body })
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.set("v", self.v.as_str());
        o.set("battle", self.battle.as_str());
        match &self.body {
            Body::BattleStart { own_team, opp_preview } => {
                o.set("type", "battle_start");
                o.set("own_team", own_team.iter().map(SetMsg::to_json).collect::<Vec<_>>());
                o.set(
                    "opp_preview",
                    opp_preview
                        .iter()
                        .map(|p| p.as_deref().map(Json::from).unwrap_or(Json::Null))
                        .collect::<Vec<_>>(),
                );
            }
            Body::PreviewRequest { deadline_ms } => {
                o.set("type", "preview_request");
                if let Some(d) = deadline_ms {
                    o.set("deadline_ms", *d);
                }
            }
            Body::Events { events, fallback } => {
                o.set("type", "events");
                o.set("events", events.iter().map(Event::to_json).collect::<Vec<_>>());
                match fallback.as_deref() {
                    Some([f]) => o.set("fallback", f.to_json()),
                    Some(fs) if !fs.is_empty() => o.set("fallback", fs.iter().map(Fallback::to_json).collect::<Vec<_>>()),
                    _ => {}
                }
            }
            Body::DecisionRequest { seq, kind, deadline_ms, refused, retry, screen_offer } => {
                o.set("type", "decision_request");
                o.set("seq", *seq);
                o.set("kind", kind.id());
                if let Some(d) = deadline_ms {
                    o.set("deadline_ms", *d);
                }
                if let Some(r) = refused {
                    let mut x = Json::obj();
                    x.set("order", r.to_json());
                    o.set("refused", x);
                }
                if *retry {
                    o.set("retry", true);
                }
                if let Some(so) = screen_offer {
                    o.set("screen_offer", so.clone());
                }
            }
            Body::BattleEnd { result } => {
                o.set("type", "battle_end");
                o.set("result", result.as_str());
            }
        }
        o
    }
}

impl Fallback {
    /// The contract's `{"seq","order","reason"}` (`order` may be absent), or a bare ORDER as older streams wrote it.
    pub fn from_json(o: &Json) -> R<Fallback> {
        if opt(o, "kind").is_some() && opt(o, "order").is_none() {
            return Ok(Fallback { seq: None, order: Some(Order::from_json(o)?), reason: None, picks: None });
        }
        let order = match opt(o, "order") {
            None => None,
            Some(x) => Some(Order::from_json(x)?),
        };
        let picks = match opt(o, "picks") {
            None => None,
            Some(_) => Some(Picks::from_json(o)?.picks),
        };
        Ok(Fallback { seq: oint(o, "seq")?, order, reason: os(o, "reason")?, picks })
    }
    /// `fallback`: one object or a list of them (S8-3).
    pub fn list(v: Option<&Json>) -> R<Option<Vec<Fallback>>> {
        match v {
            None => Ok(None),
            Some(Json::Arr(a)) => a.iter().filter(|x| !x.is_null()).map(Fallback::from_json).collect::<R<Vec<_>>>().map(|x| Some(x).filter(|x| !x.is_empty())),
            Some(x) => Ok(Some(vec![Fallback::from_json(x)?])),
        }
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        if let Some(q) = self.seq {
            o.set("seq", q);
        }
        if let Some(x) = &self.order {
            o.set("order", x.to_json());
        }
        if let Some(r) = &self.reason {
            o.set("reason", r.as_str());
        }
        if let Some(p) = &self.picks {
            o.set("picks", p.iter().map(|x| Json::from(*x as i64)).collect::<Vec<_>>());
        }
        o
    }
}

impl Picks {
    pub fn from_json(o: &Json) -> R<Picks> {
        let picks = field(o, "picks")?
            .as_arr()
            .ok_or("'picks' is not an array")?
            .iter()
            .map(|v| v.as_i64().filter(|i| (0..6).contains(i)).map(|i| i as u8).ok_or_else(|| "bad pick".to_string()))
            .collect::<R<Vec<_>>>()?;
        Ok(Picks { picks })
    }
    /// The picks a message carries in field `k` (`battle_start.picks`, S8-1), if any.
    pub fn field(o: &Json, k: &str) -> R<Option<Picks>> {
        match opt(o, k) {
            None => Ok(None),
            Some(v) => {
                let mut x = Json::obj();
                x.set("picks", v.clone());
                Picks::from_json(&x).map(Some)
            }
        }
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.set("picks", self.picks.iter().map(|p| Json::from(*p as i64)).collect::<Vec<_>>());
        o
    }
}

impl Answer {
    pub fn from_json(o: &Json) -> R<Answer> {
        Ok(Answer { seq: int(o, "seq")?, order: Order::from_json(field(o, "order")?)?, notes: opt(o, "notes").cloned() })
    }
    pub fn to_json(&self) -> Json {
        let mut o = Json::obj();
        o.set("seq", self.seq);
        o.set("order", self.order.to_json());
        if let Some(n) = &self.notes {
            o.set("notes", n.clone());
        }
        o
    }
}
