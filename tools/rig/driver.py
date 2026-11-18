#!/usr/bin/env python3
"""Test-rig driver: plays random battles on Showdown (the oracle) and checks every step against the engine
(`rig --engine sim`, the resolver). See tools/rig/CONTRACT.md.

Showdown plays each step in record mode, the engine plays the same choices with Showdown's events forced, and
the events, the options and the snapshot must be identical (no search; any difference or tape error is a
failure and ends that battle). Writes one JSON line per failed step and a final summary line (kind
"summary") with the totals. `--sim` is accepted and changes nothing (it named the resolver when the rig had
two engines). --plain builds stage-1 battles: every Pokemon gets No Ability, no item, and four moves drawn from
tools/rig/data/plain_moves.json. --stage2 builds stage-2 battles the same way with four moves drawn from
tools/rig/data/stage2_moves.json (the moves whose rules need no ability, item or field effect). --stage3
draws from tools/rig/data/stage3_moves.json (field, side and slot conditions too); --focus3 gives each
Pokemon two of the moves stage 3 added and two of the stage-2 moves the field changes (that file's "added"
and "interacting" lists), so field rules come up in nearly every battle.

    python tools/rig/driver.py --sim --plain --battles 500 --seed 1 --out tools/rig/results/sim.jsonl.gz
    python tools/rig/driver.py --sim --stage2 --battles 500 --seed 1 --out tools/rig/results/sim2.jsonl.gz
    python tools/rig/driver.py --sim --stage3 --battles 500 --seed 1 --out tools/rig/results/sim3.jsonl.gz
    python tools/rig/driver.py --sim --focus3 --battles 500 --seed 1 --out tools/rig/results/sim3f.jsonl.gz

--stage4 builds stage-4 battles from tools/rig/data/stage4.json: legal species (no Megas, no battle-only forms)
with one of their legal abilities that the resolver implements, four moves from the stage-4 pool, no items.
--focus4 gives each Pokemon any implemented ability (Mega abilities included, put on the species directly) and
two moves from that file's "interacting" list, so rare ability rules come up often. A choice Showdown's request
offers but refuses (contract section 9) is drawn now and then; both programs must refuse it the same way.

    python tools/rig/driver.py --sim --stage4 --battles 500 --seed 1 --out tools/rig/results/sim4.jsonl.gz
    python tools/rig/driver.py --sim --focus4 --battles 500 --seed 1 --out tools/rig/results/sim4f.jsonl.gz

--stage5 builds stage-5 battles from tools/rig/data/stage5.json: legal species with one of their legal abilities, a
random legal item each (a Mega stone only on its species, no item twice on a team), four moves from the full legal
pool. --focus5 weights them to rare item, Mega and ability rules: Mega-capable species and stones more often, an
ability from that file's item_abilities list half the time, and moves from its item_moves and interacting lists.
--real plays the teams of tools/rig/data/teams.json exactly as written (items, abilities, moves, spreads,
natures), three of six brought at random.

    python tools/rig/driver.py --sim --stage5 --battles 500 --seed 1 --out tools/rig/results/sim5.jsonl.gz
    python tools/rig/driver.py --sim --focus5 --battles 500 --seed 1 --out tools/rig/results/sim5f.jsonl.gz
    python tools/rig/driver.py --sim --real --battles 500 --seed 1 --out tools/rig/results/real.jsonl.gz
"""
import argparse
import gzip
import json
import os
import queue
import random
import re
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rows as rowcmp  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
NODE = os.environ.get("RIG_NODE", r"C:\Users\user\Code\pokemon_champions\pcsa\.tools\node\node.exe")
ORACLE = os.path.join(REPO, "tools", "showdown-oracle", "oracle.js")
# RIG_ENGINE: another engine command, its arguments separated by "|" (tests use a stand-in)
ENGINE = os.environ.get("RIG_ENGINE", os.path.join(REPO, "target", "release", "rig.exe" if os.name == "nt" else "rig"))
TEAMS = os.path.join(REPO, "tools", "rig", "data", "teams.json")
PLAIN_MOVES = os.path.join(REPO, "tools", "rig", "data", "plain_moves.json")
STAGE2_MOVES = os.path.join(REPO, "tools", "rig", "data", "stage2_moves.json")
STAGE3_MOVES = os.path.join(REPO, "tools", "rig", "data", "stage3_moves.json")
STAGE4 = os.path.join(REPO, "tools", "rig", "data", "stage4.json")
STAGE5 = os.path.join(REPO, "tools", "rig", "data", "stage5.json")

SIDES = ("p1", "p2")
MAX_DECISIONS = 300  # per battle; a battle that runs longer is cut off and counted
TIMEOUT = 10.0  # seconds per request; a child that does not answer in time is restarted


def open_text(path, mode):
    """A text file, gzip-compressed when the name ends in .gz."""
    if path.endswith(".gz"):
        return gzip.open(path, mode + "t", encoding="utf-8")
    return open(path, mode, encoding="utf-8")


