//! Emission and the Battles it builds, on the real teams with every rule: (a) a seat's Battle agrees with
//! the State at its decisions, (c) what a seat has not seen of the opponent changes nothing it is shown,
//! (d) a Battle rebuilt from its record equals the one built as the game went. The runner (`players_run`)
//! counts the same on larger runs.

mod common;

use battle::{Builder, Event, ReqKind};
use emit::check::{self, Tally};
use emit::Seats;
use env::teams::{distinct, TeamMode};
use env::{BatchEnv, EnvConfig, Game, RequestKind};
use players::{by_name, Rng};
use cg_engine::sim::data::*;
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::SIDES;
use cg_engine::sim::{Choice, Kind, MonSpec};
use std::sync::Arc;

/// Plays `games` games of two named Players through the environment with every check on.
fn play(a: &str, b: &str, games: u64, seed: u64) -> env::Counters {
    let pool = Arc::new(common::pool());
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 1000, checks: true, keep_records: true, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(pool, cfg);
    let mut ps = [by_name(a).unwrap(), by_name(b).unwrap()];
    let mut rng = [Rng::new(seed), Rng::new(seed ^ 1)];
    let mut legal = Vec::new();
    while env.games_done < games {
        let p = env.pending(0);
        let seat = p.seat as usize;
        let bt = *env.battle(0, seat);
        let act = match p.kind {
            RequestKind::Preview => env::batch::preview_action(ps[seat].teampreview(&bt, &mut rng[seat])).unwrap(),
            k => ps[seat].choose_move(&bt, if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn }, &mut rng[seat]) as u16,
        };
        env.legal_actions(0, &mut legal);
        let act = if legal.contains(&act) { act } else { legal[0] };
        env.act(&[act]).unwrap();
    }
    env.counters
}

#[test]
fn battles_rebuild_from_their_records() {
    let c = play("random", "max_damage", 150, 3);
    assert!(c.steps > 1000, "steps {}", c.steps);
    assert_eq!(c.rebuild_equal, c.rebuilt, "rebuilt Battles differ");
    assert_eq!(c.rebuilt, 300);
    assert_eq!(c.errors, 0);
}

#[test]
fn battles_agree_with_the_state() {
    let c = play("tower", "max_damage", 300, 4);
    assert_eq!(c.agree.unexplained(), 0, "disagreements {:?}", c.agree.examples);
    assert!(c.legal_equal * 1000 >= c.checked * 999, "legal sets agree in {} of {}: {:?}", c.legal_equal, c.checked, c.legal.examples);
    assert_eq!(c.anomalies.total(), 0, "{:?}", c.anomalies.examples);
}

/// Moves of seat 0 that show or act on what the opponent holds or knows: games with them are left out.
fn reveals(m: MoveId) -> bool {
    matches!(
        m.id(),
        "sleeptalk" | "assist" | "copycat" | "metronome" | "mefirst" | "mirrormove" | "transform" | "mimic" | "sketch" | "imprison" | "knockoff"
            | "trick" | "switcheroo" | "thief" | "covet" | "poltergeist" | "fling" | "bugbite" | "pluck" | "incinerate" | "corrosivegas"
            | "skillswap" | "roleplay" | "entrainment" | "gastroacid" | "worryseed" | "simplebeam" | "doodle" | "instruct" | "encore" | "disable"
            | "spite" | "eeriespell" | "grudge"
    )
}

/// Abilities that show or copy what the other side holds or knows.
fn reveals_ab(a: AbilityId) -> bool {
    matches!(a.id(), "frisk" | "forewarn" | "anticipation" | "trace" | "imposter" | "illusion" | "receiver" | "powerofalchemy" | "pickpocket" | "magician")
}

/// Items with no handler at all, which act only through a move or ability that the test keeps out.
const INERT: [&str; 6] = ["damprock", "heatrock", "icyrock", "smoothrock", "lightclay", "terrainextender"];

