"""The controller's recorded streams through `champions_gym.live.Session` (LIVE.md section 8, live side).

For each battle: the Session answers every request with a built-in player (decisions) and the recorded
picks (the team the game used); the recorded orders are checked against the Session's legal set at their
request, classified as `battle/src/bin/replay.rs` does (accepted, refused, re-asked, fallback). Also: the
same battles with each replace request moved to where the phone sends it live (before the rows of its
turn, LV5Q-7), and timings.

Also the picks check (S8-1): the recorded orders with the preview answered by tower instead of the recorded
picks, without and with the recorded picks given in `battle_start.picks` or a preview `fallback`.

Also the second-fallback check (S10-3): a first preview fallback wrong in one back slot, and a second one
with the true picks when the true back member first enters (or earlier), against the true picks from the
start.

Usage (Program Files Python, PYTHONPATH=python):
  live_conformance.py OUT_DIR STREAM.jsonl.gz... [--phone BATTLE_DIR]... [--players tower,max_damage] [--picks-only]
  live_conformance.py OUT_DIR STREAM.jsonl.gz... --second-fallback-only
  live_conformance.py --phone-stream BATTLE_DIR OUT.jsonl [--order live|placed]
"""

import argparse
import collections
import glob
import gzip
import json
import os
import re
import statistics
import sys
import time

from champions_gym import _cg
from champions_gym.live import Session

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# the game picked and played for us after a missed deadline: the saved picks and orders are not what was played
DROPPED = {"20261115-034548-run__battle-32"}


# ---------------------------------------------------------------- reading

def stream_battles(path):
    """(name, messages) per battle of a controller stream (one gzipped line per battle, {from, msg})."""
    with gzip.open(path, "rt", encoding="utf-8") as f:
        for line in f:
            b = json.loads(line)
            name = "%s__%s" % (b["run"], b["folder"])
            if name in DROPPED:
                continue
            yield name, [m["msg"] for m in b["messages"]]


def order_of(text):
    """'/choose move x' or '/choose switch Name' as a contract ORDER."""
    w = text.split()
    if w[1] == "move":
        return {"kind": "move", "move": w[2], "mega": "mega" in w[3:]}
    return {"kind": "switch", "mon": re.sub(r"[^a-z0-9]", "", " ".join(w[2:]).lower())}


def phone_battle(battle_dir, order="live"):
    """A live phone battle (`battle-N/events.json` and `decisions/`) as the message stream the controller
    sends: turn requests after their `turn` event, the turn's rows after the answer; replace requests before
    the rows of their turn (`live`, as the phone reads the log) or right after the own faint (`placed`)."""
    with open(os.path.join(battle_dir, "events.json"), encoding="utf-8") as f:
        d = json.load(f)
    start, evs = d["events"][0], d["events"][1:]
    tag, v = start["battle"], start["v"]
    decs = []
    for p in sorted(glob.glob(os.path.join(battle_dir, "decisions", "*.json"))):
        with open(p, encoding="utf-8") as f:
            decs.append(json.load(f))
    picks = decs[0]["team"]["picks"] if decs and decs[0].get("team", {}).get("picks") else [0, 1, 2]
    turns = collections.OrderedDict()
    cur = None
    for e in evs:
        if e["k"] == "turn":
            cur = e["n"]
            turns[cur] = [e]
        else:
            turns.setdefault(cur, []).append(e)
    hdr = {"v": v, "battle": tag}
    out = [start, dict(hdr, type="preview_request"), {"picks": picks}]

    def events(es):
        if es:
            out.append(dict(hdr, type="events", events=es))

    def request(rec):
        kind = "replace" if rec["kind"] == "forced" else "turn"
        out.append(dict(hdr, type="decision_request", seq=rec["sequence"], kind=kind))
        out.append({"seq": rec["sequence"], "order": order_of(rec["order"])})

    by_turn = collections.defaultdict(list)
    for r in decs:
        by_turn[r["turn"]].append(r)
    for n, es in turns.items():
        head = es[:1]
        rows = es[1:]
        if n == 1:
            # the leads' rows are on the log before the first action screen
            k = next((i for i, e in enumerate(rows) if e["k"] in ("gap", "move")), len(rows))
            head, rows = es[: 1 + k], rows[k:]
        events(head)
        reqs = by_turn.get(n, [])
        turn_reqs = [r for r in reqs if r["kind"] != "forced"]
        repl = [r for r in reqs if r["kind"] == "forced"]
        for r in turn_reqs:
            request(r)
        if order == "live":
            for r in repl:
                request(r)
            events(rows)
        else:
            # after the own faint or pivot withdraw it answers
            rest = list(rows)
            for r in repl:
                k = next((i for i, e in enumerate(rest) if e.get("side") == "own" and (e["k"] == "faint" or (e["k"] == "withdraw" and e.get("pivot")))), len(rest) - 1)
                events(rest[: k + 1])
                rest = rest[k + 1:]
                request(r)
            events(rest)
    out.append(dict(hdr, type="battle_end", result=next((e["result"] for e in evs if e["k"] == "end"), "unknown")))
    return out