class Proc:
    """A JSON-lines child process: one request line in, one reply line out. A reply that does not come
    within `timeout` seconds kills and restarts the child; the reply is then an error with `timeout`."""

    def __init__(self, name, argv, timeout=TIMEOUT):
        self.name, self.argv, self.timeout = name, argv, timeout
        self.p = None
        self.restarts = 0
        self.start()

    def start(self):
        self.p = subprocess.Popen(self.argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, cwd=REPO, text=True, encoding="utf-8",
                                  bufsize=1)
        # a reader thread per child, so a silent child cannot block the driver
        self.lines = queue.Queue()
        threading.Thread(target=self._read, args=(self.p.stdout, self.lines), daemon=True).start()

    @staticmethod
    def _read(stream, q):
        try:
            for line in stream:
                q.put(line)
        except (OSError, ValueError):
            pass
        q.put("")  # end of output

    def ask(self, req):
        line = json.dumps(req, separators=(",", ":"))
        try:
            self.p.stdin.write(line + "\n")
            self.p.stdin.flush()
        except OSError as e:
            self.restart()
            return {"ok": False, "error": f"{self.name}: {e}", "crash": True}
        try:
            out = self.lines.get(timeout=self.timeout)
        except queue.Empty:
            self.restart()
            return {"ok": False, "error": f"{self.name}: no reply in {self.timeout:g} s", "timeout": True}
        if not out:
            self.restart()
            return {"ok": False, "error": f"{self.name}: process exited", "crash": True}
        return json.loads(out)

    def restart(self):
        self.restarts += 1
        try:
            self.p.kill()
            self.p.wait(timeout=10)
        except (OSError, subprocess.TimeoutExpired):
            pass
        self.start()

    def close(self):
        try:
            self.p.stdin.close()
            self.p.wait(timeout=10)
        except Exception:
            self.p.kill()


# ------------------------------------------------------------------------------------------- comparing

ABSENT = "<absent>"


def diff(a, b, path="", out=None):
    """Field paths where a (Showdown) and b (engine) differ, as [path, a, b]."""
    if out is None:
        out = []
    if isinstance(a, dict) and isinstance(b, dict):
        keys = list(a.keys()) + [k for k in b.keys() if k not in a]
        for k in keys:
            p = f"{path}.{k}" if path else k
            diff(a.get(k, ABSENT), b.get(k, ABSENT), p, out)
    elif isinstance(a, list) and isinstance(b, list) and a and b and all(isinstance(x, dict) for x in a + b):
        for i in range(max(len(a), len(b))):
            diff(a[i] if i < len(a) else ABSENT, b[i] if i < len(b) else ABSENT, f"{path}[{i}]", out)
    elif not same(a, b):
        out.append([path, a, b])
    return out


def same(a, b):
    # Python counts True == 1; the snapshot does not
    if isinstance(a, bool) or isinstance(b, bool):
        return a is b
    return a == b


def compare_options(sd, en):
    """Per side and field: the fields where the engine's options differ from Showdown's."""
    fails = []
    for s in SIDES:
        a, b = sd.get(s, {}), en.get(s, {})
        for f in ("kind", "moves", "mega", "switches", "hidden"):
            va, vb = a.get(f, ABSENT), b.get(f, ABSENT)
            if f in ("moves", "switches") and isinstance(va, list) and isinstance(vb, list):
                if sorted(map(str, va)) == sorted(map(str, vb)):
                    continue
            elif f == "hidden" and isinstance(va, dict) and isinstance(vb, dict):
                if all(sorted(va.get(k, [])) == sorted(vb.get(k, [])) for k in ("switches", "moves")):
                    continue
            elif same(va, vb):
                continue
            fails.append([f"{s}.{f}", va, vb])
    return fails


# ------------------------------------------------------------------------------------------- choosing

def choose(rng, opts, refuse=0.0):
    """A random legal choice per side from Showdown's options: a move four times in five when there is
    both a move and a switch; Mega Evolution when offered, two times in three. With `refuse`, a choice the
    request offers but the game refuses (options' `hidden`) is drawn with that probability when there is one."""
    out = {}
    for s in SIDES:
        o = opts.get(s, {})
        k = o.get("kind")
        h = o.get("hidden") or {}
        hidden = [f"switch {x}" for x in h.get("switches", [])] + [f"move {x}" for x in h.get("moves", [])]
        if k == "turn" and hidden and refuse and rng.random() < refuse:
            out[s] = rng.choice(hidden)
            continue
        if k == "replace":
            out[s] = f"switch {rng.choice(o['switches'])}"
        elif k == "turn":
            moves, sw = o.get("moves", []), o.get("switches", [])
            if moves and (not sw or rng.random() < 0.8):
                m = rng.choice(moves)
                mega = o.get("mega") and isinstance(m, int) and rng.random() < 2 / 3
                out[s] = f"move {m}" + (" mega" if mega else "")
            elif sw:
                out[s] = f"switch {rng.choice(sw)}"
    return out


def moves_used(log):
    """Move ids from Showdown's |move| lines."""
    out = []
    for line in log:
        parts = line.split("|")
        if len(parts) > 3 and parts[1] == "move":
            mid = "".join(c for c in parts[3].lower() if c.isalnum())
            if mid not in out:
                out.append(mid)
    return out


