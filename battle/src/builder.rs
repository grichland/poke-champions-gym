//! The Battle builder: applies the contract's messages in order and implements LIVE.md section 5.
//! An event that cannot apply is recorded as an anomaly; nothing here panics on input.

use crate::event::*;
use crate::items;
use crate::state::*;
use cg_engine::sim::data::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Anomaly {
    /// index of the event in the battle's event list (counting every `events` entry and `battle_start` as -)
    pub at: usize,
    pub what: String,
}

/// One entry of a battle's record, in order: the controller's messages and CG's own picks and orders.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Msg(Message),
    Picks(Picks),
    /// the order CG sent for the open request
    Order(Order),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Builder {
    pub battle: Battle,
    pub anomalies: Vec<Anomaly>,
    /// events applied so far
    pub n_events: usize,
    /// the open decision request
    pub request: Option<(i64, ReqKind)>,
    /// `fallback` orders the controller reported
    pub fallbacks: Vec<Fallback>,
    /// requests that repeated a tap that did not register
    pub retries: u32,
    /// the active Pokémon and its side's turn record before the last `move` event (a nameless "no PP
    /// left" row right after it undoes the move)
    pub pre_move: Option<(Who, Active, TurnRecord, u8)>,
    /// a Berry taken and eaten by Bug Bite or Pluck: eater, Berry (its rows that follow are not the
    /// eater's own item)
    pub eaten: Option<(Who, ItemId)>,
    /// what the log put right: the picks (rule 32, a later preview fallback) and our recorded choice
    /// where a row of ours names another (rule 31)
    pub corrections: Vec<Anomaly>,
    /// our choice this turn is not known (a fallback without an order, a replay without `played`)
    pub choice_unknown: bool,
    /// per side: the withdraw before the next `enter` came from a `withdraw` row
    pub withdraw_row: [bool; 2],
}

/// Older contract versions whose messages this builder reads unchanged.
const ACCEPTED_VERSIONS: [&str; 7] = ["0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8"];

const K_OTHER: u8 = 0;
const K_HP: u8 = 1;
const K_CAUSE: u8 = 2;
const K_MOVE: u8 = 3;
const K_ABILITY: u8 = 4;

/// Moves that call another move: the move row right after theirs is the called move.
fn calls_moves(mv: MoveId) -> bool {
    matches!(mv.id(), "sleeptalk" | "copycat" | "metronome" | "assist" | "mefirst" | "mirrormove" | "naturepower")
}

/// Abilities that use a move of another Pokémon (Magic Bounce reflects it, Dancer copies it).
fn calls_by_ability(ab: AbilityId) -> bool {
    matches!(ab.id(), "magicbounce" | "dancer")
}

fn eff_cond(id: &str) -> Option<Cond> {
    let alias = match id {
        "rain" => "raindance",
        "sun" => "sunnyday",
        "snow" => "snowscape",
        "sand" => "sandstorm",
        x => x,
    };
    Cond::from_id(alias)
}

fn status_of(id: &str) -> Option<Status> {
    Some(match id {
        "brn" => Status::Brn,
        "par" => Status::Par,
        "slp" => Status::Slp,
        "frz" => Status::Frz,
        "psn" => Status::Psn,
        "tox" => Status::Tox,
        _ => return None,
    })
}

fn boost_index(id: &str) -> Option<usize> {
    let id = match id {
        "attack" => "atk",
        "defense" => "def",
        "spatk" | "specialattack" => "spa",
        "spdef" | "specialdefense" => "spd",
        "speed" => "spe",
        "evasiveness" => "evasion",
        x => x,
    };
    BOOST_IDS.iter().position(|b| *b == id)
}

/// Moves that lock the user for more turns (Outrage-like, Uproar), from the data.
fn locks_user(mv: MoveId) -> bool {
    let d = mv.data();
    d.self_effect.map_or(false, |e| e.volatile_status == conds::LOCKEDMOVE) || mv.id() == "uproar"
}

/// Abilities that take a move in instead of being hit by it (their row comes in place of a hit).
const ABSORBS: [&str; 13] = [
    "dryskin", "waterabsorb", "voltabsorb", "stormdrain", "lightningrod", "motordrive", "sapsipper", "flashfire", "wellbakedbody", "eartheater",
    "windrider", "soundproof", "bulletproof",
];

/// Causes the log writes only at the end of a turn: weather and status damage, Leech Seed, Curse, Salt
/// Cure, a partial trap's damage, end-of-turn healing, Wish, Future Sight landing.
fn residual_cause(effect: &str) -> bool {
    if matches!(effect, "brn" | "psn" | "sandstorm" | "leechseed" | "curse" | "saltcure" | "hp_restored" | "wish" | "futuresight") {
        return true;
    }
    partial_trap_move(effect)
}

/// A move id that sets a partial trap (its damage rows name it).
fn partial_trap_move(effect: &str) -> bool {
    MoveId::from_id(effect).filter(|m| !m.is_none()).map_or(false, |m| {
        let d = m.data();
        d.volatile_status == conds::PARTIALLYTRAPPED || d.secondaries.iter().any(|s| s.volatile_status == conds::PARTIALLYTRAPPED)
    })
}

fn must_recharge(mv: MoveId) -> bool {
    let d = mv.data();
    d.has_flag(flag::RECHARGE) || d.self_effect.map_or(false, |e| e.volatile_status == conds::MUSTRECHARGE)
}

/// Whether an opposing Pressure adds a PP to this move (Showdown's pressureTargets).
fn pressure_applies(mv: MoveId) -> bool {
    let d = mv.data();
    if d.has_flag(flag::MUSTPRESSURE) {
        return true;
    }
    matches!(
        d.target,
        Target::Normal | Target::AllAdjacent | Target::AllAdjacentFoes | Target::Any | Target::RandomNormal | Target::AdjacentFoe | Target::All | Target::Scripted
    )
}

fn targets_foe(mv: MoveId) -> bool {
    !matches!(
        mv.data().target,
        Target::Self_ | Target::AllySide | Target::Allies | Target::AdjacentAlly | Target::AdjacentAllyOrSelf | Target::AllyTeam
    )
}

impl Builder {
    pub fn new() -> Builder {
        Builder::default()
    }

    /// Builds a Battle from a full record in one go.
    pub fn rebuild(record: &[Input]) -> Builder {
        let mut b = Builder::new();
        for i in record {
            b.apply(i);
        }
        b
    }

    pub fn apply(&mut self, input: &Input) {
        match input {
            Input::Msg(m) => self.apply_message(m),
            Input::Picks(p) => self.apply_picks(p),
            Input::Order(o) => {
                // a replacement's answer may come before the rows of the turn it ends (LV5Q-7): the
                // turn's own choice and switch stay
                if self.request.map(|r| r.1) == Some(ReqKind::Replace) {
                    if let Order::Switch { mon } = o {
                        if self.battle.ctx.unread {
                            self.sent_in(mon);
                        }
                        return;
                    }
                }
                if self.request.map(|r| r.1) == Some(ReqKind::Turn) {
                    self.battle.ctx.unread = true;
                    self.battle.ctx.sent_in = None;
                    self.battle.ctx.replace_answer = None;
                }
                self.own_order(Some(o))
            }
        }
    }

    /// What we chose this turn: our answer, or the fallback the controller reported (None: not known).
    fn own_order(&mut self, o: Option<&Order>) {
        self.choice_unknown = o.is_none();
        self.battle.ctx.own_choice = match o {
            Some(Order::Move { mv, .. }) => MoveId::from_id(mv).filter(|m| !m.is_none()),
            _ => None,
        };
        self.battle.ctx.own_switch = match o {
            Some(Order::Switch { mon }) => SpeciesId::from_id(mon).and_then(|sp| (0..6u8).find(|&i| self.battle.sides[0].team[i as usize].species == Some(sp))),
            _ => None,
        };
    }

    /// Our replacement's answer before any row of its turn: the Pokémon it names is the one on the field at
    /// the next new replace request, unless the answer was a revival (Revival Blessing names a fainted
    /// Pokémon, which stays out).
    fn sent_in(&mut self, mon: &str) {
        let own = &self.battle.sides[0];
        let slot = SpeciesId::from_id(mon).and_then(|sp| (0..6u8).find(|&i| own.team[i as usize].species == Some(sp)));
        let revival = slot.map_or(false, |x| own.team[x as usize].fainted) && self.battle.ctx.own_choice == Some(moves::REVIVALBLESSING);
        self.battle.ctx.replace_answer = slot.filter(|_| !revival);
    }

    fn anomaly(&mut self, what: impl Into<String>) {
        self.battle.anomalies = self.battle.anomalies.saturating_add(1);
        self.anomalies.push(Anomaly { at: self.n_events, what: what.into() });
    }

    pub fn apply_message(&mut self, m: &Message) {
        if m.v != CONTRACT_VERSION && !ACCEPTED_VERSIONS.contains(&m.v.as_str()) {
            self.anomaly(format!("contract version {}", m.v));
        }
        match &m.body {
            Body::BattleStart { own_team, opp_preview } => self.start(own_team, opp_preview),
            Body::PreviewRequest { .. } => {}
            Body::Events { events, fallback } => {
                for f in fallback.iter().flatten() {
                    self.fallbacks.push(f.clone());
                    match &f.picks {
                        // the three the controller brought
                        Some(p) => self.apply_picks(&Picks { picks: p.clone() }),
                        // the game took the controller's fallback, or something it could not read
                        None => self.own_order(f.order.as_ref()),
                    }
                }
                for e in events {
                    self.apply_event(e);
                }
            }
            Body::DecisionRequest { seq, kind, refused, retry, screen_offer, .. } => {
                let prev = self.request.map(|r| r.1);
                self.request = Some((*seq, *kind));
                // a repeated tap is the same request again: nothing was refused
                if *retry {
                    self.retries = self.retries.saturating_add(1);
                    if refused.is_some() {
                        self.anomaly("decision_request: both retry and refused");
                    }
                }
                // what the game refused at one kind of request does not carry to another
                if prev != Some(*kind) && refused.is_none() {
                    self.battle.refused = [false; N_ACTIONS];
                }
                // a new replace request before any row of the turn: our last replacement went in (a retry
                // is the same screen again; a refusal means it did not)
                if *kind == ReqKind::Replace && self.battle.ctx.unread {
                    if refused.is_some() {
                        self.battle.ctx.replace_answer = None;
                    } else if !*retry {
                        if let Some(x) = self.battle.ctx.replace_answer.take() {
                            self.battle.ctx.sent_in = Some(x);
                        }
                        // a new screen is a new question: what the last one refused may be the answer now
                        // (refused as the one on the field after an unseen drag, or of the other kind of
                        // request after our Revival Blessing)
                        self.battle.refused = [false; N_ACTIONS];
                    }
                }
                // a faint is replaced after the end of the turn, and so is a switch the end of the turn
                // caused (Emergency Exit on residual damage): what lasts only to it is over
                let a = self.battle.sides[0].active;
                let fainted = a.out && a.slot.map_or(false, |s| self.battle.sides[0].team[s as usize].fainted);
                if *kind == ReqKind::Replace && (fainted || self.battle.ctx.residual) && !self.battle.ctx.post_residual {
                    self.turn_end_passed();
                }
                if let Some(so) = screen_offer {
                    self.screen_offer(so);
                }
                // no action screen is shown while locked: a turn request means our lock is over
                if *kind == ReqKind::Turn {
                    let a = &mut self.battle.sides[0].active;
                    if matches!(a.lock, Lock::Locked { .. }) {
                        a.lock = Lock::None;
                    }
                }
                if let Some(o) = refused {
                    self.refuse(o);
                }
            }
            Body::BattleEnd { result } => self.end(result),
        }
    }

    pub fn apply_picks(&mut self, p: &Picks) {
        let entered: Vec<u8> = (0..6u8).filter(|&i| self.battle.sides[0].team[i as usize].entries > 0).collect();
        if !entered.is_empty() {
            return self.picks_after_entries(p, &entered);
        }
        let b = &mut self.battle;
        b.picks = [None; 3];
        for (i, &x) in p.picks.iter().take(3).enumerate() {
            if (x as usize) < b.n_own as usize {
                b.picks[i] = Some(x);
            }
        }
        if p.picks.len() != 3 || p.picks.iter().any(|&x| x as usize >= self.battle.n_own as usize) {
            self.anomaly("picks: not three valid team indices");
        }
    }