def to_live_order(msgs):
    """Each replace request (with its answer, refusals and retries) moved to right after the answer before
    it, ahead of the rows of its turn, as the phone sends it live. Returns (messages, moved, kept)."""
    hdr = {k: msgs[0][k] for k in ("v", "battle")}
    items = []
    for m in msgs:
        if m.get("type") == "events":
            for e in m["events"]:
                items.append(("ev", e))
            if "fallback" in m:
                items.append(("msg", dict(hdr, type="events", events=[], fallback=m["fallback"])))
        else:
            items.append(("msg", m))
    moved = kept = 0
    i = 0
    while i < len(items):
        t, m = items[i]
        if not (t == "msg" and m.get("type") == "decision_request" and m.get("kind") == "replace"):
            i += 1
            continue
        j = i
        group = []
        while j < len(items):
            tj, mj = items[j]
            req = tj == "msg" and mj.get("type") == "decision_request" and mj.get("kind") == "replace"
            ans = tj == "msg" and "order" in mj and "type" not in mj
            ref = tj == "ev" and mj["k"] == "refused"
            if not (req or ans or ref) or (req and group and not mj.get("refused") and not mj.get("retry")):
                break
            group.append(items[j])
            j += 1
        # back to the answer before it, if no `turn` row is in between
        k = i - 1
        rows = 0
        while k >= 0:
            tk, mk = items[k]
            if tk == "msg" and "order" in mk and "type" not in mk:
                break
            if tk == "ev" and mk["k"] == "turn":
                k = -1
                break
            if tk == "msg" and mk.get("type") == "decision_request":
                k = -1
                break
            rows += tk == "ev"
            k -= 1
        if k >= 0 and rows > 0:
            items = items[: k + 1] + group + items[k + 1: i] + items[j:]
            moved += 1
        else:
            kept += 1
        i = j
    out, buf = [], []
    for t, m in items:
        if t == "ev":
            buf.append(m)
            continue
        if buf:
            out.append(dict(hdr, type="events", events=buf))
            buf = []
        if m.get("type") == "events" and not m["events"] and out and out[-1].get("type") == "events" and "fallback" not in out[-1]:
            out[-1] = dict(out[-1], fallback=m["fallback"])
            continue
        out.append(m)
    if buf:
        out.append(dict(hdr, type="events", events=buf))
    return out, moved, kept


# ---------------------------------------------------------------- one battle

WORDS = None


