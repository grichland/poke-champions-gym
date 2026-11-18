"""Benchmarks of the low-latency observation (rl2.features_ll) on recorded states.
  python -m rl2.scripts.ll_bench <time-base|time-belief|time-ll|parts|dmg|parity> --live <dir> --sim <glob> --out <json>
States: live decision files (rebuilt with pcsa.battle.state.replay) and simulator games (rl2.archive sidecars /
validation games.jsonl.gz, rebuilt with rl.imitate_rebuild). Every mode stops at --minutes and reports what finished."""

from __future__ import annotations

import argparse
import glob
import gzip
import json
import time
from collections import Counter
from pathlib import Path

import numpy as np

GOLD = {"Gholdengo", "Garchomp", "Golisopod", "Primarina", "Mamoswine", "Cinderace"}


class Clock:
    def __init__(self, minutes: float) -> None:
        self.end = time.perf_counter() + minutes * 60.0

    def over(self) -> bool:
        return time.perf_counter() > self.end


def fail_key(stage: str, error: Exception) -> str:
    return f"{stage}: {type(error).__name__}: {str(error)[:90]}"


# States ---------------------------------------------------------------------------------------------------------
def live_games(root: str, want: int, failures: Counter, timing: list) -> list:
    """[(game id, [(name, battle, legal, forced file), ...])] of gold-team live decisions, in play order."""
    from pcsa.battle.state import replay
    from pcsa.players import mirror

    games, n = [], 0
    for bdir in sorted(glob.glob(root + "/*-run/battle-*")):
        if n >= want:
            break
        try:
            roster = json.loads(Path(bdir, "battle.json").read_text(encoding="utf-8")).get("roster") or []
        except (OSError, ValueError):
            continue
        if set(roster) != GOLD:
            continue
        states = []
        for path in sorted(glob.glob(bdir + "/decisions/*-action.json") + glob.glob(bdir + "/decisions/*-forced.json"),
                           key=lambda p: Path(p).name):
            d = json.loads(Path(path).read_text(encoding="utf-8"))
            if not d.get("protocol") or not d.get("request"):
                failures["live: no protocol or request"] += 1
                continue
            started = time.perf_counter()
            try:
                battle = replay(d["protocol"], d["request"], Path(path).stem)
                legal = mirror.legal_actions(battle)
            except Exception as error:  # noqa: BLE001 - counted by cause
                failures[fail_key("live rebuild", error)] += 1
                continue
            timing.append(time.perf_counter() - started)
            if not legal:
                failures["live: no legal action"] += 1
                continue
            wanted = set(d.get("legal") or [])
            kept = tuple(a for a in legal if a.order is not None and a.order.message in wanted)
            states.append((path, battle, kept or legal, path.endswith("-forced.json")))
        if states:
            games.append((bdir, states))
            n += len(states)
    return games