/// The holder sets weather, terrain or screens with the move it uses, or with its ability.
fn uses_duration(m: &MonSpec) -> bool {
    let mv = m.moves[0].data();
    let field_move = !mv.weather.is_none() || !mv.terrain.is_none() || matches!(m.moves[0].id(), "reflect" | "lightscreen" | "auroraveil");
    let ab = matches!(m.ability.id(), "drought" | "drizzle" | "sandstream" | "snowwarning" | "electricsurge" | "grassysurge" | "psychicsurge" | "mistysurge" | "orichalcumpulse" | "hadronengine");
    field_move || ab
}

/// The opponent (seat 1) always uses its first move; game B changes what it never shows: its brought
/// Pokémon's other moves (and an inert item for another inert one), and everything of the ones it did
/// not bring but their species.
#[test]
fn hidden_information_changes_nothing_shown() {
    let pool = common::pool();
    let mut compared = 0;
    let mut skipped = 0;
    let mut observed = 0;
    let mut items_swapped = 0;
    let all_moves: Vec<MoveId> = pool.teams.iter().flat_map(|t| t.iter()).flat_map(|m| m.moves[..m.nmoves as usize].to_vec()).collect();
    let all_items: Vec<ItemId> = pool.teams.iter().flat_map(|t| t.iter()).map(|m| m.item).filter(|i| !i.is_none() && i.data().mega_stone.is_empty()).collect();
    for seed in 0..160u64 {
        let mut rng = Seeded::new(seed);
        let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
        let picks = [[0u8, 1, 2], [0u8, 1, 2]];
        let risky = teams[0].iter().any(|m| m.moves[..m.nmoves as usize].iter().any(|&x| reveals(x)) || reveals_ab(m.ability))
            || teams[1].iter().any(|m| reveals_ab(m.ability) || reveals(m.moves[0]));
        if risky {
            skipped += 1;
            continue;
        }
        let mut b_team: [MonSpec; 6] = teams[1];
        let others = distinct(&mut rng, all_moves.len(), 4);
        for (i, m) in b_team.iter_mut().enumerate() {
            let mut k = 1;
            for o in others.iter().map(|&x| all_moves[(x + i * 13) % all_moves.len()]) {
                if k < m.nmoves as usize && !m.moves[..m.nmoves as usize].contains(&o) && !reveals(o) {
                    m.moves[k] = o;
                    k += 1;
                }
            }
            if i >= 3 {
                // not brought: its first move, points, nature, item and ability too
                m.moves[0] = all_moves[(others[0] + 7 * i) % all_moves.len()];
                m.evs = [32, 0, 32, 0, 2, 0];
                m.nature = (m.nature + 3) % 25;
                m.item = all_items[(others[1] + i) % all_items.len()];
                let abs = m.species.data().abilities;
                if let Some(&a) = abs.iter().find(|a| a.0 != 0 && **a != m.ability) {
                    m.ability = a;
                }
            } else if INERT.contains(&m.item.id()) && !uses_duration(m) {
                let alt = INERT.iter().find(|x| **x != m.item.id()).unwrap();
                m.item = ItemId::from_id(alt).unwrap();
                items_swapped += 1;
            }
        }
        // compared up to the first time seat 1 uses another move than its first: that shows something
        let run = |t1: &[MonSpec; 6]| -> (Vec<battle::Input>, Builder, Vec<obs::ObsBuffers>) {
            let mut seen = Vec::new();
            let mut buf = Box::new(obs::ObsBuffers::default());
            let mut seats = Seats::new("h".into(), true);
            seats.start([&teams[0], t1]);
            obs::observe(&seats.builders[0].battle, ReqKind::Turn, &mut buf);
            seen.push((*buf).clone());
            seats.picks(0, picks[0]);
            seats.picks(1, picks[1]);
            let mut rows = Vec::new();
            let mut g = Game::start_logged([&teams[0], t1], picks, seed ^ 0x55, &mut rows).unwrap();
            seats.leads(&rows, &g.state);
            let mut p0 = by_name("random").unwrap();
            let mut r0 = Rng::new(seed);
            for _ in 0..400 {
                if g.state.ended {
                    break;
                }
                let o = g.state.options();
                let mut ch: [Option<Choice>; SIDES] = [None; SIDES];
                for side in 0..SIDES {
                    if !matches!(o[side].kind, Kind::Turn | Kind::Replace) {
                        continue;
                    }
                    let c = if side == 0 {
                        let kind = if o[0].kind == Kind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                        seats.request(0, kind, None, emit::screen_offer(&g.state, 0));
                        obs::observe(&seats.builders[0].battle, kind, &mut buf);
                        seen.push((*buf).clone());
                        let a = p0.choose_move(&seats.builders[0].battle, kind, &mut r0) as u16;
                        env::batch::choice_of(&o[0], a).unwrap_or(o[0].choices()[0])
                    } else if o[1].kind == Kind::Turn && (o[1].moves[0] || o[1].struggle || o[1].forced.is_some()) {
                        if o[1].moves[0] { Choice::Move { index: 0, mega: false } } else { o[1].choices()[0] }
                    } else {
                        o[1].choices()[0]
                    };
                    ch[side] = Some(c);
                }
                if matches!(ch[1], Some(Choice::Move { index, .. }) if index != 0) {
                    break;
                }
                seats.step(&mut g.state, &mut g.rng, ch).unwrap();
            }
            (seats.records[0].clone(), seats.builders[0].clone(), seen)
        };
        let (ra, ba, oa) = run(&teams[1]);
        let (rb, bb, ob) = run(&b_team);
        assert_eq!(oa.len(), ob.len(), "seed {}: seat 0 was asked a different number of times", seed);
        for (i, (x, y)) in oa.iter().zip(ob.iter()).enumerate() {
            assert!(x == y, "seed {}: observation {} differs", seed, i);
        }
        observed += oa.len();
        for (i, (x, y)) in ra.iter().zip(rb.iter()).enumerate() {
            assert_eq!(x, y, "seed {}: message {} differs", seed, i);
        }
        assert_eq!(ra.len(), rb.len(), "seed {}: seat 0 was shown a different number of messages", seed);
        assert_eq!(ba.battle, bb.battle, "seed {}: Battles differ", seed);
        compared += 1;
    }
    eprintln!("hidden: {} games compared, {} observations equal, {} skipped, {} brought items swapped", compared, observed, skipped, items_swapped);
    assert!(compared >= 40, "compared {} skipped {}", compared, skipped);
    assert!(observed >= compared * 2, "observations {}", observed);
}

