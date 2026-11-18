//! The hidden-information check on whole games (the runner's counter, N2): a finished game is replayed
//! with what neither seat may see changed — the three Pokémon each side left behind (moves, item,
//! ability, nature, points), the moves a brought Pokémon never used, and a brought Pokémon's ability (an
//! unrevealed ability, or one later changed by Skill Swap, Trace, Mega Evolution) — with the same choices
//! and seed. Every changed id is fresh (no Pokémon of the game holds it), so the twin's resolver rows and
//! each seat's record map back to the original by renaming those ids. Up to the first step whose rows
//! differ after renaming (the changed thing acted), each seat must be shown exactly the same messages: a
//! difference there is something a seat was shown that only the hidden information decided.

use crate::game::Game;
use crate::teams::{mix, Team};
use battle::{Body, Input, Order, ReqKind};
use emit::check::Tally;
use emit::Seats;
use cg_engine::sim::data::*;
use cg_engine::sim::state::{State, SIDES, TEAM};
use cg_engine::sim::{Choice, Row};
use std::collections::HashMap;

/// A call made on a game's `Seats`, in order (the env logs them for the replay).
#[derive(Clone, Debug)]
pub enum Call {
    Picks(usize, [u8; TEAM]),
    /// the leads' entry with the rows it wrote
    Leads(Vec<Row>),
    Request(usize, ReqKind, Option<Order>),
    Order(usize, Order),
    /// `State::refuse` with these choices (the State learns, nothing is shown)
    Refuse([Option<Choice>; SIDES]),
    /// a choice refused, with each side's active team slot (for the moves chosen)
    Chose([Option<Choice>; SIDES], [u8; SIDES]),
    Refused(usize, Order),
    /// one step with these choices, each side's active team slot before it, and the rows it wrote
    Step([Option<Choice>; SIDES], [u8; SIDES], Vec<Row>),
}

/// Abilities with no effect in singles: a brought Pokémon's ability is changed to one of these.
const INERT_ABILITIES: [&str; 14] = [
    "ballfetch", "honeygather", "illuminate", "propellertail", "stalwart", "telepathy", "minus", "plus", "friendguard", "healer", "battery",
    "powerspot", "steelyspirit", "curiousmedicine",
];

/// Moves whose use draws from or reads another Pokémon's moves: games with one keep their moves.
const READS_MOVES: [&str; 8] = ["metronome", "assist", "copycat", "sleeptalk", "mefirst", "mirrormove", "naturepower", "mimic"];