def sim_games(pattern: str, want: int, failures: Counter, timing: list, previews: list | None = None) -> list:
    """Simulator games with a team export and the seat's protocol, rebuilt decision by decision."""
    from pcsa.battle.adapter import ChampionsBattle
    from pcsa.battle.state import USERNAME, team_preview_request
    from pcsa.players import mirror
    from poke_env.battle import Battle
    from poke_env.data.normalize import to_id_str

    from rl import imitate_rebuild as ir

    rb = ir.Rebuilder()
    games, n = [], 0
    for path in sorted(glob.glob(pattern, recursive=True)):
        if n >= want:
            break
        with gzip.open(path, "rt", encoding="utf-8") as fh:
            rows = [json.loads(x) for x in fh if x.strip()]
        for k, row in enumerate(rows):
            if n >= want:
                break
            if not row.get("protocol") or not row.get("team") or row.get("seat") not in ("p1", "p2"):
                failures["sim: no protocol, team or seat"] += 1
                continue
            uid = f"{Path(path).name}#{k}"
            try:
                lines = mirror.as_p1(row["seat"], [x for x in row["protocol"].split("\n") if x], {})[0]
                team = ir.own_team(row["team"])
                picks = []
                for line in lines:
                    if line.startswith(("|switch|p1a: ", "|drag|p1a: ")):
                        name = line.split("|")[2].split(": ", 1)[1]
                        if name not in picks:
                            picks.append(name)
                picks += [m.name for m in team.members if to_id_str(m.name) not in {to_id_str(p) for p in picks}]
                record = {"protocol": row["protocol"], "llm_side": row["seat"],
                          "teams": {"llm": row["team"], "opp": row.get("opp_team")}, "picks": {"llm": picks[:3]}}
                g = rb.start(record, uid)
                points = ir.decision_points(g.lines)
            except Exception as error:  # noqa: BLE001
                failures[fail_key("sim start", error)] += 1
                continue
            if previews is not None and g.preview_at:
                try:
                    battle = (Battle if g.side == "p1" else ChampionsBattle)(uid + "-preview", USERNAME, ir.LOGGER, 9)
                    battle.player_role = "p1"
                    ir.feed(battle, g.lines[: g.preview_at])
                    battle.parse_request(team_preview_request(g.team))
                    previews.append((uid, battle))
                except Exception as error:  # noqa: BLE001
                    failures[fail_key("sim preview rebuild", error)] += 1
            states = []
            for cut, forced in points:
                started = time.perf_counter()
                try:
                    battle = rb.battle_at(g, cut, forced)
                    legal = mirror.legal_actions(battle)
                except Exception as error:  # noqa: BLE001
                    failures[fail_key("sim rebuild", error)] += 1
                    break  # later cuts of a broken game are not independent failures
                timing.append(time.perf_counter() - started)
                if not legal:
                    failures["sim: no legal action"] += 1
                    continue
                states.append((f"{uid}@{cut}", battle, legal, forced))
            if states:
                games.append((uid, states))
                n += len(states)
    return games


def load(args, failures: Counter, previews: list | None = None) -> tuple[list, dict]:
    t_live, t_sim = [], []
    live = live_games(args.live, args.n_live, failures, t_live) if args.live and args.n_live else []
    sim = sim_games(args.sim, args.n_sim, failures, t_sim, previews) if args.sim and args.n_sim else []
    info = {
        "live_games": len(live), "live_states": sum(len(s) for _, s in live),
        "live_forced": sum(1 for _, s in live for x in s if x[3]),
        "sim_games": len(sim), "sim_states": sum(len(s) for _, s in sim),
        "sim_forced": sum(1 for _, s in sim for x in s if x[3]),
        "rebuild_ms_live": stats(t_live), "rebuild_ms_sim": stats(t_sim),
    }
    print("states", json.dumps(info), flush=True)
    return [("live", *g) for g in live] + [("sim", *g) for g in sim], info


def stats(seconds: list) -> dict:
    if not seconds:
        return {"n": 0}
    ms = np.asarray(seconds) * 1000.0
    return {"n": int(ms.size), "mean": round(float(ms.mean()), 4), "median": round(float(np.median(ms)), 4),
            "p95": round(float(np.percentile(ms, 95)), 4)}


# Timing ---------------------------------------------------------------------------------------------------------
def variants(mode: str) -> dict:
    """name -> (per-game state factory, per-decision call)."""
    from rl2 import features as fx
    from rl2 import features_ll as ll

    if mode == "time-base":
        return {"base": (lambda: None, lambda s, b, legal: fx.observe(b, legal))}
    if mode == "time-belief":
        from rl2.belief_features import new_battle_belief

        return {"belief+mega": (new_battle_belief,
                                lambda s, b, legal: fx.observe(b, legal, belief=s, feature_mode="belief+mega"))}
    out = {}
    for name, groups in ll.CONFIGS.items():
        out[name] = (ll.Tracker, lambda s, b, legal, g=groups: ll.observe(b, legal, tracker=s, groups=g))
    for name in ("raw", "raw+lookup+dmg"):
        groups = ll.CONFIGS[name]
        out[name + " (cold tracker)"] = (
            lambda: None, lambda s, b, legal, g=groups: ll.observe(b, legal, tracker=ll.Tracker(), groups=g))
    return out


