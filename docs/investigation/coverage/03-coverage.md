# 03 - Coverage of what we use vs poke-engine 0.0.48

Scripts: `scripts/03-dump_showdown.js` (Showdown Champions Dex dump) and `scripts/03-coverage.py` (everything else). Both ran on the laptop (ssh worked after it was rebooted; the desktop did no compute). Full tables: `03-coverage-species.csv`, `-moves.csv`, `-items.csv`, `-abilities.csv` (one row per used or legal entry; columns `used`, `legal`, `pool_share`, `live_share`, `ladder_share`, `in_gold`, `status`, `confidence`, `reason`), plus `03-coverage-gaplist.json` and `03-coverage-summary.json`.

## 1. Definitions and data sources

Verified (script output):
- Legal universe = Showdown `Dex.mod('champions')` entries with no `isNonstandard`: 392 species, 515 moves, 166 items, 317 abilities. This reproduces the briefing's legal numbers exactly. Showdown commit c0845f33 (`pcsa/.tools/vendor/pokemon-showdown`, same on both machines; the `research/pcsa-rl/node_modules` path in the brief does not exist).
- Engine = `research/pcsa-rl/poke-engine-eval/poke-engine` (0.0.48, f7b147f), analysed as the `champions` (bss) build: the `cfg!(feature=..)` if/else chains in the MOVES table are evaluated with only `champions` on (147 inactive duplicate inserts skipped) and the 32 `apply_champions_modifiers` overrides are applied.
- Usage weights, three views, all shares of team slots or observations:
  - **pool** (primary rank key): the training pool `rl2/data/ladder-20261116.json`, 488 live rosters (481 complete = 2,886 slots; 7 slots had no build source and were dropped). Each slot's build is drawn as `rl2.teams.Ladder` does: the exact build seen against that roster (1,025 slots), else the species' seen builds weighted by count (1,848), else catalog `top_builds.json` weighted 10^score (35), else frozen `form-sets.json` (13). The RECENT up-weight is not modelled. Share = expected fraction of slots containing the thing.
  - **live**: `pcsa/assets/sets/seen_builds.json`, 558 distinct opposing builds, 1,919 count-weighted observations, 795 source battles.
  - **ladder**: `top_builds.json`, 157,310 ladder battles, 209 species with usage, builds weighted by 10^score.
- A Mega form counts as its own species row (weight = share of slots holding its stone) and its Mega ability is added to the ability table, so species and ability shares can sum above 100%.
- "Used" = anything with weight in pool, live or the gold team, plus every build of a used species in the catalog, the seen builds of the 20 ladder files and the frozen sets. Result: 298 species (227 excluding Mega forms), 366 moves, 124 items, 167 abilities. This does not reproduce the earlier 252 / 377 / 147 / 163. I could not find the script that produced those, so their definition is unknown; mine are the same order of size. Names that did not map to a Showdown species (Vivillon patterns, Florges colours, Furfrou-Dandy; about 57 observations) fall back to the base species.
- Gold team: `runs/team-gold/team.txt` (Gholdengo, Garchomp-Mega-Z, Golisopod-Mega, Primarina, Mamoswine, Cinderace).

## 2. How status was decided, and how far to trust it

Engine facts (verified in source): `PokemonName` 1,469, `Choices` 885 (all have a MOVES row), `Abilities` 322, `Items` 254 variants. An unknown name does not error: abilities and moves silently become `NONE`, items become `UNKNOWNITEM` (default arms of `from_str`), so a (d) is silent.

- (a) specific handling. Moves: `Choices::X` appears in engine logic outside the table (multi-hit list, choice_effects, abilities, ...), or the table row uses a volatile status / side condition that engine code references. Abilities and items: `Abilities::X` / `Items::X` appears in logic outside the enum (the Mega table in `base_stats.rs` is excluded as data). Mega stones: a `mega_evolve_target` arm exists. Mega forms: arm exists and types, ability and base stats match Showdown. Species: `PokemonName::X` in logic.
- (b) plain data. In the enum/table and Showdown shows nothing needing code (move: only bp/accuracy/type/priority/flags/target; ability or item: no `on*` or `condition` handler in Showdown; species: no battle-only forme).
- (c) present but effect missing or wrong. Move: a Showdown field with no matching table field (`volatileStatus`, `secondary`, ...), a property needing code (ohko, slotCondition, weather, willCrit, ...) with no code reference, a table volatile/side-condition token that no code references, or a data mismatch (bp, type, category, priority, accuracy, a flag). Ability or item: Showdown has handlers and the engine has no code reference. Species: Mega data differs, or a battle-only forme with no switching code.
- (d) absent from the enum (or no table row).