# ------------------------------------------------------------------------------------------- battles

def focus_team(team, rng, added, interacting, megas=False):
    """A stage-3 focus team: as plain_team, with two moves from `added` and two from `interacting`."""
    out = []
    for st in team:
        st = dict(st)
        st["ability"] = "No Ability"
        if not (megas and st.get("item", "").endswith("ite") or megas and st.get("item", "").endswith("itez")):
            st["item"] = ""
        st["moves"] = rng.sample(added, 2) + rng.sample(interacting, 2)
        out.append(st)
    return out


NATURES = ["adamant", "bashful", "bold", "brave", "calm", "careful", "docile", "gentle", "hardy", "hasty", "impish",
           "jolly", "lax", "lonely", "mild", "modest", "naive", "naughty", "quiet", "quirky", "rash", "relaxed", "sassy",
           "serious", "timid"]


def stage4_team(rng, data, teams, focus):
    """A stage-4 team: six distinct legal species, each with a legal implemented ability (with `focus`: any
    implemented ability, and two moves from the interacting list), four moves from the pool, no item; stat
    points and nature from a random ladder set."""
    impl = set(data["abilities"])
    pool = [sp for sp in data["species"] if focus or any(a in impl for a in sp["abilities"])]
    out = []
    for i, sp in enumerate(rng.sample(pool, 6)):
        donor = rng.choice(rng.choice(teams))
        if focus:
            ability = rng.choice(data["abilities"])
            moves = rng.sample(data["interacting"], 2)
            moves += rng.sample([m for m in data["moves"] if m not in moves], 2)
        else:
            ability = rng.choice([a for a in sp["abilities"] if a in impl])
            moves = rng.sample(data["moves"], 4)
        gender = sp["gender"] if sp["gender"] in ("M", "F", "N") else rng.choice(["M", "F"])
        out.append({"name": f"Mon{i}", "species": sp["species"], "item": "", "ability": ability, "moves": moves,
                    "nature": donor.get("nature") or rng.choice(NATURES), "evs": dict(donor["evs"]), "level": 50,
                    "gender": gender})
    return out


def stage5_team(rng, data, teams, focus):
    """A stage-5 team: six distinct legal species, each with a legal ability (with `focus`: often one of the
    item_abilities), a legal item (a stone only on its species; no item twice), four moves from the full pool (with
    `focus`: from the item_moves and interacting lists); stat points and nature from a random ladder set."""
    out = []
    used = set()
    megas = set(data["mega_species"])
    pool = data["species"]
    chosen = []
    while len(chosen) < 6:
        if focus and rng.random() < 0.4:
            sp = rng.choice([x for x in pool if x["species"] in megas])
        else:
            sp = rng.choice(pool)
        if sp not in chosen:
            chosen.append(sp)
    for i, sp in enumerate(chosen):
        donor = rng.choice(rng.choice(teams))
        if focus and rng.random() < 0.5:
            ability = rng.choice(data["item_abilities"])
        else:
            ability = rng.choice(sp["abilities"])
        item = ""
        stones = [x for x in sp["stones"] if x not in used]
        if stones and rng.random() < (0.7 if focus else 0.4):
            item = rng.choice(stones)
        elif rng.random() < 0.92:
            item = rng.choice([x for x in data["items"] if x not in used])
        if item:
            used.add(item)
        if focus:
            moves = []
            if rng.random() < 0.7:
                moves.append(rng.choice(data["item_moves"]))
            while len(moves) < 3:
                m = rng.choice(data["interacting"])
                if m not in moves:
                    moves.append(m)
            while len(moves) < 4:
                m = rng.choice(data["moves"])
                if m not in moves:
                    moves.append(m)
        else:
            moves = rng.sample(data["moves"], 4)
        gender = sp["gender"] if sp["gender"] in ("M", "F", "N") else rng.choice(["M", "F"])
        out.append({"name": f"Mon{i}", "species": sp["species"], "item": item, "ability": ability, "moves": moves,
                    "nature": donor.get("nature") or rng.choice(NATURES), "evs": dict(donor["evs"]), "level": 50,
                    "gender": gender})
    return out


def plain_team(team, rng, plain, nmoves=4, megas=False):
    """A stage-1 team: No Ability, no item (a Mega stone kept with `megas`), `nmoves` distinct plain moves;
    species, stat points, nature and gender kept."""
    out = []
    for st in team:
        st = dict(st)
        st["ability"] = "No Ability"
        if not (megas and st.get("item", "").endswith("ite") or megas and st.get("item", "").endswith("itez")):
            st["item"] = ""
        st["moves"] = rng.sample(plain, min(nmoves, len(plain)))
        out.append(st)
    return out


