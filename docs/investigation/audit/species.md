# Audit: SPECIES AND FORMS

Slice: all 392 rows of `03-coverage-species.csv`. Reference: Showdown `data\pokedex.ts` (there is NO `data\mods\champions\pokedex.ts`, so base pokedex is the Champions data), `data\abilities.ts`, `data\items.ts`, `data\moves.ts`, `data\mods\champions\{scripts,items,formats-data}.ts`. Engine: `genx\base_stats.rs`, `genx\state.rs`, `genx\abilities.rs`, `genx\items.rs`, `genx\choice_effects.rs`, `genx\generate_instructions.rs`, `state.rs`, `pokemon.rs`. No code was run.

Rows audited: 392 (82 Mega forms, 33 special-mechanic species/forms, 277 plain species summarised on one line).

## Counts per verdict

| verdict | count | notes |
|---|---|---|
| MATCH | 85 | 75 Megas (stats, types, ability), 10 special species |
| PLAIN | 284 | 277 plain species + Rotom, Greninja, Ogerpon x4, Terapagos-Terastal |
| DATA | 5 | Garchomp-Mega-Z, Golisopod-Mega, Absol-Mega-Z, Starmie-Mega, Drampa-Mega |
| MISSING | 12 | Lucario-Mega-Z (Aura Guard not in engine), Kangaskhan-Mega (Parental Bond has no behaviour), Aegislash x2, Castform x4, Ditto, Farfetch'd, Sirfetch'd, Meloetta-Pirouette |
| LOGIC | 3 | Mimikyu, Pikachu, Eiscue-Noice |
| CORE | 2 | Zoroark, Zoroark-Hisui (Illusion = K9) |
| UNSURE | 1 | Minior-Meteor |

Weight is NOT part of the Mega verdicts (see G1 below): the engine never changes `weight_kg` on Mega Evolution.

## Five (ten) most important findings

1. **Aegislash (legal, OU, used=1) cannot change forme.** `Abilities::STANCECHANGE` is an enum entry with no code anywhere; `AEGISLASHBLADE` is only an enum entry; `base_stats()` has no Aegislash arm (would `panic!` on any recalc). Showdown switches to Blade on any damaging move and back to Shield on King's Shield (abilities.ts:4524-4531).
2. **Mega ability errors:** Garchomp-Mega-Z (engine Sand Force, SD Levitate), Golisopod-Mega (engine Emergency Exit, SD Tough Claws), Absol-Mega-Z (engine Magic Bounce, SD Sharpness), Lucario-Mega-Z (engine Adaptability, SD Aura Guard; `Abilities::AURAGUARD` does not exist in the engine at all, so a team string "auraguard" cannot even be parsed).
3. **Mega stat errors:** Starmie-Mega Atk 140 (SD 100); Drampa-Mega SpD 114 (SD 116).
4. **Kangaskhan-Mega (legal, used=1): `Abilities::PARENTALBOND` is only an enum entry**, no behaviour anywhere (grep of all `src`). Same for `STALWART` (Skarmory-Mega, singles no-op) and `HEALER` (Audino-Mega, singles no-op).
5. **G1: Mega Evolution never updates `weight_kg`.** Showdown uses the Mega's own species weight (verified differences: Charizard 90.5 -> X 110.5 / Y 100.5, Gyarados 235 -> 305, Steelix 400 -> 740, Aggron 360 -> 395, Golisopod 108 -> 148, Drampa 185 -> 240.5, Lucario 54 -> 57.5 / Z 49.4, Garchomp 95 -> Z 99, Absol 47 -> 49). Affects Low Kick, Grass Knot, Heavy Slam, Heat Crash against/with a Mega. `mega_evolve` (generate_instructions.rs:3916-3986) changes id, stats, ability, base_ability, types only. Fix M: add weight to `MegaEvolveData` and an instruction to change it.
6. **Ditto (legal, OU) cannot Transform:** `Abilities::IMPOSTER` enum only; `Choices::TRANSFORM` is a no-op row (choices.rs:18003-18012: Normal, no effect). Fix L.
7. **Castform (legal, RU): Forecast has no behaviour** (enum only), no Castform-Sunny/Rainy/Snowy base stats or types. Fix M.
8. **Mimikyu Disguise busts too early:** handled in `ability_before_move` (abilities.rs:590-614), which runs before accuracy, Protect, Substitute checks (generate_instructions.rs:2255), so a missed or protected move still busts it and costs 1/8 HP; base_power=0 blocks every hit of a multi-hit move (SD blocks only the first); fixed-damage moves are not stopped by base_power=0.
9. **Leek is not in the engine** (`Items` has no LEEK, no STICK) but is legal in Champions (champions\items.ts:514-517 `isNonstandard: null`); Farfetch'd and Sirfetch'd are both legal (crit stage +2 missing, and the item string cannot be parsed).
10. **Caller contract (section 2):** the engine has no species table for types, weight, abilities, or non-Mega base stats. Every non-Mega species is purely what the caller writes into the `Pokemon` string. `base_stats()` covers only the Megas plus 8 forme ids and panics for all other ids.

