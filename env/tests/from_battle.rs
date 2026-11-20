//! A State built from one seat's Battle with the true hidden sets as the guesses, against the true State
//! of self-play games: no guess rejected, our seat never offered what its Battle forbids, every built
//! Game steppable, and the differences by class (printed). Covers the shipped pool and the mechanics pools
//! (PP, Substitute, Encore, Disable, Taunt, Imprison, traps, stat stages) at turn and replace requests.

mod common;

use battle::{legal, ReqKind};
use cg_engine::sim::state::SIDES;
use cg_engine::sim::{Choice, Kind};
use emit::check::state_mask;
use env::batch::RequestKind;
use env::from_battle::{state_diff_why, state_from_battle, OppGuess};
use env::teams::TeamPool;
use env::{BatchEnv, EnvConfig, TeamMode};
use players::{by_name, Rng};
use std::collections::BTreeMap;
use std::sync::Arc;

fn guesses(env: &BatchEnv, g: usize, seat: usize) -> (Vec<OppGuess>, [Option<u8>; 6]) {
    let s = env.slot(g);
    let o = 1 - seat;
    let picks = s.picks[o].expect("picks");
    let gs = picks
        .iter()
        .map(|&i| {
            let m = &s.rosters[o][i as usize];
            OppGuess { slot: i, moves: m.moves[..m.nmoves as usize].to_vec(), item: Some(m.item), ability: Some(m.ability), nature: Some(m.nature), points: Some(m.evs), gender: Some(m.gender) }
        })
        .collect();
    let mut gd = [None; 6];
    for i in 0..6 {
        gd[i] = Some(s.rosters[seat][i].gender);
    }
    (gs, gd)
}

#[derive(Default)]
struct Tally {
    classes: BTreeMap<String, u64>,
    examples: BTreeMap<String, Vec<String>>,
    hits: BTreeMap<&'static str, u64>,
    turn: u64,
    replace: u64,
    errors: u64,
    exact: u64,
    exact_but: u64,
    truth_points: u64,
    legal_eq: u64,
    subset: u64,
    state_legal_eq: u64,
    rejected: u64,
    steppable: u64,
}

impl Tally {
    fn add(&mut self, k: String, why: String) {
        let e = self.examples.entry(k.clone()).or_default();
        if e.len() < 3 && !why.is_empty() {
            e.push(why);
        }
        *self.classes.entry(k).or_default() += 1;
    }
}

fn first_choices(o: &[cg_engine::sim::SideOptions; SIDES]) -> [Option<Choice>; SIDES] {
    let mut ch = [None; SIDES];
    for s in 0..SIDES {
        if matches!(o[s].kind, Kind::Turn | Kind::Replace) {
            ch[s] = o[s].choices().first().copied();
        }
    }
    ch
}

