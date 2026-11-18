# Completeness of the audit (step 4)

17 Nov 2026. Four checks of the read audit, done before the gap register is built. Tools are in `tools/`;
second-read files are in `second-read/`. Ids (CM, DD, SR, X) match `../../../gaps/INDEX.md` and the reports
given to the user.

## Result in one table

| Check | What was checked | Result |
|---|---|---|
| Rows | Every row of the four lists has an audit row (`tools/completeness.py`) | 515 moves, 317 abilities, 166 items, 392 species: all covered once. Three rows the readers skipped were found and audited (X1 Hyper Beam, X2 Zing Zap, X3 Ice Body) |
| Step 3, data by program | Engine compiled and dumped; every number, type and flag diffed (`../data-diff/09-data-diff.md`) | 23 moves, 6 Megas, 22 items, 1 ability with real data errors. The first read had found all but one move (X4 Dragon Cheer) |
| 4a, second read | A second reader, blind to the first, on every used row (`tools/compare_reads.py`) | 657 rows read twice. 3 real faults the first read missed (SR-16, SR-18, SR-22), 1 more not yet re-read by the manager (SR-26), about 14 details extending known faults |
| 4b, `pcsa` feature code | Every move, ability, item and condition named in `pcsa`'s feature, damage and player code (`tools/pcsa_lists.py`) | 272 distinct legal entries named; all have an audit row; 149 already in the register as wrong. Every volatile, weather, terrain, room and side condition in `pcsa`'s lists is either in the engine, in the register, or not legal in Champions (CM-1 to CM-5) |
| 4c, Showdown handlers | Every legal entry with handlers has a row with a verdict | 1,227 handler pairs across 693 entries; no entry without a row; 9 rows flagged as thin (CM-8, CM-9) |
| 4d, descriptions | Every legal entry whose description states an effect has a row | 976 entries with an effect; none without a row; 55 "plain" moves with an effect field confirmed on data by step 3 (DD-8) |

## Second read in numbers (4a)

| List | Rows read twice | Same class | Different | Second reader stricter |
|---|---|---|---|---|
| Moves (all 366 used) | 366 | 309 | 57 | 19 |
| Abilities (all 167 used) | 167 | 139 | 28 | 19 |
| Items (all 124 used) | 124 | 53 | 71 | 2 |

"Same class" means both readers called the row fine (MATCH, PLAIN, CORE) or both called it wrong. For items,
69 of the 71 differences are Mega stones on which the readers agree about the facts: every stone's form,
stats, types and ability, and the one shared fault (I12). The first reader marked each stone as wrong for
that fault; the second noted it once.

Where the first reader was stricter, the causes repeat and are confirmed elsewhere: recoil, drain and heal
rounding (F7, F8), never-miss moves stored as accuracy 100 (DD-7, 135 moves), the sleep rule (C1), missing
move tags (step 3).

The second readers also missed things the first found. Example: the second reader of items batch 2 recorded
Mega Drampa's Special Defense as matching; Showdown has 116 and the engine 114 (SP6, confirmed by step 3).

## What the checks do not cover

- Unused rows were read once. The plan's 10% sample of unused rows was not done; the 149 unused moves, 150
  unused abilities and 42 unused items rest on the first read and on step 3 for their data.
- 4c and 4d confirm that a row exists and is not empty. They do not prove that each handler or each stated
  effect was judged correctly; that is what the second read did, for used rows only.
- PP is not compared anywhere: the engine has no PP field (known core difference K6).
- The three mechanics slices (turn structure, conditions, formulas) were read once and not re-read.
- Interactions between two effects in a running battle are outside what reading finds. The forced-outcome
  tests of step 7 are the check for those; anything they find that is not in the register is counted as an
  audit miss.
