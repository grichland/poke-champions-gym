"""Per-kind row agreement of the sim-log --rows runs of one seed, summed over the pools named."""
import gzip
import json
import sys

seed = sys.argv[1]
pools = sys.argv[2:] or ["real", "stage5"]
kinds, dropped_sd, dropped_en, dis = {}, {}, {}, {}
steps = equal = 0
for p in pools:
    with gzip.open(f"tools/rig/results/2026-11-18-sim-log-on-{seed}-{p}.jsonl.gz", "rt", encoding="utf-8") as f:
        for line in f:
            r = json.loads(line)
            if r.get("kind") != "summary":
                continue
            rs = r["rows"]
            steps += rs["steps"]
            equal += rs["steps_equal"]
            for k, v in rs["kinds"].items():
                d = kinds.setdefault(k, {"both": 0, "showdown_only": 0, "engine_only": 0})
                for x in d:
                    d[x] += v[x]
            for k, v in rs["showdown_lines_dropped"].items():
                dropped_sd[k] = dropped_sd.get(k, 0) + v
            for k, v in rs["engine_rows_dropped"].items():
                dropped_en[k] = dropped_en.get(k, 0) + v
            for k, v in rs["disagreements"].items():
                dis[k] = dis.get(k, 0) + v
print(f"pools {pools} seed {seed}: steps {steps}, steps with identical rows {equal}")
print(f"{'kind':16} {'both':>8} {'sd_only':>8} {'en_only':>8} {'agree%':>8}")
for k, v in sorted(kinds.items()):
    tot = v["both"] + max(v["showdown_only"], v["engine_only"])
    print(f"{k:16} {v['both']:8} {v['showdown_only']:8} {v['engine_only']:8} {100.0 * v['both'] / tot if tot else 100:8.2f}")
print("disagreements:", json.dumps(dis))
print("showdown lines dropped:", json.dumps(dropped_sd))
print("engine rows dropped:", json.dumps(dropped_en))