    /// Picks that arrive after own Pokémon entered (a later preview fallback): they replace the picks
    /// when the log agrees (the lead first, every own Pokémon that entered among them); else they are an
    /// anomaly and the picks held stay. A refused switch stays with its Pokémon.
    fn picks_after_entries(&mut self, p: &Picks, entered: &[u8]) {
        let n = self.battle.n_own;
        let q = &p.picks;
        let held = self.battle.picks;
        if q.len() != 3 || q.iter().any(|&x| x >= n) || q[0] == q[1] || q[0] == q[2] || q[1] == q[2] {
            self.anomaly(format!("picks {:?}: not three valid team indices; picks {:?} kept", q, held));
            return;
        }
        let lead = (0..6u8).find(|&i| self.battle.sides[0].team[i as usize].was_lead);
        if lead.map_or(false, |l| q[0] != l) || entered.iter().any(|s| !q.contains(s)) {
            self.anomaly(format!("picks {:?} contradict the log (lead {:?}, entered {:?}); picks {:?} kept", q, lead, entered, held));
            return;
        }
        let new = [Some(q[0]), Some(q[1]), Some(q[2])];
        if new == held {
            return;
        }
        let mut refused = [false; 3];
        for i in 0..3 {
            if let (true, Some(s)) = (self.battle.refused[8 + i], held[i]) {
                if let Some(j) = new.iter().position(|x| *x == Some(s)) {
                    refused[j] = true;
                }
            }
        }
        self.battle.refused[8..11].copy_from_slice(&refused);
        self.battle.picks = new;
        let what = format!("picks {:?} corrected to {:?} by a later picks fallback", held, new);
        self.corrections.push(Anomaly { at: self.n_events, what });
    }

    /// Our active used `mv` (its own row, not a called move): the log names our choice. A recorded choice
    /// it contradicts is put right (a tap the game did not take); an unknown one is learnt.
    fn own_used(&mut self, mv: MoveId) {
        let c = &mut self.battle.ctx;
        if self.choice_unknown {
            self.choice_unknown = false;
            c.own_choice = Some(mv);
            c.own_switch = None;
            return;
        }
        let was = (c.own_choice.map(|m| m.id()), c.own_switch);
        if c.own_switch.is_some() || c.own_choice.map_or(false, |x| x != mv) {
            c.own_choice = Some(mv);
            c.own_switch = None;
            let what = format!("own {} used: our choice (move, switch slot) {:?} corrected", mv.id(), was);
            self.corrections.push(Anomaly { at: self.n_events, what });
        }
    }

    /// Our side switched by choice to team slot `to` (a withdraw row, then the enter): a recorded switch to
    /// another Pokémon is put right; an unknown choice is learnt. A recorded move is left: Emergency Exit
    /// can write the same rows, and the choice is not used once the rows show the switch.
    fn own_switched(&mut self, to: u8) {
        let c = &mut self.battle.ctx;
        if self.choice_unknown {
            self.choice_unknown = false;
            c.own_choice = None;
            c.own_switch = Some(to);
            return;
        }
        if let Some(x) = c.own_switch.filter(|&x| x != to) {
            c.own_switch = Some(to);
            let what = format!("own switch to slot {}: our choice (switch to slot {}) corrected", to, x);
            self.corrections.push(Anomaly { at: self.n_events, what });
        }
    }

    fn start(&mut self, team: &[SetMsg], preview: &[Option<String>]) {
        if self.battle.started {
            self.anomaly("second battle_start");
            return;
        }
        self.battle.started = true;
        if team.len() > 6 || preview.len() > 6 {
            self.anomaly("team or preview longer than six");
        }
        for (i, set) in team.iter().take(6).enumerate() {
            let mut os = OwnSet::default();
            match SpeciesId::from_id(&set.species) {
                Some(s) => os.species = s,
                None => self.anomaly(format!("unknown species {}", set.species)),
            }
            os.item = match &set.item {
                None => None,
                Some(x) => {
                    let it = ItemId::from_id(x);
                    if it.is_none() {
                        self.anomaly(format!("unknown item {}", x));
                    }
                    it
                }
            };
            match AbilityId::from_id(&set.ability) {
                Some(a) => os.ability = a,
                None => self.anomaly(format!("unknown ability {}", set.ability)),
            }
            for (j, m) in set.moves.iter().take(4).enumerate() {
                os.moves[j] = MoveId::from_id(m).filter(|x| !x.is_none());
                if os.moves[j].is_none() {
                    self.anomaly(format!("unknown move {}", m));
                }
            }
            os.nature = set.nature.as_deref().and_then(nature_from_id);
            os.points = set.points.map(|p| p.map(|x| x.clamp(0, 255) as u8));
            os.stats = set.stats.map(|x| x.clamp(0, 65535) as u16);
            if os.stats[0] == 0 {
                self.anomaly(format!("no max HP for {}", set.species));
            }
            let b = &mut self.battle;
            b.sets[i] = os;
            b.sides[0].team[i] = MonState {
                species: Some(os.species),
                form: Some(os.species),
                hp: 100.0,
                item: match os.item {
                    Some(it) => ItemState::Held(it),
                    None => ItemState::Gone(None),
                },
                ability: Some(os.ability),
                base_ability: Some(os.ability),
                moves: os.moves,
                ..MonState::default()
            };
        }
        self.battle.n_own = team.len().min(6) as u8;
        for (i, p) in preview.iter().take(6).enumerate() {
            let sp = p.as_deref().and_then(|x| {
                let s = SpeciesId::from_id(x);
                s
            });
            if let (Some(x), None) = (p, sp) {
                self.anomaly(format!("unknown preview species {}", x));
            }
            self.battle.sides[1].team[i] = MonState { preview: sp, hp: 100.0, ..MonState::default() };
        }
    }

    fn end(&mut self, result: &str) {
        match Outcome::from_id(result) {
            Some(o) => self.battle.result = Some(o),
            None => self.anomaly(format!("unknown result {}", result)),
        }
    }

    // ------------------------------------------------------------ name resolution

    /// Finds a Pokémon by the name it is addressed by. `add`: an opposing Pokémon may be placed on a
    /// preview entry (matching species, or an unread one).
    fn find(&mut self, m: &Mon, add: bool) -> Option<Who> {
        let sp = SpeciesId::from_id(&m.mon);
        let Some(sp) = sp else {
            self.anomaly(format!("unknown species {}", m.mon));
            return None;
        };
        let side = m.side;
        let team = &self.battle.side(side).team;
        let n = if side == Side::Own { self.battle.n_own as usize } else { 6 };
        let base = |s: SpeciesId| s.data().base_species;
        let hit = (0..n)
            .find(|&i| team[i].species == Some(sp))
            .or_else(|| (0..n).find(|&i| team[i].form == Some(sp)))
            .or_else(|| if side == Side::Opp { (0..n).find(|&i| team[i].species.is_none() && team[i].preview == Some(sp)) } else { None })
            .or_else(|| {
                (0..n).find(|&i| team[i].species.is_none() && team[i].preview.map_or(false, |p| base(p) == base(sp)))
            })
            .or_else(|| if side == Side::Opp && add { (0..n).find(|&i| team[i].species.is_none() && team[i].preview.is_none()) } else { None });
        match hit {
            Some(i) => {
                if side == Side::Opp {
                    let t = &mut self.battle.sides[1].team[i];
                    if t.species.is_none() {
                        t.species = Some(sp);
                        t.form = Some(sp);
                        if t.preview.is_none() {
                            t.preview = Some(sp);
                        }
                    }
                }
                Some(Who { side, slot: i as u8 })
            }
            None => {
                self.anomaly(format!("no {} Pokémon {}", side.id(), m.mon));
                None
            }
        }
    }

    fn is_active(&self, w: Who) -> bool {
        let a = &self.battle.side(w.side).active;
        a.slot == Some(w.slot) && !a.out
    }

    /// Resolves a Pokémon that should be on the field; an anomaly when it is not, but it still resolves.
    fn find_active(&mut self, m: &Mon) -> Option<Who> {
        let w = self.find(m, false)?;
        if !self.is_active(w) {
            self.anomaly(format!("{} {} is not active", m.side.id(), m.mon));
        }
        Some(w)
    }

    fn active_mut(&mut self, w: Who) -> Option<&mut Active> {
        let a = &mut self.battle.sides[w.side.idx()].active;
        if a.slot == Some(w.slot) {
            Some(a)
        } else {
            None
        }
    }

    // ------------------------------------------------------------ events

    pub fn apply_event(&mut self, e: &Event) {
        let turn_before = self.battle.turn;
        let prev = self.battle.ctx.prev;
        let code = match e {
            Event::Hp { .. } => K_HP,
            // the self-hit row is the cause of the HP row after it
            Event::Cause { .. } => K_CAUSE,
            Event::VolatileHit { effect, .. } if effect == "confusion_self_hit" => K_CAUSE,
            Event::Move { .. } => K_MOVE,
            Event::Ability { .. } => K_ABILITY,
            _ => K_OTHER,
        };
        if !self.battle.started && !matches!(e, Event::Unread { .. } | Event::Gap { .. }) {
            self.anomaly(format!("{} before battle_start", e.kind()));
        }
        // a log row (a refusal comes from the action screen)
        if !matches!(e, Event::Refused { .. }) {
            self.battle.ctx.unread = false;
            self.battle.ctx.sent_in = None;
            self.battle.ctx.replace_answer = None;
        }
        match e {
            Event::Turn { n } => self.turn(*n),
            Event::Refused { order } => self.refuse(order),
            Event::End { result } => self.end(result),
            Event::Enter { who, forced } => self.enter(who, *forced),
            Event::Withdraw { who, pivot } => self.withdraw(who, *pivot),
            Event::Move { who, mv, called_by, .. } => self.mv(who, mv, called_by.as_deref(), prev),
            Event::Prepare { who, mv } => self.prepare(who, mv.as_deref(), prev),
            Event::Cant { who, reason } => self.cant(who.as_ref(), reason),
            Event::Protect { who } => self.on_last_move(Some(who), |r| r.protected = true),
            Event::Miss { who } => self.on_last_move(Some(who), |r| r.missed = true),
            Event::Fail { who, reason } => {
                // a healing move failing because HP is full shows that it is
                if reason.as_deref() == Some("hpfull") {
                    if let Some(w) = who.as_ref().and_then(|m| self.lookup(m)) {
                        let mon = self.battle.mon_mut(w);
                        mon.hp = 100.0;
                        mon.hp_estimated = false;
                    }
                }
                self.on_last_move(who.as_ref(), |r| r.failed = true)
            }
            // an immune row right after an ability row is that ability failing on it (Intimidate on a
            // Substitute), not a move's
            Event::Result { who: Some(_), .. } if prev == K_ABILITY => {}
            Event::Result { how, who } => {
                let how = *how;
                self.on_last_move(who.as_ref(), move |r| r.eff = Some(how));
            }
            Event::Hits { n } => {
                let n = (*n).min(255) as u8;
                self.on_last_move(None, move |r| r.hits = n);
            }
            Event::Hp { who, dir, hp } => self.hp(who, *dir, *hp, prev),
            Event::Cause { effect, who, of } => self.cause(effect, who.as_ref(), of.as_ref(), prev),
            Event::Faint { who } => self.faint(who),
            Event::Status { who, status, .. } => self.status(who, status),
            Event::Cure { who, status, item } => self.cure(who, status, item.as_deref()),
            Event::Volatile { who, effect, detail } => self.volatile(who, effect, detail.as_ref()),
            Event::VolatileEnd { who, effect } => self.volatile_end(who, effect),
            Event::VolatileHit { effect, who } => self.volatile_hit(effect, who.as_ref()),
            Event::Boost { who, stats, by } => {
                for st in stats {
                    self.boost(who, st, *by);
                }
            }
            Event::BoostLimit { who, stats, dir } => {
                for st in stats {
                    self.boost_limit(who, st, *dir);
                }
            }
            Event::BoostReset { who, what, of, stats } => self.boost_reset(who.as_ref(), *what, of.as_ref(), stats.as_deref()),
            Event::Blocked { effect, who, .. } => self.blocked(effect, who.as_ref()),
            Event::Ability { who, ability } => self.ability(who, ability),
            Event::AbilityChange { who, from, to } => self.ability_change(who, from, to),
            Event::Item { who, item, what, of } => self.item(who.as_ref(), item, *what, of.as_ref()),
            Event::MegaStone { who, stone } => self.mega_stone(who, stone),
            Event::Mega { who, form } => self.mega(who, form.as_deref()),
            Event::Form { who, form } => self.form(who, form),
            Event::Transform { who, into } => self.transform(who, into),
            Event::Types { who, ty, lose, add } => self.types(who, ty.as_deref(), lose.as_deref(), add.as_deref()),
            Event::Weather { weather, what } => self.weather(weather, *what),
            Event::Field { effect, what } => self.field(effect, *what),
            Event::SideCond { side, effect, what, mon } => self.side_cond(*side, effect, *what, mon.as_deref()),
            Event::Once { effect, who } => self.once(effect, who.as_ref()),
            Event::Revive { who } => self.revive(who),
            Event::Pp { who, mv, lost } => self.pp(who, mv, *lost),
            Event::Unread { .. } => self.battle.unread = self.battle.unread.saturating_add(1),
            Event::Gap { .. } => {
                self.battle.gaps = self.battle.gaps.saturating_add(1);
                self.battle.incomplete = true;
            }
        }
        if let Some(m) = e.who() {
            if let Some(w) = self.lookup(m) {
                self.battle.ctx.last_named = Some(w);
            }
        }
        self.battle.ctx.prev = code;
        crate::track::after_event(&mut self.battle, e, turn_before);
        self.n_events += 1;
    }

