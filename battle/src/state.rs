//! The Battle: what one player knows (PROJECT.md "Terms"). A plain copyable value. Unknown is `None` or
//! `ItemState::Unknown`, never a default that could be mistaken for a fact.

use crate::event::{Eff, Side};
use cg_engine::sim::data::*;

/// Volatile slots kept for each active Pokémon.
pub const MAX_VOLS: usize = 10;
/// atk, def, spa, spd, spe, accuracy, evasion (BOOST_IDS order)
pub const N_BOOSTS: usize = 7;
/// Turn number before the first `turn` event (leads, entry abilities, weather set at the start).
pub const START_TURN: u16 = 0;

/// A Pokémon by side and team index (own: index into `battle_start.own_team`; opp: preview position).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Who {
    pub side: Side,
    pub slot: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ItemState {
    #[default]
    Unknown,
    /// held and known (own from the set; opponent's once shown)
    Held(ItemId),
    /// no item now; `Some` names the item that was used up or removed when it is known
    Gone(Option<ItemId>),
}

/// Own set from `battle_start`, as ids.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct OwnSet {
    pub species: SpeciesId,
    pub item: Option<ItemId>,
    pub ability: AbilityId,
    pub moves: [Option<MoveId>; 4],
    pub nature: Option<u8>,
    pub points: Option<[u8; 6]>,
    /// hp, atk, def, spa, spd, spe as the game shows them
    pub stats: [u16; 6],
}

/// One Pokémon. Own Pokémon have every field filled from the set; the opponent's only what was shown.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct MonState {
    /// species id as brought (the name the contract uses); opponent: known once seen or matched to preview
    pub species: Option<SpeciesId>,
    /// species shown at team preview (opponent only)
    pub preview: Option<SpeciesId>,
    /// current form (Mega, Disguise busted, ...); None while unseen
    pub form: Option<SpeciesId>,
    /// has been on the field
    pub seen: bool,
    /// percent of max HP, 0..=100
    pub hp: f32,
    pub fainted: bool,
    pub status: Status,
    /// turn the status began
    pub status_start: u16,
    /// sleep: `cant slp` rows seen; tox: poison damage rows since entering
    pub status_count: u8,
    pub item: ItemState,
    /// current ability, when known
    pub ability: Option<AbilityId>,
    /// the ability it was brought with, when known (own: the set's; opponent: first shown, or `from` of a change)
    pub base_ability: Option<AbilityId>,
    /// own: the set's moves; opponent: moves seen, in order of first use
    pub moves: [Option<MoveId>; 4],
    /// PP spent per move slot, counted from `move` events
    pub pp_used: [u8; 4],
    pub mega: bool,
    /// times it entered the field
    pub entries: u8,
    pub was_lead: bool,
    /// largest single HP loss caused by an opposing move, percent
    pub biggest_hit: f32,
    /// a locked move went on while confused: a silent end of the lock may have been missed (PP)
    pub pp_uncertain: bool,
    /// `hp` is the Battle's estimate after a silent heal (the opponent's Regenerator), until an HP row
    pub hp_estimated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VolDetail {
    #[default]
    None,
    Move(MoveId),
    Num(i16),
}

/// A volatile condition on an active Pokémon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vol {
    pub cond: Cond,
    /// turn it began (the Battle counts turns from the start row)
    pub start: u16,
    pub detail: VolDetail,
    /// the opposing active when it began (partial traps and Octolock last while it stays in)
    pub by: Option<Who>,
    /// a partial trap whose user has left the field since: it ends at the end of the turn
    pub orphan: bool,
    /// an orphaned partial trap ends silently at the end of this turn
    pub ends: u16,
    /// a partial trap's damage row last came on this turn
    pub hit: u16,
    /// it may have ended without a row (an opposing Charge when its holder could not move: the move it
    /// could not use is not shown)
    pub maybe_gone: bool,
}

/// A move that removes all choice for the next turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lock {
    #[default]
    None,
    /// Outrage-like moves and Uproar: `uses` so far, `last` turn used
    Locked { mv: MoveId, uses: u8, last: u16 },
    /// charging turn of a two-turn move; the move may be unknown until it is released
    Charging { mv: Option<MoveId>, turn: u16, pp_counted: bool },
    /// must recharge after a move used on `turn`
    Recharge { turn: u16 },
}