## Section 2: where the engine gets species data

- The engine has **no pokedex**. `PokemonName` (pokemon.rs, 392/392 slice ids present) is an enum used as an id only.
- **Supplied by the caller** (`Pokemon::deserialize`, state.rs:1000-1050; fields at state.rs:852-877): `id`, `level`, `types` (current, 2 slots), `base_types` (2), `hp`, `maxhp`, `ability`, `base_ability`, `item`, `nature`, `evs` (Champions stat points), the five computed stats `attack/defense/special_attack/special_defense/speed`, `status`, `rest_turns`, `sleep_turns`, **`weight_kg`**, four moves (with PP), `mega_evolved`, `terastallized`, `tera_type`. Maxhp is passed in; the engine does not compute it.
- **Looked up by the engine** (`genx\base_stats.rs`): `base_stats()` lines 18-130 returns a 6-tuple for MINIOR, MINIORMETEOR, WISHIWASHI(+SCHOOL), PALAFIN(+HERO), EISCUE(+NOICE) and every Mega id (including Megas that are illegal here). Any other id hits `panic!("Base stats not implemented")`. `recalculate_stats` (genx\state.rs:318-365) and `calculate_stats_from_base_stats` (366-489) recompute the five stats at level (50), IV 31, Champions stat points via `stat_points_to_effective_ev` (8*sp-4), nature x11/10 or x9/10. This matches Showdown `champions\scripts.ts:10-45` (no `levelclausemod` in the format, so SD uses base+sp+20 / base+sp+75, identical at level 50). Recalc is only called by Mega, Palafin, Minior, Wishiwashi, Eiscue code.
- **Types after a forme change:** only `mega_evolve_target` supplies types (and ability). Palafin-Hero, Morpeko-Hangry, Mimikyu-Busted, Cramorant forms keep the caller's types (same as SD for these). Nothing sets types for Castform formes.
- **Abilities after a forme change:** only Mega sets `ability` and `base_ability` (matches Champions `formeChange`, scripts.ts:105-115, which resets ability and baseAbility to the Mega's slot 0).
- **Weight:** never looked up; read from `pkmn.weight_kg` by `weight_kg()` (choice_effects.rs:37-47: Heavy Metal x2, Light Metal /2 truncated to 0.1 kg, min 0.1). Used by Low Kick / Grass Knot (641-661) and Heavy Slam / Heat Crash (662-681). Not updated on Mega or any forme change; Autotomize has a volatile enum entry only (state.rs:166) and no weight effect (SD `onModifyWeight`); Float Stone is not needed (Past in Champions).
- Mega item to species mapping `mega_evolve_target` (base_stats.rs:132-711): keyed on (current id, item); Floette uses FLOETTEETERNAL (matches SD items.ts:2204), Meowstic M and F both map. Every legal Showdown Champions Mega (list taken from `champions\formats-data.ts`, all non-Illegal `*mega*` entries) has an arm. Engine arms with no legal Showdown Mega (all `Past`/`Future` there): Mewtwo X/Y, Latias, Latios, Diancie, Darkrai, Heatran, Magearna (x2), Tatsugiri (x3), Zeraora, Zygarde-Complete. Showdown Megas with no engine arm: Rayquaza-Mega (requires move, illegal), Crucibelle-Mega (CAP, not legal). `Absol`/`Garchomp`/`Lucario` have two arms each (normal and Z stone).
- Stat points: `can_mega_evolve` (genx\state.rs:307-316) tests only id + item; the "one Mega per side" rule comes from `side_can_mega` (add_available_moves), the comment about multiple Megas is stale.

## Table: Mega forms (one row per CSV id containing `mega`)

Showdown file is `data\pokedex.ts` (line = the `name:` line). Engine is `genx\base_stats.rs` `mega_evolve_target` arm start line and the matching `base_stats()` arm. "weight not updated" is global finding G1 and is not repeated per row.

| id | used | verdict | Showdown | engine | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| garchompmegaz | 1 | DATA | pokedex.ts:8185 Dragon, 108/130/85/141/85/151, Levitate | base_stats.rs:656 Dragon/Typeless, same stats, SANDFORCE | ability Sand Force vs Levitate (Levitate is implemented, abilities.rs:2630) | S | high |
| golisopodmega | 1 | DATA | pokedex.ts:14533 Bug/Steel, 75/150/175/70/120/40, Tough Claws | base_stats.rs:632 same types/stats, EMERGENCYEXIT | ability Emergency Exit vs Tough Claws | S | high |
| salamencemega | 1 | MATCH | pokedex.ts:6937 Dragon/Flying 95/145/130/120/90/120 Aerilate | base_stats.rs:350 same | | | high |
| lucariomegaz | 1 | MISSING | pokedex.ts:8260 Fighting/Steel 70/100/70/164/70/151 Aura Guard (abilities.ts:310: x0.5 damage from contact moves; champions\abilities.ts:8 makes it legal) | base_stats.rs:662 same types/stats, ADAPTABILITY | ability Adaptability vs Aura Guard; `Abilities::AURAGUARD` absent from the enum (grep of src: no hits), so the effect has no implementation and the string cannot be parsed | M | high |
| dragonitemega | 1 | MATCH | pokedex.ts:3367 Dragon/Flying 91/124/115/145/125/100 Multiscale | base_stats.rs:416 same | | | high |
| charizardmegay | 1 | MATCH | pokedex.ts:140 Fire/Flying 78/104/78/159/115/100 Drought | base_stats.rs:146 same | | | high |
| metagrossmega | 1 | MATCH | pokedex.ts:6995 Steel/Psychic 80/145/150/105/110/110 Tough Claws | base_stats.rs:356 same | | | high |
| charizardmegax | 1 | MATCH | pokedex.ts:125 Fire/Dragon 78/130/111/130/85/100 Tough Claws | base_stats.rs:140 same | | | high |
| blazikenmega | 1 | MATCH | pokedex.ts:5119 Fire/Fighting 80/160/80/130/80/100 Speed Boost | base_stats.rs:266 same | | | high |
| baxcaliburmega | 1 | MATCH | pokedex.ts:19060 Dragon/Ice 115/175/117/105/101/87 Thermal Exchange | base_stats.rs:650 same | | | high |
| staraptormega | 1 | MATCH | pokedex.ts:7440 Fighting/Flying 85/140/100/60/90/110 Contrary | base_stats.rs:560 same | | | high |
| raichumegay | 1 | MATCH | pokedex.ts:854 Electric 60/100/55/160/80/130 No Guard | base_stats.rs:620 same | | | high |
| glimmoramega | 1 | MATCH | pokedex.ts:18591 Rock/Poison 83/90/105/150/96/101 Adaptability | base_stats.rs:482 same | | | high |
| greninjamega | 1 | MATCH | pokedex.ts:12202 Water/Dark 72/125/77/133/81/142 Protean | base_stats.rs:458 same | | | high |
| clefablemega | 1 | MATCH | pokedex.ts:1047 Fairy/Flying 95/80/93/135/110/70 Magic Bounce | base_stats.rs:422 same | | | high |
| gyaradosmega | 1 | MATCH | pokedex.ts:2967 Water/Dark 95/155/109/70/130/81 Mold Breaker | base_stats.rs:200 same | | | high |
| swampertmega | 1 | MATCH | pokedex.ts:5178 Water/Ground 100/150/110/95/110/70 Swift Swim | base_stats.rs:272 same | | | high |
| starmiemega | 1 | DATA | pokedex.ts:2749 Water/Psychic 60/100/105/130/105/120 Huge Power | base_stats.rs:500 (stats at 93) 60/140/105/130/105/120 | Atk 140 vs 100 | S | high |
| delphoxmega | 1 | MATCH | pokedex.ts:12110 Fire/Psychic 75/69/72/159/125/134 Levitate | base_stats.rs:452 same | | | high |
| tyranitarmega | 1 | MATCH | pokedex.ts:4963 Rock/Dark 100/164/150/95/120/71 Sand Stream | base_stats.rs:254 same | | | high |
| gengarmega | 1 | MATCH | pokedex.ts:2226 Ghost/Poison 60/65/80/170/95/130 Shadow Tag | base_stats.rs:182 same | Telekinesis immunity of Gengar-Mega (moves.ts:19132, 19148) not implemented, but the Telekinesis volatile has no behaviour at all in the engine (moves slice) | | high |
| lopunnymega | 1 | MATCH | pokedex.ts:7934 Normal/Fighting 65/136/94/54/96/135 Scrappy | base_stats.rs:374 same | | | high |
| blastoisemega | 1 | MATCH | pokedex.ts:216 Water 79/103/120/135/115/78 Mega Launcher | base_stats.rs:152 same | | | high |
| lucariomega | 1 | MATCH | pokedex.ts:8245 Fighting/Steel 70/145/88/140/70/112 Adaptability | base_stats.rs:386 same | | | high |
| venusaurmega | 1 | MATCH | pokedex.ts:49 Grass/Poison 80/100/123/122/120/80 Thick Fat | base_stats.rs:134 same | | | high |
| banettemega | 1 | MATCH | pokedex.ts:6614 Ghost 64/165/75/93/83/75 Prankster | base_stats.rs:332 same | | | high |
| gardevoirmega | 1 | MATCH | pokedex.ts:5514 Psychic/Fairy 68/85/65/165/135/100 Pixilate | base_stats.rs:278 same | | | high |
| scizormega | 1 | MATCH | pokedex.ts:4402 Bug/Steel 70/150/140/65/100/75 Technician | base_stats.rs:236 same | | | high |
| meganiummega | 1 | MATCH | pokedex.ts:3484 Grass/Fairy 80/92/115/143/115/80 Mega Sol | base_stats.rs:428 same | | | high |
| excadrillmega | 1 | MATCH | pokedex.ts:9929 Ground/Steel 110/165/100/65/65/103 Piercing Drill | base_stats.rs:536 same | | | high |
| absolmegaz | 1 | DATA | pokedex.ts:6724 Dark/Ghost 65/154/60/75/60/151 Sharpness | base_stats.rs:644 same types/stats, MAGICBOUNCE | ability Magic Bounce vs Sharpness (Sharpness exists, abilities.rs:1902) | S | high |
| mawilemega | 1 | MATCH | pokedex.ts:5817 Steel/Fairy 50/105/125/55/95/50 Huge Power | base_stats.rs:290 same | | | high |
| slowbromega | 1 | MATCH | pokedex.ts:1965 Water/Psychic 95/75/180/130/80/30 Shell Armor | base_stats.rs:176 same | | | high |
| victreebelmega | 1 | MATCH | pokedex.ts:1734 Grass/Poison 80/125/85/135/95/70 Innards Out | base_stats.rs:494 same | | | high |
| scovillainmega | 1 | MATCH | pokedex.ts:18331 Grass/Fire 65/138/85/138/85/75 Spicy Spray | base_stats.rs:476 same | | | high |
| heracrossmega | 1 | MATCH | pokedex.ts:4440 Bug/Fighting 80/185/115/40/105/75 Skill Link | base_stats.rs:242 same | | | high |
| eelektrossmega | 1 | MATCH | pokedex.ts:11159 Electric 85/145/80/135/90/80 Eelevate | base_stats.rs:578 same | | | high |
| dragalgemega | 1 | MATCH | pokedex.ts:13001 Poison/Dragon 65/85/105/132/163/44 Regenerator | base_stats.rs:602 same | | | high |
| gallademega | 1 | MATCH | pokedex.ts:8663 Psychic/Fighting 68/165/95/65/115/110 Inner Focus | base_stats.rs:398 same | | | high |
| pyroarmega | 1 | MATCH | pokedex.ts:12524 Fire/Normal 86/88/92/129/86/126 Fire Mane | base_stats.rs:584 same | | | high |
| glaliemega | 1 | MATCH | pokedex.ts:6779 Ice 80/120/80/120/80/100 Refrigerate | base_stats.rs:344 same | | | high |
| absolmega | 1 | MATCH | pokedex.ts:6710 Dark 65/150/60/115/60/115 Magic Bounce | base_stats.rs:338 same | | | high |
| kangaskhanmega | 1 | MISSING | pokedex.ts:2653 Normal 105/125/100/60/100/100 Parental Bond | base_stats.rs:188 same data | data MATCH, but `Abilities::PARENTALBOND` is an enum entry only (abilities.rs:285; grep of src finds no other use): the second hit never happens. Abilities slice owns the behaviour | L | high |
| beedrillmega | 1 | MATCH | pokedex.ts:343 Bug/Poison 65/150/40/15/80/145 Adaptability | base_stats.rs:158 same | | | high |
| sceptilemega | 1 | MATCH | pokedex.ts:5060 Grass/Dragon 70/110/75/145/85/145 Lightning Rod | base_stats.rs:260 same | | | high |
| altariamega | 1 | MATCH | pokedex.ts:6301 Dragon/Fairy 75/110/110/110/105/80 Pixilate | base_stats.rs:326 same | | | high |
| feraligatrmega | 1 | MATCH | pokedex.ts:3604 Water/Dragon 85/160/125/89/93/78 Dragonize | base_stats.rs:434 same | | | high |
| aerodactylmega | 1 | MATCH | pokedex.ts:3192 Rock/Flying 80/135/85/70/95/150 Tough Claws | base_stats.rs:206 same | | | high |
| pinsirmega | 1 | MATCH | pokedex.ts:2870 Bug/Flying 65/155/120/65/90/105 Aerilate | base_stats.rs:194 same | | | high |
| chesnaughtmega | 1 | MATCH | pokedex.ts:12050 Grass/Fighting 88/137/172/74/115/44 Bulletproof | base_stats.rs:446 same | | | high |
| froslassmega | 1 | MATCH | pokedex.ts:8723 Ice/Ghost 70/80/70/140/100/120 Snow Warning | base_stats.rs:530 same | | | high |
| aggronmega | 1 | MATCH | pokedex.ts:5872 Steel 70/140/230/60/80/50 Filter | base_stats.rs:296 same | | | high |
| sableyemega | 1 | MATCH | pokedex.ts:5790 Dark/Ghost 50/85/125/85/115/20 Magic Bounce | base_stats.rs:284 same | | | high |
| drampamega | 1 | DATA | pokedex.ts:15100 Normal/Dragon 78/85/110/160/116/36 Berserk | base_stats.rs:542 (stats at 100) 78/85/110/160/114/36 | SpD 114 vs 116 | S | high |
| sharpedomega | 1 | MATCH | pokedex.ts:6083 Water/Dark 70/140/70/110/65/105 Strong Jaw | base_stats.rs:314 same | | | high |
| alakazammega | 1 | MATCH | pokedex.ts:1618 Psychic 55/50/65/175/105/150 Trace | base_stats.rs:170 same | | | high |
| falinksmega | 1 | MATCH | pokedex.ts:16862 Fighting 65/135/135/70/65/100 Defiant | base_stats.rs:608 same | | | high |
| pidgeotmega | 1 | MATCH | pokedex.ts:398 Normal/Flying 83/80/80/135/80/121 No Guard | base_stats.rs:164 same | | | high |
| abomasnowmega | 1 | MATCH | pokedex.ts:8427 Grass/Ice 90/132/105/132/105/30 Snow Warning | base_stats.rs:392 same | | | high |
| emboarmega | 1 | MATCH | pokedex.ts:9499 Fire/Fighting 110/148/75/110/110/75 Mold Breaker | base_stats.rs:440 same | | | high |
| barbaraclemega | 1 | MATCH | pokedex.ts:12959 Rock/Fighting 72/140/130/64/106/88 Tough Claws | base_stats.rs:596 same | | | high |
| audinomega | 1 | MATCH | pokedex.ts:9957 Normal/Fairy 103/60/126/80/126/50 Healer | base_stats.rs:404 same | Healer has no behaviour in the engine (grep: no use), a no-op in singles | | high |
| skarmorymega | 1 | MATCH | pokedex.ts:4652 Steel/Flying 65/140/110/40/100/110 Stalwart | base_stats.rs:512 same | Stalwart enum only, a no-op in singles | | high |
| floettemega | 1 | MATCH | pokedex.ts:12589 Fairy 74/85/87/155/148/102 Fairy Aura | base_stats.rs:488 (key FLOETTEETERNAL, matches items.ts:2204) same | | | high |
| steelixmega | 1 | MATCH | pokedex.ts:4318 Steel/Ground 75/125/230/55/95/30 Sand Force | base_stats.rs:230 same | | | high |
| scraftymega | 1 | MATCH | pokedex.ts:10478 Dark/Fighting 65/130/135/55/135/68 Intimidate | base_stats.rs:572 same | | | high |
| chandeluremega | 1 | MATCH | pokedex.ts:11241 Ghost/Fire 60/75/110/175/110/90 Infiltrator | base_stats.rs:554 same | | | high |
| medichammega | 1 | MATCH | pokedex.ts:5913 Fighting/Psychic 60/100/85/80/85/100 Pure Power | base_stats.rs:302 same | | | high |
| ampharosmega | 1 | MATCH | pokedex.ts:3911 Electric/Dragon 90/95/105/165/110/45 Mold Breaker | base_stats.rs:224 same | | | high |
| houndoommega | 1 | MATCH | pokedex.ts:4694 Dark/Fire 75/90/90/140/90/115 Solar Power | base_stats.rs:248 same | | | high |
| cameruptmega | 1 | MATCH | pokedex.ts:6149 Fire/Ground 70/120/100/145/105/20 Sheer Force | base_stats.rs:320 same | | | high |
| scolipedemega | 1 | MATCH | pokedex.ts:10156 Bug/Poison 60/140/149/75/99/62 Shell Armor | base_stats.rs:566 same | | | high |
| hawluchamega | 1 | MATCH | pokedex.ts:13151 Fighting/Flying 78/137/100/74/93/118 No Guard | base_stats.rs:506 same | | | high |
| crabominablemega | 1 | MATCH | pokedex.ts:13979 Fighting/Ice 97/157/122/62/107/33 Iron Fist | base_stats.rs:464 same | | | high |
| chimechomega | 1 | MATCH | pokedex.ts:6682 Psychic/Steel 75/50/110/135/120/65 Levitate | base_stats.rs:548 same | | | high |
| malamarmega | 1 | MATCH | pokedex.ts:12917 Dark/Psychic 86/102/88/98/120/88 Contrary | base_stats.rs:590 same | | | high |
| manectricmega | 1 | MATCH | pokedex.ts:5954 Electric 70/75/80/135/80/135 Intimidate | base_stats.rs:308 same | | | high |
| garchompmega | 1 | MATCH | pokedex.ts:8171 Dragon/Ground 108/170/115/120/95/92 Sand Force | base_stats.rs:380 same | | | high |
| golurkmega | 1 | MATCH | pokedex.ts:11449 Ground/Ghost 89/159/105/70/105/55 Unseen Fist | base_stats.rs:470 same | | | high |
| meowsticfmega | 1 | MATCH | pokedex.ts:12762 Psychic 74/48/76/143/101/124 Trace | base_stats.rs:524 (key MEOWSTICF) same | | | high |
| meowsticmmega | 1 | MATCH | pokedex.ts:12745 Psychic 74/48/76/143/101/124 Trace | base_stats.rs:518 (key MEOWSTIC) same | | | high |
| raichumegax | 1 | MATCH | pokedex.ts:839 Electric 60/135/95/90/95/110 Electric Surge | base_stats.rs:614 same | | | high |

(Row count above: 82. Typeless in the engine means a single type, equal to a one-type Showdown entry.)

## Table: species with a form change or species-specific mechanic

| id | used | verdict | Showdown | engine | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| mimikyu | 1 | LOGIC | abilities.ts:970-1016 Disguise: `onDamage` returns 0 for the first damaging move hit (effectType Move), crit and effectiveness handlers, forme change to Busted and 1/8 maxhp damage in `onUpdate` after the hit; later hits of a multi-hit move damage normally | abilities.rs:590-614 (champions cfg) in `ability_before_move`: any Physical/Special move vs MIMIKYU sets base_power 0, forme change, 1/8 damage. Called at generate_instructions.rs:2255, before accuracy/protect/substitute | busts and takes 1/8 even if the move misses, hits Protect or a Substitute; all hits of a multi-hit move are blocked instead of only the first (K2 aside); fixed-damage moves ignore base_power 0 (to confirm); the SD effectiveness and crit rules are not needed | M | medium |
| mimikyubusted | 0 | MATCH | pokedex.ts Mimikyu-Busted same stats and types, Disguise inactive once species is not mimikyu | enum entry; Disguise arm requires id MIMIKYU/MIMIKYUTOTEM | | | high |
| aegislash | 1 | MISSING | abilities.ts:4524-4531 Stance Change: any damaging move -> Aegislash-Blade (60/140/50/140/50/60), King's Shield -> Aegislash | abilities.rs:280 `STANCECHANGE` enum entry only; no code; `base_stats()` has no AEGISLASH arm (panic if reached) | no forme switching at all: Aegislash always stays Shield stats (Atk/SpA 50, Def/SpD 140) | L | high |
| aegislashblade | 0 | MISSING | pokedex.ts Aegislash-Blade, types Steel/Ghost | enum entry only, no base stats | never reached; caller could place it manually but engine never reverts it for King's Shield | M | high |
| palafin | 1 | MATCH | abilities.ts:5628-5647 Zero to Hero: on switch-out becomes Palafin-Hero permanently | abilities.rs:1071-1082 in `ability_on_switch_out`: id -> PALAFINHERO, `recalculate_stats`; base_stats.rs:24-25 matches pokedex (100/70/72/53/62/100 and 100/160/97/106/87/100) | edge: `ability_on_switch_out` returns early under Neutralizing Gas (abilities.rs:1056) though Zero to Hero is `cantsuppress` (not legal here) | | high |
| palafinhero | 0 | MATCH | Palafin-Hero stats as above | base_stats.rs:25 | | | high |
| morpeko | 1 | MATCH | abilities.ts:1897-1908 Hunger Switch toggles forme each residual unless terastallized; Aura Wheel type Electric/Dark (moves.ts:801-807) | abilities.rs:1190-1208 toggles in `ability_end_of_turn`; abilities.rs:1083-1092 reverts to Morpeko on switch-out (SD reverts via clearVolatile); choice_effects.rs:142-146 Aura Wheel Dark for MORPEKOHANGRY; stats identical both formes | | | medium |
| morpekohangry | 0 | MATCH | same | same arms | | | medium |
| castform | 0 | MISSING | abilities.ts:1477-1497 Forecast: Castform becomes Sunny/Rainy/Snowy/normal on weather, types Fire/Water/Ice/Normal | abilities.rs:75 `FORECAST` enum entry only; no id switching, no type change, no base stats | Castform never changes forme or type (legal, RU, unused) | M | high |
| castformrainy | 0 | MISSING | Water type, stats 70 all | enum entry only | unreachable except as a caller-supplied id; no switch | M | high |
| castformsnowy | 0 | MISSING | Ice type | enum entry only | as above | M | high |
| castformsunny | 0 | MISSING | Fire type | enum entry only | as above | M | high |
| ditto | 1 | MISSING | abilities Imposter + moves Transform copy species, stats (not HP), types, ability, moves (PP 5), boosts | abilities.rs:275 `IMPOSTER` enum only; choices.rs:18003-18012 Transform has Normal type and no effect | Ditto does nothing on switch-in; Transform no-op. Metal Powder/Quick Powder are Past in Champions (engine items.rs:1049 irrelevant) | L | high |
| zoroark | 1 | CORE | Illusion disguises as last team member until damaged; no mechanical effect other than visibility (K9) | abilities.rs:99 `ILLUSION` enum entry only; engine treats it as a plain Zoroark | none for rules; hidden-information model absent (K9) | | medium |
| zoroarkhisui | 1 | CORE | same | same | same | | medium |
| pikachu | 1 | LOGIC | items.ts:3446-3458 Light Ball: x2 Atk and x2 SpA stat for baseSpecies Pikachu | items.rs:1541-1549 `base_power *= 2.0` for `is_pikachu_variant()` (pokemon.rs:1481-1502, 17 forms; SD list 16) | doubles base power instead of the stat, so Body Press (Def stat) is wrongly doubled and Foul Play-like stat uses differ; damage otherwise equal; Eviolite is Past in Champions (champions\items.ts:294-297), so Pikachu NFE has no other mechanic | S | medium |
| farfetchd | 1 | MISSING | items.ts:3334-3349 Leek: +2 crit stage for Farfetch'd/Sirfetch'd; Champions makes it legal (champions\items.ts:514-517) | no `LEEK` (or `STICK`) in `Items` (grep of src: no hits) | item string cannot be parsed; no crit bonus (K7 aside) | M | high |
| sirfetchd | 1 | MISSING | same | same | same | M | high |
| taurospaldeablaze | 1 | MATCH | moves.ts:14630-14660 Raging Bull type by species, breaks Reflect, Light Screen, Aurora Veil | choice_effects.rs:162-183 type by id and x2 power vs Reflect/Aurora Veil; 803-82x removes Reflect, Light Screen (and presumably Aurora Veil) | damage-equivalent; details belong to moves slice | | medium |
| taurospaldeaaqua | 0 | MATCH | same, Water | choice_effects.rs:178 | | | medium |
| taurospaldeacombat | 0 | MATCH | same, Fighting | choice_effects.rs:172 | | | medium |
| rotom | 0 | PLAIN | pokedex.ts: Rotom appliance formes are separate species, no battle forme change (CSV status `c` is wrong) | enum entries for Rotom, Heat, Wash, Frost, Fan, Mow; types/abilities from caller | none | | medium |
| greninja | 1 | PLAIN | Battle Bond / Greninja-Ash are Past (formats-data.ts:3511-3518); only Protean applies | enum entry; abilities.rs:722 BATTLEBOND exists but unreachable | none | | medium |
| cramorantgorging | 0 | MATCH | Cramorant base is Past/Illegal (formats-data.ts:4582), unreachable in M-C. Gulp Missile abilities.ts:1746-1770 | abilities.rs:617-635, 828-868, 1060-1069 | unreachable; engine matches (1/4 maxhp, Def -1 or paralysis; Dive included) | | low |
| cramorantgulping | 0 | MATCH | same | same | same | | low |
| eiscuenoice | 0 | LOGIC | Eiscue is Past/Illegal (formats-data.ts:4709), unreachable. Ice Face abilities.ts:1970-2026 | abilities.rs:552-570 turns a Physical move into Status (hack) before accuracy checks; reform at abilities.rs:1426-1437 | unreachable; hack runs before accuracy/protect (as Mimikyu) | | low |
| miniormeteor | 0 | UNSURE | Minior is Past/Illegal (formats-data.ts:4205), unreachable. Shields Down abilities.ts:4240-4284 (switch-in and residual forme change, status immunity in Meteor) | abilities.rs:1209-1233 end of turn only; switch-in and status immunity not found | unreachable; would need to check switch-in and immunity | | low |
| meloettapirouette | 0 | MISSING | Meloetta is Past/Illegal (formats-data.ts:3443), unreachable. Relic Song switches formes (moves.ts:14945) | no Meloetta code | unreachable | | low |
| ogerponcornerstonetera | 0 | PLAIN | Ogerpon is Past/Illegal (formats-data.ts:5339-5354), terastallization feature not enabled in Cargo (`bss` = `champions` only) | items.rs:1132-1145, state.rs:605-615 mask code present, unreachable | unreachable | | low |
| ogerponhearthflametera | 0 | PLAIN | same | same | unreachable | | low |
| ogerpontealtera | 0 | PLAIN | same | enum entry only | unreachable | | low |
| ogerponwellspringtera | 0 | PLAIN | same | same | unreachable | | low |
| terapagosterastal | 0 | PLAIN | Terapagos is Past/Illegal (formats-data.ts:5379), unreachable | enum entry only | unreachable | | low |

Why the CSV `legal` column is wrong for the last ten rows: their base species are `isNonstandard: "Past"` in `champions\formats-data.ts`, and their forme entries carry no tag, so a naive scan marked them legal. In the real format you cannot bring Cramorant, Eiscue, Minior, Meloetta, Ogerpon or Terapagos.

## Plain species (no form change, no special mechanic)

277 species, verdict PLAIN: checked that `PokemonName` has an enum entry for every one. One grep of `pokemon.rs` with all 392 slice ids (uppercase, no punctuation) returned 392 matching enum lines, so none is missing (this includes every id with punctuation or a suffix: FARFETCHD, SIRFETCHD, MRMIME, MRRIME, KOMMOO, INDEEDEEF, BASCULEGIONF, MEOWSTICF, TOXTRICITYLOWKEY, MAUSHOLDFOUR, GOURGEISTSUPER, POLTEAGEISTANTIQUE, SINISTCHAMASTERPIECE, FLOETTEETERNAL, all 20 VIVILLON patterns, all 7 ALCREMIE flavours, SQUAWKABILLY x4, Rotom x6). Types, base types, abilities, weight, HP and stats of these species come from the caller (section 2), so there is nothing engine-side to compare. Caveat: Gourgeist forms (Small/Large/Super), Maushold, Indeedee, Basculegion, Toxtricity and Lycanroc differ in weight, ability or type per form, so the caller must supply the right values per id.

## Notes

- **N1 (cross-slice, moves):** `Telekinesis` volatile has no behaviour (no Ground immunity, no accuracy bypass, no Diglett/Gengar-Mega immunity). `Autotomize` has no weight effect.
- **N2 (cross-slice, abilities):** `PARENTALBOND`, `STALWART`, `HEALER`, `IMPOSTER`, `ILLUSION`, `STANCECHANGE`, `FORECAST` are enum entries with no behaviour; `AURAGUARD` is absent from the enum.
- **N3:** `ability_on_switch_out` returns early when the foe has Neutralizing Gas (abilities.rs:1056), skipping Zero to Hero and Gulp Missile, which are `cantsuppress` in Showdown. Neutralizing Gas holders (Weezing-Galar) are not in the legal list.
- **N4:** `Pokemon::deserialize` uses `.unwrap()` on `PokemonName::from_str`, `Abilities::from_str`, `Items::from_str`: any id absent from an enum (Leek, Aura Guard) crashes state loading rather than degrading.
- **N5:** The engine's Mega path (generate_instructions.rs:3916-3986) matches Champions `formeChange` for ability (sets ability and base_ability), stats (recomputed with level 50, IV 31, stat points) and types, and keeps the Mega permanent. It does not change weight (G1) and uses pre-Mega speed for the order (K8).
- **N6:** Not audited: behaviour of each Mega ability beyond checking that an enum entry and some code exist (abilities slice); Mega weights per row (only 12 verified, see finding 5); whether `Eiscue-Noice`/`Minior-Meteor` details are right, since both are unreachable in M-C.
