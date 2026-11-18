"""A pcsa-former checkpoint through champions_gym.neural, inference only on the CPU (docs/neural-player.md).

Run with a Python that has torch, PYTHONPATH = python;<pcsa-former>;docs/examples, CUDA hidden:
    python bench/python/neural_pcsa_former.py --ckpt <pt> --part load|eval|equiv|live [--games N]
Writes bench/results/2026-11-19-neural-<part>*.json(l).
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import time

import champions_gym as cg
from champions_gym import _cg
from champions_gym import neural as N
from champions_gym.live import Session

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
OUT = os.path.join(REPO, "bench", "results")
SPEC = "pcsa_former_policy:PcsaFormerPolicy"
PREFIX = os.environ.get("NEURAL_PREFIX", "2026-11-19-neural2")
PHONE = os.path.join(REPO, "python", "tests", "data", "live-phone-battle.jsonl")


def dump(name, obj):
    path = os.path.join(OUT, "%s-%s" % (PREFIX, name))
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(obj, f, indent=1, default=str)
    print("wrote", path, flush=True)


def policy(a, **kw):
    return N.make_policy(SPEC, a.ckpt, "cpu", dict(threads=a.threads, allow_contract=True, **kw))


def part_load(a):
    t0 = time.perf_counter()
    out = {"ckpt": a.ckpt, "cg_contract": N.contract()}
    try:
        N.load(SPEC, a.ckpt, "cpu", {"threads": a.threads, "allow_contract": True})
        out["strict_load"] = "ok"
    except N.ModelMismatch as e:
        out["strict_load"] = "refused: %s" % e
    p = N.load(SPEC, a.ckpt, "cpu", {"threads": a.threads, "allow_contract": True}, allow_mismatch=True)
    out.update(info=p.info, mismatch=p.shell.mismatch, warmup_ms=p.warmup_ms, load_s=round(time.perf_counter() - t0, 2))
    print(json.dumps(out, indent=1, default=str))
    dump("load.json", out)


def pool_weights():
    from pcsa_former import teams  # read-only: teams/pool.json

    return [float(x) for x in teams.weights(teams.load())]


def part_eval(a):
    from pcsa_former import teams as T

    eval_teams = os.path.join(os.path.dirname(T.POOL), "eval_cg_teams.json")
    w = pool_weights()
    rows = []
    t_all = time.perf_counter()
    for order in (True, False):
        for k, opp in enumerate(("tower", "max_damage")):
            if time.perf_counter() - t_all > a.limit:
                print("time limit reached; stopping", flush=True)
                break
            pol = policy(a)
            left = max(30.0, a.limit - (time.perf_counter() - t_all))
            logd = os.path.join(a.logs, "eval-%s-%s" % ("live" if order else "old", opp))
            r = N.evaluate(pol, opp, games=a.games, seed=12345 + k, threads=a.threads, batch=a.batch, teams=eval_teams, team_weights=w,
                           replace_before_rows=order, allow_mismatch=True, time_limit_s=left, log_dir=logd)  # fmt: skip
            d = r.to_dict()
            d.update(replace_before_rows=order, teams="pcsa-former eval_cg_teams.json, pool weights", ckpt=a.ckpt)
            print("EVAL " + json.dumps(d, default=str), flush=True)
            rows.append(d)
    dump("eval.json", rows)


def part_equiv(a):
    sys.path.insert(0, os.path.join(REPO, "python", "tests"))
    from test_neural import manual_play

    out = []
    for opp, games, seed in (("tower", a.games, 21), ("max_damage", a.games, 22)):
        r = N.evaluate(policy(a), opp, games=games, seed=seed, threads=a.threads, batch=a.batch, record=True, allow_mismatch=True)
        log, p = manual_play(policy(a), opp, games, seed, a.batch, a.threads, allow_mismatch=True)
        same = sum(1 for x, y in zip(r.actions, log) if x == y)
        first = next((i for i, (x, y) in enumerate(zip(r.actions, log)) if x != y), None)
        d = {"opponent": opp, "games": games, "batched_decisions": len(r.actions), "single_decisions": len(log), "identical_prefix": first if first is not None else len(log),
             "equal": r.actions == log, "same_count": same, "first_difference": None if first is None else [r.actions[first], log[first]]}  # fmt: skip
        print(json.dumps(d), flush=True)
        out.append(d)
    dump("equiv.json", out)


def part_live(a):
    def run_stream(p, ms, times):
        s = Session(p)
        n = bad = 0
        for m in ms:
            t0 = time.perf_counter()
            r = s.handle(m)
            dt = (time.perf_counter() - t0) * 1000.0
            if m.get("type") in ("decision_request", "preview_request") and r is not None:
                times.append(dt)
                n += 1
                if "order" in r and r["notes"]["action"] not in r["notes"]["legal"]:
                    bad += 1
                if p.last and p.last.get("fallback"):
                    bad += 1
        return n, bad, [x for x in s.anomalies if x["source"] != "builder"]

    p = N.load(SPEC, a.ckpt, "cpu", {"threads": a.threads, "allow_contract": True}, allow_mismatch=True, log_dir=os.path.join(a.logs, "live"))
    out = {"warmup_ms": p.warmup_ms}
    times = []
    with open(PHONE, encoding="utf-8") as f:
        ms = [json.loads(x) for x in f]
    n, bad, anom = run_stream(p, ms, times)
    out["phone"] = {"answers": n, "illegal_or_fallback": bad, "anomalies": anom}
    tot = 0
    for k in range(20):
        rec, _ = _cg.live_selfplay(cg.TEAMS, 500 + k, ("tower", "max_damage", "random", "disciplined")[k % 4], "max_damage", True)
        lines = [m for m in (json.loads(x) for x in rec) if "type" in m]
        n, b2, anom = run_stream(p, lines, times)
        tot += n
        bad += b2
        assert not anom, anom
    p.close()
    times.sort()
    out["streams"] = {"streams": 20, "answers": tot, "illegal_or_fallback": bad}
    out["handle_ms"] = {"n": len(times), "median": round(statistics.median(times), 1), "p99": round(times[int(0.99 * (len(times) - 1))], 1), "max": round(times[-1], 1)}
    out["stats"] = vars(p.stats)
    print(json.dumps(out, indent=1, default=str))
    dump("live.json", out)


def part_restart(a):
    """Answers after a restart at a random request, against the battle played straight through: with the
    observe hook, without it (CG before the hook), and with the Session kept (a controller restart)."""
    import random

    sys.path.insert(0, os.path.join(REPO, "python", "tests"))
    from test_neural import _answers, play, restart_run

    pol = policy(a)
    make = lambda: N.NeuralPlayer(pol, allow_mismatch=True, warmup=False)  # noqa: E731  own slots, shared weights
    with open(PHONE, encoding="utf-8") as f:
        streams = [("phone", [json.loads(x) for x in f])]
    for k in range(20):
        rec, _ = _cg.live_selfplay(cg.TEAMS, 700 + k, ("tower", "max_damage", "random", "disciplined")[k % 4], "max_damage", True)
        tag = "restart-%d" % k
        streams.append((tag, [dict(m, battle=tag) if "battle" in m else m for m in (json.loads(x) for x in rec) if "type" in m]))
    rng = random.Random(a.seed)
    rows = []
    tot = {"after": 0, "differ_with_hook": 0, "differ_without_hook": 0, "differ_in_process": 0}
    for name, ms in streams:
        sr = play(make(), ms)
        straight = _answers(sr)
        values = [x["notes"]["model"]["value"] for x in sr if x and "order" in x]
        reqs = [i for i, m in enumerate(ms) if m["type"] == "decision_request"]
        cut = rng.choice(reqs[1:]) if len(reqs) > 1 else reqs[0]
        n = len([1 for m in ms[cut:] if m["type"] == "decision_request"])
        want = straight[len(straight) - n :]
        r = {"stream": name, "requests": len(reqs), "restart_at_seq": ms[cut].get("seq"), "after": n}
        vwant = values[len(values) - n :]
        for key, kw in (("with_hook", {}), ("without_hook", {"hooks": False}), ("in_process", {"in_process": True})):
            got, vals = restart_run(ms, cut, make, values=True, **kw)
            r[key] = sum(1 for x, y in zip(got, want) if x != y) + abs(len(got) - len(want))
            # the model's value at each answer (4 decimals): equal only when the memory is the same
            r[key + "_value_differs"] = sum(1 for x, y in zip(vals, vwant) if x != y)
            tot["value_differs_" + key] = tot.get("value_differs_" + key, 0) + r[key + "_value_differs"]
        rows.append(r)
        tot["after"] += n
        tot["differ_with_hook"] += r["with_hook"]
        tot["differ_without_hook"] += r["without_hook"]
        tot["differ_in_process"] += r["in_process"]
        print(json.dumps(r), flush=True)
    out = {"ckpt": a.ckpt, "streams": len(streams), "total": tot, "rows": rows}
    print(json.dumps(tot), flush=True)
    dump("restart.json", out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--part", required=True)
    ap.add_argument("--games", type=int, default=2000)
    ap.add_argument("--batch", type=int, default=250)
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--limit", type=float, default=560.0, help="seconds")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--logs", default=os.path.join(REPO, "target", "neural-logs"))
    a = ap.parse_args()
    {"load": part_load, "eval": part_eval, "equiv": part_equiv, "live": part_live, "restart": part_restart}[a.part](a)


if __name__ == "__main__":
    main()
