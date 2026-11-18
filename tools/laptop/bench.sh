#!/usr/bin/env bash
# Runs on the laptop from the repo root: build, then the resolver benchmark (sim_play, real teams).
# usage: bench.sh <label> [runs] [battles]; results go to bench/results/<label>.jsonl
set -uo pipefail
label="$1"; runs="${2:-3}"; battles="${3:-100000}"
source tools/laptop/env_laptop.sh || { echo "FAILED env"; exit 1; }
out="bench/results/$label.jsonl"
: > "$out"
git log --oneline -1
cargo build --offline --release -p bench 2>&1 | tail -3 || { echo "FAILED build"; exit 1; }
for i in $(seq "$runs"); do
  ./target/release/sim_play.exe tools/rig/data/teams.json "$battles" 1000 600 --real | tee -a "$out"
done
echo "DONE $label"
