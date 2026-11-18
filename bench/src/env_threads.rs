//! BatchEnv speed on the worker pool, Rust only: random actions from the action mask, with and without the
//! observation at every decision. For each N the thread counts and the two modes run in alternating
//! rounds on one environment (`set_threads` between rounds), `secs` seconds per configuration in all. The
//! machine's CPU load and this process's share are recorded for each configuration.
//!
//! Usage: env_threads [--n 256,4096,16384] [--threads 1,8,16,24] [--secs 30] [--rounds 3] [--seed 1]
//!        [--teams tools/rig/data/teams.json] [--out FILE.jsonl] [--label L] [--profile]
//! `--profile` times the step's sections (a little slower) and adds them to each line.

use battle::json::Json;
use env::observe::{ObsBatch, ObsOptions, ROWS_LEN, ROW_IDS_LEN};
use env::teams::{TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, N_ALL_ACTIONS, PREVIEW0};
use obs::{FIELD, LAST, LAST_IDS, MASK, OPP_ACTIONS};
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    lo: u32,
    hi: u32,
}

impl FileTime {
    fn v(self) -> u64 {
        ((self.hi as u64) << 32) | self.lo as u64
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    fn GetProcessTimes(p: isize, c: *mut FileTime, e: *mut FileTime, k: *mut FileTime, u: *mut FileTime) -> i32;
    fn GetCurrentProcess() -> isize;
}

/// (machine busy, machine total, this process's CPU), in 100 ns units.
#[cfg(windows)]
fn cpu() -> (u64, u64, u64) {
    let (mut i, mut k, mut u) = (FileTime::default(), FileTime::default(), FileTime::default());
    let (mut c, mut e, mut pk, mut pu) = (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
    unsafe {
        GetSystemTimes(&mut i, &mut k, &mut u);
        GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut pk, &mut pu);
    }
    let total = k.v() + u.v();
    (total - i.v(), total, pk.v() + pu.v())
}

#[cfg(not(windows))]
fn cpu() -> (u64, u64, u64) {
    (0, 0, 0)
}

fn arg(v: &[String], k: &str) -> Option<String> {
    v.iter().position(|a| a == k).and_then(|i| v.get(i + 1)).cloned()
}

fn list(s: &str) -> Vec<usize> {
    s.split(',').filter_map(|x| x.trim().parse().ok()).collect()
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
    prior: Vec<f32>,
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
            prior: vec![0.0; n * OPP_ACTIONS],
        }
    }
    fn batch(&mut self) -> ObsBatch<'_> {
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
            opp_prior: &mut self.prior,
        }
    }
}

/// xorshift64*
struct R(u64);

impl R {
    fn below(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) * n as u64 >> 32) as u32
    }
}

