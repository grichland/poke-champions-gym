"""Speed of champions_gym.BatchEnv through Python: random legal actions chosen in numpy from the mask, one
process, one thread. Then the runner with two RandomPlayers (one Python call per decision).

Usage: PYTHONPATH=python python bench/python/pyenv_play.py TEAMS_JSON [--secs T] [--out FILE] [--games 256,4096]
One JSON line per run to stdout and to FILE (appended). Each run is capped at 60 s.
"""

import argparse
import json
import platform
import time

import numpy as np

import champions_gym as cg


def drive(teams, games, seed, secs):
    env = cg.BatchEnv(teams, games, seed=seed)
    rng = np.random.default_rng(seed)
    t_obs = t_pick = t_act = 0.0
    calls = 0
    t0 = time.perf_counter()
    while time.perf_counter() - t0 < secs:
        a = time.perf_counter()
        _, _, mask = env.observe()
        b = time.perf_counter()
        acts = (rng.random(mask.shape, dtype=np.float32) * mask).argmax(axis=1).astype(np.int32)
        c = time.perf_counter()
        env.act(acts)
        d = time.perf_counter()
        t_obs += b - a
        t_pick += c - b
        t_act += d - c
        calls += 1
    wall = time.perf_counter() - t0
    return {
        "phase": "numpy_random",
        "games_per_env": games,
        "wall_s": round(wall, 3),
        "act_calls": calls,
        "decisions": env.decisions,
        "battles": env.games_done,
        "decisions_per_s": round(env.decisions / wall, 1),
        "battles_per_s": round(env.games_done / wall, 1),
        "decisions_per_battle": round(env.decisions / max(env.games_done, 1), 2),
        "share_observe": round(t_obs / wall, 3),
        "share_numpy_pick": round(t_pick / wall, 3),
        "share_act": round(t_act / wall, 3),
    }


def runner(teams, games, seed, n_games):
    t0 = time.perf_counter()
    r = cg.play_games(cg.RandomPlayer(seed), cg.RandomPlayer(seed + 1), n_games, teams, seed=seed, batch=games)
    wall = time.perf_counter() - t0
    return {
        "phase": "runner_random_players",
        "games_per_env": games,
        "wall_s": round(wall, 3),
        "battles": len(r.games),
        "battles_per_s": round(len(r.games) / wall, 1),
        "wins": r.wins,
        "truncated": r.truncated,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("teams")
    ap.add_argument("--secs", type=float, default=30.0)
    ap.add_argument("--games", default="256,4096")
    ap.add_argument("--runner-games", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out")
    a = ap.parse_args()
    secs = min(a.secs, 60.0)
    base = {"bench": "pyenv_play", "pool": "stage3", "threads": 1, "python": platform.python_version(), "numpy": np.__version__}
    lines = []
    for n in [int(x) for x in a.games.split(",")]:
        lines.append({**base, **drive(a.teams, n, a.seed, secs)})
        print(json.dumps(lines[-1]), flush=True)
    if a.runner_games:
        lines.append({**base, **runner(a.teams, 256, a.seed, a.runner_games)})
        print(json.dumps(lines[-1]), flush=True)
    if a.out:
        with open(a.out, "a") as f:
            for l in lines:
                f.write(json.dumps(l) + "\n")


if __name__ == "__main__":
    main()
