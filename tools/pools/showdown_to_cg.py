"""Showdown team exports ('=== name ===' blocks, as rl.teams' pool reader takes them) to CG's teams.json
format: a list of teams, each a list of six sets {species, item, ability, moves, nature, evs, level, gender}.
Champions stat points are written as `EVs:` in the exports and go to `evs` unchanged. Gender is read from
a "(M)"/"(F)" mark; without one, M or F is drawn from a seed per team and set (CG ignores it for species of
one gender or none). Nothing is checked here: load the output with cg.check_teams.

Usage: python tools/pools/showdown_to_cg.py IN.txt OUT.json
"""

import json
import random
import re
import sys

STATS = {"HP": "hp", "Atk": "atk", "Def": "def", "SpA": "spa", "SpD": "spd", "Spe": "spe"}


def parse_set(lines, rng):
    head = lines[0].strip()
    item = None
    if " @ " in head:
        head, item = head.rsplit(" @ ", 1)
    gender = None
    m = re.search(r"\((M|F)\)\s*$", head)
    if m:
        gender = m.group(1)
        head = head[: m.start()].strip()
    m = re.match(r"^.*\(([^()]+)\)\s*$", head)
    species = m.group(1) if m else head.strip()
    s = {"species": species, "item": item, "ability": None, "moves": [], "nature": None,
         "evs": {k: 0 for k in STATS.values()}, "level": 50}
    for line in lines[1:]:
        line = line.strip()
        if line.startswith("Ability:"):
            s["ability"] = line.split(":", 1)[1].strip()
        elif line.startswith("Level:"):
            s["level"] = int(line.split(":", 1)[1])
        elif line.startswith("EVs:"):
            for part in line.split(":", 1)[1].split("/"):
                v, k = part.split()
                s["evs"][STATS[k]] = int(v)
        elif line.endswith(" Nature"):
            s["nature"] = line[: -len(" Nature")].strip()
        elif line.startswith("- "):
            s["moves"].append(line[2:].strip())
    s["gender"] = gender or rng.choice(["M", "F"])
    return s


def parse(text):
    teams, names = [], []
    for block in re.split(r"^=== (.+?) ===\s*$", text, flags=re.M)[1:]:
        if not names or len(names) == len(teams):
            names.append(block.strip())
            continue
        sets = [p.strip().splitlines() for p in re.split(r"\n\s*\n", block.strip()) if p.strip()]
        rng = random.Random("cg-pool:" + names[-1])
        teams.append([parse_set(lines, rng) for lines in sets])
    return names, teams


def main():
    src, out = sys.argv[1], sys.argv[2]
    with open(src, encoding="utf-8") as f:
        names, teams = parse(f.read())
    with open(out, "w", encoding="utf-8") as f:
        json.dump(teams, f, indent=1)
        f.write("\n")
    print("%s: %d teams -> %s" % (src, len(teams), out))


if __name__ == "__main__":
    main()