def run_timing(args) -> dict:
    failures: Counter = Counter()
    games, info = load(args, failures)
    clock = Clock(args.minutes)
    out = {"mode": args.mode, "states": info, "repeats": args.repeats, "variants": {}}
    for name, (factory, call) in variants(args.mode).items():
        for kind, _, states in games[:3]:  # warm the process-wide tables (not timed)
            s = factory()
            for _, battle, legal, _ in states:
                try:
                    call(s, battle, legal)
                except Exception:  # noqa: BLE001
                    pass
        reps, errors, stopped = [], Counter(), False
        for rep in range(args.repeats):
            times = {"live": [], "sim": []}
            for kind, _, states in games:
                if clock.over():
                    stopped = True
                    break
                s = factory()
                for _, battle, legal, _ in states:
                    started = time.perf_counter()
                    try:
                        call(s, battle, legal)
                    except Exception as error:  # noqa: BLE001
                        errors[fail_key("observe", error)] += 1
                        continue
                    times[kind].append(time.perf_counter() - started)
            row = {"repeat": rep + 1, "complete": not stopped, "all": stats(times["live"] + times["sim"]),
                   "live": stats(times["live"]), "sim": stats(times["sim"])}
            reps.append(row)
            print(name, json.dumps(row), flush=True)
            if stopped:
                break
        out["variants"][name] = {"repeats": reps, "errors": dict(errors)}
    out["failures"] = dict(failures)
    return out


def run_parts(args) -> dict:
    """Per-part cost of the new observation: tracker feed, encode per group set, legal mask, last-turn token."""
    from rl import features as f1
    from rl2 import features as fx
    from rl2 import features_ll as ll

    failures: Counter = Counter()
    games, info = load(args, failures)
    clock = Clock(args.minutes)
    sets = {"encode none": frozenset(), "encode hist": ll.PURE, "encode hist+lookup": ll.CONFIGS["raw+lookup"],
            "encode hist+lookup+dmg": ll.ALL, "encode hist+dmg": frozenset({"hist", "dmg"})}
    parts: dict[str, list] = {k: [] for k in ("tracker feed (incremental)", "tracker feed (cold)", *sets,
                                                "legal mask", "last-turn token")}
    pc = time.perf_counter
    for rep in range(args.repeats + 1):  # the first pass warms the tables
        for _, _, states in games:
            if clock.over():
                break
            tracker = ll.Tracker()
            for _, battle, legal, _ in states:
                t0 = pc()
                tracker.feed(battle._replay_data)
                t1 = pc()
                ll.Tracker().feed(battle._replay_data)
                t2 = pc()
                row = [t1 - t0, t2 - t1]
                ll.encode(battle, legal, tracker, ll.ALL)  # not timed: the group sets below compare warm calls
                for groups in sets.values():
                    t = pc()
                    ll.encode(battle, legal, tracker, groups)
                    row.append(pc() - t)
                t = pc()
                mask = np.zeros(ll.ACTIONS, dtype=bool)
                for a in legal:
                    mask[f1.action_index(a, battle)] = True
                row.append(pc() - t)
                t = pc()
                fx.last_turn(battle, tracker.turns.turns)
                row.append(pc() - t)
                if rep:
                    for k, v in zip(parts, row, strict=True):
                        parts[k].append(v)
    res = {k: stats(v) for k, v in parts.items()}
    mean = {k: v.get("mean", 0.0) for k, v in res.items()}
    res["groups (mean ms)"] = {
        "raw rows + moves + field (encode none)": mean["encode none"],
        "hist columns": round(mean["encode hist"] - mean["encode none"], 4),
        "lookup": round(mean["encode hist+lookup"] - mean["encode hist"], 4),
        "dmg (after lookup)": round(mean["encode hist+lookup+dmg"] - mean["encode hist+lookup"], 4),
        "dmg (without lookup columns)": round(mean["encode hist+dmg"] - mean["encode hist"], 4),
        "tracker feed (incremental)": mean["tracker feed (incremental)"],
        "legal mask": mean["legal mask"], "last-turn token": mean["last-turn token"],
    }
    for k, v in res.items():
        print(k, json.dumps(v), flush=True)
    return {"mode": "parts", "states": info, "parts": res, "failures": dict(failures)}


