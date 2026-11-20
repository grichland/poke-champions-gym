//! The observation inside the environment.
//!
//! Check mode: `games` games of two named Players (A in seat 0 in even games, seat 1 in odd), N at a
//! time, each choosing from its own Battle. At every decision `observe_batch` fills all N games and each
//! entry is checked: finite, inside each feature's range, ids inside their vocabulary, the 11-mask equal
//! to `battle::legal` of the seat's Battle (0 at preview). Run twice with the same seed: the hash of every
//! tensor must agree. Prints a summary with, per feature id, how many observations had it non-zero.
//!
//! Speed mode (`--speed`): random actions from the action mask, N games, alternating rounds of the loop
//! with and without `observe_batch`; the time inside `observe_batch` gives the cost per observation.
//!
//! Usage: obs_env --a tower --b max_damage --games 2000 [--n 64] [--seed 1] [--out FILE] [--games-out FILE]
//!        obs_env --speed --n 256 [--secs 30] [--rounds 6] [--threads 1] [--out FILE]

use battle::{legal, ReqKind};
use env::batch::preview_action;
use env::observe::{ObsBatch, ObsOptions, ROWS_LEN, ROW_IDS_LEN};
use env::teams::{mix, TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind, N_ALL_ACTIONS};
use obs::layout::*;
use obs::{vocab, Block, FIELD, LAST, LAST_IDS, MASK};
use env::builtin::by_name;
use players::Rng;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

fn arg(v: &[String], k: &str) -> Option<String> {
    v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned()
}

struct Bufs {
    rows: Vec<f32>,
    row_ids: Vec<i32>,
    field: Vec<f32>,
    last: Vec<f32>,
    last_ids: Vec<i32>,
    mask: Vec<u8>,
    actions: Vec<u8>,
    seat: Vec<u8>,
    kind: Vec<u8>,
    decision: Vec<i64>,
    opp_prior: Vec<f32>,
}

impl Bufs {
    fn new(n: usize) -> Bufs {
        Bufs {
            rows: vec![0.0; n * ROWS_LEN],
            row_ids: vec![0; n * ROW_IDS_LEN],
            field: vec![0.0; n * FIELD],
            last: vec![0.0; n * LAST],
            last_ids: vec![0; n * LAST_IDS],
            mask: vec![0; n * MASK],
            actions: vec![0; n * N_ALL_ACTIONS],
            seat: vec![0; n],
            kind: vec![0; n],
            decision: vec![0; n],
            opp_prior: vec![0.0; n * 13],
        }
    }
    fn out(&mut self) -> ObsBatch<'_> {
        ObsBatch {
            rows: &mut self.rows,
            row_ids: &mut self.row_ids,
            field: &mut self.field,
            last: &mut self.last,
            last_ids: &mut self.last_ids,
            mask: &mut self.mask,
            actions: &mut self.actions,
            seat: &mut self.seat,
            kind: &mut self.kind,
            decision: &mut self.decision,
            opp_prior: &mut self.opp_prior,
        }
    }
}

/// Feature index of each element of a block's last axis.
fn owners(block: Block, len: usize) -> Vec<usize> {
    let mut out = vec![usize::MAX; len];
    for (k, f) in FEATURES.iter().enumerate().filter(|(_, f)| f.block == block) {
        let starts: Vec<usize> = if f.slot { (0..4).map(|s| MOVE_BASE + MOVE_STRIDE * s + f.offset).collect() } else { vec![f.offset] };
        let span = if f.slot { f.size } else { f.size * f.repeats };
        for s in starts {
            for i in s..s + span {
                out[i] = k;
            }
        }
    }
    out
}

fn vocab_size(id: &str) -> usize {
    match id {
        "F1-3" | "L2-2.1" | "L2-2.2" => vocab::SPECIES_VOCAB,
        "F2-6" => vocab::ITEM_VOCAB,
        "F2-7" | "F8-10" => vocab::ABILITY_VOCAB,
        _ => vocab::MOVE_VOCAB,
    }
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}

