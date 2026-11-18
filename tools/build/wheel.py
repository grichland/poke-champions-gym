"""Assembles the champions_gym wheel from python/champions_gym and the built extension (stdlib only).

Use: python tools/build/wheel.py [OUT_DIR]   (default dist/)
Run tools/build/python-ext.sh first with the same Python: the wheel's tag comes from that interpreter
and the extension's file name must match it. When llvm-strip is on PATH (after `source tools/build/env.sh`)
the extension's debug information is left out of the wheel.
"""

import base64
import hashlib
import os
import shutil
import subprocess
import sys
import sysconfig
import tempfile
import zipfile

NAME = "champions_gym"
VERSION = "0.4.0"
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PKG = os.path.join(REPO, "python", NAME)

METADATA = f"""Metadata-Version: 2.1
Name: champions-gym
Version: {VERSION}
Summary: CG: a Pokémon Champions Ranked Singles simulator (Rust) with a batched Python environment.
License: Private; not licensed for redistribution.
Requires-Python: >=3.12,<3.13
Provides-Extra: numpy
Requires-Dist: numpy; extra == "numpy"
Description-Content-Type: text/plain

Rust engine, Battle built from log events, built-in Players (random, max damage, tower), batched
environment, observation and labels. Without numpy every array is a typed memoryview with the same shape.
See README.md in the source repository.
"""


def tag():
    """cp312-cp312-win_amd64 for the running interpreter."""
    v = f"cp{sys.version_info[0]}{sys.version_info[1]}"
    plat = sysconfig.get_platform().replace("-", "_").replace(".", "_")
    return f"{v}-{v}-{plat}"


def record_line(path, data):
    h = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
    return f"{path},sha256={h},{len(data)}"


def stripped(path):
    """The extension's bytes without debug information, when llvm-strip is available."""
    tool = shutil.which("llvm-strip")
    if tool:
        with tempfile.TemporaryDirectory() as d:
            tmp = os.path.join(d, os.path.basename(path))
            subprocess.run([tool, "--strip-debug", "-o", tmp, path], check=True)
            with open(tmp, "rb") as f:
                return f.read()
    print("wheel.py: llvm-strip not on PATH; the extension keeps its debug information")
    with open(path, "rb") as f:
        return f.read()


def files():
    """(path in the wheel, bytes) of the package: Python sources, data, the extension."""
    ext = sysconfig.get_config_var("EXT_SUFFIX")
    out = []
    found_ext = False
    for root, dirs, names in os.walk(PKG):
        dirs[:] = sorted(d for d in dirs if d != "__pycache__")
        for n in sorted(names):
            full = os.path.join(root, n)
            rel = os.path.relpath(full, os.path.dirname(PKG)).replace(os.sep, "/")
            if n.endswith(".py") or rel.startswith(f"{NAME}/data/"):
                pass
            elif n == "_cg" + ext:
                found_ext = True
                out.append((rel, stripped(full)))
                continue
            else:
                continue
            with open(full, "rb") as f:
                out.append((rel, f.read()))
    if not found_ext:
        sys.exit(f"wheel.py: {PKG}/_cg{ext} not found; run tools/build/python-ext.sh with this Python first")
    return out


def main():
    out_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(REPO, "dist")
    os.makedirs(out_dir, exist_ok=True)
    t = tag()
    whl = os.path.join(out_dir, f"{NAME}-{VERSION}-{t}.whl")
    info = f"{NAME}-{VERSION}.dist-info"
    entries = files()
    with open(os.path.join(REPO, "THIRD_PARTY_NOTICES.md"), "rb") as f:
        notices = f.read()
    entries += [
        (f"{info}/METADATA", METADATA.encode("utf-8")),
        (f"{info}/WHEEL", f"Wheel-Version: 1.0\nGenerator: champions_gym tools/build/wheel.py\nRoot-Is-Purelib: false\nTag: {t}\n".encode()),
        (f"{info}/top_level.txt", f"{NAME}\n".encode()),
        (f"{info}/THIRD_PARTY_NOTICES.md", notices),
    ]
    record = [record_line(p, d) for p, d in entries] + [f"{info}/RECORD,,"]
    entries.append((f"{info}/RECORD", ("\n".join(record) + "\n").encode()))
    with zipfile.ZipFile(whl, "w", zipfile.ZIP_DEFLATED) as z:
        for p, d in entries:
            zi = zipfile.ZipInfo(p, date_time=(2026, 11, 18, 0, 0, 0))
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.external_attr = 0o644 << 16
            z.writestr(zi, d)
    print(f"wheel.py: wrote {whl} ({os.path.getsize(whl)} bytes, {len(entries)} files)")


if __name__ == "__main__":
    main()
