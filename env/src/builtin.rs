//! Every built-in Player by name: the scripted ones of `players` and the ones that search on a State
//! built from their Battle (`lookahead`), which live here because they need the environment.

use players::Player;

/// Every built-in name, in this order (`players::NAMES` first). A lookahead name may carry options:
/// `lookahead@depth=2,samples=4,replies=2,seed=7,fill=1` (see `lookahead::Config`; not `:`, which a live
/// Session reads as module:attr).
pub const NAMES: [&str; 12] = [
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
    "lookahead",
    "lookahead2",
];

/// A built-in Player by name.
pub fn by_name(name: &str) -> Option<Box<dyn Player>> {
    players::by_name(name).or_else(|| crate::lookahead::LookaheadPlayer::by_name(name).map(|p| Box::new(p) as Box<dyn Player>))
}