Reliability. The "a" test is a reference count, not proof the effect is right (a reference can be a list entry); confidence is `medium`. Items have no (b) rows because every one of the 110 in-enum items used has some reference, so item risk is the (d) set. Moves marked (c) with confidence `low` or `medium` (a Showdown handler exists, e.g. Psychic Fangs onTryHit, while the table has the main effect) may be fine and need a code read. Data-mismatch rows are `high` confidence but measure fidelity against the Showdown Champions mod; flag mismatches (wind, slicing, punch) only matter when an ability or item keys off the flag. Behavioural correctness was not tested here (the damage differential in notes/poke-engine-eval.md did that for 220 triples). Code under `#[cfg(not(feature="champions"))]` inside abilities and items is not stripped from reference counts.

## 3. Totals (used entries)

| category | used | a | b | c | d | usage-weighted covered (a+b): pool / live / ladder |
|---|---|---|---|---|---|---|
| species (incl. Mega forms) | 298 | 72 | 218 | 8 | 0 | 91.4% / 92.0% / 93.8% |
| moves | 366 | 171 | 138 | 57 | 0 | 93.9% / 94.3% / 93.2% |
| items | 124 | 110 | 0 | 0 | 14 | 94.4% / 93.6% / 93.9% |
| abilities | 167 | 143 | 2 | 21 | 1 | 91.5% / 91.7% / 89.2% |

Whole legal universe: species a95/b288/c9; moves a220/b175/c120; items a144/d22; abilities a256/b8/c52/d1 (unused rows are in the CSVs with `used=0`). The empty "no ability/item" row in live data is ignored.

Of the 57 used (c) moves, 28 are low or medium handler-only guesses, 3 are low, the rest are data mismatches or missing fields. The 8 (c) species are Mega forms (Garchomp-Mega-Z, Golisopod-Mega, Lucario-Mega-Z, Absol-Mega-Z, Starmie-Mega, Drampa-Mega) plus Aegislash and Rotom.

## 4. Gold team gaps (verified)
- Garchomp-Mega-Z: engine ability Sand Force, Showdown Levitate. Golisopod-Mega: engine Emergency Exit, Showdown Tough Claws. Together about 7.5% of pool slots; Emergency Exit also has no code.
- Make It Rain: engine bp 95 / acc 100, Showdown mod 120 / 95.
- Sparkling Aria: the burn-cure status token (SPARKLINGARIA) exists in the enum only.

## 5. Top 30 gaps by pool usage weight (all categories)

Ranked by pool share. This is the start of the fix list; (c) rows with `low` or `medium` confidence need a read of the engine code before effort is spent.

