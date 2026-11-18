"""Decisions/s and battles/s of champions_gym.BatchEnv through Python on the worker pool: random legal actions
chosen in numpy from the action mask, with and without the observation at every decision. For each N the
thread counts and the two modes run in alternating rounds on one environment (`env.threads = t` between
rounds), `secs` seconds per configuration in all. The machine's CPU load and this process's share are
recorded, and the time inside each call (observe, the numpy choice, act).

Usage: PYTHONPATH=python python bench/python/pyenv_threads.py TEAMS_JSON [--games 256,4096,16384]
       [--threads 1,8,16,24] [--secs 30] [--rounds 3] [--out FILE] [--label L]
"""

import argparse
import ctypes
import json
import os
import platform
import time

import numpy as np

import champions_gym as cg


class _FT(ctypes.Structure):
    _fields_ = [("lo", ctypes.c_uint32), ("hi", ctypes.c_uint32)]

    def v(self):
        return (self.hi << 32) | self.lo


def cpu():
    """(machine busy, machine total, this process's CPU), in 100 ns units (Windows)."""
    if os.name != "nt":
        return 0, 0, 0
    k32 = ctypes.windll.kernel32
    k32.GetCurrentProcess.restype = ctypes.c_void_p
    k32.GetProcessTimes.argtypes = [ctypes.c_void_p] + [ctypes.c_void_p] * 4
    i, k, u = _FT(), _FT(), _FT()
    k32.GetSystemTimes(ctypes.byref(i), ctypes.byref(k), ctypes.byref(u))
    c, e, pk, pu = _FT(), _FT(), _FT(), _FT()
    k32.GetProcessTimes(k32.GetCurrentProcess(), ctypes.byref(c), ctypes.byref(e), ctypes.byref(pk), ctypes.byref(pu))
    total = k.v() + u.v()
    return total - i.v(), total, pk.v() + pu.v()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("teams")
    ap.add_argument("--games", default="256,4096,16384")
    ap.add_argument("--threads", default="1,8,16,24")
    ap.add_argument("--secs", type=float, default=30.0)
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out")
    ap.add_argument("--label", default="")
    a = ap.parse_args()
    threads = [int(x) for x in a.threads.split(",")]
    base = {"bench": "pyenv_threads", "where": "python", "label": a.label, "python": platform.python_version(), "numpy": np.__version__,
            "alternating": True, "rounds": a.rounds, "cores": os.cpu_count()}
    for n in [int(x) for x in a.games.split(",")]:
        env = cg.BatchEnv(a.teams, n, seed=a.seed)
        rng = np.random.default_rng(a.seed)
        buf = cg.new_tensors(n)
        configs = [(t, o) for t in threads for o in (False, True)]
        tot = {c: dict(decisions=0, games=0, wall=0.0, obs=0.0, choose=0.0, act=0.0, busy=0, total=0, proc=0) for c in configs}
        per = a.secs / a.rounds
        for _ in range(a.rounds):
            for c in configs:
                t, with_obs = c
                env.threads = t
                d0, g0 = env.decisions, env.games_done
                b0, a0, p0 = cpu()
                t0 = time.perf_counter()
                to = tc = ta = 0.0
                while time.perf_counter() - t0 < per:
                    x = time.perf_counter()
                    if with_obs:
                        mask = env.observe_tensors(out=buf).action_mask
                    else:
                        mask = env.masks()
                    y = time.perf_counter()
                    acts = ((1.0 - rng.random(mask.shape, dtype=np.float32)) * mask).argmax(axis=1).astype(np.int32)
                    z = time.perf_counter()
                    env.act(acts)
                    w = time.perf_counter()
                    to += y - x
                    tc += z - y
                    ta += w - z
                wall = time.perf_counter() - t0
                b1, a1, p1 = cpu()
                k = tot[c]
                k["decisions"] += env.decisions - d0
                k["games"] += env.games_done - g0
                k["wall"] += wall
                k["obs"] += to
                k["choose"] += tc
                k["act"] += ta
                k["busy"] += b1 - b0
                k["total"] += a1 - a0
                k["proc"] += p1 - p0
        for c in configs:
            t, with_obs = c
            k = tot[c]
            busy = k["busy"] / k["total"] if k["total"] else 0.0
            mine = k["proc"] / k["total"] if k["total"] else 0.0
            line = {**base, "n": n, "threads": t, "observe": with_obs, "seconds": round(k["wall"], 2), "decisions": k["decisions"], "games": k["games"],
                    "decisions_per_s": round(k["decisions"] / k["wall"], 1), "battles_per_s": round(k["games"] / k["wall"], 1),
                    "share_mask_or_observe": round(k["obs"] / k["wall"], 3), "share_numpy_choice": round(k["choose"] / k["wall"], 3),
                    "share_act": round(k["act"] / k["wall"], 3), "cpu_busy": round(busy, 3), "cpu_this_process": round(mine, 3),
                    "cpu_other_load": round(max(busy - mine, 0.0), 3)}
            print(json.dumps(line), flush=True)
            if a.out:
                with open(a.out, "a") as f:
                    f.write(json.dumps(line) + "\n")


if __name__ == "__main__":
    main()