/// The state of the Pokémon on the field for one side. Cleared when it leaves.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Active {
    /// team index of the Pokémon on the field (or the one that just left or fainted, until replaced)
    pub slot: Option<u8>,
    /// withdrawn or fainted and not yet replaced
    pub out: bool,
    pub boosts: [i8; N_BOOSTS],
    pub vols: [Option<Vol>; MAX_VOLS],
    /// set by a `types` row
    pub types: Option<[Type; 2]>,
    /// a third type added by a `types` row with `add`
    pub added_type: Option<Type>,
    pub entered: u16,
    /// move actions since entering (`move` and `cant` rows; Fake Out and First Impression)
    pub move_actions: u8,
    /// last move it used (Torment, Encore, Disable, Gigaton Hammer)
    pub last_move: Option<MoveId>,
    /// move a Choice item or Gorilla Tactics locked it into
    pub choice_lock: Option<MoveId>,
    pub lock: Lock,
    /// species it transformed into
    pub transformed: Option<SpeciesId>,
    /// moves while transformed (own: the target's moves as far as known; opponent: our moves)
    pub t_moves: [Option<MoveId>; 4],
    pub t_pp_used: [u8; 4],
    /// the game refused a switch while this opposing Pokémon was on the field: it traps by something
    /// not shown (Shadow Tag, Arena Trap, Magnet Pull)
    pub trapped_by: Option<Who>,
}

/// A weather, terrain or room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timed {
    pub cond: Cond,
    pub start: u16,
    /// who set it (rule 3), when an `ability` or `move` row came before it
    pub by: Option<Who>,
}

/// Side conditions the Battle tracks, in this order.
pub const SIDE_CONDS: [&str; 10] =
    ["reflect", "lightscreen", "auroraveil", "tailwind", "safeguard", "spikes", "toxicspikes", "stealthrock", "stickyweb", "quickguard"];
