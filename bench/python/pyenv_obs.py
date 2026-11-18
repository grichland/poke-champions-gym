"""Decisions/s of champions_gym.BatchEnv through Python with the observation requested at every decision,
against the same loop without it. Random legal actions chosen in numpy from the action mask. The modes run
in alternating rounds (the machine may be busy), each mode `secs` seconds in total per N.

Usage: PYTHONPATH=python python bench/python/pyenv_obs.py TEAMS_JSON [--secs 30] [--rounds 6]
       [--games 256,4096] [--threads 1,8] [--out FILE]
Modes: no_obs (env.masks()), obs_tN (env.observe_tensors(out=..., threads=N), its action_mask).
"""

import argparse
import json
import platform
import time

import numpy as np

import champions_gym as cg


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("teams")
    ap.add_argument("--secs", type=float, default=30.0)
    ap.add_argument("--rounds", type=int, default=6)
    ap.add_argument("--games", default="256,4096")
    ap.add_argument("--threads", default="1,8")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out")
    a = ap.parse_args()
    threads = [int(x) for x in a.threads.split(",")]
    base = {"bench": "pyenv_obs", "where": "python", "python": platform.python_version(), "numpy": np.__version__, "alternating": True, "rounds": a.rounds}
    lines = []
    for n in [int(x) for x in a.games.split(",")]:
        env = cg.BatchEnv(a.teams, n, seed=a.seed)
        rng = np.random.default_rng(a.seed)
        buf = cg.new_tensors(n)
        modes = ["no_obs"] + ["obs_t%d" % t for t in threads]
        tot = {m: [0, 0.0, 0.0, 0] for m in modes}  # decisions, wall, seconds inside observe, games finished
        per = a.secs / a.rounds
        for r in range(a.rounds):
            for m in modes:
                t0 = time.perf_counter()
                d0 = env.decisions
                g0 = env.games_done
                t_obs = 0.0
                while time.perf_counter() - t0 < per:
                    if m == "no_obs":
                        mask = env.masks()
                    else:
                        x = time.perf_counter()
                        t = env.observe_tensors(out=buf, threads=int(m[5:]))
                        t_obs += time.perf_counter() - x
                        mask = t.action_mask
                    acts = ((1.0 - rng.random(mask.shape, dtype=np.float32)) * mask).argmax(axis=1).astype(np.int32)
                    env.act(acts)
                tot[m][0] += env.decisions - d0
                tot[m][1] += time.perf_counter() - t0
                tot[m][2] += t_obs
                tot[m][3] += env.games_done - g0
        for m in modes:
            d, w, o, gm = tot[m]
            line = {**base, "mode": m, "n": n, "seconds": round(w, 2), "decisions": d, "decisions_per_s": round(d / w, 1), "battles_per_s": round(gm / w, 1)}
            if m != "no_obs":
                line["threads"] = int(m[5:])
                line["share_observe"] = round(o / w, 3)
                line["us_per_observation_incl_python"] = round(o * 1e6 / d, 3)
            lines.append(line)
            print(json.dumps(line), flush=True)
    if a.out:
        with open(a.out, "a") as f:
            for l in lines:
                f.write(json.dumps(l) + "\n")


if __name__ == "__main__":
    main()
