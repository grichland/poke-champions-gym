# Run by gen_s4.py (exec): the rowhunt's faults and rare rows found by reading.
F = "Female"


def mons(*xs):
    return list(xs)


def rep(a, b, n):
    return [(a, b)] * n


# RH-3: Wide Guard and Quick Guard against Piercing Drill and Unseen Fist
sc("wide guard against piercing drill's spread contact move: broken, a quarter of the damage",
   [mon("Snorlax", "Thick Fat", ["Wide Guard", "Calm Mind"])], [mon("Excadrill", "Piercing Drill", ["Mortal Spin", "Brutal Swing"])],
   [("move 0", "move 0"), ("move 1", "move 1"), ("move 0", "move 1")], ["|-zbroken|p1a: Mon0"])
sc("wide guard against unseen fist's spread contact move", [mon("Snorlax", "Thick Fat", ["Wide Guard"])],
   [mon("Kingambit", "Unseen Fist", ["Brutal Swing"])], [("move 0", "move 0")], ["|-zbroken|p1a: Mon0"])
sc("quick guard against unseen fist's priority contact move; a non-contact one is blocked",
   [mon("Snorlax", "Thick Fat", ["Quick Guard", "Calm Mind"])], [mon("Kingambit", "Unseen Fist", ["Quick Attack", "Vacuum Wave"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 0", "move 1")], ["|-zbroken|p1a: Mon0", "|-activate|p1a: Mon0|move: Quick Guard"])
# RH-4: Red Card used by a residual hit: the drag outside a move
sc("future sight hits a red card holder: the user is dragged out at the end of the turn",
   [mon("Slowbro", "Oblivious", ["Future Sight", "Calm Mind"]), mon("Snorlax", "Thick Fat", ["Calm Mind"]), mon("Persian", "Technician", ["Calm Mind"])],
   [mon("Garchomp", "Rough Skin", ["Swords Dance"], "Red Card")],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")], ["|drag|p1a:"])
# RH-5: Fairy Aura and Dark Aura brought in by a Mold Breaker move are not announced
sc("dragon tail from mold breaker brings in fairy aura and dark aura holders: not announced",
   [mon("Excadrill", "Mold Breaker", ["Dragon Tail"])],
   [mon("Snorlax", "Thick Fat", ["Calm Mind"]), mon("Sylveon", "Fairy Aura", ["Calm Mind"]), mon("Umbreon", "Dark Aura", ["Calm Mind"])],
   rep("move 0", "move 0", 4), ["|drag|p2a:"])
sc("dragon tail from mold breaker brings in ruin ability holders: not announced",
   [mon("Excadrill", "Mold Breaker", ["Dragon Tail"])],
   [mon("Snorlax", "Thick Fat", ["Calm Mind"]), mon("Kingambit", "Sword of Ruin", ["Calm Mind"]), mon("Umbreon", "Beads of Ruin", ["Calm Mind"])],
   rep("move 0", "move 0", 4), ["|drag|p2a:"])

# rare ability rows (reading)
sc("magic guard against rough skin and iron barbs: the attacker's ability row only",
   [mon("Clefable", "Magic Guard", ["Facade"])], [mon("Garchomp", "Rough Skin", ["Calm Mind"]), mon("Snorlax", "Iron Barbs", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0")], ["|-activate|p2a: Mon0|ability: Rough Skin"])
sc("magic guard against aftermath", [mon("Clefable", "Magic Guard", ["Facade"])],
   [mon("Persian", "Aftermath", ["Calm Mind"]), mon("Snorlax", "Thick Fat", ["Calm Mind"])], rep("move 0", "move 0", 6), [])
sc("sturdy against an ohko move: immune row", [mon("Snorlax", "Sturdy", ["Calm Mind"])],
   [mon("Garchomp", "Rough Skin", ["Fissure", "Sheer Cold"])], [("move 0", "move 0"), ("move 0", "move 1")],
   ["|-immune|p1a: Mon0|[from] ability: Sturdy"])
sc("an ohko move against a substitute: the ohko row and the doll breaks",
   [mon("Snorlax", "Thick Fat", ["Substitute", "Calm Mind"])], [mon("Glaceon", "Snow Cloak", ["Sheer Cold"])],
   [("move 0", "move 0")] + rep("move 1", "move 0", 8), [])
sc("own tempo against confuse ray: immune row naming confusion", [mon("Slowbro", "Own Tempo", ["Calm Mind"])],
   [mon("Gengar", "Cursed Body", ["Confuse Ray"])], rep("move 0", "move 0", 2), ["|-immune|p1a: Mon0|confusion|[from] ability: Own Tempo"])
sc("oblivious against taunt and attract: immune rows", [mon("Slowbro", "Oblivious", ["Calm Mind"])],
   [dict(mon("Gengar", "Cursed Body", ["Taunt", "Attract"]), gender="F")], [("move 0", "move 0"), ("move 0", "move 1")],
   ["|-immune|p1a: Mon0|[from] ability: Oblivious"])
sc("mold breaker statuses limber, water veil, insomnia and pastel veil holders: cured at once with the ability row",
   [mon("Excadrill", "Mold Breaker", ["Thunder Wave", "Will-O-Wisp", "Spore", "Toxic"])],
   [mon("Persian", "Limber", ["Calm Mind"]), mon("Milotic", "Water Veil", ["Calm Mind"]), mon("Snorlax", "Insomnia", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "switch 1"), ("move 2", "switch 2"), ("move 0", "move 0")],
   ["|-activate|p2a: Mon0|ability: Limber", "|-activate|p2a: Mon2|ability: Insomnia"])
sc("mold breaker toxic on pastel veil and immunity holders: cured at once",
   [mon("Excadrill", "Mold Breaker", ["Toxic"])], [mon("Persian", "Pastel Veil", ["Calm Mind"]), mon("Snorlax", "Immunity", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0")],
   ["|-activate|p2a: Mon0|ability: Pastel Veil", "|-activate|p2a: Mon1|ability: Immunity"])
sc("mold breaker taunt, attract and confuse ray on oblivious and own tempo holders: cured at once",
   [mon("Excadrill", "Mold Breaker", ["Taunt", "Attract", "Confuse Ray"])],
   [dict(mon("Slowbro", "Oblivious", ["Calm Mind"]), gender="F"), mon("Slowking", "Own Tempo", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "switch 1"), ("move 2", "move 0")],
   ["|-activate|p2a: Mon0|ability: Oblivious", "|-activate|p2a: Mon1|ability: Own Tempo"])
sc("damp stops explosion: the cant row names the damp holder", [mon("Snorlax", "Thick Fat", ["Explosion", "Misty Explosion"])],
   [mon("Slowbro", "Damp", ["Calm Mind"])], [("move 0", "move 0"), ("move 1", "move 0")], ["ability: Damp"])
sc("armor tail, queenly majesty and dazzling stop priority moves",
   [mon("Persian", "Technician", ["Quick Attack", "Sucker Punch"])],
   [mon("Farigiraf", "Armor Tail", ["Calm Mind"]), mon("Tsareena", "Queenly Majesty", ["Calm Mind"]), mon("Slowking", "Dazzling", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0"), ("move 0", "switch 2"), ("move 0", "move 0")],
   ["ability: Armor Tail"])
sc("guard dog and suction cups against roar and dragon tail", [mon("Garchomp", "Rough Skin", ["Roar", "Dragon Tail"])],
   [mon("Arcanine", "Guard Dog", ["Calm Mind"]), mon("Vaporeon", "Suction Cups", ["Calm Mind"]), mon("Snorlax", "Thick Fat", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0"), ("move 1", "move 0")],
   ["|-activate|p2a: Mon0|ability: Guard Dog", "|-activate|p2a: Mon1|ability: Suction Cups"])
sc("guard dog, rattled and mirror armor against intimidate",
   [mon("Snorlax", "Thick Fat", ["Calm Mind"]), mon("Gyarados", "Intimidate", ["Calm Mind"])],
   [mon("Arcanine", "Guard Dog", ["Calm Mind"]), mon("Persian", "Rattled", ["Calm Mind"]), mon("Corviknight", "Mirror Armor", ["Calm Mind"])],
   [("switch 1", "move 0"), ("switch 0", "switch 1"), ("switch 1", "move 0"), ("switch 0", "switch 2"), ("switch 1", "move 0")],
   ["Mirror Armor", "Guard Dog"])
sc("intimidate against a substitute: immune row", [mon("Snorlax", "Thick Fat", ["Calm Mind"]), mon("Gyarados", "Intimidate", ["Calm Mind"])],
   [mon("Persian", "Technician", ["Substitute", "Calm Mind"])], [("move 0", "move 0"), ("switch 1", "move 1")], ["|-immune|p2a: Mon0"])
sc("supersweet syrup against a substitute: immune row",
   [mon("Snorlax", "Thick Fat", ["Calm Mind"]), mon("Persian", "Supersweet Syrup", ["Calm Mind"])],
   [mon("Persian", "Technician", ["Substitute", "Calm Mind"])], [("move 0", "move 0"), ("switch 1", "move 1")], ["Supersweet Syrup"])
for ab, setup, mv, typ in [("Sap Sipper", "Swords Dance", "Energy Ball", "grass"), ("Motor Drive", "Agility", "Thunderbolt", "electric"),
                           ("Lightning Rod", "Nasty Plot", "Thunderbolt", "electric"), ("Storm Drain", "Nasty Plot", "Surf", "water"),
                           ("Well-Baked Body", "Iron Defense", "Flamethrower", "fire"), ("Wind Rider", "Swords Dance", "Heat Wave", "wind")]:
    sc(f"{ab.lower()} at +6: the boost fails and the immune row is written",
       [mon("Snorlax", "Thick Fat", [mv, "Calm Mind"])], [mon("Dragonite", ab, [setup])],
       [("move 1", "move 0"), ("move 1", "move 0"), ("move 1", "move 0"), ("move 0", "move 0"), ("move 0", "move 0")],
       [f"|-immune|p2a: Mon0|[from] ability: {ab}"])
for ab, mv in [("Volt Absorb", "Thunderbolt"), ("Water Absorb", "Surf"), ("Earth Eater", "Earthquake"), ("Dry Skin", "Surf")]:
    sc(f"{ab.lower()} at full hp: immune row", [mon("Snorlax", "Thick Fat", [mv])], [mon("Dragonite", ab, ["Calm Mind"])],
       [("move 0", "move 0")], [f"|-immune|p2a: Mon0|[from] ability: {ab}"])
sc("flash fire twice: the start row, then the immune row", [mon("Snorlax", "Thick Fat", ["Flamethrower"])],
   [mon("Arcanine", "Flash Fire", ["Calm Mind"])], rep("move 0", "move 0", 2), ["|-immune|p2a: Mon0|[from] ability: Flash Fire"])
sc("perish song and teatime against a pokemon in the air: miss rows",
   [mon("Gengar", "Cursed Body", ["Perish Song", "Teatime"]), mon("Snorlax", "Thick Fat", ["Calm Mind"])],
   [mon("Charizard", "Blaze", ["Fly", "Dig"], "Sitrus Berry")], [("move 0", "move 0"), ("move 0", "move 0"), ("move 1", "move 1"), ("move 1", "move 1")],
   ["|-miss|p1a: Mon0|p2a: Mon0"])
sc("gravity grounds a flier, a levitate holder and an air balloon holder; magnet rise disabled: struggle",
   [mon("Slowbro", "Oblivious", ["Gravity", "Calm Mind"])],
   [mon("Charizard", "Blaze", ["Fly", "Magnet Rise"]), mon("Gengar", "Levitate", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 1"), ("move 1", "switch 1"), ("move 1", "move 0")],
   ["|-activate|p2a: Mon0|move: Gravity"])
sc("cotton down against clear body; gooey and tangling hair against mirror armor",
   [mon("Metagross", "Clear Body", ["Facade"]), mon("Corviknight", "Mirror Armor", ["Drill Peck"])],
   [mon("Sylveon", "Cotton Down", ["Calm Mind"]), mon("Goodra", "Gooey", ["Calm Mind"]), mon("Kingambit", "Tangling Hair", ["Calm Mind"])],
   [("move 0", "move 0"), ("switch 1", "switch 1"), ("move 0", "move 0"), ("move 0", "switch 2"), ("move 0", "move 0")],
   ["Cotton Down", "Mirror Armor"])
sc("perish body on contact, and not again when the attacker already has perish song",
   [mon("Snorlax", "Thick Fat", ["Crunch"])], [mon("Gengar", "Perish Body", ["Calm Mind"])], rep("move 0", "move 0", 3), ["Perish Body"])
sc("toxic debris with two layers already: no row", [mon("Snorlax", "Thick Fat", ["Facade"])],
   [mon("Glimmora", "Toxic Debris", ["Calm Mind"])], rep("move 0", "move 0", 4), ["ability: Toxic Debris"])
sc("hydration in rain and shed skin cure at the end of the turn",
   [mon("Snorlax", "Thick Fat", ["Rain Dance", "Toxic"])], [mon("Vaporeon", "Hydration", ["Calm Mind"]), mon("Dragonite", "Shed Skin", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 1", "switch 1"), ("move 1", "move 0"), ("move 1", "move 0"), ("move 1", "move 0")],
   ["|-activate|p2a: Mon0|ability: Hydration"])
sc("comatose: sleep talk and snore work, rest fails, hex doubles",
   [mon("Snorlax", "Comatose", ["Sleep Talk", "Snore", "Rest", "Facade"])], [mon("Gengar", "Cursed Body", ["Hex"])],
   [("move 0", "move 0"), ("move 1", "move 0"), ("move 2", "move 0")], ["Comatose"])
sc("forewarn, anticipation and frisk on entry", [mon("Slowbro", "Forewarn", ["Calm Mind"]), mon("Persian", "Anticipation", ["Calm Mind"])],
   [mon("Gengar", "Frisk", ["Shadow Ball", "Sheer Cold"], "Leftovers")], [("switch 1", "move 0"), ("move 0", "move 0")], ["Forewarn"])
sc("disguise busted, then the forme row and its damage", [mon("Snorlax", "Thick Fat", ["Crunch"])],
   [mon("Mimikyu", "Disguise", ["Calm Mind"])], rep("move 0", "move 0", 3), ["ability: Disguise"])
sc("zero to hero: the switch back in shows hero form", [mon("Snorlax", "Thick Fat", ["Calm Mind"])],
   [mon("Palafin", "Zero to Hero", ["Calm Mind"]), mon("Persian", "Technician", ["Calm Mind"])],
   [("move 0", "switch 1"), ("move 0", "switch 0"), ("move 0", "move 0")], ["Zero to Hero"])
sc("hunger switch changes form each turn", [mon("Snorlax", "Thick Fat", ["Calm Mind"])], [mon("Morpeko", "Hunger Switch", ["Aura Wheel"])],
   rep("move 0", "move 0", 3), [])
sc("stance change and king's shield", [mon("Snorlax", "Thick Fat", ["Facade"])],
   [mon("Aegislash", "Stance Change", ["King's Shield", "Shadow Ball"])], [("move 0", "move 1"), ("move 0", "move 0"), ("move 0", "move 1")],
   ["Aegislash-Blade"])
sc("battle bond after a knock out", [mon("Persian", "Technician", ["Calm Mind"]), mon("Snorlax", "Thick Fat", ["Calm Mind"])],
   [mon("Greninja", "Battle Bond", ["Hydro Pump"])], rep("move 0", "move 0", 6), [])
sc("truant loafs every other turn", [mon("Snorlax", "Thick Fat", ["Calm Mind"])], [mon("Persian", "Truant", ["Facade"])],
   rep("move 0", "move 0", 3), ["ability: Truant"])
sc("imposter transforms on entry", [mon("Snorlax", "Thick Fat", ["Calm Mind"])], [mon("Ditto", "Imposter", ["Transform"])],
   rep("move 0", "move 0", 2), ["-transform"])
sc("wandering spirit and mummy and lingering aroma swap on contact",
   [mon("Snorlax", "Thick Fat", ["Facade"])], [mon("Runerigus", "Wandering Spirit", ["Calm Mind"]), mon("Cofagrigus", "Mummy", ["Calm Mind"]), mon("Garchomp", "Lingering Aroma", ["Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "switch 1"), ("move 0", "move 0"), ("move 0", "switch 2"), ("move 0", "move 0")], [])
# RH-6: Steel Beam's cost on failure is skipped when Parental Bond made it a multi-hit move
sc("parental bond steel beam into protect: no cost; a plain user pays half its hp",
   [mon("Kangaskhan", "Parental Bond", ["Steel Beam"]), mon("Snorlax", "Thick Fat", ["Steel Beam"])],
   [mon("Slowbro", "Oblivious", ["Protect", "Calm Mind"])],
   [("move 0", "move 0"), ("move 0", "move 1"), ("switch 1", "move 1"), ("move 0", "move 0")], ["|-activate|p2a: Mon0|move: Protect"])
