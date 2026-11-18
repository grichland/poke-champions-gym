//! N games between two named Players (`players::NAMES`) on the engine, each seeing only its own
//! Battle. Half the games with A in seat 0, half with A in seat 1, on the same seeds (so the same team
//! draws). Team preview goes through `Player::teampreview`. Seeded and repeatable for a given thread count.
//!
//! Usage: players_run --a tower --b max_damage --games 10000 [--seed 1] [--threads 8] [--cap 600]
//!        [--teams tools/rig/data/teams.json] [--out FILE.jsonl] [--games-out FILE.jsonl] [--events-out FILE.jsonl]
//!        [--save-events 50] [--no-checks] [--state-rule] [--team-mode real|stage3] [--teams-a FILE.json]
//!        [--replace-after-rows]
//! `--replace-after-rows`: each step's rows reach the seats at once, so a replacement is decided after the
//! rows of its turn (the default is the live order: the replace request comes before them).
//! With `--teams-a`, A draws its team from that file and B from `--teams` (per-seat team weights).
//! Prints one summary JSON line; `--out` appends it. `--games-out` gets one line per game (seed, episode,
//! seats, teams, picks, result: enough to replay it). `--events-out` gets both seats' full records of the
//! first `--save-events` games of each thread and seat assignment. Per player it also reports voluntary
//! switches (a switch chosen at a turn request while a move was legal) and its rule firings.

use battle::json::Json;
use battle::{Input, ReqKind};
use env::batch::preview_action;
use env::teams::{mix, TeamMode, TeamPool};
use env::{BatchEnv, Counters, EnvConfig, RequestKind, Rule};
use emit::check::Tally;
use players::{by_name, Rng, NAMES};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

struct Args {
    a: String,
    b: String,
    games: usize,
    seed: u64,
    threads: usize,
    cap: f64,
    teams: String,
    /// A's own team file (B keeps `teams`)
    teams_a: Option<String>,
    out: Option<String>,
    games_out: Option<String>,
    events_out: Option<String>,
    save_events: usize,
    checks: bool,
    label: String,
    /// the State's legal sets instead of the Battle's (comparison); the Players still choose from their Battle
    state_rule: bool,
    /// prints the State after each decision of this episode (thread 0, first seat assignment)
    trace: Option<u64>,
    trace_assign: usize,
    trace_thread: usize,
    team_mode: TeamMode,
    replace_before_rows: bool,
}

fn args() -> Args {
    let v: Vec<String> = std::env::args().collect();
    let get = |k: &str| v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned();
    Args {
        a: get("--a").unwrap_or_else(|| "tower".into()),
        b: get("--b").unwrap_or_else(|| "max_damage".into()),
        games: get("--games").and_then(|x| x.parse().ok()).unwrap_or(1000),
        seed: get("--seed").and_then(|x| x.parse().ok()).unwrap_or(1),
        threads: get("--threads").and_then(|x| x.parse().ok()).unwrap_or(1).max(1),
        cap: get("--cap").and_then(|x| x.parse().ok()).unwrap_or(600.0),
        teams: get("--teams").unwrap_or_else(|| "tools/rig/data/teams.json".into()),
        teams_a: get("--teams-a"),
        out: get("--out"),
        games_out: get("--games-out"),
        events_out: get("--events-out"),
        save_events: get("--save-events").and_then(|x| x.parse().ok()).unwrap_or(0),
        checks: !v.iter().any(|a| a == "--no-checks"),
        label: get("--label").unwrap_or_default(),
        state_rule: v.iter().any(|a| a == "--state-rule"),
        trace: get("--trace").and_then(|x| x.parse().ok()),
        trace_assign: get("--trace-assign").and_then(|x| x.parse().ok()).unwrap_or(0),
        trace_thread: get("--trace-thread").and_then(|x| x.parse().ok()).unwrap_or(0),
        team_mode: match get("--team-mode").as_deref() {
            Some("stage3") => TeamMode::Stage3,
            _ => TeamMode::AsGiven,
        },
        replace_before_rows: !v.iter().any(|a| a == "--replace-after-rows"),
    }
}