/// One configuration: a pool, two players, games to play.
fn run(pool: TeamPool, players: [&str; 2], games: u64, seed: u64, t: &mut Tally) {
    for rbr in [false, true] {
        let cfg = EnvConfig { games: 32, seed: seed + rbr as u64, team_mode: TeamMode::AsGiven, replace_before_rows: rbr, ..EnvConfig::default() };
        let mut env = BatchEnv::new(Arc::new(pool.clone()), cfg);
        let mut ps = [by_name(players[0]).unwrap(), by_name(players[1]).unwrap()];
        let mut rng = Rng::new(seed ^ 5);
        let mut acts = vec![0u16; 32];
        let mut lg = Vec::new();
        while env.games_done < games {
            for g in 0..32 {
                let p = env.pending(g);
                let seat = p.seat as usize;
                let b = *env.battle(g, seat);
                acts[g] = match p.kind {
                    RequestKind::Preview => env::batch::preview_action(ps[seat].teampreview(&b, &mut rng)).unwrap(),
                    k => {
                        let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                        let a = ps[seat].choose_move(&b, rk, &mut rng) as u16;
                        env.legal_actions(g, &mut lg);
                        if lg.contains(&a) {
                            a
                        } else {
                            lg[0]
                        }
                    }
                };
                if p.kind == RequestKind::Preview {
                    continue;
                }
                let rk = if p.kind == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                let (gs, gd) = guesses(&env, g, seat);
                let truth = env.slot(g).game.state;
                let bl = legal(&b, rk).mask;
                // the paths this covers
                let a = &b.sides[0].active;
                let own = b.active(battle::Side::Own);
                if let Some(w) = own {
                    if b.pp_left(w).iter().any(|x| *x == Some(0)) {
                        *t.hits.entry("own move at 0 PP").or_default() += 1;
                    }
                }
                for (name, id) in [("substitute", "substitute"), ("encore", "encore"), ("disable", "disable"), ("taunt", "taunt"), ("torment", "torment")] {
                    if a.vols.iter().flatten().any(|v| v.cond.id() == id) {
                        *t.hits.entry(name).or_default() += 1;
                    }
                }
                if b.refused.iter().any(|x| *x) {
                    *t.hits.entry("a refusal at this request").or_default() += 1;
                }
                if rk == ReqKind::Replace && b.ctx.unread {
                    *t.hits.entry("replace before its rows").or_default() += 1;
                }
                if b.turn >= 20 {
                    *t.hits.entry("turn 20 or later").or_default() += 1;
                }
                match state_from_battle(&b, rk, seat, &gs, &gd, 1) {
                    Err(e) => {
                        t.errors += 1;
                        t.add(format!("not built: {}", e), format!("turn {} kind {:?}", b.turn, rk));
                    }
                    Ok((st, rej)) => {
                        if rk == ReqKind::Turn {
                            t.turn += 1;
                        } else {
                            t.replace += 1;
                        }
                        // the true sets are never rejected, unless the opponent brought Illusion (a disguise
                        // leads and uses its own moves under another name)
                        let s = env.slot(g);
                        let illusion = s.picks[1 - seat].map_or(false, |p| p.iter().any(|&i| s.rosters[1 - seat][i as usize].ability == cg_engine::sim::data::abilities::ILLUSION));
                        if !illusion {
                            t.rejected += rej.iter().any(|r| r.field != "brought") as u64;
                        }
                        for r in &rej {
                            t.add(format!("rejected {} {}", r.field, r.reason), String::new());
                        }
                        let o = st.options();
                        let sm = state_mask(&o[seat]);
                        t.legal_eq += (bl == sm) as u64;
                        let sub = (0..battle::N_ACTIONS).all(|i| !sm[i] || bl[i]);
                        t.subset += sub as u64;
                        if !sub {
                            t.add("our State legal set not within the Battle's".into(), format!("turn {} kind {:?} battle {:?} built {:?} | picks {:?} active {:?} out {} on_field {:?} sent_in {:?} own_switch {:?} unread {} fainted {:?} own_choice {:?} now {:?} state active {} fainted {:?} truth active {} truth fainted {:?}", b.turn, rk, bl, sm, b.picks, b.sides[0].active.slot, b.sides[0].active.out, battle::legal::on_field(&b), b.ctx.sent_in, b.ctx.own_switch, b.ctx.unread, b.sides[0].team.iter().map(|m| m.fainted).collect::<Vec<_>>(), b.ctx.own_choice, b.sides[0].now.act, st.sides[seat].active, st.sides[seat].mons.iter().map(|m| m.fainted).collect::<Vec<_>>(), truth.sides[seat].active, truth.sides[seat].mons.iter().map(|m| m.fainted).collect::<Vec<_>>())
                                + &format!(" | forced {:?} lock {:?} transformed {:?} vols {:?} state vols {:?} true vols {:?} moves {:?}", legal(&b, rk).forced, a.lock, a.transformed, a.vols.iter().flatten().map(|v| v.cond.id()).collect::<Vec<_>>(), st.sides[seat].volatiles().iter().map(|v| v.id.id()).collect::<Vec<_>>(), truth.sides[seat].volatiles().iter().map(|v| v.id.id()).collect::<Vec<_>>(), st.sides[seat].active_mon().moves().iter().map(|m| (m.id.id(), m.pp, m.disabled)).collect::<Vec<_>>()));
                        }
                        // one step with the first legal actions
                        let mut s2 = st;
                        let mut r = cg_engine::sim::outcome::Seeded::new(3);
                        let ok = s2.step(first_choices(&o), &mut r).is_ok()
                            && (0..SIDES).all(|x| !matches!(o[x].kind, Kind::Turn | Kind::Replace) || !o[x].choices().is_empty());
                        t.steppable += ok as u64;
                        if !ok {
                            t.add("not steppable".into(), format!("turn {} kind {:?}", b.turn, rk));
                        }
                        if rk == ReqKind::Turn {
                            t.truth_points += 1;
                            t.state_legal_eq += (0..2).all(|x| state_mask(&o[x]) == state_mask(&truth.options()[x])) as u64;
                            let d = state_diff_why(&st, &truth, seat);
                            if d.is_empty() {
                                t.exact += 1;
                            }
                            if d.iter().all(|(k, _)| k.ends_with("order") || k == "opp_mon_hp") {
                                t.exact_but += 1;
                            }
                            for (k, why) in d {
                                t.add(k, format!("turn {} {}", b.turn, why));
                            }
                        }
                    }
                }
            }
            env.act(&acts).unwrap();
        }
    }
}

