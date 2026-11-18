//! CG environment core: every outcome of a step with its probability (`enumerate`), copyable games with
//! their outcome source (`game`), and a batched strict turn-taking environment (`batch`) fed from a team
//! pool (`teams`).

pub mod batch;
pub mod enumerate;
pub mod game;
pub mod hidden;
pub mod labels;
pub mod observe;
pub mod pool;
pub mod teams;

pub use batch::{BatchEnv, BattleRule, Counters, Rule, EnvConfig, GameResult, LegalRule, Pending, RequestKind, StateRule};
pub use enumerate::{enumerate_start, enumerate_step, enumerate_with, EnumOptions, Enumeration, Outcome, Replay};
pub use game::{tape_of, Game};
pub use labels::{Label, N_OPP_ACTIONS};
pub use observe::{ObsBatch, ObsOptions, N_ALL_ACTIONS, PREVIEW0};
pub use teams::{TeamMode, TeamPool};