| rank | cat | name | pool | live | ladder | gold | status | conf | note |
|---|---|---|---|---|---|---|---|---|---|
| 1 | species | Garchomp-Mega-Z | 4.48% | 3.75% | 0.00% | yes | c | high | Mega data differs: ability engine SANDFORCE vs SD levitate |
| 2 | moves | Sparkling Aria | 3.78% | 3.49% | 4.57% | yes | c | medium | status token(s) never referenced in engine logic: SPARKLINGARIA |
| 3 | species | Golisopod-Mega | 3.02% | 3.13% | 3.45% | yes | c | high | Mega data differs: ability engine EMERGENCYEXIT vs SD toughclaws |
| 4 | abilities | Emergency Exit | 3.02% | 3.13% | 3.45% | yes | c | medium | in enum, no code reference; Showdown handlers onEmergencyExit (+mega table only) |
| 5 | moves | Make It Rain | 2.47% | 1.56% | 3.15% | yes | c | high | data differs: basePower SD 120 vs engine 95; accuracy SD 95 vs engine 100 |
| 6 | species | Lucario-Mega-Z | 2.33% | 1.82% | 1.63% |  | c | high | Mega data differs: ability engine ADAPTABILITY vs SD auraguard |
| 7 | abilities | Aura Guard | 2.33% | 1.82% | 1.63% |  | d | high | not in Abilities enum (name silently becomes NONE) |
| 8 | items | Light Clay | 1.82% | 1.93% | 2.37% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| 9 | moves | Psychic Fangs | 1.71% | 0.83% | 1.50% |  | c | medium | Showdown handlers with no engine code: onTryHit |
| 10 | abilities | Cursed Body | 1.61% | 1.62% | 1.39% |  | c | medium | in enum, no code reference; Showdown handlers onDamagingHit |
| 11 | moves | Glaive Rush | 1.47% | 1.30% | 1.13% |  | c | high | Showdown effect without matching table field: volatile_status |
| 12 | moves | Curse | 1.47% | 1.25% | 0.52% |  | c | medium | status token(s) never referenced in engine logic: CURSE |
| 13 | moves | Healing Wish | 1.39% | 1.67% | 0.84% |  | c | high | needs special logic, none found: slotCondition,selfdestruct |
| 14 | items | Terrain Extender | 1.31% | 1.62% | 0.78% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| 15 | moves | Blizzard | 0.97% | 0.83% | 1.17% |  | c | high | data differs: flags missing in engine: wind |
| 16 | moves | Psychic Noise | 0.95% | 0.94% | 0.33% |  | c | high | Showdown effect without matching table field: secondaries |
| 17 | moves | Revival Blessing | 0.89% | 1.15% | 0.87% |  | c | high | Showdown effect without matching table field: flag:pivot; also needs logic: slotCondition |
| 18 | moves | Dire Claw | 0.89% | 0.63% | 1.60% |  | c | high | data differs: flags missing in engine: slicing |
| 19 | species | Aegislash | 0.86% | 0.68% | 1.37% |  | c | medium | has battle-only forme(s) Aegislash-Blade with no engine code switching to them |
| 20 | moves | King's Shield | 0.86% | 0.68% | 1.30% |  | c | medium | status token(s) never referenced in engine logic: KINGSSHIELD |
| 21 | abilities | Stance Change | 0.86% | 0.68% | 1.37% |  | c | medium | in enum, no code reference; Showdown handlers onModifyMove |
| 22 | items | Quick Claw | 0.82% | 0.78% | 0.25% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| 23 | moves | Double Shock | 0.80% | 0.99% | 1.11% |  | c | high | data differs: flags missing in engine: punch |
| 24 | moves | Leech Seed | 0.79% | 0.78% | 0.55% |  | c | high | data differs: flags extra in engine: powder |
| 25 | species | Starmie-Mega | 0.69% | 1.20% | 0.63% |  | c | high | Mega data differs: stats engine (60, 140, 105, 130, 105, 120) vs SD (60, 100, 105, 130, 105, 120) |
| 26 | abilities | Steadfast | 0.67% | 0.47% | 1.44% |  | c | medium | in enum, no code reference; Showdown handlers onFlinch (+mega table only) |
| 27 | moves | Sheer Cold | 0.65% | 0.47% | 0.19% |  | c | high | needs special logic, none found: ohko |
| 28 | items | Damp Rock | 0.60% | 0.83% | 1.16% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| 29 | moves | Temper Flare | 0.54% | 0.63% | 0.83% |  | c | medium | Showdown handlers with no engine code: basePowerCallback |
| 30 | abilities | Cute Charm | 0.53% | 0.68% | 0.27% |  | c | medium | in enum, no code reference; Showdown handlers onDamagingHit |

## 6. Top 50 of each category (pool / live / ladder share; gold = on our team)

### Top 50 species