    /// Finds a Pokémon by name without placing it or recording an anomaly.
    fn lookup(&self, m: &Mon) -> Option<Who> {
        let sp = SpeciesId::from_id(&m.mon)?;
        let team = &self.battle.side(m.side).team;
        (0..6).find(|&i| team[i].species == Some(sp)).map(|i| Who { side: m.side, slot: i as u8 })
    }

    /// A condition acted. A nameless row is about the last Pokémon named above it (LIVE.md 5.1).
    fn volatile_hit(&mut self, effect: &str, who: Option<&Mon>) {
        let w = match who {
            Some(m) => self.find(m, false),
            None => {
                let w = self.battle.ctx.last_named;
                if w.is_none() {
                    self.anomaly("nameless row with no Pokémon named before it");
                }
                w
            }
        };
        if let (Some(w), "confusion" | "confusion_self_hit") = (w, effect) {
            let r = &mut self.battle.side_mut(w.side).now;
            r.confusion_hit = true;
            if r.mon.is_none() {
                r.mon = Some(w.slot);
            }
        }
        // a Substitute taking the hit, or a hit endured: the move went on, so its charge (if any) was skipped
        if effect == "substitute" || effect == "endured_hit" {
            if let Some((u, ..)) = self.battle.ctx.last_move {
                self.released(u);
            }
        }
        if effect == "confusion_self_hit" {
            self.battle.ctx.cause_who = w;
            self.battle.ctx.undone = None;
            if let Some(w) = w {
                self.move_aborted(w, None);
                // an attempt to move all the same (Fake Out counts it)
                if let Some(a) = self.active_mut(w) {
                    a.move_actions = a.move_actions.saturating_add(1);
                }
            }
        }
        // hurting itself, the Pokémon did not use its locked move: the lock is not renewed
        if let (Some(w), "confusion_self_hit") = (w, effect) {
            if let Some(a) = self.active_mut(w) {
                if matches!(a.lock, Lock::Locked { .. }) {
                    a.lock = Lock::None;
                }
            }
        }
    }

    fn turn(&mut self, n: u32) {
        let n = n.min(u16::MAX as u32) as u16;
        let b = &mut self.battle;
        if n <= b.turn && b.turn != START_TURN {
            self.anomaly(format!("turn {} after turn {}", n, self.battle.turn));
        }
        self.pre_move = None;
        self.eaten = None;
        self.choice_unknown = false;
        let b = &mut self.battle;
        b.turn = n;
        b.refused = [false; N_ACTIONS];
        b.ctx.last_move = None;
        b.ctx.received = None;
        b.ctx.acted = 0;
        b.ctx.post_residual = false;
        b.ctx.residual = false;
        b.ctx.own_choice = None;
        b.ctx.own_switch = None;
        b.ctx.delayed = false;
        b.ctx.last_hp = None;
        for s in b.sides.iter_mut() {
            s.last = s.now;
            s.now = TurnRecord::default();
            let a = &mut s.active;
            // a locked move lasts until a row shows otherwise (5.10); charge and recharge last one turn
            a.lock = match a.lock {
                Lock::Locked { .. } => a.lock,
                Lock::Charging { turn, .. } if turn.saturating_add(1) == n => a.lock,
                Lock::Recharge { turn } if turn.saturating_add(1) == n => a.lock,
                _ => Lock::None,
            };
        }
        if let Some(f) = b.field.rooms[R_FAIRYLOCK] {
            if n > f.start.saturating_add(1) {
                b.field.rooms[R_FAIRYLOCK] = None;
            }
        }
        self.expire_turn_end(n);
    }

    /// The end of the turn has passed (a replacement is asked for a faint): the next `turn` row's expiries
    /// now, and effects starting from here count from the next turn.
    fn turn_end_passed(&mut self) {
        let next = self.battle.turn.saturating_add(1);
        self.expire_turn_end(next);
        self.battle.ctx.post_residual = true;
    }

    /// What ends silently at the end of the turn before turn `n`.
    fn expire_turn_end(&mut self, n: u16) {
        // a trapping move's hold ends silently at the end of a turn its user was not in for
        for side in [Side::Own, Side::Opp] {
            let foe = self.battle.active(side.other());
            if let Some(w) = self.battle.active(side) {
                let gone = self.battle.side(side).active.vols.iter().flatten().any(|v| v.cond == conds::PARTIALLYTRAPPED && v.by.is_some() && ((v.by != foe && !v.orphan) || (v.orphan && v.ends < n)));
                if gone {
                    self.remove_vol(w, conds::PARTIALLYTRAPPED);
                }
                // Endure lasts the turn it was used
                self.remove_vol(w, conds::ENDURE);
                // Yawn acts at the end of the turn after it began, with no row of its own; Throat Chop ends then
                for c in [Some(conds::YAWN), Cond::from_id("throatchop")].into_iter().flatten() {
                    let over = self.battle.side(side).active.vols.iter().flatten().any(|v| v.cond == c && v.start.saturating_add(2) <= n);
                    if over {
                        self.remove_vol(w, c);
                    }
                }
            }
        }
    }

    /// An order the game refused: illegal for the rest of this request (rule 10, section 3).
    fn refuse(&mut self, o: &Order) {
        match o {
            Order::Move { mv, .. } => {
                let found = self.battle.active(Side::Own).and_then(|w| {
                    let id = MoveId::from_id(mv)?;
                    let a = &self.battle.side(Side::Own).active;
                    let moves = if a.transformed.is_some() { a.t_moves } else { self.battle.mon(w).moves };
                    moves.iter().position(|m| *m == Some(id))
                });
                match found {
                    Some(i) => {
                        self.battle.refused[i] = true;
                        self.battle.refused[i + 4] = true;
                    }
                    None => self.anomaly(format!("refused move {} not on the active Pokémon", mv)),
                }
            }
            // at a turn request the game refuses a switch to a healthy benched Pokémon only when trapped:
            // all switches go. At a replace request there is no trap: only that target goes.
            Order::Switch { mon } => {
                if self.request.map(|r| r.1) == Some(ReqKind::Replace) {
                    let sp = SpeciesId::from_id(mon);
                    let slot = (0..6).find(|&i| sp.is_some() && self.battle.sides[0].team[i].species == sp);
                    match slot.and_then(|s| self.battle.picks.iter().position(|p| *p == Some(s as u8))) {
                        Some(i) => self.battle.refused[8 + i] = true,
                        None => self.anomaly(format!("refused switch to {} which was not picked", mon)),
                    }
                } else {
                    for i in 8..N_ACTIONS {
                        self.battle.refused[i] = true;
                    }
                    // trapped by the opposing Pokémon, by something not shown: it stays so while both stay in
                    let foe = self.battle.active(Side::Opp);
                    self.battle.sides[0].active.trapped_by = foe;
                }
            }
        }
    }

    /// The turn a field, side or weather effect starting now counts from: the next one when this turn's end
    /// already passed (it started as a fainted Pokémon was replaced).
    fn effect_start(&self) -> u16 {
        self.battle.turn.saturating_add(self.battle.ctx.post_residual as u16)
    }

    fn next_order(&mut self) -> u8 {
        self.battle.ctx.acted += 1;
        self.battle.ctx.acted
    }

    /// Runs `f` on the record of the most recent move (rule 1). `who` names the Pokémon the row is about
    /// (protected, avoided, immune); it must be the move's target or user.
    fn on_last_move(&mut self, who: Option<&Mon>, f: impl FnOnce(&mut TurnRecord)) {
        let Some((user, mv, target)) = self.battle.ctx.last_move else {
            self.anomaly("nameless row with no move before it");
            return;
        };
        // a delayed attack's rows: its user did nothing this turn
        if self.battle.ctx.delayed {
            return;
        }
        if let Some(m) = who {
            if let Some(w) = self.find(m, false) {
                if Some(w) != target && w != user {
                    self.anomaly(format!("{} {} is not in the last move", m.side.id(), m.mon));
                }
            }
        }
        self.released(user);
        let s = self.battle.side_mut(user.side);
        f(&mut s.now);
        let r = s.now;
        // a move that did not connect sets up no recharge and no lock
        let a = &mut s.active;
        if a.slot == Some(user.slot) && (r.missed || r.failed || r.protected || r.eff == Some(Eff::Immune)) {
            match a.lock {
                Lock::Recharge { .. } => a.lock = Lock::None,
                // a locked move that did not connect is not renewed: the lock ends with the turn
                Lock::Locked { mv: m, .. } if m == mv => a.lock = Lock::None,
                _ => {}
            }
        }
    }

    /// A row of the move after its charge row in the same turn: the charge was skipped (sun for Solar
    /// Beam, Power Herb), so there is no charging turn to follow.
    fn released(&mut self, user: Who) {
        let turn = self.battle.turn;
        let a = &mut self.battle.sides[user.side.idx()].active;
        if a.slot == Some(user.slot) && matches!(a.lock, Lock::Charging { turn: t, .. } if t == turn) {
            a.lock = Lock::None;
        }
    }

