//! A game: the State plus its outcome source's state. Copying it (`snapshot`, `fork`) gives an independent
//! continuation. Named `Game`, not Battle: in CG a Battle is one player's view (PROJECT.md, Terms).

use crate::enumerate::{enumerate_step, EnumOptions, Enumeration, Replay};
use cg_engine::sim::outcome::{effect_string, index_string, Event, Forced, Seeded, TapeEvent, TapeValue, Value};
use cg_engine::sim::state::{State, SIDES, TEAM};
use cg_engine::sim::{Choice, MonSpec};

#[derive(Clone, Debug)]
pub struct Game {
    pub state: State,
    pub rng: Seeded,
}

impl Game {
    /// A game at the first decision of turn 1; team preview ties are drawn from `seed`'s source.
    pub fn start(teams: [&[MonSpec]; SIDES], picks: [[u8; TEAM]; SIDES], seed: u64) -> Result<Game, String> {
        let mut rng = Seeded::new(seed);
        let state = State::start(teams, picks, &mut rng)?;
        Ok(Game { state, rng })
    }

    /// `start`, with the rows of the leads' entry recorded into `rows`.
    pub fn start_logged(teams: [&[MonSpec]; SIDES], picks: [[u8; TEAM]; SIDES], seed: u64, rows: &mut Vec<cg_engine::sim::Row>) -> Result<Game, String> {
        let mut rng = Seeded::new(seed);
        let state = State::start_logged(teams, picks, &mut rng, rows)?;
        Ok(Game { state, rng })
    }

    /// One step with the game's own seeded source.
    pub fn step(&mut self, choices: [Option<Choice>; SIDES]) -> Result<(), String> {
        self.state.step(choices, &mut self.rng)
    }

    /// An exact copy: the same seed state, so the same continuation.
    pub fn snapshot(&self) -> Game {
        self.clone()
    }

    /// A copy with its own seed: the same State, an independent continuation.
    pub fn fork(&self, seed: u64) -> Game {
        Game { state: self.state, rng: Seeded::new(seed) }
    }

    /// One step with every random event forced from `path` (an enumerated outcome's path). The seeded source
    /// is not used. Errors when the path does not fit the step.
    pub fn step_forced(&mut self, choices: [Option<Choice>; SIDES], path: &[u16]) -> Result<(), String> {
        let mut r = Replay::new(path.to_vec(), false);
        let mut s = self.state;
        s.step(choices, &mut r)?;
        if let Some(e) = r.bad {
            return Err(e);
        }
        if !r.consumed() || r.path.len() != path.len() {
            return Err(format!("path of {} events, step drew {}", path.len(), r.path.len()));
        }
        self.state = s;
        Ok(())
    }

    /// One step forced from a contract tape (outcome.rs `Forced`), e.g. `tape_of(&outcome.events)`.
    pub fn step_tape(&mut self, choices: [Option<Choice>; SIDES], tape: Vec<TapeEvent>) -> Result<(), String> {
        let mut f = Forced::new(tape);
        let mut s = self.state;
        s.step(choices, &mut f)?;
        f.finish()?;
        self.state = s;
        Ok(())
    }

    /// Every outcome of the next step with these choices.
    pub fn outcomes(&self, choices: [Option<Choice>; SIDES], opts: &EnumOptions) -> Result<Enumeration, String> {
        enumerate_step(&self.state, choices, opts)
    }
}

/// Recorded events as a contract tape for `Forced`.
pub fn tape_of(events: &[Event]) -> Vec<TapeEvent> {
    events
        .iter()
        .map(|e| TapeEvent {
            key: e.site.key,
            side: e.site.side,
            mv: e.site.mv.map(|m| m.id().to_string()).or(e.site.mv_name.map(String::from)),
            hit: e.site.hit.map(|h| h as i64),
            index: index_string(&e.site.index),
            effect: effect_string(&e.site.effect),
            value: match e.value {
                Value::Bool(b) => TapeValue::Bool(b),
                Value::Int(i) => TapeValue::Int(i as i64),
                Value::Side(s) => TapeValue::Str(if s == 0 { "p1" } else { "p2" }.into()),
                Value::Effect(x) => TapeValue::Str(x.id().into()),
                Value::Str(s) => TapeValue::Str(s.into()),
            },
        })
        .collect()
}
