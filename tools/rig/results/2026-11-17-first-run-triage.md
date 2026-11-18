# Triage of the first rig run (2026-11-17)

Input: `tools/rig/results/2026-11-17-first-run.jsonl.gz` (500 battles, 9,848 steps). Script: `tools/rig/triage.py` (`python tools/rig/triage.py` clusters by field signature; `--causes` applies the cause rules used below; `--show STEP_ID` prints one step). Register: `gaps/INDEX.md` (findings, W groups).

## How this was done, and what to trust

- Steps whose only differences are the PP shortcut and HP (the driver's `damage_explained` rule) are set aside. That leaves **1,875 mismatch steps** (`cls_nopp = mismatch`); every other step is exact or damage-only.
- Clustering by field signature is too fine to read: by the exact signature (field paths, moves chosen, ids involved) the 1,875 steps fall into 1,603 clusters; by field paths alone, 260. The same cause shows up under many signatures (for example Mega Garchomp Z's wrong ability appears with every move). So I worked from the field clusters down, opened two to five example steps for each cause, read Showdown's log and the differing fields, and wrote one rule per cause (`triage.py --causes`). Every difference of every step is then labelled by a rule, and a step is labelled by all the causes it contains.
- I read about 120 steps by hand. The other steps were labelled by rule from the same field, value and log pattern, so a minority may carry a wrong label, mostly in the two broad rules: "end of turn after the game is won" (T8) and "average roll flips a KO" (W2). Where a rule is a guess I say so.
- Steps are counted once per verdict by the most serious cause in them (NEW before UNCLEAR before RIG before KNOWN). A step can also show in several cause rows below, so the rows add up to more than the steps.
- "KNOWN" means a register entry describes this fault, not only the same move or ability.

## Summary

| Verdict | Steps | Share of 1,875 | Causes (rows in the table) |
|---|---:|---:|---:|
| KNOWN (in the register) | 1,567 | 83.6% | 69 |
| NEW (not in the register) | 288 | 15.4% | 11 |
| RIG (fault of the rig) | 4 | 0.2% | 2 |
| UNCLEAR | 16 | 0.9% | 2 |
| Total | 1,875 | 100% | 84 |

Share of mismatch steps with a determined cause (KNOWN + NEW + RIG): 1,859 of 1,875, **99.1%**. 237 of the 288 NEW steps are one cause (N1). Without N1, 51 steps are NEW.

Answer to the owner's question: the reading audit missed **12 causes** (N1 to N12 below). One is large (N1, 237 steps, 12.6% of the mismatch steps, plus its effect on Choice locks). Three are small but real rules (N2 Outrage lock on a failed move, N3 PP on continuation turns, N9 actions after the win). The rest are 1 to 8 steps each. One more gap (move order on a Mega turn) is in the register only as a half sentence in T18, with no id and no work group; it is in the table as KNOWN (pointer only), 2 steps (1:207:7, 1:21:10). The audit file `docs/investigation/audit/mechanics-turn.md` (Q11, K8) describes it: the engine orders the turn with pre-Mega Speed, Showdown with post-Mega Speed. Example 1:207:7: Mega Garchomp Z has Speed 169 before and 223 after Mega Evolution, Raichu has 200; Showdown moves Garchomp first (its Draco Meteor knocks Raichu out), the engine moves Raichu first and paralyses Garchomp (`status` Showdown `""`, engine `"par"`).

Other counts from the run: 57 damage values outside Showdown's roll range, 19 legal-action failures, 2 load-check differences, 2 engine errors (sections below).

## Table of causes

Verdict, then the register ids that describe the cause (or the new id). "Only this cause" = steps in which this is the only cause found. Example steps are the first three in file order.

| Steps | Only this cause | Verdict | Register ids / new id | Cause in plain words | Example steps |
|---:|---:|---|---|---|---|
| 237 | 165 | NEW | N1 | Last move (and Choice lock) recorded although the Pokémon could not move (asleep, frozen, paralysed, flinched, confusion hit) | 1:3:14, 1:5:14, 1:5:17 |
| 189 | 168 | KNOWN | T8, W8 | End of turn still runs after the game is won (weather/terrain/screen/Trick Room counters, Leftovers, Speed Boost, Yawn) | 1:2:11, 1:3:31, 1:4:17 |
| 159 | 108 | KNOWN | T16, M01-4, M02-16 (W7) | Fake Out / First Impression still selectable after the first turn | 1:14:8, 1:17:0, 1:17:8 |
| 112 | 50 | KNOWN | W1 (M01-11, M05-18) | Outrage/Thrash lock length and the confusion at its end not modelled; engine keeps the other moves locked | 1:10:7, 1:10:9, 1:18:3 |
| 98 | 36 | KNOWN | T14 (W4) | PP spent (below 10) when the move did not happen | 1:3:14, 1:3:18, 1:3:19 |
| 90 | 83 | KNOWN | M01-2 (W28) | Sparkling Aria puts a marker volatile on the target instead of curing its burn | 1:1:0, 1:13:3, 1:24:3 |
| 77 | 74 | KNOWN | C4, I11, M04-16, A01-14 (W26) | Weather/terrain lasts 8 turns with Damp/Smooth/Heat/Icy Rock or Terrain Extender; engine 5 | 1:2:4, 1:3:11, 1:9:13 |
| 69 | 54 | KNOWN | SP1 (W9) | Mega Garchomp Z has Sand Force, should be Levitate | 1:3:0, 1:6:2, 1:10:0 |
| 66 | 62 | KNOWN | A02-6 (W29) | Moody does nothing | 1:40:0, 1:40:3, 1:40:4 |
| 64 | 13 | KNOWN | W1 | Confusion never ends / never starts (duration not modelled); engine exports no counter | 1:10:9, 1:18:5, 1:26:9 |
| 61 | 54 | KNOWN | A01-3 (W25) | Protean/Libero: two-type holder keeps its 2nd type; type also changes when the move then fails | 1:9:4, 1:13:2, 1:22:0 |
| 52 | 23 | KNOWN | SP2 (W9) | Mega Golisopod has Emergency Exit, should be Tough Claws | 1:13:4, 1:17:0, 1:19:4 |
| 50 | 44 | KNOWN | C4, M02-15 (W17, W26) | Light Clay: screens last 8 turns, engine 5 | 1:11:2, 1:15:0, 1:19:13 |
| 50 | 41 | KNOWN | F7, F8, T3 (W3) | Heal/drain/recoil rounding, 1-2 HP | 1:3:14, 1:30:7, 1:53:15 |
| 41 | 40 | KNOWN | W2 | Average roll makes a hit KO (or not) differently, or shifts a later threshold | 1:9:18, 1:9:19, 1:9:21 |
| 39 | 36 | KNOWN | W1 | Sleep length (Showdown draws 1-3 turns) not modelled | 1:3:27, 1:34:27, 1:93:1 |
| 37 | 29 | KNOWN | M02-1 (W28) | Ghost-type Curse played as the non-Ghost version | 1:9:20, 1:9:22, 1:9:25 |
| 37 | 29 | KNOWN | SP4 (W9) | Mega Lucario Z: loader substitutes Adaptability for Aura Guard (no contact-damage halving, wrong STAB) | 1:28:5, 1:49:0, 1:74:0 |
| 36 | 27 | KNOWN | SP7 (W25) | Aegislash never changes form | 1:85:0, 1:85:1, 1:85:2 |
| 31 | 29 | KNOWN | T9, A01-6 (W5) | Speed Boost triggers on the turn of entry | 1:15:9, 1:25:9, 1:37:16 |
| 30 | 22 | KNOWN | A02-6, M07-3 (W29, W28) | Electromorphosis gives no Charge; Charge never wears off | 1:41:8, 1:41:9, 1:41:16 |
| 30 | 23 | KNOWN | I1 (W4) | Focus Sash never used up | 1:18:5, 1:29:9, 1:41:0 |
| 28 | 18 | KNOWN | M03-1, C2 (W28) | King's Shield does nothing (no block, no Attack drop, no Protect counter) | 1:85:1, 1:85:3, 1:85:5 |
| 25 | 20 | KNOWN | A01-4 (W29) | Cursed Body does nothing | 1:27:1, 1:28:2, 1:78:4 |
| 24 | 19 | KNOWN | W1 | Partial-trap length not modelled; engine exports no counter | 1:36:0, 1:65:0, 1:65:15 |
| 23 | 15 | KNOWN | C2, M05-3 (W28) | Throat Chop does nothing | 1:78:2, 1:78:3, 1:83:9 |
| 23 | 18 | KNOWN | M04-2 (W28) | Soak does nothing | 1:26:0, 1:41:10, 1:82:0 |
| 22 | 15 | KNOWN | SP11 (W25) | Disguise breaks on a blocked or immune move | 1:30:0, 1:90:9, 1:90:12 |
| 20 | 15 | KNOWN | M03-10 (W28) | Heal block never ends | 1:64:5, 1:107:1, 1:107:5 |
| 19 | 18 | KNOWN | A01-2, SP11 (W25, W4) | Disguise swallows the move's side effects (status, drops, recharge, Knock Off, pivot) | 1:8:6, 1:9:4, 1:35:5 |
| 19 | 18 | KNOWN | T5 (W8) | Simultaneous switch-ins always player one first (Intimidate/Trace hit the wrong Pokémon) | 1:48:5, 1:149:5, 1:153:10 |
| 18 | 15 | KNOWN | C3 (W28) | Glaive Rush has no drawback | 1:58:6, 1:173:17, 1:206:10 |
| 18 | 16 | KNOWN | M03-9 (W28) | Rest works when it should fail | 1:5:13, 1:27:5, 1:34:20 |
| 18 | 12 | KNOWN | T4, I7 (W5) | Sitrus Berry eaten too late (or on a different HP, W2) | 1:111:2, 1:111:10, 1:127:9 |
| 18 | 18 | KNOWN | A01-4, T12 (W24) | Emergency Exit: Showdown stops mid-turn for the switch, engine plays on | 1:12:8, 1:13:0, 1:25:15 |
| 17 | 9 | NEW | N3 | PP taken again on the second turn of a charge move and on every Outrage/Thrash turn | 1:39:11, 1:52:7, 1:97:7 |
| 14 | 12 | KNOWN | M01-7, C10 (W7, W5) | Encore lands where it should fail, or lasts a different number of turns | 1:20:47, 1:31:23, 1:146:27 |
| 13 | 11 | KNOWN | SP5 (W9) | Mega Starmie Attack 140, should be 100 | 1:29:5, 1:67:0, 1:68:9 |
| 13 | 8 | KNOWN | T15, C8 (W16) | No freeze counter (guaranteed thaw); engine exports none | 1:17:5, 1:29:2, 1:125:0 |
| 12 | 12 | KNOWN | C1, M05-2 (W8) | Yawn refused while a teammate of the target sleeps (sleep clause) | 1:3:26, 1:149:0, 1:159:5 |
| 12 | 12 | KNOWN | F5, M01-3, M01-1 (W6, W10) | Evasion / accuracy (Minimize, Make It Rain 95) | 1:190:1, 1:277:1, 1:354:22 |
| 11 | 8 | KNOWN | T12 (W24) | Eject Button / Red Card: forced switch missing | 1:9:2, 1:31:8, 1:38:8 |
| 10 | 5 | NEW | N9 | Winner's pending move still runs after the last foe fainted mid-turn | 1:106:13, 1:127:10, 1:136:25 |
| 10 | 6 | UNCLEAR | - | Not determined | 1:78:15, 1:122:7, 1:123:5 |
| 9 | 0 | KNOWN | C3 (W29) | Unburden never switches on | 1:84:17, 1:121:0, 1:131:12 |
| 8 | 7 | KNOWN | M03-8 (W28) | Taunt ends one action early | 1:163:12, 1:163:19, 1:256:17 |
| 8 | 6 | NEW | N2 | Outrage lock applied although the move failed (target immune) or could not be used (Disable) | 1:14:12, 1:18:11, 1:18:12 |
| 7 | 4 | UNCLEAR | - | After a pivot move, the engine appears to have already played the other side's pending move | 1:31:16, 1:290:9, 1:296:17 |
| 7 | 7 | KNOWN | SP9 (W25) | Ditto / Imposter / Transform not modelled | 1:0:0, 1:4:1, 1:6:5 |
| 7 | 6 | KNOWN | T17, C2 (W5) | Disable never wears off | 1:78:9, 1:92:4, 1:256:13 |
| 6 | 6 | KNOWN | M03-2 (W28) | Revival Blessing does nothing | 1:36:8, 1:90:10, 1:95:9 |
| 6 | 2 | KNOWN | M05-9 (W28) | Shed Tail | 1:5:21, 1:202:38, 1:202:42 |
| 5 | 5 | NEW | N4 | PP edge cases: -1 PP under Pressure (Showdown stops at 0); no PP spent when the foe is already gone | 1:88:6, 1:103:3, 1:386:41 |
| 5 | 4 | KNOWN | SR-26 (W7) | Choice lock outlives the item after Trick | 1:136:2, 1:305:4, 1:305:5 |
| 4 | 4 | KNOWN | C2, M05-3 (W28) | Smack Down marker without grounding | 1:203:7, 1:203:14, 1:203:19 |
| 4 | 4 | KNOWN | M05-4 (W28) | Armor Cannon does not lower defences | 1:198:0, 1:198:4, 1:490:4 |
| 4 | 3 | KNOWN | W7 (K6, X8) | No Struggle | 1:376:20, 1:382:45, 1:386:42 |
| 3 | 3 | KNOWN | M04-11 (W28) | Spiky Shield / Baneful Bunker do not fail when moving last | 1:8:0, 1:136:16, 1:160:3 |
| 3 | 3 | NEW | N5 | Failed consecutive Protect leaves the counter standing | 1:282:54, 1:382:2, 1:419:9 |
| 3 | 3 | RIG | - | Replacement-only step drops the waiting side's pending recharge | 1:25:18, 1:326:5, 1:393:3 |
| 3 | 3 | KNOWN | SP3 (W9) | Mega Absol Z ability | 1:180:10, 1:322:10, 1:498:9 |
| 3 | 3 | KNOWN | T19 (open question in the register; this run answers it) | A Pokémon fainting to Stealth Rock on entry must not set weather/terrain; the engine lets it | 1:57:15, 1:259:11, 1:356:33 |
| 3 | 2 | KNOWN | T7 (W8) | Both last Pokémon faint: engine always gives player two the win | 1:341:12, 1:415:17, 1:431:16 |
| 2 | 0 | KNOWN | I3 (W4) | Normal Gem used up although the move was blocked | 1:166:0, 1:192:13 |
| 2 | 1 | KNOWN | T18 (pointer only, no finding id, no W) | Move order on a Mega turn uses pre-Mega Speed | 1:21:10, 1:207:7 |
| 2 | 2 | KNOWN | M01-9 (W19) | Roost does not drop the Flying type for the turn | 1:218:7, 1:397:15 |
| 2 | 1 | NEW | N6 | Pain Split sets HP above the maximum | 1:110:0, 1:240:0 |
| 2 | 2 | NEW | N8 | Spicy Spray burns the user of a status move (Toxic) | 1:79:11, 1:79:13 |
| 2 | 2 | NEW | N10 | Trace copies Disguise | 1:218:2, 1:343:6 |
| 2 | 2 | KNOWN | W4, SR-16 | Effects pass through Protect (Trick, Water Absorb heal) | 1:382:38, 1:496:21 |
| 1 | 1 | KNOWN | A01-1 (W12) | Prankster field/self moves blocked by a Dark-type | 1:429:8 |
| 1 | 1 | KNOWN | A01-5 (W13) | Magic Bounce does not reflect stat drops | 1:289:15 |
| 1 | 1 | KNOWN | A02-6 (W29) | Harvest does nothing | 1:76:33 |
| 1 | 1 | KNOWN | I6 (W30) | Shell Bell cancels drain | 1:97:5 |
| 1 | 1 | KNOWN | M02-5 (W28) | Pain Split ignores Protect | 1:452:12 |
| 1 | 0 | KNOWN | M03-5 (W10) | Dire Claw side effect missing | 1:347:5 |
| 1 | 1 | KNOWN | M04-4 (W28) | Counter | 1:396:3 |
| 1 | 1 | KNOWN | M05-6 (W28) | Beak Blast does not burn on contact | 1:497:1 |
| 1 | 1 | KNOWN | M06-3 (W28) | Skill Swap does nothing | 1:44:8 |
| 1 | 1 | NEW | N11 | Double Shock keeps the user's Electric type | 1:480:6 |
| 1 | 1 | NEW | N7 | Pain Split goes through Good as Gold | 1:183:6 |
| 1 | 1 | RIG | - | Choice lock lost on load | 1:168:13 |
| 1 | 1 | KNOWN | SP6 (W9) | Mega Drampa Special Defense | 1:69:10 |
| 1 | 1 | KNOWN | M01-9 (W19) | Roost holder not grounded for Grassy Terrain | 1:42:6 |

47 steps had a difference that no rule covered. I assigned 37 of them by reading the step (the `MANUAL` list in `triage.py`) and left 10 as unclear (last row of the table and the Unclear section).

## New findings

**N1. The engine records a move as the "last move" (and Choice-locks it) before checking whether the Pokémon can move.** 237 steps (165 with no other cause). Showdown sets the last move only after the move passes its "can this Pokémon act" checks (sleep, freeze, paralysis, flinch, confusion self-hit); the engine records it first.
- Step 1:5:14: `sides.p1.pokemon[2].lastMove` Showdown `"rest"`, engine `"bodypress"`. Log: `|cant|p1a: Orthworm|slp`.
- Step 1:10:13: `sides.p2.pokemon[0].lastMove` Showdown `"flamecharge"`, engine `"swordsdance"`. Log: `|-activate|p2a: Charizard|confusion`, `|-damage|p2a: Charizard|17/155|[from] confusion` (confusion self-hit, so no move).
- Choice lock follows: step 1:17:6 (Choice Scarf Basculegion, `|cant|p1a: Basculegion|frz`): `volatiles.choicelock` Showdown absent, engine `{"move":"aquajet"}`, and `moves[0,2,3].disabled` false in Showdown, true in the engine. Step 1:382:37 (Choice Scarf Rotom, asleep): engine `choicelock {"move":"hydropump"}`.
- Why it matters in play: a Pokémon that stays asleep keeps Choice-locked into a move it never used, and Encore picks the wrong move to repeat.
- Source: engine `genx/generate_instructions.rs:2103-2110` (`set_last_used_move_as_move`) runs before `cannot_use_move` (`:2112`, flinch) and before the sleep, freeze, paralysis and confusion handling (`:2142` onward). Showdown: `sim/battle-actions.ts:312-348` returns at `BeforeMove` before `pokemon.moveUsed(...)` (`:348`, which sets `lastMove`, `sim/pokemon.ts:925-931`).
- Related register entries: T14 (PP spent when the move does not happen) is the same family (same missing "did it act" gate) but not the same fault; M01-5 is about Fake Out after a flinch.

**N2. The engine locks the user into Outrage/Thrash even when the move had no effect.** 8 steps (1:14:12, 1:18:11, 1:18:12, 1:195:20, 1:246:6, 1:453:5, 1:457:5, 1:320:8).
- Step 1:14:12: Garchomp uses Outrage on Mimikyu: `|-immune|p2a: Mimikyu`. Showdown: `moves[0,1,3].disabled` false; engine true. Step 1:246:6 also leaves a `confusion {}` volatile in the engine only. Step 1:320:8: Outrage was disabled (`|cant|p2a: Garchomp|Disable|Outrage`); engine still locks the other three moves.
- Showdown applies the lock as the move's own effect (`data/moves.ts:13088-13090`, `self: { volatileStatus: 'lockedmove' }`), which only happens when the move connects. The engine side was not traced in source. W1 and T6/X6 cover the lock length and what a locked Pokémon may choose; none says the lock must not start on a failed move.

**N3. The engine takes PP again on the second turn of a charge move and on every turn of Outrage/Thrash.** 17 steps (9 with no other cause). Showdown takes PP once: it skips the deduction when the Pokémon is in a locked move (`sim/battle-actions.ts:338-346`, `getLockedMove()`).
- Step 1:39:11 (Electro Shot, second turn, log `|move|p2a: Archaludon|Electro Shot|...|[from] lockedmove`): `moves[1].pp` Showdown 9, engine 8. Step 1:323:16 (Phantom Force release): `moves[3].pp` 9 against 8. Step 1:138:36 (Solar Beam release, after an earlier PP difference): 8 against 6.
- Engine: `generate_instructions.rs:2118-2135` decrements PP on every execution when PP is below 10 and has no case for the release or continuation turn.

**N4. Two PP edge cases.** 5 steps.
- PP goes below zero: steps 1:386:41, 1:388:74, 1:388:76. The move had 1 PP left and the foe has Pressure. Showdown ends at 0, the engine at -1 (`moves[2].pp` 0 against -1). Engine `:2122-2135` takes 2 and does not stop at 0.
- No PP spent when the foe is already gone: steps 1:88:6 and 1:103:3 (`|move|p1a: Gholdengo|Make It Rain|p2: Basculegion|[notarget]`, `|-fail`). `moves[3].pp` Showdown 7, engine 8: Showdown still takes the PP, the engine returns at `cannot_use_move` (`:1486-1491`, `:2112`) before the PP line.

**N5. After a consecutive Protect fails, the engine keeps the Protect counter.** 3 steps (1:282:54, 1:382:2, 1:419:9). Step 1:282:54: Showdown's random Protect check failed (`protect` event, value false, p 0.667) and the `stall` volatile is gone; the engine's failure branch still has `volatiles.stall {"counter":3,"left":1}`. The next Protect would again have only a 1/3 chance in the engine and a certain success in Showdown. Engine: the fail branch is cut off at `generate_instructions.rs:1863-1871` without clearing `side_conditions.protect`. M04-11 is about moving last, not this.

**N6. Pain Split can set a Pokémon's HP above its maximum.** 2 steps. Step 1:110:0: Ninetales-Alola `hp` Showdown 150 (max 150), engine 153; Showdown log `|-sethp|p1a: Ninetales|150/150|[from] move: Pain Split|[silent]`. Step 1:240:0: Meowscarada 153/153 in Showdown, 155 in the engine. (M02-5 only says Pain Split ignores Protect and misses.)

**N7. Pain Split goes through Good as Gold.** 1 step (1:183:6). Log: `|-immune|p1a: Gholdengo|[from] ability: Good as Gold`. Gholdengo `hp` Showdown 12, engine 34. This is the opposite of SR-23 (which says the engine blocks too much).

**N8. Spicy Spray burns the user of a status move.** 2 steps (1:79:11, 1:79:13). Toxapex uses Toxic on Mega Scovillain (Spicy Spray); `sides.p1.pokemon[2].status` Showdown `""`, engine `"brn"`. A02-5 covers Spicy Spray through a Substitute, not on status moves. The engine source was not read for this one.

**N9. After the last opposing Pokémon faints in mid-turn, the engine lets the winner use its pending move.** 10 steps (1:106:13, 1:127:10, 1:136:25, 1:193:13, 1:197:18, 1:252:24, 1:259:13, 1:303:21, 1:311:10, 1:326:22). Step 1:136:25: Meowscarada's Life Orb kills it after Triple Axel, the log ends `|win|p1`; Toxapex's Recover never happens in Showdown. Engine: `moves[1].pp` 3 against 4 and `lastMove` `"recover"` against `"infestation"`. Probable cause: `cannot_use_move` (`generate_instructions.rs:1486-1491`) only stops non-status moves against a fainted foe. T8 is about end-of-turn damage after the win, not about moves.

**N10. Trace copies Disguise.** 2 steps (1:218:2, 1:343:6). Gardevoir (Trace) comes in against Mimikyu: `ability` Showdown `"trace"` (nothing copied), engine `"disguise"`. Showdown marks Disguise `notrace` (`data/abilities.ts:1010`); the engine copies any ability (`engine/src/genx/abilities.rs:1354`). Trace is not mentioned anywhere in the register. (Two other Trace-looking differences, 1:153:10 and 1:343:10, are T5, switch order.)

**N11. Double Shock does not remove the user's Electric type.** 1 step (1:480:6). Log: `|-start|p1a: Pawmot|typechange|???/Fighting|[from] move: Double Shock`. `types` Showdown `["fighting"]`, engine `["electric","fighting"]`. The register only lists Double Shock's missing punch tag (A01-10, M03-7).

**N12. Mega Evolution is offered to a Pokémon that is locked into Outrage.** 3 legal-action failures (1:155:7, 1:155:8, 1:258:1). `p1.mega` Showdown false, engine true; options otherwise equal (`moves: [2]`, no switches). W7 covers locked moves but not the Mega flag.

## Rig faults

- **R1. A replacement-only step lets the engine play a whole turn for the waiting side.** 3 steps (1:25:18, 1:326:5, 1:393:3). Showdown still has `volatiles.mustrecharge {"left":1}` on the side that waits for the other side's replacement; the engine has dropped it (and the four moves are "disabled" in Showdown, not in the engine). The decision loaded in the engine is a turn boundary, so the recharge was spent. Suspected, not proven.
- **R2. Choice lock lost on load.** 2 decisions (load check: 1:168:13, 1:168:14): `sides.p1.pokemon[2].volatiles.choicelock` Showdown `{"move":"wavecrash"}`, engine absent after load. The loader rebuilds the lock from the `disabled` flags (`rig/src/snapshot.rs`, table row `volatiles.choicelock.move`), which fails for this Pokémon. Step 1:168:13 is the same thing seen in the step comparison.
- **R3. Fields the engine cannot hold are reported as differences.** The engine exports no frozen-turn counter, no confusion counter, no trap counter or `boundDivisor`/`move`, no `lockedmove.trueDuration` (`rig/src/snapshot.rs`, "not exported"). These show as differences in 13, 35, 19 and 61 fields. The cause is real and in the register (W16, W1), so I counted them as KNOWN, but the comparer should say "not modelled" instead of "different" so these 100-odd differences stop hiding other faults in the same steps.
- **R4. Game-end snapshots.** After a win Showdown leaves bookkeeping volatiles (`twoturnmove`, `flinch`, `stall`) that the engine has already cleared (1:39:11, 1:138:36, 1:323:16, 1:340:16 and others). The engine really does run the end of the turn after a win (T8), but part of the T8 differences are only this bookkeeping. The comparer could skip volatiles and counters when `ended` is true.
- **R5. Aura Guard.** The engine has no Aura Guard, so the loader substitutes Adaptability (ability pair `auraguard`/`adaptability` in 37 steps). That is SP4 in the register, but it also changes STAB, so Mega Lucario Z damage is wrong in both directions (8 of the 57 out-of-range damage values).
- **R6. Weights (Grass Knot).** If the rig supplies the weights, Grass Knot damage differences (2 values, ratio 1.12 and 1.21) may be the rig's table; the register says the engine takes weights from the caller (SP14, W11).

## Unclear

16 steps. What I would need is given with each.
- Seven steps where, after a pivot move (U-turn, Flip Turn), the engine appears to have already played the other side's pending move: 1:31:16, 1:290:9, 1:296:17, 1:302:25, 1:318:7, 1:393:7, 1:433:12. Example 1:21:10 is the Mega-turn order (KNOWN), so some of these may be too. I need the engine's state at the force-switch (does `branches` stop at the pending switch?) and the move order for each.
- 1:78:15 (Strength Sap and Taunt: Attack stages 0/-1 and 5/6), 1:353:6 and 1:177:12 (Parting Shot: Attack and Special Attack stages): stat-stage differences I could not trace to a cause. Need the engine's event order (Parting Shot may switch the user out when it did nothing, M04-8).
- 1:122:7: `volatiles.trapped` Showdown `{}`, engine absent, after No Retreat (the register says the opposite, M06-7).
- 1:123:5 and 1:128:47: `lastMove` differences with no move-order or KO explanation (Roar and Whirlwind both failing; engine says Stealth Rock).
- 1:285:14: `volatiles.stall` present in Showdown only after an ordinary Protect that held.
- 1:321:18: Detect's `protect` volatile present in Showdown, absent in the engine.
- 1:349:18: the engine's Thunderbolt branch paralysed Toxapex, Showdown's did not. Probably the driver picked the nearest branch badly (a rig fault: the nearest-branch rule counts differences, then HP, and the Thunderbolt paralysis chance is a 10% branch).
- Make It Rain damage, 5 of 16 values (1:81:9, 1:196:6, 1:244:5, 1:353:12, 1:443:8): the engine hits harder than Showdown (ratio 1.10 to 1.18) although M01-1 says the engine's power is too low (95 against 120). I would need the user's Special Attack stage in both programs.
- Damage 1:91:1 (Body Press, ratio 1.48), 1:471:12 (Shadow Sneak by Mega Banette, ratio 2.33), 1:488:35 (Earthquake, Hippowdon into Hippowdon, ratio 0.74): no register entry fits; I need the full damage calculation inputs.

## Damage values outside the roll range (57)

These are the 57 values of the report's roll-range check (damage-only steps, one hit, one roll). The ratio is engine damage divided by Showdown damage. The engine uses the average roll, so a gap within 0.85 to 1.0 of the hit is expected; the 57 lie beyond it.

| Values | Cause | Verdict | Register ids |
|---:|---|---|---|
| 17 | Moves with no special rule, ratio 0.74 to 1.22 (Fire Blast, Flamethrower, Rock Tomb, Sucker Punch, Rapid Spin, Smack Down, Close Combat and others). Consistent with damage rounded at the wrong steps and the roll applied at the wrong place. | KNOWN | F1, F2, F3, W2 |
| 16 | Make It Rain. 11 values have ratio 0.72 to 0.87 (power 95 against 120 gives 0.79). 5 values are 1.10 to 1.18 and are not explained. | 11 KNOWN, 5 UNCLEAR | M01-1 (W10) |
| 8 | Mega Lucario Z as attacker (STAB doubled by the substituted Adaptability: 1:39:0, ratio 1.34) or defender (no Aura Guard halving of contact moves: Sucker Punch 2.00, Ceaseless Edge 2.07, Bullet Punch 2.12, Aqua Jet 2.18, Shadow Sneak 1.92) | KNOWN | SP4 (W9) |
| 4 | Mega Excadrill with Piercing Drill, Iron Head and Brick Break ratio 0.36 to 0.59: the quarter damage is applied when the target is not protecting | KNOWN | A02-1 (W29), A06-1 |
| 4 | Bellibolt (Electromorphosis) Volt Switch and Parabolic Charge, ratio 0.51 to 0.83: Showdown's Charge doubles the next Electric move | KNOWN | A02-6 |
| 2 | Rage Fist, ratio 0.60 and 0.48 (Showdown's power grows each time the user is hit) | KNOWN | M05-3 |
| 2 | Grass Knot, ratio 1.12 and 1.21 (weight) | KNOWN, rig suspected | SP14, W11 |
| 1 | Double Shock by Pawmot (Iron Fist), ratio 0.72: the missing punch tag | KNOWN | A01-10, M03-7 |
| 3 | Body Press 1:91:1, Shadow Sneak 1:471:12, Earthquake 1:488:35 | UNCLEAR | none |

## Legal-action failures (19 decisions)

- 16 decisions: Showdown offers `["struggle"]`, the engine offers no moves (1:100:1, 1:100:2, 1:168:14, 1:376:20, 1:376:21, 1:382:45, 1:382:46, 1:386:42 to 1:386:44, 1:388:77 to 1:388:81, 1:390:10). KNOWN: no Struggle (W7, audit K6, X8). The two engine errors (1:100:1 and 1:100:2, "move struggle is not a legal choice") are the same fault seen from the other side.
- 3 decisions: Mega offered while locked in Outrage: N12.

## Load check (2 decisions) and engine errors (2)

- Load check: both are the choice-lock loss (R2).
- Engine errors: both are `move struggle is not a legal choice` (W7).
