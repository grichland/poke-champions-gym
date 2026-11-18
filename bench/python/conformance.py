"""Check (f): the events CG emits against the controller's recorded stream, per kind.

conformance.py RECORDED.jsonl.gz CG_EVENTS.jsonl [CG_EVENTS.jsonl ...] [--out FILE.json]

RECORDED is the controller's stream (one battle per line, `messages` from the controller); each CG file
is a `players_run --events-out` file (`record0`, `record1`: each seat's messages). For every event kind:
the field sets seen on each side, the values of enumerated fields, and the most common neighbours. Prints
what CG emits that the recordings never show and the other way round, and writes the counts as JSON.
"""

import collections
import gzip
import json
import sys

ENUMS = {"effect", "reason", "what", "how", "dir", "status", "by", "forced", "pivot", "called_by", "weather", "type", "lose", "add", "item"}


def label(e):
    k = e["k"]
    if k == "item":
        return "item:" + str(e.get("what"))
    if k in ("cause", "volatile", "volatile_end", "volatile_hit", "once", "blocked"):
        return k + ":" + str(e.get("effect"))
    if k == "hp":
        return "hp:" + str(e.get("dir"))
    return k


class Tally:
    def __init__(self):
        self.shapes = collections.Counter()
        self.values = collections.defaultdict(collections.Counter)
        self.next = collections.defaultdict(collections.Counter)
        self.events = 0

    def add_list(self, evs):
        for i, e in enumerate(evs):
            self.events += 1
            k = e["k"]
            self.shapes[(k, tuple(sorted(x for x in e if x != "k")))] += 1
            for f, v in e.items():
                if f in ENUMS:
                    self.values[(k, f)][v if isinstance(v, str) else json.dumps(v)] += 1
            if i + 1 < len(evs):
                self.next[label(e)][label(evs[i + 1])] += 1


def recorded(path):
    t = Tally()
    with gzip.open(path, "rt", encoding="utf-8") as f:
        for line in f:
            d = json.loads(line)
            evs = []
            for m in d["messages"]:
                if m["from"] == "controller" and m["msg"].get("type") == "events":
                    evs.extend(m["msg"]["events"])
            t.add_list(evs)
    return t


def cg(paths):
    t = Tally()
    for p in paths:
        with open(p, encoding="utf-8") as f:
            for line in f:
                d = json.loads(line)
                for key in ("record0", "record1"):
                    evs = []
                    for m in d.get(key, []):
                        if m.get("type") == "events":
                            evs.extend(m["events"])
                    t.add_list(evs)
    return t


def main():
    args = sys.argv[1:]
    out = None
    if "--out" in args:
        i = args.index("--out")
        out = args[i + 1]
        del args[i : i + 2]
    rec = recorded(args[0])
    ours = cg(args[1:])
    kinds_r = {k for k, _ in rec.shapes}
    kinds_c = {k for k, _ in ours.shapes}
    shapes_r = set(rec.shapes)
    shapes_c = set(ours.shapes)
    report = {"recorded_events": rec.events, "cg_events": ours.events, "kinds_cg_only": sorted(kinds_c - kinds_r), "kinds_recorded_only": sorted(kinds_r - kinds_c)}
    report["shapes_cg_only"] = sorted([[k, list(f), ours.shapes[(k, f)]] for k, f in shapes_c - shapes_r if k in kinds_r])
    report["shapes_recorded_only"] = sorted([[k, list(f), rec.shapes[(k, f)]] for k, f in shapes_r - shapes_c if k in kinds_c])
    vals = []
    for key in sorted(set(rec.values) | set(ours.values)):
        a, b = rec.values.get(key, {}), ours.values.get(key, {})
        only_c = sorted(set(b) - set(a))
        only_r = sorted(set(a) - set(b))
        if only_c or only_r:
            vals.append({"kind": key[0], "field": key[1], "cg_only": {v: b[v] for v in only_c}, "recorded_only": {v: a[v] for v in only_r}})
    report["values"] = vals
    # the phone's order: the most common next event of each label, where the two differ
    order = []
    for lab in sorted(set(rec.next) & set(ours.next)):
        r1 = rec.next[lab].most_common(1)[0][0]
        c1 = ours.next[lab].most_common(1)[0][0]
        if r1 != c1:
            order.append({"label": lab, "recorded_next": rec.next[lab].most_common(3), "cg_next": ours.next[lab].most_common(3)})
    report["order_differences"] = order
    s = json.dumps(report, indent=1)
    print(s)
    if out:
        with open(out, "w", encoding="utf-8") as f:
            f.write(s)


if __name__ == "__main__":
    main()