# what a step exercised, from Showdown's log
COVERAGE = {
    "struggle": lambda log: any(l.startswith("|move|") and "|Struggle|" in l for l in log),
    "crit": lambda log: any(l.startswith("|-crit|") for l in log),
    "miss": lambda log: any(l.startswith("|-miss|") for l in log),
    "immune": lambda log: any(l.startswith("|-immune|") for l in log),
    "drain": lambda log: any(l.startswith("|-heal|") and "[from] drain" in l for l in log),
    "recoil": lambda log: any(l.startswith("|-damage|") and "[from] Recoil" in l for l in log),
    "struggle_recoil": lambda log: any(l.startswith("|-damage|") and "[from] recoil" in l for l in log),
    "multihit": lambda log: any(l.startswith("|-hitcount|") for l in log),
    "faint": lambda log: any(l.startswith("|faint|") for l in log),
    "two_faints": lambda log: sum(l.startswith("|faint|") for l in log) >= 2,
    "win": lambda log: any(l.startswith("|win|") for l in log),
    "tie": lambda log: any(l == "|tie" for l in log),
    "notarget_fail": lambda log: any(l.startswith("|-fail|") for l in log),
    "switch": lambda log: any(l.startswith("|switch|") for l in log),
    "mega": lambda log: any(l.startswith("|-mega|") for l in log),
    "two_megas": lambda log: sum(l.startswith("|-mega|") for l in log) >= 2,
    # stage 2
    "status_set": lambda log: any(l.startswith("|-status|") for l in log),
    "thaw": lambda log: any(l.startswith("|-curestatus|") and "|frz|" in l for l in log),
    "wake": lambda log: any(l.startswith("|-curestatus|") and "|slp|" in l for l in log),
    "full_para": lambda log: any(l.startswith("|cant|") and l.endswith("|par") for l in log),
    "frozen_cant": lambda log: any(l.startswith("|cant|") and l.endswith("|frz") for l in log),
    "sleep_cant": lambda log: any(l.startswith("|cant|") and l.endswith("|slp") for l in log),
    "confusion_hit": lambda log: any("[from] confusion" in l for l in log),
    "flinch": lambda log: any(l.startswith("|cant|") and "|flinch" in l for l in log),
    "recharge": lambda log: any(l.startswith("|cant|") and "|recharge" in l for l in log),
    "fatigue": lambda log: any("confusion|[fatigue]" in l for l in log),
    "protect_block": lambda log: any(l.startswith("|-activate|") and "move: Protect" in l for l in log),
    "sub_broken": lambda log: any(l.startswith("|-end|") and "|Substitute" in l for l in log),
    "sub_hit": lambda log: any("move: Substitute|[damage]" in l for l in log),
    "destinybond": lambda log: any("move: Destiny Bond" in l and l.startswith("|-activate|") for l in log),
    "perish_faint": lambda log: any("|perish0" in l for l in log),
    "drag": lambda log: any(l.startswith("|drag|") for l in log),
    "pivot_switch": lambda log: any(l.startswith("|switch|") and "[from]" in l for l in log),
    "crash": lambda log: any("High Jump Kick" in l and "-damage" in l or "Axe Kick|" in l and "-damage" in l for l in log),
    "boost": lambda log: any(l.startswith("|-boost|") for l in log),
    "unboost": lambda log: any(l.startswith("|-unboost|") for l in log),
    "toxic_dmg": lambda log: any("[from] psn" in l and " tox" in l for l in log),
    "trap_dmg": lambda log: any("[partiallytrapped]" in l and l.startswith("|-damage|") for l in log),
    "leech": lambda log: any("[from] Leech Seed" in l for l in log),
    "encore": lambda log: any(l.startswith("|-start|") and "|Encore" in l for l in log),
    "disable": lambda log: any(l.startswith("|-start|") and "|Disable|" in l for l in log),
    "taunt": lambda log: any(l.startswith("|-start|") and "move: Taunt" in l for l in log),
    "attract_cant": lambda log: any(l.startswith("|cant|") and "|Attract" in l for l in log),
    "yawn_sleep": lambda log: any("move: Yawn|[silent]" in l for l in log),
    "transform": lambda log: any(l.startswith("|-transform|") for l in log),
    "endure": lambda log: any("move: Endure" in l and l.startswith("|-activate|") for l in log),
    "ohko": lambda log: any(l.startswith("|-ohko") for l in log),
}


def _has(log, prefix, *parts):
    return any(l.startswith(prefix) and all(p in l for p in parts) for l in log)


def _weather_set(log, name):
    return any(l.startswith("|-weather|" + name) and "[upkeep]" not in l for l in log)