    fn enter(&mut self, m: &Mon, forced: bool) {
        let Some(w) = self.find(m, true) else { return };
        let side = w.side;
        let prev = self.battle.side(side).active;
        let withdrawn = self.battle.ctx.withdrawn[side.idx()];
        let turn = self.battle.turn;
        if self.battle.mon(w).fainted {
            // the faint was an Illusion user's under this Pokémon's name: it goes to the opposing
            // Pokémon that can have Illusion, and this one is whole again
            let liar = (0..6).find(|&i| {
                let t = &self.battle.sides[1].team[i];
                i != w.slot as usize && !t.fainted && !t.seen && t.preview.map_or(false, |p| p.data().abilities.contains(&abilities::ILLUSION))
            });
            match liar.filter(|_| side == Side::Opp) {
                Some(i) => {
                    let t = &mut self.battle.sides[1].team[i];
                    t.species = t.preview;
                    t.form = t.preview;
                    t.seen = true;
                    t.fainted = true;
                    t.hp = 0.0;
                    t.ability = Some(abilities::ILLUSION);
                    t.base_ability = Some(abilities::ILLUSION);
                    let mon = self.battle.mon_mut(w);
                    mon.fainted = false;
                    mon.hp = 100.0;
                    mon.hp_estimated = true;
                }
                None => self.anomaly(format!("{} {} enters fainted", side.id(), m.mon)),
            }
        }
        let mut withdrawn = withdrawn;
        if prev.slot.is_some() && !prev.out && prev.slot != Some(w.slot) {
            // the previous Pokémon left without a withdraw row: a drag; Baton Pass, Shed Tail and other
            // switching moves the game writes no withdraw row for; Emergency Exit or Wimp Out; or a lost row
            let old = Who { side, slot: prev.slot.unwrap() };
            let now = self.battle.side(side).now;
            let by_move = now.act == Act::Move && now.mon == prev.slot && now.mv.map_or(false, |m| m.data().self_switch != SelfSwitch::No);
            let by_ability = matches!(self.battle.mon(old).ability.map(|a| a.id()), Some("emergencyexit") | Some("wimpout"));
            if by_move {
                match now.mv.unwrap().data().self_switch {
                    SelfSwitch::CopyVolatile => self.battle.ctx.passed[side.idx()] = Some((prev, true)),
                    SelfSwitch::ShedTail => self.battle.ctx.passed[side.idx()] = Some((prev, false)),
                    _ => {}
                }
                withdrawn = Some((old.slot, true));
            } else if by_ability {
                withdrawn = Some((old.slot, false));
            } else if !forced {
                self.anomaly(format!("{} enter without withdraw", side.id()));
            }
            self.leave(old);
        }
        if prev.slot == Some(w.slot) && !prev.out {
            self.anomaly(format!("{} {} enters while on the field", side.id(), m.mon));
        }
        // a fainted Pokemon is replaced after the end of the turn
        if prev.slot.is_some() && prev.out && prev.slot.map_or(false, |s| self.battle.side(side).team[s as usize].fainted) && turn != START_TURN {
            self.battle.ctx.post_residual = true;
        }
        // a switch during the end of the turn (Emergency Exit on residual damage) comes after it
        if self.battle.ctx.residual && turn != START_TURN {
            self.battle.ctx.post_residual = true;
        }
        let passed = self.battle.ctx.passed[side.idx()].take();
        // what the Battle held for the entering Pokémon, for an Illusion found out later
        self.battle.ctx.entering[side.idx()] = Some((w.slot, *self.battle.mon(w)));
        let mut a = Active { slot: Some(w.slot), entered: turn, ..Active::default() };
        if let Some((old, baton)) = passed {
            if baton {
                a.boosts = old.boosts;
                let mut k = 0;
                for v in old.vols.iter().flatten() {
                    if !v.cond.data().no_copy {
                        a.vols[k] = Some(*v);
                        k += 1;
                    }
                }
            } else {
                a.vols[0] = old.vols.iter().flatten().find(|v| v.cond == conds::SUBSTITUTE).copied();
            }
        }
        let b = &mut self.battle;
        b.side_mut(side).active = a;
        let mon = b.mon_mut(w);
        mon.seen = true;
        mon.entries = mon.entries.saturating_add(1);
        // the first Pokémon of a side to enter (the `turn` 1 event may come before the leads)
        if prev.slot.is_none() {
            mon.was_lead = true;
        }
        if mon.form.is_none() {
            mon.form = mon.species;
        }
        if mon.status == Status::Tox {
            mon.status_count = 0;
        }
        let mut corrected = None;
        if side == Side::Own {
            b.refused = [false; N_ACTIONS];
            // the picks held were not the ones played: a lead other than the first pick, or a Pokémon
            // outside full picks. The log corrects them; picks not contradicted stay (rule 32).
            let wrong = if prev.slot.is_none() {
                b.picks.iter().any(|p| p.is_some()) && b.picks[0] != Some(w.slot)
            } else {
                !b.picks.contains(&Some(w.slot)) && !b.picks.contains(&None)
            };
            if wrong {
                let was = b.picks;
                let team = &b.sides[0].team;
                if prev.slot.is_none() {
                    // the lead is the first pick: it swaps with it, or takes its place
                    match b.picks.iter().position(|p| *p == Some(w.slot)) {
                        Some(i) => b.picks.swap(0, i),
                        None => b.picks[0] = Some(w.slot),
                    }
                } else if let Some(i) = (0..3).find(|&i| b.picks[i].map_or(false, |s| team[s as usize].entries == 0)) {
                    // the first pick not seen entering was not brought
                    b.picks[i] = Some(w.slot);
                }
                corrected = Some(was);
            }
            // brought Pokémon are known from the picks; without them, from what entered
            if !b.picks.contains(&Some(w.slot)) {
                if let Some(i) = b.picks.iter().position(|p| p.is_none()) {
                    b.picks[i] = Some(w.slot);
                }
            }
        }
        b.ctx.withdrawn[side.idx()] = None;
        if turn != START_TURN {
            if forced {
                let r = &mut b.side_mut(side).now;
                r.dragged = true;
                r.to = Some(w.slot);
            } else if let Some((from, pivot)) = withdrawn {
                if pivot {
                    let r = &mut b.side_mut(side).now;
                    r.act = Act::Pivot;
                    r.to = Some(w.slot);
                } else if b.side(side).now.act == Act::None {
                    let order = self.next_order();
                    let r = &mut self.battle.side_mut(side).now;
                    *r = TurnRecord { act: Act::Switch, mon: Some(from), to: Some(w.slot), order, ..TurnRecord::default() };
                    if side == Side::Own && self.withdraw_row[0] {
                        self.own_switched(w.slot);
                    }
                } else {
                    b.side_mut(side).now.to = Some(w.slot);
                }
            }
        }
        self.withdraw_row[side.idx()] = false;
        if let Some(was) = corrected {
            let what = format!("own {} entered: picks {:?} corrected to {:?}", m.mon, was, self.battle.picks);
            self.corrections.push(Anomaly { at: self.n_events, what });
        }
        if side == Side::Own && !self.battle.picks.contains(&Some(w.slot)) {
            self.anomaly(format!("own {} entered but was not picked", m.mon));
        }
        let opp_seen = self.battle.sides[1].team.iter().filter(|t| t.seen).count();
        if side == Side::Opp && opp_seen > 3 {
            self.anomaly("more than three opposing Pokémon entered");
        }
    }

    /// The Pokémon leaves the field: abilities that act on switching out (silent in Champions).
    fn leave(&mut self, w: Who) {
        let b = &mut self.battle;
        let set_hp = b.sets[w.slot as usize].stats[0];
        let mon = b.mon_mut(w);
        if mon.fainted {
            return;
        }
        self.drop_linked(w);
        // Gastro Acid suppresses the ability on the way out too
        let suppressed = self.battle.side(w.side).active.slot == Some(w.slot) && self.battle.side(w.side).active.vols.iter().flatten().any(|v| v.cond.id() == "gastroacid");
        let mon = self.battle.mon_mut(w);
        // the ability it has now acts on the way out; then an ability changed on the field (Trace, Skill
        // Swap, Transform) is the brought one again, known or not
        let now = if suppressed { None } else { mon.ability };
        mon.ability = mon.base_ability;
        match now.map(|a| a.id()) {
            Some("regenerator") => {
                mon.hp = if w.side == Side::Own && set_hp > 0 {
                    let max = set_hp as f32;
                    let cur = (mon.hp * max / 100.0).round();
                    ((cur + (max / 3.0).floor()).min(max)) * 100.0 / max
                } else {
                    mon.hp_estimated = true;
                    (mon.hp + 100.0 / 3.0).min(100.0)
                };
            }
            Some("naturalcure") => mon.status = Status::None,
            _ => {}
        }
        if let Some(a) = self.active_mut(w) {
            a.out = true;
        }
    }

    fn withdraw(&mut self, m: &Mon, pivot: bool) {
        let Some(w) = self.find_active(m) else { return };
        let side = w.side;
        // Baton Pass and Shed Tail pass state to the next Pokémon
        let last = self.battle.side(side).now;
        if last.mon == Some(w.slot) {
            if let Some(mv) = last.mv {
                match mv.data().self_switch {
                    SelfSwitch::CopyVolatile => self.battle.ctx.passed[side.idx()] = Some((self.battle.side(side).active, true)),
                    SelfSwitch::ShedTail => self.battle.ctx.passed[side.idx()] = Some((self.battle.side(side).active, false)),
                    _ => {}
                }
            }
        }
        // a pivot withdraw with no move above it is accepted: the Move row can be lost (5.9)
        let _ = pivot;
        self.leave(w);
        self.battle.ctx.withdrawn[side.idx()] = Some((w.slot, pivot));
        self.withdraw_row[side.idx()] = true;
    }

    fn mv(&mut self, m: &Mon, mv: &str, called_by: Option<&str>, prev: u8) {
        let Some(w) = self.find_active(m) else { return };
        let Some(id) = MoveId::from_id(mv).filter(|x| !x.is_none()) else {
            self.anomaly(format!("unknown move {}", mv));
            return;
        };
        let side = w.side;
        let turn = self.battle.turn;
        let before = self.battle.ctx;
        self.pre_move = Some((w, self.battle.side(side).active, self.battle.side(side).now, before.acted));
        self.eaten = None;
        let target = if targets_foe(id) { self.battle.active(side.other()) } else { Some(w) };
        self.battle.ctx.last_move = Some((w, id, target));
        self.battle.ctx.delayed = false;
        // an exchange's two rows follow one move
        self.battle.ctx.received = None;
        self.battle.ctx.last_actor = Some(w);
        let struggle = id == moves::STRUGGLE;

        // a called move (OE-11) is not one of the user's moves: named by `called_by`, or, as the phone
        // writes it, the move row right after a calling move's row of the same Pokémon, or right after
        // its Magic Bounce or Dancer row
        let by_move = match called_by {
            Some(c) => MoveId::from_id(c).filter(|x| !x.is_none()),
            None => before.last_move.filter(|(u, x, _)| prev == K_MOVE && *u == w && calls_moves(*x)).map(|(_, x, _)| x),
        };
        let by_ability = match called_by {
            Some(c) => AbilityId::from_id(c).filter(|a| a.0 != 0),
            None => before.last_ability.filter(|(u, a)| prev == K_ABILITY && *u == w && calls_by_ability(*a)).map(|(_, a)| a),
        };
        if called_by.is_some() && by_move.is_none() && by_ability.is_none() {
            self.anomaly(format!("unknown calling move {:?}", called_by));
        }
        if let Some(ab) = by_ability {
            self.reveal_ability(w, ab);
            return;
        }
        if let Some(c) = by_move {
            self.battle.side_mut(side).now.called = Some(id);
            // Sleep Talk calls one of the user's own moves
            if c.id() == "sleeptalk" && !struggle {
                self.slot_of(w, id, true);
            }
            if self.pressure_cost(w, id) {
                if let Some(i) = self.slot_of(w, c, false) {
                    self.spend_pp(w, i, 1);
                }
            }
            return;
        }

        if side == Side::Own {
            self.own_used(id);
        }
        let a = self.battle.side(side).active;
        let (continuing, charge_counted) = match a.lock {
            // a fourth use is a new choice: the lock lasts three turns at most (Uproar: any number)
            Lock::Locked { mv: lm, last, uses } => (lm == id && last < turn && (uses < 3 || lm.id() == "uproar"), false),
            Lock::Charging { mv: cm, turn: t, pp_counted } => {
                if (cm == Some(id) || cm.is_none()) && t.saturating_add(1) == turn {
                    (true, pp_counted)
                } else if (cm == Some(id) || cm.is_none()) && t == turn {
                    // the charge row came before the move row of the same turn
                    (true, pp_counted)
                } else {
                    (false, false)
                }
            }
            _ => (false, false),
        };
        let charging_now = matches!(a.lock, Lock::Charging { turn: t, .. } if t == turn);
        // a lock going on while the Pokémon is confused may have ended without its fatigue row (it is
        // confused already): the PP counted for it may be one short
        if continuing && matches!(a.lock, Lock::Locked { .. }) && a.vols.iter().flatten().any(|v| v.cond == conds::CONFUSION) {
            self.battle.mon_mut(w).pp_uncertain = true;
        }
        if !struggle {
            let slot = self.slot_of(w, id, true);
            let pay = !continuing || (matches!(a.lock, Lock::Charging { .. }) && !charge_counted);
            if pay {
                if let Some(i) = slot {
                    let n = 1 + self.pressure_cost(w, id) as u8;
                    self.spend_pp(w, i, n);
                }
            }
        }
        let order = if self.battle.side(side).now.order == 0 { self.next_order() } else { self.battle.side(side).now.order };
        let b = &mut self.battle;
        let s = b.side_mut(side);
        if charging_now {
            if let Lock::Charging { mv: None, turn: t, .. } = s.active.lock {
                s.active.lock = Lock::Charging { mv: Some(id), turn: t, pp_counted: true };
            }
        }
        s.now = TurnRecord { act: Act::Move, mon: Some(w.slot), mv: Some(id), order, ..TurnRecord::default() };
        let a = &mut s.active;
        a.last_move = Some(id);
        if !charging_now {
            a.move_actions = a.move_actions.saturating_add(1);
            a.lock = match a.lock {
                Lock::Locked { mv: lm, uses, .. } if continuing && lm == id => Lock::Locked { mv: id, uses: uses.saturating_add(1), last: turn },
                _ if continuing => Lock::None,
                _ if locks_user(id) => Lock::Locked { mv: id, uses: 1, last: turn },
                _ if must_recharge(id) => Lock::Recharge { turn },
                _ => Lock::None,
            };
        }
        // Charge is used up by an Electric move, without a row
        if id.data().ty == Type::Electric && id.id() != "charge" && a.vols.iter().flatten().any(|v| v.cond == conds::CHARGE) {
            self.remove_vol(w, conds::CHARGE);
        }
        let b = &mut self.battle;
        let a = &mut b.sides[side.idx()].active;
        // a lock on a move the Pokémon no longer has (it transformed since) is gone
        let cur = if a.transformed.is_some() { a.t_moves } else { b.sides[side.idx()].team[w.slot as usize].moves };
        if a.choice_lock.map_or(false, |l| !cur.contains(&Some(l))) {
            a.choice_lock = None;
        }
        if !struggle && a.choice_lock.is_none() {
            let mon = &b.sides[side.idx()].team[w.slot as usize];
            let choice_item = matches!(mon.item, ItemState::Held(it) if it.data().is_choice);
            let gorilla = mon.ability.map(|x| x.id()) == Some("gorillatactics");
            if choice_item || gorilla {
                b.sides[side.idx()].active.choice_lock = Some(id);
            }
        }
    }

