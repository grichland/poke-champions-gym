"""Sums the rowhunt runs of one tag: battles, identical battles, row steps, and every difference with its steps.

usage: python 2026-11-18-sim-rowhunt-sum.py TAG [TAG ...]
"""
import glob
import gzip
import json
import os
import sys

RES = os.path.dirname(os.path.abspath(__file__))


def main():
    tot = {"battles": 0, "identical": 0, "steps": 0, "rows_steps": 0, "rows_equal": 0}
    diffs = {}
    print(f"{'run':44} {'battles':>7} {'ident':>7} {'steps':>8} {'rowsEq':>8} {'secs':>6}")
    for tag in sys.argv[1:]:
        for path in sorted(glob.glob(os.path.join(RES, f"2026-11-18-sim-rowhunt-{tag}-*.jsonl.gz"))):
            summ = None
            fails = []
            exs = []
            try:
                with gzip.open(path, "rt", encoding="utf-8") as f:
                    for line in f:
                        r = json.loads(line)
                        k = r.get("kind")
                        if k == "summary":
                            summ = r
                        elif k == "rows_examples":
                            exs.append(r)
                        elif "fail" in r or k in ("oracle_error",) or r.get("cls") == "engine_error":
                            fails.append(r)
            except (EOFError, OSError) as e:
                print(f"{os.path.basename(path)}: unfinished ({e.__class__.__name__})")
                continue
            name = os.path.basename(path)[len("2026-11-18-sim-rowhunt-"):-len(".jsonl.gz")]
            if not summ:
                print(f"{name}: no summary")
                continue
            rs = summ.get("rows", {})
            print(f"{name:44} {summ['battles']:7} {summ['battles_identical']:7} {summ['steps']:8} "
                  f"{rs.get('steps_equal', 0):8} {summ['seconds']:6}")
            tot["battles"] += summ["battles"]
            tot["identical"] += summ["battles_identical"]
            tot["steps"] += summ["steps"]
            tot["rows_steps"] += rs.get("steps", 0)
            tot["rows_equal"] += rs.get("steps_equal", 0)
            for r in fails:
                key = "state/" + str(r.get("fail") or r.get("kind"))
                d = diffs.setdefault(key, {"count": 0, "steps": []})
                d["count"] += 1
                d["steps"].append(r.get("step_id") or f"{r.get('battle')}")
            for r in exs:
                d = diffs.setdefault("rows/" + r["key"], {"count": 0, "steps": []})
                d["count"] += r["count"]
                d["steps"] += [s["step"] for s in r["steps"]]
    print(json.dumps(tot))
    for k, v in sorted(diffs.items(), key=lambda kv: -kv[1]["count"]):
        print(f"  {v['count']:5}  {k}  {' '.join(map(str, v['steps'][:8]))}")


if __name__ == "__main__":
    main()
