"""Decisions/s of a built-in player asked for every game's pending decision: `env.builtin_actions(name)`
(Rust, the environment's workers) against the per-Battle Python loop
(`BuiltinPlayer.action_for(env.battle(g))` for every g). For each thread count the two run in alternating
rounds on one environment until each has `secs` seconds of measured time; between rounds the games advance
with the player's actions (not timed). Needs no numpy.

Usage: PYTHONPATH=python python bench/python/builtin_batch.py [--teams T] [--games 4096] [--threads 1,8,24]
       [--player tower] [--secs 10] [--out FILE] [--label L]
       [--players random,tower,...]   (instead: only builtin_actions, alternating rounds over these players
                                       until each has `secs` of measured time; the games advance with the
                                       measured player's actions)
"""

import argparse
import ctypes
import json
import os
import platform
import time

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
    ap.add_argument("--teams", default=cg.TEAMS)
    ap.add_argument("--games", type=int, default=4096)
    ap.add_argument("--threads", default="1,8,24")
    ap.add_argument("--player", default="tower")
    ap.add_argument("--secs", type=float, default=10.0)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out")
    ap.add_argument("--label", default="")
    ap.add_argument("--players", default="")
    ap.add_argument("--max-wall", type=float, default=270.0, help="--players: wall seconds per thread count")
    a = ap.parse_args()
    if a.players:
        return many(a)
    n = a.games
    env = cg.BatchEnv(a.teams, n, seed=a.seed)
    # past the previews into a mix of turns and replaces
    for _ in range(40):
        env.act(env.builtin_actions(a.player))
    base = {"bench": "builtin_batch", "where": "python", "label": a.label, "python": platform.python_version(), "player": a.player,
            "games": n, "alternating": True, "cores": os.cpu_count(), "numpy": cg.env.np is not None}
    rows = []
    for t in [int(x) for x in a.threads.split(",")]:
        env.threads = t
        took = {"builtin_actions": 0.0, "python_loop": 0.0}
        dec = {"builtin_actions": 0, "python_loop": 0}
        kinds = {"builtin_actions": [0, 0, 0], "python_loop": [0, 0, 0]}
        rounds = 0
        c0 = cpu()
        w0 = time.perf_counter()
        seed = 0
        while min(took.values()) < a.secs:
            for mode in ("builtin_actions", "python_loop"):
                if took[mode] >= a.secs:
                    continue
                seed += 1
                _, kind = env.pending()
                for g in range(n):
                    kinds[mode][kind[g]] += 1
                t0 = time.perf_counter()
                if mode == "builtin_actions":
                    acts = env.builtin_actions(a.player, seed=seed)
                else:
                    acts = []
                    for g in range(n):
                        b = env.battle(g)
                        acts.append(cg.BuiltinPlayer(a.player, cg.mix_seed(cg.mix_seed(seed, g), b.decision)).action_for(b))
                took[mode] += time.perf_counter() - t0
                dec[mode] += n
                env.act(acts)
            rounds += 1
        c1 = cpu()
        wall = time.perf_counter() - w0
        busy = (c1[0] - c0[0]) / max(1, c1[1] - c0[1])
        mine = (c1[2] - c0[2]) / max(1, c1[1] - c0[1])
        for mode in ("builtin_actions", "python_loop"):
            r = dict(base, mode=mode, threads=t if mode == "builtin_actions" else 1, env_threads=t, rounds=rounds, decisions=dec[mode],
                     seconds=took[mode], decisions_per_s=dec[mode] / took[mode],
                     kinds={"preview": kinds[mode][0], "turn": kinds[mode][1], "replace": kinds[mode][2]},
                     wall_seconds=wall, cpu_busy=busy, cpu_this_process=mine, cpu_other_load=max(0.0, busy - mine))
            rows.append(r)
            print(json.dumps(r), flush=True)
        sp = rows[-2]["decisions_per_s"] / rows[-1]["decisions_per_s"]
        print("threads %d: builtin_actions %.0f/s, python loop %.0f/s, x%.1f" % (t, rows[-2]["decisions_per_s"], rows[-1]["decisions_per_s"], sp), flush=True)
    if a.out:
        with open(a.out, "a", encoding="utf-8") as f:
            for r in rows:
                f.write(json.dumps(r, sort_keys=True) + "\n")


def many(a):
    """builtin_actions only, alternating rounds over several players, per thread count."""
    names = a.players.split(",")
    n = a.games
    env = cg.BatchEnv(a.teams, n, seed=a.seed)
    for _ in range(40):
        env.act(env.builtin_actions("max_damage"))
    rows = []
    for t in [int(x) for x in a.threads.split(",")]:
        env.threads = t
        took = {k: 0.0 for k in names}
        dec = {k: 0 for k in names}
        kinds = {k: [0, 0, 0] for k in names}
        rounds = 0
        seed = 0
        c0 = cpu()
        w0 = time.perf_counter()
        # per-round CPU of the other processes, to spot disturbed rounds
        disturbed = 0
        while min(took.values()) < a.secs and time.perf_counter() - w0 < a.max_wall:
            r0 = cpu()
            for k in names:
                if took[k] >= a.secs:
                    continue
                seed += 1
                _, kind = env.pending()
                for g in range(n):
                    kinds[k][kind[g]] += 1
                t0 = time.perf_counter()
                acts = env.builtin_actions(k, seed=seed)
                took[k] += time.perf_counter() - t0
                dec[k] += n
                env.act(acts)
            r1 = cpu()
            other = max(0.0, ((r1[0] - r0[0]) - (r1[2] - r0[2])) / max(1, r1[1] - r0[1]))
            disturbed += other > 0.10
            rounds += 1
        c1 = cpu()
        wall = time.perf_counter() - w0
        busy = (c1[0] - c0[0]) / max(1, c1[1] - c0[1])
        mine = (c1[2] - c0[2]) / max(1, c1[1] - c0[1])
        for k in names:
            r = {"bench": "builtin_batch", "where": "python", "label": a.label, "python": platform.python_version(), "player": k,
                 "games": n, "alternating": "players", "cores": os.cpu_count(), "numpy": cg.env.np is not None,
                 "mode": "builtin_actions", "threads": t, "rounds": rounds, "decisions": dec[k], "seconds": took[k],
                 "decisions_per_s": dec[k] / took[k], "us_per_decision": 1e6 * took[k] / dec[k],
                 "kinds": {"preview": kinds[k][0], "turn": kinds[k][1], "replace": kinds[k][2]},
                 "wall_seconds": wall, "cpu_busy": busy, "cpu_this_process": mine, "cpu_other_load": max(0.0, busy - mine),
                 "rounds_other_load_over_10pct": disturbed}
            rows.append(r)
            print("threads %d %-24s %9.0f decisions/s  %7.2f us/decision  (%.1f s measured)" % (t, k, r["decisions_per_s"], r["us_per_decision"], took[k]), flush=True)
        print("threads %d: other CPU load %.1f%%, %d of %d rounds over 10%%" % (t, 100 * max(0.0, busy - mine), disturbed, rounds), flush=True)
        if a.out:
            with open(a.out, "a", encoding="utf-8") as f:
                for r in rows[-len(names):]:
                    f.write(json.dumps(r, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