#[test]
fn check_counts_a_wrong_battle() {
    // the check itself: a Battle missing the opponent's lead disagrees on the active and its HP
    let pool = common::pool();
    let mut rng = Seeded::new(9);
    let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
    let picks = [[0u8, 1, 2], [0u8, 1, 2]];
    let mut seats = Seats::new("t".into(), false);
    seats.start([&teams[0], &teams[1]]);
    seats.picks(0, picks[0]);
    seats.picks(1, picks[1]);
    let mut rows = Vec::new();
    let g = Game::start_logged([&teams[0], &teams[1]], picks, 1, &mut rows).unwrap();
    let mut t = Tally::default();
    check::agree(&seats.builders[0].battle, &g.state, 0, None, "before leads", &mut t);
    assert!(t.counts.contains_key("own_active") && t.counts.contains_key("opp_active"));
    seats.leads(&rows, &g.state);
    let mut t = Tally::default();
    check::agree(&seats.builders[0].battle, &g.state, 0, None, "after leads", &mut t);
    assert_eq!(t.total(), 0, "{:?}", t.examples);
}

/// The phone's order and wording on scripted rows: effectiveness after the HP row of its hit, Life Orb's
/// Held Item row before and its sentence after, a resist Berry's two rows around the hit, Focus Sash's
/// two rows after it.
#[test]
fn phone_order_of_a_hit() {
    use cg_engine::sim::log::{Fx, How, ItemWhat, Row};
    use cg_engine::sim::state::State;
    let pool = common::pool();
    let mut rng = Seeded::new(1);
    let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
    let mut rows = Vec::new();
    let s = State::start_logged([&teams[0], &teams[1]], [[0, 1, 2], [0, 1, 2]], &mut rng, &mut rows).unwrap();
    let names = emit::Names::of(&s);
    let eq = MoveId::from_id("earthquake").unwrap();
    let sash = ItemId::from_id("focussash").unwrap();
    let lo = ItemId::from_id("lifeorb").unwrap();
    let shuca = ItemId::from_id("shucaberry").unwrap();
    let step = vec![
        Row::Move { p: (0, 0), mv: eq, target: Some((1, 0)), called_by: Fx::None, priority: 0 },
        Row::Result { how: How::Super, p: Some((1, 0)) },
        Row::Item { p: (1, 0), item: shuca, what: ItemWhat::Used, of: None, by: Fx::None },
        Row::Item { p: (1, 0), item: shuca, what: ItemWhat::Activated, of: None, by: Fx::None },
        Row::Hp { p: (1, 0), before: 100, after: 40, cause: Fx::Move(eq), of: Some((0, 0)) },
        Row::Hp { p: (0, 0), before: 100, after: 90, cause: Fx::Item(lo), of: None },
        Row::Move { p: (0, 0), mv: eq, target: Some((1, 0)), called_by: Fx::None, priority: 0 },
        Row::Result { how: How::Super, p: Some((1, 0)) },
        Row::Item { p: (1, 0), item: sash, what: ItemWhat::Used, of: None, by: Fx::None },
        Row::Hp { p: (1, 0), before: 40, after: 1, cause: Fx::Move(eq), of: Some((0, 0)) },
    ];
    let mut e = emit::Emitter::new(&s);
    let ev = e.events(&step, &s, &s, &names, false);
    let kinds: Vec<String> = ev[0]
        .iter()
        .map(|x| match x {
            Event::Item { item, what, .. } => format!("item:{}:{}", item, what.id()),
            Event::Cause { effect, .. } => format!("cause:{}", effect),
            Event::Result { how, .. } => format!("result:{}", how.id()),
            x => x.kind().to_string(),
        })
        .collect();
    let want = [
        "move", "item:shucaberry:activated", "hp", "result:super", "item:shucaberry:activated", "item:lifeorb:activated", "hp", "cause:lost_some_hp", "move", "hp",
        "result:super", "item:focussash:activated", "item:focussash:used",
    ];
    assert_eq!(kinds, want);
    // the opponent's HP in percent for seat 0, in whole numbers for seat 1
    assert!(matches!(&ev[0][2], Event::Hp { hp: battle::Hp::Opp { .. }, .. }));
    assert!(matches!(&ev[1][2], Event::Hp { hp: battle::Hp::Own { before: 100, after: 40 }, .. }));
}

