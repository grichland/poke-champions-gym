"""Decisions/s of champions_gym.BatchEnv through Python with the observation at every decision, the replace
request before its turn's rows (replace_before_rows=True, the live order) against after them (False). Random
legal actions chosen in numpy from the action mask. The two run in alternating rounds of `secs` seconds (the
machine may be busy). Also reports the share of decisions that are replacements and the events unseen at them.

Usage: PYTHONPATH=python python bench/python/pyenv_rbr.py TEAMS_JSON [--secs 10] [--rounds 3] [--games 4096]
       [--threads 24] [--out FILE]
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
    ap.add_argument("--secs", type=float, default=10.0)
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--games", type=int, default=4096)
    ap.add_argument("--threads", type=int, default=24)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out")
    a = ap.parse_args()
    n = a.games
    base = {"bench": "pyenv_rbr", "python": platform.python_version(), "numpy": np.__version__, "n": n, "threads": a.threads, "round_secs": a.secs, "rounds": a.rounds}
    envs = {m: cg.BatchEnv(a.teams, n, seed=a.seed, threads=a.threads, replace_before_rows=(m == "on")) for m in ("on", "off")}
    bufs = {m: cg.new_tensors(n) for m in envs}
    rng = np.random.default_rng(a.seed)
    tot = {m: [0, 0.0, 0, 0] for m in envs}  # decisions, wall, games, replace decisions observed
    rounds = []
    for r in range(a.rounds):
        for m, env in envs.items():
            t0 = time.perf_counter()
            d0, g0, rep = env.decisions, env.games_done, 0
            while time.perf_counter() - t0 < a.secs:
                t = env.observe_tensors(out=bufs[m], threads=a.threads)
                rep += int((t.kind == 2).sum())
                mask = t.action_mask
                acts = ((1.0 - rng.random(mask.shape, dtype=np.float32)) * mask).argmax(axis=1).astype(np.int32)
                env.act(acts)
            w = time.perf_counter() - t0
            d = env.decisions - d0
            tot[m][0] += d
            tot[m][1] += w
            tot[m][2] += env.games_done - g0
            tot[m][3] += rep
            rounds.append({"round": r, "mode": m, "decisions_per_s": round(d / w, 1)})
            print(json.dumps(rounds[-1]), flush=True)
    lines = []
    for m, env in envs.items():
        d, w, gm, rep = tot[m]
        asked, stale, events = env.replace_counters()
        refusals, fallbacks, auto = env.counters()
        line = {
            **base,
            "mode": "replace_before_rows" if m == "on" else "replace_after_rows",
            "seconds": round(w, 2),
            "decisions": d,
            "decisions_per_s": round(d / w, 1),
            "battles_per_s": round(gm / w, 1),
            "replace_share": round(rep / max(d, 1), 4),
            "replace_asked": asked,
            "stale_replace": stale,
            "events_unseen_per_stale_replace": round(events / max(stale, 1), 2),
            "refusals": refusals,
            "fallbacks": fallbacks,
            "rounds": [x["decisions_per_s"] for x in rounds if x["mode"] == m],
        }
        lines.append(line)
        print(json.dumps(line), flush=True)
    if a.out:
        with open(a.out, "a") as f:
            for l in lines:
                f.write(json.dumps(l) + "\n")


if __name__ == "__main__":
    main()
