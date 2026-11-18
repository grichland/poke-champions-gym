# Use: `source tools/build/env.sh` from Git Bash, then `cargo build --release`.
# Finds llvm-mingw from $LLVM_MINGW, else from x86_64-w64-mingw32-clang on PATH, else this desktop's copy.
# Puts cargo and llvm-mingw on PATH and sets RUSTFLAGS/RUSTDOCFLAGS to the link shim in target/linkshim.

_repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [ -z "$LLVM_MINGW" ] && command -v x86_64-w64-mingw32-clang >/dev/null 2>&1; then
    LLVM_MINGW="$(cd "$(dirname "$(command -v x86_64-w64-mingw32-clang)")/.." && pwd)"
fi
LLVM_MINGW="${LLVM_MINGW:-/c/Users/user/Code/pokemon_champions/research/pcsa-rl/poke-engine-eval/_dl/llvm-mingw/llvm-mingw-20261020-ucrt-x86_64}"
export LLVM_MINGW

if [ ! -d "$LLVM_MINGW/bin" ]; then
    echo "env.sh: no llvm-mingw at $LLVM_MINGW (set LLVM_MINGW)" >&2
    return 1 2>/dev/null || exit 1
fi

# llvm-mingw has no libgcc / libgcc_eh: stand in compiler-rt builtins and libunwind under those names
_shim="$_repo/target/linkshim"
mkdir -p "$_shim"
_builtins="$(ls "$LLVM_MINGW"/lib/clang/*/lib/windows/libclang_rt.builtins-x86_64.a 2>/dev/null | head -n 1)"
if [ -z "$_builtins" ]; then
    echo "env.sh: libclang_rt.builtins-x86_64.a not found under $LLVM_MINGW" >&2
    return 1 2>/dev/null || exit 1
fi
cp -u "$_builtins" "$_shim/libgcc.a"
cp -u "$LLVM_MINGW/x86_64-w64-mingw32/lib/libunwind.a" "$_shim/libgcc_eh.a"

export PATH="$HOME/.cargo/bin:$LLVM_MINGW/bin:$PATH"
export RUSTFLAGS="-L $(cygpath -w "$_shim")"
export RUSTDOCFLAGS="$RUSTFLAGS"  # doctests link through rustdoc, which ignores RUSTFLAGS
unset _repo _shim _builtins