# stage 3: field, side and slot conditions, from Showdown's log
COVERAGE.update({
    # weather
    "sun_start": lambda log: _weather_set(log, "SunnyDay"),
    "rain_start": lambda log: _weather_set(log, "RainDance"),
    "sand_start": lambda log: _weather_set(log, "Sandstorm"),
    "snow_start": lambda log: _weather_set(log, "Snowscape"),
    "weather_upkeep": lambda log: _has(log, "|-weather|", "[upkeep]"),
    "weather_end": lambda log: any(l == "|-weather|none" for l in log),
    "sand_damage": lambda log: _has(log, "|-damage|", "[from] Sandstorm"),
    # terrain
    "electric_terrain": lambda log: _has(log, "|-fieldstart|move: Electric Terrain"),
    "grassy_terrain": lambda log: _has(log, "|-fieldstart|move: Grassy Terrain"),
    "misty_terrain": lambda log: _has(log, "|-fieldstart|move: Misty Terrain"),
    "psychic_terrain": lambda log: _has(log, "|-fieldstart|move: Psychic Terrain"),
    "terrain_end": lambda log: _has(log, "|-fieldend|", "Terrain"),
    "grassy_heal": lambda log: _has(log, "|-heal|", "[from] Grassy Terrain"),
    "terrain_block": lambda log: _has(log, "|-activate|", "Terrain"),
    # rooms
    "trickroom": lambda log: _has(log, "|-fieldstart|move: Trick Room"),
    "trickroom_end": lambda log: _has(log, "|-fieldend|move: Trick Room"),
    "gravity": lambda log: _has(log, "|-fieldstart|move: Gravity"),
    "gravity_cant": lambda log: _has(log, "|cant|", "move: Gravity"),
    "magicroom": lambda log: _has(log, "|-fieldstart|move: Magic Room"),
    "wonderroom": lambda log: _has(log, "|-fieldstart|move: Wonder Room"),
    "fairylock": lambda log: _has(log, "|-fieldactivate|move: Fairy Lock"),
    "room_end": lambda log: _has(log, "|-fieldend|move: "),
    # side conditions
    "reflect": lambda log: _has(log, "|-sidestart|", "Reflect"),
    "lightscreen": lambda log: _has(log, "|-sidestart|", "Light Screen"),
    "auroraveil": lambda log: _has(log, "|-sidestart|", "Aurora Veil"),
    "tailwind": lambda log: _has(log, "|-sidestart|", "Tailwind"),
    "safeguard": lambda log: _has(log, "|-sidestart|", "Safeguard"),
    "safeguard_block": lambda log: _has(log, "|-activate|", "move: Safeguard"),
    "quickguard": lambda log: _has(log, "|-singleturn|", "Quick Guard"),
    "wideguard": lambda log: _has(log, "|-singleturn|", "Wide Guard"),
    "guard_block": lambda log: _has(log, "|-activate|", "Guard"),
    "screen_end": lambda log: (_has(log, "|-sideend|", "Reflect") or _has(log, "|-sideend|", "Light Screen")
                               or _has(log, "|-sideend|", "Aurora Veil")),
    "side_end": lambda log: _has(log, "|-sideend|"),
    # hazards
    "stealthrock": lambda log: _has(log, "|-sidestart|", "Stealth Rock"),
    "spikes": lambda log: _has(log, "|-sidestart|", "Spikes"),
    "toxicspikes": lambda log: _has(log, "|-sidestart|", "Toxic Spikes"),
    "stickyweb": lambda log: _has(log, "|-sidestart|", "Sticky Web"),
    "sr_damage": lambda log: _has(log, "|-damage|", "[from] Stealth Rock"),
    "spikes_damage": lambda log: _has(log, "|-damage|", "[from] Spikes"),
    "tspikes_absorb": lambda log: _has(log, "|-sideend|", "Toxic Spikes", "[of]"),
    "web_drop": lambda log: _has(log, "|-activate|", "move: Sticky Web"),
    "hazard_removed": lambda log: _has(log, "|-sideend|", "[from] move:"),
    "courtchange": lambda log: _has(log, "|-swapsideconditions"),
    "both_switch_in": lambda log: sum(l.startswith("|switch|") for l in log) >= 2,
    # slot conditions
    "wish_heal": lambda log: _has(log, "|-heal|", "[from] move: Wish"),
    "healingwish_heal": lambda log: _has(log, "|-heal|", "[from] move: Healing Wish"),
    "futuresight_start": lambda log: _has(log, "|-start|", "move: Future Sight"),
    "futuresight_hit": lambda log: _has(log, "|-end|", "move: Future Sight"),
    "revival": lambda log: _has(log, "|-heal|", "[from] move: Revival Blessing"),
})


SCREENS = ("reflect", "lightscreen", "auroraveil")


def state_coverage(s0, log, evs):
    """Stage-3 rules that need the state before the step as well as the log."""
    out = []
    if not s0:
        return out
    sides = s0.get("sides", {})
    for l in log:
        if l.startswith("|-crit|p"):
            sd = l.split("|")[2][:2]
            if any(c in sides.get(sd, {}).get("conditions", {}) for c in SCREENS):
                out.append("crit_through_screen")
        if l.startswith("|-heal|") and "[from] move: Wish" in l and "[wisher]" in l:
            name = l.split("|")[2].split(": ", 1)[-1]
            if l.split("[wisher] ")[-1] != name:
                out.append("wish_to_other")
    if "trickroom" in s0.get("rooms", {}) and any(e.get("key") == "speedtie" and "effect" not in e for e in evs):
        out.append("trickroom_action_tie")
    if any("|Defog|" in l for l in log) and any(l.startswith("|-activate|") and "move: Protect" in l for l in log):
        out.append("defog_protected")
    w = s0.get("weather", {})
    if w.get("left") == 1 and any(l.startswith("|-weather|") and "[upkeep]" not in l and l != "|-weather|none" for l in log):
        out.append("weather_set_on_last_turn")
    if sum(l.startswith("|switch|") for l in log) >= 2 and any("[from] Stealth Rock" in l or "[from] Spikes" in l for l in log):
        out.append("both_switch_into_hazards")
    return out


