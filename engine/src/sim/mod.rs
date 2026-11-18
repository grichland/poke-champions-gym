//! The new turn resolver (PROJECT.md, step 8). One battle is one `State` value; a step plays one outcome
//! of every random event, drawn through an `Outcomes` source (seeded, forced from a tape, or recording),
//! in Showdown's order of steps. Data comes from tables generated from Showdown (`data`).
//!
//! Stage 1: action order, switches, plain attacks with exact damage, PP and Struggle, fainting and
//! replacements, the win rule. Stage 2: statuses, volatile conditions, stat stages, moves with their own
//! rules, the end-of-turn sequence (with No Ability, no items and no field effects). Stage 3: weather,
//! terrain, rooms and Gravity, side conditions, hazards, slot conditions (`field`). Stage 4: abilities
//! (`abilities`), through the same handler lists as conditions, and choices the game offers but refuses.
//! Stage 5: items (`items`) at sub-order 8 after the ability, Mega Evolution, the item-dependent moves.
//! Condition and ability handlers run through Showdown's event order (`events`, `conds`, `field`,
//! `abilities`); moves' own handlers are in `movefx`. Effects whose handlers have no implementation are
//! recorded in `State::unimpl` at the hook points of `hooks`.

pub mod abilities;
pub mod battle;
pub mod calc;
pub mod conds;
pub mod data;
pub mod events;
pub mod field;
pub mod hooks;
pub mod items;
pub mod log;
pub mod movefx;
pub mod moves;
pub mod options;
pub mod outcome;
pub mod residual;
pub mod state;
pub mod status;

pub use log::Row;
pub use options::{Choice, Kind, MonSpec, SideOptions};
pub use outcome::{Forced, Outcomes, Recording, Seeded};
pub use state::State;
