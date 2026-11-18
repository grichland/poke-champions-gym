# Builds the champions_gym extension and places it in python/champions_gym/ (Git Bash).
# Use: bash tools/build/python-ext.sh [PYTHON]
# PYTHON defaults to $PYO3_PYTHON, then the Program Files Python 3.12. The extension is built for that
# interpreter's version. Afterwards: PYTHONPATH=python "$PYTHON" -c "import champions_gym"
set -e
_repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$_repo"
PYO3_PYTHON="${1:-${PYO3_PYTHON:-/c/Program Files/Python312/python.exe}}"
export PYO3_PYTHON
if [ ! -x "$PYO3_PYTHON" ]; then
    echo "python-ext.sh: no Python at $PYO3_PYTHON" >&2
    exit 1
fi
source tools/build/env.sh
cargo build --offline --release -p cg-bindings
_suffix="$("$PYO3_PYTHON" -c "import sysconfig; print(sysconfig.get_config_var('EXT_SUFFIX') or '.pyd')")"
_dest="python/champions_gym/_cg$_suffix"
cp -f target/release/_cg.dll "$_dest"
echo "python-ext.sh: wrote $_dest"
