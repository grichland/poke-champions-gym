# Phone battle-log inventory against LIVE.md section 4

Read-only counting, 17 Nov 2026. Replaces the Showdown-text inventory (`battle-log-inventory.md`) as the evidence for what the phone game showed. Contract: `contracts/LIVE.md` v0.1.

## Source and counts

- Source used: `pcsa/output/runs/*/battle-*/log.jsonl`. One JSON line per raw battle-log row, keys `battle_id, turn, index, text, sprite`. `text` is the OCR of one log row exactly as read from the phone screen, before translation (`log_rows.py` later splits the leading category). `turn` and `index` come from the collector, not from the screen.
- Other places hold the same rows: `log.txt` and `battle_log.txt` (a rendering of the same rows grouped under "TurnN:" headers plus controller comments), `battle.json` and `battle_final.txt`. I used `log.jsonl` because it is the unrendered row store; the others add only turn headers and `# controller:` notes (not game text). `ocr_audit.json` and `coverage.json` hold reader diagnostics, not extra rows. Screenshots (`log-frames/*.png`) are the images the rows were read from; I did not re-read them.
- Runs under `output/runs`: 47. Per-battle `log.jsonl` files: 549. One is empty, so **548 battles with rows** (all 548 battle ids are distinct).
- **Raw rows: 37420** (per battle: min 0, median 62, max 228).
- Rows with the same text as the row before in the same turn: 32 (0.1%). The reader merges overlapping scroll frames; some repeats are real (two identical events in a row), some are merge repeats. They are counted as rows here, not removed.
- Row format: some rows start with a category and ": ": `Move`, `Move +N Priority` (also `-N`), `HP Lost`, `HP Restored`, `Held Item`, `Ability`, `Ability Changed`, `Fainted`, `Mega Evolution`. Rows with no category are plain sentences (stat changes, status, weather, withdrawals, effectiveness...). Rows by category: no category 15625, HP Lost 6897, Move 6287, HP Restored 2151, Held Item 2025, Fainted 1595, Ability 1170, Mega Evolution 802, Ability Changed 4 ("Move +N Priority" and "Move -N Priority" rows, 864 in total, are counted under Move in the table).
- Templating: Pokemon, move, item and ability names are matched by id against `assets/game/vocab.json` and `species.json`; trainer names (the opponents write their own names, read by OCR as anything from `<` to a word) are `{TRAINER}`; numbers are `{N}`; stat names are `{STATS}`; types in "-type" text are `{TYPE}`. Arrow spacing is normalised. The text is otherwise kept as read, so OCR variants (`..` vs `...`, missing `!`) stay separate templates. A name the vocab does not hold stays in the template as plain text (those are the OCR-damaged names, see UNREADABLE).
- Templates: 306 (PL-001 to PL-306), ordered by row count then first appearance.

| class | templates | rows | share of rows |
|---|---|---|---|
| KIND | 230 | 30280 | 80.92% |
| NO-KIND | 35 | 4602 | 12.30% |
| AMBIGUOUS | 11 | 2478 | 6.62% |
| NOT-BATTLE | 1 | 3 | 0.01% |
| UNREADABLE | 29 | 57 | 0.15% |

`KIND` means the row maps to a section 4 kind (the kind and the fields it gives are in the table; "fields missing" are noted). No template is NOT-BATTLE except one UI string. Sections that follow give the answers, then the NO-KIND and AMBIGUOUS lists, then the full table.

## Limits of this inventory

- Only these 548 battles: ranked singles on the Master Ball and neighbouring tiers, own team from 6 own rosters, mostly the team the user fielded at the time. Moves, items and abilities that did not occur in them are not in the table. "Never seen" below always means "never seen in 548 battles / 37420 rows", not "the game never shows it".
- The side (own or opposing) is not stored in the row. It is read here from the words "The opposing" and from the row type (see HP below); the Held Item, Ability and HP rows carry only a name.
- Where the table says what mechanic a row shows (Charge, Poltergeist, Red Card...), that is my reading of the wording and the neighbouring rows; the phone game does not name the move.
- Battle end, forfeit and refusals are not log rows (see answers). The forfeit and result screens were read by `pcsa/phone` and only their outcome was saved; the screen text was not kept in the run files.
- Opponent trainer names are real third-party names read by OCR; they are not reproduced here.

## Answers to the section 4 questions

### Things section 5 says never appear

- Critical hits: no row contains "critical" or "crit" (0 rows, 548 battles). Never seen. The game shows no critical-hit text in these recordings.
- Effectiveness: shown, as rows with no category and no Pokemon named except for immunity. PL-008 (1296 rows, 461 battles), PL-012 (830 rows, 357 battles), PL-024 (271 rows, 188 battles), PL-034 (139 rows, 113 battles), PL-046 (91 rows, 58 battles), PL-076 (40 rows, 37 battles), PL-088 (31 rows, 26 battles), PL-091 (30 rows, 29 battles), PL-099 (24 rows, 22 battles), PL-119 (17 rows, 17 battles), PL-206 (3 rows, 3 battles), PL-212 (3 rows, 3 battles), PL-215 (3 rows, 1 battles), PL-267 (1 rows, 1 battles), PL-283 (1 rows, 1 battles), PL-293 (1 rows, 1 battles), PL-303 (1 rows, 1 battles). Wording: "It's super effective!", "It's not very effective.." (two or three dots), "It's mostly ineffective..", "It's extremely effective!", "It doesn't affect {MON}.." and "It's a one-hit KO!". The row does not say which Pokemon was hit; position (the row after the Move row) is the only link. "Mostly ineffective" and "extremely effective" are the x0.25 and x4 wordings.
- Protect activation: shown as "The opposing {MON} protected itself!" (PL-032 (150 rows, 49 battles)). The own-side form (without "The opposing") appears 0 times, so only the opposing Pokemon's Protect is shown (both sides' Protect moves exist as Move rows). This contradicts section 5. The Protect move itself also shows in a `Move +N Priority` row.
- Who attacked on a miss: the row names only the Pokemon that avoided (PL-052 (75 rows, 60 battles), PL-070 (45 rows, 43 battles)). No row of the form "{X}'s attack missed!" in 548 battles. So section 5 holds here: the attacker is not named.
- PP changes: no row mentions PP (0 rows, 548 battles). Never seen. Eerie Spell, Spite and Grudge text would have shown here; none occurred, so this is not proof about those moves.
- Perish counts: shown. "{MON}'s perish count fell to {N}!" (PL-161 (7 rows, 3 battles), PL-204 (3 rows, 2 battles), PL-305 (1 rows, 1 battles)). The song announcement "All Pokemon that heard the song will faint in three turns!" names no one (PL-232 (2 rows, 2 battles), PL-306 (1 rows, 1 battles)). This contradicts section 5 ("how many turns... remain" is shown for Perish Song).
- Frisk reveals: shown. "The opposing {MON} frisked {MON} and found its {ITEM}!" (PL-179 (6 rows, 6 battles)), preceded by an `Ability` row for the Frisk holder. Forewarn: never seen (no row with "alerted", "revealed", "Forewarn"; the ability could not occur as a row either: the `Ability: {MON}'s Forewarn` row never occurs).
- "Hit N times": shown, "The Pokemon was hit {N} times!" (PL-037 (123 rows, 93 battles), PL-208 (3 rows, 3 battles)). It names no Pokemon; the plural "time" form also appears. LIVE.md `hits` matches (side and mon omitted).
- Turn marker: no row text is a turn marker ("Turn" appears only inside move names such as U-turn and Flip Turn). The collector's own `turn` field is stored beside each row, and `log.txt` prints "TurnN:" headers from it. Whether the game's log screen shows turn headings was not determined from the rows. Never seen as a row in 548 battles.

### How HP is shown

