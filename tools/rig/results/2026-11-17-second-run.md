# Rig run: second-run (2026-11-17)

Showdown (oracle) against the engine (rig), random legal choices drawn from Showdown's options. At each decision Showdown's state is loaded into the engine, so differences never carry over. Produced by `tools/rig/report.py` from `tools/rig/driver.py` runs; raw lines in `2026-11-17-second-run.jsonl.gz` (every non-exact step, with teams, picks, seed, states, choices, events and Showdown's log).

Runs: seed 1 battles 0-499. Time: 20 s in all.

## Totals

| | count | share of steps |
|---|---:|---:|
| battles | 500 | |
| steps (decisions played) | 9848 | |
| exact | 2352 | 23.9% |
| damage_only | 238 | 2.4% |
| mismatch | 6911 | 70.2% |
| skipped | 345 | 3.5% |
| engine_error | 2 | 0.0% |
| oracle errors | 0 | |
| requests timed out (10 s; battle given up, child restarted) | 0 | |
| child restarts (oracle, engine) | 0, 0 | |
| battles cut off at 300 decisions | 0 | |

Skipped: midturn 315, dropped_moves 30 (midturn: a decision in the middle of a turn after a pivot or forced switch, which the engine cannot load; dropped_moves: the active Pokémon has moves the engine cannot hold, i.e. a transformed Ditto).

Legal-action check (options of both sides, every field): 9484 passed, 19 failed, out of 9503 loaded decisions (30 not checked: moves dropped).

Load check (snapshot loaded into the engine and exported again, dropped fields aside): 0 decisions came back different.

### With the PP shortcut set aside

The engine spends PP only when a move has fewer than 10 left (a deliberate shortcut in `engine/src/genx/generate_instructions.rs`). That alone makes most steps a mismatch. Counting a PP difference where the engine kept 10 or more PP as no difference:

| | count | share of steps |
|---|---:|---:|
| exact | 4795 | 48.7% |
| damage_only | 2916 | 29.6% |
| mismatch | 1790 | 18.2% |

The sections below use this view: "mismatch" means a mismatch other than the PP shortcut.

## Damage-only steps

2916 steps differ only in HP (and what follows from a faint): 3365 HP values in all. HP error = |engine HP - Showdown HP| / max HP, per Pokémon. The engine uses one average damage roll per branch; Showdown draws one of 16.

| HP error | values | share |
|---|---:|---:|
| up to 1% | 1343 | 39.9% |
| 1-2% | 939 | 27.9% |
| 2-5% | 792 | 23.5% |
| 5-10% | 175 | 5.2% |
| 10-20% | 62 | 1.8% |
| 20-50% | 41 | 1.2% |
| over 50% | 13 | 0.4% |

Median 1.27%, mean 2.66% of max HP.

Within Showdown's own roll range: of the HP values that changed by one hit with one roll and nothing else, 1324 engine values lie within the 85-100% range of that hit (1 HP tolerance) and 59 outside; 1982 could not be told (several hits, other HP changes, a faint, a switch).

## Mismatches by first differing field

First difference that is not HP or a consequence of a faint, side and slot stripped. 6911 mismatch steps; 1790 apart from the PP shortcut.

| field | steps | share of steps | examples |
|---|---:|---:|---|
| `moves.pp: PP not spent (engine spends PP only below 10)` | 5121 | 52.0% | 1:0:4, 1:0:5, 1:0:6 |
| `side.pokemon.moves.disabled` | 205 | 2.1% | 1:10:9, 1:14:8, 1:14:12 |
| `side.pokemon.lastMove` | 184 | 1.9% | 1:5:14, 1:5:17, 1:5:18 |
| `side.pokemon.ability` | 159 | 1.6% | 1:3:0, 1:6:2, 1:10:0 |
| `side.pokemon.moves.pp` | 113 | 1.1% | 1:3:14, 1:3:18, 1:3:19 |
| `weather.left` | 107 | 1.1% | 1:2:4, 1:2:11, 1:11:20 |
| `side.pokemon.hp` | 85 | 0.9% | 1:3:31, 1:13:10, 1:17:13 |
| `side.pokemon.types` | 84 | 0.9% | 1:9:4, 1:13:2, 1:22:0 |
| `side.pokemon.volatiles.sparklingaria` | 83 | 0.8% | 1:1:0, 1:13:3, 1:24:3 |
| `side.pokemon.species` | 63 | 0.6% | 1:0:0, 1:4:1, 1:6:5 |
| `side.pokemon.boosts.atk` | 59 | 0.6% | 1:40:5, 1:40:8, 1:40:9 |
| `terrain.left` | 57 | 0.6% | 1:3:11, 1:9:13, 1:23:18 |
| `side.pokemon.item` | 47 | 0.5% | 1:18:5, 1:29:9, 1:76:33 |
| `side.pokemon.boosts.spe` | 45 | 0.5% | 1:8:6, 1:15:9, 1:25:9 |
| `side.pokemon.volatiles.lockedmove.left` | 45 | 0.5% | 1:10:7, 1:18:3, 1:26:12 |
| `turn` | 37 | 0.4% | 1:9:2, 1:12:8, 1:13:0 |
| `side.pokemon.statusData.turns` | 36 | 0.4% | 1:3:27, 1:93:1, 1:98:8 |
| `side.pokemon.status` | 30 | 0.3% | 1:27:5, 1:34:20, 1:34:27 |
| `side.pokemon.volatiles.curse` | 28 | 0.3% | 1:9:20, 1:9:22, 1:9:25 |
| `side.conditions.reflect` | 27 | 0.3% | 1:15:0, 1:15:21, 1:19:13 |
| `side.pokemon.volatiles.charge` | 26 | 0.3% | 1:41:8, 1:41:9, 1:41:16 |
| `side.conditions.lightscreen` | 23 | 0.2% | 1:20:36, 1:61:3, 1:111:8 |
| `side.pokemon.boosts.spa` | 22 | 0.2% | 1:40:0, 1:40:3, 1:40:13 |
| `side.pokemon.boosts.def` | 21 | 0.2% | 1:21:10, 1:40:4, 1:40:15 |
| `side.pokemon.volatiles.lockedmove` | 19 | 0.2% | 1:110:6, 1:124:7, 1:128:9 |
| `side.pokemon.volatiles.stall` | 16 | 0.2% | 1:8:0, 1:85:5, 1:85:8 |
| `side.conditions.auroraveil` | 15 | 0.2% | 1:11:2, 1:34:0, 1:110:0 |
| `side.pokemon.stats.atk` | 13 | 0.1% | 1:29:5, 1:68:9, 1:73:5 |
| `side.pokemon.volatiles.glaiverush` | 13 | 0.1% | 1:206:10, 1:237:6, 1:270:37 |
| `terrain.id` | 13 | 0.1% | 1:16:17, 1:99:23, 1:107:15 |
| `side.pokemon.volatiles.yawn` | 12 | 0.1% | 1:3:26, 1:149:0, 1:159:5 |
| `weather.id` | 12 | 0.1% | 1:31:29, 1:57:15, 1:133:14 |
| `side.pokemon.boosts.evasion` | 10 | 0.1% | 1:190:1, 1:277:1, 1:354:22 |
| `side.pokemon.boosts.spd` | 9 | 0.1% | 1:40:6, 1:40:10, 1:40:11 |
| `side.conditions.healingwish` | 8 | 0.1% | 1:9:18, 1:9:19, 1:9:21 |
| `side.pokemon.volatiles.kingsshield` | 8 | 0.1% | 1:85:9, 1:155:11, 1:155:21 |
| `side.pokemon.volatiles.taunt.left` | 7 | 0.1% | 1:163:12, 1:163:19, 1:256:17 |
| `side.pokemon.volatiles.throatchop` | 6 | 0.1% | 1:78:3, 1:83:20, 1:83:28 |
| `side.pokemon.volatiles.confusion` | 5 | 0.1% | 1:131:7, 1:357:10, 1:433:6 |
| `side.pokemon.volatiles.healblock` | 5 | 0.1% | 1:64:5, 1:107:6, 1:245:2 |
| `rooms.trickroom` | 4 | 0.0% | 1:33:14, 1:193:13, 1:243:18 |
| `side.pokemon.volatiles.smackdown` | 4 | 0.0% | 1:203:7, 1:203:14, 1:203:19 |
| `side.pokemon.volatiles.substitute` | 4 | 0.0% | 1:5:21, 1:46:3, 1:202:38 |
| `side.pokemon.volatiles.unburden` | 3 | 0.0% | 1:121:0, 1:131:12, 1:243:7 |
| `winner` | 3 | 0.0% | 1:341:12, 1:415:17, 1:431:16 |
| `side.pokemon.volatiles.encore.left` | 2 | 0.0% | 1:31:23, 1:181:10 |
| `side.pokemon.volatiles.partiallytrapped` | 2 | 0.0% | 1:65:20, 1:349:19 |
| `side.wish` | 2 | 0.0% | 1:74:22, 1:199:23 |
| `side.conditions.revivalblessing` | 1 | 0.0% | 1:463:6 |
| `side.conditions.spikes` | 1 | 0.0% | 1:384:5 |
| `side.conditions.tailwind` | 1 | 0.0% | 1:180:12 |
| `side.pokemon.stats.spd` | 1 | 0.0% | 1:69:10 |
| `side.pokemon.statusData.stage` | 1 | 0.0% | 1:373:9 |
| `side.pokemon.volatiles.counter` | 1 | 0.0% | 1:396:3 |
| `side.pokemon.volatiles.disable` | 1 | 0.0% | 1:320:6 |
| `side.pokemon.volatiles.substitute.hp` | 1 | 0.0% | 1:121:3 |
| `side.pokemon.volatiles.trapped` | 1 | 0.0% | 1:122:7 |

## Legal-action failures

| field: values | decisions | examples |
|---|---:|---|
| moves: Showdown ["struggle"], engine [] | 16 | 1:100:1, 1:100:2, 1:168:14 |
| mega: Showdown false, engine true | 3 | 1:155:7, 1:155:8, 1:258:1 |

## Mismatches and legal-action failures by entity in play

Entities in play in a step: the moves used (Showdown's log) and the abilities and items of the two active Pokémon at the start of the step. A step counts once for each of its entities, so an entity here was present, not necessarily the cause. "in play" is the number of compared steps it was in; "rate" = (mismatch + legal) / in play. "gaps/INDEX.md" says whether the entity's English name appears anywhere in the register (a Mega stone also by its Mega's name).

| entity | kind | pool_share | mismatch steps | legal-action failures | in play | rate | gaps/INDEX.md |
|---|---|---:|---:|---:|---:|---:|---|
| Leftovers | item | 19.1% | 692 | 13 | 3342 | 21.1% | listed |
| Torrent | ability | 5.4% | 278 | 1 | 1091 | 25.6% | NOT LISTED |
| Sand Stream | ability | 4.7% | 234 | 8 | 1095 | 22.1% | listed |
| Life Orb | item | 12.4% | 239 | 1 | 1552 | 15.5% | listed |
| Rough Skin | ability | 8.1% | 204 | 2 | 985 | 20.9% | listed |
| Rocky Helmet | item | 4.6% | 201 | 3 | 1292 | 15.8% | listed |
| Garchompite Z | item | 4.5% | 173 | 2 | 843 | 20.8% | listed (as Mega Garchomp Z) |
| Sitrus Berry | item | 4.8% | 172 | 1 | 727 | 23.8% | listed |
| Golisopite | item | 3.0% | 164 | 2 | 562 | 29.5% | listed (as Mega Golisopod) |
| Levitate | ability | 7.9% | 140 | 2 | 1182 | 12.0% | listed |
| Focus Sash | item | 5.9% | 140 | 0 | 845 | 16.6% | listed |
| Choice Scarf | item | 5.5% | 133 | 5 | 970 | 14.2% | listed |
| Stamina | ability | 3.3% | 133 | 2 | 688 | 19.6% | listed |
| Protean | ability | 4.5% | 126 | 0 | 618 | 20.4% | listed |
| Emergency Exit | ability | 3.0% | 112 | 2 | 241 | 47.3% | listed |
| Disguise | ability | 3.0% | 108 | 0 | 563 | 19.2% | listed |
| Tough Claws | ability | 6.6% | 107 | 0 | 684 | 15.6% | listed |
| Outrage | move | 3.7% | 99 | 3 | 130 | 78.5% | listed |
| Sparkling Aria | move | 3.8% | 98 | 0 | 154 | 63.6% | listed |
| Pressure | ability | 2.1% | 87 | 8 | 749 | 12.7% | listed |
| Light Clay | item | 1.8% | 93 | 1 | 358 | 26.3% | listed |
| Multiscale | ability | 3.9% | 90 | 2 | 576 | 16.0% | listed |
| Intimidate | ability | 5.4% | 90 | 0 | 557 | 16.2% | listed |
| Dragoninite | item | 1.9% | 80 | 2 | 535 | 15.3% | NOT LISTED |
| Pixilate | ability | 0.8% | 77 | 2 | 235 | 33.6% | NOT LISTED |
| Earthquake | move | 16.0% | 75 | 0 | 474 | 15.8% | listed |
| Stance Change | ability | 0.9% | 72 | 0 | 134 | 53.7% | listed |
| Lucarionite Z | item | 2.3% | 71 | 0 | 344 | 20.6% | listed (as Mega Lucario Z) |
| Stealth Rock | move | 10.3% | 70 | 0 | 385 | 18.2% | listed |
| Prankster | ability | 1.3% | 65 | 0 | 276 | 23.6% | listed |
| Swords Dance | move | 11.2% | 65 | 0 | 313 | 20.8% | listed |
| Moody | ability | 0.2% | 64 | 0 | 82 | 78.0% | listed |
| Sucker Punch | move | 7.6% | 64 | 0 | 237 | 27.0% | listed |
| Flamethrower | move | 9.5% | 56 | 0 | 344 | 16.3% | NOT LISTED |
| Spell Tag | item | 0.1% | 55 | 0 | 108 | 50.9% | NOT LISTED |
| Protect | move | 5.5% | 55 | 0 | 238 | 23.1% | listed |
| Good as Gold | ability | 2.5% | 54 | 0 | 524 | 10.3% | listed |
| Salamencite | item | 2.4% | 54 | 0 | 411 | 13.1% | NOT LISTED |
| Mold Breaker | ability | 1.6% | 52 | 0 | 262 | 19.8% | listed |
| Regenerator | ability | 1.9% | 52 | 0 | 297 | 17.5% | NOT LISTED |
| Snow Warning | ability | 1.6% | 51 | 1 | 245 | 21.2% | listed |
| Yawn | move | 5.1% | 52 | 0 | 218 | 23.9% | listed |
| Grassy Surge | ability | 2.1% | 51 | 0 | 264 | 19.3% | NOT LISTED |
| Unburden | ability | 1.0% | 51 | 0 | 158 | 32.3% | listed |
| Metagrossite | item | 1.7% | 50 | 0 | 317 | 15.8% | NOT LISTED |
| Charizardite Y | item | 1.7% | 48 | 1 | 299 | 16.4% | NOT LISTED |
| Electromorphosis | ability | 0.5% | 47 | 1 | 114 | 42.1% | listed |
| Cursed Body | ability | 1.6% | 45 | 1 | 285 | 16.1% | listed |
| Normal Gem | item | 0.7% | 46 | 0 | 127 | 36.2% | listed |
| Curse | move | 1.5% | 46 | 0 | 49 | 93.9% | listed |
| Blaze | ability | 3.4% | 43 | 2 | 235 | 19.1% | listed |
| Flame Body | ability | 1.4% | 43 | 1 | 327 | 13.5% | listed |
| Speed Boost | ability | 3.1% | 44 | 0 | 327 | 13.5% | listed |
| Adaptability | ability | 3.5% | 42 | 1 | 364 | 11.8% | NOT LISTED |
| Drizzle | ability | 1.0% | 43 | 0 | 276 | 15.6% | NOT LISTED |
| Greninjite | item | 0.8% | 43 | 0 | 189 | 22.8% | NOT LISTED |
| Gyaradosite | item | 0.7% | 42 | 0 | 249 | 16.9% | NOT LISTED |
| Gardevoirite | item | 0.3% | 41 | 0 | 139 | 29.5% | NOT LISTED |
| Aerilate | ability | 2.5% | 40 | 0 | 251 | 15.9% | NOT LISTED |
| Raichunite Y | item | 1.1% | 40 | 0 | 135 | 29.6% | NOT LISTED |
| Close Combat | move | 6.2% | 39 | 0 | 147 | 26.5% | NOT LISTED |
| Roost | move | 6.7% | 39 | 0 | 283 | 13.8% | listed |
| Thunderbolt | move | 5.8% | 37 | 2 | 181 | 21.5% | NOT LISTED |
| Sturdy | ability | 0.5% | 38 | 0 | 183 | 20.8% | listed |
| Technician | ability | 1.1% | 38 | 0 | 150 | 25.3% | listed |
| Draco Meteor | move | 5.9% | 37 | 0 | 171 | 21.6% | NOT LISTED |
| Iron Head | move | 4.8% | 37 | 0 | 137 | 27.0% | NOT LISTED |
| Inner Focus | ability | 1.7% | 36 | 0 | 266 | 13.5% | NOT LISTED |
| U-turn | move | 6.6% | 32 | 4 | 222 | 16.2% | listed |
| Thermal Exchange | ability | 2.5% | 35 | 0 | 216 | 16.2% | listed |
| Blazikenite | item | 1.3% | 34 | 0 | 220 | 15.5% | NOT LISTED |
| Swampertite | item | 0.7% | 34 | 0 | 218 | 15.6% | NOT LISTED |
| Dark Pulse | move | 5.2% | 34 | 0 | 131 | 26.0% | NOT LISTED |
| Slack Off | move | 3.6% | 34 | 0 | 173 | 19.7% | NOT LISTED |
| Justified | ability | 1.9% | 32 | 0 | 109 | 29.4% | listed |
| Magic Bounce | ability | 1.4% | 29 | 2 | 314 | 9.9% | listed |
| Psychic Surge | ability | 1.5% | 31 | 0 | 203 | 15.3% | NOT LISTED |
| Clefablite | item | 0.8% | 31 | 0 | 342 | 9.1% | NOT LISTED |
| Baxcalibrite | item | 1.1% | 30 | 0 | 187 | 16.0% | NOT LISTED |
| Charizardite X | item | 1.4% | 29 | 1 | 204 | 14.7% | NOT LISTED |
| Slowbronite | item | 0.2% | 30 | 0 | 146 | 20.5% | NOT LISTED |
| Shadow Sneak | move | 4.6% | 30 | 0 | 140 | 21.4% | NOT LISTED |
| Dragon Pulse | move | 4.3% | 29 | 0 | 130 | 22.3% | NOT LISTED |
| King's Shield | move | 0.9% | 29 | 0 | 33 | 87.9% | listed |
| Aura Guard | ability | 2.3% | 28 | 0 | 209 | 13.4% | listed |
| Unaware | ability | 1.4% | 28 | 0 | 340 | 8.2% | NOT LISTED |
| Damp Rock | item | 0.6% | 28 | 0 | 165 | 17.0% | listed |
| Supreme Overlord | ability | 1.1% | 27 | 0 | 207 | 13.0% | listed |
| Terrain Extender | item | 1.3% | 27 | 0 | 165 | 16.4% | listed |
| Leech Life | move | 2.4% | 27 | 0 | 60 | 45.0% | listed |
| Earth Eater | ability | 0.1% | 26 | 0 | 63 | 41.3% | listed |
| Iron Fist | ability | 1.0% | 26 | 0 | 144 | 18.1% | listed |
| Rest | move | 1.2% | 26 | 0 | 79 | 32.9% | listed |
| Swift Swim | ability | 0.7% | 25 | 0 | 167 | 15.0% | listed |
| Encore | move | 7.8% | 25 | 0 | 212 | 11.8% | listed |
| Flower Trick | move | 2.4% | 25 | 0 | 60 | 41.7% | NOT LISTED |
| Moonblast | move | 5.6% | 25 | 0 | 206 | 12.1% | NOT LISTED |
| Static | ability | 1.1% | 24 | 0 | 54 | 44.4% | listed |
| Smooth Rock | item | 0.3% | 24 | 0 | 144 | 16.7% | listed |
| Knock Off | move | 3.1% | 24 | 0 | 82 | 29.3% | listed |
| Reflect | move | 1.2% | 24 | 0 | 40 | 60.0% | listed |
| Thick Fat | ability | 0.8% | 23 | 0 | 155 | 14.8% | NOT LISTED |
| Water Absorb | ability | 0.1% | 23 | 0 | 91 | 25.3% | listed |
| Staraptite | item | 1.1% | 23 | 0 | 176 | 13.1% | NOT LISTED |
| Starminite | item | 0.7% | 23 | 0 | 95 | 24.2% | listed (as Mega Starmie) |
| Leech Seed | move | 0.8% | 23 | 0 | 45 | 51.1% | listed |
| Play Rough | move | 4.2% | 23 | 0 | 117 | 19.7% | NOT LISTED |
| Substitute | move | 2.4% | 23 | 0 | 79 | 29.1% | listed |
| Shell Armor | ability | 0.2% | 22 | 0 | 105 | 21.0% | listed |
| Black Glasses | item | 0.9% | 22 | 0 | 191 | 11.5% | NOT LISTED |
| Shuca Berry | item | 0.7% | 22 | 0 | 187 | 11.8% | listed |
| Flip Turn | move | 3.1% | 22 | 0 | 81 | 27.2% | NOT LISTED |
| Light Screen | move | 1.4% | 22 | 0 | 46 | 47.8% | listed |
| Struggle | move | 0.0% | 8 | 14 | 14 | 157.1% | listed |
| Huge Power | ability | 1.2% | 21 | 0 | 194 | 10.8% | NOT LISTED |
| Weak Armor | ability | 0.9% | 21 | 0 | 129 | 16.3% | listed |
| Aqua Jet | move | 4.6% | 21 | 0 | 127 | 16.5% | NOT LISTED |
| Fake Out | move | 2.7% | 21 | 0 | 21 | 100.0% | listed |
| Contrary | ability | 1.2% | 20 | 0 | 97 | 20.6% | listed |
| First Impression | move | 2.1% | 20 | 0 | 22 | 90.9% | listed |
| Giga Drain | move | 1.9% | 20 | 0 | 80 | 25.0% | NOT LISTED |
| Aura Sphere | move | 4.0% | 19 | 0 | 106 | 17.9% | listed |
| Ice Punch | move | 3.2% | 19 | 0 | 81 | 23.5% | NOT LISTED |
| Drought | ability | 1.8% | 18 | 0 | 146 | 12.3% | listed |
| Libero | ability | 1.2% | 18 | 0 | 152 | 11.8% | listed |
| Nasty Plot | move | 3.4% | 18 | 0 | 108 | 16.7% | NOT LISTED |
| Surf | move | 2.9% | 18 | 0 | 72 | 25.0% | listed |
| Thrash | move | 0.2% | 18 | 0 | 22 | 81.8% | listed |
| Will-O-Wisp | move | 4.4% | 18 | 0 | 224 | 8.0% | listed |
| No Guard | ability | 1.2% | 17 | 0 | 96 | 17.7% | NOT LISTED |
| Sharpness | ability | 1.7% | 17 | 0 | 201 | 8.5% | listed |
| Flash Cannon | move | 3.7% | 17 | 0 | 127 | 13.4% | NOT LISTED |
| Iron Defense | move | 2.4% | 16 | 1 | 156 | 10.9% | NOT LISTED |
| Clear Body | ability | 1.7% | 16 | 0 | 110 | 14.5% | listed |
| Calm Mind | move | 2.5% | 16 | 0 | 105 | 15.2% | NOT LISTED |
| Hyper Voice | move | 1.0% | 16 | 0 | 43 | 37.2% | NOT LISTED |
| Wish | move | 1.1% | 16 | 0 | 68 | 23.5% | listed |
| Gooey | ability | 0.5% | 15 | 0 | 121 | 12.4% | listed |
| Air Balloon | item | 1.2% | 15 | 0 | 126 | 11.9% | NOT LISTED |
| Aurora Veil | move | 1.2% | 15 | 0 | 32 | 46.9% | listed |
| Body Press | move | 2.4% | 15 | 0 | 150 | 10.0% | listed |
| Ice Beam | move | 4.1% | 15 | 0 | 107 | 14.0% | listed |
| Shadow Ball | move | 5.6% | 15 | 0 | 182 | 8.2% | NOT LISTED |
| Solar Beam | move | 2.0% | 15 | 0 | 66 | 22.7% | listed |
| Wave Crash | move | 2.8% | 15 | 0 | 86 | 17.4% | listed |
| Steadfast | ability | 0.7% | 14 | 0 | 36 | 38.9% | listed |
| Lopunnite | item | 0.5% | 14 | 0 | 57 | 24.6% | NOT LISTED |
| Earth Power | move | 3.2% | 14 | 0 | 97 | 14.4% | NOT LISTED |
| Flare Blitz | move | 2.5% | 14 | 0 | 67 | 20.9% | listed |
| Parabolic Charge | move | 0.4% | 14 | 0 | 23 | 60.9% | listed |
| Phantom Force | move | 1.3% | 14 | 0 | 63 | 22.2% | listed |
| Poltergeist | move | 1.7% | 14 | 0 | 44 | 31.8% | NOT LISTED |
| Soak | move | 0.5% | 14 | 0 | 16 | 87.5% | listed |
| Competitive | ability | 0.3% | 13 | 0 | 104 | 12.5% | listed |
| Illusion | ability | 0.4% | 13 | 0 | 49 | 26.5% | listed |
| Drum Beating | move | 1.7% | 13 | 0 | 36 | 36.1% | NOT LISTED |
| Grass Knot | move | 1.1% | 13 | 0 | 37 | 35.1% | listed |
| Hydro Pump | move | 1.7% | 13 | 0 | 86 | 15.1% | NOT LISTED |
| Make It Rain | move | 2.5% | 13 | 0 | 81 | 16.0% | listed |
| Toxic | move | 1.8% | 13 | 0 | 61 | 21.3% | listed |
| Vacuum Wave | move | 1.3% | 13 | 0 | 46 | 28.3% | NOT LISTED |
| Natural Cure | ability | 0.3% | 12 | 0 | 19 | 63.2% | NOT LISTED |
| Toxic Debris | ability | 1.7% | 12 | 0 | 133 | 9.0% | NOT LISTED |
| Banettite | item | 0.3% | 12 | 0 | 100 | 12.0% | NOT LISTED |
| Glimmoranite | item | 1.0% | 12 | 0 | 103 | 11.7% | NOT LISTED |
| Never-Melt Ice | item | 0.5% | 12 | 0 | 57 | 21.1% | NOT LISTED |
| Electro Shot | move | 1.4% | 11 | 1 | 54 | 22.2% | NOT LISTED |
| Glaive Rush | move | 1.5% | 12 | 0 | 22 | 54.5% | listed |
| Recover | move | 2.0% | 12 | 0 | 67 | 17.9% | listed |
| Shadow Claw | move | 1.8% | 12 | 0 | 55 | 21.8% | NOT LISTED |
| Gengarite | item | 0.5% | 11 | 0 | 59 | 18.6% | NOT LISTED |
| Bullet Punch | move | 3.5% | 11 | 0 | 96 | 11.5% | NOT LISTED |
| Dire Claw | move | 0.9% | 11 | 0 | 24 | 45.8% | listed |
| Power Gem | move | 3.1% | 11 | 0 | 68 | 16.2% | NOT LISTED |
| Scald | move | 0.9% | 11 | 0 | 53 | 20.8% | listed |
| Sludge Wave | move | 3.6% | 11 | 0 | 77 | 14.3% | NOT LISTED |
| Thunder Punch | move | 3.3% | 11 | 0 | 83 | 13.3% | NOT LISTED |
| Defiant | ability | 0.3% | 10 | 0 | 53 | 18.9% | listed |
| Quick Claw | item | 0.8% | 10 | 0 | 101 | 9.9% | listed |
| Shell Bell | item | 0.1% | 10 | 0 | 47 | 21.3% | listed |
| Tyranitarite | item | 0.6% | 10 | 0 | 68 | 14.7% | NOT LISTED |
| Dragon Tail | move | 1.9% | 10 | 0 | 59 | 16.9% | listed |
| Foul Play | move | 1.6% | 10 | 0 | 73 | 13.7% | listed |
| Healing Wish | move | 1.4% | 10 | 0 | 33 | 30.3% | listed |
| Throat Chop | move | 0.2% | 10 | 0 | 20 | 50.0% | listed |
| Toxic Spikes | move | 1.3% | 10 | 0 | 45 | 22.2% | listed |
| Volt Switch | move | 2.4% | 10 | 0 | 110 | 9.1% | listed |
| Telepathy | ability | 0.1% | 9 | 0 | 28 | 32.1% | NOT LISTED |
| Pecha Berry | item | 0.1% | 9 | 0 | 28 | 32.1% | NOT LISTED |
| Venusaurite | item | 0.4% | 9 | 0 | 108 | 8.3% | NOT LISTED |
| Mystical Fire | move | 2.3% | 9 | 0 | 48 | 18.8% | NOT LISTED |
| Psychic Noise | move | 0.9% | 9 | 0 | 32 | 28.1% | listed |
| Rock Tomb | move | 3.0% | 9 | 0 | 89 | 10.1% | NOT LISTED |
| Taunt | move | 1.0% | 9 | 0 | 24 | 37.5% | listed |
| Mega Launcher | ability | 1.1% | 8 | 0 | 135 | 5.9% | NOT LISTED |
| Blastoisinite | item | 0.5% | 8 | 0 | 133 | 6.0% | NOT LISTED |
| Mawilite | item | 0.2% | 8 | 0 | 84 | 9.5% | NOT LISTED |
| Destiny Bond | move | 1.4% | 8 | 0 | 52 | 15.4% | NOT LISTED |
| Fire Blast | move | 1.8% | 8 | 0 | 39 | 20.5% | NOT LISTED |
| Focus Blast | move | 1.2% | 8 | 0 | 22 | 36.4% | NOT LISTED |
| Freeze-Dry | move | 1.3% | 7 | 1 | 29 | 27.6% | listed |
| Grassy Glide | move | 1.9% | 8 | 0 | 31 | 25.8% | listed |
| High Horsepower | move | 1.9% | 8 | 0 | 34 | 23.5% | NOT LISTED |
| Minimize | move | 0.1% | 8 | 0 | 12 | 66.7% | listed |
| Moonlight | move | 1.2% | 8 | 0 | 79 | 10.1% | listed |
| Pain Split | move | 1.4% | 8 | 0 | 57 | 14.0% | listed |
| Revival Blessing | move | 0.9% | 8 | 0 | 13 | 61.5% | listed |
| Steel Beam | move | 2.2% | 7 | 1 | 36 | 22.2% | listed |
| Superpower | move | 0.7% | 8 | 0 | 23 | 34.8% | NOT LISTED |
| Cute Charm | ability | 0.5% | 7 | 0 | 21 | 33.3% | listed |
| Scrappy | ability | 0.6% | 7 | 0 | 58 | 12.1% | NOT LISTED |
| Shadow Tag | ability | 0.5% | 7 | 0 | 27 | 25.9% | NOT LISTED |
| Heracronite | item | 0.2% | 7 | 0 | 36 | 19.4% | NOT LISTED |
| Dragon Dance | move | 5.1% | 7 | 0 | 128 | 5.5% | NOT LISTED |
| Ice Shard | move | 1.8% | 7 | 0 | 45 | 15.6% | NOT LISTED |
| Shed Tail | move | 0.1% | 7 | 0 | 8 | 87.5% | listed |
| Triple Axel | move | 1.7% | 7 | 0 | 36 | 19.4% | listed |
| Waterfall | move | 0.6% | 7 | 0 | 32 | 21.9% | NOT LISTED |
| Illuminate | ability | 0.5% | 6 | 0 | 21 | 28.6% | listed |
| Limber | ability | 0.4% | 6 | 0 | 14 | 42.9% | listed |
| Skill Link | ability | 0.3% | 6 | 0 | 34 | 17.6% | NOT LISTED |
| Absolite Z | item | 0.3% | 6 | 0 | 23 | 26.1% | listed (as Mega Absol Z) |
| Eject Button | item | 0.1% | 6 | 0 | 17 | 35.3% | listed |
| Falinksite | item | 0.0% | 6 | 0 | 20 | 30.0% | NOT LISTED |
| Lucarionite | item | 0.4% | 6 | 0 | 62 | 9.7% | listed (as Mega Lucario) |
| Scizorite | item | 0.3% | 6 | 0 | 47 | 12.8% | NOT LISTED |
| Brave Bird | move | 1.5% | 6 | 0 | 32 | 18.8% | NOT LISTED |
| Crunch | move | 0.8% | 6 | 0 | 48 | 12.5% | NOT LISTED |
| Expanding Force | move | 1.7% | 6 | 0 | 33 | 18.2% | listed |
| Ice Spinner | move | 1.0% | 5 | 1 | 34 | 17.6% | NOT LISTED |
| Roar | move | 1.0% | 6 | 0 | 56 | 10.7% | listed |
| Sludge Bomb | move | 1.5% | 6 | 0 | 33 | 18.2% | NOT LISTED |
| Trick | move | 0.4% | 6 | 0 | 16 | 37.5% | listed |
| Zap Cannon | move | 1.1% | 6 | 0 | 21 | 28.6% | NOT LISTED |
| Frisk | ability | 0.4% | 5 | 0 | 35 | 14.3% | listed |
| Rock Head | ability | 0.2% | 5 | 0 | 54 | 9.3% | NOT LISTED |
| Excadrite | item | 0.3% | 5 | 0 | 60 | 8.3% | NOT LISTED |
| Fairy Feather | item | 0.1% | 5 | 0 | 24 | 20.8% | NOT LISTED |
| Scope Lens | item | 0.1% | 5 | 0 | 46 | 10.9% | listed |
| Wide Lens | item | 0.0% | 5 | 0 | 34 | 14.7% | listed |
| Detect | move | 0.8% | 5 | 0 | 28 | 17.9% | NOT LISTED |
| Double-Edge | move | 2.1% | 5 | 0 | 65 | 7.7% | NOT LISTED |
| Haze | move | 0.8% | 5 | 0 | 27 | 18.5% | listed |
| Hyper Beam | move | 0.2% | 5 | 0 | 9 | 55.6% | listed |
| Mach Punch | move | 0.8% | 5 | 0 | 30 | 16.7% | NOT LISTED |
| Morning Sun | move | 1.2% | 5 | 0 | 40 | 12.5% | listed |
| Poison Jab | move | 1.3% | 5 | 0 | 46 | 10.9% | NOT LISTED |
| Psychic Fangs | move | 1.7% | 5 | 0 | 42 | 11.9% | listed |
| Rock Slide | move | 1.1% | 5 | 0 | 35 | 14.3% | NOT LISTED |
| Scale Shot | move | 0.8% | 5 | 0 | 15 | 33.3% | NOT LISTED |
| Trick Room | move | 1.5% | 5 | 0 | 45 | 11.1% | NOT LISTED |
| Wood Hammer | move | 1.1% | 5 | 0 | 18 | 27.8% | NOT LISTED |
| Chlorophyll | ability | 0.4% | 4 | 0 | 32 | 12.5% | listed |
| Early Bird | ability | 0.1% | 4 | 0 | 14 | 28.6% | listed |
| Flash Fire | ability | 0.3% | 4 | 0 | 48 | 8.3% | listed |
| Mirror Armor | ability | 0.2% | 4 | 0 | 31 | 12.9% | listed |
| Piercing Drill | ability | 0.3% | 4 | 0 | 48 | 8.3% | listed |
| Purifying Salt | ability | 0.1% | 4 | 0 | 33 | 12.1% | listed |
| Water Bubble | ability | 0.2% | 4 | 0 | 38 | 10.5% | listed |
| Zero to Hero | ability | 0.1% | 4 | 0 | 10 | 40.0% | NOT LISTED |
| Altarianite | item | 0.1% | 4 | 0 | 22 | 18.2% | NOT LISTED |
| Chesto Berry | item | 0.0% | 4 | 0 | 23 | 17.4% | NOT LISTED |
| Delphoxite | item | 0.7% | 4 | 0 | 57 | 7.0% | NOT LISTED |
| Eelektrossite | item | 0.1% | 4 | 0 | 38 | 10.5% | NOT LISTED |
| Scovillainite | item | 0.2% | 4 | 0 | 30 | 13.3% | NOT LISTED |
| Armor Cannon | move | 0.1% | 4 | 0 | 4 | 100.0% | listed |
| Avalanche | move | 0.6% | 4 | 0 | 21 | 19.0% | listed |
| Bitter Blade | move | 1.1% | 4 | 0 | 26 | 15.4% | listed |
| Bug Buzz | move | 0.3% | 4 | 0 | 10 | 40.0% | NOT LISTED |
| Dazzling Gleam | move | 1.7% | 4 | 0 | 35 | 11.4% | NOT LISTED |
| Defog | move | 0.1% | 4 | 0 | 4 | 100.0% | listed |
| Double Shock | move | 0.8% | 4 | 0 | 16 | 25.0% | listed |
| Dragon Darts | move | 0.3% | 3 | 1 | 10 | 40.0% | NOT LISTED |
| Drill Run | move | 0.9% | 4 | 0 | 17 | 23.5% | NOT LISTED |
| Extreme Speed | move | 1.3% | 4 | 0 | 45 | 8.9% | NOT LISTED |
| Fire Punch | move | 0.2% | 4 | 0 | 7 | 57.1% | NOT LISTED |
| High Jump Kick | move | 1.5% | 4 | 0 | 27 | 14.8% | NOT LISTED |
| Hurricane | move | 0.9% | 4 | 0 | 26 | 15.4% | listed |
| Ice Fang | move | 0.5% | 4 | 0 | 22 | 18.2% | NOT LISTED |
| Infestation | move | 0.7% | 4 | 0 | 35 | 11.4% | listed |
| Metal Burst | move | 0.1% | 4 | 0 | 9 | 44.4% | listed |
| Overheat | move | 1.2% | 4 | 0 | 23 | 17.4% | listed |
| Psychic Terrain | move | 0.1% | 4 | 0 | 8 | 50.0% | listed |
| Rapid Spin | move | 0.1% | 4 | 0 | 6 | 66.7% | listed |
| Sandstorm | move | 0.1% | 4 | 0 | 10 | 40.0% | listed |
| Shell Smash | move | 0.8% | 4 | 0 | 27 | 14.8% | NOT LISTED |
| Smack Down | move | 0.2% | 4 | 0 | 9 | 44.4% | listed |
| Whirlwind | move | 0.7% | 4 | 0 | 29 | 13.8% | listed |
| Eelevate | ability | 0.1% | 3 | 0 | 26 | 11.5% | NOT LISTED |
| Harvest | ability | 0.2% | 3 | 0 | 11 | 27.3% | listed |
| Overgrow | ability | 0.5% | 3 | 0 | 39 | 7.7% | NOT LISTED |
| Sand Rush | ability | 0.1% | 3 | 0 | 16 | 18.8% | listed |
| Sap Sipper | ability | 0.3% | 3 | 0 | 25 | 12.0% | listed |
| Kangaskhanite | item | 0.1% | 3 | 0 | 11 | 27.3% | listed (as Mega Kangaskhan) |
| Lum Berry | item | 0.5% | 3 | 0 | 45 | 6.7% | listed |
| Meganiumite | item | 0.3% | 3 | 0 | 33 | 9.1% | NOT LISTED |
| Miracle Seed | item | 0.0% | 3 | 0 | 27 | 11.1% | NOT LISTED |
| Mystic Water | item | 0.1% | 3 | 0 | 23 | 13.0% | NOT LISTED |
| Baton Pass | move | 1.5% | 3 | 0 | 28 | 10.7% | listed |
| Blizzard | move | 1.0% | 3 | 0 | 16 | 18.8% | listed |
| Bulk Up | move | 1.4% | 3 | 0 | 26 | 11.5% | NOT LISTED |
| Double Team | move | 0.1% | 3 | 0 | 3 | 100.0% | listed |
| Drain Punch | move | 0.5% | 3 | 0 | 15 | 20.0% | listed |
| Kowtow Cleave | move | 1.1% | 3 | 0 | 32 | 9.4% | listed |
| Quiver Dance | move | 1.5% | 3 | 0 | 54 | 5.6% | NOT LISTED |
| Rock Blast | move | 0.4% | 3 | 0 | 23 | 13.0% | listed |
| Scary Face | move | 0.1% | 3 | 0 | 6 | 50.0% | NOT LISTED |
| Spiky Shield | move | 0.6% | 3 | 0 | 13 | 23.1% | listed |
| Strength Sap | move | 0.5% | 3 | 0 | 8 | 37.5% | listed |
| Thunder | move | 0.3% | 3 | 0 | 23 | 13.0% | listed |
| Torch Song | move | 0.5% | 3 | 0 | 47 | 6.4% | NOT LISTED |
| Insomnia | ability | 0.1% | 2 | 0 | 19 | 10.5% | listed |
| Spicy Spray | ability | 0.2% | 2 | 0 | 9 | 22.2% | listed |
| Swarm | ability | 0.2% | 2 | 0 | 7 | 28.6% | NOT LISTED |
| Synchronize | ability | 0.1% | 2 | 0 | 10 | 20.0% | listed |
| Absolite | item | 0.1% | 2 | 0 | 12 | 16.7% | listed (as Mega Absol) |
| Black Belt | item | 0.0% | 2 | 0 | 30 | 6.7% | NOT LISTED |
| Froslassite | item | 0.1% | 2 | 0 | 8 | 25.0% | NOT LISTED |
| Kebia Berry | item | 0.1% | 2 | 0 | 9 | 22.2% | NOT LISTED |
| White Herb | item | 0.2% | 2 | 0 | 15 | 13.3% | NOT LISTED |
| Acrobatics | move | 0.4% | 2 | 0 | 9 | 22.2% | NOT LISTED |
| Alluring Voice | move | 0.4% | 2 | 0 | 6 | 33.3% | listed |
| Aqua Cutter | move | 0.5% | 2 | 0 | 10 | 20.0% | NOT LISTED |
| Belly Drum | move | 0.1% | 2 | 0 | 11 | 18.2% | listed |
| Bitter Malice | move | 0.2% | 2 | 0 | 2 | 100.0% | NOT LISTED |
| Blaze Kick | move | 1.0% | 2 | 0 | 28 | 7.1% | listed |
| Brick Break | move | 0.4% | 2 | 0 | 14 | 14.3% | listed |
| Brutal Swing | move | 0.1% | 2 | 0 | 3 | 66.7% | NOT LISTED |
| Bulldoze | move | 0.1% | 2 | 0 | 7 | 28.6% | listed |
| Dragon Claw | move | 1.1% | 2 | 0 | 21 | 9.5% | NOT LISTED |
| Fiery Dance | move | 1.2% | 2 | 0 | 46 | 4.3% | listed |
| Fire Fang | move | 0.4% | 2 | 0 | 16 | 12.5% | NOT LISTED |
| Hydro Cannon | move | 0.3% | 2 | 0 | 8 | 25.0% | NOT LISTED |
| Hypnosis | move | 0.1% | 2 | 0 | 6 | 33.3% | listed |
| Icy Wind | move | 0.8% | 2 | 0 | 17 | 11.8% | NOT LISTED |
| Nuzzle | move | 0.5% | 2 | 0 | 3 | 66.7% | NOT LISTED |
| Parting Shot | move | 0.5% | 2 | 0 | 11 | 18.2% | listed |
| Pin Missile | move | 0.2% | 2 | 0 | 6 | 33.3% | listed |
| Psycho Cut | move | 0.7% | 2 | 0 | 10 | 20.0% | NOT LISTED |
| Sacred Sword | move | 0.9% | 2 | 0 | 17 | 11.8% | NOT LISTED |
| Sheer Cold | move | 0.7% | 2 | 0 | 8 | 25.0% | listed |
| Spikes | move | 0.6% | 2 | 0 | 13 | 15.4% | listed |
| Stored Power | move | 0.5% | 2 | 0 | 27 | 7.4% | NOT LISTED |
| Swagger | move | 0.3% | 2 | 0 | 8 | 25.0% | listed |
| Thunder Wave | move | 0.3% | 2 | 0 | 3 | 66.7% | listed |
| Water Pulse | move | 0.2% | 2 | 0 | 3 | 66.7% | NOT LISTED |
| Weather Ball | move | 1.7% | 2 | 0 | 38 | 5.3% | listed |
| Armor Tail | ability | 0.0% | 1 | 0 | 25 | 4.0% | listed |
| Berserk | ability | 0.1% | 1 | 0 | 11 | 9.1% | NOT LISTED |
| Corrosion | ability | 0.1% | 1 | 0 | 14 | 7.1% | NOT LISTED |
| Healer | ability | 0.1% | 1 | 0 | 4 | 25.0% | NOT LISTED |
| Heatproof | ability | 0.0% | 1 | 0 | 9 | 11.1% | NOT LISTED |
| Leaf Guard | ability | 0.3% | 1 | 0 | 16 | 6.2% | listed |
| Marvel Scale | ability | 0.0% | 1 | 0 | 8 | 12.5% | NOT LISTED |
| Mega Sol | ability | 0.3% | 1 | 0 | 9 | 11.1% | listed |
| Motor Drive | ability | 0.0% | 1 | 0 | 13 | 7.7% | listed |
| Moxie | ability | 0.2% | 1 | 0 | 19 | 5.3% | NOT LISTED |
| Oblivious | ability | 0.2% | 1 | 0 | 14 | 7.1% | NOT LISTED |
| Parental Bond | ability | 0.1% | 1 | 0 | 6 | 16.7% | listed |
| Super Luck | ability | 0.1% | 1 | 0 | 6 | 16.7% | listed |
| Tangled Feet | ability | 0.0% | 1 | 0 | 10 | 10.0% | listed |
| Unnerve | ability | 0.2% | 1 | 0 | 6 | 16.7% | listed |
| Audinite | item | 0.0% | 1 | 0 | 4 | 25.0% | NOT LISTED |
| Drampanite | item | 0.0% | 1 | 0 | 11 | 9.1% | listed (as Mega Drampa) |
| Emboarite | item | 0.0% | 1 | 0 | 4 | 25.0% | NOT LISTED |
| Galladite | item | 0.1% | 1 | 0 | 4 | 25.0% | NOT LISTED |
| Iron Ball | item | 0.0% | 1 | 0 | 8 | 12.5% | listed |
| King's Rock | item | 0.1% | 1 | 0 | 5 | 20.0% | listed |
| Pyroarite | item | 0.1% | 1 | 0 | 6 | 16.7% | NOT LISTED |
| Red Card | item | 0.1% | 1 | 0 | 1 | 100.0% | listed |
| Sharpedonite | item | 0.0% | 1 | 0 | 6 | 16.7% | NOT LISTED |
| Aqua Step | move | 0.1% | 1 | 0 | 2 | 50.0% | listed |
| Baneful Bunker | move | 0.3% | 1 | 0 | 5 | 20.0% | listed |
| Body Slam | move | 0.4% | 1 | 0 | 13 | 7.7% | NOT LISTED |
| Bullet Seed | move | 0.3% | 1 | 0 | 5 | 20.0% | listed |
| Coil | move | 0.2% | 1 | 0 | 10 | 10.0% | NOT LISTED |
| Cosmic Power | move | 0.5% | 1 | 0 | 20 | 5.0% | NOT LISTED |
| Cotton Guard | move | 0.2% | 1 | 0 | 2 | 50.0% | NOT LISTED |
| Counter | move | 0.3% | 1 | 0 | 9 | 11.1% | listed |
| Darkest Lariat | move | 0.1% | 1 | 0 | 2 | 50.0% | NOT LISTED |
| Draining Kiss | move | 0.5% | 1 | 0 | 5 | 20.0% | listed |
| Dual Wingbeat | move | 0.1% | 1 | 0 | 2 | 50.0% | NOT LISTED |
| Energy Ball | move | 1.0% | 1 | 0 | 6 | 16.7% | NOT LISTED |
| Flame Charge | move | 0.5% | 1 | 0 | 24 | 4.2% | NOT LISTED |
| Gigaton Hammer | move | 0.2% | 1 | 0 | 1 | 100.0% | NOT LISTED |
| Gunk Shot | move | 1.0% | 1 | 0 | 16 | 6.2% | NOT LISTED |
| Heavy Slam | move | 0.5% | 1 | 0 | 8 | 12.5% | NOT LISTED |
| Hex | move | 0.1% | 1 | 0 | 1 | 100.0% | listed |
| Horn Drill | move | 0.3% | 1 | 0 | 13 | 7.7% | listed |
| Icicle Crash | move | 0.4% | 1 | 0 | 7 | 14.3% | NOT LISTED |
| Last Respects | move | 2.3% | 1 | 0 | 39 | 2.6% | listed |
| Leaf Storm | move | 0.3% | 1 | 0 | 7 | 14.3% | NOT LISTED |
| Liquidation | move | 1.3% | 1 | 0 | 18 | 5.6% | NOT LISTED |
| Memento | move | 0.2% | 1 | 0 | 1 | 100.0% | listed |
| Misty Explosion | move | 0.1% | 1 | 0 | 2 | 50.0% | listed |
| Night Slash | move | 0.5% | 1 | 0 | 4 | 25.0% | NOT LISTED |
| No Retreat | move | 0.0% | 1 | 0 | 2 | 50.0% | listed |
| Power Whip | move | 0.6% | 1 | 0 | 7 | 14.3% | NOT LISTED |
| Psychic | move | 0.8% | 1 | 0 | 9 | 11.1% | listed |
| Quick Attack | move | 0.4% | 1 | 0 | 11 | 9.1% | NOT LISTED |
| Razor Shell | move | 0.4% | 1 | 0 | 12 | 8.3% | NOT LISTED |
| Skill Swap | move | 0.0% | 1 | 0 | 1 | 100.0% | listed |
| Sticky Web | move | 0.3% | 1 | 0 | 12 | 8.3% | listed |
| Stomping Tantrum | move | 0.1% | 1 | 0 | 4 | 25.0% | listed |
| Water Shuriken | move | 0.4% | 1 | 0 | 2 | 50.0% | listed |
| Zen Headbutt | move | 0.2% | 1 | 0 | 3 | 33.3% | NOT LISTED |

## Likely new

134 entities not named in `gaps/INDEX.md` were in play in at least 3 mismatch steps or legal-action failures (all in the table above). Most are there only because they are common. Listed here: the 21 of them whose rate is at least 1.5 times the rate over all compared steps (19.0%, so at least 28.6%), highest rate first, with one example each (the step with the fewest entities in play). The fields are the differences that are not HP or a faint; the log lines are Showdown's for that step.

### Natural Cure (ability): 12 mismatch steps, 0 legal-action failures, rate 63.2%

Example `1:29:5` (battle 29, step 5), choices {"p1": "switch 2", "p2": "move 2 mega"}, entities icespinner, goodasgold, lifeorb, naturalcure, starminite.

- `sides.p2.pokemon[0].stats.atk`: Showdown `152`, engine `192`

```
|switch|p1a: Hydreigon|Hydreigon, L50, M|169/169
|detailschange|p2a: Starmie|Starmie-Mega, L50
|-mega|p2a: Starmie|Starmie|Starminite
|move|p2a: Starmie|Ice Spinner|p1a: Hydreigon
|-supereffective|p1a: Hydreigon|1
|-damage|p1a: Hydreigon|0 fnt
|faint|p1a: Hydreigon
|-weather|Snowscape|[upkeep]
|upkeep
```

### Fire Punch (move): 4 mismatch steps, 0 legal-action failures, rate 57.1%

Example `1:56:1` (battle 56, step 1), choices {"p1": "move 3", "p2": "switch 1"}, entities firepunch, earlybird, rockyhelmet, grassysurge, lifeorb.

- `turn`: Showdown `2`, engine `3`
- `weather.left`: Showdown `5`, engine `4`
- `terrain.left`: Showdown `4`, engine `3`
- `sides.p1.pokemon[0].hp`: Showdown `72`, engine `83`
- `sides.p1.pokemon[0].activeTurns`: Showdown `2`, engine `3`
- `sides.p2.pokemon[1].activeTurns`: Showdown `0`, engine `1`

```
|switch|p2a: Politoed|Politoed, L50, M|197/197
|-weather|RainDance|[from] ability: Drizzle|[of] p2a: Politoed
|move|p1a: Kangaskhan|Fire Punch|p2a: Politoed
|-resisted|p2a: Politoed|1
|-damage|p2a: Politoed|187/197
|-status|p2a: Politoed|brn
|-enditem|p2a: Politoed|Eject Button
```

### Spell Tag (item): 55 mismatch steps, 0 legal-action failures, rate 50.9%

Example `1:85:7` (battle 85, step 7), choices {"p1": "move 3", "p2": "switch 0"}, entities kingsshield, stancechange, spelltag, stamina.

- `sides.p1.pokemon[0].species`: Showdown `"aegislash"`, engine `"aegislashblade"`
- `sides.p1.pokemon[0].stats.atk`: Showdown `112`, engine `211`
- `sides.p1.pokemon[0].stats.def`: Showdown `160`, engine `70`
- `sides.p1.pokemon[0].stats.spa`: Showdown `63`, engine `144`
- `sides.p1.pokemon[0].stats.spd`: Showdown `162`, engine `72`
- `sides.p1.pokemon[0].volatiles.kingsshield`: Showdown `"<absent>"`, engine `{}`

```
|switch|p2a: Charizard|Charizard, L50, F|155/155
|-formechange|p1a: Aegislash|Aegislash|
|move|p1a: Aegislash|King's Shield||[still]
|-fail|p1a: Aegislash
|upkeep
|turn|9
```

### Scary Face (move): 3 mismatch steps, 0 legal-action failures, rate 50.0%

Example `1:224:0` (battle 224, step 0), choices {"p1": "move 1", "p2": "move 1 mega"}, entities powergem, scaryface, sturdy, smoothrock, roughskin, garchompitez.

- `sides.p2.pokemon[0].ability`: Showdown `"levitate"`, engine `"sandforce"`
- `sides.p2.pokemon[0].baseAbility`: Showdown `"levitate"`, engine `"sandforce"`

```
|detailschange|p2a: Garchomp|Garchomp-Mega-Z, L50, M
|-mega|p2a: Garchomp|Garchomp|Garchompite Z
|move|p2a: Garchomp|Power Gem|p1a: Aggron
|-resisted|p1a: Aggron|1
|-damage|p1a: Aggron|149/177
|move|p1a: Aggron|Scary Face|p2a: Garchomp
|-unboost|p2a: Garchomp|spe|2
|upkeep
|turn|2
```

### Flower Trick (move): 25 mismatch steps, 0 legal-action failures, rate 41.7%

Example `1:96:13` (battle 96, step 13), choices {"p1": "switch 2", "p2": "move 0"}, entities flowertrick, toughclaws, metagrossite, protean.

- `sides.p2.pokemon[0].types`: Showdown `["grass"]`, engine `["grass", "dark"]`

```
|switch|p1a: Meowscarada|Meowscarada, L50, M|54/153
|move|p2a: Meowscarada|Flower Trick|p1a: Meowscarada
|-start|p2a: Meowscarada|typechange|Grass|[from] ability: Protean
|-resisted|p1a: Meowscarada|1
|-crit|p1a: Meowscarada
|-damage|p1a: Meowscarada|0 fnt
|faint|p1a: Meowscarada
|upkeep
```

### Zero to Hero (ability): 4 mismatch steps, 0 legal-action failures, rate 40.0%

Example `1:476:9` (battle 476, step 9), choices {"p1": "move 0", "p2": "move 0"}, entities paraboliccharge, zerotohero, electromorphosis, leftovers.

- `sides.p2.pokemon[0].hp`: Showdown `117`, engine `116`
- `sides.p2.pokemon[0].volatiles.charge`: Showdown `"<absent>"`, engine `{}`

```
|move|p2a: Bellibolt|Parabolic Charge|p1a: Palafin
|-supereffective|p1a: Palafin|1
|-damage|p1a: Palafin|0 fnt
|-heal|p2a: Bellibolt|104/215|[from] drain|[of] p1a: Palafin
|faint|p1a: Palafin
|-end|p2a: Bellibolt|Charge|[silent]
|-heal|p2a: Bellibolt|117/215|[from] item: Leftovers
|upkeep
```

### Bug Buzz (move): 4 mismatch steps, 0 legal-action failures, rate 40.0%

Example `1:327:8` (battle 327, step 8), choices {"p1": "move 2", "p2": "move 1"}, entities bugbuzz, thickfat, leftovers, flamebody.

- `sides.p1.pokemon[1].lastMove`: Showdown `"rest"`, engine `"earthquake"`

```
|move|p2a: Volcarona|Bug Buzz|p1a: Snorlax
|-damage|p1a: Snorlax|164/257 slp
|cant|p1a: Snorlax|slp
|-heal|p1a: Snorlax|180/257 slp|[from] item: Leftovers
|upkeep
|turn|8
```

### Dragon Darts (move): 3 mismatch steps, 1 legal-action failures, rate 40.0%

Example `1:251:6` (battle 251, step 6), choices {"p1": "move 2", "p2": "move 0"}, entities dragondarts, swordsdance, disguise, lifeorb, cursedbody.

- `sides.p1.pokemon[0].species`: Showdown `"mimikyu"`, engine `"mimikyubusted"`

```
|move|p2a: Dragapult|Dragon Darts|p1a: Mimikyu
|-immune|p1a: Mimikyu
|move|p1a: Mimikyu|Swords Dance|p1a: Mimikyu
|-boost|p1a: Mimikyu|atk|2
|upkeep
|turn|6
```

### Hyper Voice (move): 16 mismatch steps, 0 legal-action failures, rate 37.2%

Example `1:40:10` (battle 40, step 10), choices {"p1": "switch 0", "p2": "move 1"}, entities hypervoice, moody, leftovers, gardevoirite.

- `sides.p2.pokemon[2].boosts.spd`: Showdown `2`, engine `0`
- `sides.p2.pokemon[2].boosts.spe`: Showdown `-2`, engine `-1`

```
|switch|p1a: Greninja|Greninja, L50, F|149/149
|move|p2a: Gardevoir|Hyper Voice|p1a: Greninja
|-damage|p1a: Greninja|76/149
|-ability|p2a: Gardevoir|Moody|boost
|-boost|p2a: Gardevoir|spd|2
|-unboost|p2a: Gardevoir|spe|1
|upkeep
|turn|10
```

### Focus Blast (move): 8 mismatch steps, 0 legal-action failures, rate 36.4%

Example `1:37:26` (battle 37, step 26), choices {"p1": "move 2", "p2": "move 1"}, entities sparklingaria, focusblast, torrent, regenerator, leftovers.

- `sides.p2.pokemon[2].volatiles.sparklingaria`: Showdown `"<absent>"`, engine `{}`

```
|move|p1a: Primarina|Sparkling Aria|p2a: Reuniclus
|-damage|p2a: Reuniclus|80/217 slp
|-curestatus|p2a: Reuniclus|slp|[msg]
|move|p2a: Reuniclus|Focus Blast|p1a: Primarina
|-resisted|p1a: Primarina|1
|-damage|p1a: Primarina|112/187
|-heal|p2a: Reuniclus|93/217|[from] item: Leftovers
|upkeep
|turn|25
```

### Drum Beating (move): 13 mismatch steps, 0 legal-action failures, rate 36.1%

Example `1:25:21` (battle 25, step 21), choices {"p1": "move 1", "p2": "move 3"}, entities drumbeating, grassysurge, lifeorb, emergencyexit, golisopite.

- `terrain.left`: Showdown `5`, engine `4`

```
|move|p1a: Rillaboom|Drum Beating|p2a: Golisopod
|-damage|p2a: Golisopod|0 fnt
|faint|p2a: Golisopod
|-damage|p1a: Rillaboom|131/207|[from] item: Life Orb
|win|p1
```

### Superpower (move): 8 mismatch steps, 0 legal-action failures, rate 34.8%

Example `1:92:19` (battle 92, step 19), choices {"p1": "move 1", "p2": "move 2"}, entities superpower, torrent, sandstream, tyranitarite.

- `sides.p1.pokemon[1].moves[1].pp`: Showdown `6`, engine `5`

```
|move|p2a: Tyranitar|Superpower|p1a: Primarina
|-resisted|p1a: Primarina|1
|-damage|p1a: Primarina|116/187 slp
|-unboost|p2a: Tyranitar|atk|1
|-unboost|p2a: Tyranitar|def|1
|cant|p1a: Primarina|slp
|-weather|Sandstorm|[upkeep]
|-damage|p1a: Primarina|105/187 slp|[from] Sandstorm
|upkeep
|turn|19
```

### Pixilate (ability): 77 mismatch steps, 2 legal-action failures, rate 33.6%

Example `1:64:5` (battle 64, step 5), choices {"p1": "move 0 mega", "p2": "move 0"}, entities thunderbolt, synchronize, gardevoirite, pixilate.

- `sides.p1.pokemon[1].volatiles.healblock`: Showdown `"<absent>"`, engine `{}`

```
|detailschange|p1a: Gardevoir|Gardevoir-Mega, L50, M
|-mega|p1a: Gardevoir|Gardevoir|Gardevoirite
|move|p1a: Gardevoir|Thunderbolt|p2a: Gardevoir
|-damage|p2a: Gardevoir|0 fnt
|faint|p2a: Gardevoir
|upkeep
```

### Scale Shot (move): 5 mismatch steps, 0 legal-action failures, rate 33.3%

Example `1:136:0` (battle 136, step 0), choices {"p1": "switch 2", "p2": "move 0"}, entities scaleshot, levitate, choicescarf, roughskin, focussash.

- `weather.left`: Showdown `7`, engine `4`

```
|switch|p1a: Pelipper|Pelipper, L50, M|137/137
|-weather|RainDance|[from] ability: Drizzle|[of] p1a: Pelipper
|move|p2a: Garchomp|Scale Shot|p1a: Pelipper
|-damage|p1a: Pelipper|112/137
|-damage|p1a: Pelipper|88/137
|-hitcount|p1a: Pelipper|2
|-unboost|p2a: Garchomp|def|1
|-boost|p2a: Garchomp|spe|1
|-weather|RainDance|[upkeep]
|upkeep
|turn|2
```

### Telepathy (ability): 9 mismatch steps, 0 legal-action failures, rate 32.1%

Example `1:121:0` (battle 121, step 0), choices {"p1": "switch 1", "p2": "move 2 mega"}, entities solarbeam, telepathy, pechaberry, blaze, charizarditey.

- `sides.p1.pokemon[1].volatiles.unburden`: Showdown `{}`, engine `"<absent>"`
- `sides.p2.active`: Showdown `2`, engine `0`
- `sides.p2.pokemon[0].lastMove`: Showdown `""`, engine `"solarbeam"`
- `sides.p2.pokemon[0].activeTurns`: Showdown `0`, engine `2`
- `sides.p2.pokemon[2].activeTurns`: Showdown `1`, engine `0`

```
|switch|p1a: Slurpuff|Slurpuff, L50, F|189/189
|detailschange|p2a: Charizard|Charizard-Mega-Y, L50, F
|-mega|p2a: Charizard|Charizard|Charizardite Y
|-weather|SunnyDay|[from] ability: Drought|[of] p2a: Charizard
|move|p2a: Charizard|Solar Beam||[still]
|-prepare|p2a: Charizard|Solar Beam
|-anim|p2a: Charizard|Solar Beam|p1a: Slurpuff
|-damage|p1a: Slurpuff|67/189
|-enditem|p1a: Slurpuff|Red Card|[of] p2a: Charizard
|drag|p2a: Archaludon|Archaludon, L50, F|197/197
|-weather|SunnyDay|[upkeep]
|upkeep
|turn|2
```

### Pecha Berry (item): 9 mismatch steps, 0 legal-action failures, rate 32.1%

Example `1:121:0` (battle 121, step 0), choices {"p1": "switch 1", "p2": "move 2 mega"}, entities solarbeam, telepathy, pechaberry, blaze, charizarditey.

- `sides.p1.pokemon[1].volatiles.unburden`: Showdown `{}`, engine `"<absent>"`
- `sides.p2.active`: Showdown `2`, engine `0`
- `sides.p2.pokemon[0].lastMove`: Showdown `""`, engine `"solarbeam"`
- `sides.p2.pokemon[0].activeTurns`: Showdown `0`, engine `2`
- `sides.p2.pokemon[2].activeTurns`: Showdown `1`, engine `0`

```
|switch|p1a: Slurpuff|Slurpuff, L50, F|189/189
|detailschange|p2a: Charizard|Charizard-Mega-Y, L50, F
|-mega|p2a: Charizard|Charizard|Charizardite Y
|-weather|SunnyDay|[from] ability: Drought|[of] p2a: Charizard
|move|p2a: Charizard|Solar Beam||[still]
|-prepare|p2a: Charizard|Solar Beam
|-anim|p2a: Charizard|Solar Beam|p1a: Slurpuff
|-damage|p1a: Slurpuff|67/189
|-enditem|p1a: Slurpuff|Red Card|[of] p2a: Charizard
|drag|p2a: Archaludon|Archaludon, L50, F|197/197
|-weather|SunnyDay|[upkeep]
|upkeep
|turn|2
```

### Poltergeist (move): 14 mismatch steps, 0 legal-action failures, rate 31.8%

Example `1:202:34` (battle 202, step 34), choices {"p1": "move 3", "p2": "move 0"}, entities poltergeist, eartheater, stancechange, spelltag.

- `sides.p1.pokemon[1].moves[3].pp`: Showdown `7`, engine `6`
- `sides.p2.pokemon[1].species`: Showdown `"aegislashblade"`, engine `"aegislash"`
- `sides.p2.pokemon[1].stats.atk`: Showdown `211`, engine `112`
- `sides.p2.pokemon[1].stats.def`: Showdown `70`, engine `160`
- `sides.p2.pokemon[1].stats.spa`: Showdown `144`, engine `63`
- `sides.p2.pokemon[1].stats.spd`: Showdown `72`, engine `162`

```
|cant|p1a: Orthworm|slp
|-formechange|p2a: Aegislash|Aegislash-Blade|
|move|p2a: Aegislash|Poltergeist||[still]
|-fail|p2a: Aegislash
|upkeep
|turn|30
```

### Falinksite (item): 6 mismatch steps, 0 legal-action failures, rate 30.0%

Example `1:292:14` (battle 292, step 14), choices {"p1": "switch 0", "p2": "move 3"}, entities sandstream, defiant, falinksite.

- `sides.p2.pokemon[2].lastMove`: Showdown `"closecombat"`, engine `"knockoff"`

```
|switch|p1a: Archaludon|Archaludon, L50, M|134/167
|cant|p2a: Falinks|slp
|upkeep
|turn|14
```

### Raichunite Y (item): 40 mismatch steps, 0 legal-action failures, rate 29.6%

Example `1:293:1` (battle 293, step 1), choices {"p1": "switch 2", "p2": "switch 2"}, entities static, raichunitey, multiscale, dragoninite.

- `terrain.left`: Showdown `7`, engine `4`

```
|switch|p1a: Archaludon|Archaludon, L50, F|167/167
|switch|p2a: Indeedee|Indeedee, L50, M|137/137
|-fieldstart|move: Psychic Terrain|[from] ability: Psychic Surge|[of] p2a: Indeedee
|upkeep
|turn|3
```

### Gardevoirite (item): 41 mismatch steps, 0 legal-action failures, rate 29.5%

Example `1:40:3` (battle 40, step 3), choices {"p1": "move 2", "p2": "switch 0"}, entities protect, moody, leftovers, gardevoirite.

- `sides.p1.pokemon[2].boosts.spa`: Showdown `4`, engine `2`
- `sides.p1.pokemon[2].boosts.spd`: Showdown `-1`, engine `0`

```
|switch|p2a: Golisopod|Golisopod, L50, F|55/182
|move|p1a: Scovillain|Protect||[still]
|-fail|p1a: Scovillain
|-heal|p1a: Scovillain|84/145|[from] item: Leftovers
|-ability|p1a: Scovillain|Moody|boost
|-boost|p1a: Scovillain|spa|2
|-unboost|p1a: Scovillain|spd|1
|upkeep
|turn|4
```

### Zap Cannon (move): 6 mismatch steps, 0 legal-action failures, rate 28.6%

Example `1:207:2` (battle 207, step 2), choices {"p1": "move 1 mega", "p2": "move 0"}, entities zapcannon, static, raichunitey, torrent, leftovers.

- `sides.p1.pokemon[2].moves[2].disabled`: Showdown `true`, engine `false`

```
|detailschange|p1a: Raichu|Raichu-Mega-Y, L50, F
|-mega|p1a: Raichu|Raichu|Raichunite Y
|move|p1a: Raichu|Zap Cannon|p2a: Primarina
|-supereffective|p2a: Primarina|1
|-damage|p2a: Primarina|0 fnt
|faint|p2a: Primarina
|upkeep
```

## What the engine cannot hold (dropped on load)

Snapshot content the engine rig declared dropped when loading Showdown's state, once per kind, with the number of loads it occurred in. A dropped field is not compared for the rest of that step; 5074 field differences were set aside that way.

| kind | loads |
|---|---:|
| `side.pokemon.item:lightclay` | 1209 |
| `side.pokemon.item:terrainextender` | 715 |
| `side.pokemon.ability:auraguard` | 574 |
| `side.pokemon.baseAbility:auraguard` | 574 |
| `side.pokemon.item:damprock` | 562 |
| `side.pokemon.item:smoothrock` | 435 |
| `side.pokemon.item:quickclaw` | 360 |
| `side.pokemon.item:scopelens` | 142 |
| `side.pokemon.moves:<move>` (34 move ids, not in the set: a transformed Ditto's moves, or Struggle) | 120 |
| `side.pokemon.volatiles.lockedmove.trueDuration` | 110 |
| `side.pokemon.volatiles.confusion.left` | 93 |
| `side.pokemon.statusData.turns` | 80 |
| `side.pokemon.item:zoomlens` | 65 |
| `side.pokemon.volatiles.disable.left` | 65 |
| `side.pokemon.volatiles.disable.move` | 65 |
| `side.pokemon.item:ejectbutton` | 59 |
| `side.pokemon.volatiles.partiallytrapped.boundDivisor` | 45 |
| `side.pokemon.volatiles.partiallytrapped.left` | 45 |
| `side.pokemon.volatiles.partiallytrapped.move` | 45 |
| `side.pokemon.item:kingsrock` | 29 |
| `side.pokemon.lastMove:<move>` (8 move ids, not in the set: a transformed Ditto's moves, or Struggle) | 27 |
| `side.pokemon.item:redcard` | 22 |
| `side.pokemon.volatiles.throatchop.left` | 18 |
| `side.pokemon.item:heatrock` | 15 |
| `side.pokemon.volatiles.healblock.left` | 13 |

Ids the engine does not have (replaced by nothing when the team is built), by loads: `p2:mega-ability:auraguard` 708, `p1:item:lightclay` 705, `p1:mega-ability:auraguard` 569, `p2:item:lightclay` 538, `p1:item:terrainextender` 461, `p1:item:damprock` 365, `p2:item:terrainextender` 254, `p2:item:quickclaw` 238, `p2:item:smoothrock` 237, `p1:item:smoothrock` 213, `p2:item:damprock` 197, `p1:item:quickclaw` 122, `p2:item:scopelens` 122, `p1:item:ejectbutton` 95, `p2:item:ejectbutton` 72, `p1:item:redcard` 51, `p2:item:zoomlens` 42, `p1:item:zoomlens` 23, `p1:item:scopelens` 20, `p1:item:kingsrock` 17, `p2:item:heatrock` 15, `p2:item:kingsrock` 12. `mega-ability:x`: the set's Mega has ability x in Showdown, which the engine lacks; the engine's own Mega table gives that Mega another ability when it Mega Evolves.

## Not modelled by the engine (not counted as differences)

Fields the engine has no counter for, declared by the engine rig on every load (`*.` entries of `dropped`). A difference where Showdown has the field and the engine does not is set aside, once per kind below with the number of differences set aside in the chosen branches; whether the condition itself is present is still compared.

| kind | differences set aside |
|---|---:|
| `side.pokemon.volatiles.lockedmove.trueDuration` | 63 |
| `side.pokemon.volatiles.confusion.left` | 37 |
| `side.pokemon.volatiles.partiallytrapped.boundDivisor` | 22 |
| `side.pokemon.volatiles.partiallytrapped.left` | 22 |
| `side.pokemon.volatiles.partiallytrapped.move` | 22 |
| `side.pokemon.volatiles.throatchop.left` | 15 |
| `side.pokemon.statusData.turns` | 13 |
| `side.pokemon.volatiles.healblock.left` | 13 |
| `side.pokemon.volatiles.roost.left` | 2 |

Finished battles: volatiles and moves' `disabled` flags are not compared once Showdown's battle has ended (Showdown keeps bookkeeping volatiles the engine has cleared, and no decision is left); 114 differences set aside that way: `side.pokemon.moves.disabled` 61, `side.pokemon.volatiles.yawn` 8, `side.pokemon.volatiles.stall` 8, `side.pokemon.volatiles.encore.left` 6, `side.pokemon.volatiles.twoturnmove` 6, `side.pokemon.volatiles.encore` 5, `side.pokemon.volatiles.curse` 5, `side.pokemon.volatiles.glaiverush` 4, `side.pokemon.volatiles.taunt.left` 2, `side.pokemon.volatiles.choicelock` 2, `side.pokemon.volatiles.charge` 1, `side.pokemon.volatiles.taunt` 1, `side.pokemon.volatiles.protect` 1, `side.pokemon.volatiles.stall.left` 1, `side.pokemon.volatiles.flinch` 1, `side.pokemon.volatiles.disable` 1, `side.pokemon.volatiles.focusenergy` 1.

## Engine errors

- 2 x `p2: move struggle is not a legal choice`

## Probability

Steps where the engine has a branch equal to Showdown's result in every compared field: 2352. For each, the engine's probability of that result (the sum over its branches equal to the result, usually one) is compared with the product of the `p` of Showdown's events for the step, `roll` events left out (the engine does not branch on the damage roll).

| | steps | share |
|---|---:|---:|
| agree within 1e-6 | 1973 | 83.9% |
| disagree | 379 | 16.1% |
| of which within 1e-4 relative (float rounding in the engine's f32 percentages) | 0 | 0.0% |

What this can show: on these steps, whether the engine gives the outcome Showdown reached the same chance as Showdown's own draws, event by event. What it cannot show: (1) it covers only steps the engine got exactly right, which are the simpler steps; nothing is said about the probabilities of steps with any difference, nor about outcomes Showdown did not reach. (2) Showdown's product is the chance of one sequence of draws, not of the resulting state: when several sequences lead to the same state (a critical hit that knocks out either way, the order of two effects that do not interact), the engine rightly merges them into one branch with a higher probability, and Showdown draws that change nothing (the order of end-of-turn effects, `speedtie` with an `effect`) have no engine branch at all. So a disagreement is a place to look, not a proven fault; an engine probability below Showdown's product, or a step with no Showdown events and an engine probability below 1, cannot come from merging. (3) Damage rolls are not checked at all, and one step per outcome is no test of a distribution (the chi-square test of PROJECT.md section 7 needs the engine to sample).

Disagreements grouped by the event keys of the step (`speedtie:x+y` is a tie between two effects; "engine higher" fits merged outcomes, "engine lower" does not):

| event keys | steps | engine higher | engine lower | examples (step: Showdown p, engine p) |
|---|---:|---:|---:|---|
| `crit` | 106 | 93 | 13 | 1:5:4: 0.9583, 1; 1:8:2: 0.9583, 1; 1:25:12: 0.9583, 1 |
| `accuracy + crit` | 65 | 54 | 11 | 1:5:8: 0.7, 0.8; 1:25:17: 0.8625, 0.9; 1:32:7: 0.8625, 0.9 |
| `speedtie:pokemon` | 53 | 53 | 0 | 1:0:10: 0.0625, 1; 1:3:20: 0.0625, 1; 1:11:8: 0.125, 1 |
| `speedtie:protect+stall` | 28 | 28 | 0 | 1:12:3: 0.5, 1; 1:18:0: 0.5, 1; 1:75:16: 0.5, 1 |
| `accuracy + crit + secondary` | 18 | 16 | 2 | 1:2:7: 0.23, 0.8; 1:10:17: 0.7331, 0.85; 1:30:9: 0.8539, 0.9 |
| `crit + secondary` | 18 | 16 | 2 | 1:18:29: 0.8625, 0.9583; 1:67:20: 0.4792, 1; 1:88:32: 0.7667, 1 |
| `sleepturns` | 15 | 15 | 0 | 1:5:16: 0.2222, 1; 1:53:18: 0.6667, 1; 1:92:18: 0.6667, 1 |
| `(no events)` | 7 | 0 | 7 | 1:31:18: 1, 0.667; 1:102:3: 1, 0.667; 1:302:1: 1, 0.667 |
| `proc:quickclaw` | 6 | 6 | 0 | 1:20:9: 0.8, 1; 1:20:10: 0.8, 1; 1:20:11: 0.8, 1 |
| `accuracy` | 4 | 3 | 1 | 1:6:11: 0.7, 1; 1:270:10: 0.85, 0.15; 1:382:40: 0.2, 0.9667 (2 branches) |
| `accuracy + crit + sleepturns` | 4 | 4 | 0 | 1:27:11: 0.5111, 1 (3 branches); 1:128:40: 0.2875, 0.9625 (2 branches); 1:128:43: 0.2875, 1 (3 branches) |
| `crit + secondary + sleepturns` | 4 | 4 | 0 | 1:53:33: 0.3194, 0.5 (2 branches); 1:128:31: 0.2875, 0.8625; 1:354:36: 0.3194, 1 (2 branches) |
| `sleepturns + speedtie:protect+stall` | 4 | 4 | 0 | 1:37:9: 0.3333, 1; 1:37:30: 0.3333, 1; 1:327:15: 0.3333, 1 |
| `accuracy + crit + speedtie:pokemon` | 3 | 3 | 0 | 1:156:1: 0.05391, 0.9; 1:413:7: 0.1078, 0.9; 1:454:19: 0.1078, 0.8625 |
| `crit + proc:quickclaw` | 3 | 3 | 0 | 1:53:9: 0.1917, 1; 1:133:3: 0.1917, 1; 1:133:13: 0.7667, 0.9583 |
| `accuracy + confusionturns + crit` | 2 | 2 | 0 | 1:130:2: 0.1837, 0.2875; 1:155:8: 0.2181, 0.9104 |
| `accuracy + crit + hits` | 2 | 1 | 1 | 1:128:54: 0.1091, 0.0375; 1:128:65: 0.3019, 1 (2 branches) |
| `accuracy + crit + proc:cursedbody` | 2 | 2 | 0 | 1:27:13: 0.5367, 0.7521; 1:60:7: 0.6038, 0.9 |
| `accuracy + crit + proc:quickclaw` | 2 | 2 | 0 | 1:133:10: 0.6133, 0.8; 1:194:10: 0.1533, 0.7042 |
| `accuracy + crit + speedtie + speedtie:leftovers+leftovers + speedtie:pokemon` | 2 | 2 | 0 | 1:214:3: 0.01348, 0.45; 1:413:9: 0.01348, 0.45 |
| `confusionturns + crit` | 2 | 2 | 0 | 1:288:6: 0.2396, 1; 1:338:3: 0.2396, 1 |
| `crit + sleepturns` | 2 | 2 | 0 | 1:176:3: 0.6389, 1 (2 branches); 1:325:9: 0.6389, 1 (2 branches) |
| `speedtie` | 2 | 2 | 0 | 1:271:5: 0.5, 1 (2 branches); 1:274:6: 0.5, 1 (2 branches) |
| `speedtie + speedtie:pokemon` | 2 | 2 | 0 | 1:138:5: 0.0625, 1 (2 branches); 1:312:17: 0.001953, 1 (2 branches) |
| `speedtie:leftovers+leftovers + speedtie:pokemon` | 2 | 2 | 0 | 1:172:2: 0.01562, 1; 1:282:18: 0.01562, 1 |
| `accuracy + confusionhit + crit` | 1 | 0 | 1 | 1:279:12: 0.5137, 0.1181 |
| `accuracy + confusionturns + crit + speedtie + speedtie:pokemon` | 1 | 1 | 0 | 1:274:5: 0.000852, 0.475 |
| `accuracy + crit + secondary + sleepturns` | 1 | 1 | 0 | 1:282:12: 0.4888, 1 (5 branches) |
| `accuracy + crit + secondary + speedtie:pokemon` | 1 | 1 | 0 | 1:172:0: 0.01438, 0.7665 (2 branches) |
| `accuracy + speedtie:protect+stall` | 1 | 1 | 0 | 1:384:7: 0.35, 1 |
| `confusionhit` | 1 | 1 | 0 | 1:83:6: 0.33, 0.3333 |
| `confusionturns + speedtie:choicelock+encore` | 1 | 1 | 0 | 1:289:5: 0.125, 1 |
| `crit + proc:cursedbody` | 1 | 1 | 0 | 1:47:2: 0.02917, 0.04167 |
| `crit + proc:cutecharm` | 1 | 1 | 0 | 1:332:6: 0.2875, 1 (2 branches) |
| `crit + protect` | 1 | 1 | 0 | 1:337:14: 0.6389, 1 |
| `crit + secondary + speedtie:pokemon` | 1 | 1 | 0 | 1:358:0: 0.1078, 1 |
| `crit + speedtie + speedtie:leftovers+leftovers + speedtie:pokemon` | 1 | 1 | 0 | 1:172:15: 0.01497, 0.5 |
| `crit + speedtie + speedtie:pokemon` | 1 | 1 | 0 | 1:11:14: 0.02995, 0.5 |
| `crit + speedtie:leftovers+leftovers + speedtie:pokemon` | 1 | 1 | 0 | 1:149:29: 0.01497, 1 |
| `protect + speedtie:protect+stall` | 1 | 1 | 0 | 1:199:21: 0.1667, 0.3333 |
| `sleepturns + speedtie + speedtie:pokemon` | 1 | 1 | 0 | 1:282:43: 4.069e-05, 1 (2 branches) |
| `sleepturns + speedtie:pokemon + speedtie:protect+stall` | 1 | 1 | 0 | 1:74:18: 0.000651, 1 |
| `speedtie:banefulbunker+stall` | 1 | 1 | 0 | 1:352:10: 0.5, 1 |
| `speedtie:choicelock+disable` | 1 | 1 | 0 | 1:168:13: 0.5, 1 |
| `speedtie:phantomforce+twoturnmove` | 1 | 1 | 0 | 1:97:9: 0.5, 1 |
| `speedtie:spikyshield+stall` | 1 | 1 | 0 | 1:335:14: 0.5, 1 |
