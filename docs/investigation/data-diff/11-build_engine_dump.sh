#!/usr/bin/env bash
# 11-build_engine_dump.sh: copy poke-engine (read-only checkout, commit f7b147f) into work/, add the dump
# program 11-engine_dump.rs as a Cargo example, build it with features champions,bss, write out/engine.json.
# Run from Git Bash. work/ is git-ignored and never committed.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC=/c/Users/user/Code/pokemon_champions/research/pcsa-rl/poke-engine-eval/poke-engine
LLVM=/c/Users/user/Code/pokemon_champions/research/pcsa-rl/poke-engine-eval/_dl/llvm-mingw/llvm-mingw-20261020-ucrt-x86_64
W="$HERE/work"

mkdir -p "$W/engine/examples" "$W/linkshim"
printf '*\n' > "$W/.gitignore"
cp -r "$SRC/Cargo.toml" "$SRC/Cargo.lock" "$SRC/LICENSE" "$SRC/src" "$SRC/data" "$W/engine/"
# drop the [workspace] section (poke-engine-py is not copied)
python - "$W/engine/Cargo.toml" <<'EOF'
import sys
p = sys.argv[1]; s = open(p).read()
open(p, 'w').write(s[:s.index('[workspace]')] if '[workspace]' in s else s)
EOF
cp "$HERE/11-engine_dump.rs" "$W/engine/examples/dump.rs"

# the x86_64-pc-windows-gnu toolchain links through llvm-mingw, which has no libgcc / libgcc_eh:
# stand in compiler-rt builtins and libunwind under those names
cp "$LLVM"/lib/clang/*/lib/windows/libclang_rt.builtins-x86_64.a "$W/linkshim/libgcc.a"
cp "$LLVM/x86_64-w64-mingw32/lib/libunwind.a" "$W/linkshim/libgcc_eh.a"

export PATH="$HOME/.cargo/bin:$LLVM/bin:$PATH"
export RUSTFLAGS="-L $(cygpath -w "$W/linkshim")"
cd "$W/engine"
cargo build --offline --release --example dump --no-default-features --features champions,bss
./target/release/examples/dump.exe > "$HERE/out/engine.json"
echo "wrote $HERE/out/engine.json"