- Every HP change is an `HP Lost` or `HP Restored` row of the form "{MON}'s HP: A -> B", with A the value before and B the value after, never the amount lost or restored. PL-001 (3577 rows, 542 battles), PL-002 (3317 rows, 524 battles), PL-010 (1118 rows, 346 battles), PL-011 (1033 rows, 282 battles) (the first four; the rest are OCR variants of the same two shapes).
- Own Pokemon: whole numbers (A and B are current HP, e.g. "195 -> 154"). The row carries no max HP. Opposing Pokemon: percent ("100% -> 70%"). Check on side-resolved rows (name matched against own "Go!" rows and opposing rows in the same battle): own with numbers 2913, own with percent 4; opposing with percent 3209 and opposing with numbers 13 (the 17 odd ones are Pokemon of the same species on both sides, or OCR). So section 1 rule 6 (own exact, opponent percent) matches what the phone shows; `maxhp` is not shown in any row.
- A Pokemon on the field is never shown with its HP at entry: "Go! {MON}!" and "{TRAINER} sent out {MON}!" carry no number or percent (PL-005 (2155 rows, 548 battles), PL-007 (1873 rows, 531 battles)). So `enter` cannot carry `hp`/`pct`; the first HP value is the "before" number of the first HP row.
- The cause of an HP change is a separate row next to the HP row, not part of it: the HP row follows a `Move` row (damage), a `Held Item` row (Leftovers, Life Orb, berries), or a plain row such as "was hurt by its burn!", "is buffeted by the sandstorm!", "Pointed stones dug into {MON}!", "had its energy drained!". See the table (`hp` cause rows).

### Mega Evolution, forms, weather, terrain, screens, hazards, items, abilities

- Mega Evolution: two rows. First a plain row "{MON}'s {ITEM} is reacting to {TRAINER}'s Omni Ring!" (names the stone: PL-021 (385 rows, 385 battles), PL-022 (383 rows, 383 battles)), then `Mega Evolution: {MON} has Mega Evolved into {MON}!` (PL-019 (407 rows, 407 battles), PL-020 (393 rows, 393 battles)). Both name the base species and the Mega form; the stone is the item in the first row. Rows exist for both sides ("The opposing" prefix).
- Other form changes: "The opposing {MON}'s disguise was busted!" (PL-085 (32 rows, 32 battles), PL-126 (14 rows, 14 battles)), "{MON} underwent a heroic transformation!" (PL-276 (1 rows, 1 battles)), "transformed into {MON}!" (PL-174 (6 rows, 5 battles), Ditto or form change: ambiguous). Type changes: "{MON} transformed into the {TYPE} type!" (221 rows). Temporary forms: none identified.
- Weather: start and end shown as plain sentences: "A sandstorm kicked up!" (PL-047 (88 rows, 66 battles)), "The sandstorm subsided." (PL-062 (56 rows, 48 battles)), rain (PL-089 (31 rows, 22 battles) / PL-128 (14 rows, 14 battles)), sun (PL-065 (53 rows, 37 battles) / PL-090 (31 rows, 28 battles)), snow (PL-083 (32 rows, 22 battles) / PL-106 (20 rows, 18 battles)). **Continuing-weather lines never appear** ("The sandstorm is raging." and similar: 0 rows). Damage from weather appears as a cause row ("{MON} is buffeted by the sandstorm!", PL-036 (129 rows, 39 battles), PL-043 (101 rows, 46 battles)). The weather setter is not named in the weather row; an `Ability` row (for example Snow Warning) precedes it. Hail and Delta Stream text: never seen. Sun/rain/snow/sand only.
- Terrain: Grassy start/end (PL-075 (40 rows, 30 battles) / PL-105 (20 rows, 20 battles)), Psychic start/end (PL-078 (39 rows, 35 battles) / PL-116 (18 rows, 18 battles)); Electric and Misty Terrain never seen. Trick Room: start "The opposing {MON} twisted the dimensions!" (PL-148 (10 rows, 10 battles)), end (PL-192 (5 rows, 5 battles)). Gravity, Magic Room, Wonder Room: never seen.
- Screens: start "Light Screen/Reflect/Aurora Veil made the opposing side stronger against ... moves!" (PL-110 (19 rows, 13 battles), PL-114 (18 rows, 16 battles), PL-115 (18 rows, 16 battles)); end "The opposing side's {screen} wore off!" (PL-139 (11 rows, 10 battles), PL-152 (9 rows, 6 battles), PL-164 (7 rows, 7 battles)). Only the opposing side's screens occur (own-side wording never seen: "Your side" screen start/end, 0 rows). Tailwind: start "A tailwind started blowing on the opposing side!" (PL-244 (2 rows, 2 battles)), end "The opposing side's tailwind petered out!" (PL-292 (1 rows, 1 battles)). Safeguard, Mist: never seen.
- Hazards: start rows exist for Stealth Rock on both sides (PL-053 (72 rows, 72 battles), PL-183 (5 rows, 5 battles)), Spikes (PL-125 (15 rows, 11 battles)), Toxic Spikes (PL-104 (21 rows, 17 battles)), Sticky Web (PL-248 (2 rows, 2 battles)), all "on your side" wording except Stealth Rock. Removal rows ("The pointed stones disappeared...", "The spikes disappeared...") never seen. Damage on switch-in is a cause row ("Pointed stones dug into {MON}!", PL-033 (147 rows, 64 battles), PL-144 (10 rows, 5 battles); "was hurt by the spikes!", PL-129 (14 rows, 8 battles)); "was caught in a sticky web!" (PL-201 (4 rows, 2 battles)) shows Sticky Web acting.
- Item use and removal: the `Held Item: {MON}'s {ITEM}` row (PL-006 (2006 rows, 486 battles)) is an announcement that an item acted or is held; it is followed by an HP Restored/Lost row for Leftovers, berries and Life Orb. Plain rows name items used up or removed: "hung on using its {ITEM}!" (Focus Sash), "{MON}'s {ITEM} popped!" (Air Balloon), "held up its {ITEM} against {MON}!" (Red Card), "is switched out with the {ITEM}!" (Eject Button), "returned its stats to normal using its {ITEM}!" (White Herb), "knocked off {MON}'s {ITEM}!", "frisked ... found its {ITEM}", "harvested a {ITEM}!". Items passed or swapped (Trick, Switcheroo, Thief): never seen. Scarf and other held items that nothing makes visible are never shown (consistent with section 5).
- Ability reveal: `Ability: {MON}'s {ABILITY}` (PL-009 (1160 rows, 457 battles)) when an ability triggers or enters (Intimidate, Stamina...), and plain sentences for some abilities ("is exerting its pressure!", "breaks the mold!", "is radiating a fairy aura!"). Ability change: `Ability Changed: {MON}: {OLD} -> {NEW}` (PL-197 (4 rows, 4 battles)). Ability changes by Skill Swap, Mummy, Wandering Spirit: never seen as a row. In the 4 Ability Changed rows the old ability was Trace.

### Battle end, forfeit, refusal

- Battle end: no log row says the battle ended or who won (0 rows containing win/lose/defeat text, 548 battles). Of the 549 recorded battles the outcome comes from the result screen (427), a forfeit screen (71), a final-message screen (10), the rating change (13) or is unknown (28); that is `battle_log.txt` metadata, not a log row.
- Forfeit: not a log row (0 rows with "forfeit"). It is a separate message screen (`FORFEIT_MSG`) that `pcsa/phone` recognises by the word "forfeit"; its text was not kept, only that 71 battles ended that way (opponent forfeit).
- Refused choice: no game text exists in the recordings. The controller infers a refusal when the action screen comes back in the same turn (`move_peek.refused_moves`). So `refused.shown` has no recorded value: undetermined.

## Section 4 and 5 against the recordings

### Section 4 kinds and fields never produced by any recorded row

