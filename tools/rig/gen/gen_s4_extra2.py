# Run by gen_s4.py (exec): one-off rows of moves (once, activate) and form changes, by reading.
P = mon("Persian", "Technician", ["Calm Mind"])
SN = mon("Snorlax", "Thick Fat", ["Calm Mind"])

sc("feint breaks protect", [mon("Persian", "Technician", ["Feint", "Calm Mind"])], [mon("Snorlax", "Thick Fat", ["Protect", "Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 1"), ("move 0", "move 0")], ["move: Feint"])
sc("lock-on then an ohko move", [mon("Glaceon", "Snow Cloak", ["Lock-On", "Sheer Cold"])], [SN],
   [("move 0", "move 0"), ("move 1", "move 0")], ["move: Lock-On"])
sc("guard split, power split and speed swap", [mon("Slowbro", "Oblivious", ["Guard Split", "Power Split", "Speed Swap"])], [SN],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0")], ["move: Guard Split", "move: Power Split", "move: Speed Swap"])
sc("tidy up clears hazards and substitutes", [mon("Persian", "Technician", ["Tidy Up", "Substitute"])],
   [mon("Garchomp", "Rough Skin", ["Stealth Rock", "Spikes"])], [("move 1", "move 0"), ("move 1", "move 1"), ("move 0", "move 0")],
   ["move: Tidy Up"])
sc("heal bell: a soundproof team member keeps its status",
   [mon("Clefable", "Magic Guard", ["Heal Bell", "Calm Mind"]), mon("Snorlax", "Soundproof", ["Calm Mind"])],
   [mon("Gengar", "Cursed Body", ["Toxic", "Will-O-Wisp"])],
   [("switch 1", "move 0"), ("switch 0", "move 1"), ("move 0", "move 1")], ["move: Heal Bell"])
sc("ingrain stops roar; fairy lock", [mon("Venusaur", "Overgrow", ["Ingrain", "Fairy Lock"])], [mon("Garchomp", "Rough Skin", ["Roar"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")], ["move: Ingrain"])
sc("court change swaps the hazards", [mon("Cinderace", "Blaze", ["Court Change", "Calm Mind"])],
   [mon("Garchomp", "Rough Skin", ["Stealth Rock", "Spikes"])], [("move 1", "move 0"), ("move 1", "move 1"), ("move 0", "move 0")],
   ["move: Court Change"])
sc("electrify makes the next move electric", [mon("Jolteon", "Volt Absorb", ["Electrify", "Calm Mind"])],
   [mon("Garchomp", "Rough Skin", ["Facade"])], [("move 0", "move 0"), ("move 0", "move 0")], ["Electrify"])
sc("endure, focus punch interrupted, beak blast burns on contact",
   [mon("Snorlax", "Thick Fat", ["Endure", "Focus Punch", "Beak Blast"])], [mon("Garchomp", "Rough Skin", ["Facade", "Earth Power"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0"), ("move 1", "move 1")], ["Endure", "Focus Punch", "Beak Blast"])
sc("destiny bond takes the attacker down", [mon("Gengar", "Cursed Body", ["Destiny Bond"])], [mon("Garchomp", "Rough Skin", ["Crunch"])],
   rep("move 0", "move 0", 4), ["Destiny Bond"])
sc("skill swap, role play, entrainment, simple beam, worry seed, gastro acid",
   [mon("Slowbro", "Oblivious", ["Skill Swap", "Role Play", "Entrainment", "Simple Beam"]), mon("Slowking", "Own Tempo", ["Worry Seed", "Gastro Acid"])],
   [mon("Garchomp", "Rough Skin", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0"), ("move 3", "move 0"), ("switch 1", "move 0"), ("move 0", "move 0"), ("move 1", "move 0")],
   ["Skill Swap"])
sc("instruct, quash and after you fail in singles; copycat copies", [mon("Slowbro", "Oblivious", ["Instruct", "Quash", "After You", "Copycat"])],
   [mon("Garchomp", "Rough Skin", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0"), ("move 3", "move 0")], [])
sc("castform's forecast in sun, rain and snow, then cloud nine",
   [mon("Castform", "Forecast", ["Sunny Day", "Rain Dance", "Snowscape", "Calm Mind"])], [mon("Persian", "Technician", ["Calm Mind"]), mon("Slowking", "Cloud Nine", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0"), ("move 3", "switch 1"), ("move 0", "move 0")], ["Castform-Sunny"])
sc("healing wish and wish", [mon("Clefable", "Magic Guard", ["Wish", "Healing Wish"]), SN],
   [mon("Garchomp", "Rough Skin", ["Facade"])], [("move 0", "move 0"), ("move 1", "move 0"), ("move 0", "move 0"), ("move 0", "move 0")],
   ["Healing Wish"])
sc("pain split, spite and eerie spell", [mon("Gengar", "Cursed Body", ["Pain Split", "Spite"]), mon("Slowking", "Own Tempo", ["Eerie Spell"])],
   [mon("Garchomp", "Rough Skin", ["Facade"])], [("move 0", "move 0"), ("move 1", "move 0"), ("switch 1", "move 0"), ("move 0", "move 0")],
   ["Pain Split", "move: Spite"])
sc("poltergeist against no item fails; corrosive gas; sticky web",
   [mon("Gengar", "Cursed Body", ["Poltergeist", "Corrosive Gas", "Sticky Web"])],
   [mon("Snorlax", "Thick Fat", ["Calm Mind"], "Leftovers"), mon("Persian", "Technician", ["Calm Mind"])],
   [("move 1", "move 0"), ("move 0", "move 0"), ("move 2", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0")], ["Corrosive Gas"])
sc("imprison, disable, taunt, torment and throat chop refuse moves",
   [mon("Slowbro", "Oblivious", ["Imprison", "Disable", "Taunt", "Calm Mind"])],
   [mon("Garchomp", "Rough Skin", ["Calm Mind", "Swords Dance", "Facade", "Throat Chop"])],
   [("move 0", "move 0"), ("move 1", "move 2"), ("move 2", "move 2"), ("move 3", "move 1"), ("move 3", "move 3")], ["Imprison"])
sc("psychic noise blocks healing; syrup bomb; salt cure; octolock",
   [mon("Slowbro", "Oblivious", ["Psychic Noise", "Syrup Bomb", "Salt Cure", "Octolock"])],
   [mon("Snorlax", "Thick Fat", ["Rest", "Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 1"), ("move 2", "move 1"), ("move 3", "move 1"), ("move 0", "move 1")], ["Syrup Bomb"])
sc("uproar wakes and keeps awake", [mon("Snorlax", "Thick Fat", ["Uproar"])], [mon("Persian", "Technician", ["Rest", "Yawn"])],
   [("move 0", "move 1"), ("move 0", "move 0"), ("move 0", "move 0")], ["Uproar"])
sc("revival blessing brings back a fainted member", [mon("Persian", "Technician", ["Revival Blessing", "Memento"]), SN, P],
   [mon("Garchomp", "Rough Skin", ["Calm Mind"])], [("move 1", "move 0"), ("move 0", "move 0"), ("move 0", "move 0")], [])
sc("transform copies, then the copied moves", [mon("Ditto", "Limber", ["Transform"])], [mon("Garchomp", "Rough Skin", ["Swords Dance", "Earthquake"])],
   [("move 0", "move 0"), ("move 0", "move 1"), ("move 1", "move 1")], ["-transform"])