| name | pool | live | ladder | gold | status | conf | note |
|---|---|---|---|---|---|---|---|
| Garchomp | 8.09% | 6.88% | 4.34% | yes | b | medium | enum entry only (stats/types are supplied by the caller) |
| Garchomp-Mega-Z | 4.48% | 3.75% | 0.00% | yes | c | high | Mega data differs: ability engine SANDFORCE vs SD levitate |
| Primarina | 4.29% | 3.96% | 4.57% | yes | b | medium | enum entry only (stats/types are supplied by the caller) |
| Archaludon | 3.40% | 3.23% | 3.56% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Hippowdon | 3.19% | 2.14% | 2.79% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Charizard | 3.09% | 3.70% | 1.40% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Golisopod | 3.02% | 3.13% | 3.45% | yes | b | medium | enum entry only (stats/types are supplied by the caller) |
| Golisopod-Mega | 3.02% | 3.13% | 3.45% | yes | c | high | Mega data differs: ability engine EMERGENCYEXIT vs SD toughclaws |
| Mimikyu | 3.02% | 2.50% | 2.64% |  | a | medium | species-specific code in abilities.rs |
| Lucario | 2.74% | 2.08% | 2.38% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Salamence | 2.54% | 2.45% | 4.43% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Gholdengo | 2.47% | 1.56% | 3.15% | yes | b | medium | enum entry only (stats/types are supplied by the caller) |
| Meowscarada | 2.43% | 1.82% | 2.20% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Salamence-Mega | 2.40% | 2.34% | 3.32% |  | a | high | mega_evolve_target arm SALAMENCE+SALAMENCITE |
| Corviknight | 2.37% | 2.29% | 2.59% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Lucario-Mega-Z | 2.33% | 1.82% | 1.63% |  | c | high | Mega data differs: ability engine ADAPTABILITY vs SD auraguard |
| Basculegion | 2.13% | 2.34% | 2.31% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Rillaboom | 2.09% | 2.08% | 2.38% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Dragonite | 2.02% | 2.03% | 0.80% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Dragonite-Mega | 1.92% | 1.88% | 0.80% |  | a | high | mega_evolve_target arm DRAGONITE+DRAGONINITE |
| Charizard-Mega-Y | 1.73% | 2.19% | 0.17% |  | a | high | mega_evolve_target arm CHARIZARD+CHARIZARDITEY |
| Glimmora | 1.71% | 1.30% | 1.47% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Metagross | 1.71% | 1.09% | 1.44% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Metagross-Mega | 1.71% | 1.09% | 1.44% |  | a | high | mega_evolve_target arm METAGROSS+METAGROSSITE |
| Indeedee | 1.51% | 1.88% | 1.43% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Baxcalibur | 1.47% | 1.30% | 1.45% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Greninja | 1.44% | 1.04% | 0.59% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Rotom-Wash | 1.41% | 1.77% | 1.13% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Charizard-Mega-X | 1.35% | 1.51% | 1.22% |  | a | high | mega_evolve_target arm CHARIZARD+CHARIZARDITEX |
| Volcarona | 1.34% | 1.09% | 1.15% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Blaziken | 1.27% | 1.72% | 0.94% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Blaziken-Mega | 1.27% | 1.72% | 0.94% |  | a | high | mega_evolve_target arm BLAZIKEN+BLAZIKENITE |
| Gyarados | 1.23% | 1.62% | 0.53% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Umbreon | 1.23% | 0.57% | 1.01% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Cinderace | 1.20% | 1.82% | 1.06% | yes | b | medium | enum entry only (stats/types are supplied by the caller) |
| Kingambit | 1.13% | 0.94% | 1.48% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Baxcalibur-Mega | 1.07% | 0.99% | 0.22% |  | a | high | mega_evolve_target arm BAXCALIBUR+BAXCALIBRITE |
| Staraptor | 1.06% | 1.25% | 0.38% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Staraptor-Mega | 1.06% | 1.25% | 0.38% |  | a | high | mega_evolve_target arm STARAPTOR+STARAPTITE |
| Ceruledge | 1.06% | 0.89% | 0.77% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Raichu | 1.06% | 0.57% | 1.00% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Raichu-Mega-Y | 1.06% | 0.57% | 1.00% |  | a | high | mega_evolve_target arm RAICHU+RAICHUNITEY |
| Glimmora-Mega | 1.02% | 0.73% | 0.00% |  | a | high | mega_evolve_target arm GLIMMORA+GLIMMORANITE |
| Gengar | 0.99% | 0.73% | 0.93% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Tyranitar | 0.99% | 0.47% | 0.34% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Ninetales-Alola | 0.93% | 1.04% | 1.26% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Clefable | 0.93% | 0.63% | 0.64% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Pawmot | 0.89% | 1.15% | 1.13% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Sneasler | 0.89% | 0.63% | 1.60% |  | b | medium | enum entry only (stats/types are supplied by the caller) |
| Toxapex | 0.86% | 0.89% | 0.92% |  | b | medium | enum entry only (stats/types are supplied by the caller) |

### Top 50 moves

