# Run by gen_s4.py (exec): ability immunity and block rows, by reading.
sc("soundproof against perish song and uproar", [mon("Gengar", "Cursed Body", ["Perish Song", "Uproar"])],
   [mon("Snorlax", "Soundproof", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0")],
   ["|-immune|p2a: Mon0|[from] ability: Soundproof"])
sc("bulletproof against shadow ball and sludge bomb", [mon("Gengar", "Cursed Body", ["Shadow Ball", "Sludge Bomb"])],
   [mon("Snorlax", "Bulletproof", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0")],
   ["|-immune|p2a: Mon0|[from] ability: Bulletproof"])
sc("good as gold against taunt and toxic", [mon("Gengar", "Cursed Body", ["Taunt", "Toxic"])],
   [mon("Gholdengo", "Good as Gold", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0")],
   ["|-immune|p2a: Mon0|[from] ability: Good as Gold"])
sc("magic bounce reflects toxic, taunt and stealth rock", [mon("Gengar", "Cursed Body", ["Toxic", "Taunt", "Stealth Rock"])],
   [mon("Hatterene", "Magic Bounce", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0")],
   ["[from] ability: Magic Bounce"])
sc("wonder guard against neutral hits and toxic damage", [mon("Persian", "Technician", ["Facade", "Toxic"])],
   [mon("Garchomp", "Wonder Guard", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0")],
   ["|-immune|p2a: Mon0|[from] ability: Wonder Guard"])
sc("overcoat and a grass type against spore and sleep powder",
   [mon("Venusaur", "Overgrow", ["Spore", "Sleep Powder"])],
   [mon("Snorlax", "Overcoat", ["Calm Mind"]), mon("Leafeon", "Leaf Guard", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "switch 1"), ("move 0", "move 0")], ["|-immune|p2a: Mon0|[from] ability: Overcoat"])
sc("psychic terrain blocks priority on a grounded target; dazzling blocks prankster's status move",
   [mon("Hatterene", "Magic Bounce", ["Psychic Terrain", "Calm Mind"])],
   [mon("Persian", "Technician", ["Quick Attack"]), mon("Garchomp", "Rough Skin", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 0")], ["|-activate|p1a: Mon0|move: Psychic Terrain"])
sc("prankster's status move against a dark type: immune row", [mon("Sableye", "Prankster", ["Taunt", "Thunder Wave"])],
   [mon("Umbreon", "Inner Focus", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0")], ["|-immune|p2a: Mon0"])
sc("levitate and air balloon against ground moves; earth eater heals",
   [mon("Garchomp", "Rough Skin", ["Earthquake"])], [mon("Gengar", "Levitate", ["Calm Mind"]), mon("Snorlax", "Thick Fat", ["Calm Mind"], "Air Balloon")],
   [("move 0", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0")], ["|-immune|p2a: Mon0|[from] ability: Levitate"])