def canon(x):
    return json.dumps(x, sort_keys=True, separators=(",", ":"))


class SimRun:
    """Showdown in record mode against the new resolver with Showdown's events forced."""

    def __init__(self, args):
        self.args = args
        with open(TEAMS, encoding="utf-8") as f:
            self.teams = json.load(f)
        with open(PLAIN_MOVES, encoding="utf-8") as f:
            self.plain = json.load(f)["moves"]
        with open(STAGE2_MOVES, encoding="utf-8") as f:
            self.stage2 = json.load(f)["moves"]
        with open(STAGE3_MOVES, encoding="utf-8") as f:
            s3 = json.load(f)
            self.stage3, self.added3, self.interact3 = s3["moves"], s3["added"], s3["interacting"]
        self.stage4 = None
        if args.stage4 or args.focus4:
            with open(STAGE4, encoding="utf-8") as f:
                self.stage4 = json.load(f)
        self.stage5 = None
        if args.stage5 or args.focus5:
            with open(STAGE5, encoding="utf-8") as f:
                self.stage5 = json.load(f)
        self.oracle = Proc("oracle", [NODE, ORACLE], args.timeout)
        self.engine = Proc("engine", ENGINE.split("|") + ["--engine", "sim"] + (["--rows"] if args.rows else []),
                           args.timeout)
        # --rows: the engine's battle-log rows against Showdown's lines, on identical steps
        self.rows = rowcmp.RowStats() if args.rows else None
        self.out = open_text(args.out, "w")
        self.tot = {"battles": 0, "battles_identical": 0, "steps": 0, "options_identical": 0,
                    "states_identical": 0, "events_identical": 0, "tape_errors": 0, "engine_errors": 0,
                    "oracle_errors": 0, "capped": 0, "unimplemented_battles": 0, "events": 0,
                    "fail_kinds": {}, "unimplemented": {}, "event_keys": {}, "coverage": {},
                    "both_replace": 0, "move_uses": {}, "fail_moves": {}, "refusals": 0, "ability_steps": {},
                    "ability_activations": {}, "fail_abilities": {}, "item_steps": {}, "item_activations": {},
                    "megas": {}, "fail_items": {}}

    def write(self, rec):
        self.out.write(json.dumps(rec, separators=(",", ":")) + "\n")

    def battle(self, bi):
        T = self.tot
        rng = random.Random(f"{self.args.seed}:{bi}")
        ti = rng.sample(range(len(self.teams)), 2)
        teams = {"p1": self.teams[ti[0]], "p2": self.teams[ti[1]]}
        if self.args.stage5 or self.args.focus5:
            teams = {s: stage5_team(rng, self.stage5, self.teams, self.args.focus5) for s in SIDES}
        elif self.args.real:
            pass
        elif self.args.stage4 or self.args.focus4:
            teams = {s: stage4_team(rng, self.stage4, self.teams, self.args.focus4) for s in SIDES}
        elif self.args.focus3:
            teams = {s: focus_team(teams[s], rng, self.added3, self.interact3, self.args.megas) for s in SIDES}
        elif self.args.plain or self.args.stage2 or self.args.stage3:
            pool = self.args.pool.split(",") if self.args.pool else (
                self.stage3 if self.args.stage3 else self.stage2 if self.args.stage2 else self.plain)
            teams = {s: plain_team(teams[s], rng, pool, self.args.nmoves, self.args.megas) for s in SIDES}
        picks = {s: rng.sample(range(6), 3) for s in SIDES}
        seed = [rng.randrange(1, 65536) for _ in range(4)]
        bid = f"b{bi}"
        base = {"battle": bi, "run_seed": self.args.seed, "seed": seed, "team_index": ti, "picks": picks}
        r = self.oracle.ask({"op": "new", "id": bid, "seed": seed, "teams": teams, "picks": picks})
        if not r.get("ok"):
            T["oracle_errors"] += 1
            self.write(dict(base, kind="oracle_error", step=-1, error=r, teams=teams))
            return
        T["battles"] += 1
        e = self.engine.ask({"op": "new", "id": bid, "seed": seed, "teams": teams, "picks": picks,
                             "force": r.get("events", [])})
        ok = self.check(base, teams, -1, None, None, r, e)
        unimpl = set()
        if ok:
            unimpl.update(e.get("unimplemented", []))
            s0, o0 = r["state"], r["options"]
            refuse = 0.25 if (self.args.stage4 or self.args.focus4 or self.args.stage5 or self.args.focus5
                              or self.args.real) else 0.0
            for step in range(MAX_DECISIONS):
                if s0.get("ended") or all(o0.get(s, {}).get("kind") == "done" for s in SIDES):
                    break
                choices = choose(rng, o0, refuse)
                r = self.oracle.ask({"op": "step", "id": bid, "choices": choices})
                if not r.get("ok"):
                    T["oracle_errors"] += 1
                    self.write(dict(base, kind="oracle_error", step=step, error=r, s0=s0, choices=choices))
                    ok = False
                    break
                e = self.engine.ask({"op": "step", "id": bid, "choices": choices, "force": r.get("events", [])})
                if not self.check(base, teams, step, s0, choices, r, e):
                    ok = False
                    break
                unimpl.update(e.get("unimplemented", []))
                s0, o0 = r["state"], r["options"]
            else:
                T["capped"] += 1
        if unimpl:
            T["unimplemented_battles"] += 1
            for u in unimpl:
                T["unimplemented"][u] = T["unimplemented"].get(u, 0) + 1
        if ok:
            T["battles_identical"] += 1
        self.oracle.ask({"op": "release", "id": bid})
        self.engine.ask({"op": "release", "id": bid})

    def check(self, base, teams, step, s0, choices, r, e):
        """Compares one step; writes and returns False on the first difference."""
        T = self.tot
        T["steps"] += 1
        sid = f"{self.args.seed}:{base['battle']}:{step}"
        evs = r.get("events", [])
        T["events"] += len(evs)
        log = r.get("log", [])
        for k, f in COVERAGE.items():
            if f(log):
                T["coverage"][k] = T["coverage"].get(k, 0) + 1
        for k in state_coverage(s0, log, evs):
            T["coverage"][k] = T["coverage"].get(k, 0) + 1
        if r.get("ok") and all(r["options"].get(x, {}).get("kind") == "replace" for x in SIDES):
            T["both_replace"] += 1
        for ev in evs:
            T["event_keys"][ev["key"]] = T["event_keys"].get(ev["key"], 0) + 1
        for line in log:
            parts = line.split("|")
            if len(parts) > 3 and parts[1] == "move":
                mid = "".join(c for c in parts[3].lower() if c.isalnum())
                T["move_uses"][mid] = T["move_uses"].get(mid, 0) + 1
            # an ability shown in the log: |-ability|, [from] ability: X, |-activate|...|ability: X
            for m in re.finditer(r"ability: ([A-Za-z' -]+)", line):
                aid = "".join(c for c in m.group(1).lower() if c.isalnum())
                T["ability_activations"][aid] = T["ability_activations"].get(aid, 0) + 1
            if len(parts) > 3 and parts[1] == "-ability":
                aid = "".join(c for c in parts[3].lower() if c.isalnum())
                T["ability_activations"][aid] = T["ability_activations"].get(aid, 0) + 1
            # an item shown in the log: |-item|, |-enditem|, [from] item: X, |-activate|...|item: X
            seen = set()
            for m in re.finditer(r"item: ([A-Za-z' -]+)", line):
                seen.add("".join(c for c in m.group(1).lower() if c.isalnum()))
            if len(parts) > 3 and parts[1] in ("-item", "-enditem"):
                seen.add("".join(c for c in parts[3].lower() if c.isalnum()))
            for iid in seen:
                T["item_activations"][iid] = T["item_activations"].get(iid, 0) + 1
            if len(parts) > 4 and parts[1] == "-mega":
                # by stone: Charizard, Raichu, Absol, Garchomp and Lucario have two
                sid = "".join(c for c in parts[4].lower() if c.isalnum())
                T["megas"][sid] = T["megas"].get(sid, 0) + 1
        if s0:
            for sd in SIDES:
                a = s0["sides"][sd].get("active")
                if a is not None:
                    ab = s0["sides"][sd]["pokemon"][a].get("ability", "")
                    T["ability_steps"][ab] = T["ability_steps"].get(ab, 0) + 1
                    it = s0["sides"][sd]["pokemon"][a].get("item", "")
                    if it:
                        T["item_steps"][it] = T["item_steps"].get(it, 0) + 1
        rec = dict(base, kind="fail", step=step, step_id=sid, s0=s0, choices=choices, events=evs,
                   log=r.get("log", []), unimplemented=e.get("unimplemented", []))
        if step < 0:
            rec["teams"] = teams

        def fail(kind, **kw):
            T["fail_kinds"][kind] = T["fail_kinds"].get(kind, 0) + 1
            for m in moves_used(r.get("log", [])):
                T["fail_moves"][m] = T["fail_moves"].get(m, 0) + 1
            if s0:
                for sd in SIDES:
                    a = s0["sides"][sd].get("active")
                    if a is not None:
                        ab = s0["sides"][sd]["pokemon"][a].get("ability", "")
                        T["fail_abilities"][ab] = T["fail_abilities"].get(ab, 0) + 1
                        it = s0["sides"][sd]["pokemon"][a].get("item", "")
                        if it:
                            T["fail_items"][it] = T["fail_items"].get(it, 0) + 1
            self.write(dict(rec, fail=kind, **kw))
            return False

        if not e.get("ok"):
            if e.get("error") == "tape":
                T["tape_errors"] += 1
                return fail("tape", detail=e.get("detail"), engine_events=e.get("events"))
            T["engine_errors"] += 1
            return fail("engine_error", error=e.get("error"))
        if r.get("refused") or e.get("refused"):
            T["refusals"] += 1
            if canon(r.get("refused")) != canon(e.get("refused")):
                return fail("refused", oracle_refused=r.get("refused"), engine_refused=e.get("refused"))
        of = compare_options(r["options"], e.get("options", {}))
        sd = diff(r["state"], e.get("state", {}))
        ev_ok = canon(evs) == canon(e.get("events", []))
        T["options_identical"] += not of
        T["states_identical"] += not sd
        T["events_identical"] += ev_ok
        if of or sd or not ev_ok:
            kind = "state" if sd else "options" if of else "events"
            return fail(kind, options_diff=of, state_diff=sd[:20], engine_events=e.get("events"),
                        s1=r["state"], engine_state=e.get("state"))
        if self.args.trace:
            self.write(dict(rec, kind="trace"))
        if self.rows is not None:
            if "rows" not in e:
                return fail("rows_missing")
            self.rows.compare(log, e["rows"], r["state"], sid, teams=teams, picks=base["picks"])
        return True

    def run(self):
        t0 = time.time()
        for bi in range(self.args.start, self.args.start + self.args.battles):
            self.battle(bi)
            if self.args.progress and (bi + 1) % self.args.progress == 0:
                print(f"{bi + 1} battles, {self.tot['steps']} steps, {self.tot['battles_identical']} identical, "
                      f"{time.time() - t0:.0f}s", file=sys.stderr, flush=True)
        self.tot["seconds"] = round(time.time() - t0, 1)
        self.tot["restarts"] = {"oracle": self.oracle.restarts, "engine": self.engine.restarts}
        self.tot["args"] = vars(self.args)
        if self.rows is not None:
            self.tot["rows"] = self.rows.summary()
            for k, ex in sorted(self.rows.examples.items(), key=lambda kv: -kv[1]["count"]):
                self.write({"kind": "rows_examples", "key": k, "count": ex["count"], "steps": ex["steps"]})
        cov = self.engine.ask({"op": "coverage", "id": ""})
        if cov.get("ok"):
            self.tot["ability_triggers"] = cov["abilities"]
            if "items" in cov:
                self.tot["item_triggers"] = cov["items"]
        self.write({"kind": "summary", **self.tot})
        self.out.close()
        self.oracle.close()
        self.engine.close()
        return self.tot


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--battles", type=int, default=10)
    ap.add_argument("--start", type=int, default=0, help="first battle index (battle i uses seed:i)")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default=os.path.join(REPO, "tools", "rig", "results", "run.jsonl.gz"),
                    help="output JSONL, gzip-compressed when it ends in .gz")
    ap.add_argument("--progress", type=int, default=0, help="print progress every N battles")
    ap.add_argument("--timeout", type=float, default=TIMEOUT, help="seconds to wait for each reply")
    ap.add_argument("--sim", action="store_true", help="no effect: the resolver is the only engine")
    ap.add_argument("--plain", action="store_true", help="stage-1 battles: No Ability, no item, plain moves")
    ap.add_argument("--stage2", action="store_true", help="stage-2 battles: No Ability, no item, stage-2 pool moves")
    ap.add_argument("--stage3", action="store_true", help="stage-3 battles: No Ability, no item, stage-3 pool moves")
    ap.add_argument("--focus3", action="store_true",
                    help="stage-3 battles weighted to the field: two added and two interacting moves each")
    ap.add_argument("--stage4", action="store_true", help="stage-4 battles: legal abilities on, stage-4 pool, no items")
    ap.add_argument("--focus4", action="store_true",
                    help="stage-4 battles weighted to abilities: any implemented ability, two interacting moves")
    ap.add_argument("--stage5", action="store_true",
                    help="stage-5 battles: legal abilities, random legal items and Megas, the full move pool")
    ap.add_argument("--focus5", action="store_true",
                    help="stage-5 battles weighted to rare item, Mega and ability interactions")
    ap.add_argument("--real", action="store_true", help="the real teams of teams.json exactly as written")
    ap.add_argument("--trace", action="store_true", help="also write every identical step (kind trace)")
    ap.add_argument("--rows", action="store_true",
                    help="the engine records battle-log rows; compare them with Showdown's lines (rows.py)")
    ap.add_argument("--nmoves", type=int, default=4, help="--plain: moves per Pokemon")
    ap.add_argument("--megas", action="store_true", help="--plain: keep Mega stones")
    ap.add_argument("--pool", default="", help="--plain: comma-separated move ids to draw from instead")
    args = ap.parse_args()
    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    tot = SimRun(args).run()
    keys = ["battles", "battles_identical", "steps", "options_identical", "states_identical",
            "events_identical", "tape_errors", "engine_errors", "oracle_errors", "unimplemented_battles",
            "fail_kinds", "capped", "refusals", "seconds"]
    print(json.dumps({k: tot[k] for k in keys}))
    if args.rows:
        rs = tot["rows"]
        print(json.dumps({"rows_steps": rs["steps"], "rows_steps_equal": rs["steps_equal"], "kinds": rs["kinds"]}))


if __name__ == "__main__":
    main()