#[derive(Default)]
struct Out {
    /// per seat assignment (0: A in seat 0): A wins, B wins, ties, truncated, games
    by_assign: [[u64; 5]; 2],
    /// seat 0 wins, seat 1 wins
    seat_wins: [u64; 2],
    turns: Vec<u16>,
    counters: Counters,
    decisions: u64,
    fallbacks: u64,
    games_lines: Vec<String>,
    events_lines: Vec<String>,
    capped: bool,
    errors: Vec<String>,
    /// per player (0: A, 1: B): voluntary switches, turn and replace decisions asked
    vol_switches: [u64; 2],
    asked: [u64; 2],
    /// per player (0: A, 1: B): rule firings by name
    fired: [BTreeMap<&'static str, u64>; 2],
}

fn record_json(r: &[Input]) -> Json {
    Json::Arr(
        r.iter()
            .map(|i| match i {
                Input::Msg(m) => m.to_json(),
                Input::Picks(p) => p.to_json(),
                Input::Order(o) => {
                    let mut j = Json::obj();
                    j.set("order", o.to_json());
                    j
                }
            })
            .collect(),
    )
}

/// One thread: `n` games under each seat assignment.
fn run_thread(a: &Args, pool: Arc<TeamPool>, t: usize, n: usize, t0: Instant) -> Out {
    let mut out = Out::default();
    let seed = mix(a.seed, t as u64);
    for assign in 0..2usize {
        let names = if assign == 0 { [a.a.as_str(), a.b.as_str()] } else { [a.b.as_str(), a.a.as_str()] };
        let mut players = [by_name(names[0]).expect("player"), by_name(names[1]).expect("player")];
        let cfg = EnvConfig { games: 1, seed, team_mode: a.team_mode, max_steps: 1000, checks: a.checks, keep_records: true, auto_locked: true, replace_before_rows: a.replace_before_rows };
        let rule = if a.state_rule { Rule::State } else { Rule::Battle };
        let mut env = if a.teams_a.is_some() { BatchEnv::with_agent(pool.clone(), cfg, rule, vec![assign as u8]) } else { BatchEnv::with_rule(pool.clone(), cfg, rule) };
        let mut done = 0usize;
        let mut rngs = [Rng::new(mix(seed, 1)), Rng::new(mix(seed, 2))];
        let mut picks = [[0u8; 3]; 2];
        let mut legal = Vec::new();
        while done < n {
            if t0.elapsed().as_secs_f64() > a.cap {
                out.capped = true;
                break;
            }
            let p = env.pending(0);
            let seat = p.seat as usize;
            let ep = env.slot(0).episode;
            if p.kind == RequestKind::Preview && seat == 0 {
                rngs = [Rng::new(mix(mix(seed, ep), 1)), Rng::new(mix(mix(seed, ep), 2))];
            }
            let b = *env.battle(0, seat);
            let act = match p.kind {
                RequestKind::Preview => {
                    let pk = players[seat].teampreview(&b, &mut rngs[seat]);
                    picks[seat] = pk;
                    preview_action(pk).unwrap_or(0)
                }
                k => {
                    let rk = if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                    let act = players[seat].choose_move(&b, rk, &mut rngs[seat]);
                    let who = if seat == assign { 0 } else { 1 };
                    out.asked[who] += 1;
                    if rk == ReqKind::Turn && act >= 8 && battle::legal(&b, rk).mask[..8].iter().any(|x| *x) {
                        out.vol_switches[who] += 1;
                    }
                    act as u16
                }
            };
            env.legal_actions(0, &mut legal);
            let act = if legal.contains(&act) {
                act
            } else {
                out.fallbacks += 1;
                legal[0]
            };
            if let Err(e) = env.act(&[act]) {
                out.errors.push(format!("thread {} assign {} ep {}: {}", t, assign, ep, e));
                env.counters.errors += 1;
                break;
            }
            if a.trace == Some(ep) && t == a.trace_thread && assign == a.trace_assign {
                for r in &env.slot(0).last_rows {
                    eprintln!("  row {}", emit::fmt::show(r));
                }
                for (k, evs) in env.slot(0).seats.last.iter().enumerate() {
                    for e in evs {
                        eprintln!("  seat{} {}", k, e.to_json().to_string());
                    }
                }
                for k in 0..2 {
                    for an in &env.slot(0).seats.builders[k].anomalies {
                        eprintln!("  ANOMALY seat{} at {} {}", k, an.at, an.what);
                    }
                }
                for k in 0..2 {
                    let bt = env.battle(0, k);
                    if let Some(w) = bt.active(battle::Side::Opp) {
                        let vols: Vec<&str> = bt.side(battle::Side::Opp).active.vols.iter().flatten().map(|v| v.cond.id()).collect();
                        let mut pr = [0f32; 13];
                        obs::opp_prior(bt, &mut pr);
                        eprintln!("  BATTLE seat{} opp moves {:?} pp {:?} lock {:?} vols {:?} item {:?} ability {:?} prior {:?}", k, bt.mon(w).moves.map(|m| m.map(|x| x.id())), bt.pp_left(w), bt.side(battle::Side::Opp).active.lock, vols, bt.mon(w).item, bt.mon(w).ability.map(|a| a.id()), pr);
                    }
                }
                let mut tl = Tally::default();
                let st = &env.slot(0).game.state;
                for x in 0..2 {
                    emit::check::agree(env.battle(0, x), st, x, env.slot(0).seats.emitter.disguise[1 - x], &format!("seat {}", x), &mut tl);
                }
                for (k, v) in &tl.examples {
                    eprintln!("  CHECK {} {:?}", k, v);
                }
                let pn = env.pending(0);
                if pn.kind != RequestKind::Preview {
                    let rk = if pn.kind == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
                    let ps = pn.seat as usize;
                    let cl = emit::check::legal_classes(env.battle(0, ps), st, ps, rk, &env.slot(0).state_opts[ps]);
                    if !cl.is_empty() {
                        eprintln!("  LEGAL seat {} {:?}", ps, cl);
                    }
                }
            }
            if a.trace == Some(ep) && t == a.trace_thread && assign == a.trace_assign {
                let st = &env.slot(0).game.state;
                let sd: Vec<String> = (0..2)
                    .map(|x| {
                        let m = st.active(x);
                        let v: Vec<&str> = st.sides[x].volatiles().iter().map(|v| v.id.id()).collect();
                        format!("{} hp {}/{} {:?} st {:?} vols {:?}", m.set_species.id(), m.hp, m.maxhp, m.boosts, m.status, v)
                    })
                    .collect();
                eprintln!("trace assign {} seat {} act {} turn {} | {} | {}", assign, seat, act, st.turn, sd[0], sd[1]);
            }
            let r = env.results()[0];
            if r.done {
                done += 1;
                let s = env.slot(0);
                let ids = s.team_ids;
                let _ = ids;
                let a_seat = assign;
                let k = &mut out.by_assign[assign];
                k[4] += 1;
                match r.winner {
                    Some(w) if r.truncated => {
                        let _ = w;
                        k[3] += 1
                    }
                    Some(w) => {
                        out.seat_wins[w as usize] += 1;
                        if w as usize == a_seat {
                            k[0] += 1
                        } else {
                            k[1] += 1
                        }
                    }
                    None if r.truncated => k[3] += 1,
                    None => k[2] += 1,
                }
                out.turns.push(r.turns);
                let mut g = Json::obj();
                g.set("thread", t as i64);
                g.set("seed", seed.to_string());
                g.set("episode", r.episode as i64);
                g.set("seat0", names[0]);
                g.set("seat1", names[1]);
                g.set("picks0", Json::Arr(picks[0].iter().map(|x| Json::from(*x as i64)).collect()));
                g.set("picks1", Json::Arr(picks[1].iter().map(|x| Json::from(*x as i64)).collect()));
                g.set("winner", r.winner.map_or(Json::Null, |w| Json::from(w as i64)));
                g.set("turns", r.turns as i64);
                g.set("truncated", r.truncated);
                out.games_lines.push(g.to_string());
                if (r.episode as usize) < a.save_events {
                    if let Some(rec) = &s.last_records {
                        let mut e = Json::obj();
                        e.set("thread", t as i64);
                        e.set("seed", seed.to_string());
                        e.set("episode", r.episode as i64);
                        e.set("seat0", names[0]);
                        e.set("seat1", names[1]);
                        e.set("record0", record_json(&rec[0]));
                        e.set("record1", record_json(&rec[1]));
                        out.events_lines.push(e.to_string());
                    }
                }
            }
        }
        out.decisions += env.decisions;
        merge(&mut out.counters, &env.counters);
        for (seat, p) in players.iter().enumerate() {
            let who = if seat == assign { 0 } else { 1 };
            for (k, v) in p.fired() {
                *out.fired[who].entry(k).or_default() += v;
            }
        }
    }
    out
}

fn merge(a: &mut Counters, b: &Counters) {
    a.merge(b);
}

fn tally_json(t: &Tally) -> Json {
    let mut o = Json::obj();
    let mut v: Vec<(&String, &u64)> = t.counts.iter().collect();
    v.sort_by(|x, y| y.1.cmp(x.1));
    for (k, c) in v {
        let mut e = Json::obj();
        e.set("n", *c as i64);
        e.set("examples", Json::Arr(t.examples.get(k).map(|x| x.iter().map(|s| Json::from(s.as_str())).collect()).unwrap_or_default()));
        o.set(k, e);
    }
    o
}

/// One pool of B's teams then A's, weighted so the first-drawing seat (A's) takes only A's.
fn two_pools(b_file: &str, a_file: &str) -> TeamPool {
    let read = |f: &str| match Json::parse(&std::fs::read_to_string(f).expect("teams file")).expect("teams json") {
        Json::Arr(v) => v,
        _ => panic!("{}: not a list of teams", f),
    };
    let (mut all, extra) = (read(b_file), read(a_file));
    let nb = all.len();
    all.extend(extra);
    let n = all.len();
    let mut p = TeamPool::from_json(&Json::Arr(all).to_string()).expect("teams");
    if !p.errors.is_empty() {
        eprintln!("warning: {}", p.dropped_message());
    }
    let first: Vec<f64> = (0..n).map(|i| if i >= nb { 1.0 } else { 0.0 }).collect();
    let other: Vec<f64> = first.iter().map(|w| 1.0 - w).collect();
    p.set_weights(&first, &other, false).expect("weights");
    p
}

fn main() {
    let a = args();
    let pool = Arc::new(match &a.teams_a {
        None => TeamPool::open(&a.teams).expect("teams"),
        Some(ta) => two_pools(&a.teams, ta),
    });
    if by_name(&a.a).is_none() || by_name(&a.b).is_none() {
        eprintln!("unknown player: use one of {}", NAMES.join(", "));
        std::process::exit(2);
    }
    let per = a.games / 2;
    let t0 = Instant::now();
    let shares: Vec<usize> = (0..a.threads).map(|t| per / a.threads + (t < per % a.threads) as usize).collect();
    let outs: Vec<Out> = std::thread::scope(|sc| {
        let hs: Vec<_> = shares.iter().enumerate().map(|(t, &n)| {
            let pool = pool.clone();
            let a = &a;
            sc.spawn(move || run_thread(a, pool, t, n, t0))
        }).collect();
        hs.into_iter().map(|h| h.join().expect("thread")).collect()
    });
    let wall = t0.elapsed().as_secs_f64();
    let mut all = Out::default();
    for o in outs {
        for i in 0..2 {
            for k in 0..5 {
                all.by_assign[i][k] += o.by_assign[i][k];
            }
            all.seat_wins[i] += o.seat_wins[i];
        }
        all.turns.extend(o.turns);
        merge(&mut all.counters, &o.counters);
        all.decisions += o.decisions;
        all.fallbacks += o.fallbacks;
        all.games_lines.extend(o.games_lines);
        all.events_lines.extend(o.events_lines);
        all.capped |= o.capped;
        all.errors.extend(o.errors);
        for i in 0..2 {
            all.vol_switches[i] += o.vol_switches[i];
            all.asked[i] += o.asked[i];
            for (k, v) in &o.fired[i] {
                *all.fired[i].entry(k).or_default() += v;
            }
        }
    }
    let games: u64 = all.by_assign.iter().map(|x| x[4]).sum();
    let a_wins: u64 = all.by_assign[0][0] + all.by_assign[1][0];
    let b_wins: u64 = all.by_assign[0][1] + all.by_assign[1][1];
    let c = &all.counters;
    let mut o = Json::obj();
    o.set("label", a.label.as_str());
    o.set("rule", if a.state_rule { "state" } else { "battle" });
    o.set("checks", a.checks);
    o.set("team_mode", if a.team_mode == TeamMode::Stage3 { "stage3" } else { "real" });
    o.set("teams", a.teams.as_str());
    o.set("teams_a", a.teams_a.as_deref().map_or(Json::Null, Json::from));
    o.set("a", a.a.as_str());
    o.set("b", a.b.as_str());
    o.set("seed", a.seed as i64);
    o.set("threads", a.threads as i64);
    o.set("games", games as i64);
    o.set("a_wins", a_wins as i64);
    o.set("b_wins", b_wins as i64);
    o.set("a_win_rate", if games > 0 { a_wins as f64 / games as f64 } else { 0.0 });
    let mut asg = vec![];
    for i in 0..2 {
        let k = all.by_assign[i];
        let mut e = Json::obj();
        e.set("a_seat", i as i64);
        e.set("games", k[4] as i64);
        e.set("a_wins", k[0] as i64);
        e.set("b_wins", k[1] as i64);
        e.set("ties", k[2] as i64);
        e.set("truncated", k[3] as i64);
        e.set("a_win_rate", if k[4] > 0 { k[0] as f64 / k[4] as f64 } else { 0.0 });
        asg.push(e);
    }
    o.set("by_assignment", Json::Arr(asg));
    o.set("seat0_win_rate", if games > 0 { all.seat_wins[0] as f64 / games as f64 } else { 0.0 });
    let n = all.turns.len().max(1) as f64;
    o.set("mean_turns", all.turns.iter().map(|&t| t as f64).sum::<f64>() / n);
    o.set("max_turns", all.turns.iter().copied().max().unwrap_or(0) as i64);
    o.set("truncated", (all.by_assign[0][3] + all.by_assign[1][3]) as i64);
    o.set("ties", (all.by_assign[0][2] + all.by_assign[1][2]) as i64);
    o.set("decisions", all.decisions as i64);
    o.set("asked", c.asked as i64);
    o.set("auto_decisions", c.auto as i64);
    o.set("refusals", c.refusals as i64);
    o.set("refusals_by_class", tally_json(&c.refused));
    o.set("battle_empty", c.battle_empty as i64);
    o.set("refusal_fallbacks", c.refusal_fallbacks as i64);
    o.set("player_fallbacks", all.fallbacks as i64);
    o.set("anomalies", c.anomalies.unexplained() as i64);
    o.set("anomalies_explained", (c.anomalies.total() - c.anomalies.unexplained()) as i64);
    o.set("anomalies_by_kind", tally_json(&c.anomalies));
    o.set("checked_decisions", c.checked as i64);
    o.set("check_a_disagreements", c.agree.unexplained() as i64);
    o.set("check_a_explained", (c.agree.total() - c.agree.unexplained()) as i64);
    o.set("check_a_by_field", tally_json(&c.agree));
    o.set("check_b_agreement_rate", if c.checked > 0 { c.legal_equal as f64 / c.checked as f64 } else { 0.0 });
    o.set("check_b_battle_forbids", c.battle_forbids as i64);
    o.set("check_b_battle_allows_more", c.battle_allows_more as i64);
    o.set("check_b_classes", tally_json(&c.legal));
    o.set("check_d_rebuilt", c.rebuilt as i64);
    o.set("check_d_equal", c.rebuild_equal as i64);
    o.set("steps", c.steps as i64);
    o.set("rows", c.rows as i64);
    o.set("refused_trapped", c.refused_trapped as i64);
    o.set("refused_disabled", c.refused_disabled as i64);
    o.set("labels_checked", c.labels_checked as i64);
    o.set("prior0_actions", c.prior0.unexplained() as i64);
    o.set("prior0_by_kind", tally_json(&c.prior0));
    o.set("truth_errors", c.truth.unexplained() as i64);
    o.set("truth_by_kind", tally_json(&c.truth));
    o.set("hidden_games", c.hidden_games as i64);
    o.set("hidden_steps", c.hidden_steps as i64);
    o.set("hidden_differences", c.hidden.unexplained() as i64);
    o.set("hidden_by_kind", tally_json(&c.hidden));
    o.set("replace_before_rows", a.replace_before_rows);
    o.set("replace_asked", c.replace_asked as i64);
    o.set("stale_replace", c.stale_replace as i64);
    o.set("stale_events", c.stale_events as i64);
    o.set("stale_checked", c.stale_checked as i64);
    o.set("stale_unsnapped", c.stale_unsnapped as i64);
    o.set("replace_twins", c.replace_twins as i64);
    o.set("replace_twins_varied", c.replace_twins_varied as i64);
    o.set("errors", Json::Arr(all.errors.iter().take(20).map(|s| Json::from(s.as_str())).collect()));
    for (i, k) in ["a", "b"].iter().enumerate() {
        o.set(&format!("{}_asked", k), all.asked[i] as i64);
        o.set(&format!("{}_voluntary_switches", k), all.vol_switches[i] as i64);
        let mut f = Json::obj();
        for (name, v) in &all.fired[i] {
            f.set(name, *v as i64);
        }
        o.set(&format!("{}_fired", k), f);
    }
    o.set("wall_s", wall);
    o.set("battles_per_s", games as f64 / wall);
    o.set("decisions_per_s", all.decisions as f64 / wall);
    o.set("time_capped", all.capped);
    let line = o.to_string();
    println!("{}", line);
    eprintln!(
        "{} vs {}: {} games, A {:.1}% (A seat0 {:.1}%, A seat1 {:.1}%), turns mean {:.1} max {}, truncated {}, refusals {}, anomalies {}, check a {}, check b {:.4}, rebuild {}/{}, prior0 {}, truth {}, hidden {} ({} games, {} steps), refused trapped {}, steps {}, {:.0} battles/s, {:.0} decisions/s{}",
        a.a,
        a.b,
        games,
        100.0 * a_wins as f64 / games.max(1) as f64,
        100.0 * all.by_assign[0][0] as f64 / all.by_assign[0][4].max(1) as f64,
        100.0 * all.by_assign[1][0] as f64 / all.by_assign[1][4].max(1) as f64,
        all.turns.iter().map(|&t| t as f64).sum::<f64>() / n,
        all.turns.iter().copied().max().unwrap_or(0),
        all.by_assign[0][3] + all.by_assign[1][3],
        c.refusals,
        c.anomalies.unexplained(),
        c.agree.unexplained(),
        if c.checked > 0 { c.legal_equal as f64 / c.checked as f64 } else { 0.0 },
        c.rebuild_equal,
        c.rebuilt,
        c.prior0.unexplained(),
        c.truth.unexplained(),
        c.hidden.unexplained(),
        c.hidden_games,
        c.hidden_steps,
        c.refused_trapped,
        c.steps,
        games as f64 / wall,
        all.decisions as f64 / wall,
        if all.capped { " (time capped)" } else { "" }
    );
    if a.replace_before_rows {
        eprintln!(
            "replace before rows: {} of {} replace requests stale, {:.1} events unseen each, check a at the last turn request {} (no State yet {}), replays {} ({} varied)",
            c.stale_replace,
            c.replace_asked,
            c.stale_events as f64 / c.stale_replace.max(1) as f64,
            c.stale_checked,
            c.stale_unsnapped,
            c.replace_twins,
            c.replace_twins_varied
        );
    }
    let append = |path: &Option<String>, lines: &[String]| {
        if let Some(p) = path {
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).expect("open");
            for l in lines {
                writeln!(f, "{}", l).unwrap();
            }
        }
    };
    append(&a.out, &[line]);
    append(&a.games_out, &all.games_lines);
    append(&a.events_out, &all.events_lines);
}