/// Rows the player-view hunt found shown wrong: Piercing Drill's ability is the attacker's; an Air
/// Balloon's announcement and a White Herb's use come after the Trick that gave them; a Berry taken by Bug
/// Bite and eaten is named as the target's, not the eater's.
#[test]
fn rows_of_the_hunt() {
    use cg_engine::sim::log::{Fx, ItemWhat, Row};
    use cg_engine::sim::state::State;
    let pool = common::pool();
    let mut rng = Seeded::new(1);
    let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
    let mut rows = Vec::new();
    let s = State::start_logged([&teams[0], &teams[1]], [[0, 1, 2], [0, 1, 2]], &mut rng, &mut rows).unwrap();
    let names = emit::Names::of(&s);
    let pd = AbilityId::from_id("piercingdrill").unwrap();
    let balloon = ItemId::from_id("airballoon").unwrap();
    let sitrus = ItemId::from_id("sitrusberry").unwrap();
    let step = vec![
        Row::Ability { p: (0, 0), ab: pd, of: None },
        Row::Once { p: Some((1, 0)), fx: Fx::Ability(pd), of: Some((0, 0)) },
        Row::Item { p: (1, 0), item: balloon, what: ItemWhat::Found, of: None, by: Fx::None },
        Row::Item { p: (1, 0), item: balloon, what: ItemWhat::Received, of: Some((0, 0)), by: Fx::Move(MoveId::from_id("trick").unwrap()) },
        Row::Move { p: (0, 0), mv: MoveId::from_id("bugbite").unwrap(), target: Some((1, 0)), called_by: Fx::None, priority: 0 },
        Row::Item { p: (1, 0), item: sitrus, what: ItemWhat::Removed, of: Some((0, 0)), by: Fx::Move(MoveId::from_id("bugbite").unwrap()) },
        Row::Hp { p: (0, 0), before: 10, after: 40, cause: Fx::Item(sitrus), of: None },
    ];
    let mut e = emit::Emitter::new(&s);
    let ev = e.events(&step, &s, &s, &names, false);
    let own = names.ids[0][0];
    let opp = names.ids[1][0];
    // one Ability row, naming the attacker
    let ab: Vec<&Event> = ev[0].iter().filter(|x| matches!(x, Event::Ability { .. })).collect();
    assert_eq!(ab.len(), 1);
    assert!(matches!(ab[0], Event::Ability { who, .. } if who.mon == own));
    // the balloon is received before it is announced
    let kinds: Vec<String> = ev[0].iter().filter_map(|x| match x { Event::Item { item, what, .. } if item == "airballoon" => Some(what.id().to_string()), _ => None }).collect();
    assert_eq!(kinds.first().map(|s| s.as_str()), Some("received"));
    // "stole and ate its target's Berry!" names only the eater (LV5Q-5); its Held Item row names the eater
    let taken = ev[0].iter().find(|x| matches!(x, Event::Item { item, what: battle::ItemWhat::Removed, .. } if item == "sitrusberry")).unwrap();
    assert!(matches!(taken, Event::Item { who: None, of: Some(o), .. } if o.mon == own), "{:?}", taken);
    let eaten = ev[0].iter().find(|x| matches!(x, Event::Item { item, what: battle::ItemWhat::Activated, .. } if item == "sitrusberry")).unwrap();
    assert!(matches!(eaten, Event::Item { who: Some(w), of: None, .. } if w.mon == own), "{:?}", eaten);
    let _ = opp;
}

