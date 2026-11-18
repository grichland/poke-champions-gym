//! CG's Battle: what one player knows of a battle, built only from the events of contracts/LIVE.md.
//! The same builder runs in training (events emitted by the engine) and live (events from the phone).

pub mod builder;
pub mod event;
pub mod items;
pub mod json;
pub mod legal;
pub mod live;
pub mod state;
pub mod track;

pub use builder::{Anomaly, Builder, Input};
pub use event::*;
pub use legal::{legal, order_for, Forced, Legal};
pub use state::*;
