"""Writes tools/rig/data/stage5_scenarios4.json: status immunities and blocks reached through Synchronize and
other non-move sources, and the row faults the rowhunt found."""
import json
import os
import sys

OUT = r"C:\Users\user\Code\pokemon_champions\champions-gym\tools\rig\data\stage5_scenarios4.json"
S = []


def mon(species, ability, moves, item=""):
    return {"species": species, "ability": ability, "item": item, "moves": moves}


def sc(name, p1, p2, steps, expect, seed=None, picks=None):
    d = {"name": name, "teams": {"p1": p1, "p2": p2}, "steps": [{"p1": a, "p2": b} for a, b in steps], "expect": expect}
    if seed:
        d["seed"] = seed
    if picks:
        d["picks"] = picks
    S.append(d)


SYNC = mon("Umbreon", "Synchronize", ["Calm Mind", "Facade"])
CM = [("move 0", "move 0")] * 3

# Synchronize passes a move's status back: the status-immunity abilities show (effect.status is set)
sc("synchronize passes toxic back to an immunity holder: immune row", [mon("Snorlax", "Immunity", ["Toxic"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Immunity"])
sc("synchronize passes paralysis back to a limber holder: immune row", [mon("Persian", "Limber", ["Thunder Wave"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Limber"])
sc("synchronize passes a burn back to a water veil holder: immune row", [mon("Milotic", "Water Veil", ["Will-O-Wisp"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Water Veil"])
sc("synchronize passes a burn back to a water bubble holder: immune row", [mon("Milotic", "Water Bubble", ["Will-O-Wisp"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Water Bubble"])
sc("synchronize passes a burn back to a thermal exchange holder: immune row", [mon("Milotic", "Thermal Exchange", ["Will-O-Wisp"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Thermal Exchange"])
sc("synchronize passes toxic back to a pastel veil holder: its ally handler's block row", [mon("Snorlax", "Pastel Veil", ["Toxic"])], [SYNC], CM,
   ["|-block|p1a: Mon0|ability: Pastel Veil"])
sc("synchronize passes toxic back to a purifying salt holder: immune row", [mon("Garganacl", "Purifying Salt", ["Toxic"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Purifying Salt"])
sc("synchronize passes toxic back to a comatose holder: immune row", [mon("Snorlax", "Comatose", ["Toxic"])], [SYNC], CM,
   ["|-immune|p1a: Mon0|[from] ability: Comatose"])
sc("synchronize passes toxic back to a leaf guard holder in sun: immune row",
   [mon("Leafeon", "Leaf Guard", ["Sunny Day", "Toxic"])], [SYNC], [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")],
   ["|-immune|p1a: Mon0|[from] ability: Leaf Guard"])
sc("synchronize passes a burn back to a flower veil grass holder: silent (the effect has no name)",
   [mon("Venusaur", "Flower Veil", ["Will-O-Wisp"])], [SYNC], CM, ["|-activate|p2a: Mon0|ability: Synchronize"])
sc("two synchronize holders: the second pass fails with the status row",
   [mon("Espeon", "Synchronize", ["Toxic"])], [SYNC], CM, ["|-fail|p2a: Mon0|tox"])
# type immunities through Synchronize: the bare immune row
sc("synchronize passes toxic back to a steel type: immune row", [mon("Corviknight", "Pressure", ["Toxic"])], [SYNC], CM,
   ["|-immune|p1a: Mon0"])
sc("synchronize passes paralysis back to an electric type: immune row", [mon("Jolteon", "Volt Absorb", ["Thunder Wave"])], [SYNC], CM,
   ["|-immune|p1a: Mon0"])
sc("synchronize passes a burn back to a fire type: immune row", [mon("Arcanine", "Justified", ["Will-O-Wisp"])], [SYNC], CM,
   ["|-immune|p1a: Mon0"])
sc("corrosion poisons a steel synchronize holder; the poison goes back to the poison type: immune row",
   [mon("Salazzle", "Corrosion", ["Toxic"])], [mon("Corviknight", "Synchronize", ["Calm Mind"])], CM, ["|-immune|p1a: Mon0"])
# field blocks through Synchronize
sc("synchronize's poison into misty terrain on a grounded user: misty terrain row",
   [mon("Hatterene", "Magic Bounce", ["Misty Terrain", "Toxic"])], [mon("Umbreon", "Synchronize", ["Calm Mind"], "Air Balloon")],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")], ["|-activate|p1a: Mon0|move: Misty Terrain"])
sc("synchronize's poison into safeguard: safeguard row",
   [mon("Snorlax", "Thick Fat", ["Safeguard", "Toxic"])], [SYNC],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")], ["|-activate|p1a: Mon0|move: Safeguard"])
# Synchronize against a user already statused
sc("synchronize against a paralysed user: the failure row names the synchronize holder",
   [mon("Snorlax", "Thick Fat", ["Toxic"])], [mon("Umbreon", "Synchronize", ["Thunder Wave", "Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "move 1"), ("move 0", "move 1"), ("move 0", "move 1")], ["|-fail|p2a: Mon0"])
sc("synchronize against a user with the same status: the failure row names the user and the status",
   [mon("Snorlax", "Thick Fat", ["Toxic"])], [mon("Umbreon", "Synchronize", ["Toxic", "Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "move 1"), ("move 0", "move 1")], ["|-fail|p1a: Mon0|tox"])
sc("static paralyses a synchronize attacker; the paralysis goes back to the electric holder: immune row",
   [mon("Pikachu", "Static", ["Calm Mind"])], [mon("Umbreon", "Synchronize", ["Facade"])], [("move 0", "move 0")] * 8,
   ["|-immune|p1a: Mon0"])
# non-move sources: no immune or block row (effect.status is not set)
sc("static against a limber attacker: silent", [mon("Pikachu", "Static", ["Calm Mind"])], [mon("Persian", "Limber", ["Facade"])],
   [("move 0", "move 0")] * 8, [])
sc("poison point against an immunity attacker: silent", [mon("Toxapex", "Poison Point", ["Calm Mind"])],
   [mon("Snorlax", "Immunity", ["Facade"])], [("move 0", "move 0")] * 8, [])
sc("flame body against a water veil attacker: silent", [mon("Arcanine", "Flame Body", ["Calm Mind"])],
   [mon("Milotic", "Water Veil", ["Facade"])], [("move 0", "move 0")] * 8, [])
sc("effect spore against a vital spirit attacker: silent", [mon("Venusaur", "Effect Spore", ["Calm Mind"])],
   [mon("Persian", "Vital Spirit", ["Facade"])], [("move 0", "move 0")] * 8, [])
sc("toxic chain against an immunity target: silent", [mon("Salazzle", "Toxic Chain", ["Flamethrower"])],
   [mon("Snorlax", "Immunity", ["Calm Mind"])], [("move 0", "move 0")] * 6, [])
sc("static against an attacker under safeguard: silent", [mon("Pikachu", "Static", ["Calm Mind"])],
   [mon("Persian", "Technician", ["Safeguard", "Facade"])], [("move 0", "move 0")] + [("move 0", "move 1")] * 4, [])
sc("static against an attacker in misty terrain: silent", [mon("Pikachu", "Static", ["Misty Terrain", "Calm Mind"])],
   [mon("Persian", "Technician", ["Facade"])], [("move 0", "move 0")] + [("move 1", "move 0")] * 4, [])
sc("toxic spikes on an immunity switch-in: silent; on a poison type: absorbed",
   [mon("Corviknight", "Pressure", ["Toxic Spikes", "Roar"])],
   [mon("Persian", "Technician", ["Facade"]), mon("Snorlax", "Immunity", ["Facade"]), mon("Toxapex", "Regenerator", ["Facade"])],
   [("move 0", "move 0"), ("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")],
   ["|-sidestart|p2: p2|move: Toxic Spikes"])
sc("toxic spikes into safeguard and misty terrain: silent",
   [mon("Corviknight", "Pressure", ["Toxic Spikes", "Roar"])],
   [mon("Hatterene", "Magic Guard", ["Safeguard", "Misty Terrain"]), mon("Snorlax", "Thick Fat", ["Facade"]),
    mon("Persian", "Technician", ["Facade"])],
   [("move 0", "move 0"), ("move 0", "move 1"), ("move 1", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")], [])
# Yawn's sleep: its condition is the effect
sc("yawn's sleep into electric terrain: terrain row", [mon("Persian", "Technician", ["Calm Mind", "Electric Terrain"])],
   [mon("Snorlax", "Thick Fat", ["Yawn", "Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 1"), ("move 0", "move 1")],
   ["|-activate|p1a: Mon0|move: Electric Terrain"])
sc("yawn's sleep into misty terrain: terrain row", [mon("Persian", "Technician", ["Calm Mind", "Misty Terrain"])],
   [mon("Snorlax", "Thick Fat", ["Yawn", "Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 1"), ("move 0", "move 1")],
   ["|-activate|p1a: Mon0|move: Misty Terrain"])
sc("yawn's sleep goes through safeguard set after it", [mon("Persian", "Technician", ["Calm Mind", "Safeguard"])],
   [mon("Snorlax", "Thick Fat", ["Yawn", "Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 1"), ("move 0", "move 1")],
   ["|-status|p1a: Mon0|slp"])
sc("yawn's sleep against leaf guard in sun set after it: silent", [mon("Leafeon", "Leaf Guard", ["Calm Mind", "Sunny Day"])],
   [mon("Snorlax", "Thick Fat", ["Yawn", "Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 1"), ("move 0", "move 1")], [])
# Rest
sc("rest: insomnia and vital spirit fail with their rows", [mon("Persian", "Insomnia", ["Calm Mind", "Rest"]), mon("Snorlax", "Vital Spirit", ["Calm Mind", "Rest"])],
   [mon("Garchomp", "Rough Skin", ["Facade"])], [("move 0", "move 0"), ("move 1", "move 0"), ("switch 1", "move 0"), ("move 0", "move 0"), ("move 1", "move 0")],
   ["[from] ability: Insomnia", "[from] ability: Vital Spirit"])
sc("rest in electric terrain: terrain row; in misty terrain: silent",
   [mon("Persian", "Technician", ["Electric Terrain", "Misty Terrain", "Rest"])], [mon("Garchomp", "Rough Skin", ["Facade"])],
   [("move 0", "move 0"), ("move 2", "move 0"), ("move 1", "move 0"), ("move 2", "move 0")],
   ["|-activate|p1a: Mon0|move: Electric Terrain"])
sc("rest with leaf guard in sun: silent failure", [mon("Leafeon", "Leaf Guard", ["Sunny Day", "Rest"])],
   [mon("Garchomp", "Rough Skin", ["Facade"])], [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")], [])

import glob
for extra in sorted(glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen_s4_extra*.py"))):
    exec(open(extra, encoding="utf-8").read())

with open(OUT, "w", encoding="utf-8") as f:
    json.dump(S, f, indent=1)
print(len(S), "scenarios")
