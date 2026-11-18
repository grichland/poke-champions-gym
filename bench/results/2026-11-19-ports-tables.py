"""Tables from the 2026-11-19-ports result files (read only)."""
import json
import math
import os
import sys

R = "bench/results/"
P = ["random", "max_damage", "tower", "best_matchup_max_damage", "disciplined", "endgame", "pilot_encore", "pilot_weather", "pilot_hazard", "pilot_priority"]
SHORT = {"random": "rand", "max_damage": "md", "tower": "tower", "best_matchup_max_damage": "bm", "disciplined": "disc", "endgame": "endg",
         "pilot_encore": "p_enc", "pilot_weather": "p_wea", "pilot_hazard": "p_haz", "pilot_priority": "p_pri"}


def rows(f):
    p = R + f
    return [json.loads(l) for l in open(p, encoding="utf-8")] if os.path.exists(p) else []


def wilson(k, n, z=1.96):
    if n == 0:
        return (0, 0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return c - h, c + h


def latency():
    print("## latency")
    for r in rows("2026-11-19-ports-latency.jsonl"):
        print("%-24s t%-2d %9.0f dec/s %7.2f us other_load %.1f%% disturbed %s/%s" % (r["player"], r["threads"], r["decisions_per_s"], r["us_per_decision"],
              100 * r["cpu_other_load"], r.get("rounds_other_load_over_10pct"), r["rounds"]))
    for r in rows("2026-11-19-ports-players-speed.jsonl"):
        print("speed %-24s %8.2f us/decision  previews/s %.0f illegal %d" % (r["player"], r["us_per_decision"], r["previews_per_s"], r["illegal"]))
    for r in rows("2026-11-19-ports-selfplay-1thread.jsonl"):
        print("self %-24s %8.0f dec/s %7.2f us/decision %.0f battles/s" % (r["a"], r["decisions_per_s"], 1e6 / r["decisions_per_s"], r["battles_per_s"]))


def rr():
    rs = rows("2026-11-19-ports-roundrobin.jsonl")
    win = {}
    stats = {p: {"games": 0, "turns": 0.0, "vs": 0, "fired": {}} for p in P}
    zero = []
    for r in rs:
        a, b, n = r["a"], r["b"], r["games"]
        win[(a, b)] = (r["a_wins"], n, r["mean_turns"], r["ties"], r["truncated"])
        win[(b, a)] = (r["b_wins"], n, r["mean_turns"], r["ties"], r["truncated"])
        for side, p in (("a", a), ("b", b)):
            s = stats[p]
            s["games"] += n
            s["vs"] += r[side + "_voluntary_switches"]
            for k, v in r[side + "_fired"].items():
                s["fired"][k] = s["fired"].get(k, 0) + v
        z = {k: r[k] for k in ("anomalies", "check_a_disagreements", "prior0_actions", "truth_errors", "hidden_differences", "player_fallbacks", "errors")}
        z["check_b"] = r["check_b_agreement_rate"]
        z["capped"] = r["time_capped"]
        if any(v for k, v in z.items() if k not in ("check_b", "errors")) or z["check_b"] != 1.0 or z["errors"]:
            zero.append((a, b, z))
    print("## round robin: %d pairings, games %s" % (len(rs), sorted({r["games"] for r in rs})))
    print("nonzero checks:", zero or "none")
    print("| row \\ col | " + " | ".join(SHORT[p] for p in P) + " | mean |")
    print("|" + "---|" * (len(P) + 2))
    for a in P:
        cells, tot = [], [0, 0]
        for b in P:
            if a == b:
                cells.append("-")
                continue
            if (a, b) not in win:
                cells.append("?")
                continue
            k, n = win[(a, b)][:2]
            lo, hi = wilson(k, n)
            cells.append("%.1f [%.1f-%.1f]" % (100 * k / n, 100 * lo, 100 * hi))
            tot[0] += k
            tot[1] += n
        cells.append("%.1f" % (100 * tot[0] / max(1, tot[1])))
        print("| %s | %s |" % (SHORT[a], " | ".join(cells)))
    print("mean turns per pairing:")
    for a in P:
        print(SHORT[a], " ".join("%5.2f" % win[(a, b)][2] if (a, b) in win else "  -  " for b in P))
    print("ties/truncated:", sum(r["ties"] for r in rs), sum(r["truncated"] for r in rs))
    print("## per player over all its games")
    for p in P:
        s = stats[p]
        g = max(1, s["games"])
        f = s["fired"]
        print("%-24s games %6d vol.switches/game %.3f" % (p, s["games"], s["vs"] / g))
        if f:
            print("    " + ", ".join("%s %.3f" % (k, v / g) for k, v in sorted(f.items(), key=lambda x: -x[1])))
    print("## vs best_matchup_max_damage column")
    for p in P:
        if (p, "best_matchup_max_damage") in win:
            k, n = win[(p, "best_matchup_max_damage")][:2]
            lo, hi = wilson(k, n)
            print("%-24s %d-%d %.1f%% [%.1f-%.1f]" % (p, k, n - k, 100 * k / n, 100 * lo, 100 * hi))
    return rs


def pools(rr_rows):
    print("## pilot pools")
    for r in rows("2026-11-19-ports-pilot-pools.jsonl"):
        n = r["games"]
        lo, hi = wilson(r["a_wins"], n)
        f = r["a_fired"]
        rule = sum(v for k, v in f.items() if k not in ("decisions", "applies"))
        print("%-15s vs %-10s %.1f%% [%.1f-%.1f] turns %.2f fired/game %.3f applies/game %.3f fired/decision %.3f vol.sw/game %.3f checks %s" % (
            r["a"], r["b"], 100 * r["a_wins"] / n, 100 * lo, 100 * hi, r["mean_turns"], rule / n, f.get("applies", 0) / n, rule / max(1, f.get("decisions", 1)),
            r["a_voluntary_switches"] / n,
            (r["anomalies"], r["check_a_disagreements"], r["check_b_agreement_rate"], r["prior0_actions"], r["truth_errors"], r["hidden_differences"], r["player_fallbacks"])))
        print("     " + ", ".join("%s %.3f" % (k, v / n) for k, v in f.items() if k != "decisions"))
    for r in rr_rows:
        for side in ("a", "b"):
            p, o = r[side], r["b" if side == "a" else "a"]
            if p.startswith("pilot") and o in ("tower", "max_damage"):
                n = r["games"]
                f = r[side + "_fired"]
                rule = sum(v for k, v in f.items() if k not in ("decisions", "applies"))
                w = r[side + "_wins"]
                print("shipped %-15s vs %-10s %.1f%% fired/game %.3f applies/game %.3f fired/decision %.3f" % (p, o, 100 * w / n, rule / n, f.get("applies", 0) / n, rule / max(1, f.get("decisions", 1))))


latency()
rs = rr()
pools(rs)
