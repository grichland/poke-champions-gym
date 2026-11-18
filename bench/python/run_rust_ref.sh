# Rebuilds and reruns the Rust-only env_play for comparison with the Python figures (Git Bash).
# Use: bash bench/python/run_rust_ref.sh OUT_JSONL [GAMES] [SECS]
set -e
_repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$_repo"
source tools/build/env.sh
cargo build --offline --release -p bench --bin env_play
target/release/env_play.exe tools/rig/data/teams.json --games "${2:-256}" --secs "${3:-15}" --enum-secs 0 --out "$1"
