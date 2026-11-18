#!/usr/bin/env python3
"""Scripted battles on Showdown and the new resolver: rare rules random play seldom reaches.

Each scenario in a JSON file gives the two teams (three sets each are enough: the rest repeat the first), the
picks, a seed and the choices of each step. Showdown plays in record mode; the engine plays the same choices
with Showdown's events forced; options, snapshots and events must be identical at every step, as in
`driver.py --sim`. A scenario may also name log lines that must appear (`expect`), to show the rule was reached.

    python tools/rig/scenarios.py tools/rig/data/stage5_scenarios.json [--rows]

With --rows the engine records its battle-log rows and they are compared with Showdown's lines (rows.py).
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import driver  # noqa: E402
import rows as rowcmp  # noqa: E402

SIDES = ("p1", "p2")


def full_team(sets):
    sets = [dict(s) for s in sets]
    for i, s in enumerate(sets):
        s.setdefault("name", f"Mon{i}")
        s.setdefault("level", 50)
        s.setdefault("gender", "M")
        s.setdefault("nature", "Serious")
        s.setdefault("evs", {"hp": 0, "atk": 0, "def": 0, "spa": 0, "spd": 0, "spe": 0})
        s.setdefault("item", "")
    while len(sets) < 6:
        x = dict(sets[0])
        x["name"] = f"Mon{len(sets)}"
        x["item"] = ""
        sets.append(x)
    return sets


def run(path, with_rows=False):
    with open(path, encoding="utf-8") as f:
        scenarios = json.load(f)
    oracle = driver.Proc("oracle", [driver.NODE, driver.ORACLE])
    engine = driver.Proc("engine", driver.ENGINE.split("|") + ["--engine", "sim"] + (["--rows"] if with_rows else []))
    stats = rowcmp.RowStats() if with_rows else None
    ok_all = True
    results = []
    for k, sc in enumerate(scenarios):
        bid = f"s{k}"
        teams = {s: full_team(sc["teams"][s]) for s in SIDES}
        picks = sc.get("picks", {"p1": [0, 1, 2], "p2": [0, 1, 2]})
        seed = sc.get("seed", [1, 2, 3, 4])
        r = oracle.ask({"op": "new", "id": bid, "seed": seed, "teams": teams, "picks": picks})
        status, logs, why = "identical", [], ""
        if not r.get("ok"):
            status, why = "oracle_error", r.get("error")
        else:
            e = engine.ask({"op": "new", "id": bid, "seed": seed, "teams": teams, "picks": picks,
                            "force": r.get("events", [])})
            logs += r.get("log", [])
            why = compare(r, e)
            if why:
                status = "different"
            elif stats is not None:
                stats.compare(r.get("log", []), e.get("rows", []), r["state"], f"{k}:-1", teams=teams, picks=picks)
            for i, want in enumerate(sc["steps"]):
                if status != "identical":
                    break
                ch = adapt(want, r["options"])
                r = oracle.ask({"op": "step", "id": bid, "choices": ch})
                if not r.get("ok"):
                    status, why = "oracle_error", f"step {i}: {r.get('error')}"
                    break
                e = engine.ask({"op": "step", "id": bid, "choices": ch, "force": r.get("events", [])})
                logs += r.get("log", [])
                why = compare(r, e)
                if why:
                    status, why = "different", f"step {i}: {why}"
                elif stats is not None:
                    stats.compare(r.get("log", []), e.get("rows", []), r["state"], f"{k}:{i}", teams=teams, picks=picks)
                if r["state"].get("ended"):
                    break
            if e.get("unimplemented"):
                status, why = "unimplemented", str(e.get("unimplemented"))
        missing = [x for x in sc.get("expect", []) if not any(x in line for line in logs)]
        if status == "identical" and missing:
            status, why = "not_reached", f"missing log lines {missing}"
        ok_all &= status == "identical"
        results.append({"name": sc["name"], "status": status, "why": why})
        print(f"{status:14} {sc['name']}" + (f"  -- {why}" if why else ""), flush=True)
        if os.environ.get("SCENARIO_LOG") and os.environ["SCENARIO_LOG"] in sc["name"]:
            for line in logs:
                if not line.startswith("|t:|") and line not in ("|", "|upkeep"):
                    print("    " + line)
        oracle.ask({"op": "release", "id": bid})
        engine.ask({"op": "release", "id": bid})
    oracle.close()
    engine.close()
    print(json.dumps({"scenarios": len(results), "identical": sum(r["status"] == "identical" for r in results)}))
    if stats is not None:
        s = stats.summary()
        print(json.dumps({"rows_steps": s["steps"], "rows_steps_equal": s["steps_equal"], "disagreements": s["disagreements"]}))
    return ok_all


def adapt(want, opts):
    """The scripted choice where it is legal; otherwise the first legal one (a forced replacement, a lock)."""
    out = {}
    for s in SIDES:
        o = opts.get(s, {})
        k = o.get("kind")
        w = want.get(s, "")
        parts = w.split()
        if k == "turn":
            moves, sw = o.get("moves", []), o.get("switches", [])
            if parts[:1] == ["move"] and parts[1].isdigit() and int(parts[1]) in moves:
                out[s] = f"move {parts[1]}" + (" mega" if "mega" in parts and o.get("mega") else "")
            elif parts[:1] == ["switch"] and int(parts[1]) in sw:
                out[s] = w
            elif moves:
                out[s] = f"move {moves[0]}"
            else:
                out[s] = f"switch {sw[0]}"
        elif k == "replace":
            sw = o.get("switches", [])
            out[s] = w if parts[:1] == ["switch"] and int(parts[1]) in sw else f"switch {sw[0]}"
    return out


def compare(r, e):
    if not e.get("ok"):
        return f"engine: {e.get('error')} {e.get('detail', '')}"
    if driver.canon(r.get("refused")) != driver.canon(e.get("refused")):
        return f"refused {r.get('refused')} vs {e.get('refused')}"
    of = driver.compare_options(r["options"], e.get("options", {}))
    if of:
        return f"options {of}"
    sd = driver.diff(r["state"], e.get("state", {}))
    if sd:
        return f"state {sd[:5]}"
    if driver.canon(r.get("events", [])) != driver.canon(e.get("events", [])):
        return f"events {r.get('events')} vs {e.get('events')}"
    return ""


if __name__ == "__main__":
    sys.exit(0 if run(sys.argv[1], "--rows" in sys.argv[2:]) else 1)