    /// Move slot of `mv` on a Pokémon (its transformed moves while transformed). `reveal`: an opponent's
    /// unseen move takes the next free slot.
    fn slot_of(&mut self, w: Who, mv: MoveId, reveal: bool) -> Option<usize> {
        let transformed = self.battle.side(w.side).active.slot == Some(w.slot) && self.battle.side(w.side).active.transformed.is_some();
        let moves = if transformed { self.battle.side(w.side).active.t_moves } else { self.battle.mon(w).moves };
        if let Some(i) = moves.iter().position(|m| *m == Some(mv)) {
            return Some(i);
        }
        if !reveal {
            return None;
        }
        if w.side == Side::Opp || transformed {
            if let Some(i) = moves.iter().position(|m| m.is_none()) {
                if transformed {
                    self.battle.side_mut(w.side).active.t_moves[i] = Some(mv);
                } else {
                    self.battle.mon_mut(w).moves[i] = Some(mv);
                }
                return Some(i);
            }
        }
        self.anomaly(format!("{} used {} which is not among its moves", w.side.id(), mv.id()));
        None
    }

    fn spend_pp(&mut self, w: Who, i: usize, n: u8) {
        let a = &mut self.battle.sides[w.side.idx()].active;
        if a.slot == Some(w.slot) && a.transformed.is_some() {
            a.t_pp_used[i] = a.t_pp_used[i].saturating_add(n).min(5);
            return;
        }
        let mon = self.battle.mon_mut(w);
        if let Some(mv) = mon.moves[i] {
            mon.pp_used[i] = mon.pp_used[i].saturating_add(n).min(mv.data().max_pp);
        }
    }

    /// A use of `mv` by `w` costs one more PP: an opposing Pressure shown and a move it applies to (Curse by
    /// a Pokémon that is not a Ghost targets its user).
    fn pressure_cost(&self, w: Who, mv: MoveId) -> bool {
        if mv == moves::CURSE && !self.battle.types(w).map_or(false, |t| t.contains(&Type::Ghost)) {
            return false;
        }
        pressure_applies(mv) && self.pressure_against(w)
    }

    /// An opposing Pressure that has been shown (ours is known from the set).
    fn pressure_against(&self, w: Who) -> bool {
        match self.battle.active(w.side.other()) {
            Some(o) => self.battle.mon(o).ability.map(|a| a.id()) == Some("pressure") && !self.battle.side(o.side).active.vols.iter().flatten().any(|v| v.cond.id() == "gastroacid"),
            None => false,
        }
    }

    fn prepare(&mut self, m: &Mon, mv: Option<&str>, prev: u8) {
        let Some(w) = self.find_active(m) else { return };
        let turn = self.battle.turn;
        let mut id = mv.and_then(MoveId::from_id);
        if mv.is_some() && id.is_none() {
            self.anomaly(format!("unknown move {:?}", mv));
        }
        // a move row of this Pokémon just before is the charging turn's own row: PP already spent
        let after_move = prev == K_MOVE && self.battle.ctx.last_move.map(|(u, ..)| u) == Some(w);
        if after_move && id.is_none() {
            id = self.battle.ctx.last_move.map(|(_, x, _)| x);
        }
        let mut counted = after_move;
        if !after_move {
            if id.is_none() {
                let a = &mut self.battle.side_mut(w.side).active;
                a.move_actions = a.move_actions.saturating_add(1);
            }
            if let Some(x) = id {
                if w.side == Side::Own {
                    self.own_used(x);
                }
                if let Some(i) = self.slot_of(w, x, true) {
                    let n = 1 + self.pressure_cost(w, x) as u8;
                    self.spend_pp(w, i, n);
                }
                counted = true;
                let order = if self.battle.side(w.side).now.order == 0 { self.next_order() } else { self.battle.side(w.side).now.order };
                let a = &mut self.battle.side_mut(w.side).active;
                a.move_actions = a.move_actions.saturating_add(1);
                a.last_move = Some(x);
                self.battle.side_mut(w.side).now = TurnRecord { act: Act::Move, mon: Some(w.slot), mv: Some(x), order, ..TurnRecord::default() };
            }
        }
        self.battle.side_mut(w.side).active.lock = Lock::Charging { mv: id, turn, pp_counted: counted };
    }

    fn cant(&mut self, m: Option<&Mon>, reason: &str) {
        let w = match m {
            Some(m) => self.find_active(m),
            // the nameless "no PP left" row is about the move row above it (5.1)
            None => {
                let u = self.battle.ctx.last_move.map(|(u, ..)| u);
                if u.is_none() {
                    self.anomaly("nameless row with no move before it");
                }
                u
            }
        };
        let Some(w) = w else { return };
        // no PP left: the move row above was not a use (no lock, no Choice lock, no last move), and the
        // move has no PP now
        if reason == "nopp" && self.battle.ctx.prev == K_MOVE {
            if let (Some((pw, act, now, acted)), Some((_, mv, _))) = (self.pre_move, self.battle.ctx.last_move) {
                if pw == w {
                    self.battle.side_mut(w.side).active = act;
                    self.battle.side_mut(w.side).now = now;
                    self.battle.ctx.acted = acted;
                    if let Some(i) = self.slot_of(w, mv, false) {
                        self.spend_pp(w, i, u8::MAX);
                    }
                }
            }
        }
        // an ability of another Pokémon stopped the move (Damp, Dazzling, Queenly Majesty, Armor Tail): the
        // row names the move, which is not a condition even when a condition has its name (Taunt)
        let stopped = self.battle.ctx.prev == K_ABILITY
            && self.battle.ctx.last_ability.map_or(false, |(h, ab)| h != w && matches!(ab.id(), "damp" | "dazzling" | "queenlymajesty" | "armortail"));
        let r = if let Some(mv) = MoveId::from_id(reason).filter(|x| stopped && !x.is_none()) {
            CantReason::Move(mv)
        } else if let Some(s) = status_of(reason) {
            CantReason::Status(s)
        } else if let Some(c) = Cond::from_id(reason).filter(|c| !c.is_none()) {
            CantReason::Cond(c)
        } else if let Some(mv) = MoveId::from_id(reason).filter(|x| !x.is_none()) {
            CantReason::Move(mv)
        } else if let Some(ab) = AbilityId::from_id(reason).filter(|a| a.0 != 0) {
            // an ability that stopped the move (Truant)
            self.reveal_ability(w, ab);
            CantReason::Other
        } else {
            CantReason::Other
        };
        let named = match r {
            CantReason::Move(mv) => Some(mv),
            _ if reason == "nopp" => self.battle.ctx.last_move.filter(|(u, ..)| *u == w).map(|(_, mv, _)| mv),
            _ => None,
        };
        self.move_aborted(w, named);
        let order = if self.battle.side(w.side).now.order == 0 { self.next_order() } else { self.battle.side(w.side).now.order };
        let b = &mut self.battle;
        b.side_mut(w.side).now = TurnRecord { act: Act::Cant, mon: Some(w.slot), order, cant: Some(r), ..TurnRecord::default() };
        let a = &mut b.side_mut(w.side).active;
        a.move_actions = a.move_actions.saturating_add(1);
        a.lock = Lock::None;
        if r == CantReason::Status(Status::Slp) {
            let mon = b.mon_mut(w);
            mon.status_count = mon.status_count.saturating_add(1);
            if mon.status != Status::Slp {
                self.anomaly(format!("{} {} cant slp while not asleep", w.side.id(), m.map_or("?", |m| m.mon.as_str())));
            }
        }
    }

    /// A Pokémon could not use the move it was to use: Charge ends silently when that move was Electric.
    /// Ours is the move the row names, else our choice; the opponent's is not shown, so its Charge may be
    /// gone, and so may ours when our choice is unknown.
    fn move_aborted(&mut self, w: Who, named: Option<MoveId>) {
        if w.side != Side::Own {
            if let Some(a) = self.active_mut(w) {
                for v in a.vols.iter_mut().flatten() {
                    if v.cond == conds::CHARGE {
                        v.maybe_gone = true;
                    }
                }
            }
            return;
        }
        if let Some(m) = named {
            self.own_used(m);
        }
        let a = &self.battle.side(Side::Own).active;
        let lock = match a.lock {
            Lock::Locked { mv, .. } => Some(mv),
            Lock::Charging { mv, .. } => mv,
            _ => None,
        };
        let mv = named.or(self.battle.ctx.own_choice).or(lock);
        if mv.map_or(false, |m| m.data().ty == Type::Electric && m.id() != "charge") {
            self.remove_vol(w, conds::CHARGE);
        } else if mv.is_none() && self.choice_unknown {
            if let Some(a) = self.active_mut(w) {
                for v in a.vols.iter_mut().flatten() {
                    if v.cond == conds::CHARGE {
                        v.maybe_gone = true;
                    }
                }
            }
        }
    }

    fn hp(&mut self, m: &Mon, dir: HpDir, hp: Hp, prev: u8) {
        let Some(w) = self.find(m, false) else { return };
        let (before, after) = match hp {
            Hp::Own { before, after } => {
                if w.side != Side::Own {
                    self.anomaly("own HP numbers on an opposing Pokémon");
                    return;
                }
                let max = self.battle.sets[w.slot as usize].stats[0];
                if max == 0 {
                    self.anomaly("own HP with no max HP");
                    return;
                }
                if after > max as i32 || before > max as i32 {
                    self.anomaly(format!("own HP above max for {}", m.mon));
                }
                let pct = |x: i32| (x.clamp(0, max as i32) as f32) * 100.0 / max as f32;
                (pct(before), pct(after))
            }
            Hp::Opp { before_pct, after_pct } => {
                if w.side != Side::Opp {
                    self.anomaly("percent HP on an own Pokémon");
                    return;
                }
                (before_pct.clamp(0.0, 100.0) as f32, after_pct.clamp(0.0, 100.0) as f32)
            }
        };
        let tol = if w.side == Side::Own { 0.01 } else { 1.0 };
        let held = self.battle.mon(w).hp;
        if (held - before).abs() > tol {
            // an opposing Pokémon back on the field with more HP than it left with: Regenerator's silent
            // heal, the only one there is (rule 11)
            let mon = self.battle.mon(w);
            let regen = w.side == Side::Opp && before > held && (mon.base_ability.map_or(true, |a| a == abilities::REGENERATOR) || mon.mega) && self.battle.side(w.side).active.slot == Some(w.slot);
            if regen {
                let mon = self.battle.mon_mut(w);
                if mon.ability.is_none() {
                    mon.ability = Some(abilities::REGENERATOR);
                    mon.base_ability = Some(abilities::REGENERATOR);
                }
            } else if !(self.battle.mon(w).hp_estimated && (held - before).abs() <= 2.0) {
                // (a Battle's estimate after a silent heal is corrected by the row without an anomaly)
                self.anomaly(format!("{} {} HP was {} by the Battle, {} by the row", w.side.id(), m.mon, held, before));
            }
        }
        if (dir == HpDir::Lost) != (after < before) && after != before {
            self.anomaly("HP row direction does not match its numbers");
        }
        self.battle.mon_mut(w).hp = after;
        self.battle.mon_mut(w).hp_estimated = false;
        let loss = before - after;
        // rule 2: the cause row beside it (the one before wins); with none, the most recent move
        let ctx = self.battle.ctx;
        let cause_here = prev == K_CAUSE && (ctx.cause_who.is_none() || ctx.cause_who == Some(w));
        if cause_here {
            // the cause row was this change's, not the one before it: give that one back to its move
            if let Some(u) = ctx.undone {
                if let Some(s) = u.move_side {
                    self.battle.side_mut(s).now.damage += u.loss;
                    let mon = self.battle.mon_mut(u.who);
                    mon.biggest_hit = mon.biggest_hit.max(u.loss);
                }
            }
        }
        self.battle.ctx.undone = None;
        let mut link = HpLink { who: w, loss: loss.max(0.0), move_side: None, prev_biggest: self.battle.mon(w).biggest_hit };
        if loss > 0.0 && !cause_here && !self.battle.ctx.delayed {
            if let Some((user, mv, _)) = self.battle.ctx.last_move {
                if user.side != w.side && mv.id() == "throatchop" {
                    self.throat_chop(user, w);
                }
                if user.side != w.side {
                    self.released(user);
                    link.move_side = Some(user.side);
                    self.battle.side_mut(user.side).now.damage += loss;
                    let mon = self.battle.mon_mut(w);
                    mon.biggest_hit = mon.biggest_hit.max(loss);
                }
            }
        }
        self.battle.ctx.last_hp = Some(link);
    }