| name | pool | live | ladder | gold | status | conf | note |
|---|---|---|---|---|---|---|---|
| Earthquake | 16.04% | 14.90% | 17.55% | yes | a | medium | referenced in damage_calc.rs,generate_instructions.rs |
| Swords Dance | 11.20% | 9.80% | 10.51% |  | a | medium | referenced in generate_instructions.rs |
| Stealth Rock | 10.26% | 8.75% | 11.40% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Flamethrower | 9.49% | 9.38% | 4.44% | yes | b | high | plain table row, nothing special in Showdown |
| Encore | 7.79% | 8.03% | 7.52% |  | a | medium | referenced in generate_instructions.rs,state.rs |
| Sucker Punch | 7.58% | 7.92% | 6.73% | yes | a | medium | referenced in choice_effects.rs,generate_instructions.rs |
| Roost | 6.65% | 6.83% | 5.90% |  | a | medium | referenced in choice_effects.rs |
| U-turn | 6.55% | 7.40% | 11.35% | yes | b | high | plain table row, nothing special in Showdown |
| Close Combat | 6.17% | 6.20% | 7.92% |  | b | high | plain table row, nothing special in Showdown |
| Draco Meteor | 5.88% | 5.89% | 6.94% | yes | b | high | plain table row, nothing special in Showdown |
| Thunderbolt | 5.83% | 5.32% | 6.21% | yes | a | medium | referenced in generate_instructions.rs |
| Moonblast | 5.64% | 5.63% | 7.18% | yes | b | high | plain table row, nothing special in Showdown |
| Shadow Ball | 5.59% | 4.95% | 6.62% | yes | b | high | plain table row, nothing special in Showdown |
| Protect | 5.52% | 5.78% | 4.54% |  | a | medium | referenced in damage_calc.rs |
| Dark Pulse | 5.19% | 4.59% | 3.23% |  | a | low | effect via status token(s) with engine code: FLINCH |
| Yawn | 5.13% | 4.48% | 5.86% |  | a | low | effect via status token(s) with engine code: YAWN; Showdown handlers onTryHit,condition |
| Dragon Dance | 5.07% | 5.26% | 4.98% |  | b | high | plain table row, nothing special in Showdown |
| Iron Head | 4.77% | 4.79% | 5.87% | yes | a | low | effect via status token(s) with engine code: FLINCH |
| Shadow Sneak | 4.64% | 4.33% | 5.34% |  | b | high | plain table row, nothing special in Showdown |
| Aqua Jet | 4.61% | 4.85% | 8.29% | yes | b | high | plain table row, nothing special in Showdown |
| Will-O-Wisp | 4.39% | 5.05% | 3.05% |  | b | high | plain table row, nothing special in Showdown |
| Dragon Pulse | 4.33% | 4.53% | 1.22% |  | b | high | plain table row, nothing special in Showdown |
| Play Rough | 4.15% | 3.75% | 3.35% |  | b | high | plain table row, nothing special in Showdown |
| Ice Beam | 4.08% | 4.59% | 3.38% |  | b | high | plain table row, nothing special in Showdown |
| Aura Sphere | 3.98% | 3.23% | 3.45% |  | b | high | plain table row, nothing special in Showdown |
| Sparkling Aria | 3.78% | 3.49% | 4.57% | yes | c | medium | status token(s) never referenced in engine logic: SPARKLINGARIA |
| Flash Cannon | 3.72% | 3.34% | 5.48% |  | b | high | plain table row, nothing special in Showdown |
| Outrage | 3.68% | 3.39% | 5.29% |  | a | low | effect via status token(s) with engine code: LOCKEDMOVE |
| Slack Off | 3.64% | 2.76% | 3.69% |  | b | high | plain table row, nothing special in Showdown |
| Sludge Wave | 3.59% | 2.81% | 2.79% |  | b | high | plain table row, nothing special in Showdown |
| Bullet Punch | 3.54% | 2.40% | 2.47% |  | b | high | plain table row, nothing special in Showdown |
| Nasty Plot | 3.36% | 2.55% | 5.66% | yes | b | high | plain table row, nothing special in Showdown |
| Thunder Punch | 3.34% | 3.02% | 2.12% |  | b | high | plain table row, nothing special in Showdown |
| Earth Power | 3.24% | 2.50% | 1.61% | yes | b | high | plain table row, nothing special in Showdown |
| Ice Punch | 3.24% | 3.39% | 3.23% |  | b | high | plain table row, nothing special in Showdown |
| Power Gem | 3.12% | 2.50% | 1.23% | yes | b | high | plain table row, nothing special in Showdown |
| Knock Off | 3.10% | 2.19% | 4.58% |  | a | medium | referenced in choice_effects.rs,generate_instructions.rs |
| Flip Turn | 3.10% | 3.75% | 4.51% |  | b | high | plain table row, nothing special in Showdown |
| Rock Tomb | 3.00% | 2.61% | 3.28% |  | b | high | plain table row, nothing special in Showdown |
| Surf | 2.90% | 3.28% | 1.33% |  | a | medium | referenced in abilities.rs |
| Wave Crash | 2.79% | 3.28% | 2.31% |  | b | high | plain table row, nothing special in Showdown |
| Fake Out | 2.73% | 2.50% | 2.67% |  | a | medium | referenced in choice_effects.rs,generate_instructions.rs,state.rs |
| Flare Blitz | 2.52% | 2.71% | 2.72% |  | a | medium | referenced in choice_effects.rs |
| Calm Mind | 2.48% | 2.81% | 2.04% | yes | b | high | plain table row, nothing special in Showdown |
| Make It Rain | 2.47% | 1.56% | 3.15% | yes | c | high | data differs: basePower SD 120 vs engine 95; accuracy SD 95 vs engine 100 |
| Flower Trick | 2.43% | 1.82% | 2.20% |  | a | medium | referenced in choices.rs(rest) |
| Leech Life | 2.43% | 2.55% | 1.58% | yes | b | high | plain table row, nothing special in Showdown |
| Iron Defense | 2.43% | 2.71% | 3.06% |  | b | high | plain table row, nothing special in Showdown |
| Body Press | 2.41% | 2.50% | 2.80% |  | a | medium | referenced in damage_calc.rs |
| Substitute | 2.38% | 2.08% | 1.61% |  | a | medium | referenced in choice_effects.rs,generate_instructions.rs |

