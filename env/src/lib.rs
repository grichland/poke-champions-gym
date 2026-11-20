//! CG environment core: every outcome of a step with its probability (`enumerate`), copyable games with
//! their outcome source (`game`), each seat's view of a game (`views`), and a batched strict turn-taking environment (`batch`) fed from a team
//! pool (`teams`). `builtin` names every built-in Player, the searching `lookahead` among them.

pub mod batch;
pub mod builtin;
pub mod enumerate;
pub mod from_battle;
pub mod game;
pub mod hidden;
pub mod labels;
pub mod lookahead;
pub mod observe;
pub mod pool;
pub mod teams;
pub mod views;

pub use batch::{BatchEnv, BattleRule, Counters, Rule, EnvConfig, GameResult, LegalRule, Pending, RequestKind, SlotSave, StateRule};
pub use enumerate::{enumerate_start, enumerate_step, enumerate_with, sample_step, EnumOptions, Enumeration, Outcome, Replay, Sampler};
pub use game::{tape_of, Game};
pub use labels::{Label, N_OPP_ACTIONS};
pub use observe::{ObsBatch, ObsOptions, N_ALL_ACTIONS, PREVIEW0};
pub use teams::{TeamMode, TeamPool};
pub use views::{Source, Tracked, Views, KIND_NONE};