/// What the phone shows for the rows of the controller's questions (LV5Q): an ability stopping a move
/// names the would-be user and the move; "no PP left" is the move row and a nameless row; a Future Sight
/// landing is its own row with no cause row; Trick's `received` rows name no giver; no `called_by`.
#[test]
fn rows_as_the_phone_shows_them() {
    use cg_engine::sim::log::{Fx, ItemWhat, Row};
    use cg_engine::sim::state::State;
    let pool = common::pool();
    let mut rng = Seeded::new(1);
    let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
    let mut rows = Vec::new();
    let s = State::start_logged([&teams[0], &teams[1]], [[0, 1, 2], [0, 1, 2]], &mut rng, &mut rows).unwrap();
    let names = emit::Names::of(&s);
    let damp = AbilityId::from_id("damp").unwrap();
    let boom = MoveId::from_id("explosion").unwrap();
    let fs = MoveId::from_id("futuresight").unwrap();
    let trick = MoveId::from_id("trick").unwrap();
    let scarf = ItemId::from_id("choicescarf").unwrap();
    let opp_move = s.sides[1].mons[0].moves[0].id;
    let step = vec![
        Row::Cant { p: (0, 0), why: Fx::Ability(damp), mv: opp_move },
        Row::Cant { p: (1, 0), why: Fx::None, mv: opp_move },
        Row::Once { p: Some((0, 0)), fx: Fx::Move(fs), of: None },
        Row::Hp { p: (0, 0), before: 100, after: 50, cause: Fx::Cond(Cond::from_id("futuremove").unwrap()), of: Some((1, 0)) },
        Row::Move { p: (1, 0), mv: trick, target: Some((0, 0)), called_by: Fx::Move(MoveId::from_id("sleeptalk").unwrap()), priority: 0 },
        Row::Once { p: Some((1, 0)), fx: Fx::Move(trick), of: Some((0, 0)) },
        Row::Item { p: (0, 0), item: scarf, what: ItemWhat::Received, of: Some((1, 0)), by: Fx::Move(trick) },
    ];
    let mut e = emit::Emitter::new(&s);
    let ev = e.events(&step, &s, &s, &names, false);
    let own = names.ids[0][0];
    let opp = names.ids[1][0];
    let shown: Vec<String> = ev[0].iter().map(|x| x.to_json().to_string()).collect();
    let k = |i: usize| ev[0][i].clone();
    assert!(matches!(k(0), Event::Ability { who, ability } if who.mon == own && ability == "damp"), "{:?}", shown);
    assert!(matches!(k(1), Event::Cant { who: Some(w), reason } if w.mon == opp && reason == opp_move.id()), "{:?}", shown);
    assert!(matches!(k(2), Event::Move { who, mv, .. } if who.mon == opp && mv == opp_move.id()), "{:?}", shown);
    assert!(matches!(k(3), Event::Cant { who: None, reason } if reason == "nopp"), "{:?}", shown);
    assert!(matches!(k(4), Event::Once { effect, .. } if effect == "futuresight"), "{:?}", shown);
    assert!(matches!(k(5), Event::Hp { .. }), "{:?}", shown);
    assert!(matches!(k(6), Event::Move { called_by: None, .. }), "{:?}", shown);
    assert!(matches!(k(7), Event::Once { effect, .. } if effect == "trick"), "{:?}", shown);
    assert!(matches!(k(8), Event::Item { of: None, .. }), "{:?}", shown);
    assert_eq!(ev[0].len(), 9, "{:?}", shown);
    let _ = boom;
}

