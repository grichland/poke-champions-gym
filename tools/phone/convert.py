"""Recorded phone battles (pcsa/output/runs/*/battle-*) -> LIVE.md 0.2 messages, one JSON object per line.

A development test tool, not the controller. One table entry per row template; every row gives one event
(a few multi-stat rows give one per stat, noted in the summary), or `unread`. Names become Showdown ids from
CG's data (data/*.json and engine/src/sim/data/generated.rs).

Lines written: battle_start, {"picks"}, events messages, decision_request + {"seq","order","notes"}
answers, battle_end. `src` on each event is "turn.index" of the row (not a contract field).

Usage: convert.py OUT_DIR (BATTLE_DIR ... | RUNS_DIR)
"""

import glob
import json
import os
import re
import sys
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
V = "0.3"


def toid(s):
    return re.sub(r"[^a-z0-9]", "", (s or "").lower())


# ------------------------------------------------------------------ CG data

def load_data():
    with open(os.path.join(ROOT, "data", "species.json"), encoding="utf-8") as f:
        species = json.load(f)
    with open(os.path.join(ROOT, "data", "moves.json"), encoding="utf-8") as f:
        moves = json.load(f)
    with open(os.path.join(ROOT, "data", "natures.json"), encoding="utf-8") as f:
        natures = json.load(f)
    gen = open(os.path.join(ROOT, "engine", "src", "sim", "data", "generated.rs"), encoding="utf-8").read()
    items = {i: n for i, n in re.findall(r'ItemData \{ id: "(\w+)", name: "([^"]*)"', gen)}
    abilities = {i: n for i, n in re.findall(r'AbilityData \{ id: "(\w+)", name: "([^"]*)"', gen)}
    return species, moves, natures, items, abilities


SPECIES, MOVES, NATURES, ITEMS, ABILITIES = load_data()
MOVE_BY = {toid(v["name"]): k for k, v in MOVES.items()}
ITEM_BY = {toid(n): i for i, n in ITEMS.items()}
ABIL_BY = {toid(n): i for i, n in ABILITIES.items()}
# display name of the base species -> non-Mega forms with that base
FORMS = {}
for sid, d in SPECIES.items():
    if not d.get("isMega"):
        FORMS.setdefault(d["baseSpecies"], []).append(sid)
# preview classifier names that are not plain Showdown names
REGION = {"alolan": "alola", "galarian": "galar", "hisuian": "hisui", "paldean": "paldea"}


def fuzzy(name, table):
    """Exact id, else the common OCR slip v for y."""
    k = toid(name)
    if k in table:
        return table[k]
    k2 = toid(name.replace("v", "y"))
    return table.get(k2)


def item_id(n):
    return fuzzy(n, ITEM_BY)


def ability_id(n):
    return fuzzy(n, ABIL_BY)


def move_id(n):
    return fuzzy(n, MOVE_BY)


def preview_id(name):
    """Species id of a preview classifier name ("Paldean Tauros Blaze Breed", "Vivillon High Plains Pattern")."""
    if not name:
        return None
    k = toid(name)
    if k in SPECIES:
        return k
    words = name.split()
    if words and words[0].lower() in REGION:
        rest = words[1:]
        tail = [w for w in rest[1:] if w.lower() not in ("breed", "form")]
        k = toid(rest[0]) + REGION[words[0].lower()] + toid("".join(tail))
        if k in SPECIES:
            return k
    k = toid(re.sub(r"\b(Pattern|Form|Forme|Breed)\b", "", name))
    return k if k in SPECIES else None


# ------------------------------------------------------------------ own team

STAT_KEYS = ["hp", "atk", "def", "spa", "spd", "spe"]
EV_KEYS = {"HP": "hp", "Atk": "atk", "Def": "def", "SpA": "spa", "SpD": "spd", "Spe": "spe"}


def parse_export(text):
    sets = []
    for block in text.strip().split("\n\n"):
        lines = [l.strip() for l in block.strip().splitlines() if l.strip()]
        if not lines:
            continue
        head = lines[0]
        name, _, item = head.partition(" @ ")
        s = {"name": name.strip(), "item": item.strip() or None, "moves": [], "points": None, "nature": None}
        for l in lines[1:]:
            if l.startswith("Ability:"):
                s["ability"] = l.split(":", 1)[1].strip()
            elif l.startswith("EVs:"):
                pts = {k: 0 for k in STAT_KEYS}
                for part in l[4:].split("/"):
                    n, k = part.split()
                    pts[EV_KEYS[k]] = int(n)
                s["points"] = pts
            elif l.endswith("Nature"):
                s["nature"] = l.split()[0]
            elif l.startswith("- "):
                s["moves"].append(l[2:])
        sets.append(s)
    return sets


def calc_stats(sid, points, nature):
    """Champions level-50 stats: IV 31, one stat point per point."""
    base = SPECIES[sid]["baseStats"]
    nat = NATURES.get(toid(nature), {}) if nature else {}
    out = {}
    for k in STAT_KEYS:
        core = (2 * base[k] + 31) // 2 + (points or {}).get(k, 0)
        if k == "hp":
            out[k] = core + 60
        else:
            v = core + 5
            mult = 1.1 if nat.get("plus") == k else 0.9 if nat.get("minus") == k else 1.0
            out[k] = int(v * mult)
    return out


# ------------------------------------------------------------------ row table

OPP = r"(?P<o>(?:The|Ihe|Tne|I ne|the) o[pn][pn]osin[ga]\s+)?"
M = r"(?P<m>.+?)"
END = r"[.!,]*\s*$"
STAT_RE = re.compile(r"Sp\. ?Atk|Sp\. ?Def|Attack|Defense|Speed|accuracy|evasiveness", re.I)
STAT_ID = {"spatk": "spa", "spdef": "spd", "attack": "atk", "defense": "def", "speed": "spe", "accuracy": "accuracy", "evasiveness": "evasion"}
BY = {"rose": 1, "rose sharply": 2, "rose drastically": 3, "fell": -1, "fel": -1, "harshly fell": -2, "harshly fel": -2, "severely fell": -3, "severely fel": -3}
TYPES = {"normal", "fire", "water", "electric", "grass", "ice", "fighting", "poison", "ground", "flying", "psychic", "bug", "rock", "ghost", "dragon", "dark", "steel", "fairy"}