fn bytes<T>(x: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(x.as_ptr() as *const u8, std::mem::size_of_val(x)) }
}

#[derive(Default)]
struct Check {
    observations: u64,
    previews: u64,
    /// per feature: observations with it non-zero; and at preview
    nonzero: Vec<u64>,
    nonzero_preview: Vec<u64>,
    bad: Vec<String>,
    bad_count: u64,
    /// 11-mask entries that differ from the turn part of the action mask (the environment's fallbacks)
    mask_vs_actions: u64,
    fallbacks: u64,
    hash: u64,
    games: u64,
    wins_a: u64,
    turns: u64,
    games_lines: Vec<String>,
    decisions: u64,
    /// labels: valid, with an opposing action, forced, shown, refused
    labels: [u64; 5],
    /// opposing actions taken that had opp_prior 0, by class (move, mega, switch)
    mask_violations: [u64; 3],
    violation_examples: Vec<String>,
    /// observations with each opp_prior entry non-zero
    prior_nonzero: [u64; 13],
}

fn run_check(pool: Arc<TeamPool>, a: &str, b: &str, games: u64, n: usize, seed: u64) -> Check {
    let cfg = EnvConfig { games: n, seed, team_mode: team_mode(), max_steps: 1000, checks: false, keep_records: false, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(pool, cfg);
    let mut ps: Vec<[Box<dyn players::Player>; 2]> = (0..n).map(|g| if g % 2 == 0 { [by_name(a).unwrap(), by_name(b).unwrap()] } else { [by_name(b).unwrap(), by_name(a).unwrap()] }).collect();
    let mut rngs: Vec<[Rng; 2]> = (0..n).map(|g| [Rng::new(mix(seed, 2 * g as u64)), Rng::new(mix(seed, 2 * g as u64 + 1))]).collect();
    let (row_own, field_own, last_own) = (owners(Block::Row, ROW_LEN), owners(Block::Field, FIELD_LEN), owners(Block::Last, LAST_LEN));
    let rowid_own = owners(Block::RowIds, ROWIDS_LEN);
    let lastid_own = owners(Block::LastIds, LASTIDS_LEN);
    let mask_feat = FEATURES.iter().position(|f| f.block == Block::Mask).unwrap();
    let mut c = Check { nonzero: vec![0; FEATURES.len()], nonzero_preview: vec![0; FEATURES.len()], hash: 0xcbf29ce484222325, ..Default::default() };
    let mut buf = Bufs::new(n);
    let mut acts = vec![0u16; n];
    let mut legal_v = Vec::new();
    let mut hit = vec![false; FEATURES.len()];
    while c.games < games {
        env.observe_batch(buf.out()).unwrap();
        fnv(&mut c.hash, bytes(&buf.rows));
        fnv(&mut c.hash, bytes(&buf.row_ids));
        fnv(&mut c.hash, bytes(&buf.field));
        fnv(&mut c.hash, bytes(&buf.last));
        fnv(&mut c.hash, bytes(&buf.last_ids));
        fnv(&mut c.hash, &buf.mask);
        fnv(&mut c.hash, &buf.actions);
        fnv(&mut c.hash, bytes(&buf.opp_prior));
        for g in 0..n {
            let p = env.pending(g);
            let seat = p.seat as usize;
            let bt = env.battle(g, seat);
            let preview = p.kind == RequestKind::Preview;
            let rk = if p.kind == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
            c.observations += 1;
            c.previews += preview as u64;
            hit.iter_mut().for_each(|x| *x = false);
            let bad = |what: String, c: &mut Check| {
                c.bad_count += 1;
                if c.bad.len() < 20 {
                    c.bad.push(what);
                }
            };
            let rows = &buf.rows[g * ROWS_LEN..(g + 1) * ROWS_LEN];
            for (i, x) in rows.iter().enumerate() {
                let k = row_own[i % ROW_LEN];
                let f = &FEATURES[k];
                if !(x.is_finite() && *x >= f.lo && *x <= f.hi) {
                    bad(format!("{} row {} = {}", f.id, i / ROW_LEN, x), &mut c);
                }
                if *x != 0.0 {
                    hit[k] = true;
                }
            }
            for (vals, own) in [(&buf.field[g * FIELD..(g + 1) * FIELD], &field_own), (&buf.last[g * LAST..(g + 1) * LAST], &last_own)] {
                for (i, x) in vals.iter().enumerate() {
                    let f = &FEATURES[own[i]];
                    if !(x.is_finite() && *x >= f.lo && *x <= f.hi) {
                        bad(format!("{} = {}", f.id, x), &mut c);
                    }
                    if *x != 0.0 {
                        hit[own[i]] = true;
                    }
                }
            }
            for (ids, own) in [(&buf.row_ids[g * ROW_IDS_LEN..(g + 1) * ROW_IDS_LEN], &rowid_own), (&buf.last_ids[g * LAST_IDS..(g + 1) * LAST_IDS], &lastid_own)] {
                let w = own.len();
                for (i, x) in ids.iter().enumerate() {
                    let f = &FEATURES[own[i % w]];
                    if *x < 0 || *x as usize >= vocab_size(f.id) {
                        bad(format!("{} = {} outside vocab", f.id, x), &mut c);
                    }
                    if *x != 0 {
                        hit[own[i % w]] = true;
                    }
                }
            }
            let m = &buf.mask[g * MASK..(g + 1) * MASK];
            let want: Vec<bool> = if preview { vec![false; MASK] } else { legal(bt, rk).mask.to_vec() };
            if m.iter().map(|&x| x == 1).collect::<Vec<_>>() != want {
                bad(format!("mask {:?} vs legal {:?}", m, want), &mut c);
            }
            if m.iter().any(|&x| x == 1) {
                hit[mask_feat] = true;
            }
            let pr = &buf.opp_prior[g * 13..(g + 1) * 13];
            if !pr.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x)) {
                bad(format!("opp_prior {:?}", pr), &mut c);
            }
            for (k, x) in pr.iter().enumerate() {
                c.prior_nonzero[k] += (*x != 0.0) as u64;
            }
            if !preview {
                let am = &buf.actions[g * N_ALL_ACTIONS..g * N_ALL_ACTIONS + MASK];
                c.mask_vs_actions += (am != m) as u64;
            }
            for (k, h) in hit.iter().enumerate() {
                if *h {
                    c.nonzero[k] += 1;
                    if preview {
                        c.nonzero_preview[k] += 1;
                    }
                }
            }
            // the Player's action
            let pl = &mut ps[g][seat];
            let rng = &mut rngs[g][seat];
            let act = if preview { preview_action(pl.teampreview(bt, rng)).unwrap_or(0) } else { pl.choose_move(bt, rk, rng) as u16 };
            env.legal_actions(g, &mut legal_v);
            acts[g] = if legal_v.contains(&act) {
                act
            } else {
                c.fallbacks += 1;
                legal_v[0]
            };
        }
        env.act(&acts).unwrap();
        c.decisions += n as u64;
        for (g, ls) in env.labels().iter().enumerate() {
            for l in ls.iter().filter(|l| l.valid) {
                c.labels[0] += 1;
                c.labels[1] += (l.opp_action >= 0) as u64;
                c.labels[2] += l.opp_forced as u64;
                c.labels[3] += l.opp_shown as u64;
                c.labels[4] += l.refused as u64;
                if l.opp_action >= 0 && l.opp_prior[l.opp_action as usize] == 0.0 {
                    let k = if l.opp_action < 4 { 0 } else if l.opp_action < 8 { 1 } else { 2 };
                    c.mask_violations[k] += 1;
                    if c.violation_examples.len() < 10 {
                        c.violation_examples.push(format!("game {} decision {} action {} move {} forced {} prior {:?}", g, l.decision, l.opp_action, cg_engine::sim::data::MoveId(l.opp_move as u16).id(), l.opp_forced, l.opp_prior));
                    }
                }
            }
        }
        for (g, r) in env.results().iter().enumerate() {
            if r.done && c.games < games {
                c.games += 1;
                c.turns += r.turns as u64;
                let a_seat = (g % 2) as u8;
                c.wins_a += (r.winner == Some(a_seat) && !r.truncated) as u64;
                let ids = env.slot(g).team_ids;
                c.games_lines.push(format!(
                    "{{\"seed\":{},\"game\":{},\"episode\":{},\"teams\":[{},{}],\"seat0\":\"{}\",\"seat1\":\"{}\",\"winner\":{},\"turns\":{},\"truncated\":{}}}",
                    seed,
                    g,
                    r.episode,
                    ids[0],
                    ids[1],
                    if g % 2 == 0 { a } else { b },
                    if g % 2 == 0 { b } else { a },
                    r.winner.map_or("null".to_string(), |w| w.to_string()),
                    r.turns,
                    r.truncated
                ));
            }
        }
    }
    c
}