fn tokens_map(s: &str, map: &HashMap<&'static str, &'static str>) -> String {
    if map.is_empty() {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut word = String::new();
    let flush = |w: &mut String, out: &mut String| {
        if !w.is_empty() {
            out.push_str(map.get(w.as_str()).copied().unwrap_or(w.as_str()));
            w.clear();
        }
    };
    for ch in s.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}

fn input_text(i: &Input) -> String {
    match i {
        Input::Msg(m) => m.to_json().to_string(),
        Input::Picks(p) => p.to_json().to_string(),
        Input::Order(o) => o.to_json().to_string(),
    }
}

/// The twin's teams and the renaming from its fresh ids back to the originals.
fn twin_teams(rosters: &[Team; 2], picks: [[u8; TEAM]; SIDES], fin: &State, log: &[Call], change_abilities: bool) -> ([Team; 2], HashMap<&'static str, &'static str>) {
    let mut tw = *rosters;
    let mut map: HashMap<&'static str, &'static str> = HashMap::new();
    let all = rosters.iter().flat_map(|t| t.iter());
    let used_ab: Vec<AbilityId> = all.clone().map(|m| m.ability).collect();
    let used_mv: Vec<MoveId> = all.clone().flat_map(|m| m.moves[..m.nmoves as usize].to_vec()).collect();
    let illusion = all.clone().any(|m| m.ability == abilities::ILLUSION);
    let keep_moves = used_mv.iter().any(|m| READS_MOVES.contains(&m.id())) || illusion;
    let mut fresh_ab = INERT_ABILITIES.iter().filter_map(|x| AbilityId::from_id(x)).filter(|a| a.0 != 0 && !used_ab.contains(a));
    let mut fresh_mv = (1..MOVES.len() as u16).map(MoveId).filter(|m| !used_mv.contains(m) && m.id() != "struggle" && !READS_MOVES.contains(&m.id()));
    let other_item = |it: ItemId| ITEMS.iter().position(|d| d.mega_stone.is_empty() && !d.id.is_empty() && d.id != it.id()).map(|i| ItemId(i as u16)).unwrap_or(it);
    for side in 0..SIDES {
        for i in 0..6 {
            let m = &mut tw[side][i];
            if !picks[side].contains(&(i as u8)) {
                // never on the field: everything but the species
                for k in 0..m.nmoves as usize {
                    if let Some(f) = fresh_mv.next() {
                        m.moves[k] = f;
                    }
                }
                m.item = other_item(m.item);
                m.ability = abilities::NOABILITY;
                m.nature = (m.nature + 7) % 25;
                m.evs = [m.evs[5], m.evs[4], m.evs[3], m.evs[2], m.evs[1], m.evs[0]];
                continue;
            }
            if change_abilities && m.ability != abilities::ILLUSION {
                if let Some(a) = fresh_ab.next() {
                    map.insert(a.id(), m.ability.id());
                    m.ability = a;
                }
            }
            if keep_moves {
                continue;
            }
            // moves it never chose and never used: full PP at the end, never transformed
            let Some(slot) = fin.sides[side].mons.iter().position(|x| x.set_species == m.species) else { continue };
            let fm = &fin.sides[side].mons[slot];
            if fm.transformed {
                continue;
            }
            let chosen = |k: usize| {
                log.iter().any(|c| match c {
                    Call::Step(ch, act, _) | Call::Chose(ch, act) => act[side] as usize == slot && matches!(ch[side], Some(Choice::Move { index, .. }) if index as usize == k),
                    _ => false,
                })
            };
            for k in 0..m.nmoves as usize {
                if chosen(k) {
                    continue;
                }
                let bm = fm.moves[..fm.nmoves as usize].iter().find(|x| x.id == m.moves[k]);
                if bm.map_or(false, |x| x.pp == x.max_pp) {
                    if let Some(f) = fresh_mv.next() {
                        map.insert(f.id(), m.moves[k].id());
                        m.moves[k] = f;
                    }
                }
            }
        }
    }
    (tw, map)
}

/// Replays the game with hidden information changed (the brought Pokémon's abilities too when
/// `change_abilities`); returns the steps compared. Differences go to `t`.
#[allow(clippy::too_many_arguments)]
pub fn twin_check(
    rosters: &[Team; 2],
    picks: [[u8; TEAM]; SIDES],
    episode_seed: u64,
    tag: &str,
    log: &[Call],
    base: &[Vec<Input>; SIDES],
    fin: &State,
    change_abilities: bool,
    replace_before_rows: bool,
    ctx: &str,
    t: &mut Tally,
) -> u64 {
    let (tw, map) = twin_teams(rosters, picks, fin, log, change_abilities);
    let mut seats = Seats::new(tag.to_string(), true);
    seats.replace_before_rows = replace_before_rows;
    seats.start([&tw[0], &tw[1]]);
    // the battle_start messages differ in each seat's own team: compared from the next message on
    let mut cur = [1usize; SIDES];
    let mut game: Option<Game> = None;
    let mut rows = Vec::with_capacity(64);
    let mut steps = 0;
    let debug = std::env::var("CG_HIDDEN_DEBUG").is_ok();
    let same_rows = |a: &[Row], b: &[Row]| {
        let ok = a.len() == b.len() && a.iter().zip(b).all(|(x, y)| emit::fmt::show(x) == tokens_map(&emit::fmt::show(y), &map));
        if !ok && debug {
            eprintln!("HIDDEN diverged {}: base {:?} twin {:?}", ctx, a.iter().map(emit::fmt::show).collect::<Vec<_>>(), b.iter().map(|y| tokens_map(&emit::fmt::show(y), &map)).collect::<Vec<_>>());
        }
        ok
    };
    for call in log {
        match call {
            Call::Picks(seat, p) => seats.picks(*seat, *p),
            Call::Leads(base_rows) => {
                rows.clear();
                let Ok(g) = Game::start_logged([&tw[0], &tw[1]], picks, mix(episode_seed, 1), &mut rows) else { return steps };
                if !same_rows(base_rows, &rows) {
                    return steps;
                }
                seats.leads(&rows, &g.state);
                game = Some(g);
            }
            Call::Request(seat, kind, refused) => {
                let Some(g) = &game else { return steps };
                let offer = if *kind == ReqKind::Turn { emit::screen_offer(&g.state, *seat) } else { None };
                seats.request(*seat, *kind, refused.clone(), offer);
            }
            Call::Order(seat, o) => seats.order(*seat, o.clone()),
            Call::Refuse(ch) => {
                let Some(g) = &mut game else { return steps };
                g.state.refuse(*ch);
            }
            Call::Refused(seat, o) => seats.refused(*seat, o.clone()),
            Call::Chose(..) => {}
            Call::Step(ch, _, base_rows) => {
                let Some(g) = &mut game else { return steps };
                let Game { state, rng } = g;
                if seats.step_into(state, rng, *ch, &mut rows).is_err() || !same_rows(base_rows, &rows) {
                    return steps;
                }
                steps += 1;
            }
        }
        for seat in 0..SIDES {
            let tr = &seats.records[seat];
            for k in cur[seat]..tr.len() {
                let bt = base[seat].get(k).map(input_text);
                let tt = tokens_map(&input_text(&tr[k]), &map);
                if bt.as_deref() != Some(tt.as_str()) {
                    let kind = match &tr[k] {
                        Input::Msg(m) => match &m.body {
                            Body::Events { .. } => "events",
                            Body::DecisionRequest { .. } => "request",
                            _ => "message",
                        },
                        _ => "input",
                    };
                    t.add(&format!("hidden:{}", kind), || format!("{} seat {} message {}: shown {} twin {}", ctx, seat, k, bt.unwrap_or_default(), tt));
                    return steps;
                }
            }
            cur[seat] = tr.len();
        }
    }
    steps
}