# (template id, inventory templates covered, pattern, handler name, fixed args)
TABLE = [
    ("T01", "PL-001 002 010 011", r"^(?P<cat>HP Lost|HP Restored): " + M + r"'s HP:? (?P<a>\d+)(?P<ap>%?) ?→ ?(?P<b>\d+)(?P<bp>%?)$", "hp", None),
    ("T02", "PL-003 004 017 026 057 ...", r"^(?P<cat>Move[^:]*): " + OPP + M + r" used (?P<mv>.+?)" + END, "move", None),
    ("T03", "PL-006 146 153", r"^Held Item: " + M + r"'s (?P<it>.+?)$", "held_item", None),
    ("T04", "PL-197", r"^Ability Changed: " + M + r": (?P<a>.+?) ?→ ?(?P<b>.+?)$", "ability_change", None),
    ("T05", "PL-009 234 246 ...", r"^Ability: " + M + r"'s (?P<a>.+?)$", "ability_row", None),
    ("T06", "PL-013 014", r"^Fainted: " + OPP + M + r" fainted" + END, "simple", ("faint",)),
    ("T07", "PL-019 020 240", r"^Mega Evolution: " + OPP + M + r" has Mega Evolved into (?P<f>.+?)" + END, "mega", None),
    ("T08", "PL-005", r"^Go! " + M + END, "enter_own", None),
    ("T09", "PL-007 023 250", r"^(?P<tr>.*?) ?sent out " + M + END, "enter_opp", None),
    ("T10", "PL-015", r"^" + M + r", come back" + END, "withdraw", ("own", False)),
    ("T11", "PL-016 239", r"^(?P<tr>.*?) ?withdrew " + M + END, "withdraw", ("opp", False)),
    ("T12", "PL-035 064 220", r"^" + OPP + M + r" went back to.*$", "withdraw_pivot", None),
    ("T13", "PL-086", r"^" + OPP + M + r" was dragged out" + END, "dragged", None),
    ("T14", "PL-008", r"^It's super effective" + END, "result", ("super",)),
    ("T15", "PL-012 024", r"^It's not very effective" + END, "result", ("resisted",)),
    ("T16", "PL-046 119", r"^It's mostly ineffective" + END, "result", ("mostly_ineffective",)),
    ("T17", "PL-034", r"^It's extremely effective" + END, "result", ("extreme",)),
    ("T18", "PL-283", r"^It's a one-hit KO" + END, "result", ("ohko",)),
    ("T19", "PL-076 088 091 099 206 212 215", r"^It doesn't affect " + OPP + M + END, "immune", None),
    ("T20", "PL-037 208", r"^The Pok[ée]mon was hit (?P<n>\d+) times?" + END, "hits", None),
    ("T21", "PL-032", r"^" + OPP + M + r" protected itself" + END, "simple", ("protect",)),
    ("T22", "PL-052 070", r"^" + OPP + M + r" avoided the attack" + END, "simple", ("miss",)),
    ("T23", "PL-042", r"^But it failed" + END, "fail_bare", None),
    ("T24", "PL-136 165 226 256", r"^" + OPP + M + r" (?:already has a substitute|is already \w+)" + END, "simple", ("fail",)),
    ("T25", "PL-213 231", r"^" + OPP + M + r"'s HP is full" + END, "simple", ("fail",)),
    ("T26", "PL-069 121", r"^" + OPP + M + r"'s (?P<s>.+?) won't go any (?P<d>higher|lower)" + END, "boost_limit", None),
    ("T27", "PL-018 025 027 028 038 044 073 097 173 203 219 222 227", r"^" + OPP + M + r"'s (?P<s>.+?) (?P<w>rose drastically|rose sharply|rose|harshly fell?|severely fell?|fell?)" + END, "boost", None),
    ("T28", "PL-059 117", r"^" + OPP + M + r" was burned" + END, "status", ("brn",)),
    ("T29", "PL-060 150", r"^" + OPP + M + r" fell asleep" + END, "status", ("slp",)),
    ("T30", "PL-101 108", r"^" + OPP + M + r" was poisoned" + END, "status", ("psn",)),
    ("T31", "PL-120", r"^" + OPP + M + r" was badly poisoned" + END, "status", ("tox",)),
    ("T32", "PL-229 235", r"^" + OPP + M + r" was frozen solid" + END, "status", ("frz",)),
    ("T33", "PL-107 189", r"^" + OPP + M + r" is paralyzed, so it may be unable to move" + END, "status", ("par",)),
    ("T34", "PL-049 133", r"^" + OPP + M + r" is fast asleep" + END, "cant", ("slp",)),
    ("T35", "PL-082 224", r"^" + OPP + M + r" flinched and couldn't move" + END, "cant", ("flinch",)),
    ("T36", "PL-172 297", r"^" + OPP + M + r" couldn't move because it's paralyzed" + END, "cant", ("par",)),
    ("T37", "PL-268", r"^" + OPP + M + r" must recharge" + END, "cant", ("mustrecharge",)),
    ("T38", "PL-207", r"^" + OPP + M + r" can't use (?P<mv>.+?) after the taunt" + END, "cant", ("taunt",)),
    ("T39", "PL-098 162", r"^" + OPP + M + r" woke up" + END, "cure", ("slp",)),
    ("T40", "PL-252 287", r"^" + OPP + M + r" thawed out" + END, "cure", ("frz",)),
    ("T41", "PL-216 258 265 296", r"^" + OPP + M + r"'s (?P<it>.+?) (?P<w>cured its poisoning|cured its paralysis|cured its burn|woke it up|defrosted it)" + END, "cure_item", None),
    ("T42", "PL-058 103", r"^" + OPP + M + r" grew drowsy" + END, "vol", ("yawn",)),
    ("T43", "PL-061", r"^" + OPP + M + r" must do an encore" + END, "vol", ("encore",)),
    ("T44", "PL-077 237", r"^" + OPP + M + r" put in a substitute" + END, "vol", ("substitute",)),
    ("T45", "PL-291", r"^" + OPP + M + r" shed its tail to create a decoy" + END, "vol", ("substitute",)),
    ("T46", "PL-187", r"^" + OPP + M + r" fell for the taunt" + END, "vol", ("taunt",)),
    ("T47", "PL-156", r"^" + OPP + M + r" was prevented from healing" + END, "vol", ("healblock",)),
    ("T48", "PL-154 171 193", r"^" + OPP + M + r" became confused(?: due to fatigue)?" + END, "vol", ("confusion",)),
    ("T49", "PL-138", r"^" + OPP + M + r" was seeded" + END, "vol", ("leechseed",)),
    ("T50", "PL-096", r"^" + OPP + M + r" began charging power" + END, "vol", ("charge",)),
    ("T51", "PL-299", r"^" + OPP + M + r" fell straight down" + END, "vol", ("smackdown",)),
    ("T52", "PL-228", r"^" + OPP + M + r" became trapped in the fiery vortex" + END, "vol", ("firespin",)),
    ("T53", "PL-112", r"^" + OPP + M + r" has been afflicted with an infestation by .*$", "vol", ("infestation",)),
    ("T54", "PL-149", r"^" + OPP + M + r"'s (?P<mv>.+?) was disabled" + END, "vol_move", ("disable",)),
    ("T55", "PL-161 204 305", r"^" + OPP + M + r"'s perish count fell to (?P<n>\d+|O)" + END, "perish", None),
    ("T56", "PL-158", r"^" + OPP + r"(?P<m0>.+?) cut its own HP and put a curse on (?P<m>.+?)" + END, "curse_on", None),
    ("T57", "PL-087 264", r"^" + OPP + M + r"'s substitute faded" + END, "vol_end", ("substitute",)),
    ("T58", "PL-094", r"^" + OPP + M + r" ended its encore" + END, "vol_end", ("encore",)),
    ("T59", "PL-195", r"^" + OPP + M + r" shook off the taunt" + END, "vol_end", ("taunt",)),
    ("T60", "PL-194", r"^" + OPP + M + r" is no longer prevented from healing" + END, "vol_end", ("healblock",)),
    ("T61", "PL-205", r"^" + OPP + M + r" snapped out of its confusion" + END, "vol_end", ("confusion",)),
    ("T62", "PL-211", r"^" + OPP + M + r" was freed from (?P<mv>.+?)" + END, "vol_end_move", None),
    ("T63", "PL-191", r"^" + OPP + M + r"'s illusion wore off" + END, "vol_end", ("illusion",)),
    ("T64", "PL-068 263", r"^The substitute took damage for " + OPP + M + END, "vol_hit", ("substitute",)),
    ("T65", "PL-147 233", r"^" + OPP + M + r" is confused" + END, "vol_hit", ("confusion",)),
    ("T66", "PL-186", r"^It hurt itself in its confusion" + END, "vol_hit_bare", ("confusion",)),
    ("T67", "PL-177", r"^" + OPP + M + r" endured the hit" + END, "vol_hit", ("endure",)),
    ("T68", "PL-168", r"^" + OPP + M + r" took its attacker down with it" + END, "vol_hit", ("destinybond",)),
    ("T69", "PL-029 067", r"^" + OPP + M + r" was hurt by its burn" + END, "cause", ("brn",)),
    ("T70", "PL-039 080", r"^" + OPP + M + r" was hurt by its poisoning" + END, "cause", ("psn",)),
    ("T71", "PL-036 043", r"^" + OPP + M + r" is buffeted by the sandstorm" + END, "cause", ("sandstorm",)),
    ("T72", "PL-033 144", r"^Pointed stones dug into " + OPP + M + END, "cause", ("stealthrock",)),
    ("T73", "PL-129", r"^" + OPP + M + r" was hurt by the spikes" + END, "cause", ("spikes",)),
    ("T74", "PL-055 111 132 145 254", r"^" + OPP + M + r" was damaged by the recoil+" + END, "cause", ("recoil",)),
    ("T75", "PL-041 102", r"^" + OPP + M + r" lost some of its HP" + END, "cause", ("lostsomehp",)),
    ("T76", "PL-030 050", r"^" + OPP + M + r" had its energy drained" + END, "cause", ("drain",)),
    ("T77", "PL-045 048", r"^" + OPP + M + r" had its HP restored" + END, "cause", ("healed",)),
    ("T78", "PL-141", r"^" + OPP + M + r" kept going and crashed" + END, "cause", ("crash",)),
    ("T79", "PL-176", r"^" + OPP + M + r" was hurt" + END, "cause", ("hurt",)),
    ("T80", "PL-131", r"^" + OPP + M + r" is afflicted by the curse" + END, "cause", ("curse",)),
    ("T81", "PL-074", r"^" + OPP + M + r"'s health is sapped by (?P<mv>.+?)" + END, "cause_move", None),
    ("T82", "PL-071", r"^" + OPP + M + r" is hurt by (?P<mv>.+?)" + END, "cause_move", None),
    ("T83", "PL-135", r"^" + OPP + M + r"'s wish came true" + END, "cause_of", ("wish",)),
    ("T84", "PL-242", r"^The healing wish came true for " + OPP + M + END, "cause", ("healingwish",)),
    ("T85", "PL-140", r"^The battlers shared their pain" + END, "cause_bare", ("painsplit",)),
    ("T86", "PL-031 093", r"^" + OPP + M + r" floats in the air with its (?P<it>.+?)" + END, "item", ("activated",)),
    ("T87", "PL-040 100", r"^" + OPP + M + r"'s (?P<it>.+?) popped" + END, "item", ("removed",)),
    ("T88", "PL-051 056", r"^" + OPP + M + r" hung on using its (?P<it>.+?)" + END, "item", ("used",)),
    ("T89", "PL-181", r"^" + OPP + M + r" is switched out with the (?P<it>.+?)" + END, "item", ("used",)),
    ("T90", "PL-142", r"^" + OPP + M + r"'s (?P<it>.+?) lessened the damage it took" + END, "item", ("used",)),
    ("T91", "PL-198", r"^" + OPP + M + r" can act faster than normal, thanks to its (?P<it>.+?)" + END, "item", ("activated",)),
    ("T92", "PL-137", r"^" + OPP + M + r" is about to be attacked by its (?P<it>.+?)" + END, "item", ("found",)),
    ("T93", "PL-236", r"^" + OPP + M + r" harvested an? (?P<it>.+?)" + END, "item", ("received",)),
    ("T94", "PL-123", r"^" + OPP + M + r" held up its (?P<it>.+?) against (?P<m2>.+?)" + END, "item_of", ("used",)),
    ("T95", "PL-214", r"^" + OPP + r"(?P<m2>.+?) knocked off (?P<m>.+?)'s (?P<it>.+?)" + END, "item_knock", None),
    ("T96", "PL-179", r"^" + OPP + r"(?P<m2>.+?) frisked (?P<m>.+?) and found its (?P<it>.+?)" + END, "item_frisk", None),
    ("T97", "PL-188", r"^" + OPP + M + r" returned its stats to normal using its (?P<it>.+?)" + END, "white_herb", None),
    ("T98", "PL-021 022 178 210 269 274 280 281 301", r"^" + OPP + M + r"'s (?P<it>.+?) is reacting to.*Omni Ring" + END, "mega_stone", None),
    ("T99", "PL-081", r"^" + OPP + M + r" is exerting its pressure" + END, "ability", ("pressure",)),
    ("T100", "PL-118", r"^" + OPP + M + r" breaks the mold" + END, "ability", ("moldbreaker",)),
    ("T101", "PL-143 167", r"^" + OPP + M + r" gained strength from the fallen" + END, "ability", ("supremeoverlord",)),
    ("T102", "PL-202", r"^" + OPP + M + r" is radiating a fairy aura" + END, "ability", ("fairyaura",)),
    ("T103", "PL-180", r"^(?P<a>.+?) made " + OPP + M + r" move faster" + END, "ability_named", None),
    ("T104", "PL-163", r"^It traced " + OPP + M + r"'s (?P<a>.+?)" + END, "ability_named", None),
    ("T105", "PL-054 066 072 ... 262", r"^" + OPP + M + r" transformed into the (?P<t>\w+) type" + END, "types", None),
    ("T106", "PL-174", r"^" + OPP + M + r" transformed into (?P<x>.+?)" + END, "transform", None),
    ("T107", "PL-085 126", r"^" + OPP + M + r"'s disguise was busted" + END, "form", ("mimikyubusted",)),
    ("T108", "PL-276", r"^" + OPP + M + r" underwent a heroic transformation" + END, "form", ("palafinhero",)),
    ("T109", "PL-157", r"^" + OPP + M + r" used up all its electricity" + END, "once", ("doubleshock",)),
    ("T110", "PL-279", r"^" + OPP + M + r" burned itself out" + END, "once", ("burnup",)),
    ("T111", "PL-159 284", r"^" + OPP + M + r" is hoping to take its attacker down with it" + END, "once", ("destinybond",)),
    ("T112", "PL-225", r"^" + OPP + M + r" started heating up its beak" + END, "once", ("beakblast",)),
    ("T113", "PL-251", r"^" + OPP + M + r" is preparing to tell a chillingly bad joke" + END, "once", ("chillyreception",)),
    ("T114", "PL-217", r"^" + OPP + M + r" has no moves left that it can use" + END, "once", ("struggle",)),
    ("T115", "PL-270", r"^" + OPP + M + r" braced itself" + END, "once", ("endure",)),
    ("T116", "PL-232 306", r"^All Pok[ée]mon that heard the song will ?faint in three turns" + END, "once_bare", ("perishsong",)),
    ("T117", "PL-122", r"^" + OPP + M + r" was revived and is ready to fight again" + END, "simple", ("revive",)),
    ("T118", "PL-084", r"^" + OPP + M + r" absorbed light" + END, "prepare", None),
    ("T119", "PL-151", r"^" + OPP + M + r" absorbed electricity" + END, "prepare", None),
    ("T120", "PL-295", r"^" + OPP + M + r" vanished instantly" + END, "prepare", None),
    ("T121", "PL-190", r"^" + OPP + M + r" slept and restored its HP" + END, "status", ("slp",)),
    ("T122", "PL-169 300", r"^" + OPP + M + r" is protected by the Psychic Terrain" + END, "blocked", ("psychicterrain", "priority")),
    ("T123", "PL-257", r"^" + OPP + M + r"'s stats were not lowered" + END, "blocked", ("statdrop", None)),
    ("T124", "PL-249", r"^(?P<side>Your|The opposing) side is too nervous to eat Berries" + END, "blocked_side", None),
    ("T125", "PL-047", r"^A sandstorm kicked up" + END, "weather", ("sandstorm", "start")),
    ("T126", "PL-062", r"^The sandstorm subsided" + END, "weather", ("sandstorm", "end")),
    ("T127", "PL-089", r"^It started to rain" + END, "weather", ("raindance", "start")),
    ("T128", "PL-128", r"^The rain stopped" + END, "weather", ("raindance", "end")),
    ("T129", "PL-065", r"^The sunlight turned harsh" + END, "weather", ("sunnyday", "start")),
    ("T130", "PL-090", r"^The harsh sunlight faded" + END, "weather", ("sunnyday", "end")),
    ("T131", "PL-083", r"^It started to snow" + END, "weather", ("snowscape", "start")),
    ("T132", "PL-106", r"^The snow stopped" + END, "weather", ("snowscape", "end")),
    ("T133", "PL-075", r"^Grass grew to cover the battlefield" + END, "field", ("grassyterrain", "start")),
    ("T134", "PL-105", r"^The grass disappeared from the battlefield" + END, "field", ("grassyterrain", "end")),
    ("T135", "PL-078", r"^The battlefield got weird" + END, "field", ("psychicterrain", "start")),
    ("T136", "PL-116", r"^The weirdness disappeared from the battlefield" + END, "field", ("psychicterrain", "end")),
    ("T137", "PL-148", r"^" + OPP + M + r" twisted the dimensions" + END, "field", ("trickroom", "start")),
    ("T138", "PL-192", r"^The twisted dimensions returned to normal" + END, "field", ("trickroom", "end")),
    ("T139", "PL-053 183", r"^Pointed stones float in the air on (?P<side>your|the opposing) side" + END, "side", ("stealthrock", "start")),
    ("T140", "PL-125", r"^Spikes were scattered on the ground all around (?P<side>your|the opposing) side" + END, "side", ("spikes", "start")),
    ("T141", "PL-104", r"^Toxic spikes were scattered on the ground all around (?P<side>your|the opposing) side" + END, "side", ("toxicspikes", "start")),
    ("T142", "PL-248", r"^A sticky web has been laid out on the ground on (?P<side>your|the opposing) side" + END, "side", ("stickyweb", "start")),
    ("T143", "PL-110 114 115", r"^(?P<eff>Light Screen|Reflect|Aurora Veil) made (?P<side>your|the opposing) side stronger against .*$", "side_named", ("start",)),
    ("T144", "PL-139 152 164", r"^(?P<side>Your|The opposing) side's (?P<eff>Light Screen|Reflect|Aurora Veil) wore off" + END, "side_named", ("end",)),
    ("T145", "PL-244", r"^A tailwind started blowing on (?P<side>your|the opposing) side" + END, "side", ("tailwind", "start")),
    ("T146", "PL-292", r"^(?P<side>Your|The opposing) side's tailwind petered out" + END, "side", ("tailwind", "end")),
    ("T147", "PL-201", r"^" + OPP + M + r" was caught in a sticky web" + END, "side_acted", ("stickyweb",)),
    ("T148", "PL-134", r"^All stat changes were eliminated" + END, "haze", None),
]
TABLE = [(tid, pl, re.compile(p), h, a) for tid, pl, p, h, a in TABLE]
# rows the controller cannot read as battle text
NOT_BATTLE = {"Until Battle Ends"}


