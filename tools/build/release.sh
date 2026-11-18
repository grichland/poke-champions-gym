# Builds the wheel for publishing into dist/public/ (Git Bash). Use: bash tools/build/release.sh [PYTHON]
# Builds in target/pub with local paths remapped, so the extension holds no machine paths, and leaves
# python/champions_gym/ and target/release untouched.
set -e
_repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$_repo"
PYO3_PYTHON="${1:-${PYO3_PYTHON:-/c/Program Files/Python312/python.exe}}"
export PYO3_PYTHON
source tools/build/env.sh
_repo="$(pwd)"  # env.sh unsets it
export CARGO_TARGET_DIR="$_repo/target/pub"
export RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$(cygpath -w "$HOME/.cargo")=cargo --remap-path-prefix=$(cygpath -w "$HOME/.rustup")=rustup --remap-path-prefix=$(cygpath -w "$_repo")=cg"
cargo build --offline --release -p cg-bindings
CG_EXT="$CARGO_TARGET_DIR/release/_cg.dll" "$PYO3_PYTHON" tools/build/wheel.py dist/public
