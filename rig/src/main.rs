//! `rig`: JSON-lines protocol on stdin/stdout (tools/rig/CONTRACT.md), one reply line per request line.
//! `--engine sim` is accepted for older command lines; the resolver is the only engine.

use std::io::{BufRead, Write};

fn main() {
    let mut rig = rig::protocol::Rig::default();
    let args: Vec<String> = std::env::args().collect();
    rig.rows = args.iter().any(|a| a == "--rows");
    if let Some(i) = args.iter().position(|a| a == "--engine") {
        let e = args.get(i + 1).map(String::as_str);
        if e != Some("sim") {
            eprintln!("rig: --engine takes sim, not {:?}", e);
            std::process::exit(2);
        }
    }
    cg_engine::sim::abilities::COVERAGE.store(true, std::sync::atomic::Ordering::Relaxed);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = rig.handle_line(&line);
        if writeln!(out, "{}", reply).and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}
