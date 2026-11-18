"""The controller's stream (one gzipped JSON line per battle, messages in {from, msg} envelopes) -> one file
per battle of plain message lines, led by {"meta": {run, folder, battle, row_ids}}, for battle/src/bin/replay.rs.

Usage: unpack_stream.py STREAM.jsonl.gz OUT_DIR
"""

import gzip
import json
import os
import sys


def is_trigger(e):
    """An own row after which our replace decision comes: a faint, a pivot withdraw, Eject Button, Emergency Exit."""
    if e.get("side") != "own":
        return False
    return (e["k"] == "faint" or (e["k"] == "withdraw" and e.get("pivot"))
            or (e["k"] == "item" and e.get("item") == "ejectbutton" and e.get("what") == "used")
            or (e["k"] == "ability" and e.get("ability") == "emergencyexit"))


def place_replaces(msgs, stats):
    """The stream puts a replace request where it was recorded, often before the rows of its turn. Move each
    replace group (requests, answers, refused events) to just before the own `enter` that follows the own
    faint or pivot it answers, within the same turn (CF-S5)."""
    hdr = {k: msgs[0][k] for k in ("v", "battle")} if msgs and "v" in msgs[0] else {"v": "0.2", "battle": ""}
    items = []  # ("ev", event) or ("msg", message)
    for m in msgs:
        if m.get("type") == "events":
            for e in m["events"]:
                items.append(("ev", e))
            if "fallback" in m:
                items.append(("msg", dict(hdr, type="events", events=[], fallback=m["fallback"])))
        else:
            items.append(("msg", m))
    i = 0
    while i < len(items):
        t, m = items[i]
        if t == "msg" and m.get("type") == "decision_request" and m.get("kind") == "replace":
            j = i
            group = []
            while j < len(items):
                tj, mj = items[j]
                req = tj == "msg" and mj.get("type") == "decision_request" and mj.get("kind") == "replace"
                ans = tj == "msg" and "order" in mj and "type" not in mj
                ref = tj == "ev" and mj["k"] == "refused"
                # a new replace request that refuses nothing is the next group
                if not (req or ans or ref) or (req and group and not mj.get("refused")):
                    break
                group.append(items[j])
                j += 1
            # already right after its trigger?
            before_ok = False
            k2 = i - 1
            while k2 >= 0 and not (items[k2][0] == "ev" and items[k2][1]["k"] == "turn"):
                if items[k2][0] == "ev" and items[k2][1].get("side") == "own" and items[k2][1]["k"] == "enter":
                    break
                if items[k2][0] == "ev" and is_trigger(items[k2][1]):
                    before_ok = True
                    break
                k2 -= 1
            target = None
            if not before_ok:
                trig = False
                for q in range(j, len(items)):
                    tq, mq = items[q]
                    if tq == "ev" and mq["k"] == "turn":
                        break
                    if tq == "msg" and mq.get("type") == "decision_request":
                        break
                    if tq == "ev" and is_trigger(mq):
                        trig = True
                    elif tq == "ev" and trig and mq["k"] == "enter" and mq.get("side") == "own" and not mq.get("forced"):
                        target = q
                        break
            if target is not None:
                moved = items[:i] + items[j:target] + group + items[target:]
                items = moved
                stats["replace moved"] += 1
                i = target  # continue after the moved group
                continue
            stats["replace left" + (" (after its trigger)" if before_ok else " (no trigger found)")] += 1
            i = j
            continue
        i += 1
    out = []
    buf = []
    for t, m in items:
        if t == "ev":
            buf.append(m)
            continue
        if buf:
            out.append(dict(hdr, type="events", events=buf))
            buf = []
        out.append(m)
    if buf:
        out.append(dict(hdr, type="events", events=buf))
    return out


def main():
    from collections import Counter
    src, out = sys.argv[1], sys.argv[2]
    reorder = len(sys.argv) > 3 and sys.argv[3] == "--place-replaces"
    stats = Counter()
    os.makedirs(out, exist_ok=True)
    paths = []
    with gzip.open(src, "rt", encoding="utf-8") as f:
        for line in f:
            b = json.loads(line)
            name = "%s__%s.jsonl" % (b["run"], b["folder"])
            p = os.path.join(out, name)
            with open(p, "w", encoding="utf-8", newline="\n") as g:
                meta = {k: b[k] for k in ("run", "folder", "battle", "row_ids")}
                g.write(json.dumps({"meta": meta}, ensure_ascii=False) + "\n")
                msgs = [m["msg"] for m in b["messages"]]
                if reorder:
                    msgs = place_replaces(msgs, stats)
                for m in msgs:
                    g.write(json.dumps(m, ensure_ascii=False) + "\n")
            paths.append(os.path.abspath(p))
    with open(os.path.join(out, "list.txt"), "w", encoding="utf-8", newline="\n") as g:
        g.write("\n".join(paths) + "\n")
    print("battles", len(paths), dict(stats))


if __name__ == "__main__":
    main()