    /// Throat Chop's hit silences sound moves for this turn and the next, without a row; Sheer Force on
    /// the user, Shield Dust or a Covert Cloak on the target stop it.
    fn throat_chop(&mut self, user: Who, target: Who) {
        let b = &self.battle;
        let ab = |w: Who| b.mon(w).ability.map(|a| a.id());
        if ab(user) == Some("sheerforce") || ab(target) == Some("shielddust") || matches!(b.mon(target).item, ItemState::Held(i) if i.id() == "covertcloak") {
            return;
        }
        let turn = self.battle.turn;
        let Some(c) = Cond::from_id("throatchop") else { return };
        let Some(a) = self.active_mut(target) else { return };
        if a.out {
            return;
        }
        // a second hit while it holds does not restart it
        if a.vols.iter().flatten().any(|v| v.cond == c) {
            return;
        }
        if let Some(i) = a.vols.iter().position(|v| v.is_none()) {
            a.vols[i] = Some(Vol { cond: c, start: turn, detail: VolDetail::None, by: None, orphan: false, maybe_gone: false, ends: 0, hit: 0 });
        }
    }

    fn cause(&mut self, effect: &str, who: Option<&Mon>, of: Option<&Mon>, prev: u8) {
        let named = who.and_then(|m| self.find(m, false));
        self.battle.ctx.cause_who = named;
        self.battle.ctx.undone = None;
        if residual_cause(effect) {
            self.battle.ctx.residual = true;
        }
        // a partial trap's damage: its check at the end of this turn is done
        if let Some(w) = named {
            let turn = self.battle.turn;
            if partial_trap_move(effect) {
                if let Some(a) = self.active_mut(w) {
                    for v in a.vols.iter_mut().flatten() {
                        if v.cond == conds::PARTIALLYTRAPPED {
                            v.hit = turn;
                        }
                    }
                }
            }
        }
        // a cause row right after an HP row takes that change off its move, until an HP row of the
        // same Pokémon right after shows the cause was that row's
        if prev == K_HP {
            if let Some(l) = self.battle.ctx.last_hp {
                if (named.is_none() || named == Some(l.who)) && l.move_side.is_some() {
                    let s = l.move_side.unwrap();
                    self.battle.side_mut(s).now.damage -= l.loss;
                    self.battle.mon_mut(l.who).biggest_hit = l.prev_biggest;
                    self.battle.ctx.last_hp = Some(HpLink { move_side: None, ..l });
                    self.battle.ctx.undone = Some(l);
                }
            }
        }
        // an item or ability named as the cause is revealed on its holder: `of` when given, else the Pokémon
        let holder = match (of, who) {
            (Some(o), _) => self.find(o, false),
            (None, Some(m)) => self.find(m, false),
            _ => None,
        };
        // Healing Wish also cures, without a row
        if effect == "healingwish" {
            if let Some(w) = named {
                self.battle.mon_mut(w).status = Status::None;
            }
        }
        if let Some(h) = holder {
            if let Some(it) = ItemId::from_id(effect).filter(|i| i.0 != 0) {
                self.reveal_item(h, ItemState::Held(it));
            } else if let Some(ab) = AbilityId::from_id(effect).filter(|a| a.0 != 0) {
                self.reveal_ability(h, ab);
            }
        }
    }

    fn faint(&mut self, m: &Mon) {
        let Some(w) = self.find(m, false) else { return };
        let b = &mut self.battle;
        let mon = b.mon_mut(w);
        if mon.fainted {
            self.anomaly(format!("{} {} fainted twice", m.side.id(), m.mon));
        }
        let mon = self.battle.mon_mut(w);
        mon.fainted = true;
        mon.hp = 0.0;
        mon.hp_estimated = false;
        // an ability changed on the field (Trace, Skill Swap) is the brought one again
        mon.ability = mon.base_ability;
        if let Some(a) = self.active_mut(w) {
            a.out = true;
            a.lock = Lock::None;
        }
        self.drop_linked(w);
    }

    /// Traps tied to the Pokémon that set them (Mean Look, Octolock, a trapping move) end without a row when it leaves.
    fn drop_linked(&mut self, gone: Who) {
        let turn = self.battle.turn;
        let a = &mut self.battle.sides[gone.side.other().idx()].active;
        for v in a.vols.iter_mut().flatten() {
            if v.cond == conds::PARTIALLYTRAPPED && v.by == Some(gone) && !v.orphan {
                // it ends at the next end of turn that checks it: this one, unless its damage was already
                // done this turn (its user fainted later in the end of the turn)
                v.orphan = true;
                v.ends = if v.hit == turn { turn.saturating_add(1) } else { turn };
            }
        }
        let before = a.vols;
        for v in a.vols.iter_mut() {
            // a partial trap ends with its own row at the end of the turn
            if v.map_or(false, |x| x.by == Some(gone) && (x.cond == conds::TRAPPED || x.cond == conds::ATTRACT || x.cond.id() == "octolock")) {
                *v = None;
            }
        }
        if a.vols != before {
            let mut packed = [None; MAX_VOLS];
            for (i, v) in a.vols.iter().flatten().enumerate() {
                packed[i] = Some(*v);
            }
            a.vols = packed;
        }
    }

    fn status(&mut self, m: &Mon, status: &str) {
        let Some(w) = self.find(m, false) else { return };
        let Some(s) = status_of(status) else {
            self.anomaly(format!("unknown status {}", status));
            return;
        };
        let turn = self.battle.turn;
        let mon = self.battle.mon_mut(w);
        mon.status = s;
        mon.status_start = turn;
        mon.status_count = 0;
        // Yawn stays to its end (the end of the turn after it began) even if the Pokémon fell asleep
        // otherwise; `turn` removes it
    }

    fn cure(&mut self, m: &Mon, status: &str, item: Option<&str>) {
        let Some(w) = self.find(m, false) else { return };
        if status_of(status).is_none() && status != "confusion" {
            self.anomaly(format!("unknown status {}", status));
        }
        if status == "confusion" {
            self.remove_vol(w, conds::CONFUSION);
        } else {
            if self.battle.mon(w).status.id() != status {
                self.anomaly(format!("{} {} cure of {} that was not held", w.side.id(), m.mon, status));
            }
            self.battle.mon_mut(w).status = Status::None;
            self.battle.mon_mut(w).status_count = 0;
        }
        if let Some(x) = item {
            match ItemId::from_id(x).filter(|i| i.0 != 0) {
                Some(it) if self.eaten == Some((w, it)) => {}
                Some(it) => {
                    let st = if items::spent_when_activated(it) { ItemState::Gone(Some(it)) } else { ItemState::Held(it) };
                    self.reveal_item(w, st);
                }
                None => self.anomaly(format!("unknown item {}", x)),
            }
        }
    }

    fn vol_cond(&mut self, effect: &str) -> Option<(Cond, Option<MoveId>)> {
        if let Some(c) = Cond::from_id(effect).filter(|c| !c.is_none()) {
            return Some((c, None));
        }
        if let Some(mv) = MoveId::from_id(effect).filter(|m| !m.is_none()) {
            let d = mv.data();
            if !d.volatile_status.is_none() {
                return Some((d.volatile_status, Some(mv)));
            }
            if let Some(c) = d.secondaries.iter().map(|s| s.volatile_status).find(|c| !c.is_none()) {
                return Some((c, Some(mv)));
            }
        }
        self.anomaly(format!("unknown condition {}", effect));
        None
    }

    fn volatile(&mut self, m: &Mon, effect: &str, detail: Option<&Detail>) {
        let Some(w) = self.find_active(m) else { return };
        // "The power of X's T-type moves rose!": Charge and the abilities that set it; the type is not a move
        let (effect, detail) = match (effect, detail) {
            ("type_power_rose", Some(Detail::Id(t))) if t == "fire" => ("flashfire", None),
            ("type_power_rose", _) => ("charge", None),
            x => x,
        };
        let Some((cond, from_move)) = self.vol_cond(effect) else { return };
        let turn = self.battle.turn;
        let mut d = match detail {
            None => VolDetail::None,
            Some(Detail::Num(n)) => VolDetail::Num((*n).clamp(i16::MIN as i64, i16::MAX as i64) as i16),
            Some(Detail::Id(x)) => match MoveId::from_id(x).filter(|m| !m.is_none()) {
                Some(mv) => VolDetail::Move(mv),
                None => {
                    self.anomaly(format!("unknown detail {}", x));
                    VolDetail::None
                }
            },
        };
        if d == VolDetail::None {
            if let Some(mv) = from_move {
                d = VolDetail::Move(mv);
            }
        }
        let foe = self.battle.active(w.side.other());
        let self_made = self.battle.ctx.last_move.map_or(false, |(u, m, _)| u == w && m.id() == "noretreat");
        let lm_last = self.battle.ctx.last_move.map(|(u, m, _)| (u, m));
        // a Substitute frees its user from a partial trap, without a row
        if cond == conds::SUBSTITUTE {
            self.remove_vol(w, conds::PARTIALLYTRAPPED);
        }
        let Some(a) = self.active_mut(w) else { return };
        // Encore and Disable name the last move used when the row does not
        if d == VolDetail::None && (cond == conds::ENCORE || cond == conds::DISABLE) {
            if let Some(lm) = a.last_move {
                d = VolDetail::Move(lm);
            }
        }
        // fatigue: a confusion that starts right after a locked move ends the lock (not one another
        // Pokémon's move caused)
        if cond == conds::CONFUSION {
            if let Lock::Locked { last, mv: lm, .. } = a.lock {
                if last == turn && lm_last == Some((w, lm)) {
                    a.lock = Lock::None;
                }
            }
        }
        // the trapper: the opposing Pokémon, unless the Pokémon trapped itself (No Retreat)
        let by = if (cond == conds::PARTIALLYTRAPPED || cond == conds::TRAPPED || cond == conds::ATTRACT || cond.id() == "octolock") && !self_made { foe } else { None };
        if let Some(v) = a.vols.iter_mut().flatten().find(|v| v.cond == cond) {
            v.detail = if d == VolDetail::None { v.detail } else { d };
            return;
        }
        match a.vols.iter().position(|v| v.is_none()) {
            Some(i) => a.vols[i] = Some(Vol { cond, start: turn, detail: d, by, orphan: false, maybe_gone: false, ends: 0, hit: 0 }),
            None => self.anomaly("too many volatile conditions"),
        }
    }

    fn remove_vol(&mut self, w: Who, cond: Cond) -> bool {
        let Some(a) = self.active_mut(w) else { return false };
        let mut found = false;
        for v in a.vols.iter_mut() {
            if v.map_or(false, |x| x.cond == cond) {
                *v = None;
                found = true;
            }
        }
        // keep the slots packed
        let mut packed = [None; MAX_VOLS];
        for (i, v) in a.vols.iter().flatten().enumerate() {
            packed[i] = Some(*v);
        }
        a.vols = packed;
        if cond.id() == "uproar" {
            a.lock = Lock::None;
        }
        found
    }

    fn volatile_end(&mut self, m: &Mon, effect: &str) {
        if effect == "illusion" {
            // the Pokémon on the field was the named one all along: what was shown under the disguise is its (5.14)
            let Some(w) = self.find(m, true) else { return };
            let s = self.battle.side_mut(w.side);
            if s.active.slot != Some(w.slot) {
                let shown = s.active.slot;
                s.active.slot = Some(w.slot);
                s.active.out = false;
                let mut real = s.team[w.slot as usize];
                let before = self.battle.ctx.entering[w.side.idx()];
                let s = self.battle.side_mut(w.side);
                if let Some(d) = shown {
                    let fake = s.team[d as usize];
                    // what the disguise was before it "entered": everything shown since is the real one's
                    let was = before.filter(|(slot, _)| *slot == d).map(|(_, m)| m);
                    real.hp = fake.hp;
                    real.status = fake.status;
                    real.status_start = fake.status_start;
                    real.status_count = fake.status_count;
                    let old_moves = was.map_or([None; 4], |m| m.moves);
                    for (k, mv) in fake.moves.iter().enumerate() {
                        if let Some(mv) = mv {
                            if !old_moves.contains(&Some(*mv)) && !real.moves.contains(&Some(*mv)) {
                                if let Some(j) = real.moves.iter().position(|x| x.is_none()) {
                                    real.moves[j] = Some(*mv);
                                    real.pp_used[j] = fake.pp_used[k].saturating_sub(was.map_or(0, |m| m.pp_used[k]));
                                }
                            }
                        }
                    }
                    if was.map_or(true, |m| m.item != fake.item) {
                        real.item = fake.item;
                    }
                    if was.map_or(true, |m| m.ability != fake.ability) && fake.ability.is_some() {
                        real.ability = fake.ability;
                        real.base_ability = real.base_ability.or(fake.ability);
                    }
                    real.biggest_hit = fake.biggest_hit;
                    real.was_lead = fake.was_lead && fake.entries <= 1;
                    real.entries = real.entries.saturating_add(1);
                    // the disguise is as it was before (never on the field, unless it had entered before)
                    let f = &mut s.team[d as usize];
                    match was {
                        Some(m) => *f = m,
                        None => {
                            let keep = (f.species, f.preview);
                            *f = MonState { species: keep.0, preview: keep.1, hp: 100.0, ..MonState::default() };
                        }
                    }
                    if f.entries == 0 && f.species.is_some() && f.preview != f.species {
                        f.species = None;
                    }
                }
                real.seen = true;
                if real.form.is_none() {
                    real.form = real.species;
                }
                s.team[w.slot as usize] = real;
            }
            return;
        }
        let Some(w) = self.find_active(m) else { return };
        let Some((cond, _)) = self.vol_cond(effect) else { return };
        if !self.remove_vol(w, cond) {
            self.anomaly(format!("{} ended but was not on {}", effect, m.mon));
        }
        // a Substitute broken by a move: the move went on, so its charge (if any) was skipped
        if cond == conds::SUBSTITUTE {
            if let Some((u, ..)) = self.battle.ctx.last_move.filter(|(u, ..)| u.side != w.side && !self.battle.ctx.delayed) {
                self.released(u);
            }
        }
    }