fn append(path: &Option<String>, lines: &[String]) {
    if let Some(p) = path {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).expect("out");
        for l in lines {
            writeln!(f, "{}", l).unwrap();
        }
    }
}

fn check_main(v: &[String]) {
    let a = arg(v, "--a").unwrap_or_else(|| "tower".into());
    let b = arg(v, "--b").unwrap_or_else(|| "max_damage".into());
    let games: u64 = arg(v, "--games").and_then(|x| x.parse().ok()).unwrap_or(2000);
    let n: usize = arg(v, "--n").and_then(|x| x.parse().ok()).unwrap_or(64);
    let seed: u64 = arg(v, "--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let teams = arg(v, "--teams").unwrap_or_else(|| "tools/rig/data/teams.json".into());
    let pool = Arc::new(TeamPool::open(&teams).expect("teams"));
    let t0 = Instant::now();
    let c = run_check(pool.clone(), &a, &b, games, n, seed);
    let secs = t0.elapsed().as_secs_f64();
    let again = run_check(pool, &a, &b, games, n, seed);
    let never: Vec<&str> = FEATURES.iter().zip(&c.nonzero).filter(|(f, k)| **k == 0 && f.block != Block::Table).map(|(f, _)| f.id).collect();
    let counts: Vec<String> = FEATURES.iter().zip(&c.nonzero).filter(|(f, _)| f.block != Block::Table).map(|(f, k)| format!("\"{}\":{}", f.id, k)).collect();
    let pcounts: Vec<String> = FEATURES.iter().zip(&c.nonzero_preview).filter(|(_, k)| **k > 0).map(|(f, k)| format!("\"{}\":{}", f.id, k)).collect();
    let line = format!(
        "{{\"bench\":\"obs_env_check\",\"a\":\"{}\",\"b\":\"{}\",\"games\":{},\"n\":{},\"seed\":{},\"decisions\":{},\"observations\":{},\"previews\":{},\"bad\":{},\"bad_examples\":{:?},\"mask_vs_action_mask\":{},\"player_fallbacks\":{},\"a_wins\":{},\"mean_turns\":{:.2},\"hash\":\"{:016x}\",\"rerun_hash\":\"{:016x}\",\"deterministic\":{},\"seconds\":{:.2},\"labels\":{{\"valid\":{},\"with_opp_action\":{},\"opp_forced\":{},\"opp_shown\":{},\"refused\":{}}},\"opp_prior_zero_on_taken\":{{\"move\":{},\"mega\":{},\"switch\":{}}},\"opp_prior_nonzero\":{:?},\"violation_examples\":{:?},\"never_nonzero\":{:?},\"nonzero\":{{{}}},\"nonzero_at_preview\":{{{}}}}}",
        a,
        b,
        c.games,
        n,
        seed,
        c.decisions,
        c.observations,
        c.previews,
        c.bad_count,
        c.bad,
        c.mask_vs_actions,
        c.fallbacks,
        c.wins_a,
        c.turns as f64 / c.games.max(1) as f64,
        c.hash,
        again.hash,
        c.hash == again.hash && c.games_lines == again.games_lines,
        secs,
        c.labels[0],
        c.labels[1],
        c.labels[2],
        c.labels[3],
        c.labels[4],
        c.mask_violations[0],
        c.mask_violations[1],
        c.mask_violations[2],
        c.prior_nonzero,
        c.violation_examples,
        never,
        counts.join(","),
        pcounts.join(",")
    );
    println!("{}", line);
    append(&arg(v, "--out"), &[line]);
    append(&arg(v, "--games-out"), &c.games_lines);
}

fn speed_main(v: &[String]) {
    let n: usize = arg(v, "--n").and_then(|x| x.parse().ok()).unwrap_or(256);
    let secs: f64 = arg(v, "--secs").and_then(|x| x.parse().ok()).unwrap_or(30.0);
    let rounds: usize = arg(v, "--rounds").and_then(|x| x.parse().ok()).unwrap_or(6);
    let threads: usize = arg(v, "--threads").and_then(|x| x.parse().ok()).unwrap_or(1);
    let seed: u64 = arg(v, "--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let teams = arg(v, "--teams").unwrap_or_else(|| "tools/rig/data/teams.json".into());
    let pool = Arc::new(TeamPool::open(&teams).expect("teams"));
    let cfg = EnvConfig { games: n, seed, team_mode: team_mode(), max_steps: 1000, ..Default::default() };
    let mut env = BatchEnv::new(pool, cfg);
    let mut buf = Bufs::new(n);
    let mut rng = Rng::new(seed);
    let mut acts = vec![0u16; n];
    let mut row = [0u8; N_ALL_ACTIONS];
    // per mode: decisions, wall seconds, seconds inside observe_batch, games finished
    let mut tot = [[0f64; 4]; 2];
    let per_round = secs / rounds as f64;
    let opt = ObsOptions { threads, damage: true };
    for r in 0..rounds * 2 {
        let with = r % 2 == 1;
        let t0 = Instant::now();
        let d0 = env.decisions;
        let g0 = env.games_done;
        let mut in_obs = 0f64;
        while t0.elapsed().as_secs_f64() < per_round {
            if with {
                let t = Instant::now();
                env.observe_batch_with(buf.out(), opt).unwrap();
                in_obs += t.elapsed().as_secs_f64();
            }
            for g in 0..n {
                env.action_mask(g, &mut row);
                let legal: Vec<usize> = (0..N_ALL_ACTIONS).filter(|&a| row[a] == 1).collect();
                let a = legal[rng.below(legal.len())];
                acts[g] = if a >= env::PREVIEW0 { (a - env::PREVIEW0) as u16 } else { a as u16 };
            }
            env.act(&acts).unwrap();
        }
        let k = with as usize;
        tot[k][0] += (env.decisions - d0) as f64;
        tot[k][1] += t0.elapsed().as_secs_f64();
        tot[k][2] += in_obs;
        tot[k][3] += (env.games_done - g0) as f64;
    }
    let mut lines = vec![];
    for (k, name) in [(0, "no_obs"), (1, "obs")] {
        let [d, w, o, gm] = tot[k];
        lines.push(format!(
            "{{\"bench\":\"obs_env_speed\",\"where\":\"rust\",\"mode\":\"{}\",\"n\":{},\"threads\":{},\"rounds\":{},\"alternating\":true,\"seconds\":{:.2},\"decisions\":{},\"decisions_per_s\":{:.0},\"battles_per_s\":{:.1},\"us_per_observation\":{}}}",
            name,
            n,
            threads,
            rounds,
            w,
            d,
            d / w,
            gm / w,
            if k == 1 { format!("{:.3}", o * 1e6 / d) } else { "null".into() }
        ));
    }
    for l in &lines {
        println!("{}", l);
    }
    append(&arg(v, "--out"), &lines);
}

fn main() {
    let v: Vec<String> = std::env::args().collect();
    if v.iter().any(|a| a == "--speed") {
        speed_main(&v);
    } else {
        check_main(&v);
    }
}

/// The real teams as written; `--stage3` for No Ability, no item and stage-3 moves.
fn team_mode() -> TeamMode {
    if std::env::args().any(|a| a == "--stage3") {
        TeamMode::Stage3
    } else {
        TeamMode::AsGiven
    }
}
