# Second rig run: rig faults fixed, comparison with the first run (2026-11-17)

Input: `2026-11-17-second-run.jsonl.gz` (seed 1, battles 0-499, the same battles and choices as the first run:
the driver draws its choices from Showdown's options only). Report: `2026-11-17-second-run.md`. Cause labels:
`2026-11-17-second-run-causes.txt` (`python tools/rig/triage.py <run> --causes`, rules as now in `triage.py`);
the first run's labels with the rules as committed are in `2026-11-17-first-run-causes.txt`.

## What was fixed, and what turned out not to be the rig

| Item | Verdict | What was done |
|---|---|---|
| R1. Replacement step drops the waiting side's recharge (1:25:18, 1:326:5, 1:393:3) | **engine**, not the rig | Reproduced. The snapshot loads with `mustrecharge` intact (load check equal). Engine-only chain for 1:393:2 to 1:393:3: the engine's own branch after the Hyper Beam turn has `mustrecharge`, and resolving the replacement on that engine state removes it. Cause: on a replacement the engine offers the waiting side `MoveChoice::None` (`engine/src/genx/state.rs:1230-1238`), and a `None` choice is played as the recharge turn (`generate_instructions.rs:1901-1915` removes `MUSTRECHARGE`). In the engine's own play a Hyper Beam that knocks out is never recharged. Labelled `N14_replace_spends_recharge` in `triage.py` (N14 is a proposed id; the register is not edited). |
| R2. Choice lock lost on load (1:168:13, 1:168:14) | rig | The lock lives in the moves' `disabled` flags; with the locked move also Disabled, all four are disabled and the export could not name the lock. Export now takes the last move used when a Choice item holds and every move is disabled (`rig/src/snapshot.rs`). Load check: 2 decisions came back different, now 0. Test `choice_lock_on_a_disabled_move_survives_load`. |
| R3. Fields the engine cannot hold counted as differences | rig | The engine rig declares them on every `load` as `*.`-entries of `dropped` (`snapshot::NOT_MODELLED`): freeze turns, confusion, partial trap (`left`, `boundDivisor`, `move`), `lockedmove.trueDuration`, and the other counters the snapshot table marks "not exported" (Disable, Heal Block, Throat Chop, Magnet Rise, Roost). The driver sets aside a difference where Showdown has such a field and the engine does not, and counts it once per kind (report section "Not modelled by the engine"). Whether the condition itself is present is still compared. |
| R4. Volatiles of a finished battle | rig | Not compared once Showdown's battle has ended, and neither are the moves' `disabled` flags (they follow those volatiles and no decision is left); 114 differences set aside. |
| R5. Aura Guard / Adaptability | **engine**, not the rig loader | The loader already loads Aura Guard as no ability and lists it in `dropped` (checked: 1:28:6 loads `ability ""`, dropped `p2.pokemon[2].ability:auraguard`). Adaptability appears only on the step where Lucario Mega Evolves inside the engine, from the engine's own Mega table (`engine/src/genx/base_stats.rs:662-666`, `ability: Abilities::ADAPTABILITY`): SP4. Added: team building lists `mega-ability:auraguard` in `unknown` when a set's Mega stone leads to a Mega whose ability the engine lacks. The 37 SP4 steps stay, as engine differences. |
| Driver timeout | rig | 10 s per request (`--timeout`); on a timeout the step is recorded as `engine_error` or `oracle_error`, that child is killed and restarted, the battle is given up and the run goes on. Tested with a stand-in engine that stops answering (3 battles, 3 timeouts, 3 restarts, run completed). |
| Unclear: 7 steps "engine played the pending move after a pivot" (1:31:16, 1:290:9, 1:296:17, 1:302:25, 1:318:7, 1:393:7, 1:433:12) | **engine** (known and N1); the label was a fault of the triage rule | None is a pivot fault. 5 are N1 (the Pokémon could not act: `|cant|...|slp` in 1:31:16, 1:296:17, 1:302:25, 1:318:7; confusion self-hit in 1:433:12) and the engine still records the chosen move as the last move. 1:290:9 is K8 (Mega Garchomp Z: Speed 169 before Mega Evolution, 223 after; Choice Scarf Basculegion 214; the engine orders by 169, so Flip Turn goes first and Garchomp's move is saved). 1:393:7 is W24 (Golisopod's Emergency Exit stops the turn in Showdown; the engine plays its U-turn, which raises Archaludon's Defense through Stamina). The rule checked "no `|turn|` line" before "could not act"; it now checks the log for `|cant|` and confusion self-hits first and for Emergency Exit. |
| Unclear: 1:349:18 | rig | Bad nearest branch, as suspected: the trap's three uncounted counters weighed three differences, so the branch where Toxapex was fully paralysed (one whole missing volatile and a status) looked nearer. With R3 the right branch (no paralysis) is chosen and the step is damage-only. The nearest-branch rule now also counts a volatile once, however many fields differ in it. |
| Probability check | added | Report section "Probability". |

## Steps per class

| | first run | second run |
|---|---:|---:|
| steps | 9,848 | 9,848 |
| exact | 2,340 | 2,352 |
| damage_only | 237 | 238 |
| mismatch | 6,924 | 6,911 |
| skipped | 345 | 345 |
| engine_error | 2 | 2 |
| PP shortcut set aside: exact | 4,763 | 4,795 |
| PP shortcut set aside: damage_only | 2,863 | 2,916 |
| PP shortcut set aside: mismatch | 1,875 | 1,790 |
| legal-action failures | 19 | 19 |
| load-check differences | 2 | 0 |
| timeouts | (no timeout then) | 0 |

Of the 1,875 mismatch steps (PP aside) of the first run, 53 are now damage-only and 32 exact; no step moved the
other way. Unlabelled steps: 10, now 7 (1:78:15, 1:122:7, 1:123:5, 1:128:47, 1:177:12, 1:285:14, 1:353:6, the
same stat-stage, trap, `lastMove` and Protect puzzles listed in the first triage; 1:349:18 and 1:393:7 left the
list, 1:321:18 left it because Detect's volatile is no longer compared after the win).

## Cause labels that changed (steps, steps where it is the only cause)

| Label | first run | second run | Why |
|---|---|---|---|
| `RIG_replace_recharge` | 3, 3 | gone | engine: now `N14_replace_spends_recharge` 3, 3 |
| `RIG_choicelock_load` | 1, 1 | gone | fixed (R2); 1:168:13 is exact |
| `N10_midturn_saved_move` | 7, 4 | gone | split into N1, K8, W24 (above) |
| `W16_freeze` | 13, 8 | gone | the freeze counter is not modelled (R3) |
| `W1_trap` | 24, 19 | 2, 1 | trap counters not modelled (R3); 2 steps where the trap itself differs |
| `W1_confusion` | 64, 13 | 30, 4 | confusion counter not modelled; presence still compared |
| `W1_lock` | 112, 50 | 97, 63 | `trueDuration` not modelled; finished battles |
| `C2_throatchop` | 23, 15 | 10, 7 | counter not modelled |
| `M03-10_healblock` | 20, 15 | 11, 8 | counter not modelled |
| `T8_afterwin` | 189, 168 | 173, 167 | volatiles and flags of finished battles (R4) |
| `W7_fakeout` | 159, 108 | 155, 110 | flags of finished battles |
| `C3_glaiverush` | 18, 15 | 14, 12 | finished battles |
| `M01-7_C10_encore` | 14, 12 | 11, 9 | finished battles |
| `M02-1_curse` | 37, 29 | 35, 29 | finished battles |
| `N1_lastmove_notmoved` | 237, 165 | 242, 172 | +5 from the pivot list |
| `K8_mega_order` | 2, 1 | 3, 1 | +1 (1:290:9) |
| `W24_ee` | 18, 18 | 19, 19 | +1 (1:393:7) |
| `T17_disable` | 7, 6 | 9, 6 | disabled flags under a Disable whose counter is not compared |

Smaller shifts (W2, A02-6/M07-3, A01-4, A01-2, T4, N3, N9, M03-8, SP4, SP5, W26) are one to three steps each,
where a step lost a co-cause and became a "only this cause" step or vice versa.

## Engine differences found while checking rig faults

- **N14 (proposed). A replacement step spends the waiting side's recharge.** 3 steps (1:25:18, 1:326:5,
  1:393:3). Evidence above.
- SP4 (known): Mega Lucario Z gets Adaptability from the engine's Mega table on the Mega Evolution step.
- The pivot list: N1, K8, W24, all known.

## Contract points (not edited)

- Section 4 says a field the engine has no concept of is reported as a difference "which is the point". Section 7
  now needs a point for fields the engine has the condition for but no counter: declared as `*.`-entries of the
  `load` reply's `dropped`, set aside by the driver, reported once per kind.
- Section 6 should say the comparer does not compare volatiles or `disabled` flags once Showdown's battle has ended.
- Section 7.13: the engine's `new` reply may list `mega-ability:<id>` in `unknown`.
- The driver now writes a short `prob` line for every exact step (for the probability check); the contract says
  nothing about which steps are written.
- `RIG_ENGINE` (engine command, arguments separated by `|`) and `--timeout` are driver options, not contract.