fn pool_file(name: &str) -> TeamPool {
    TeamPool::from_file(&format!("{}/../bench/data/pools/{}.json", env!("CARGO_MANIFEST_DIR"), name)).expect("pool")
}

fn report(name: &str, t: &Tally) {
    let n = t.turn + t.replace;
    println!(
        "GV-2 {}: built {} (turn {}, replace {}), not built {}; our State legal within the Battle's {}, equal {}; steppable {}; with a rejected guess {}; turn requests vs truth: both seats' State legal equal {} of {}, exact on every compared part {}, exact but for team order and opponent HP rounding {}",
        name, n, t.turn, t.replace, t.errors, t.subset, t.legal_eq, t.steppable, t.rejected, t.state_legal_eq, t.truth_points, t.exact, t.exact_but
    );
    println!("  paths reached: {:?}", t.hits);
    let mut v: Vec<_> = t.classes.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1));
    for (k, c) in v.iter().take(40) {
        println!("  {:>6} {}", c, k);
        if std::env::var("CG_EXAMPLES").map_or(false, |x| x == "1") {
            for e in t.examples.get(*k).into_iter().flatten() {
                println!("           {}", e);
            }
        }
    }
}

fn check(t: &Tally, exact_share: Option<u64>) {
    let n = t.turn + t.replace;
    assert!(n > 500);
    if exact_share.is_some() {
        // the mechanics pools are made up for their mechanics: their sets need not be legal
        assert_eq!(t.rejected, 0, "true sets were rejected");
    }
    assert_eq!(t.subset, n, "our State offered what the Battle forbids");
    assert_eq!(t.steppable, n, "a built Game could not be stepped");
    // the only Games not built: a replace request with nobody left to send in
    assert!(t.classes.keys().filter(|k| k.starts_with("not built")).all(|k| k.contains("nothing to choose")), "{:?}", t.classes.keys().filter(|k| k.starts_with("not built")).collect::<Vec<_>>());
    if let Some(pct) = exact_share {
        assert!(t.exact_but * 100 >= t.truth_points * pct, "fewer than {}% exact but for order and HP rounding", pct);
    }
}

#[test]
fn built_states_match_the_truth() {
    let mut t = Tally::default();
    run(common::pool(), ["tower", "max_damage"], 150, 61, &mut t);
    report("shipped pool, tower vs max_damage", &t);
    check(&t, Some(95));
    assert_eq!(t.legal_eq, t.turn + t.replace, "our State legal set differs from the Battle's");
}

#[test]
fn built_states_on_mechanics_pools() {
    let mut all = Tally::default();
    for (pool, players) in [
        ("pp", ["disciplined", "tower"]),
        ("pilot_encore", ["pilot_encore", "max_damage"]),
        ("imprison", ["tower", "random"]),
        ("traps", ["tower", "disciplined"]),
        ("stages", ["endgame", "tower"]),
        ("charges", ["random", "tower"]),
        ("pilot_weather", ["pilot_weather", "disciplined"]),
        ("pilot_hazard", ["pilot_hazard", "tower"]),
    ] {
        let mut t = Tally::default();
        run(pool_file(pool), players, 60, 300 + pool.len() as u64, &mut t);
        report(&format!("{} pool, {} vs {}", pool, players[0], players[1]), &t);
        check(&t, None);
        for (k, v) in t.hits {
            *all.hits.entry(k).or_default() += v;
        }
    }
    println!("GV-2 mechanics pools, paths reached in all: {:?}", all.hits);
    for k in ["own move at 0 PP", "substitute", "encore", "disable", "taunt", "a refusal at this request", "replace before its rows", "turn 20 or later"] {
        assert!(all.hits.get(k).copied().unwrap_or(0) > 0, "path not reached: {}", k);
    }
}