    fn boost(&mut self, m: &Mon, stat: &str, by: i32) {
        let Some(w) = self.find_active(m) else { return };
        let Some(i) = boost_index(stat) else {
            self.anomaly(format!("unknown stat {}", stat));
            return;
        };
        if let Some(a) = self.active_mut(w) {
            a.boosts[i] = (a.boosts[i] as i32 + by).clamp(-6, 6) as i8;
        }
    }

    /// Rule 4: the stat is at its limit; the attempt belongs to the most recent move or ability.
    fn boost_limit(&mut self, m: &Mon, stat: &str, dir: LimitDir) {
        let Some(w) = self.find_active(m) else { return };
        let Some(i) = boost_index(stat) else {
            self.anomaly(format!("unknown stat {}", stat));
            return;
        };
        if self.battle.ctx.last_actor.is_none() {
            self.anomaly("stat limit row with no move or ability before it");
        }
        if let Some(a) = self.active_mut(w) {
            a.boosts[i] = if dir == LimitDir::Up { 6 } else { -6 };
        }
    }

    fn boost_reset(&mut self, who: Option<&Mon>, what: ResetWhat, of: Option<&Mon>, stats: Option<&[String]>) {
        let targets: Vec<Who> = match who {
            Some(m) => self.find_active(m).into_iter().collect(),
            None => [Side::Own, Side::Opp].iter().filter_map(|&s| self.battle.active(s)).collect(),
        };
        let src = match of {
            Some(o) => self.find_active(o).and_then(|w| self.active_mut(w).map(|a| a.boosts)),
            None => None,
        };
        if matches!(what, ResetWhat::Copy | ResetWhat::Swap) && src.is_none() {
            self.anomaly("boost copy or swap with no source");
            return;
        }
        // the stats a swap exchanges: those named, else all
        let mut which = [true; N_BOOSTS];
        if let Some(st) = stats {
            which = [false; N_BOOSTS];
            for x in st {
                match boost_index(x) {
                    Some(i) => which[i] = true,
                    None => self.anomaly(format!("unknown stat {}", x)),
                }
            }
        }
        for w in targets.iter().copied() {
            let mine = self.active_mut(w).map(|a| a.boosts);
            if let Some(a) = self.active_mut(w) {
                match what {
                    ResetWhat::All => a.boosts = [0; N_BOOSTS],
                    ResetWhat::Negative => a.boosts.iter_mut().for_each(|b| *b = (*b).max(0)),
                    ResetWhat::Positive => a.boosts.iter_mut().for_each(|b| *b = (*b).min(0)),
                    ResetWhat::Invert => a.boosts.iter_mut().for_each(|b| *b = -*b),
                    ResetWhat::Copy => a.boosts = src.unwrap_or_default(),
                    ResetWhat::Swap => {
                        let o = src.unwrap_or_default();
                        for i in 0..N_BOOSTS {
                            if which[i] {
                                a.boosts[i] = o[i];
                            }
                        }
                    }
                }
            }
            // a swap changes the other Pokemon's stages too
            if what == ResetWhat::Swap {
                if let (Some(o), Some(mine)) = (of.and_then(|o| self.lookup(o)), mine) {
                    if let Some(b) = self.active_mut(o) {
                        for i in 0..N_BOOSTS {
                            if which[i] {
                                b.boosts[i] = mine[i];
                            }
                        }
                    }
                }
            }
        }
    }

    fn blocked(&mut self, effect: &str, who: Option<&Mon>) {
        // a blocking ability belongs to the Pokémon the row names (Clear Body, Inner Focus, ...)
        if let (Some(ab), Some(m)) = (AbilityId::from_id(effect).filter(|a| a.0 != 0), who) {
            if let Some(w) = self.find(m, false) {
                self.reveal_ability(w, ab);
            }
        }
    }

    fn reveal_ability(&mut self, w: Who, ab: AbilityId) {
        let mon = self.battle.mon_mut(w);
        if w.side == Side::Own {
            // unknown after Skill Swap or Transform with an ability never shown: learnt now
            if mon.ability.is_some() && mon.ability != Some(ab) {
                self.anomaly(format!("own ability shown as {}", ab.id()));
            }
            self.battle.mon_mut(w).ability = Some(ab);
            return;
        }
        if mon.base_ability.is_none() && mon.ability.is_none() {
            mon.base_ability = Some(ab);
        }
        mon.ability = Some(ab);
    }

    fn ability(&mut self, m: &Mon, ability: &str) {
        let Some(w) = self.find(m, false) else { return };
        // the move's target taking it in (Sap Sipper, Volt Absorb), or the user's Mega Sol standing in for
        // sun: the move went on, so its charge (if any) was skipped
        if let Some((u, mv, t)) = self.battle.ctx.last_move.filter(|_| !self.battle.ctx.delayed) {
            if (t == Some(w) && u.side != w.side && ABSORBS.contains(&ability)) || (u == w && ability == "megasol") {
                self.released(u);
            }
            // the target's ability took the move in (Dry Skin, Volt Absorb, Flash Fire, ...): it did not
            // connect, so it sets up no recharge and no lock
            if t == Some(w) && u.side != w.side && ABSORBS.contains(&ability) {
                let a = &mut self.battle.sides[u.side.idx()].active;
                if a.slot == Some(u.slot) {
                    match a.lock {
                        Lock::Recharge { .. } => a.lock = Lock::None,
                        Lock::Locked { mv: m, .. } if m == mv => a.lock = Lock::None,
                        _ => {}
                    }
                }
            }
        }
        self.battle.ctx.last_actor = Some(w);
        match AbilityId::from_id(ability).filter(|a| a.0 != 0) {
            Some(ab) => {
                self.reveal_ability(w, ab);
                self.battle.ctx.last_ability = Some((w, ab));
            }
            None => self.anomaly(format!("unknown ability {}", ability)),
        }
    }

    fn ability_change(&mut self, m: &Mon, from: &str, to: &str) {
        let Some(w) = self.find(m, false) else { return };
        self.battle.ctx.last_actor = Some(w);
        let (Some(f), Some(t)) = (AbilityId::from_id(from), AbilityId::from_id(to)) else {
            self.anomaly(format!("unknown ability {} or {}", from, to));
            return;
        };
        let mon = self.battle.mon_mut(w);
        if w.side == Side::Opp && mon.base_ability.is_none() {
            mon.base_ability = Some(f);
        }
        // our ability is unknown after copying one never shown (Transform, Skill Swap): the row shows it
        if w.side == Side::Own && mon.ability.is_some() && mon.ability != Some(f) {
            self.anomaly(format!("own ability changed from {} it did not have", from));
        }
        self.battle.mon_mut(w).ability = Some(t);
        self.ability_gone(w, t);
    }

    /// What an ability started ends with it, without a row, when another takes its place (Flash Fire's boost).
    fn ability_gone(&mut self, w: Who, now: AbilityId) {
        if now.id() != "flashfire" {
            if let Some(c) = Cond::from_id("flashfire") {
                self.remove_vol(w, c);
            }
        }
    }

    fn reveal_item(&mut self, w: Who, st: ItemState) {
        if w.side == Side::Own {
            let known = match self.battle.mon(w).item {
                ItemState::Held(i) => Some(i),
                _ => None,
            };
            let shown = match st {
                ItemState::Held(i) | ItemState::Gone(Some(i)) => Some(i),
                _ => None,
            };
            if known.is_some() && shown.is_some() && known != shown {
                self.anomaly(format!("own item shown as {}", shown.unwrap().id()));
            }
        }
        let mon = self.battle.mon_mut(w);
        let lost_choice = matches!(mon.item, ItemState::Held(i) if i.data().is_choice) && !matches!(st, ItemState::Held(i) if i.data().is_choice);
        mon.item = st;
        if lost_choice {
            if let Some(a) = self.active_mut(w) {
                a.choice_lock = None;
            }
        }
    }

    fn item(&mut self, m: Option<&Mon>, item: &str, what: ItemWhat, of: Option<&Mon>) {
        // "X stole and ate its target's ITEM!" (Bug Bite, Pluck) names only the eater, as `of`: the Berry was
        // the other active Pokémon's
        if let (None, ItemWhat::Removed, Some(e)) = (m, what, of.and_then(|o| self.lookup(o))) {
            let Some(it) = ItemId::from_id(item).filter(|i| i.0 != 0) else {
                self.anomaly(format!("unknown item {}", item));
                return;
            };
            match self.battle.active(e.side.other()) {
                Some(h) => self.reveal_item(h, ItemState::Gone(Some(it))),
                None => self.anomaly("eaten item with no Pokémon to take it from"),
            }
            self.eaten = Some((e, it));
            return;
        }
        let w = match m {
            Some(m) => self.find(m, false),
            // a Gem's row names no Pokémon: the user of the move above (5.1)
            None => {
                let u = self.battle.ctx.last_move.map(|(u, ..)| u);
                if u.is_none() {
                    self.anomaly("nameless row with no move before it");
                }
                u
            }
        };
        let Some(w) = w else { return };
        let Some(it) = ItemId::from_id(item).filter(|i| i.0 != 0) else {
            self.anomaly(format!("unknown item {}", item));
            return;
        };
        match what {
            ItemWhat::Activated => {
                if matches!(it.id(), "leftovers" | "blacksludge") {
                    self.battle.ctx.residual = true;
                }
                // the Berry just eaten acting on its eater: not the eater's own item
                if self.eaten == Some((w, it)) {
                    return;
                }
                // another Pokémon's item acting on this one (a flung or stolen Berry eaten): it is gone
                if let Some(o) = of.and_then(|o| self.lookup(o)).filter(|o| *o != w) {
                    if !matches!(self.battle.mon(o).item, ItemState::Gone(_)) {
                        self.reveal_item(o, ItemState::Gone(Some(it)));
                    }
                    return;
                }
                let spent = items::spent_when_activated(it);
                self.reveal_item(w, if spent { ItemState::Gone(Some(it)) } else { ItemState::Held(it) });
                if it.id() == "leppaberry" {
                    self.leppa(w);
                }
            }
            ItemWhat::Used => {
                self.reveal_item(w, ItemState::Gone(Some(it)));
                if matches!(it.id(), "ejectbutton" | "ejectpack") && self.is_active(w) {
                    self.leave(w);
                    self.battle.ctx.withdrawn[w.side.idx()] = Some((w.slot, false));
                }
            }
            ItemWhat::Removed => self.reveal_item(w, ItemState::Gone(Some(it))),
            ItemWhat::Received | ItemWhat::Found => {
                // a received item replaces what was held without a check against the old one
                if what == ItemWhat::Received {
                    // the giver has no item now, unless it is the one that just received in this exchange
                    // (Trick, Switcheroo: two rows); a theft (Thief, Covet, Pickpocket, Magician) takes it
                    let swap = matches!(self.battle.ctx.last_move, Some((_, m, _)) if matches!(m.id(), "trick" | "switcheroo")) && self.battle.ctx.prev != K_ABILITY;
                    if let Some(g) = of.and_then(|o| self.lookup(o)) {
                        if g != w && !(swap && self.battle.ctx.received == Some(g)) {
                            self.battle.mon_mut(g).item = ItemState::Gone(None);
                        }
                    }
                    self.battle.ctx.received = Some(w);
                    self.battle.mon_mut(w).item = ItemState::Unknown;
                    // a new item starts fresh: no Choice lock carries over (choice items' onStart)
                    if let Some(a) = self.active_mut(w) {
                        a.choice_lock = None;
                    }
                }
                self.reveal_item(w, ItemState::Held(it))
            }
        }
    }

    /// Leppa Berry: 10 PP back to the first move with none left, else the first move short of its maximum.
    fn leppa(&mut self, w: Who) {
        let left = self.battle.pp_left(w);
        let i = left.iter().position(|x| *x == Some(0)).or_else(|| {
            let mon = self.battle.mon(w);
            (0..4).find(|&i| mon.pp_used[i] > 0)
        });
        if let Some(i) = i {
            let mon = self.battle.mon_mut(w);
            mon.pp_used[i] = mon.pp_used[i].saturating_sub(10);
        }
    }