# Damage accuracy ------------------------------------------------------------------------------------------------
def run_dmg(args) -> dict:
    """The closed-form estimate against the engine (pcsa.players.threat's fighters: our exact set, their assumed
    set), in fractions of the target's max HP, and its KO flags against threat._ko_moves."""
    from pcsa.battle.builds import Catalog
    from pcsa.battle.damage import damage
    from pcsa.players import threat
    from pcsa.players.opponent import conditions, move_power
    from poke_env.battle import Move

    from rl import features as f1
    from rl2 import features_ll as ll

    failures: Counter = Counter()
    games, info = load(args, failures)
    clock = Clock(args.minutes)
    catalog = Catalog.load(seen=True)
    acc = {"ours": {"err": [], "inside": [], "eng0_mine": 0, "mine0_eng": 0, "width": []},
           "theirs": {"err": [], "inside": [], "eng0_mine": 0, "mine0_eng": 0, "width": []}}
    ko = Counter()
    examples = []

    def engine(battle, att, dfn, fa, fd, move) -> tuple[float, float]:
        low, high = damage(fa, fd, move.id, conditions(battle, dfn), move_power(att, dfn, fa, fd, move))
        return low, high

    def compare(side: str, low: float, high: float, e_low: float, e_high: float, hits: float) -> None:
        a = acc[side]
        e_mid = (e_low + e_high) / 2 * hits
        if e_high <= 0 and high <= 0:
            return
        if e_high <= 0:
            a["eng0_mine"] += 1
            return
        if high <= 0:
            a["mine0_eng"] += 1
            return
        a["err"].append(abs((low + high) / 2 - e_mid) * 100.0)
        a["inside"].append(low - 1e-9 <= e_mid <= high + 1e-9)
        a["width"].append((high - low) * 100.0)

    for kind, _, states in games:
        if clock.over():
            ko["stopped at the time limit"] += 1
            break
        tracker = ll.Tracker()
        for name, battle, legal, forced in states:
            detail: dict = {}
            try:
                obs = ll.observe(battle, legal, tracker=tracker, groups=ll.ALL, detail=detail)
            except Exception as error:  # noqa: BLE001
                failures[fail_key("ll observe", error)] += 1
                continue
            me = next((m for m in battle.team.values() if m.active), None)
            opp = battle.opponent_active_pokemon
            if forced or me is None or opp is None or opp.fainted or me.fainted:
                continue
            try:
                fa = threat.fighters(battle, me, catalog, None)
                fd = threat.fighters(battle, opp, catalog, None)
                kos = threat._ko_moves(battle, me, opp, catalog, None)
            except Exception as error:  # noqa: BLE001
                failures[fail_key("engine", error)] += 1
                continue
            ko["action decisions"] += 1
            mine = f1._slot_moves(me, legal)
            legal_moves = {a.order.order.id for a in legal if a.action.kind == "move" and a.order is not None
                           and isinstance(a.order.order, Move)}
            ko_ids = {m.id for _, m in kos if m.id in legal_moves}
            ko["decisions with a legal KO move (engine)"] += bool(ko_ids)
            flags = obs.rows[0, ll.DMG_AT:ll.DMG_AT + ll.DMG].reshape(4, 4)
            any_possible = any_certain = False
            for j, move in enumerate(mine):
                if move is None or move.category.name == "STATUS":
                    continue
                try:
                    e_low, e_high = engine(battle, me, opp, fa, fd, move)
                except Exception as error:  # noqa: BLE001
                    failures[fail_key("engine damage", error)] += 1
                    continue
                compare("ours", float(detail["low"][0, j]), float(detail["high"][0, j]), e_low, e_high,
                        threat._hits(move))
                if move.id not in legal_moves:
                    continue
                certain, possible = bool(flags[j, 2]), bool(flags[j, 3])
                if move.id in ko_ids:
                    ko["KO moves (engine)"] += 1
                    ko["KO moves flagged possible"] += possible
                    ko["KO moves flagged certain"] += certain
                    any_possible, any_certain = any_possible or possible, any_certain or certain
                    if not possible and len(examples) < 60:
                        examples.append({"state": name, "move": move.id, "hp": round(float(opp.current_hp_fraction), 3),
                                         "mine": [round(float(detail["low"][0, j]), 3),
                                                  round(float(detail["high"][0, j]), 3)],
                                         "engine": [round(e_low, 3), round(e_high, 3)], "target": opp.species})
                else:
                    ko["legal damaging moves that do not KO (engine)"] += 1
                    ko["non-KO moves flagged possible"] += possible
                    ko["non-KO moves flagged certain"] += certain
            if ko_ids:
                ko["KO decisions with a KO move flagged possible"] += any_possible
                ko["KO decisions with a KO move flagged certain"] += any_certain
            theirs = [m for k, m in opp.moves.items() if ll.move_facts(k) is not None][:4]
            for j, move in enumerate(theirs):
                if move.category.name == "STATUS":
                    continue
                try:
                    e_low, e_high = engine(battle, opp, me, fd, fa, move)
                except Exception as error:  # noqa: BLE001
                    failures[fail_key("engine damage", error)] += 1
                    continue
                compare("theirs", float(detail["low"][6, j]), float(detail["high"][6, j]), e_low, e_high,
                        threat._hits(move))
    res = {}
    for side, a in acc.items():
        err = np.asarray(a["err"]) if a["err"] else np.zeros(1)
        res[side] = {
            "pairs": len(a["err"]), "midpoint_abs_error_pct_of_max_hp_mean": round(float(err.mean()), 2),
            "midpoint_abs_error_median": round(float(np.median(err)), 2),
            "midpoint_abs_error_p95": round(float(np.percentile(err, 95)), 2),
            "engine_mid_inside_min_max": round(float(np.mean(a["inside"])), 4) if a["inside"] else None,
            "mean_width_pct": round(float(np.mean(a["width"])), 2) if a["width"] else None,
            "engine_zero_mine_positive": a["eng0_mine"], "mine_zero_engine_positive": a["mine0_eng"],
        }
    k = dict(ko)
    n, non = max(1, k.get("KO moves (engine)", 0)), max(1, k.get("legal damaging moves that do not KO (engine)", 0))
    k["recall KO-possible (moves)"] = round(k.get("KO moves flagged possible", 0) / n, 4)
    k["recall KO-certain (moves)"] = round(k.get("KO moves flagged certain", 0) / n, 4)
    k["false-positive rate KO-possible (non-KO moves)"] = round(k.get("non-KO moves flagged possible", 0) / non, 4)
    k["false-positive rate KO-certain (non-KO moves)"] = round(k.get("non-KO moves flagged certain", 0) / non, 4)
    out = {"mode": "dmg", "states": info, "accuracy": res, "ko": k, "missed_examples": examples,
           "failures": dict(failures)}
    print(json.dumps({"accuracy": res, "ko": k, "failures": dict(failures)}, indent=1), flush=True)
    return out


