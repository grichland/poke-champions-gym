"""Totals over replay summaries (battle/src/bin/replay.rs output), with example rows' text.

Usage: report.py REPLAY_SUMMARY.jsonl RUNS_DIR > report.txt
"""

import json
import os
import sys
from collections import Counter, defaultdict


def main():
    summ = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
    runs = sys.argv[2]
    texts = {}

    def row_text(s, rid):
        if not rid or not s.get("run"):
            return None
        key = (s["run"], s["folder"])
        if key not in texts:
            d = {}
            p = os.path.join(runs, s["run"], s["folder"], "log.jsonl")
            if os.path.exists(p):
                for l in open(p, encoding="utf-8"):
                    r = json.loads(l)
                    d["T%d#%d" % (r["turn"], r["index"])] = r["text"]
            texts[key] = d
        return texts[key].get(rid)

    def ctx(s, rid, k=2):
        """The row and up to k rows before it."""
        if not rid:
            return None
        t, i = rid[1:].split("#")
        return [row_text(s, "T%s#%d" % (t, j)) for j in range(max(0, int(i) - k), int(i) + 1)]

    out = []
    p = out.append
    n = len(summ)
    gapped = [s for s in summ if s["gap_before_end"]]
    p("battles %d, with a gap before the end %d, without %d" % (n, len(gapped), n - len(gapped)))
    kinds = Counter()
    for s in summ:
        kinds.update(s["events"])
    rows = sum(s["rows"] for s in summ)
    p("row events %d, unread %d (%.3f%%)" % (rows, kinds["unread"], 100.0 * kinds["unread"] / max(rows, 1)))
    p("events per kind: " + ", ".join("%s %d" % kv for kv in kinds.most_common()))
    ad = Counter()
    for s in summ:
        ad.update(s["adapt"])
    p("adapted: " + json.dumps(ad))
    pe = Counter()
    for s in summ:
        for k, v in s["parse"].items():
            pe[k] += v["n"]
    p("parse errors: " + json.dumps(pe))

    for label, key in (("ANOMALIES (no gap before)", "anomalies"), ("ANOMALIES AFTER A GAP", "anomalies_after_gap")):
        for grp_name, grp in (("battles without a mid gap", [s for s in summ if not s["gap_before_end"]]), ("battles with a mid gap", gapped)):
            agg = defaultdict(lambda: [0, 0, []])
            for s in grp:
                for k, v in s[key].items():
                    a = agg[k]
                    a[0] += v["n"]
                    a[1] += 1
                    for e in v["examples"]:
                        if len(a[2]) < 3:
                            a[2].append((s.get("run"), s.get("folder"), e.get("row") if isinstance(e, dict) else None, e if not isinstance(e, dict) else e.get("what"), ctx(s, e.get("row")) if isinstance(e, dict) else None, e.get("event") if isinstance(e, dict) else None))
            p("")
            p("== %s, %s: %d kinds, %d anomalies" % (label, grp_name, len(agg), sum(a[0] for a in agg.values())))
            for k, a in sorted(agg.items(), key=lambda x: -x[1][0]):
                p("  %5d in %3d battles  %s" % (a[0], a[1], k))
                for ex in a[2]:
                    p("        %s/%s %s | %s | rows %s | ev %s" % ex)

    p("")
    p("== DECISIONS")
    dec = Counter()
    for s in summ:
        for k, v in s["decisions"].items():
            dec[k] += v["n"]
    for k, v in sorted(dec.items()):
        p("  %6d %s" % (v, k))
    for grp_name, grp in (("without a mid gap", [s for s in summ if not s["gap_before_end"]]), ("with a mid gap", gapped)):
        agg = defaultdict(lambda: [0, []])
        for s in grp:
            for k, v in s["misses"].items():
                agg[k][0] += v["n"]
                for e in v["examples"]:
                    if len(agg[k][1]) < 4:
                        agg[k][1].append((s.get("run"), s.get("folder"), e))
        p("  misses (accepted, outside the legal set), battles %s: %d" % (grp_name, sum(a[0] for a in agg.values())))
        for k, a in sorted(agg.items(), key=lambda x: -x[1][0]):
            p("    %5d %s" % (a[0], k))
            for r, f, e in a[1]:
                p("          %s/%s seq %s turn %s %s inc=%s legal=%s" % (r, f, e["seq"], e["turn"], json.dumps(e["order"]), e["incomplete"], json.dumps(e["legal"])))
    ref = defaultdict(lambda: [0, []])
    for s in summ:
        for k, v in s["refusals"].items():
            ref[k][0] += v["n"]
            for e in v["examples"]:
                ref[k][1].append((s.get("run"), s.get("folder"), e))
    p("  refusals:")
    for k, a in ref.items():
        p("    %d %s" % (a[0], k))
        for r, f, e in a[1]:
            p("          %s/%s seq %s turn %s %s why=%s legal=%s" % (r, f, e["seq"], e["turn"], json.dumps(e["order"]), e["why"], json.dumps(e["legal"])))

    p("")
    p("== HP")
    hpn = sum(s["hp_rows"] for s in summ)
    agg = defaultdict(lambda: [0, []])
    for s in summ:
        for k, v in s["hp_disagree"].items():
            agg[k][0] += v["n"]
            for e in v["examples"]:
                if len(agg[k][1]) < 3:
                    agg[k][1].append((s.get("run"), s.get("folder"), e, ctx(s, e.get("row"), 4)))
    p("hp rows %d, before disagreeing %d" % (hpn, sum(a[0] for a in agg.values())))
    for k, a in sorted(agg.items()):
        p("  %5d %s" % (a[0], k))
        for ex in a[1]:
            p("        %s/%s %s rows %s" % ex)

    p("")
    p("== TURNS")
    tw = Counter()
    ex = defaultdict(list)
    for s in summ:
        for t in s["turns_without_request"]:
            k = "rows, no turn request, lock at start: " + t["lock"].split(" x")[0]
            tw[k] += 1
            if len(ex[k]) < 4:
                ex[k].append((s.get("run"), s.get("folder"), t))
        for t in s["turns_empty_without_request"]:
            k = "no rows, no turn request (not the last turn), gap=%s, lock at start: %s" % (t["gap"], t["lock"].split(" x")[0])
            tw[k] += 1
            if len(ex[k]) < 4:
                ex[k].append((s.get("run"), s.get("folder"), t))
    for k, v in tw.most_common():
        p("  %5d %s" % (v, k))
        for e in ex[k]:
            p("        %s/%s %s" % e)
    print("\n".join(str(x) for x in out))


if __name__ == "__main__":
    main()