- `turn` (`n`): no row. The collector supplies `turn`; the game's own turn heading was not seen.
- `enter` fields `hp`, `maxhp`, `pct`: never shown in the entry rows.
- `hp.maxhp`: never shown in HP rows (own current HP only).
- `fail` `side`, `mon`: "But it failed!" (PL-042 (106 rows, 74 battles)) names nobody. Rows for failed secondary effects name the mon (already poisoned/drowsy, already has a substitute, HP is full).
- `hits` `side`, `mon`: never named (as the contract already says).
- `volatile_hit` for confusion: "It hurt itself in its confusion!" (PL-186 (5 rows, 4 battles)) names nobody.
- `cant` reason `frz`: no "is frozen solid!" row without status (frozen solid occurs only as the status start "was frozen solid!"); `cant` reasons seen: asleep, paralyzed, flinch, recharge, taunt.
- `status` `from` and `cure` `from`: the row never names the cause except item cures ("{MON}'s {ITEM} cured its poisoning!/woke it up!/defrosted it!").
- `boost_reset` what=`copy` (Psych Up) and `negative` by name: only Haze ("All stat changes were eliminated!") and White Herb ("returned its stats to normal using its {ITEM}!") seen.
- `weather` `from`, `of`: not in the weather row; the cause is a separate `Ability` row or a `Move` row before it.
- `field` Electric Terrain, Misty Terrain, Gravity, Magic Room, Wonder Room: never seen. `side` for own screens, Safeguard, Mist, hazard removal (including Rapid Spin/Defog clearing): never seen.
- `item` what=`received` from passing (Trick, Thief): never seen; "harvested a {ITEM}!" is the only "received" row.
- `transform`: no unambiguous row; "transformed into {MON}!" (6 rows) could be Transform or a form change.
- `revive` `hp`/`pct`: the revive row names the Pokemon only (PL-122 (16 rows, 16 battles)).
- `refused`: no row; `end`: no row; `battle_start`/`preview` rows: not log rows (not examined here).
- `prepare` for most two-turn moves: only wordings that match Fly, Dig, Phantom Force/Shadow Force ("vanished instantly!"), Solar Beam ("absorbed light!"), Electro Shot ("absorbed electricity!") seen; the move is not named in the row.

### Section 5 "never appears" items that a recorded row does show

- Protect activation: shown (PL-032 (150 rows, 49 battles)).
- Perish count: shown (PL-161 (7 rows, 3 battles), PL-204 (3 rows, 2 battles), PL-305 (1 rows, 1 battles)).
- An unused opponent move: no (Forewarn never seen; Protect, Poltergeist and Frisk show items, not moves). The opponent's Choice Scarf: shown only when Frisk or Poltergeist reveals it (PL-179 (6 rows, 6 battles), PL-137 (11 rows, 10 battles)), so "unrevealed" still holds. Poltergeist reveal is "is about to be attacked by its {ITEM}!".
- The move name when a Pokemon is trapped or disabled: shown ("{MON}'s {MOVE} was disabled!", "was freed from {MOVE}!", "is hurt by {MOVE}!"). Section 5 does not list this, so it is not a conflict.
- Items an opponent holds are shown when they act (Held Item row, Focus Sash, Air Balloon, Red Card...). Section 5 only says unrevealed items never appear, so this is consistent.
- Critical hit, damage roll, opponent stats/nature/points, opponent PP, remaining sleep/confusion/trap/lock turns, who attacked on a miss: never seen in these rows. Consistent with section 5.

## NO-KIND list (battle information that fits no section 4 kind)

| id | rows | battles | category | template | what it carries |
|---|---|---|---|---|---|
| PL-008 | 1296 | 461 | - | `It's super effective!` | effectiveness: super effective |
| PL-012 | 830 | 357 | - | `It's not very effective..` | effectiveness: not very effective |
| PL-015 | 693 | 353 | - | `{MON}, come back!` | own withdrawal |
| PL-016 | 680 | 334 | - | `{TRAINER} withdrew {MON}!` | withdrawal of a Pokemon (side only from the trainer name) |
| PL-024 | 271 | 188 | - | `It's not very effective...` | effectiveness: not very effective |
| PL-032 | 150 | 49 | - | `The opposing {MON} protected itself!` | Protect activation (names the protected mon) |
| PL-034 | 139 | 113 | - | `It's extremely effective!` | effectiveness: extremely effective (x4) |
| PL-035 | 138 | 99 | - | `The opposing {MON} went back to {TRAINER}!` | withdrawal after a pivot move |
| PL-046 | 91 | 58 | - | `It's mostly ineffective..` | effectiveness: mostly ineffective (x0.25) |
| PL-064 | 54 | 47 | - | `{MON} went back to {TRAINER}!` | withdrawal after a pivot move |
| PL-069 | 47 | 8 | - | `{MON}'s {STATS} won't go any higher!` | stat already at +6, no change |
| PL-076 | 40 | 37 | - | `It doesn't affect {MON}..` | effectiveness: immune |
| PL-088 | 31 | 26 | - | `It doesn't affect the opposing {MON}..` | effectiveness: immune |
| PL-091 | 30 | 29 | - | `It doesn't affect {MON}...` | effectiveness: immune |
| PL-099 | 24 | 22 | - | `It doesn't affect the opposing {MON}.` | effectiveness: immune |
| PL-119 | 17 | 17 | - | `It's mostly ineffective...` | effectiveness: mostly ineffective (x0.25) |
| PL-121 | 17 | 4 | - | `The opposing {MON}'s {STATS} won't go any higher!` | stat already at +6, no change |
| PL-140 | 11 | 10 | - | `The battlers shared their pain!` | Pain Split (no Pokemon named) |
| PL-169 | 6 | 4 | - | `The opposing {MON} is protected by the Psychic Terrain!` | terrain blocked a priority move |
| PL-191 | 5 | 5 | - | `The opposing {MON}'s illusion wore off!` | Illusion ended |
| PL-201 | 4 | 2 | - | `{MON} was caught in a sticky web!` | Sticky Web acted on switch-in (side effect acted) |
| PL-206 | 3 | 3 | - | `It doesn't affect the opposing {MON}` | effectiveness: immune |
| PL-212 | 3 | 3 | - | `It doesn't affect the opposing {MON}...` | effectiveness: immune |
| PL-215 | 3 | 1 | - | `It doesn't affect {MON}.` | effectiveness: immune |
| PL-217 | 3 | 3 | - | `{MON} has no moves left that it can use!` | Struggle forced |
| PL-220 | 3 | 3 | - | `The opposing {MON} went back to!` | withdrawal after a pivot move (trainer name unread) |
| PL-232 | 2 | 2 | - | `All Pokémon that heard the song will faint in three turns!` | Perish Song start (no Pokemon named) |
| PL-239 | 2 | 2 | - | `{TRAINER} withdrew {MON}` | withdrawal of a Pokemon (side only from the trainer name) |
| PL-249 | 2 | 1 | - | `Your side is too nervous to eat Berries!` | Berries blocked by Unnerve/As One (side named) |
| PL-251 | 2 | 1 | - | `The opposing {MON} is preparing to tell a chillingly bad joke!` | Chilly Reception announcement |
| PL-257 | 1 | 1 | - | `{MON}'s stats were not lowered!` | stat drop prevented |
| PL-283 | 1 | 1 | - | `It's a one-hit KO!` | OHKO |
| PL-299 | 1 | 1 | - | `{MON} fell straight down!` | grounded (Smack Down) |
| PL-300 | 1 | 1 | - | `{MON} is protected by the Psychic Terrain!` | terrain blocked a priority move |
| PL-306 | 1 | 1 | - | `All Pokémon that heard the song willfaint in three turns!` | Perish Song start (no Pokemon named) |

No-kind total: 35 templates, 4602 rows.

## AMBIGUOUS list (same text, several mechanics)

| id | rows | battles | category | template | mechanics sharing the text |
|---|---|---|---|---|---|
| PL-006 | 2006 | 486 | Held Item | `{MON}'s {ITEM}` | item what=shown/used/removed (an HP Restored/Lost row often follows: Leftovers, berries, Life Orb) |
| PL-030 | 173 | 91 | - | `The opposing {MON} had its energy drained!` | hp, restored by drain: Giga Drain, Drain Punch... or Leech Seed receiver |
| PL-045 | 96 | 28 | - | `The opposing {MON} had its HP restored.` | hp, restored: Leftovers, Grassy Terrain, healing moves, Wish... |
| PL-048 | 85 | 25 | - | `{MON} had its HP restored.` | hp, restored: Leftovers, Grassy Terrain, healing moves, Wish... |
| PL-050 | 80 | 42 | - | `{MON} had its energy drained!` | hp, restored by drain: Giga Drain, Drain Punch... or Leech Seed receiver |
| PL-141 | 11 | 8 | - | `{MON} kept going and crashed!` | hp, cause row (crash): Jump Kick, High Jump Kick, Axe Kick, Supercell Slam |
| PL-174 | 6 | 5 | - | `The opposing {MON} transformed into {MON}!` | transform (Ditto) or form change |
| PL-176 | 6 | 3 | - | `{MON} was hurt!` | hp, cause row with the cause not named |
| PL-177 | 6 | 6 | - | `The opposing {MON} endured the hit!` | volatile_hit Endure / ability Sturdy |
| PL-190 | 5 | 5 | - | `The opposing {MON} slept and restored its HP!` | Rest: status slp plus hp restored in one row |
| PL-200 | 4 | 4 | - | `The power of the opposing {MON}'s {TYPE}-type moves rose!` | volatile/ability: Charge, Electromorphosis, Wind Power... |