### Top 50 items

| name | pool | live | ladder | gold | status | conf | note |
|---|---|---|---|---|---|---|---|
| Leftovers | 19.14% | 18.08% | 9.72% | yes | a | medium | referenced in generate_instructions.rs,items.rs |
| Life Orb | 12.42% | 12.82% | 7.00% |  | a | medium | referenced in items.rs |
| Focus Sash | 5.87% | 6.20% | 10.99% | yes | a | medium | referenced in generate_instructions.rs |
| Choice Scarf | 5.45% | 6.15% | 13.36% | yes | a | medium | referenced in generate_instructions.rs,items.rs |
| Sitrus Berry | 4.80% | 4.64% | 13.08% |  | a | medium | referenced in items.rs |
| Rocky Helmet | 4.56% | 4.59% | 3.08% |  | a | medium | referenced in items.rs |
| Garchompite Z | 4.48% | 3.75% | 0.00% | yes | a | high | mega stone arm in mega_evolve_target GARCHOMP |
| Golisopite | 3.02% | 3.13% | 3.45% | yes | a | high | mega stone arm in mega_evolve_target GOLISOPOD |
| Salamencite | 2.40% | 2.34% | 3.32% |  | a | high | mega stone arm in mega_evolve_target SALAMENCE |
| Lucarionite Z | 2.33% | 1.82% | 1.63% |  | a | high | mega stone arm in mega_evolve_target LUCARIO |
| Dragoninite | 1.92% | 1.88% | 0.80% |  | a | high | mega stone arm in mega_evolve_target DRAGONITE |
| Light Clay | 1.82% | 1.93% | 2.37% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| Charizardite Y | 1.73% | 2.19% | 0.17% |  | a | high | mega stone arm in mega_evolve_target CHARIZARD |
| Metagrossite | 1.71% | 1.09% | 1.44% |  | a | high | mega stone arm in mega_evolve_target METAGROSS |
| Charizardite X | 1.35% | 1.51% | 1.22% |  | a | high | mega stone arm in mega_evolve_target CHARIZARD |
| Terrain Extender | 1.31% | 1.62% | 0.78% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| Blazikenite | 1.27% | 1.72% | 0.94% |  | a | high | mega stone arm in mega_evolve_target BLAZIKEN |
| Air Balloon | 1.22% | 0.83% | 3.74% | yes | a | medium | referenced in items.rs,state.rs |
| Baxcalibrite | 1.07% | 0.99% | 0.22% |  | a | high | mega stone arm in mega_evolve_target BAXCALIBUR |
| Staraptite | 1.06% | 1.25% | 0.38% |  | a | high | mega stone arm in mega_evolve_target STARAPTOR |
| Raichunite Y | 1.06% | 0.57% | 1.00% |  | a | high | mega stone arm in mega_evolve_target RAICHU |
| Glimmoranite | 1.02% | 0.73% | 0.00% |  | a | high | mega stone arm in mega_evolve_target GLIMMORA |
| Black Glasses | 0.94% | 0.83% | 1.62% |  | a | medium | referenced in items.rs |
| Greninjite | 0.83% | 0.63% | 0.54% |  | a | high | mega stone arm in mega_evolve_target GRENINJA |
| Clefablite | 0.83% | 0.52% | 0.48% |  | a | high | mega stone arm in mega_evolve_target CLEFABLE |
| Quick Claw | 0.82% | 0.78% | 0.25% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| Gyaradosite | 0.74% | 0.99% | 0.00% |  | a | high | mega stone arm in mega_evolve_target GYARADOS |
| Normal Gem | 0.72% | 0.57% | 0.78% |  | a | medium | referenced in items.rs |
| Shuca Berry | 0.70% | 0.57% | 0.40% |  | a | medium | referenced in items.rs |
| Swampertite | 0.69% | 0.99% | 0.00% |  | a | high | mega stone arm in mega_evolve_target SWAMPERT |
| Starminite | 0.69% | 1.20% | 0.63% |  | a | high | mega stone arm in mega_evolve_target STARMIE |
| Delphoxite | 0.65% | 0.89% | 0.62% |  | a | high | mega stone arm in mega_evolve_target DELPHOX |
| Damp Rock | 0.60% | 0.83% | 1.16% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| Tyranitarite | 0.56% | 0.26% | 0.13% |  | a | high | mega stone arm in mega_evolve_target TYRANITAR |
| Gengarite | 0.54% | 0.42% | 0.00% |  | a | high | mega stone arm in mega_evolve_target GENGAR |
| Lopunnite | 0.51% | 0.73% | 0.45% |  | a | high | mega stone arm in mega_evolve_target LOPUNNY |
| Blastoisinite | 0.51% | 0.26% | 0.97% |  | a | high | mega stone arm in mega_evolve_target BLASTOISE |
| Lum Berry | 0.48% | 0.31% | 0.25% |  | a | medium | referenced in generate_instructions.rs,items.rs |
| Never-Melt Ice | 0.48% | 0.31% | 0.16% |  | a | medium | referenced in items.rs |
| Lucarionite | 0.41% | 0.26% | 0.75% |  | a | high | mega stone arm in mega_evolve_target LUCARIO |
| Venusaurite | 0.38% | 0.47% | 0.24% |  | a | high | mega stone arm in mega_evolve_target VENUSAUR |
| Banettite | 0.34% | 0.26% | 0.51% |  | a | high | mega stone arm in mega_evolve_target BANETTE |
| Gardevoirite | 0.34% | 0.31% | 0.09% |  | a | high | mega stone arm in mega_evolve_target GARDEVOIR |
| Scizorite | 0.33% | 0.16% | 0.45% |  | a | high | mega stone arm in mega_evolve_target SCIZOR |
| Meganiumite | 0.31% | 0.36% | 0.49% |  | a | high | mega stone arm in mega_evolve_target MEGANIUM |
| Smooth Rock | 0.30% | 0.10% | 0.10% |  | d | high | not in Items enum (becomes UNKNOWNITEM) |
| Excadrite | 0.30% | 0.31% | 0.00% |  | a | high | mega stone arm in mega_evolve_target EXCADRILL |
| Absolite Z | 0.25% | 0.21% | 0.56% |  | a | high | mega stone arm in mega_evolve_target ABSOL |
| Mawilite | 0.24% | 0.36% | 0.30% |  | a | high | mega stone arm in mega_evolve_target MAWILE |
| Slowbronite | 0.24% | 0.10% | 0.06% |  | a | high | mega stone arm in mega_evolve_target SLOWBRO |

