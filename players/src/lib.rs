//! CG's scripted Players (PROJECT.md G5, D4, D7): random, max damage and tower, and ports of the older
//! Python players (best-matchup max damage, disciplined, endgame, the four strategy pilots). Each reads a
//! Battle only (what one side knows) and chooses inside `battle::legal`. Rules follow
//! `docs/investigation/findings/players-spec.md` (ids MD-n, T-n, PS-n) and each module's header.
//!
//! Actions are the Battle's layout: 0..=3 moves, 4..=7 the same moves with Mega Evolution, 8..=10 switches
//! to `picks[0..=2]`. Team preview returns three indices into the own team, lead first.

pub mod best_matchup;
pub mod chain;
pub mod disciplined;
pub mod endgame;
pub mod estimate;
pub mod fired;
pub mod gen;
pub mod lists;
pub mod matchup;
pub mod max_damage;
pub mod pilots;
pub mod random;
pub mod rng;
pub mod threat;
pub mod tower;
pub mod usable;
pub mod view;

use battle::{Battle, ReqKind};

pub use estimate::{estimate, Estimator, Hit, ROLL_MAX, ROLL_MID, ROLL_MIN};
pub use best_matchup::BestMatchupPlayer;
pub use disciplined::DisciplinedPlayer;
pub use endgame::EndgamePlayer;
pub use max_damage::MaxDamagePlayer;
pub use pilots::PilotPlayer;
pub use random::RandomPlayer;
pub use rng::Rng;
pub use tower::TowerPlayer;
pub use view::{Fighter, FieldView};

/// Something that chooses for one side from what that side knows.
pub trait Player: Send {
    fn name(&self) -> &'static str;
    /// Three indices into the own team (`battle_start.own_team`), lead first.
    fn teampreview(&mut self, b: &Battle, rng: &mut Rng) -> [u8; 3];
    /// An action index inside `battle::legal(b, kind)`. Returns 0 only when nothing is legal.
    fn choose_move(&mut self, b: &Battle, kind: ReqKind, rng: &mut Rng) -> usize;
    /// Rule names and how often each fired since the Player was made (empty for players without them).
    fn fired(&self) -> Vec<(&'static str, u64)> {
        Vec::new()
    }
}

/// Decisions on the spec's open questions; each can be flipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// PS-Q3 / PS-B1: take the Mega variant of a move whenever Mega Evolution is offered
    pub mega_when_offered: bool,
    /// PS-Q4 / PS-B4: drop a status move the Battle shows cannot work
    pub skip_status_that_cannot_work: bool,
    /// PS-B6: drop a move whose volatile the target already has (confusion, attraction, Leech Seed, ...)
    pub skip_repeat_volatile: bool,
    /// PS-Q2: with no revealed damaging move the foe cannot KO us, so setup counts as safe
    pub setup_safe_without_revealed_attacks: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { mega_when_offered: true, skip_status_that_cannot_work: true, skip_repeat_volatile: true, setup_safe_without_revealed_attacks: true }
    }
}

/// Every built-in Player's name, in this order.
pub const NAMES: [&str; 10] = [
    "random",
    "max_damage",
    "tower",
    "best_matchup_max_damage",
    "disciplined",
    "endgame",
    "pilot_encore",
    "pilot_weather",
    "pilot_hazard",
    "pilot_priority",
];

/// Every Player by name, for runners.
pub fn by_name(name: &str) -> Option<Box<dyn Player>> {
    use pilots::Kind;
    Some(match name {
        "random" => Box::new(RandomPlayer::default()),
        "max_damage" => Box::new(MaxDamagePlayer::default()),
        "tower" => Box::new(TowerPlayer::default()),
        "best_matchup_max_damage" => Box::new(BestMatchupPlayer::default()),
        "disciplined" => Box::new(DisciplinedPlayer::default()),
        "endgame" => Box::new(EndgamePlayer::default()),
        "pilot_encore" => Box::new(PilotPlayer::of(Kind::Encore)),
        "pilot_weather" => Box::new(PilotPlayer::of(Kind::Weather)),
        "pilot_hazard" => Box::new(PilotPlayer::of(Kind::Hazard)),
        "pilot_priority" => Box::new(PilotPlayer::of(Kind::Priority)),
        _ => return None,
    })
}

/// Lowest legal action in a range, if any.
pub(crate) fn first_legal(mask: &[bool; battle::N_ACTIONS], range: std::ops::Range<usize>) -> Option<usize> {
    range.into_iter().find(|&i| mask[i])
}