/// Future Sight writes two rows: its use names the user (`futuresight_start`), its landing names the
/// target (`futuresight`). A landing on a Pokémon that used Future Sight itself this turn is still a landing.
#[test]
fn future_sight_use_and_landing() {
    use cg_engine::sim::log::{Fx, Row};
    use cg_engine::sim::state::State;
    let pool = common::pool();
    let mut rng = Seeded::new(1);
    let (_, teams) = pool.draw(&mut rng, TeamMode::AsGiven);
    let mut rows = Vec::new();
    let s = State::start_logged([&teams[0], &teams[1]], [[0, 1, 2], [0, 1, 2]], &mut rng, &mut rows).unwrap();
    let names = emit::Names::of(&s);
    let fs = MoveId::from_id("futuresight").unwrap();
    let fm = Cond::from_id("futuremove").unwrap();
    let step = vec![
        Row::Move { p: (0, 0), mv: fs, target: Some((1, 0)), called_by: Fx::None, priority: 0 },
        Row::Once { p: Some((0, 0)), fx: Fx::Move(fs), of: None },
        Row::Move { p: (1, 0), mv: fs, target: Some((0, 0)), called_by: Fx::None, priority: 0 },
        Row::Once { p: Some((1, 0)), fx: Fx::Move(fs), of: None },
        Row::Once { p: Some((1, 0)), fx: Fx::Move(fs), of: None },
        Row::Hp { p: (1, 0), before: 100, after: 40, cause: Fx::Cond(fm), of: Some((0, 0)) },
    ];
    let mut e = emit::Emitter::new(&s);
    let ev = e.events(&step, &s, &s, &names, false);
    let once: Vec<(String, battle::Side)> =
        ev[0].iter().filter_map(|x| match x { Event::Once { effect, who: Some(w) } => Some((effect.clone(), w.side)), _ => None }).collect();
    let want = [
        ("futuresight_start".to_string(), battle::Side::Own),
        ("futuresight_start".to_string(), battle::Side::Opp),
        ("futuresight".to_string(), battle::Side::Opp),
    ];
    assert_eq!(once, want);
}