def anomaly_kind(what):
    """Anomaly text with ids and numbers taken out (as replay.rs's kind_of)."""
    global WORDS
    if WORDS is None:
        WORDS = set()
        for p in ("battle/src/builder.rs", "battle/src/live.rs"):
            with open(os.path.join(ROOT, p), encoding="utf-8") as f:
                for lit in re.findall(r'"([^"\\]*)"', f.read()):
                    WORDS.update(re.findall(r"[a-z_]+", lit))
    out = []
    for w in what.split(" "):
        core = w.strip(',:()"')
        if any(c.isdigit() for c in core):
            out.append("N")
        elif re.fullmatch(r"[a-z0-9]+", core) and core not in WORDS:
            out.append("X")
        elif core.startswith("Some(") or core.startswith("Move") or core.startswith("Switch"):
            out.append("X")
        else:
            out.append(w)
    return " ".join(out)


class Picker:
    """A built-in player for decisions; the recorded picks for team preview (the team the game used)."""

    def __init__(self, name, picks, seed=0):
        self.p = _cg.RustPlayer(name, seed)
        self.picks = picks
        self.name = name

    def teampreview(self, battle):
        return self.picks

    def choose_move(self, battle):
        return self.p.choose_move(battle)


class Recorded:
    """The recorded orders (to compare Battles between stream orders with the same answers). `picks` "tower":
    tower's own picks instead of the recorded ones."""

    name = "recorded"

    def __init__(self, picks, orders):
        self.picks = picks
        self.orders = orders
        self.sess = None

    def teampreview(self, battle):
        if self.picks == "tower":
            return list(_cg.RustPlayer("tower", 0).teampreview(battle))
        return self.picks

    def choose_move(self, battle):
        o = self.orders.get(battle.decision)
        for a, x in self.sess.legal_orders():
            if x == o:
                return a
        return -1


def recorded_picks(msgs):
    return next((m["picks"] for m in msgs if "picks" in m and "type" not in m), [0, 1, 2])


def run_battle(msgs, player, keep_battles=False, picks=None):
    """One battle through a Session. Returns a dict of counts. `picks`: the preview answer of the "recorded"
    player (default: the recorded picks; "tower": tower's own)."""
    picks = recorded_picks(msgs) if picks is None else picks
    recorded = {}
    for m in msgs:
        if "order" in m and "type" not in m:
            recorded[m["seq"]] = m["order"]
    if player == "recorded":
        p = Recorded(picks, recorded)
        s = Session(p)
        p.sess = s
    elif player.endswith("-own"):
        # the built-in path alone (its own picks): what a live run times
        s = Session(player[:-4])
    else:
        s = Session(Picker(player, picks))
    r = collections.Counter()
    times = []
    battles = {}
    legal_at = {}
    pending = None
    since_rows, since_turn = 0, False
    fallback_seqs = set()
    outcomes = collections.Counter()
    errors = []

    def settle(p, outcome):
        outcomes["%s %s %s" % (p["kind"], outcome, "inside" if p["inside"] else "outside")] += 1

    for m in msgs:
        if "type" not in m:
            if "order" in m:
                q = m["seq"]
                lo = legal_at.get(q)
                inside = lo is not None and m["order"] in lo["orders"]
                if pending is not None:
                    settle(pending, "superseded")
                pending = {"seq": q, "kind": lo["kind"] if lo else "turn", "order": m["order"], "inside": inside}
                since_rows, since_turn = 0, False
                ours = lo["reply"] if lo else None
                if ours is not None:
                    r["answers compared"] += 1
                    r["answers equal recorded"] += ours == m["order"]
            continue
        if m.get("type") == "events":
            if isinstance(m.get("fallback"), dict) and isinstance(m["fallback"].get("seq"), int):
                fallback_seqs.add(m["fallback"]["seq"])
            for e in m["events"]:
                if e["k"] == "turn":
                    since_turn = True
                elif e["k"] not in ("refused", "end", "gap"):
                    since_rows += 1
        if m.get("type") == "decision_request" and pending is not None:
            ref = (m.get("refused") or {}).get("order")
            if ref is not None and ref == pending["order"]:
                outcome = "refused"
            elif ref is not None:
                outcome = "refused (another order named)"
            elif m.get("kind") == pending["kind"] and since_rows == 0 and not since_turn:
                outcome = "re-asked"
            elif pending["seq"] in fallback_seqs:
                outcome = "fallback"
            else:
                outcome = "accepted"
            settle(pending, outcome)
            pending = None
        t0 = time.perf_counter()
        rep = s.handle(m)
        dt = time.perf_counter() - t0
        if m.get("type") == "decision_request":
            times.append(dt)
            r["decision requests"] += 1
            if rep is None or "order" not in rep:
                r["requests without an order"] += 1
                errors.append(rep)
            else:
                if rep.get("notes", {}).get("last_resort"):
                    r["no legal action"] += 1
                if "error" in rep:
                    r["stale or error replies"] += 1
            orders = [o for _, o in s.legal_orders()]
            legal_at[m["seq"]] = {"orders": orders, "kind": m["kind"], "reply": rep.get("order") if rep else None}
            if keep_battles:
                battles[m["seq"]] = (m["kind"], s.battle, tuple(json.dumps(o, sort_keys=True) for o in orders))
        elif rep is not None and "error" in rep:
            r["error replies"] += 1
            errors.append(rep)
    if pending is not None:
        if pending["seq"] in fallback_seqs:
            settle(pending, "fallback")
        elif since_rows == 0:
            settle(pending, "unconfirmed (no row after it)")
        else:
            settle(pending, "accepted")
    an = collections.Counter()
    for a in s.anomalies:
        an["%s: %s" % (a["source"], anomaly_kind(a["what"]))] += 1
    internal = [a for a in s.anomalies if a["source"] in ("internal", "python")]
    r["completed"] = int(not internal and not errors and s.ended)
    return {"counts": r, "outcomes": outcomes, "anomalies": an, "times": times, "battles": battles, "errors": errors[:3], "internal": [a["what"] for a in internal][:3], "session": s}