    fn mega_stone(&mut self, m: &Mon, stone: &str) {
        let Some(w) = self.find_active(m) else { return };
        match ItemId::from_id(stone).filter(|i| i.0 != 0) {
            Some(it) => {
                self.reveal_item(w, ItemState::Held(it));
                self.battle.ctx.last_stone = Some((w, it));
            }
            None => self.anomaly(format!("unknown item {}", stone)),
        }
    }

    fn mega(&mut self, m: &Mon, form: Option<&str>) {
        let Some(w) = self.find_active(m) else { return };
        let base = self.battle.mon(w).form.or(self.battle.mon(w).species);
        let f = match form {
            Some(x) => match SpeciesId::from_id(x) {
                Some(f) => Some(f),
                None => {
                    self.anomaly(format!("unknown form {}", x));
                    return;
                }
            },
            // 5.13: the stone of the mega_stone row above, else the held item (our set), else the only Mega
            None => {
                let stone = match (self.battle.ctx.last_stone, self.battle.mon(w).item) {
                    (Some((sw, it)), _) if sw == w => Some(it),
                    (_, ItemState::Held(it)) => Some(it),
                    _ => None,
                };
                let by_stone = stone.zip(base).and_then(|(it, b)| items::mega_form(it, b));
                by_stone.or_else(|| {
                    let b = base?;
                    let mut all = ITEMS.iter().flat_map(|i| i.mega_stone.iter()).filter(|(x, _)| *x == b).map(|(_, m)| *m);
                    match (all.next(), all.next()) {
                        (Some(m), None) => Some(m),
                        _ => None,
                    }
                })
            }
        };
        let Some(f) = f else {
            self.anomaly(format!("Mega form of {} not known", m.mon));
            self.battle.side_mut(w.side).mega_used = true;
            self.battle.mon_mut(w).mega = true;
            return;
        };
        if self.battle.side(w.side).mega_used {
            self.anomaly(format!("{} side Mega Evolved twice", w.side.id()));
        }
        if !matches!(self.battle.mon(w).item, ItemState::Held(_)) {
            if let Some(stone) = base.and_then(|b| items::mega_stone_for(b, f)) {
                self.reveal_item(w, ItemState::Held(stone));
            }
        }
        self.battle.side_mut(w.side).mega_used = true;
        let mon = self.battle.mon_mut(w);
        mon.mega = true;
        mon.form = Some(f);
        // a Mega form has one ability, which stays with it
        let ab = f.data().abilities;
        if ab[0].0 != 0 && ab[1..].iter().all(|a| a.0 == 0) {
            mon.ability = Some(ab[0]);
            mon.base_ability = Some(ab[0]);
            self.ability_gone(w, ab[0]);
        }
    }

    fn form(&mut self, m: &Mon, form: &str) {
        let Some(w) = self.find(m, false) else { return };
        match SpeciesId::from_id(form) {
            Some(f) => self.battle.mon_mut(w).form = Some(f),
            None => self.anomaly(format!("unknown form {}", form)),
        }
    }

    fn transform(&mut self, m: &Mon, into: &str) {
        let Some(w) = self.find_active(m) else { return };
        let Some(sp) = SpeciesId::from_id(into) else {
            self.anomaly(format!("unknown species {}", into));
            return;
        };
        // Transform copies the target's moves (5 PP each): ours are known, the opponent's as far as seen
        let foe = self.battle.active(w.side.other());
        let moves = match foe {
            Some(f) => self.battle.mon(f).moves,
            None => [None; 4],
        };
        if let Some(a) = self.active_mut(w) {
            a.transformed = Some(sp);
            a.t_moves = moves;
            a.t_pp_used = [0; 4];
        }
        // and its ability, as far as it is known
        if let Some(f) = foe {
            let ab = self.battle.mon(f).ability;
            self.battle.mon_mut(w).ability = ab;
        }
        if let Some(f) = foe {
            let boosts = self.battle.side(f.side).active.boosts;
            if let Some(a) = self.active_mut(w) {
                a.boosts = boosts;
            }
        }
    }

    fn types(&mut self, m: &Mon, ty: Option<&str>, lose: Option<&str>, add: Option<&str>) {
        let Some(w) = self.find_active(m) else { return };
        // Forest's Curse, Trick-or-Treat: a third type while it stays in
        if let Some(x) = add {
            match type_from_id(x) {
                Some(t) => {
                    if let Some(a) = self.active_mut(w) {
                        a.added_type = Some(t);
                    }
                }
                None => self.anomaly(format!("unknown type {}", x)),
            }
            return;
        }
        if let Some(ty) = ty {
            match type_from_id(ty) {
                Some(t) => {
                    if let Some(a) = self.active_mut(w) {
                        a.types = Some([t, Type::Typeless]);
                        a.added_type = None;
                    }
                }
                None => self.anomaly(format!("unknown type {}", ty)),
            }
        }
        // Burn Up, Double Shock: the type is gone; a Pokémon with no type left is typeless
        if let Some(l) = lose {
            let Some(t) = type_from_id(l) else {
                self.anomaly(format!("unknown type {}", l));
                return;
            };
            let cur = self.battle.types(w).unwrap_or([Type::Typeless; 2]);
            let rest: Vec<Type> = cur.iter().copied().filter(|x| *x != t && *x != Type::Typeless).collect();
            let new = [rest.first().copied().unwrap_or(Type::Typeless), rest.get(1).copied().unwrap_or(Type::Typeless)];
            if let Some(a) = self.active_mut(w) {
                a.types = Some(new);
            }
        }
        if ty.is_none() && lose.is_none() {
            self.anomaly("types row with no type");
        }
    }

    fn weather(&mut self, id: &str, what: StartEnd) {
        let c = eff_cond(id).filter(|c| c.data().kind & ckind::WEATHER != 0);
        let Some(c) = c else {
            self.anomaly(format!("unknown weather {}", id));
            return;
        };
        let turn = self.effect_start();
        let by = self.battle.ctx.last_actor;
        let f = &mut self.battle.field;
        match what {
            StartEnd::Start => f.weather = Some(Timed { cond: c, start: turn, by }),
            StartEnd::End => {
                self.battle.ctx.residual = true;
                let f = &mut self.battle.field;
                if f.weather.map(|x| x.cond) == Some(c) {
                    f.weather = None;
                } else {
                    f.weather = None;
                    self.anomaly(format!("{} ended but was not up", id));
                }
            }
        }
    }

    fn field(&mut self, id: &str, what: StartEnd) {
        let Some(c) = eff_cond(id).filter(|c| !c.is_none()) else {
            self.anomaly(format!("unknown field effect {}", id));
            return;
        };
        let turn = self.effect_start();
        let by = self.battle.ctx.last_actor;
        let t = Timed { cond: c, start: turn, by };
        let is_terrain = c.data().kind & ckind::TERRAIN != 0;
        let room = ROOMS.iter().position(|r| *r == c.id());
        let f = &mut self.battle.field;
        let slot = if is_terrain {
            &mut f.terrain
        } else if let Some(r) = room {
            &mut f.rooms[r]
        } else {
            self.anomaly(format!("unknown field effect {}", id));
            return;
        };
        match what {
            StartEnd::Start => *slot = Some(t),
            StartEnd::End => {
                let was = slot.map(|x| x.cond) == Some(c);
                *slot = None;
                if !was {
                    self.anomaly(format!("{} ended but was not up", id));
                }
            }
        }
    }

    fn side_cond(&mut self, side: Side, id: &str, what: SideWhat, mon: Option<&str>) {
        if let Some(x) = mon {
            self.find_active(&Mon::new(side, x));
        }
        let id = match id {
            "stealthrocks" | "pointedstones" => "stealthrock",
            x => x,
        };
        let Some(i) = SIDE_CONDS.iter().position(|s| *s == id) else {
            // a known side condition the Battle does not track (Mist, Lucky Chant, Wide Guard, ...)
            if Cond::from_id(id).map_or(true, |c| c.is_none()) {
                self.anomaly(format!("unknown side effect {}", id));
            }
            return;
        };
        let turn = self.effect_start();
        let by = self.battle.ctx.last_actor;
        let max_layers = match id {
            "spikes" => 3,
            "toxicspikes" => 2,
            _ => 1,
        };
        let slot = &mut self.battle.side_mut(side).conds[i];
        match what {
            SideWhat::Start => match slot {
                Some(c) if max_layers > 1 => c.layers = (c.layers + 1).min(max_layers),
                _ => *slot = Some(SideCond { start: turn, layers: 1, by }),
            },
            SideWhat::End => {
                let was = slot.is_some();
                *slot = None;
                if !was {
                    self.anomaly(format!("{} ended but was not up", id));
                }
            }
            SideWhat::Acted => {
                if slot.is_none() {
                    self.anomaly(format!("{} acted but was not up", id));
                }
            }
        }
    }

    /// A one-off announcement. A Future Sight landing (`futuresight`, naming the target) is the move the
    /// nameless rows after it belong to; its use (`futuresight_start`, naming the user) follows the user's
    /// own move row and changes nothing (Skill Swap's abilities come as `ability_change` rows).
    fn once(&mut self, effect: &str, who: Option<&Mon>) {
        // Trick, Switcheroo ("switched items with its target!"): both gave their item; a `received` row
        // follows for each one that got one (rule 23)
        if effect == "trick" || effect == "switcheroo" {
            let user = who.and_then(|m| self.lookup(m)).or(self.battle.ctx.last_move.map(|(u, ..)| u));
            if let Some(u) = user {
                let t = self.battle.active(u.side.other());
                for w in [Some(u), t].into_iter().flatten() {
                    self.reveal_item(w, ItemState::Gone(None));
                }
                self.battle.ctx.received = None;
            }
        }
        if effect == "futuresight" {
            // it lands at the end of the turn, and is the cause of the HP row after it
            self.battle.ctx.residual = true;
            if let Some(t) = who.and_then(|m| self.lookup(m)) {
                let user = self.battle.active(t.side.other()).unwrap_or(t);
                if let Some(mv) = MoveId::from_id("futuresight") {
                    self.battle.ctx.last_move = Some((user, mv, Some(t)));
                    self.battle.ctx.delayed = true;
                }
            }
        }
    }

    /// "X lost N PP from MOVE!" (rule 6)
    fn pp(&mut self, m: &Mon, mv: &str, lost: u32) {
        let Some(w) = self.find(m, false) else { return };
        let Some(id) = MoveId::from_id(mv).filter(|x| !x.is_none()) else {
            self.anomaly(format!("unknown move {}", mv));
            return;
        };
        match self.slot_of(w, id, true) {
            Some(i) => self.spend_pp(w, i, lost.min(255) as u8),
            None => self.anomaly(format!("pp row for {} which is not among its moves", mv)),
        }
    }

    /// The moves the action screen showed. Used for our own transformed Pokemon, whose copied moves the
    /// log does not list.
    fn screen_offer(&mut self, so: &crate::json::Json) {
        let a = self.battle.sides[0].active;
        if a.transformed.is_none() || a.out {
            return;
        }
        // the copied ability, which the log does not name
        if let (Some(ab), Some(slot)) = (so.get("ability").and_then(|x| x.as_str()).and_then(AbilityId::from_id), a.slot) {
            self.battle.sides[0].team[slot as usize].ability = Some(ab);
        }
        let Some(list) = so.get("moves").and_then(|x| x.as_arr()) else { return };
        let mut moves = [None; 4];
        for (i, x) in list.iter().take(4).enumerate() {
            moves[i] = x.as_str().and_then(MoveId::from_id).filter(|m| !m.is_none());
        }
        // in the screen's order, with the uses counted so far
        let mut used = [0u8; 4];
        for i in 0..4 {
            if let Some(m) = moves[i] {
                if let Some(j) = a.t_moves.iter().position(|x| *x == Some(m)) {
                    used[i] = a.t_pp_used[j];
                }
            }
        }
        let a = &mut self.battle.sides[0].active;
        a.t_moves = moves;
        a.t_pp_used = used;
    }

    fn revive(&mut self, m: &Mon) {
        let Some(w) = self.find(m, false) else { return };
        let max = self.battle.sets[w.slot as usize].stats[0];
        if !self.battle.mon(w).fainted {
            self.anomaly(format!("{} revived but had not fainted", m.mon));
        }
        let mon = self.battle.mon_mut(w);
        mon.fainted = false;
        mon.status = Status::None;
        // Revival Blessing restores half of max HP
        mon.hp = if w.side == Side::Own && max > 0 { (max / 2) as f32 * 100.0 / max as f32 } else { 50.0 };
    }
}