/// One uniformly random action per game from a mask of 71 per game.
fn choose(mask: &[u8], acts: &mut [u16], rng: &mut R) {
    for (g, a) in acts.iter_mut().enumerate() {
        let m = &mask[g * N_ALL_ACTIONS..(g + 1) * N_ALL_ACTIONS];
        let k = m.iter().filter(|x| **x == 1).count() as u32;
        let mut pick = rng.below(k.max(1));
        for (i, x) in m.iter().enumerate() {
            if *x == 1 {
                if pick == 0 {
                    *a = if i >= PREVIEW0 { (i - PREVIEW0) as u16 } else { i as u16 };
                    break;
                }
                pick -= 1;
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Tot {
    decisions: u64,
    games: u64,
    wall: f64,
    obs: f64,
    act: f64,
    busy: u64,
    total: u64,
    proc_: u64,
}

fn main() {
    let v: Vec<String> = std::env::args().collect();
    let ns = list(&arg(&v, "--n").unwrap_or_else(|| "256,4096,16384".into()));
    let ts = list(&arg(&v, "--threads").unwrap_or_else(|| "1,8,16,24".into()));
    let secs: f64 = arg(&v, "--secs").and_then(|x| x.parse().ok()).unwrap_or(30.0);
    let rounds: usize = arg(&v, "--rounds").and_then(|x| x.parse().ok()).unwrap_or(3);
    let seed: u64 = arg(&v, "--seed").and_then(|x| x.parse().ok()).unwrap_or(1);
    let teams = arg(&v, "--teams").unwrap_or_else(|| "tools/rig/data/teams.json".into());
    let out = arg(&v, "--out");
    let label = arg(&v, "--label").unwrap_or_default();
    let profile = v.iter().any(|a| a == "--profile");
    let pool = Arc::new(TeamPool::open(&teams).expect("teams"));
    let cores = std::thread::available_parallelism().map_or(0, |x| x.get());
    for &n in &ns {
        let cfg = EnvConfig { games: n, seed, team_mode: TeamMode::AsGiven, ..EnvConfig::default() };
        let mut env = BatchEnv::new(pool.clone(), cfg);
        env.set_profile(profile);
        let mut bufs = Bufs::new(n);
        let mut mask = vec![0u8; n * N_ALL_ACTIONS];
        let mut acts = vec![0u16; n];
        let mut rng = R(seed | 1);
        let configs: Vec<(usize, bool)> = ts.iter().flat_map(|&t| [(t, false), (t, true)]).collect();
        let mut tot = vec![Tot::default(); configs.len()];
        let mut prof = vec![[0u64; env::batch::PROF_SECTIONS]; configs.len()];
        let per = secs / rounds as f64;
        for _ in 0..rounds {
            for (ci, &(t, with_obs)) in configs.iter().enumerate() {
                env.set_threads(t);
                let p0 = env.counters.prof;
                let (d0, g0) = (env.decisions, env.games_done);
                let (b0, a0, c0) = cpu();
                let t0 = Instant::now();
                let (mut to, mut ta) = (0.0, 0.0);
                while t0.elapsed().as_secs_f64() < per {
                    let x = Instant::now();
                    if with_obs {
                        env.observe_batch_with(bufs.batch(), ObsOptions { threads: t, damage: true }).unwrap();
                        to += x.elapsed().as_secs_f64();
                        choose(&bufs.actions, &mut acts, &mut rng);
                    } else {
                        for g in 0..n {
                            env.action_mask(g, &mut mask[g * N_ALL_ACTIONS..(g + 1) * N_ALL_ACTIONS]);
                        }
                        choose(&mask, &mut acts, &mut rng);
                    }
                    let y = Instant::now();
                    env.act(&acts).unwrap();
                    ta += y.elapsed().as_secs_f64();
                }
                let wall = t0.elapsed().as_secs_f64();
                let (b1, a1, c1) = cpu();
                let k = &mut tot[ci];
                k.decisions += env.decisions - d0;
                k.games += env.games_done - g0;
                k.wall += wall;
                k.obs += to;
                k.act += ta;
                k.busy += b1 - b0;
                k.total += a1 - a0;
                k.proc_ += c1 - c0;
                for i in 0..env::batch::PROF_SECTIONS {
                    prof[ci][i] += env.counters.prof[i] - p0[i];
                }
            }
        }
        for (ci, &(t, with_obs)) in configs.iter().enumerate() {
            let k = tot[ci];
            let mut o = Json::obj();
            o.set("bench", "env_threads");
            o.set("where", "rust");
            o.set("label", label.as_str());
            o.set("n", n as i64);
            o.set("threads", t as i64);
            o.set("observe", with_obs);
            o.set("profile", profile);
            o.set("rounds", rounds as i64);
            o.set("seconds", k.wall);
            o.set("decisions", k.decisions as i64);
            o.set("games", k.games as i64);
            o.set("decisions_per_s", k.decisions as f64 / k.wall);
            o.set("battles_per_s", k.games as f64 / k.wall);
            o.set("share_act", k.act / k.wall);
            o.set("share_observe", k.obs / k.wall);
            o.set("cores", cores as i64);
            // machine CPU busy over the run, and the part of it that was this process
            let busy = if k.total > 0 { k.busy as f64 / k.total as f64 } else { 0.0 };
            let mine = if k.total > 0 { k.proc_ as f64 / k.total as f64 } else { 0.0 };
            o.set("cpu_busy", busy);
            o.set("cpu_this_process", mine);
            o.set("cpu_other_load", (busy - mine).max(0.0));
            if profile {
                let mut p = Json::obj();
                let d = k.decisions.max(1) as f64;
                for i in 0..env::batch::PROF_SECTIONS {
                    p.set(env::batch::PROF_NAMES[i], prof[ci][i] as f64 / d);
                }
                o.set("ns_per_decision_by_section_summed_over_threads", p);
            }
            let line = o.to_string();
            println!("{}", line);
            if let Some(f) = &out {
                let mut fh = std::fs::OpenOptions::new().create(true).append(true).open(f).expect("out");
                writeln!(fh, "{}", line).unwrap();
            }
        }
    }
}