Ambiguous total: 11 templates, 2478 rows.

## UNREADABLE (OCR damage)

29 templates, 57 rows (0.15% of rows). They are OCR-damaged names, dropped letters or characters, or partial reads. Five examples (raw text):

- `Held Item: Hippowdon’s Sitrus Berrv`
- `Held Item: Archaludon’s Sitrus Berrv`
- `Held Item: Primarina’s Sitrus Berrv`
- `Held Item: Corviknight’s Rockv Helmet`
- `Held Item: Skarmory’s Rockv Helmet`

Most damaged rows are item or ability names with one wrong letter ("Berrv", "Rockv", "Bodv"), a damaged Omni Ring line, or a stat row with a dropped letter ("fel!"). The meaning of most is recoverable by edit distance; I did not do that here.

## Full template table

Columns: id, rows, battles containing it, category (- = none), template, class, section 4 kind and fields, note. Side words ("The opposing") are kept in the template; a row with none is the own side (or a name-only row).

| id | rows | battles | cat | template | class | kind / fields | note |
|---|---|---|---|---|---|---|---|
| PL-001 | 3577 | 542 | HP Lost | `{MON}'s HP: {N}% → {N}%` | KIND | hp (opp: pct) | percent before and after; no maxhp |
| PL-002 | 3317 | 524 | HP Lost | `{MON}'s HP: {N} → {N}` | KIND | hp (own: hp) | current HP before and after; maxhp not in the row |
| PL-003 | 3119 | 533 | Move | `The opposing {MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-004 | 2938 | 539 | Move | `{MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-005 | 2155 | 548 | - | `Go! {MON}!` | KIND | enter side=own, mon (no hp shown) |  |
| PL-006 | 2006 | 486 | Held Item | `{MON}'s {ITEM}` | AMBIGUOUS | item what=shown/used/removed (an HP Restored/Lost row often follows: Leftovers, berries, Life Orb) | same row for an item revealed, an item that acted, or a berry eaten |
| PL-007 | 1873 | 531 | - | `{TRAINER} sent out {MON}!` | KIND | enter side=opp, mon (no pct shown) |  |
| PL-008 | 1296 | 461 | - | `It's super effective!` | NO-KIND | effectiveness: super effective |  |
| PL-009 | 1160 | 457 | Ability | `{MON}'s {ABILITY}` | KIND | ability (shown) | side not in the row text; read from the sprite or position |
| PL-010 | 1118 | 346 | HP Restored | `{MON}'s HP: {N} → {N}` | KIND | hp (own: hp) | current HP before and after; maxhp not in the row |
| PL-011 | 1033 | 282 | HP Restored | `{MON}'s HP: {N}% → {N}%` | KIND | hp (opp: pct) | percent before and after; no maxhp |
| PL-012 | 830 | 357 | - | `It's not very effective..` | NO-KIND | effectiveness: not very effective |  |
| PL-013 | 815 | 460 | Fainted | `{MON} fainted!` | KIND | faint (side, mon) |  |
| PL-014 | 780 | 474 | Fainted | `The opposing {MON} fainted!` | KIND | faint (side, mon) |  |
| PL-015 | 693 | 353 | - | `{MON}, come back!` | NO-KIND | own withdrawal | implied by next enter |
| PL-016 | 680 | 334 | - | `{TRAINER} withdrew {MON}!` | NO-KIND | withdrawal of a Pokemon (side only from the trainer name) | implied by next enter |
| PL-017 | 542 | 279 | Move +{N} Priority | `{MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-018 | 430 | 186 | - | `The opposing {MON}'s {STATS} rose!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-019 | 407 | 407 | Mega Evolution | `The opposing {MON} has Mega Evolved into {MON}!` | KIND | mega (side, mon, form; stone from the 'reacting' row) |  |
| PL-020 | 393 | 393 | Mega Evolution | `{MON} has Mega Evolved into {MON}!` | KIND | mega (side, mon, form; stone from the 'reacting' row) |  |
| PL-021 | 385 | 385 | - | `{MON}'s {ITEM} is reacting to {TRAINER}'s Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-022 | 383 | 383 | - | `The opposing {MON}'s {ITEM} is reacting to {TRAINER}'s Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-023 | 312 | 156 | - | `{TRAINER} sent out {MON} the {TITLE}!` | KIND | enter side=opp, mon (plus the trainer's title) | title is extra text |
| PL-024 | 271 | 188 | - | `It's not very effective...` | NO-KIND | effectiveness: not very effective |  |
| PL-025 | 237 | 169 | - | `{MON}'s {STATS} fell!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-026 | 231 | 161 | Move +{N} Priority | `The opposing {MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-027 | 187 | 125 | - | `{MON}'s {STATS} harshly fell!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-028 | 179 | 79 | - | `{MON}'s {STATS} rose!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-029 | 179 | 45 | - | `{MON} was hurt by its burn!` | KIND | hp, cause row (from=brn/psn); amounts in the HP Lost row after it |  |
| PL-030 | 173 | 91 | - | `The opposing {MON} had its energy drained!` | AMBIGUOUS | hp, restored by drain: Giga Drain, Drain Punch... or Leech Seed receiver | same text for all drain moves; HP Restored row follows |
| PL-031 | 165 | 137 | - | `{MON} floats in the air with its {ITEM}!` | KIND | item shown (Air Balloon) |  |
| PL-032 | 150 | 49 | - | `The opposing {MON} protected itself!` | NO-KIND | Protect activation (names the protected mon) | contradicts section 5 |
| PL-033 | 147 | 64 | - | `Pointed stones dug into {MON}!` | KIND | hp, cause row (from=stealthrock) |  |
| PL-034 | 139 | 113 | - | `It's extremely effective!` | NO-KIND | effectiveness: extremely effective (x4) | Champions wording |
| PL-035 | 138 | 99 | - | `The opposing {MON} went back to {TRAINER}!` | NO-KIND | withdrawal after a pivot move | implied by next enter |
| PL-036 | 129 | 39 | - | `{MON} is buffeted by the sandstorm!` | KIND | hp, cause row (from=sandstorm) |  |
| PL-037 | 123 | 93 | - | `The Pokémon was hit {N} times!` | KIND | hits (n; no side or mon named) |  |
| PL-038 | 120 | 87 | - | `The opposing {MON}'s {STATS} rose sharply!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-039 | 120 | 23 | - | `{MON} was hurt by its poisoning!` | KIND | hp, cause row (from=brn/psn); amounts in the HP Lost row after it |  |
| PL-040 | 116 | 114 | - | `{MON}'s {ITEM} popped!` | KIND | item removed (Air Balloon popped) |  |
| PL-041 | 113 | 72 | - | `The opposing {MON} lost some of its HP!` | KIND | hp, cause row (Life Orb; item not named in the row) |  |
| PL-042 | 106 | 74 | - | `But it failed!` | KIND | fail (side and mon NOT named in the row) | kind requires side, mon |
| PL-043 | 101 | 46 | - | `The opposing {MON} is buffeted by the sandstorm!` | KIND | hp, cause row (from=sandstorm) |  |
| PL-044 | 100 | 85 | - | `The opposing {MON}'s {STATS} fell!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-045 | 96 | 28 | - | `The opposing {MON} had its HP restored.` | AMBIGUOUS | hp, restored: Leftovers, Grassy Terrain, healing moves, Wish... | same text; HP Restored row follows |
| PL-046 | 91 | 58 | - | `It's mostly ineffective..` | NO-KIND | effectiveness: mostly ineffective (x0.25) | Champions wording |
| PL-047 | 88 | 66 | - | `A sandstorm kicked up!` | KIND | weather Sandstorm start |  |
| PL-048 | 85 | 25 | - | `{MON} had its HP restored.` | AMBIGUOUS | hp, restored: Leftovers, Grassy Terrain, healing moves, Wish... | same text; HP Restored row follows |
| PL-049 | 82 | 33 | - | `{MON} is fast asleep.` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-050 | 80 | 42 | - | `{MON} had its energy drained!` | AMBIGUOUS | hp, restored by drain: Giga Drain, Drain Punch... or Leech Seed receiver | same text for all drain moves; HP Restored row follows |
| PL-051 | 77 | 77 | - | `{MON} hung on using its {ITEM}!` | KIND | item used (Focus Sash/Band; item named) |  |
| PL-052 | 75 | 60 | - | `The opposing {MON} avoided the attack!` | KIND | miss (mon that avoided) | attacker not named |
| PL-053 | 72 | 72 | - | `Pointed stones float in the air on your side!` | KIND | side Stealth Rock start |  |
| PL-054 | 70 | 62 | - | `{MON} transformed into the Poison type!` | KIND | types (type named after 'the') |  |
| PL-055 | 69 | 56 | - | `The opposing {MON} was damaged by the recoil!` | KIND | hp, cause row (from=recoil) |  |
| PL-056 | 67 | 67 | - | `The opposing {MON} hung on using its {ITEM}!` | KIND | item used (Focus Sash/Band; item named) |  |
| PL-057 | 67 | 67 | Move | `The opposing {MON} used Stealth Rock!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-058 | 64 | 39 | - | `{MON} grew drowsy!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-059 | 60 | 47 | - | `{MON} was burned!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-060 | 59 | 38 | - | `{MON} fell asleep!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-061 | 59 | 34 | - | `{MON} must do an encore!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-062 | 56 | 48 | - | `The sandstorm subsided.` | KIND | weather none (Sandstorm end) |  |
| PL-063 | 54 | 39 | Move +{N} Priority | `The opposing {MON} used Protect!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-064 | 54 | 47 | - | `{MON} went back to {TRAINER}!` | NO-KIND | withdrawal after a pivot move | implied by next enter |
| PL-065 | 53 | 37 | - | `The sunlight turned harsh!` | KIND | weather SunnyDay start |  |
| PL-066 | 52 | 44 | - | `{MON} transformed into the Bug type!` | KIND | types (type named after 'the') |  |
| PL-067 | 50 | 17 | - | `The opposing {MON} was hurt by its burn!` | KIND | hp, cause row (from=brn/psn); amounts in the HP Lost row after it |  |
| PL-068 | 50 | 19 | - | `The substitute took damage for the opposing {MON}!` | KIND | volatile_hit Substitute |  |
| PL-069 | 47 | 8 | - | `{MON}'s {STATS} won't go any higher!` | NO-KIND | stat already at +6, no change | a boost of 0 |
| PL-070 | 45 | 43 | - | `{MON} avoided the attack!` | KIND | miss (mon that avoided) | attacker not named |
| PL-071 | 44 | 8 | - | `{MON} is hurt by {MOVE}!` | KIND | hp, cause row (trapping move) |  |
| PL-072 | 44 | 42 | - | `{MON} transformed into the Fighting type!` | KIND | types (type named after 'the') |  |
| PL-073 | 42 | 30 | - | `{MON}'s {STATS} rose sharply!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-074 | 41 | 8 | - | `{MON}'s health is sapped by {MOVE}!` | KIND | hp, cause row (Leech Seed; of=move) |  |
| PL-075 | 40 | 30 | - | `Grass grew to cover the battlefield!` | KIND | field Grassy Terrain start |  |
| PL-076 | 40 | 37 | - | `It doesn't affect {MON}..` | NO-KIND | effectiveness: immune |  |
| PL-077 | 40 | 21 | - | `The opposing {MON} put in a substitute!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-078 | 39 | 35 | - | `The battlefield got weird!` | KIND | field Psychic Terrain start |  |
| PL-079 | 39 | 20 | Move | `The opposing {MON} used Substitute!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-080 | 37 | 22 | - | `The opposing {MON} was hurt by its poisoning!` | KIND | hp, cause row (from=brn/psn); amounts in the HP Lost row after it |  |
| PL-081 | 34 | 17 | - | `The opposing {MON} is exerting its pressure!` | KIND | ability shown (Pressure) |  |
| PL-082 | 33 | 26 | - | `{MON} flinched and couldn't move!` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-083 | 32 | 22 | - | `It started to snow!` | KIND | weather Snowscape start |  |
| PL-084 | 32 | 25 | - | `The opposing {MON} absorbed light!` | KIND | prepare (two-turn move; move not named) |  |
| PL-085 | 32 | 32 | - | `The opposing {MON}'s disguise was busted!` | KIND | form (Disguise busted) plus ability shown |  |
| PL-086 | 32 | 26 | - | `{MON} was dragged out!` | KIND | enter forced=true |  |
| PL-087 | 32 | 16 | - | `The opposing {MON}'s substitute faded!` | KIND | volatile_end Substitute |  |
| PL-088 | 31 | 26 | - | `It doesn't affect the opposing {MON}..` | NO-KIND | effectiveness: immune |  |
| PL-089 | 31 | 22 | - | `It started to rain!` | KIND | weather RainDance start |  |
| PL-090 | 31 | 28 | - | `The harsh sunlight faded.` | KIND | weather none (sun end) |  |
| PL-091 | 30 | 29 | - | `It doesn't affect {MON}...` | NO-KIND | effectiveness: immune |  |
| PL-092 | 30 | 21 | Move | `The opposing {MON} used {MOVE}` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-093 | 29 | 26 | - | `The opposing {MON} floats in the air with its {ITEM}!` | KIND | item shown (Air Balloon) |  |
| PL-094 | 27 | 14 | - | `{MON} ended its encore!` | KIND | volatile_end |  |
| PL-095 | 27 | 21 | Move -{N} Priority | `The opposing {MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-096 | 26 | 8 | - | `The opposing {MON} began charging power!` | KIND | volatile Charge |  |
| PL-097 | 25 | 22 | - | `The opposing {MON}'s {STATS} harshly fell!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-098 | 25 | 15 | - | `{MON} woke up!` | KIND | cure (slp/frz) |  |
| PL-099 | 24 | 22 | - | `It doesn't affect the opposing {MON}.` | NO-KIND | effectiveness: immune |  |
| PL-100 | 23 | 23 | - | `The opposing {MON}'s {ITEM} popped!` | KIND | item removed (Air Balloon popped) |  |
| PL-101 | 23 | 23 | - | `The opposing {MON} was poisoned!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-102 | 21 | 13 | - | `{MON} lost some of its HP!` | KIND | hp, cause row (Life Orb; item not named in the row) |  |
| PL-103 | 21 | 19 | - | `The opposing {MON} grew drowsy!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-104 | 21 | 17 | - | `Toxic spikes were scattered on the ground all around your side!` | KIND | side Toxic Spikes start |  |
| PL-105 | 20 | 20 | - | `The grass disappeared from the battlefield.` | KIND | field Grassy Terrain end |  |
| PL-106 | 20 | 18 | - | `The snow stopped.` | KIND | weather none (snow end) |  |
| PL-107 | 20 | 18 | - | `{MON} is paralyzed, so it may be unable to move!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-108 | 20 | 13 | - | `{MON} was poisoned!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-109 | 19 | 13 | Move | `The opposing {MON} used Light Screen!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-110 | 19 | 13 | - | `Light Screen made the opposing side stronger against special moves!` | KIND | side screen start |  |
| PL-111 | 19 | 14 | - | `{MON} was damaged by the recoil!` | KIND | hp, cause row (from=recoil) |  |
| PL-112 | 19 | 7 | - | `{MON} has been afflicted with an infestation by the opposing {MON}!` | KIND | volatile Infestation (of) |  |
| PL-113 | 18 | 16 | Move | `The opposing {MON} used Reflect!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-114 | 18 | 16 | - | `Reflect made the opposing side stronger against physical moves!` | KIND | side screen start |  |
| PL-115 | 18 | 16 | - | `Aurora Veil made the opposing side stronger against physical and special moves!` | KIND | side screen start |  |
| PL-116 | 18 | 18 | - | `The weirdness disappeared from the battlefield!` | KIND | field Psychic Terrain end |  |
| PL-117 | 18 | 18 | - | `The opposing {MON} was burned!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-118 | 18 | 11 | - | `The opposing {MON} breaks the mold!` | KIND | ability shown (Mold Breaker) |  |
| PL-119 | 17 | 17 | - | `It's mostly ineffective...` | NO-KIND | effectiveness: mostly ineffective (x0.25) | Champions wording |
| PL-120 | 17 | 14 | - | `{MON} was badly poisoned!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-121 | 17 | 4 | - | `The opposing {MON}'s {STATS} won't go any higher!` | NO-KIND | stat already at +6, no change | a boost of 0 |
| PL-122 | 16 | 16 | - | `The opposing {MON} was revived and is ready to fight again!` | KIND | revive |  |
| PL-123 | 16 | 16 | - | `The opposing {MON} held up its {ITEM} against {MON}!` | KIND | item used (Red Card; of) |  |
| PL-124 | 15 | 14 | Move | `The opposing {MON} used Aurora Veil!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-125 | 15 | 11 | - | `Spikes were scattered on the ground all around your side!` | KIND | side Spikes start |  |
| PL-126 | 14 | 14 | - | `{MON}'s disguise was busted!` | KIND | form (Disguise busted) plus ability shown |  |
| PL-127 | 14 | 13 | - | `The opposing {MON} transformed into the Poison type!` | KIND | types (type named after 'the') |  |
| PL-128 | 14 | 14 | - | `The rain stopped.` | KIND | weather none (rain end) |  |
| PL-129 | 14 | 8 | - | `{MON} was hurt by the spikes!` | KIND | hp, cause row (from=spikes) |  |
| PL-130 | 14 | 2 | Move | `{MON} used Substitute!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-131 | 14 | 8 | - | `{MON} is afflicted by the curse!` | KIND | volatile Curse (and hp cost) |  |
| PL-132 | 13 | 12 | - | `The opposing {MON} was damaged by the recoill` | KIND | hp, cause row (from=recoil) |  |
| PL-133 | 13 | 8 | - | `The opposing {MON} is fast asleep.` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-134 | 13 | 8 | - | `All stat changes were eliminated!` | KIND | boost_reset all (Haze) |  |
| PL-135 | 13 | 8 | - | `The opposing {MON}'s wish came true!` | KIND | hp, cause row (Wish) |  |
| PL-136 | 12 | 2 | - | `{MON} already has a substitute!` | KIND | fail (mon named, reason in text) |  |
| PL-137 | 11 | 10 | - | `{MON} is about to be attacked by its {ITEM}!` | KIND | item shown (Poltergeist) |  |
| PL-138 | 11 | 8 | - | `{MON} was seeded!` | KIND | volatile Leech Seed |  |
| PL-139 | 11 | 10 | - | `The opposing side's Aurora Veil wore off!` | KIND | side end |  |
| PL-140 | 11 | 10 | - | `The battlers shared their pain!` | NO-KIND | Pain Split (no Pokemon named) |  |
| PL-141 | 11 | 8 | - | `{MON} kept going and crashed!` | AMBIGUOUS | hp, cause row (crash): Jump Kick, High Jump Kick, Axe Kick, Supercell Slam | same text; move not named |
| PL-142 | 10 | 10 | - | `The opposing {MON}'s {ITEM} lessened the damage it took!` | KIND | item used (resist berry) |  |
| PL-143 | 10 | 8 | - | `The opposing {MON} gained strength from the fallen!` | KIND | ability shown (Supreme Overlord) |  |
| PL-144 | 10 | 5 | - | `Pointed stones dug into the opposing {MON}!` | KIND | hp, cause row (from=stealthrock) |  |
| PL-145 | 10 | 6 | - | `{MON} was damaged by the recoill` | KIND | hp, cause row (from=recoil) |  |
| PL-146 | 10 | 10 | Held Item | `{MON}'s Sitrus Berrv` | UNREADABLE | - |  |
| PL-147 | 10 | 6 | - | `{MON} is confused!` | KIND | volatile_hit confusion (status check at move time) |  |
| PL-148 | 10 | 10 | - | `The opposing {MON} twisted the dimensions!` | KIND | field Trick Room start (names the user) |  |
| PL-149 | 9 | 8 | - | `{MON}'s {MOVE} was disabled!` | KIND | volatile Disable detail=move |  |
| PL-150 | 9 | 8 | - | `The opposing {MON} fell asleep!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-151 | 9 | 7 | - | `The opposing {MON} absorbed electricity!` | KIND | prepare (Electro Shot charging turn; move not named) |  |
| PL-152 | 9 | 6 | - | `The opposing side's Light Screen wore off!` | KIND | side end |  |
| PL-153 | 9 | 7 | Held Item | `{MON}'s Rockv Helmet` | UNREADABLE | - |  |
| PL-154 | 8 | 6 | - | `{MON} became confused due to fatigue!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-155 | 8 | 8 | - | `The opposing {MON} transformed into the Fighting type!` | KIND | types (type named after 'the') |  |
| PL-156 | 8 | 6 | - | `{MON} was prevented from healing!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-157 | 8 | 8 | - | `The opposing {MON} used up all its electricity!` | KIND | types (loses Electric: Double Shock) |  |
| PL-158 | 8 | 8 | - | `The opposing {MON} cut its own HP and put a curse on {MON}!` | KIND | volatile Curse (of) plus hp cost |  |
| PL-159 | 7 | 7 | - | `The opposing {MON} is hoping to take its attacker down with it!` | KIND | once (Destiny Bond) |  |
| PL-160 | 7 | 7 | - | `The opposing {MON} transformed into the Grass type!` | KIND | types (type named after 'the') |  |
| PL-161 | 7 | 3 | - | `{MON}'s perish count fell to {N}!` | KIND | volatile Perish detail=n (shows the count) | contradicts section 5 |
| PL-162 | 7 | 6 | - | `The opposing {MON} woke up!` | KIND | cure (slp/frz) |  |
| PL-163 | 7 | 6 | - | `It traced {MON}'s {ABILITY}!` | KIND | ability (Trace copies; of) |  |
| PL-164 | 7 | 7 | - | `The opposing side's Reflect wore off!` | KIND | side end |  |
| PL-165 | 7 | 7 | - | `{MON} is already drowsy.` | KIND | fail (mon named, reason in text) |  |
| PL-166 | 7 | 7 | Move | `The opposing {MON} used Toxic Spikes!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-167 | 6 | 6 | - | `{MON} gained strength from the fallen!` | KIND | ability shown (Supreme Overlord) |  |
| PL-168 | 6 | 6 | - | `The opposing {MON} took its attacker down with it!` | KIND | volatile_hit Destiny Bond |  |
| PL-169 | 6 | 4 | - | `The opposing {MON} is protected by the Psychic Terrain!` | NO-KIND | terrain blocked a priority move |  |
| PL-170 | 6 | 5 | - | `The opposing {MON} transformed into the Ice type!` | KIND | types (type named after 'the') |  |
| PL-171 | 6 | 6 | - | `The opposing {MON} became confused due to fatigue!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-172 | 6 | 5 | - | `{MON} couldn't move because it's paralyzed!` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-173 | 6 | 6 | - | `The opposing {MON}'s {STATS} fel!` | UNREADABLE | - |  |
| PL-174 | 6 | 5 | - | `The opposing {MON} transformed into {MON}!` | AMBIGUOUS | transform (Ditto) or form change | same text |
| PL-175 | 6 | 6 | - | `The opposing {MON} transformed into the Dark type!` | KIND | types (type named after 'the') |  |
| PL-176 | 6 | 3 | - | `{MON} was hurt!` | AMBIGUOUS | hp, cause row with the cause not named | same row for several causes |
| PL-177 | 6 | 6 | - | `The opposing {MON} endured the hit!` | AMBIGUOUS | volatile_hit Endure / ability Sturdy | same text |
| PL-178 | 6 | 6 | - | `The opposing {MON}'s {ITEM} is reacting to's Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-179 | 6 | 6 | - | `The opposing {MON} frisked {MON} and found its {ITEM}!` | KIND | item shown from=Frisk (names both mons) |  |
| PL-180 | 6 | 2 | - | `{ABILITY} made the opposing {MON} move faster!` | KIND | ability shown (Quick Draw) |  |
| PL-181 | 5 | 5 | - | `The opposing {MON} is switched out with the {ITEM}!` | KIND | item used (Eject Button) |  |
| PL-182 | 5 | 5 | Move | `{MON} used Stealth Rock!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-183 | 5 | 5 | - | `Pointed stones float in the air on the opposing side!` | KIND | side Stealth Rock start |  |
| PL-184 | 5 | 5 | Move | `The opposing {MON} used Spikes!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-185 | 5 | 5 | Move -{N} Priority | `The opposing {MON} used {MOVE}` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-186 | 5 | 4 | - | `It hurt itself in its confusion!` | KIND | volatile_hit confusion (mon NOT named) | kind requires side, mon |
| PL-187 | 5 | 5 | - | `{MON} fell for the taunt!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-188 | 5 | 5 | - | `The opposing {MON} returned its stats to normal using its {ITEM}!` | KIND | boost_reset negative plus item used (White Herb) |  |
| PL-189 | 5 | 4 | - | `The opposing {MON} is paralyzed, so it may be unable to move!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-190 | 5 | 5 | - | `The opposing {MON} slept and restored its HP!` | AMBIGUOUS | Rest: status slp plus hp restored in one row |  |
| PL-191 | 5 | 5 | - | `The opposing {MON}'s illusion wore off!` | NO-KIND | Illusion ended |  |
| PL-192 | 5 | 5 | - | `The twisted dimensions returned to normal!` | KIND | field Trick Room end |  |
| PL-193 | 4 | 4 | - | `{MON} became confused!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-194 | 4 | 4 | - | `{MON} is no longer prevented from healing!` | KIND | volatile_end Heal Block |  |
| PL-195 | 4 | 4 | - | `{MON} shook off the taunt!` | KIND | volatile_end |  |
| PL-196 | 4 | 4 | - | `{MON} transformed into the Water type!` | KIND | types (type named after 'the') |  |
| PL-197 | 4 | 4 | Ability Changed | `{MON}: {ABILITY} {ABILITY}` | KIND | ability (changed_from) | row reads 'MON: OLD -> NEW' |
| PL-198 | 4 | 3 | - | `The opposing {MON} can act faster than normal, thanks to its {ITEM}!` | KIND | item shown (Quick Claw) |  |
| PL-199 | 4 | 4 | - | `The opposing {MON} transformed into the Water type!` | KIND | types (type named after 'the') |  |
| PL-200 | 4 | 4 | - | `The power of the opposing {MON}'s {TYPE}-type moves rose!` | AMBIGUOUS | volatile/ability: Charge, Electromorphosis, Wind Power... | same text |
| PL-201 | 4 | 2 | - | `{MON} was caught in a sticky web!` | NO-KIND | Sticky Web acted on switch-in (side effect acted) |  |
| PL-202 | 4 | 3 | - | `The opposing {MON} is radiating a fairy aura!` | KIND | ability shown (Fairy Aura) |  |
| PL-203 | 3 | 3 | - | `The opposing {MON}'s {STATS} fell` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-204 | 3 | 2 | - | `The opposing {MON}'s perish count fell to {N}!` | KIND | volatile Perish detail=n (shows the count) | contradicts section 5 |
| PL-205 | 3 | 3 | - | `{MON} snapped out of its confusion!` | KIND | volatile_end |  |
| PL-206 | 3 | 3 | - | `It doesn't affect the opposing {MON}` | NO-KIND | effectiveness: immune |  |
| PL-207 | 3 | 3 | - | `{MON} can't use {MOVE} after the taunt!` | KIND | cant reason=taunt (move named) |  |
| PL-208 | 3 | 3 | - | `The Pokémon was hit {N} time!` | KIND | hits (n; no side or mon named) |  |
| PL-209 | 3 | 3 | - | `The onnosing {MON}'s Sn. Atk Sn. Def, and Sneed rosel` | UNREADABLE | - |  |
| PL-210 | 3 | 3 | - | `The opposing {MON}'s {ITEM} is reacting to s Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-211 | 3 | 2 | - | `{MON} was freed from {MOVE}!` | KIND | volatile_end (trapping move) |  |
| PL-212 | 3 | 3 | - | `It doesn't affect the opposing {MON}...` | NO-KIND | effectiveness: immune |  |
| PL-213 | 3 | 3 | - | `The opposing {MON}'s HP is full!` | KIND | fail (mon named, reason in text) |  |
| PL-214 | 3 | 3 | - | `The opposing {MON} knocked off {MON}'s {ITEM}!` | KIND | item removed (of) |  |
| PL-215 | 3 | 1 | - | `It doesn't affect {MON}.` | NO-KIND | effectiveness: immune |  |
| PL-216 | 3 | 3 | - | `The opposing {MON}'s {ITEM} woke it up!` | KIND | cure + item used (from=item) |  |
| PL-217 | 3 | 3 | - | `{MON} has no moves left that it can use!` | NO-KIND | Struggle forced |  |
| PL-218 | 3 | 3 | Move | `{MON} used Struggle!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-219 | 3 | 3 | - | `{MON}'s {STATS} fell` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-220 | 3 | 3 | - | `The opposing {MON} went back to!` | NO-KIND | withdrawal after a pivot move (trainer name unread) | implied by next enter |
| PL-221 | 3 | 1 | - | `Until Battle Ends` | NOT-BATTLE | UI text read as a log row |  |
| PL-222 | 3 | 2 | - | `{MON}'s {STATS} harshly fel!` | UNREADABLE | - |  |
| PL-223 | 3 | 3 | Move | `The opposing {MON} used Aurora Veil` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-224 | 3 | 3 | - | `The opposing {MON} flinched and couldn't move!` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-225 | 3 | 1 | - | `The opposing {MON} started heating up its beak!` | KIND | once (Beak Blast) |  |
| PL-226 | 3 | 2 | - | `{MON} is already poisoned!` | KIND | fail (mon named, reason in text) |  |
| PL-227 | 3 | 2 | - | `The opposing {MON}'s {STATS} rose drastically!` | KIND | boost (side, mon, stat(s), by from wording) | one row can name several stats |
| PL-228 | 3 | 1 | - | `{MON} became trapped in the fiery vortex!` | KIND | volatile Fire Spin |  |
| PL-229 | 2 | 2 | - | `The opposing {MON} was frozen solid!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-230 | 2 | 2 | - | `The opposing {MON} transformed into the Bug type!` | KIND | types (type named after 'the') |  |
| PL-231 | 2 | 1 | - | `{MON}'s HP is full!` | KIND | fail (mon named, reason in text) |  |
| PL-232 | 2 | 2 | - | `All Pokémon that heard the song will faint in three turns!` | NO-KIND | Perish Song start (no Pokemon named) |  |
| PL-233 | 2 | 2 | - | `The opposing {MON} is confused!` | KIND | volatile_hit confusion (status check at move time) |  |
| PL-234 | 2 | 2 | Ability | `{MON}'s Cursed Bodv` | UNREADABLE | - |  |
| PL-235 | 2 | 2 | - | `{MON} was frozen solid!` | KIND | status (brn par slp frz psn tox) | cause not named |
| PL-236 | 2 | 1 | - | `The opposing {MON} harvested a {ITEM}!` | KIND | item received (Harvest) |  |
| PL-237 | 2 | 2 | - | `{MON} put in a substitute!` | KIND | volatile (Yawn, Taunt, Encore, Substitute, confusion, Heal Block) | effect from the wording |
| PL-238 | 2 | 2 | Move {N} Priority | `The opposing {MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-239 | 2 | 2 | - | `{TRAINER} withdrew {MON}` | NO-KIND | withdrawal of a Pokemon (side only from the trainer name) | implied by next enter |
| PL-240 | 2 | 2 | Mega Evolution | `The opposing {MON} has Mega Evolved into {MON}` | KIND | mega (side, mon, form; stone from the 'reacting' row) |  |
| PL-241 | 2 | 2 | Move —{N} Driorit" | `The opposing {MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-242 | 2 | 2 | - | `The healing wish came true for the opposing {MON}!` | KIND | hp/cure, cause row (Healing Wish) |  |
| PL-243 | 2 | 2 | Move | `The opposing {MON} used Tailwind!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-244 | 2 | 2 | - | `A tailwind started blowing on the opposing side!` | KIND | side Tailwind start |  |
| PL-245 | 2 | 2 | - | `The opposing {MON} transformed into the Electric type!` | KIND | types (type named after 'the') |  |
| PL-246 | 2 | 2 | Ability | `{MON}'s Gooev` | UNREADABLE | - |  |
| PL-247 | 2 | 2 | Move | `The opposing {MON} used Sticky Web!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-248 | 2 | 2 | - | `A sticky web has been laid out on the ground on your side!` | KIND | side Sticky Web start |  |
| PL-249 | 2 | 1 | - | `Your side is too nervous to eat Berries!` | NO-KIND | Berries blocked by Unnerve/As One (side named) |  |
| PL-250 | 2 | 1 | - | `{TRAINER} sent out 's {MON}!` | KIND | enter side=opp, mon (stray text before the name) |  |
| PL-251 | 2 | 1 | - | `The opposing {MON} is preparing to tell a chillingly bad joke!` | NO-KIND | Chilly Reception announcement |  |
| PL-252 | 1 | 1 | - | `The opposing {MON} thawed out!` | KIND | cure (slp/frz) |  |
| PL-253 | 1 | 1 | - | `{TRAINER} sent out Rillaboo!` | UNREADABLE | - |  |
| PL-254 | 1 | 1 | - | `{MON} was damaged by the recoil` | KIND | hp, cause row (from=recoil) |  |
| PL-255 | 1 | 1 | - | `The {ITEM} strengthened {MOVE}'s power!` | KIND | item used (Gem; names no mon) |  |
| PL-256 | 1 | 1 | - | `The opposing {MON} is already drowsy.` | KIND | fail (mon named, reason in text) |  |
| PL-257 | 1 | 1 | - | `{MON}'s stats were not lowered!` | NO-KIND | stat drop prevented |  |
| PL-258 | 1 | 1 | - | `{MON}'s {ITEM} defrosted it!` | KIND | cure + item used (from=item) |  |
| PL-259 | 1 | 1 | - | `Theoin eTiE S. Dan Spee!` | UNREADABLE | - |  |
| PL-260 | 1 | 1 | - | `I ne opposing voicarona's {STATS}, Sp. Det, ana Speea rose!` | UNREADABLE | - |  |
| PL-261 | 1 | 1 | - | `The opposing {MON} transformed into the Rock type!` | KIND | types (type named after 'the') |  |
| PL-262 | 1 | 1 | - | `The opposing {MON} transformed into the Fairy type!` | KIND | types (type named after 'the') |  |
| PL-263 | 1 | 1 | - | `The substitute took damage for {MON}!` | KIND | volatile_hit Substitute |  |
| PL-264 | 1 | 1 | - | `{MON}'s substitute faded!` | KIND | volatile_end Substitute |  |
| PL-265 | 1 | 1 | - | `The opposing {MON}'s {ITEM} defrosted it!` | KIND | cure + item used (from=item) |  |
| PL-266 | 1 | 1 | - | `The onnosina Felektross's {STATS} {STATS} rosel` | UNREADABLE | - |  |
| PL-267 | 1 | 1 | - | `It doesn't affect the opposing Eelektros..` | UNREADABLE | - |  |
| PL-268 | 1 | 1 | - | `The opposing {MON} must recharge!` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-269 | 1 | 1 | - | `The opposing {MON}'s {ITEM} is reacting to Ü{N}s Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-270 | 1 | 1 | - | `The opposing {MON} braced itself!` | KIND | once (Endure) |  |
| PL-271 | 1 | 1 | Move . {N} Driorit. | `{MON} used {MOVE}!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-272 | 1 | 1 | Ability | `{MON}'s Emergencv Fxit` | UNREADABLE | - |  |
| PL-273 | 1 | 1 | Move | `The opposing {MON} used Psychic Terrain!` | KIND | move (side, mon, move) | the category also carries a priority value when not 0 (not in section 4) |
| PL-274 | 1 | 1 | - | `The opposing {MON}'s {ITEM} is reacting to{N}l's Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-275 | 1 | 1 | Ability | `{MON}'s Sturdv` | UNREADABLE | - |  |
| PL-276 | 1 | 1 | - | `The opposing {MON} underwent a heroic transformation!` | KIND | form (Zero to Hero) |  |
| PL-277 | 1 | 1 | - | `Gho` | UNREADABLE | - |  |
| PL-278 | 1 | 1 | HP Lost | `{MON}'s HP:` | UNREADABLE | - |  |
| PL-279 | 1 | 1 | - | `The opposing {MON} burned itself out!` | KIND | types (loses Fire: Burn Up) |  |
| PL-280 | 1 | 1 | - | `The opposing {MON}'s {ITEM} is reacting toy's Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-281 | 1 | 1 | - | `The opposing {MON}'s {ITEM} is reacting to sOmni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-282 | 1 | 1 | Ability | `{MON}'s Psvchic Surge` | UNREADABLE | - |  |
| PL-283 | 1 | 1 | - | `It's a one-hit KO!` | NO-KIND | OHKO |  |
| PL-284 | 1 | 1 | - | `The opposing {MON} is hoping to take its attacker down with it` | KIND | once (Destiny Bond) |  |
| PL-285 | 1 | 1 | Ability | `{MON}'s Emergencv Exit` | UNREADABLE | - |  |
| PL-286 | 1 | 1 | Ability | `{MON}'s Fairv Aura` | UNREADABLE | - |  |
| PL-287 | 1 | 1 | - | `{MON} thawed out!` | KIND | cure (slp/frz) |  |
| PL-288 | 1 | 1 | - | `{TRAINER} sent out Mimikyuthe Sociable!` | UNREADABLE | - |  |
| PL-289 | 1 | 1 | - | `{TRAINER} sent out Politoe!` | UNREADABLE | - |  |
| PL-290 | 1 | 1 | HP Lost | `{MON}'s HPp {N} → {N}` | UNREADABLE | - |  |
| PL-291 | 1 | 1 | - | `The opposing {MON} shed its tail to create a decoy!` | KIND | volatile Substitute (Shed Tail) |  |
| PL-292 | 1 | 1 | - | `The opposing side's tailwind petered out!` | KIND | side end |  |
| PL-293 | 1 | 1 | - | `It doesn't affect the opposing Indeede.` | UNREADABLE | - |  |
| PL-294 | 1 | 1 | - | `om` | UNREADABLE | - |  |
| PL-295 | 1 | 1 | - | `The opposing {MON} vanished instantly!` | KIND | prepare (two-turn move; move not named) |  |
| PL-296 | 1 | 1 | - | `The opposing {MON}'s {ITEM} cured its poisoning!` | KIND | cure + item used (from=item) |  |
| PL-297 | 1 | 1 | - | `The opposing {MON} couldn't move because it's paralyzed!` | KIND | cant (reason: flinch/slp/recharge/par) |  |
| PL-298 | 1 | 1 | HP Lost | `{MON}'s HP: {N} → ]` | UNREADABLE | - |  |
| PL-299 | 1 | 1 | - | `{MON} fell straight down!` | NO-KIND | grounded (Smack Down) |  |
| PL-300 | 1 | 1 | - | `{MON} is protected by the Psychic Terrain!` | NO-KIND | terrain blocked a priority move |  |
| PL-301 | 1 | 1 | - | `The opposing {MON}'s {ITEM} is reacting tos Omni Ring!` | KIND | mega.stone (names mon and stone item; precedes the Mega Evolution row) |  |
| PL-302 | 1 | 1 | Ability | `lauros's {ABILITY}` | UNREADABLE | - |  |
| PL-303 | 1 | 1 | - | `It doesn't affect the opposing Metagros.` | UNREADABLE | - |  |
| PL-304 | 1 | 1 | - | `{N}elite body {N} - {N} AMmiRing` | UNREADABLE | - |  |
| PL-305 | 1 | 1 | - | `{MON}'s perish count fell to O!` | UNREADABLE | - |  |
| PL-306 | 1 | 1 | - | `All Pokémon that heard the song willfaint in three turns!` | NO-KIND | Perish Song start (no Pokemon named) |  |