# ---------------------------------------------------------------- the run

def add(into, c):
    for k, v in c.items():
        into[k] += v


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out", nargs="?")
    ap.add_argument("streams", nargs="*")
    ap.add_argument("--phone", action="append", default=[])
    ap.add_argument("--players", default="tower,max_damage")
    ap.add_argument("--phone-stream", default=None)
    ap.add_argument("--order", default="live")
    ap.add_argument("--picks-only", action="store_true", help="only the picks check (S8-1): OUT/picks.json")
    ap.add_argument("--restart-only", action="store_true", help="only the restart check (S8-2): OUT/restart.json")
    ap.add_argument("--second-fallback-only", action="store_true", help="only the second-fallback check (S10-3): OUT/second-fallback.json")
    a = ap.parse_args()
    if a.phone_stream:
        msgs = phone_battle(a.phone_stream, a.order)
        with open(a.out, "w", encoding="utf-8", newline="\n") as f:
            for m in msgs:
                if "type" in m:
                    f.write(json.dumps(m, ensure_ascii=False) + "\n")
        print("wrote", a.out, len(msgs))
        return
    os.makedirs(a.out, exist_ok=True)
    sources = []
    for st in a.streams:
        for name, msgs in stream_battles(st):
            sources.append((os.path.basename(st).split(".jsonl")[0], name, msgs))
    for pdir in a.phone:
        sources.append(("phone-live", os.path.basename(os.path.dirname(pdir)) + "__" + os.path.basename(pdir), phone_battle(pdir, "live")))
        sources.append(("phone-placed", os.path.basename(os.path.dirname(pdir)) + "__" + os.path.basename(pdir), phone_battle(pdir, "placed")))
    picks_sources = [x for x in sources if x[0] != "phone-placed"]
    if a.restart_only:
        by = collections.defaultdict(list)
        for x in picks_sources:
            by[x[0]].append(x)
        res = {src: restart_played(xs) for src, xs in by.items()}
        print("restart_played", json.dumps(res), flush=True)
        with open(os.path.join(a.out, "restart.json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(res, f, indent=1, ensure_ascii=False)
        return
    if a.second_fallback_only:
        by = collections.defaultdict(list)
        for x in picks_sources:
            by[x[0]].append(x)
        res = {src: second_fallback(xs) for src, xs in by.items()}
        with open(os.path.join(a.out, "second-fallback.json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(res, f, indent=1, ensure_ascii=False)
        return
    if a.picks_only:
        by = collections.defaultdict(list)
        for x in picks_sources:
            by[x[0]].append(x)
        res = {src: picks_check(xs) for src, xs in by.items()}
        with open(os.path.join(a.out, "picks.json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(res, f, indent=1, ensure_ascii=False)
        return
    summary = {}
    per_battle = open(os.path.join(a.out, "per-battle.jsonl"), "w", encoding="utf-8", newline="\n")
    for player in a.players.split(","):
        by_src = collections.defaultdict(lambda: {"battles": 0, "counts": collections.Counter(), "outcomes": collections.Counter(), "anomalies": collections.Counter(), "times": []})
        for src, name, msgs in sources:
            res = run_battle(msgs, player)
            agg = by_src[src]
            agg["battles"] += 1
            add(agg["counts"], res["counts"])
            add(agg["outcomes"], res["outcomes"])
            add(agg["anomalies"], res["anomalies"])
            agg["times"] += res["times"]
            per_battle.write(json.dumps({"player": player, "source": src, "battle": name, "counts": res["counts"], "outcomes": res["outcomes"], "errors": res["errors"], "internal": res["internal"]}, ensure_ascii=False) + "\n")
        summary[player] = {}
        for src, agg in by_src.items():
            t = sorted(agg["times"])
            oc = agg["outcomes"]
            acc_in = sum(v for k, v in oc.items() if " accepted inside" in k)
            acc = sum(v for k, v in oc.items() if " accepted " in k)
            kinds = {}
            for kind in ("turn", "replace"):
                i = oc.get("%s accepted inside" % kind, 0)
                o = oc.get("%s accepted outside" % kind, 0)
                kinds[kind] = {"accepted": i + o, "inside": i, "share": round(i / (i + o), 4) if i + o else None}
            c = agg["counts"]
            summary[player][src] = {
                "battles": agg["battles"],
                "completed": c["completed"],
                "decision_requests": c["decision requests"],
                "requests_without_order": c["requests without an order"],
                "no_legal_action": c["no legal action"],
                "error_replies": c["error replies"],
                "answers_equal_recorded": round(c["answers equal recorded"] / max(1, c["answers compared"]), 4),
                "accepted_inside": acc_in,
                "accepted": acc,
                "accepted_inside_share": round(acc_in / acc, 4) if acc else None,
                "by_kind": kinds,
                "outcomes": dict(sorted(oc.items())),
                "anomalies_total": sum(agg["anomalies"].values()),
                "anomalies_top15": dict(agg["anomalies"].most_common(15)),
                "decision_ms": {"n": len(t), "median": round(1000 * statistics.median(t), 3) if t else None, "p99": round(1000 * t[int(0.99 * (len(t) - 1))], 3) if t else None, "max": round(1000 * t[-1], 3) if t else None},
            }
        print(player, json.dumps({k: {x: v[x] for x in ("battles", "completed", "decision_requests", "accepted_inside_share", "no_legal_action", "decision_ms")} for k, v in summary[player].items()}), flush=True)
    per_battle.close()
    summary["lv5q7"] = lv5q7([x for x in sources if x[0].startswith("stream") or x[0] == "phone-placed"])
    print("lv5q7", json.dumps(summary["lv5q7"]), flush=True)
    summary["restart"] = restart_cost([x for x in sources if x[0] == "phone-live"])
    print("restart", json.dumps(summary["restart"]), flush=True)
    by = collections.defaultdict(list)
    for x in picks_sources:
        by[x[0]].append(x)
    summary["picks"] = {src: picks_check(xs) for src, xs in by.items()}
    with open(os.path.join(a.out, "summary.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(summary, f, indent=1, ensure_ascii=False)


def lv5q7(sources):
    """The recorded orders replayed with each replace request where the stream placed it (after its rows)
    and where the phone sends it live (before them): legal sets at the replace requests, the Battle at every
    turn request, accepted orders inside the legal set, anomalies."""
    out = collections.Counter()
    shown = 0
    for src, name, msgs in sources:
        live, moved, kept = to_live_order(msgs)
        out["battles"] += 1
        out["replace groups moved"] += moved
        out["replace groups not moved"] += kept
        out["battles with a moved request"] += moved > 0
        a = run_battle(msgs, "recorded", keep_battles=True)
        b = run_battle(live, "recorded", keep_battles=True)
        for q, (kind, bt, lo) in a["battles"].items():
            if q not in b["battles"]:
                out["requests missing in live order"] += 1
                continue
            kb, bb, lb = b["battles"][q]
            same = _cg.live_battle_equal(bt, bb)
            if kind == "replace":
                out["replace requests"] += 1
                out["replace legal set equal"] += lo == lb
                out["replace Battle equal"] += same
            else:
                out["turn requests"] += 1
                out["turn Battle equal"] += same
                out["turn legal set equal"] += lo == lb
                if not same and shown < 8:
                    shown += 1
                    print("  turn Battle differs", src, name, q, _cg.live_battle_diff(bt, bb), file=sys.stderr)
        for k, v in a["outcomes"].items():
            out["placed " + k] += v
        for k, v in b["outcomes"].items():
            out["live " + k] += v
        out["placed anomalies"] += sum(a["anomalies"].values())
        out["live anomalies"] += sum(b["anomalies"].values())
        out["live completed"] += b["counts"]["completed"]
    return dict(sorted(out.items()))


PICK_MODES = ("recorded", "tower", "start", "fallback")


def with_picks(msgs, mode):
    """The stream as sent when CG answered the preview with other picks than the game used: `start` gives
    the recorded picks in `battle_start.picks` (S8-1a), `fallback` in a preview fallback on the first
    `events` message (S8-1b)."""
    rec = recorded_picks(msgs)
    out = [dict(m) for m in msgs]
    if mode == "start":
        out[0]["picks"] = rec
    elif mode == "fallback":
        i = next((k for k, m in enumerate(out) if m.get("type") == "events"), None)
        if i is not None:
            f = {"picks": rec, "reason": "overruled"}
            old = out[i].get("fallback")
            out[i]["fallback"] = f if old is None else [f] + (old if isinstance(old, list) else [old])
    return out


def picks_check(sources):
    """Decisions as recorded, picks per PICK_MODES: anomalies, picks corrected from the log, the legal set
    at each request against the one with the recorded picks, and recorded accepted orders inside it."""
    out = {}
    base = {}
    for mode in PICK_MODES:
        c = collections.Counter()
        for src, name, msgs in sources:
            ms = with_picks(msgs, mode)
            res = run_battle(ms, "recorded", keep_battles=True, picks=None if mode == "recorded" else "tower")
            s = res["session"]
            c["battles"] += 1
            c["completed"] += res["counts"]["completed"]
            c["anomalies"] += sum(res["anomalies"].values())
            c["anomalies 'not picked'"] += sum(v for k, v in res["anomalies"].items() if "not picked" in k)
            c["battles with a 'not picked' anomaly"] += any("not picked" in k for k in res["anomalies"])
            corr = [x for x in getattr(s, "corrections", []) if "our choice" not in x["what"]]
            c["picks corrections"] += len(corr)
            c["battles with a correction"] += bool(corr)
            c["no legal action"] += res["counts"]["no legal action"]
            rec = recorded_picks(msgs)
            got = list(s.battle.picks)
            c["final picks: lead right"] += got[:1] == rec[:1]
            c["final picks: same three"] += sorted(x for x in got if x is not None) == sorted(rec)
            c["final picks: same order"] += got == rec
            for k, v in res["outcomes"].items():
                if " accepted " in k:
                    c["accepted"] += v
                    c["accepted inside"] += v if k.endswith(" inside") else 0
            for q, (kind, _, lo) in res["battles"].items():
                key = (src, name, q)
                if mode == "recorded":
                    base[key] = lo
                    continue
                c["requests"] += 1
                c["legal set equal to recorded picks"] += base.get(key) == lo
        c["accepted inside share"] = round(c["accepted inside"] / c["accepted"], 4) if c["accepted"] else None
        if c["requests"]:
            c["legal set equal share"] = round(c["legal set equal to recorded picks"] / c["requests"], 4)
        out[mode] = dict(sorted(c.items()))
        print("picks", mode, json.dumps(out[mode]), flush=True)
    return out


def add_fallback(msgs, i, f):
    """`f` first among the fallbacks of message i (an `events` message)."""
    old = msgs[i].get("fallback")
    msgs[i] = dict(msgs[i], fallback=f if old is None else [f] + (old if isinstance(old, list) else [old]))


SECOND_MODES = ("wrong only", "second at entry", "second early")


def second_fallback(sources):
    """S10-3: the first preview fallback has a wrong Pokemon in back slot k (1 or 2: the first team index not
    brought); a second preview fallback with the true picks follows on the `events` message that carries the
    true back member's first `enter` ("second at entry"), or on the first `events` message after the first
    decision request ("second early"); "wrong only" has no second. Legal set at each request against the run
    with the true picks from the start; recorded accepted orders inside the legal set."""
    out = {}
    base = {}
    for src, name, msgs in sources:
        res = run_battle(msgs, "recorded", keep_battles=True)
        base[(src, name)] = {q: lo for q, (_, _, lo) in res["battles"].items()}
    for k in (1, 2):
        for mode in SECOND_MODES:
            c = collections.Counter()
            for src, name, msgs in sources:
                rec = recorded_picks(msgs)
                team = [x["species"] for x in msgs[0]["own_team"]]
                wrong = next(i for i in range(len(team)) if i not in rec)
                first = list(rec)
                first[k] = wrong
                ms = [dict(m) for m in msgs]
                evi = [i for i, m in enumerate(ms) if m.get("type") == "events"]
                if not evi:
                    continue
                add_fallback(ms, evi[0], {"picks": first, "reason": "overruled"})
                at = None
                if mode == "second at entry":
                    at = next((i for i in evi if any(e["k"] == "enter" and e.get("side") == "own" and e.get("mon") == team[rec[k]] for e in ms[i]["events"])), None)
                elif mode == "second early":
                    q = next((i for i, m in enumerate(ms) if m.get("type") == "decision_request"), None)
                    at = next((i for i in evi if q is not None and i > q), None)
                if at is not None:
                    add_fallback(ms, at, {"picks": rec, "reason": "corrected"})
                    # a second fallback on the first message rides behind the first
                    if at == evi[0]:
                        f = ms[at]["fallback"]
                        ms[at]["fallback"] = [f[1], f[0]] + f[2:]
                c["second fallbacks sent"] += at is not None
                res = run_battle(ms, "recorded", keep_battles=True)
                s = res["session"]
                c["battles"] += 1
                c["completed"] += res["counts"]["completed"]
                c["anomalies"] += sum(res["anomalies"].values())
                c["picks corrections"] += sum("picks" in x["what"] for x in s.corrections)
                c["choice corrections"] += sum("our choice" in x["what"] for x in s.corrections)
                c["no legal action"] += res["counts"]["no legal action"]
                c["final picks: same order"] += list(s.battle.picks) == rec
                for kk, v in res["outcomes"].items():
                    if " accepted " in kk:
                        c["accepted"] += v
                        c["accepted inside"] += v if kk.endswith(" inside") else 0
                b = base[(src, name)]
                for q, (_, _, lo) in res["battles"].items():
                    c["requests"] += 1
                    c["legal set equal to true picks"] += b.get(q) == lo
            c["accepted inside share"] = round(c["accepted inside"] / c["accepted"], 4) if c["accepted"] else None
            c["legal set equal share"] = round(c["legal set equal to true picks"] / c["requests"], 4) if c["requests"] else None
            out["slot %d: %s" % (k, mode)] = dict(sorted(c.items()))
            print("second", k, mode, json.dumps(out["slot %d: %s" % (k, mode)]), flush=True)
    return out


class Never:
    """A player that must not be asked (counts it if it is)."""

    name = "never"

    def __init__(self):
        self.asked = 0

    def teampreview(self, battle):
        self.asked += 1
        return [5, 4, 3]

    def choose_move(self, battle):
        self.asked += 1
        return battle.legal_actions()[-1]


def restart_played(sources):
    """S8-2: each battle played by tower with the recorded picks in battle_start, then sent again to a new
    Session whose Player must not be asked: requests marked `replay` with `played` = the first pass's
    answers (and, second, `replay` without `played`). Same answers, same final Battle, Player never asked."""
    out = collections.Counter()
    for src, name, msgs in sources:
        ms = [m for m in msgs if "type" in m]
        ms[0] = dict(ms[0], picks=recorded_picks(msgs))
        first = Session("tower")
        replies = [first.handle(m) for m in ms]
        played = {r["seq"]: r["order"] for r in replies if r and "order" in r and "error" not in r}
        sent = []
        for m in ms:
            if m["type"] == "preview_request":
                m = dict(m, replay=True)
            elif m["type"] == "decision_request" and m["seq"] in played:
                m = dict(m, played=played[m["seq"]])
            sent.append(m)
        p = Never()
        again = Session(p)
        rep = [again.handle(m) for m in sent]
        out["battles"] += 1
        out["requests"] += len(played)
        out["player asked"] += p.asked
        out["same answers"] += sum(1 for r in rep if r and "order" in r and played.get(r.get("seq")) == r["order"])
        out["final Battle equal"] += _cg.live_battle_equal(first.battle, again.battle)
        out["session notes"] += sum(1 for a in again.anomalies if a["source"] == "session")
        q = Never()
        unknown = Session(q)
        for m in sent:
            unknown.handle(dict(m, played=None, replay=True) if m["type"] == "decision_request" else m)
        out["no played: player asked"] += q.asked
        out["no played: final Battle equal"] += _cg.live_battle_equal(first.battle, unknown.battle)
    return dict(sorted(out.items()))


def restart_cost(sources):
    """Milliseconds to handle a battle again from battle_start after it was played (the answers come from the
    session's record of them), against the first pass."""
    res = {}
    for src, name, msgs in sources:
        ms = [m for m in msgs if "type" in m]
        s = Session("tower")
        t0 = time.perf_counter()
        first = [s.handle(m) for m in ms]
        t1 = time.perf_counter()
        again = [s.handle(m) for m in ms]
        t2 = time.perf_counter()
        key = lambda r: ((r or {}).get("order"), (r or {}).get("picks"))
        turns = max((e["n"] for m in ms if m.get("type") == "events" for e in m["events"] if e["k"] == "turn"), default=0)
        res[name] = {"messages": len(ms), "turns": turns, "first_pass_ms": round(1000 * (t1 - t0), 2), "restart_replay_ms": round(1000 * (t2 - t1), 2), "same_answers": [key(r) for r in first] == [key(r) for r in again], "restarts": s.restarts}
    return res


if __name__ == "__main__":
    main()