/// Rooms and other field effects, in this order.
pub const ROOMS: [&str; 5] = ["trickroom", "magicroom", "wonderroom", "gravity", "fairylock"];
pub const R_MAGICROOM: usize = 1;
pub const R_GRAVITY: usize = 3;
pub const R_FAIRYLOCK: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideCond {
    pub start: u16,
    pub layers: u8,
    pub by: Option<Who>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Act {
    #[default]
    None,
    Move,
    Switch,
    /// a move that switched the user out (U-turn, Parting Shot, ...)
    Pivot,
    Cant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CantReason {
    Status(Status),
    Cond(Cond),
    Move(MoveId),
    Other,
}

/// What one side did in one turn.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TurnRecord {
    pub act: Act,
    /// the Pokémon that acted
    pub mon: Option<u8>,
    pub mv: Option<MoveId>,
    /// move called by it (Sleep Talk, Copycat)
    pub called: Option<MoveId>,
    /// 1 when this side acted first in the turn, 2 second, 0 not at all
    pub order: u8,
    pub eff: Option<Eff>,
    pub hits: u8,
    pub failed: bool,
    pub missed: bool,
    pub protected: bool,
    /// HP percent the move took from the opposing Pokémon (rule 2)
    pub damage: f32,
    /// switched to (Switch, Pivot) or dragged in
    pub to: Option<u8>,
    pub dragged: bool,
    pub cant: Option<CantReason>,
    /// hurt itself in its confusion
    pub confusion_hit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SideState {
    pub team: [MonState; 6],
    pub active: Active,
    pub mega_used: bool,
    pub conds: [Option<SideCond>; SIDE_CONDS.len()],
    /// what this side did last completed turn, and in the turn under way
    pub last: TurnRecord,
    pub now: TurnRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Loss,
    Tie,
    Forfeit,
    Unknown,
}

impl Outcome {
    pub fn from_id(s: &str) -> Option<Outcome> {
        Some(match s {
            "win" => Outcome::Win,
            "loss" => Outcome::Loss,
            "tie" => Outcome::Tie,
            "forfeit" => Outcome::Forfeit,
            "unknown" => Outcome::Unknown,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Field {
    pub weather: Option<Timed>,
    pub terrain: Option<Timed>,
    pub rooms: [Option<Timed>; ROOMS.len()],
}

/// Linking state for the rules of LIVE.md section 5. Part of the Battle so a Battle can resume from itself.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Ctx {
    /// most recent `move` this turn: user, move, the Pokémon it was used on
    pub last_move: Option<(Who, MoveId, Option<Who>)>,
    /// most recent `move` or `ability` (rules 3 and 4)
    pub last_actor: Option<Who>,
    /// kind of the previous event (`kind_code`)
    pub prev: u8,
    /// last HP change: who, loss, side of the move it was put on, values before
    pub last_hp: Option<HpLink>,
    /// the HP change a cause row took off its move, kept until the row after shows whose it was
    pub undone: Option<HpLink>,
    /// the Pokémon the last cause row named
    pub cause_who: Option<Who>,
    /// a `withdraw` waiting for its `enter`: slot and pivot flag
    pub withdrawn: [Option<(u8, bool)>; 2],
    /// state passed by Baton Pass (true) or Shed Tail (false) to the next Pokémon in
    pub passed: [Option<(Active, bool)>; 2],
    /// sides that acted so far this turn
    pub acted: u8,
    /// the last Pokémon a row named (a nameless confusion self-hit is its)
    pub last_named: Option<Who>,
    /// the last mega_stone row: holder and stone (5.13)
    pub last_stone: Option<(Who, ItemId)>,
    /// the last `ability` row: holder and ability (a Magic Bounce or Dancer row before the move it uses)
    pub last_ability: Option<(Who, AbilityId)>,
    /// the Pokémon of the last `received` item row (an exchange writes two rows)
    pub received: Option<Who>,
    /// this turn's end has passed: a fainted Pokemon was replaced
    pub post_residual: bool,
    /// per side, the Pokémon that entered last and what the Battle held for it before (Illusion)
    pub entering: [Option<(u8, MonState)>; 2],
    /// `last_move` is a delayed attack landing (Future Sight): its user did not move now
    pub delayed: bool,
    /// a row only the end of the turn writes was seen this turn (residual damage or healing, an end row)
    pub residual: bool,
    /// the move of our own last order (CG's answer; `Input::Order`)
    pub own_choice: Option<MoveId>,
    /// the team slot our last switch order this turn named (a replace request before the rows that show it
    /// replaces that Pokémon)
    pub own_switch: Option<u8>,
    /// no log row since our last answer to a `turn` request: a `replace` request now comes before the rows
    /// of its turn (LV5Q-7)
    pub unread: bool,
    /// while `unread`: the team slot our answer to the open replace request names (None for a revival)
    pub replace_answer: Option<u8>,
    /// while `unread`: the team slot an earlier replacement of this turn brought in, the one on the field
    /// at a new replace request
    pub sent_in: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HpLink {
    pub who: Who,
    pub loss: f32,
    pub move_side: Option<Side>,
    pub prev_biggest: f32,
}

/// Action count: 4 moves, 4 moves with Mega Evolution, 3 switches.
pub const N_ACTIONS: usize = 11;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Battle {
    pub started: bool,
    pub turn: u16,
    pub sets: [OwnSet; 6],
    pub n_own: u8,
    /// team indices brought, lead first
    pub picks: [Option<u8>; 3],
    /// index 0 own, 1 opponent
    pub sides: [SideState; 2],
    pub field: Field,
    /// actions the game refused for the current request (cleared by `turn` and by our own `enter`)
    pub refused: [bool; N_ACTIONS],
    pub unread: u16,
    /// `gap` events: rows not read; the Battle is incomplete from the first one
    pub gaps: u16,
    pub incomplete: bool,
    pub anomalies: u16,
    pub result: Option<Outcome>,
    pub ctx: Ctx,
    /// running counters for the observation
    pub track: crate::track::Track,
}

impl Battle {
    pub fn side(&self, s: Side) -> &SideState {
        &self.sides[s.idx()]
    }
    pub fn side_mut(&mut self, s: Side) -> &mut SideState {
        &mut self.sides[s.idx()]
    }
    pub fn mon(&self, w: Who) -> &MonState {
        &self.sides[w.side.idx()].team[w.slot as usize]
    }
    pub fn mon_mut(&mut self, w: Who) -> &mut MonState {
        &mut self.sides[w.side.idx()].team[w.slot as usize]
    }
    /// The Pokémon on the field for a side (None before the lead, or after it left and before the replacement).
    pub fn active(&self, s: Side) -> Option<Who> {
        let a = &self.side(s).active;
        match a.slot {
            Some(slot) if !a.out => Some(Who { side: s, slot }),
            _ => None,
        }
    }
    /// Current types of an active Pokémon: a `types` row, else its form's types.
    pub fn types(&self, w: Who) -> Option<[Type; 2]> {
        let a = &self.side(w.side).active;
        if a.slot == Some(w.slot) {
            if let Some(t) = a.types {
                return Some(t);
            }
            if let Some(sp) = a.transformed {
                return Some(sp.data().types);
            }
        }
        self.mon(w).form.map(|f| f.data().types)
    }
    /// Turns since a field effect began, at the current turn.
    pub fn elapsed(&self, start: u16) -> u16 {
        self.turn.saturating_sub(start)
    }
    /// PP left per move slot of a Pokémon (own: exact if no row was lost; opponent: of seen moves).
    pub fn pp_left(&self, w: Who) -> [Option<u8>; 4] {
        let m = self.mon(w);
        let a = &self.side(w.side).active;
        let (moves, used, transformed) = if a.slot == Some(w.slot) && !a.out && a.transformed.is_some() {
            (a.t_moves, a.t_pp_used, true)
        } else {
            (m.moves, m.pp_used, false)
        };
        let mut out = [None; 4];
        for i in 0..4 {
            if let Some(mv) = moves[i] {
                // Transform copies a move with 5 PP, or 1 when its maximum is 1
                let max = if transformed { if mv.data().max_pp == 1 { 1 } else { 5 } } else { mv.data().max_pp };
                out[i] = Some(max.saturating_sub(used[i]));
            }
        }
        out
    }
}