### Top 50 abilities

| name | pool | live | ladder | gold | status | conf | note |
|---|---|---|---|---|---|---|---|
| Rough Skin | 8.09% | 6.88% | 4.34% | yes | a | medium | referenced in abilities.rs |
| Levitate | 7.91% | 8.29% | 3.36% | yes | a | medium | referenced in abilities.rs,state.rs |
| Tough Claws | 6.59% | 6.20% | 6.52% | yes | a | medium | referenced in abilities.rs |
| Torrent | 5.35% | 5.32% | 5.37% | yes | a | medium | referenced in abilities.rs |
| Intimidate | 5.35% | 6.20% | 5.64% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Sand Stream | 4.74% | 2.87% | 3.27% |  | a | medium | referenced in abilities.rs |
| Protean | 4.54% | 3.39% | 3.30% |  | a | medium | referenced in abilities.rs |
| Multiscale | 3.94% | 3.91% | 1.60% |  | a | medium | referenced in abilities.rs |
| Adaptability | 3.53% | 3.39% | 3.28% |  | a | medium | referenced in abilities.rs |
| Blaze | 3.44% | 4.22% | 2.03% |  | a | medium | referenced in abilities.rs |
| Stamina | 3.26% | 3.07% | 2.95% |  | a | medium | referenced in abilities.rs |
| Speed Boost | 3.05% | 4.53% | 2.65% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Emergency Exit | 3.02% | 3.13% | 3.45% | yes | c | medium | in enum, no code reference; Showdown handlers onEmergencyExit (+mega table only) |
| Disguise | 3.02% | 2.50% | 2.64% |  | a | medium | referenced in abilities.rs |
| Thermal Exchange | 2.54% | 2.29% | 1.67% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Good as Gold | 2.47% | 1.56% | 3.15% | yes | a | medium | referenced in abilities.rs,choice_effects.rs |
| Aerilate | 2.47% | 2.34% | 3.35% |  | a | medium | referenced in abilities.rs |
| Aura Guard | 2.33% | 1.82% | 1.63% |  | d | high | not in Abilities enum (name silently becomes NONE) |
| Pressure | 2.15% | 2.19% | 2.38% |  | a | medium | referenced in generate_instructions.rs |
| Grassy Surge | 2.09% | 2.08% | 2.38% |  | a | medium | referenced in abilities.rs |
| Justified | 1.92% | 1.56% | 0.59% |  | a | medium | referenced in abilities.rs |
| Regenerator | 1.87% | 1.82% | 1.98% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Drought | 1.84% | 2.45% | 0.41% |  | a | medium | referenced in abilities.rs |
| Inner Focus | 1.75% | 0.99% | 1.42% |  | a | medium | referenced in abilities.rs,state.rs |
| Clear Body | 1.74% | 1.15% | 1.55% |  | a | medium | referenced in abilities.rs,generate_instructions.rs,state.rs |
| Toxic Debris | 1.71% | 1.30% | 1.47% |  | a | medium | referenced in abilities.rs |
| Sharpness | 1.66% | 1.67% | 1.81% |  | a | medium | referenced in abilities.rs |
| Cursed Body | 1.61% | 1.62% | 1.39% |  | c | medium | in enum, no code reference; Showdown handlers onDamagingHit |
| Snow Warning | 1.57% | 1.51% | 1.70% |  | a | medium | referenced in abilities.rs |
| Mold Breaker | 1.56% | 1.93% | 0.76% |  | a | medium | referenced in abilities.rs,damage_calc.rs |
| Psychic Surge | 1.54% | 1.88% | 1.73% |  | a | medium | referenced in abilities.rs |
| Unaware | 1.44% | 1.15% | 1.43% |  | a | medium | referenced in abilities.rs,damage_calc.rs |
| Magic Bounce | 1.43% | 0.99% | 1.44% |  | a | medium | referenced in abilities.rs |
| Flame Body | 1.37% | 1.15% | 0.96% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Prankster | 1.32% | 1.25% | 1.89% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| Huge Power | 1.22% | 1.93% | 1.17% |  | a | medium | referenced in abilities.rs |
| Libero | 1.20% | 1.82% | 1.06% | yes | a | medium | referenced in abilities.rs |
| Contrary | 1.20% | 1.35% | 0.64% |  | a | medium | referenced in abilities.rs,generate_instructions.rs |
| No Guard | 1.20% | 0.68% | 1.13% |  | a | medium | referenced in abilities.rs |
| Supreme Overlord | 1.13% | 0.94% | 1.39% |  | a | medium | referenced in abilities.rs |
| Mega Launcher | 1.10% | 0.63% | 1.88% |  | a | medium | referenced in abilities.rs |
| Static | 1.10% | 0.63% | 0.80% |  | a | medium | referenced in abilities.rs |
| Technician | 1.08% | 0.63% | 1.12% |  | a | medium | referenced in abilities.rs |
| Iron Fist | 1.03% | 1.30% | 1.37% |  | a | medium | referenced in abilities.rs |
| Drizzle | 0.96% | 1.30% | 1.42% |  | a | medium | referenced in abilities.rs |
| Unburden | 0.96% | 0.73% | 1.71% |  | a | medium | referenced in generate_instructions.rs |
| Weak Armor | 0.90% | 0.94% | 1.15% |  | a | medium | referenced in abilities.rs |
| Stance Change | 0.86% | 0.68% | 1.37% |  | c | medium | in enum, no code reference; Showdown handlers onModifyMove |
| Pixilate | 0.78% | 0.78% | 0.84% |  | a | medium | referenced in abilities.rs |
| Thick Fat | 0.75% | 1.51% | 0.52% | yes | a | medium | referenced in abilities.rs |