# Parity ---------------------------------------------------------------------------------------------------------
def run_parity(args) -> dict:
    """Every state through the one code path, in every configuration: errors by cause, non-finite values, an empty
    mask, and whether the incremental tracker gives the cold tracker's observation."""
    from rl2 import features_ll as ll

    failures: Counter = Counter()
    previews: list = []
    games, info = load(args, failures, previews)
    clock = Clock(args.minutes)
    count = {"live": Counter(), "sim": Counter()}
    for kind, _, states in games:
        if clock.over():
            count[kind]["stopped at the time limit"] += 1
            break
        tracker = ll.Tracker()
        for name, battle, legal, _ in states:
            c = count[kind]
            c["states"] += 1
            ok = True
            warm = None
            for config, groups in ll.CONFIGS.items():
                try:
                    o = ll.observe(battle, legal, tracker=tracker, groups=groups)
                except Exception as error:  # noqa: BLE001
                    failures[fail_key(f"{kind} observe {config}", error)] += 1
                    ok = False
                    continue
                warm = o
                if o.rows.shape != (ll.ROWS, ll.WIDTH) or o.ids.shape != (ll.ROWS, ll.IDS) or o.field.shape != (
                        ll.FIELD,):
                    failures[f"{kind} {config}: shape"] += 1
                    ok = False
                if not all(np.isfinite(x).all() for x in (o.rows, o.field, o.last)):
                    failures[f"{kind} {config}: non-finite"] += 1
                    ok = False
                if not o.mask.any():
                    failures[f"{kind} {config}: empty mask"] += 1
                    ok = False
                if np.abs(o.rows).max() > 2.0 + 1e-6:
                    failures[f"{kind} {config}: value above 2"] += 1
                    ok = False
            if warm is not None:
                try:
                    cold = ll.observe(battle, legal, tracker=ll.Tracker(), groups=ll.ALL)
                    same = all(np.array_equal(getattr(warm, f), getattr(cold, f))
                               for f in ("rows", "ids", "field", "last", "last_ids", "mask"))
                    c["incremental tracker == cold tracker"] += same
                    c["incremental tracker != cold tracker"] += not same
                except Exception as error:  # noqa: BLE001
                    failures[fail_key(f"{kind} observe cold", error)] += 1
            c["ok in every configuration"] += ok
    pv = Counter()
    for uid, battle in previews:
        pv["preview states"] += 1
        try:
            obs = ll.preview_observations(battle, [(0, 1, 2), (1, 0, 2), (3, 4, 5)])
            good = all(np.isfinite(o.rows).all() and o.rows[0, ll.ACTIVE_COL] == 1.0 for o in obs)
            pv["ok" if good else "bad values"] += 1
        except Exception as error:  # noqa: BLE001
            failures[fail_key("preview_observations", error)] += 1
    out = {"mode": "parity", "states": info, "live": dict(count["live"]), "sim": dict(count["sim"]),
           "preview": dict(pv), "failures": dict(failures)}
    print(json.dumps({k: v for k, v in out.items() if k != "states"}, indent=1), flush=True)
    return out


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("mode", choices=("time-base", "time-belief", "time-ll", "parts", "dmg", "parity"))
    p.add_argument("--live", default="")
    p.add_argument("--sim", default="")
    p.add_argument("--n-live", type=int, default=2000)
    p.add_argument("--n-sim", type=int, default=2000)
    p.add_argument("--repeats", type=int, default=3)
    p.add_argument("--minutes", type=float, default=8.5)
    p.add_argument("--out", required=True)
    args = p.parse_args()
    started = time.perf_counter()
    run = {"parts": run_parts, "dmg": run_dmg, "parity": run_parity}.get(args.mode, run_timing)
    out = run(args)
    out["seconds"] = round(time.perf_counter() - started, 1)
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(json.dumps(out, indent=1), encoding="utf-8")
    print("DONE", args.mode, out["seconds"], "s", flush=True)


if __name__ == "__main__":
    main()