def norm(text):
    t = text.replace("’", "'").replace("‘", "'").replace("->", "→").replace("—", "-").replace("−", "-")
    return re.sub(r"\s+", " ", t).strip()


def clean_name(n):
    n = n.strip()
    n = re.sub(r"^'s ", "", n)
    n = re.sub(r" the .*$", "", n)
    return n.strip()


# ------------------------------------------------------------------ one battle

class Battle:
    def __init__(self, bdir):
        self.dir = bdir
        self.rows = []
        with open(os.path.join(bdir, "log.jsonl"), encoding="utf-8") as f:
            for l in f:
                if l.strip():
                    self.rows.append(json.loads(l))
        self.meta = None
        p = os.path.join(bdir, "battle.json")
        if os.path.exists(p):
            with open(p, encoding="utf-8") as f:
                self.meta = json.load(f)
        self.decisions = []
        for p in sorted(glob.glob(os.path.join(bdir, "decisions", "*.json"))):
            with open(p, encoding="utf-8") as f:
                self.decisions.append(json.load(f))
        self.decisions.sort(key=lambda d: d["sequence"])
        self.coverage = {}
        p = os.path.join(bdir, "coverage.json")
        if os.path.exists(p):
            with open(p, encoding="utf-8") as f:
                self.coverage = json.load(f)
        self.stats = Counter()  # summary counters
        self.notes = []
        self.tag = self.rows[0]["battle_id"] if self.rows else os.path.basename(bdir)
        self.build_team()
        self.opp_cache = {}
        self.opp_sprites = self.collect_sprites()
        self.active = {"own": None, "opp": None}
        self.stone = {}

    # ---- team, picks, preview
    def build_team(self):
        export = next((d.get("own_export") for d in self.decisions if d.get("own_export")), None)
        sets = parse_export(export) if export else []
        seen_stats = {}
        for d in self.decisions:
            for p in (d.get("request") or {}).get("side", {}).get("pokemon", []):
                sid = toid(p["details"].split(",")[0])
                st = dict(p.get("stats") or {})
                cond = p.get("condition", "")
                m = re.match(r"\d+/(\d+)", cond)
                if m and st:
                    st["hp"] = int(m.group(1))
                    seen_stats.setdefault(sid, st)
        self.team = []
        for s in sets:
            sid = toid(s["name"])
            if sid not in SPECIES:
                self.notes.append("unknown own species " + s["name"])
            calc = calc_stats(sid, s["points"], s["nature"]) if sid in SPECIES else {k: 0 for k in STAT_KEYS}
            rec = seen_stats.get(sid)
            if rec:
                self.stats["own stats recorded"] += 1
                if any(rec.get(k) != calc[k] for k in STAT_KEYS):
                    self.stats["own stats formula differs"] += 1
                    self.notes.append("stats %s calc %s recorded %s" % (sid, calc, rec))
            else:
                self.stats["own stats computed"] += 1
            st = {k: (rec or calc)[k] for k in STAT_KEYS}
            out = {"species": sid, "ability": ability_id(s.get("ability", "")) or toid(s.get("ability", "")),
                   "moves": [move_id(m) or toid(m) for m in s["moves"]], "stats": st}
            if s["item"]:
                out["item"] = item_id(s["item"]) or toid(s["item"])
            if s["nature"]:
                out["nature"] = toid(s["nature"])
            if s["points"]:
                out["points"] = s["points"]
            self.team.append(out)
        self.own_ids = [s["species"] for s in self.team]
        # picks: lead first
        names = None
        if self.meta:
            names = self.meta.get("team_played_names") or self.meta.get("team_requested_names")
        if not names:
            for d in self.decisions:
                ps = (d.get("request") or {}).get("side", {}).get("pokemon", [])
                if ps:
                    act = [p for p in ps if p.get("active")]
                    names = [p["details"].split(",")[0] for p in act + [p for p in ps if not p.get("active")]]
                    self.stats["picks from request"] += 1
                    break
        self.picks = []
        for n in names or []:
            k = toid(n)
            if k in self.own_ids:
                self.picks.append(self.own_ids.index(k))
        self.brought = [self.own_ids[i] for i in self.picks]
        self.preview = [None] * 6
        if self.meta and self.meta.get("opponent_preview"):
            for p in self.meta["opponent_preview"]:
                if p.get("species") and p.get("slot") is not None and p["slot"] < 6:
                    self.preview[p["slot"]] = preview_id(p["species"][0][0])
                    if self.preview[p["slot"]] is None:
                        self.stats["preview name unmapped"] += 1
                        self.notes.append("preview " + p["species"][0][0])

    def collect_sprites(self):
        """Opposing display name -> form id seen on matched sprites of rows that name it."""
        out = {}
        for r in self.rows:
            s = r.get("sprite") or {}
            if s.get("status") != "matched" or not s.get("name"):
                continue
            t = norm(r["text"])
            m = re.match(r"^Move[^:]*: (?:The opposing )(.+?) used ", t) or re.match(r"^HP \w+: (.+?)'s HP:? \d+% ", t)
            if m:
                form = toid(re.sub(r"-Mega(-[XYZ])?$", "", s["name"]))
                out.setdefault(clean_name(m.group(1)), Counter())[form] += 1
        return {k: c.most_common(1)[0][0] for k, c in out.items()}

    # ---- names
    def own_id(self, name):
        k = toid(clean_name(name))
        k = re.sub(r"^mega", "", k) if k not in SPECIES else k
        pool = self.brought + [x for x in self.own_ids if x not in self.brought]
        for sid in pool:
            if k == sid or k == toid(SPECIES[sid]["name"]) or k == SPECIES[sid]["baseSpecies"]:
                return sid
        k2 = toid(clean_name(name).replace("v", "y"))
        for sid in pool:
            if k2 == sid:
                return sid
        return None

    def opp_id(self, name):
        n = clean_name(name)
        if n in self.opp_cache:
            return self.opp_cache[n]
        k = toid(n)
        cands = FORMS.get(k) or FORMS.get(toid(n.replace("v", "y"))) or []
        sid = None
        how = None
        spr = self.opp_sprites.get(n)
        if spr in cands:
            sid, how = spr, "sprite"
        if sid is None:
            for p in self.preview:
                if p in cands:
                    sid, how = p, "preview"
                    break
        if sid is None and k in SPECIES:
            sid, how = k, "name"
        if sid is None and cands:
            sid, how = cands[0], "first form"
        if sid is not None:
            self.stats["opp name by " + how] += 1
            self.opp_cache[n] = sid
        return sid

    def mon(self, side, name):
        sid = self.own_id(name) if side == "own" else self.opp_id(name)
        return None if sid is None else {"side": side, "mon": sid}

    def side_of_name(self, name, i, rows):
        """Side of a row that names a Pokémon without "The opposing" (Held Item, Ability)."""
        own = self.own_id(name)
        own = own if own in self.brought else None
        n = clean_name(name)
        k = toid(n)
        cands = FORMS.get(k, [k])
        opp = any(p in cands for p in self.preview if p) or n in self.opp_cache
        if own and not opp:
            return "own", "team"
        if opp and not own:
            return "opp", "team"
        if own and opp:
            oa = self.active["own"] == own
            pa = self.active["opp"] is not None and self.active["opp"] in cands
            if oa and not pa:
                return "own", "active"
            if pa and not oa:
                return "opp", "active"
            for j in list(range(i + 1, min(i + 4, len(rows)))) + list(range(i - 1, max(i - 4, -1), -1)):
                t = norm(rows[j]["text"])
                m = re.match(r"^HP \w+: (.+?)'s HP:? \d+(%?) ", t)
                if m and clean_name(m.group(1)) == n:
                    return ("opp" if m.group(2) else "own"), "neighbour HP row"
            return "opp", "mirror default"
        return "opp", "unknown default"

    # ---- one row -> events
    def row_events(self, r, i, rows):
        t = norm(r["text"])
        if t in NOT_BATTLE:
            return [self.unread(r)], "not-battle"
        for tid, _, rx, h, a in TABLE:
            m = rx.match(t)
            if not m:
                continue
            evs = getattr(self, "h_" + h)(m, a, r, i, rows)
            if evs is None:
                continue
            return evs, tid
        return [self.unread(r)], None

    def unread(self, r):
        t = norm(r["text"])
        e = {"k": "unread", "text": r["text"]}
        m = re.match(r"^(Move[^:]*|HP Lost|HP Restored|Held Item|Ability Changed|Ability|Fainted|Mega Evolution):", t)
        if m:
            e["category"] = m.group(1)
        return e

    def side(self, m):
        return "opp" if m.groupdict().get("o") else "own"

    def who(self, m, key="m"):
        return self.mon(self.side(m) if key == "m" else ("opp" if m.group("o") else "own"), m.group(key))

    def ev(self, k, who=None, **kw):
        e = {"k": k}
        if who:
            e.update(who)
        e.update({x: y for x, y in kw.items() if y is not None})
        return e

    def named(self, s):
        """'the opposing X' or 'X' inside a sentence."""
        m = re.match(r"^(?:the|The) o[pn][pn]osin[ga] (.+)$", s.strip())
        return self.mon("opp", m.group(1)) if m else self.mon("own", s)

    def h_hp(self, m, a, r, i, rows):
        pct = bool(m.group("ap") or m.group("bp"))
        side = "opp" if pct else "own"
        w = self.mon(side, m.group("m"))
        if not w:
            return None
        bef, aft = int(m.group("a")), int(m.group("b"))
        d = "lost" if m.group("cat") == "HP Lost" else "restored"
        if pct:
            return [self.ev("hp", w, dir=d, before_pct=bef, after_pct=aft)]
        return [self.ev("hp", w, dir=d, before=bef, after=aft)]

    def h_move(self, m, a, r, i, rows):
        w = self.who(m)
        mv = move_id(m.group("mv"))
        if not w or not mv:
            return None
        cat = m.group("cat")
        pr = None
        pm = re.search(r"(\d+)", cat[4:])
        if pm:
            sign = "+" if "+" in cat else "-" if re.search(r"[-—−]", cat[4:]) else None
            if sign:
                pr = int(pm.group(1)) * (1 if sign == "+" else -1)
            else:
                self.stats["priority sign unread"] += 1
        self.active[w["side"]] = w["mon"]
        return [self.ev("move", w, move=mv, priority=pr)]

    def h_held_item(self, m, a, r, i, rows):
        it = item_id(m.group("it"))
        if not it:
            return None
        side, how = self.side_of_name(m.group("m"), i, rows)
        self.stats["name-only side by " + how] += 1
        w = self.mon(side, m.group("m"))
        return [self.ev("item", w, item=it, what="activated")] if w else None

    def h_ability_row(self, m, a, r, i, rows):
        ab = ability_id(m.group("a"))
        if not ab:
            return None
        side, how = self.side_of_name(m.group("m"), i, rows)
        self.stats["name-only side by " + how] += 1
        w = self.mon(side, m.group("m"))
        return [self.ev("ability", w, ability=ab)] if w else None

    def h_ability_change(self, m, a, r, i, rows):
        f, t = ability_id(m.group("a")), ability_id(m.group("b"))
        if not f or not t:
            return None
        side, how = self.side_of_name(m.group("m"), i, rows)
        self.stats["name-only side by " + how] += 1
        w = self.mon(side, m.group("m"))
        return [self.ev("ability_change", w, **{"from": f, "to": t})] if w else None

    def h_simple(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev(a[0], w)] if w else None

    def h_mega(self, m, a, r, i, rows):
        w = self.who(m)
        if not w:
            return None
        base = w["mon"]
        megas = SPECIES[base].get("megas") or {}
        form = None
        if w["side"] == "own":
            it = self.team[self.own_ids.index(base)].get("item")
            form = megas.get(it)
        else:
            form = megas.get(self.stone.get(base))
            if not form:
                s = (r.get("sprite") or {})
                if s.get("status") == "matched" and toid(s.get("name")) in SPECIES and SPECIES[toid(s["name"])].get("isMega"):
                    form = toid(s["name"])
                    self.stats["opp mega form by sprite"] += 1
            if not form and len(set(megas.values())) == 1:
                form = list(megas.values())[0]
                self.stats["opp mega form by the only Mega"] += 1
        if not form:
            self.stats["mega form unknown"] += 1
            return None
        return [self.ev("mega", w, form=form)]

    def h_enter_own(self, m, a, r, i, rows):
        w = self.mon("own", m.group("m"))
        if not w:
            return None
        self.active["own"] = w["mon"]
        return [self.ev("enter", w)]

    def h_enter_opp(self, m, a, r, i, rows):
        w = self.mon("opp", m.group("m"))
        if not w:
            return None
        self.active["opp"] = w["mon"]
        return [self.ev("enter", w)]

    def h_withdraw(self, m, a, r, i, rows):
        w = self.mon(a[0], m.group("m"))
        return [self.ev("withdraw", w)] if w else None

    def h_withdraw_pivot(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("withdraw", w, pivot=True)] if w else None

    def h_dragged(self, m, a, r, i, rows):
        w = self.who(m)
        if not w:
            return None
        self.active[w["side"]] = w["mon"]
        return [self.ev("enter", w, forced=True)]

    def h_result(self, m, a, r, i, rows):
        return [self.ev("result", how=a[0])]

    def h_immune(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("result", w, how="immune")] if w else None

    def h_hits(self, m, a, r, i, rows):
        return [self.ev("hits", n=int(m.group("n")))]

    def h_fail_bare(self, m, a, r, i, rows):
        return [self.ev("fail")]

    def stats_of(self, s):
        found = STAT_RE.findall(s)
        rest = STAT_RE.sub("", s)
        if not found or re.search(r"[A-Za-z]", re.sub(r"\band\b", "", rest)):
            return None
        return [STAT_ID[toid(x)] for x in found]

    def h_boost_limit(self, m, a, r, i, rows):
        w = self.who(m)
        st = self.stats_of(m.group("s"))
        if not w or not st:
            return None
        return [self.ev("boost_limit", w, stats=st, dir="up" if m.group("d") == "higher" else "down")]

    def h_boost(self, m, a, r, i, rows):
        w = self.who(m)
        st = self.stats_of(m.group("s"))
        if not w or not st:
            return None
        return [self.ev("boost", w, stats=st, by=BY[m.group("w")])]

    def h_status(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("status", w, status=a[0])] if w else None

    def h_cant(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("cant", w, reason=a[0])] if w else None

    def h_cure(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("cure", w, status=a[0])] if w else None

    def h_cure_item(self, m, a, r, i, rows):
        w = self.who(m)
        it = item_id(m.group("it"))
        st = {"cured its poisoning": "psn", "cured its paralysis": "par", "cured its burn": "brn", "woke it up": "slp", "defrosted it": "frz"}[m.group("w")]
        return [self.ev("cure", w, status=st, item=it)] if w and it else None

    def h_vol(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("volatile", w, effect=a[0])] if w else None

    def h_vol_move(self, m, a, r, i, rows):
        w = self.who(m)
        mv = move_id(m.group("mv"))
        return [self.ev("volatile", w, effect=a[0], detail=mv)] if w and mv else None

    def h_perish(self, m, a, r, i, rows):
        w = self.who(m)
        n = m.group("n")
        return [self.ev("volatile", w, effect="perishsong", detail=0 if n == "O" else int(n))] if w else None

    def h_curse_on(self, m, a, r, i, rows):
        w = self.named(m.group("m"))
        return [self.ev("volatile", w, effect="curse")] if w else None

    def h_vol_end(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("volatile_end", w, effect=a[0])] if w else None

    def h_vol_end_move(self, m, a, r, i, rows):
        w = self.who(m)
        mv = move_id(m.group("mv"))
        return [self.ev("volatile_end", w, effect=mv)] if w and mv else None

    def h_vol_hit(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("volatile_hit", w, effect=a[0])] if w else None

    def h_vol_hit_bare(self, m, a, r, i, rows):
        return [self.ev("volatile_hit", effect=a[0])]

    def h_cause(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("cause", w, effect=a[0])] if w else None

    def h_cause_move(self, m, a, r, i, rows):
        w = self.who(m)
        mv = move_id(m.group("mv"))
        return [self.ev("cause", w, effect=mv)] if w and mv else None

    def h_cause_of(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("cause", effect=a[0], of=w)] if w else None

    def h_cause_bare(self, m, a, r, i, rows):
        return [self.ev("cause", effect=a[0])]

    def h_item(self, m, a, r, i, rows):
        w = self.who(m)
        it = item_id(m.group("it"))
        return [self.ev("item", w, item=it, what=a[0])] if w and it else None

    def h_item_of(self, m, a, r, i, rows):
        w = self.who(m)
        it = item_id(m.group("it"))
        of = self.named(m.group("m2"))
        return [self.ev("item", w, item=it, what=a[0], of=of)] if w and it and of else None

    def h_item_knock(self, m, a, r, i, rows):
        knocker = self.mon("opp" if m.group("o") else "own", m.group("m2"))
        w = self.named(m.group("m"))
        it = item_id(m.group("it"))
        return [self.ev("item", w, item=it, what="removed", of=knocker)] if w and it and knocker else None

    def h_item_frisk(self, m, a, r, i, rows):
        frisker = self.mon("opp" if m.group("o") else "own", m.group("m2"))
        w = self.named(m.group("m"))
        it = item_id(m.group("it"))
        return [self.ev("item", w, item=it, what="found", of=frisker)] if w and it and frisker else None

    def h_white_herb(self, m, a, r, i, rows):
        w = self.who(m)
        it = item_id(m.group("it"))
        if not w or not it:
            return None
        self.stats["two-event rows (White Herb)"] += 1
        return [self.ev("item", w, item=it, what="used"), self.ev("boost_reset", w, what="negative")]

    def h_mega_stone(self, m, a, r, i, rows):
        w = self.who(m)
        it = item_id(m.group("it"))
        if not w or not it:
            return None
        self.stone[w["mon"]] = it
        return [self.ev("mega_stone", w, stone=it)]

    def h_ability(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("ability", w, ability=a[0])] if w else None

    def h_ability_named(self, m, a, r, i, rows):
        w = self.who(m)
        ab = ability_id(m.group("a"))
        return [self.ev("ability", w, ability=ab)] if w and ab else None

    def h_types(self, m, a, r, i, rows):
        w = self.who(m)
        ty = m.group("t").lower()
        return [self.ev("types", w, type=ty)] if w and ty in TYPES else None

    def h_transform(self, m, a, r, i, rows):
        w = self.who(m)
        x = clean_name(m.group("x"))
        if not w:
            return None
        if w["mon"] == "ditto":
            into = self.opp_id(x) if w["side"] == "own" else self.own_id(x)
            return [self.ev("transform", w, into=into)] if into else None
        k = toid(x)
        return [self.ev("form", w, form=k)] if k in SPECIES else None

    def h_form(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("form", w, form=a[0])] if w else None

    def h_once(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("once", w, effect=a[0])] if w else None

    def h_once_bare(self, m, a, r, i, rows):
        return [self.ev("once", effect=a[0])]

    def h_prepare(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("prepare", w)] if w else None

    def h_blocked(self, m, a, r, i, rows):
        w = self.who(m)
        return [self.ev("blocked", w, effect=a[0], what=a[1])] if w else None

    def h_blocked_side(self, m, a, r, i, rows):
        # names a side, not a Pokémon: the side is lost (contract gap)
        self.stats["blocked row naming a side"] += 1
        return [self.ev("blocked", effect="unnerve", what="berries")]

    def h_weather(self, m, a, r, i, rows):
        return [self.ev("weather", weather=a[0], what=a[1])]

    def h_field(self, m, a, r, i, rows):
        return [self.ev("field", effect=a[0], what=a[1])]

    def side_word(self, m):
        return "own" if m.group("side").lower() == "your" else "opp"

    def h_side(self, m, a, r, i, rows):
        return [{"k": "side", "side": self.side_word(m), "effect": a[0], "what": a[1]}]

    def h_side_named(self, m, a, r, i, rows):
        return [{"k": "side", "side": self.side_word(m), "effect": toid(m.group("eff")), "what": a[0]}]

    def h_side_acted(self, m, a, r, i, rows):
        w = self.who(m)
        return [{"k": "side", "side": w["side"], "effect": a[0], "what": "acted"}] if w else None

    def h_haze(self, m, a, r, i, rows):
        return [self.ev("boost_reset", what="all")]

    # ---- decisions
    def order_of(self, d):
        o = d.get("order") or ""
        parts = o.split()
        req = d.get("request") or {}
        if o.startswith("/choose move") and len(parts) >= 3:
            mv = move_id(parts[2]) or parts[2]
            return {"kind": "move", "move": mv, "mega": "mega" in parts[3:]}
        if o.startswith("/choose switch"):
            sid = self.own_id(" ".join(parts[2:]))
            return {"kind": "switch", "mon": sid} if sid else None
        c = d.get("choice") or o
        m = re.match(r"(mega\+)?move(\d)", c)
        if m:
            try:
                mv = req["active"][0]["moves"][int(m.group(2))]["id"]
            except (KeyError, IndexError, TypeError):
                return None
            return {"kind": "move", "move": mv, "mega": bool(m.group(1))}
        m = re.match(r"switch(\d)", c)
        if m:
            try:
                p = req["side"]["pokemon"][int(m.group(1))]
            except (KeyError, IndexError, TypeError):
                return None
            return {"kind": "switch", "mon": toid(p["details"].split(",")[0])}
        return None

    # ---- the whole stream
    def convert(self):
        out = []

        def msg(type_, **kw):
            o = {"v": V, "battle": self.tag, "type": type_}
            o.update(kw)
            out.append(o)

        msg("battle_start", own_team=self.team, opp_preview=self.preview)
        out.append({"picks": self.picks})
        by_turn = {}
        for r in self.rows:
            by_turn.setdefault(r["turn"], []).append(r)
        for t in by_turn:
            by_turn[t].sort(key=lambda r: r["index"])
        actions, forced = {}, {}
        for d in self.decisions:
            (actions if d["kind"] == "action" else forced).setdefault(d["turn"], []).append(d)
        cov = self.coverage.get("coverage", [])
        gap_turns = {c["turn"]: c["kind"] for c in cov if c.get("kind") in ("no_own_action_screen", "final_turn") and c.get("turn")}
        gap_rows = Counter()
        for f in self.coverage.get("failures", []) + [c.get("detail", "") for c in cov if c.get("kind") == "failure"]:
            mt = re.search(r"turn (\d+)", str(f))
            mv = re.search(r"visit (\d+)", str(f))
            if mt:
                gap_rows[int(mt.group(1))] += 1
            elif mv:
                # the rows of a visit are those read before the decision of that visit
                v = int(mv.group(1))
                d = next((d for d in self.decisions if d.get("visit") == v), None)
                gap_rows[(d.get("log_turn") or d["turn"]) if d else -1] += 1
        last = max([0] + list(by_turn) + list(actions) + list(forced) + list(gap_turns))
        conv = {}
        tids = Counter()
        for t, rows in by_turn.items():
            lst = []
            for i, r in enumerate(rows):
                evs, tid = self.row_events(r, i, rows)
                tids[tid or "unread"] += 1
                for e in evs:
                    e["src"] = "%d.%d" % (r["turn"], r["index"])
                lst.append(evs)
            conv[t] = lst
        self.tids = tids

        buf = []

        def flush():
            if buf:
                msg("events", events=list(buf))
                buf.clear()

        seq_prev = {}

        def decide(group, kind):
            prev = None
            for j, d in enumerate(group):
                order = self.order_of(d)
                nxt = group[j + 1] if j + 1 < len(group) else None
                flush()
                req = {"seq": d["sequence"], "kind": kind}
                if prev is not None and d.get("same_turn", d.get("attempt", 1) > 1) and prev != order:
                    req["refused"] = {"order": prev}
                msg("decision_request", **req)
                if nxt is None:
                    rec = "accepted"
                else:
                    no = self.order_of(nxt)
                    rec = "refused" if no != order else "retry"
                if order is None:
                    self.stats["order unreadable"] += 1
                    rec = "unreadable"
                else:
                    out.append({"seq": d["sequence"], "order": order, "notes": {"recorded": rec, "turn": d["turn"], "file": "%04d-%s" % (d["sequence"], d["kind"])}})
                prev = order
                self.stats["decisions " + rec] += 1

        pre_kinds = {"enter", "ability", "boost", "weather", "field", "item", "side"}
        for t in range(1, last + 1):
            lst = conv.get(t, [])
            if t == 1:
                k = 0
                while k < len(lst) and all(e["k"] in pre_kinds and not e.get("forced") for e in lst[k]):
                    k += 1
                for evs in lst[:k]:
                    buf.extend(evs)
                lst = lst[k:]
            buf.append({"k": "turn", "n": t})
            if t in gap_turns:
                buf.append({"k": "gap", "what": "turn", "n": t})
                self.stats["gap turn (%s)" % gap_turns[t]] += 1
            if gap_rows.get(t):
                buf.append({"k": "gap", "what": "rows", "n": t})
                self.stats["gap rows"] += 1
            decide(actions.get(t, []), "turn")
            if not actions.get(t) and lst:
                self.stats["turns with rows and no turn decision"] += 1
            # replace decisions go before an own enter that follows a faint or a pivot withdraw of ours
            fq = list(forced.get(t, []))
            groups = []
            for d in fq:
                if groups and d.get("attempt", 1) > 1 and groups[-1][-1]["turn"] == d["turn"]:
                    groups[-1].append(d)
                else:
                    groups.append([d])
            pending = False
            for evs in lst:
                for e in evs:
                    if e.get("side") == "own" and (e["k"] == "faint" or (e["k"] == "withdraw" and e.get("pivot")) or (e["k"] == "item" and e.get("what") == "used" and e.get("item") == "ejectbutton")):
                        pending = True
                    if e["k"] == "enter" and e.get("side") == "own" and not e.get("forced") and pending and groups:
                        decide(groups.pop(0), "replace")
                        pending = False
                    elif e["k"] == "enter" and e.get("side") == "own":
                        pending = False
                    buf.append(e)
            for g in groups:
                self.stats["replace decision placed at turn end"] += 1
                decide(g, "replace")
        flush()
        oc = (self.meta or {}).get("outcome", "unknown")
        msg("battle_end", result={"opponent_forfeit": "forfeit"}.get(oc, oc if oc in ("win", "loss", "tie") else "unknown"))
        return out


