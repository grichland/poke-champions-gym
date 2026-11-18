# Source on the laptop (Git Bash) from the repo root: points at the copied toolchain, then the common env.
export RUSTUP_HOME='C:\Users\grich\Code\pcsa\tools\rust\rustup'
export CARGO_HOME='C:\Users\grich\Code\pcsa\tools\rust\cargo'
export LLVM_MINGW=/c/Users/grich/Code/pcsa/tools/rust/llvm-mingw
export PATH="/c/Users/grich/Code/pcsa/tools/rust/cargo/bin:$PATH"
source tools/build/env.sh