def main():
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)
    summ = []
    dirs = []
    for a in sys.argv[2:]:
        # a runs folder stands for all its battles
        dirs += sorted(glob.glob(os.path.join(a, "*", "battle-*"))) if not os.path.exists(os.path.join(a, "log.jsonl")) and not glob.glob(os.path.join(a, "battle-*")) else [a]
    for bdir in dirs:
        if not os.path.exists(os.path.join(bdir, "log.jsonl")) or os.path.getsize(os.path.join(bdir, "log.jsonl")) == 0:
            continue
        b = Battle(bdir)
        msgs = b.convert()
        name = os.path.basename(os.path.dirname(os.path.normpath(bdir))) + "__" + os.path.basename(os.path.normpath(bdir))
        with open(os.path.join(out_dir, name + ".jsonl"), "w", encoding="utf-8", newline="\n") as f:
            for m in msgs:
                f.write(json.dumps(m, ensure_ascii=False) + "\n")
        kinds = Counter()
        unread = []
        for m in msgs:
            for e in m.get("events", []):
                kinds[e["k"]] += 1
                if e["k"] == "unread":
                    unread.append(e["text"])
        summ.append({"battle": name, "rows": len(b.rows), "kinds": kinds, "templates": b.tids, "unread": unread,
                     "stats": b.stats, "notes": b.notes[:20]})
    with open(os.path.join(out_dir, "convert-summary.jsonl"), "w", encoding="utf-8", newline="\n") as f:
        for s in summ:
            f.write(json.dumps(s, ensure_ascii=False) + "\n")
    print("converted", len(summ))


if __name__ == "__main__":
    main()
