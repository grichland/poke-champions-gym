# 08 - Observation, action space, rule masks, opponent bot and per-step bookkeeping

Investigator 08. Read-only study (files and grep only, nothing run). Dates: sources read 16 Nov 2026.
Paths: LL = `C:\Users\user\Code\research\pcsa-rl-low-latency` (worktree, branch low-latency, untracked files), RL = `C:\Users\user\Code\research\pcsa-rl`, PCSA = `C:\Users\user\Code\pcsa`.

## Summary (10 lines)

1. `LL\rl2\features_ll.py` (697 lines) is code-complete as an encoder and has a layout test and a 6-mode benchmark, but it is NOT wired into any model, player, trainer or config. No network has ever been trained or evaluated on it, so there is no win-rate or accuracy comparison. `docs/low-latency.md` (promised by its docstring) does not exist.
2. ll observation per decision: `rows [12,268] f32` (69 raw + 4x37 moves + 4 history + 31 lookup + 16 damage), `ids [12,7] i64`, `field [55] f32`, `last [20] f32`, `last_ids [4] i64`, `mask [11] bool`, plus `turn:int`, `forced:bool`. Base/belief v2 observation it replaces: rows `[12,338]`, ids `[12,10]`, field `[126]`, same last/mask.
3. Timing per decision, 5,015 recorded states (mean ms): base 1.24, belief+mega 4.74, ll raw 0.107, raw+lookup 0.150, raw+lookup+dmg 0.240 (cold tracker 0.355). About 11x faster than base and 20x faster than belief. These exclude the Battle rebuild (0.37 ms median) and the network.
4. Parity: 0 shape/finite/empty-mask failures on 4,926 live + 6,009 sim states; incremental tracker differs from cold tracker on 20 of 4,926 live states (cause not recorded). Closed-form damage: midpoint error 6-9% of max HP, engine midpoint inside [min,max] 95-99.9%, KO-possible recall 99.2-99.9% with 12-16% false positives; misses cluster on Garchomp targets (unexplained).
5. ll still depends on information-set bookkeeping (revealed items/abilities/moves, opposing HP percent, Mega status, bench reveal order) and on an incremental protocol tracker (entries, who-moved-first, forms, last-turn token). It needs no damage engine, no belief, no catalog; the optional `dmg` group is a closed-form estimate with about 25 hand-coded ability/item/move special cases.
6. Action space = 11: slots 0-3 moves, 4-7 same moves with Mega, 8-10 switches to bench slots in Showdown request order (slot 10 never legal in 3v3). Team preview is a separate 60-way choice (lead + 2 ascending), not in the 11.
7. Nine live rule masks: 2 trivial (no_full_heal, useless_status), no_effect medium (type/ability/item tables), encore_switch needs the whole-battle protocol walk, sucker_punch / mega_weakness / priority_ko need the OPPONENT HEAD's output (post-network), and locked_futile / yawn_switch / mega_weakness / priority_ko need the damage engine with catalog-assumed opposing builds plus a speed calculation. These conflict with the "no damage engine" rule; masks cannot all be pure observation-time arrays.
8. max_damage opponent: argmax over legal moves of `tactics.attack_score` (power x STAB x atk/def ratio x accuracy x hits x type multiplier, midpoint stats for the unknown opponent, no switching, Mega whenever offered); training uses it wrapped in `Stochastic` (`max_damage~`, KEEP 0.85/0.65/0.45, KO floor 0.8, which calls the full damage engine for the KO check).
9. Reward: +0.1 per opposing faint, -0.1 per own faint (credited to the decision before), terminal +1/-1 on the last decision, 0 for a tie; gamma 0.999, lambda 0.9, no bootstrap after the last step. Showdown ties at turn > 1000; the only other limit is a 120 s wall-clock battle timeout that discards the battle.
10. Phone-side differences: crits never logged, Scarf/items only via reveals, opposing HP as a screen percent, disabled/PP of our moves reconstructed by pcsa rules, bench PP and opposing PP from poke-env's `pp*8//5` rule, preview species can be misread.

---

## 1. Status of the low-latency observation

### 1.1 What exists (verified)

| Item | Where | State |
|---|---|---|
| Encoder | `LL\rl2\features_ll.py` (697 lines) | Complete: tables (`chart`, `form`, `move_facts`, `_type_mult`), incremental `Tracker`, `encode`, `_field`, `_damage`, `observe`, `preview_observations`. Untracked in the worktree. |
| Unit tests | `LL\rl2\tests\test_features_ll.py` (75 lines) | Layout (`WIDTH == 268`, offsets 69/217/221/252), type chart, form bounds vs `pcsa.battle.damage.investment_bounds`, move facts, incremental==cold tracker on a toy protocol, `_type_mult`. No test of the full `encode` output values. Not known to have been run (not run here). |
| Bench script | `LL\rl2\scripts\ll_bench.py` (modes time-base, time-belief, time-ll, parts, dmg, parity) | Complete. Capped at `--minutes` 7.5 default. |
| Laptop launchers | `LL\configs\laptop\ll_bench_start.ps1`, `ll_bench_window.ps1` | Run the bench on the AWL laptop in a visible window (paths `C:\Users\grich\...`). |
| Results | `RL\runs\low-latency\*.json`, logs, `jobs.log` ("=== all jobs done 2026-11-16T15:08:03") | All six modes finished. |
| Design doc | `LL\docs\low-latency.md` | DOES NOT EXIST (named in `features_ll.py:5`). `LL\docs\` holds only design-v1/v2 html and two plan md files; grep of "latency", "features_ll" across `RL` (excluding .venv, runs), `RL\notes` (3 files) and `LL\docs` found nothing else. |

### 1.2 Not wired in (verified by grep)

`features_ll` is referenced only by `ll_bench.py`, its own test file and the docstrings. `rl2/model.py` (RL and LL) still takes `fx.WIDTH` rows (338), 126-float field and 10 ids (`RL\rl2\model.py:89-91,131-135`); its constants `HP_COL = 34`, `COULD_MEGA_COL = fx.ROW + 12` (`model.py:27-28`) differ from ll's `HP_COL = 30`, `COULD_MEGA_COL = 64` (`features_ll.py:36`). `rl2/player.py` has no "ll" feature mode (`MODES = base|belief|belief+mega`, `belief_features.py:19`). No config in `LL\configs` names it. Therefore: no checkpoint, no win rate, no policy-accuracy comparison exists. Using ll needs a new model input stem (268-wide rows, 7 ids, 55 field) and training.

### 1.3 Timing (ms per decision, mean of 3 repeats over 5,015 states: 2,508 live + 2,507 sim; `time-*.json`)

| Variant | mean | median | p95 |
|---|---|---|---|
| base (`fx.observe`) | 1.236 (reps 1.273/1.214/1.222) | 1.26 | 1.52-1.62 |
| belief+mega | 4.745 (4.911/4.663/4.661) | 4.55-4.74 | 7.9-8.5 |
| ll raw (hist group only) | 0.108 | 0.106 | 0.13-0.14 |
| ll raw+lookup | 0.150 | 0.149 | 0.18 |
| ll raw+lookup+dmg | 0.240 | 0.238 | 0.27 |
| ll raw, cold tracker (re-feed whole protocol) | 0.208 | 0.194 | 0.34 |
| ll raw+lookup+dmg, cold tracker | 0.355 | 0.342 | 0.50 |

Parts (`parts.json`, 15,045 samples): tracker feed incremental 0.024 ms (cold 0.122); encode none 0.0735; hist columns about 0; lookup +0.0375; dmg +0.0885 (0.1036 without lookup); legal mask 0.0047; last-turn token 0.005. Python on one core of the laptop. Excluded from all numbers: rebuilding the poke-env Battle from protocol (0.37 ms median live, 0.49 sim, `rebuild_ms_*`) and the network pass. In Rust the Battle object disappears, so these are only a ceiling for the Python path.

### 1.4 Parity (`parity.json`)

- live: 4,926 states, "ok in every configuration" 4,926 (finite, shapes, non-empty mask, |value| <= 2 for raw/lookup/dmg); incremental tracker == cold tracker 4,906, != 20 (0.4%). Reason not recorded anywhere. sim: 6,009/6,009 equal. preview: 464/464 ok.
- Failures: "live: no legal action" 8, "sim: no legal action" 2, two sim rebuild ValueErrors ("p1's team already has 3 pokemons", Whimsicott/Heracross/Polteageist case). These are rebuild/legal-action failures, not encoder failures.
- "Parity" here means self-consistency of the encoder, NOT bit equality with the base observation (columns differ by design) and NOT equality with any Rust implementation.

### 1.5 Closed-form damage estimate vs the real engine (`dmg-live.json` 4,926 states, `dmg-sim.json` 3,004 states; `dmg-smoke` = `smoke.json`)

Reference engine = `pcsa.players.threat` fighters (our exact set; their catalog-assumed set). Errors in % of target max HP.

| | live ours | live theirs | sim ours | sim theirs |
|---|---|---|---|---|
| pairs | 14,693 | 2,923 | 6,019 | 2,573 |
| mean midpoint abs error | 7.78 | 9.04 | 6.14 | 9.51 |
| median / p95 | 4.48 / 26.42 | 5.81 / 28.88 | 3.71 / 18.87 | 5.73 / 29.56 |
| engine midpoint inside [min,max] | 96.4% | 95.3% | 95.9% | 99.9% |
| mean interval width | 38.5 | 35.7 | 34.1 | 45.3 |
| engine 0, estimate > 0 | 36 | 0 | 100 | 0 |

KO flags (our legal moves): live recall KO-possible 0.999 (3,375/3,378), false positive 16.2% of non-KO moves; KO-certain recall 0.56, false positive 0.05%; KO-midpoint recall 0.915, FP 3.6%. Sim: possible 0.9915 / FP 11.7%; certain 0.53 / 0.7%; midpoint 0.9115 / 2.1%. Decisions with a legal KO move: live 1,969 of 4,199 (KO-possible flagged on all 1,969; certain on 1,051).

Problems visible in the results:
- All listed missed-KO examples (3 live, 14 sim; `missed_examples`) have target Garchomp and the estimate is far below the engine (e.g. Ice Beam 0.284-0.623 vs engine 0.930-1.103). The cause is not diagnosed in any file (candidates, inferred: form/type handling of Garchomp / Mega Garchomp Z in the estimate, or the engine's assumed Garchomp set). Treat the estimate as unverified for Ground/Dragon targets.
- `zero_disagreements`: the engine assumes unrevealed abilities (Levitate on Garchomp, Good as Gold on Gholdengo, Water Absorb on Vaporeon) and so reports 0 where the estimate (known abilities only) reports damage. This is an information-set difference by design, not a bug.
- The estimate is compared against an engine that itself guesses the opponent build; it is not compared against true Showdown damage.

### 1.6 Unfinished / inconsistent (verified unless marked)

1. No model, no training, no live use, no win-rate data.
2. `docs/low-latency.md` missing; the column table promised in the docstring is not written (this file's section 2 reconstructs it from code).
3. `Tracker.feed` calls `self.reset()` twice in a row (`features_ll.py:244-245`), harmless.
4. 20 live incremental-vs-cold tracker mismatches unexplained (parity.json:25-26).
5. Test file covers tables and tracker only; `encode`/`_damage`/`_field` have no value tests.
6. Estimate misses on Garchomp unexplained (1.5).
7. Benchmarks cover the encoder only; the p2-seat mirrored Battle rebuild per decision (`rl2/lanes.py:111-118`) is not part of the number.

---

## 2. Exact observation spec (ll mode) and what it derives from

Source of truth: `LL\rl2\features_ll.py`. Everything is built per decision from a poke-env `Battle` in the p1 view (a p2 seat is mirrored to p1: `PCSA\src\pcsa\players\mirror.py:47-51`, `RL\rl2\lanes.py:111-118`).

### 2.1 Tensors

| Name | Shape | dtype | Notes |
|---|---|---|---|
| rows | [12, 268] | float32 | `features_ll.py:404` (zeros; cols 0-45... below) |
| ids | [12, 7] | int64 | species, item, ability, move1..move4 (vocab ids; 0 = unknown/empty) |
| field | [55] | float32 | `_field` (`:359-397`) |
| last | [20] | float32 | `rl2.features.last_turn` (RL `rl2\features.py:225-252`), from `rl2.turns` records |
| last_ids | [4] | int64 | our move, our switch-in species, their move, their switch-in species |
| mask | [11] | bool | legal actions (section 4) |
| turn | int | | `battle.turn` (0 at preview) |
| forced | bool | | `request.forceSwitch` or no move/mega action legal (`:681-682`) |

Row layout: rows 0-2 = our trio in REQUEST order (active first), rows 3-5 always zero (present flag 0), row 6 = their active (or the most recently entered if none is active), rows 7-11 = their other revealed members in poke-env insertion order (order of first appearance), then preview-only members in preview order, max 6 (`_their_members` `:304-317`). Ours: `f1._request_members(battle)[:3]` (`RL\rl\features.py:89-95`).

Float size per decision: 12x268 + 55 + 20 = 3,291 floats. Base v2 for comparison: rows [12,338] (`fx.ROW`=318 + `EXTRA`=20, `RL\rl2\features.py:28-31`), ids [12,10], field [126].

### 2.2 Row columns (268), in order

All indices are columns of one row; "R" = this member, "T" = the opposing active (row 6 for ours, row 0 for theirs; `TARGET`, `:101`).

RAW block, columns 0-68:

| Col | Feature | Definition / source | Info-set flag |
|---|---|---|---|
| 0-17 | type one-hot | `form(key).vec[types]`; types in poke-env `PokemonType` order BUG,DARK,DRAGON,ELECTRIC,FAIRY,FIGHTING,FIRE,FLYING,GHOST,GRASS,GROUND,ICE,NORMAL,POISON,PSYCHIC,ROCK,STEEL,WATER (THREE_QUESTION_MARKS and STELLAR excluded, `:49`). If `mon._temporary_types` is set (or the form is unknown) it overrides with those types (`:441-451`). | form of theirs = last form in protocol (Mega only after `-mega`) |
| 18-23 | base stats | `min(1, base/255)` for hp,atk,def,spa,spd,spe of the CURRENT form (`tracker.form`, else `mon.species`) from `game.species_entry` (Champions species overlay on poke-env gen9 dex) | |
| 24 | max HP | ours only: `min(2, mon.max_hp/400)` | hidden for theirs (0) |
| 25-29 | atk,def,spa,spd,spe | ours only: `min(2, mon.stats[s]/400)` (exact stats from request) | hidden for theirs (0) |
| 30 | HP fraction | `HP_COL`. ours: `current_hp_fraction`; theirs: fraction if shown (revealed in battle) else 1.0 | theirs = percent from protocol `x/100` (integer), engine knows exact HP |
| 31-36 | status | one-hot BRN,FRZ,PAR,PSN,SLP,TOX | visible |
| 37 | status counter | `min(1, mon.status_counter/5)`, only when a status is set; poke-env counter (sleep: +1 per move/cant-move attempt; toxic: +1 per end of turn, `pokemon.py:195,401,469`) | derived from the log |
| 38-44 | boosts | only if any stage nonzero: `log2(stage multiplier)/2` for atk,def,spa,spd,spe,accuracy,evasion, stage clipped -6..6; stage mult = (2+s)/2 or 2/(2-s) (`BOOST_SCALAR` `:60`) | visible |
| 45 | present | 1.0 for any filled row | |
| 46 | ours | 1.0 / 0.0 | |
| 47 | active | `ACTIVE_COL`; not fainted and (ours: row 0; theirs: shown and is `opponent_active_pokemon`) | |
| 48 | fainted | `Status.FNT` | |
| 49 | shown | ours always; theirs 1 if sent out in battle, 0 if preview-only | |
| 50 | not brought | not shown and >= 3 of theirs revealed | derived (3-of-6) |
| 51-58 | volatiles | 51 ENCORE, 52 SUBSTITUTE, 53 TAUNT, 54 YAWN, 55 LEECH_SEED, 56 CONFUSION, 57 TRAPPED, 58 any partial-trap effect (BIND/WRAP/FIRE_SPIN/WHIRLPOOL/SAND_TOMB/INFESTATION/MAGMA_STORM/CLAMP/SNAP_TRAP/THUNDER_CAGE/PARTIALLY_TRAPPED) (`:56-59,468-473`) | poke-env effect tracking from log (`Effect.TRAPPED`, `YAWN` set by protocol start lines) |
| 59 | must recharge | `mon.must_recharge` | |
| 60 | preparing | `mon.preparing` (two-turn move charging) | |
| 61 | first turn | `active and mon.first_turn` (`_active_turns == 1`) | counted from log |
| 62 | protect counter | `min(1, protect_counter/3)` | counted from log |
| 63 | is Mega | current form `Mega` in forme | |
| 64 | could Mega | `COULD_MEGA_COL`; not fainted, not Mega, side has not used Mega (`used_mega_evolve` ours / `opponent_used_mega_evolve` theirs), species has a Mega form: if item known, 1 iff `megas()[1][item]` (stone -> form) is a form of this species, else 1.0 (unknown item may be a stone) (`:481-482`) | theirs: depends on item reveal |
| 65 | item known | ours always; theirs after reveal (item set, or known gone) | HIDDEN item |
| 66 | item known gone | `item_known and not item` | |
| 67 | ability known | `bool(mon.ability or form.only_ability)`; `only_ability` = the single distinct ability of the form's entry (so Mega forms and single-ability species count as known) | HIDDEN ability; poke-env may also auto-fill a single possible ability |
| 68 | move count | `known moves / 4` (moves with `move_facts` not None) | HIDDEN moves |

MOVES block, columns 69-216 (4 slots x 37 = 148; slot j starts at `69 + 37*j`): per slot `move_facts(id).vec` (34) then 3 state floats.

| Within slot | Feature | Definition |
|---|---|---|
| 0-17 | move type one-hot | type from `base_data` (Champions table else poke-env), blank if none |
| 18-20 | category one-hot | physical, special, status |
| 21 | power | `min(2, power/150)`, 0 for status |
| 22 | accuracy | 1.0 if True/101+, else `min(1, acc/100)` |
| 23 | priority | `clip(priority/5, -1.5, 1.5)` |
| 24 | contact | `"contact" in flags` |
| 25 | sound | flag |
| 26 | pivot | `bool(move.self_switch)` |
| 27 | recovery | `heal > 0 or drain > 0` |
| 28 | setup | `_is_setup`: self-targeting positive boosts, or status move with positive `self_boost` (`:163-168`) |
| 29 | inflicts status | `move.status` or any secondary with status |
| 30 | multi-hit | `n_hit != (1,1)` |
| 31 | self-drop | any negative `self_boost` |
| 32 | first turn only | id in {fakeout, firstimpression, matblock} |
| 33 | protect | `move.is_protect_move` |
| 34 | present | 1.0 if the slot holds a move |
| 35 | PP fraction | ours-active: request `pp/maxpp`; else poke-env `current_pp/max_pp` (`max_pp = entry.pp*8//5`, `move.py:461`) |
| 36 | off | ours-active: request `disabled`, or move not in the legal-order set; else 0 |

Slot order: our active = move j is the move whose legal order has slot j (`f1._slot_moves`, `RL\rl\features.py:75-86`), i.e. the index in the request/dict move order (struggle/recharge are dropped by `NOT_MOVES`, leaving the slot empty); our bench = `list(mon.moves.values())[:4]`; theirs = revealed moves in reveal order, compacted, max 4 (`:485-487`). `ids[i, 3+j]` = move vocab id.

HIST block, columns 217-220 (group "hist"; switchable but ON in every config, `PURE = {"hist"}`, `:37`; zero for preview-only rows): `entries/4` (switch-in count of that member via `|switch|/|drag|/|replace|`), `first/5` (times it moved before the opposing active in a turn where both moved), `second/5`, `turns on field since entered/10` (active only). All `min(1,...)`. Source: `Tracker` (`:224-285`), fed once per protocol line.

LOOKUP block, columns 221-251 (group "lookup", 31):
- 221 faster, 222 slower, 223 tie: speed-interval comparison with `TARGET`; defined only if both rows alive (`:555-563`). Speed interval (`_speed` `:328-349`): ours = exact `stats.spe`; theirs = `[lo, hi]` of the form's level-50 stat with zero investment/hindering nature (`_stats(base,0,90)`) to 32 points/helpful nature (`_stats(base,32,110)`); Choice Scarf x1.5 if item is Scarf (both bounds), else if item unknown the upper bound x1.5; weather-speed ability (swiftswim/chlorophyll/sandrush/slushrush) x2 when ability known and matches, or upper bound only when ability unknown but the form can have such an ability; stage multiplier then `int(...)`, paralysis x0.5, Tailwind x2. `faster = a.lo > b.hi`, `slower = a.hi < b.lo`, reversed under Trick Room, else tie.
- 224-251: 4 moves x 7. Per move: [STAB flag, 6 effectiveness codes vs the 6 rows of the OPPOSING side (ours: their rows 6-11; theirs: our rows 0-5)]. STAB = move type equals either type of the user, damaging only; Libero/Protean give STAB to all. Effectiveness code = `(clip(log2(eff), -2, 2) + 3)/5` for eff > 0 (0.2/0.4/0.6/0.8/1.0), -1.0 if eff == 0, 0 for status moves or an absent target row. `eff` = chart[move type, type1] x chart[move type, type2] (Gen 9 chart `GenData.from_gen(9).type_chart`, index 18 = none = neutral) x the target's ability/item immunity multiplier (known ability/item only; `ABILITY_TYPE`, `ITEM_TYPE` tables `:68-75`), Freeze-Dry x4 on a Water-type target (`:542-543`). Move type = ate-ability retype (pixilate, aerilate, refrigerate, galvanize, liquidvoice) when the user's ability is known (`:509-510`).

DMG block, columns 252-267 (group "dmg", 4 moves x 4): [min fraction, max fraction of the target's CURRENT HP (clipped 2.0), KO certain (min >= 1), KO possible (max >= 1)]. Closed form, level 50 (`_damage` `:569-668`):
`low = floor((floor(22*P*A_lo/D_hi)//50 + 2) * mod * 0.85) * hits_min`, `high = floor((floor(22*P*A_hi/D_lo)//50 + 2) * mod * upper) * hits_max`, fractions divide by the target's current HP bound (`hp_hi` for low, `hp_lo` for high). Ours attack with exact stats; theirs with the form's zero/max-investment bounds; both defend with the opposite bound. `mod` = type effectiveness x STAB (1.5, Adaptability 2.0) x weather (sun/rain on fire/water) x ability tag/retype power multipliers (sharpness, ironfist, strongjaw, megalauncher, reckless, toughclaws, punkrock, ate-abilities, technician <= 60, hugepower/purepower) x item (choiceband/specs 1.5 on the matching category, lifeorb 1.3) x burn 0.5 on physical unless guts x screens 0.5 (reflect/aurora veil/light screen on the defender side). `upper` = 1.5 if the attacker's item is unrevealed. Weight moves (grassknot/lowkick) use form weight table, knockoff x1.5 if the target holds or may hold an item, seismictoss/nightshade fixed 50, bodypress uses Defense, psyshock-family hit Defense. Not modelled: crits, sheerforce, many conditional-power moves, defender abilities other than the immunity table. Output is float64 math cast to float32.

### 2.3 Field vector (55), `_field` `:359-397`

0 turn/30 (clip 1) | 1 force_switch | 2 trapped | 3 maybe_trapped | 4 can_mega_evolve | 5 used_mega_evolve (ours) | 6 opponent_used_mega_evolve | 7-10 weather one-hot sun, rain, sand, snow (Desolate Land=sun, Primordial Sea=rain, Hail=snow; Delta Stream not covered) | 11 weather age `(turn-start)/8` | 12-15 terrain electric, grassy, misty, psychic | 16 terrain age /8 | 17 Trick Room | 18 TR age /5 | 19 Gravity | 20-34 our side, 35-49 their side: [+0..+9] REFLECT, LIGHT_SCREEN, AURORA_VEIL, TAILWIND, SAFEGUARD, MIST, STEALTH_ROCK, SPIKES (layers/3), TOXIC_SPIKES (layers/2), STICKY_WEB (1.0 flags), [+10..+14] elapsed turns /8,/8,/8,/4,/5 for reflect, light screen, aurora veil, tailwind, safeguard | 50 our alive trio members/3 | 51 their fainted/3 | 52 their revealed/3 (clip 1) | 53 legal non-Mega move actions/4 | 54 legal switches/2. The "age/elapsed" values need the turn each condition started (poke-env stores it; stealth rock stores the start turn but is encoded as flag 1.0).

### 2.4 Last-turn token (20 floats + 4 ids), shared with base v2

`fx.last_turn` (`RL\rl2\features.py:225-252`), records from `rl2.turns._Reader` (`RL\rl2\turns.py`): record for turn `battle.turn - 1` (segment after `|turn|t-1` up to `|turn|t`; empty if absent). Per side (ours cols 0-8, theirs 9-17): action==move, action==switch, no action, Mega this turn, moved first, `min(2, damage_taken)` (HP fraction lost, direct and indirect, from `|-damage|/|-heal|/|-sethp|` against a running per-member HP map), `min(1, fainted/3)`, item revealed, ability revealed. 18 = `min(1, turn/30)`, 19 = turn <= 1. ids: move id (first non-`[from]` move line), switch-in species id (a switch chosen at turn start, not after a faint). `revealed_item` fires on `-item`/`-enditem` or any `[from] item:` annotation; `revealed_ability` on `-ability` or `[from] ability:`.

### 2.5 State the Rust engine/wrapper must track or compute

Per Pokemon (both sides): species form (incl. Mega, Mimikyu busted, persistent forms), types incl. temporary types, HP/max HP, status + counter (sleep attempts, toxic turns), boosts incl. accuracy/evasion, volatiles listed above, must_recharge, preparing, `_active_turns` (first turn), protect counter, item (+ known/gone), ability (+ known), moves in reveal order with PP/max PP (and `disabled` for the active), Mega-used flags, entry counts, who-moved-first counts, entry turn, last-seen move.
Side: Showdown's request order of our trio (changes on each switch), opposing members in order of first appearance, preview order of their six, side conditions with start turn / layers, weather + start turn, terrain + start turn, Trick Room/Gravity + start turn, trapped / maybe_trapped, canMegaEvo, forceSwitch, turn number.
Tables: gen9 type chart; Champions species table (base stats, types, abilities, weight, forme, requiredItem/megas); move table (type, category, power, accuracy, priority, flags, pivot, heal/drain, boosts/self_boost/target, status/secondary, n_hit, recoil, protect, PP); level-50 stat formula with IV 31, nature 90/110, points 0..32 (`PCSA\src\pcsa\battle\stats.py:13-26`); stone -> Mega form map; STAB, ability, item tables listed above; weather/terrain speed tables; vocab ids.
Protocol-derived history: whole-battle ordered event list (or incremental equivalents): turn records, move-order counts, forms, last-turn token.

### 2.6 Features that depend on information-set bookkeeping (engine knows the full state and must hide it identically)

- Opposing HP: only the integer percent that `x/100` shows (col 30). Ours exact.
- Opposing max HP, atk/def/spa/spd/spe (cols 24-29): zero for theirs always.
- Opposing item: id 0 until revealed (`-item`, `-enditem`, `[from] item:`, Mega stone via `-mega`); "known gone" after consumption/knock off; also feeds could-Mega (64), speed upper bound x1.5, Knock Off power, `upper` in the damage estimate.
- Opposing ability: id 0 unless revealed or the form has exactly one distinct ability (poke-env sets a single possible ability automatically; Mega forms with one ability count as known only once the form is Mega).
- Opposing moves: only moves used (or revealed by other protocol lines poke-env tracks); compacted in reveal order, so row slot j means "j-th revealed".
- Opposing form/Mega: form changes only from `switch/drag/replace/detailschange/-formechange/-mega`; unrevealed Mega is invisible (64 stays 1 while the item is unknown).
- Opposing bench: preview-only rows (active=0, shown=0, HP 1.0, no moves/item/ability, types and base stats of the PREVIEW species); not-brought flag (50) after 3 are revealed.
- Opposing PP and status counters: poke-env's local tracking from visible uses.
- Speed intervals (221-223) and all opposing-side damage bounds: from public base stats only.
- Which of their members are "brought": hidden until sent out; the engine must not leak the real trio.
- Last-turn token: both sides' actions come from the public protocol (a hidden action such as an unrevealed switch target is still public once it happens).
- History columns 217-220 and field 51-52: public counts.
- Our own side is fully visible to us (exact stats, moves, PP, item, ability).

### 2.7 Features that still need a damage calculation, the belief model or the battle log

- Damage calculation: the `dmg` group (252-267) only, and it is the closed-form estimate above (a numpy sketch of the formula, no catalog, no engine call). If the group is left off, the observation has no damage computation at all.
- Belief model: none. No catalog, no guessed builds, no belief (the module docstring `:1-5` and the code agree). Speed/damage bounds come from public base stats.
- Battle log: yes, incrementally. `Tracker` feeds each protocol line once (O(1) per line) for entries, move order, forms, entry turns; the last-turn token reads the same lines; poke-env's own counters (status_counter, first_turn, protect_counter, effects, boosts) are log-derived. This is the one place the "raw reads + constant-time lookups only" rule is bent: the Rust engine should maintain these counters natively instead of re-reading a log, but must produce the same values (the Python reference includes poke-env quirks, e.g. status_counter semantics).
- The old v2 belief columns and catalog-based damage (base and belief modes) are gone from ll.

---

## 3. Vocabulary and id tables

- Source: `RL\rl\features_vocab.py:17-47,60-65`; table file `RL\rl\data\vocab.json` (1-based ids in file order, 0 = unknown/padding), built from `PCSA\assets\game\species.json` keys (incl. Mega forms) in file order, the `tables.item/ability/move` lists in `PCSA\assets\game\vocab.json`, then extras from `assets/sets/top_builds.json`. Keys are `to_id_str` (lowercase alphanumerics). `index(kind, name)` returns 0 for empty/unknown.
- Sizes (embedding rows; `RL\rl\spec.py:10`): species 1519, item 238, ability 318, move 955. File check: vocab.json holds 1,518 species, 237 items, 317 abilities, 954 moves (line ranges 3-1520, 1523-1759, 1762-2078, 2081-3034).
- ll ids: `ids[row] = [species(form), item, ability, move1..4]`. Species id = the CURRENT form (e.g. `garchompmega`/`garchompmegaz` after Mega, `mimikyubusted`); for unknown forms falls back to `mon.species`. Item/ability ids only when known.
- `last_ids`: move id of the side's first own move, species id of the switch-in (`rl2.turns._species` = id of the first comma field of the details string).
- Rust must load the same `vocab.json` (freeze a copy; the Python file is rebuilt only by `python -m rl.features_vocab`). The pinned data come from the Showdown commit in `tools.lock.json`; `game.dex()` overlays the Champions species table on poke-env's gen9 dex (`PCSA\src\pcsa\battle\game.py:45-52`), so base stats/abilities/formes of the Champions overlay are authoritative.
- Move constants come from `base_data` (`PCSA\src\pcsa\battle\moves.py:67-85`): Champions table first (type, category, power, accuracy, pp), else poke-env gen9 with Champions fallback PP; the flag/boost/secondary fields come from poke-env's `Move`.
- Learnset table (opponent head allowed moves, `rl2/model.py:63-76,181-183`): `game.learnsets()` of the base species; not an observation but part of the network's input handling.

---

## 4. Action space

- Size 11 (`RL\rl\spec.py:7`). Index layout (`RL\rl\features.py:24,210-224`): 0-3 move slot j (slot = index of the move in `battle.active_pokemon.moves` capped at 3, `mirror.py:58-61`); 4-7 same slot with Mega Evolution (`MEGA_AT = 4`); 8-10 switch to bench member k = `SWITCH_AT + min(2, k)` where k is the position among `_request_members(battle)[1:]` (request side.pokemon order minus the first). With 3 brought members only 8-9 are ever legal; 10 exists in the net but never unmasked. After a switch Showdown reorders `side.pokemon` (active first), so the bench slot meaning changes every switch (inferred from Showdown behaviour; the code takes the order straight from `battle.last_request`).
- Legal mask: `mask[action_index(a, battle)] = True` for every legal order (`features_ll.py:679-680`, same in `RL\rl2\features.py:264-266`). Legal orders = poke-env `Battle.valid_orders` (`poke_env\battle\battle.py:270-298`, wrapped by `mirror.legal_actions` `mirror.py:54-64`): switches = request side members not active and not fainted (omitted when `trapped` from the request); moves = `available_moves_from_request` (non-disabled moves, Struggle/recharge as the request dictates); if `canMegaEvo` every available move is duplicated with `mega=True`; forced switch (`forceSwitch`) returns switches only; `wait` returns a default order. Z/dynamax/tera are never active in this format.
- `forced` (`:681-682`): `request.forceSwitch` or no move actions legal; no opponent-action label is made for forced decisions (`RL\rl2\labels.py:18-19`).
- Team preview: not part of the 11. It is a 60-way choice over `all_picks(6, 3)` = every (lead, second, third) with second/third ascending, ordered by `combinations(range(6),3)` x lead (`RL\rl\preview.py:11-13`); sent as `/team <1-based indices in pick order>` (`rl2/lanes.py:136`). Training samples it as: preview value from `preview_observations` (one turn-0 observation per pick, our chosen trio with the lead as row 0 active, their six preview rows, `mask` all False, `forced` True, turn 0; ll `:686-696`), then softmax(value/0.1) with a top-p 0.9 nucleus (`preview.py:16-33`); with `preview_random` the pick is uniform (`RL\rl2\player.py:178-188`). In league-1116 both seats pick uniformly (`preview_random: 1.0`, `opponent_preview_random: true`, `LL\configs\v2-league-1116.yaml:17-18`).
- Mask invariants the wrapper must give: never an all-False mask for a non-forced decision (rule masks are dropped if they would remove everything, section 5); the recorded mask is the masked one (`RL\rl2\train_masks.py:1-5`, `rl2\player.py:230-237`).

---

## 5. The nine live rule masks

Config: `PCSA\config\players\neural.yaml:58-59` lists exactly `encore_switch, no_effect, no_full_heal, useless_status, locked_futile, sucker_punch, yawn_switch, mega_weakness, priority_ko` with `rule_mode: mask`, `sucker_punch_passive: 0.5`, `opponent_top: 3` (live only). Training uses `rule_masks: live` (`LL\configs\v2-league-1116.yaml:33`) which reads that file (`RL\rl2\train_masks.py:22,33-35`). `weak4x_switch` exists in code but is not in the list.

### 5.1 How the masks are applied (matters for the wrapper design)

`rl2/player.py:_finish` (`RL\rl2\player.py:227-238`): the network forward runs first (`out`), then `Masker.mask` (`train_masks.py:110-170`) builds a `Spot` from the Battle, ALL legal actions, the ranked list (legal actions by network logit), the opponent head's kept actions `their_actions(out)` (`kept_idx`, `kept_p`), the catalog and an opponent `Tracker`. `neural_rules.masks` (`PCSA\src\pcsa\players\neural_rules.py:284-301`) tries every legal action as the "chosen" one against the rules in list order; the first rule that returns a replacement different from the action (and, for priority_ko only, not the same move with/without Mega, `SAME_MOVE_OK` `:281`) masks it. Skipped entirely when `obs.forced`, when there is no tracked active on either side or no legal move (`:287`); if every action would be masked the masks are dropped for that decision (`ignored`); any exception means no masks (`errors`). The rules' replacement is only used to test "does a replacement exist" in mask mode, so network ranking does not change which actions are masked (it only matters when the replacement would equal the action itself, which the rules exclude); exact dependence on the head output is below.

Consequence: 3 of 9 rules read the opponent head's output (kept_p) and 4 of 9 need damage/speed calculations, so a fast "no Python per decision" step cannot return all rule masks as observation-time arrays. Either the wrapper computes head-independent masks in Rust and returns per-rule candidate masks plus the scalars the head-dependent rules need, or masks are applied after the network step.

### 5.2 Rule by rule

| Rule | Precise rule (mask mode: an action is masked when the rule fires with it as the chosen action and a replacement exists) | Inputs | Port to Rust |
|---|---|---|---|
| encore_switch (`neural_rules.py:193-198`, `encore_switch.py`) | Chosen is not a switch, `encore.why(battle)` is not None, and a legal switch exists: mask every legal move/Mega action. `why`: their active used Encore on us during its current stay AND the protocol shows for sure it moves before our active in the same pair/speed state: `ours_first == 0` and (`theirs_first >= 2`, or `>= 1` when a speed tie and hidden Quick Claw-type items are ruled out by `tie_impossible`: plain speed state, their item known and not Quick Claw/Custap/Lagging Tail/Full Incense, our speed outside their species' range with Scarf handling). | Whole-battle protocol walk (`_Walk`, `encore_switch.py:57-233`): per turn first mover for both sides' non-called moves, move priorities (Prankster/Gale Wings/Triage/Stall/Mycelium Might via possible abilities, early-mover items), speed-state signature (stages, status, tailwind, trick room, weather, terrain, boost epochs, form changes, item changes, Scarf reveals), text scan for "Quick Claw/Custap Berry/Quick Draw". No damage. Needs our exact Speed and their base-stat speed range. | Needs the full log history in Rust (or an equivalent incremental state machine); no damage calc, no belief. Medium; fiddly equality (signature tuple semantics). |
| no_effect (`:108-114`, `no_effect.py`) | Chosen is a move for which `no_effect.why(...)` is not None (attacker/defender both present, target in FOE_TARGETS) and a non-useless replacement exists. Reasons: first-turn-only move (Fake Out/First Impression) when `not attacker.first_turn`; priority blocked (positive priority vs grounded target in Psychic Terrain, or target surely has Queenly Majesty/Dazzling/Armor Tail); Ground move into surely airborne target (Flying type, Air Balloon, Magnet Rise, surely Levitate; not under Gravity/Iron Ball); type immunity via `defender.damage_multiplier(move) == 0` (Scrappy/Mind's Eye exempt, status moves only Thunder Wave, variable-type moves skipped); absorb abilities (volt/water/flash/sap/levitate/earth families) and soundproof/bulletproof when the defender surely has them (revealed, or every possible ability is in the set); Good as Gold / Magic Bounce on status moves; status-immunity (types, powder vs Grass/Safety Goggles, abilities, Misty/Electric Terrain on grounded). Mold-Breaker-family attacker abilities (surely) switch ability checks off. | Our/their current types (incl. temporary), `Pokemon.possible_abilities` (species ability list), revealed ability/item, fields, `is_grounded`, move category/flags/priority/target, type chart. | Trivial-to-medium: constant tables + information-set reads. No damage, no history, no belief. Needs poke-env's `damage_multiplier`/`is_grounded` semantics. |
| no_full_heal (`:117-122`) | Chosen move id in HEALS = {recover, roost, slackoff, softboiled, milkdrink, shoreup, synthesis, moonlight, morningsun, rest} (wish excluded, rest included) and our active HP fraction >= 0.99, and a replacement that is a switch or a non-heal move exists. | our HP fraction, move id. | Trivial. |
| useless_status (`:125-130`, `useless` `:334-348`) | Chosen move is `useless`: `no_effect.why` not None, or hazard at maximum (Stealth Rock/Spikes 3/Toxic Spikes 2 on their side), or one of yawn/thunderwave/willowisp/toxic/poisonpowder/sleeppowder/spore/hypnosis/glare/stunspore into an already-statused foe (Yawn also into already-drowsy). Needs a replacement that is a switch or non-useless move. | their status/effects, opponent side conditions, no_effect. | Trivial/medium (same inputs as no_effect). |
| locked_futile (`:133-145`) | Chosen is a move, there is at least one legal move, and (we are Encored or all legal moves share one id), and the best legal move's `hit_fraction` < 0.05 of their max HP, and no legal status move is non-useless: mask the move actions if a legal switch exists. | `threat.hit_fraction` = expected damage (midpoint of `damage()` x accuracy x expected hits) for our exact attacker vs the catalog-ASSUMED opposing defender (top catalog build under revealed item/ability/moves, corrected by `Tracker.fast/strong` for Scarf and power items). Also status-move checks. | NEEDS the damage engine and the assumed opposing build (catalog/tracker): conflicts with the no-damage-engine rule. Only fires in locked/Encore situations, so it could be gated, but the gate is damage-dependent. |
| sucker_punch (`:148-166`, `sucker_punch.py:91-99`) | Chosen move is Sucker Punch or Thunderclap, not (one Pokemon left each: `|teamsize|` minus fainted), and the share of the opponent head's kept-column probability that is a switch or a status move (switches to a species that cannot be on their team, i.e. team closed and species never seen, are dropped and the rest renormalised) exceeds 0.5: masked if a replacement (non-sucker, non-useless preferred) exists. The config comment also says "or when they already failed against that foe this battle"; the code has NO such history check (`why` uses only `last_one_each` and `passive_share`). | Opponent head output (`kept_idx`, `kept_p`), move categories, `_team_size` from `|teamsize|`, seen species/bases, fainted counts. | Needs the OPPONENT HEAD (network output) -> cannot be an observation-time array; can be returned as a per-action candidate mask + the head-derived scalar threshold test. Light logic otherwise. |
| yawn_switch (`:236-246`) | Chosen is not a switch, our active has the Yawn effect, can fall asleep (no status, ability not insomnia/vitalspirit/sweetveil/comatose, not on grounded Electric/Misty Terrain), it is not our active's first turn, and no legal move has `hit_fraction >= their current HP fraction` (no KO this turn): mask move actions if a switch exists. | Our effects/ability/status/terrain, `s.hit()` per legal move = damage engine + assumed opposing build + accuracy + hits. | NEEDS the damage engine and assumed build (conflicts with the rule); `hit` here is an expected-value KO test (midpoint x accuracy x hits >= HP fraction). |
| mega_weakness (`:201-210`, `mega_weakness.py:73-83`) | Chosen action is a Mega action, our active has a Mega form for its held stone (`mega_form_of`), and some attack of theirs (opponent-head kept moves with probability >= 0.15 and damaging, plus every damaging move they have revealed) hits the Mega form 4x but not the base form, and our chosen move does not KO first (`hit >= their HP` and (higher priority than that attack, or equal priority and `outspeeds`)): mask the Mega action (the plain version of the same move, or the best non-Mega action, is the replacement). | Opponent head output, type chart, Mega form types, `threat.hit_fraction`, `threat.outspeeds` (speed with assumed set, stages, Scarf/ability/weather/tailwind/trick room). | NEEDS the opponent head AND damage AND speed with assumed opposing build. Hardest. |
| priority_ko (`:174-190`) | If a switch is in the head's top-3 kept actions (by probability): no mask. Else `threat.priority_ko`: our strongest positive-priority move among known moves that KOs (KO check: midpoint damage x expected hits >= their current HP fraction, `_ko_share`); skip if it is Sucker Punch and the foe's last action was not an attack (`foe_attacked`, protocol walk of this and the previous turn), or the sucker_punch rule would block it, or it is `useless` or not legal. Then every legal action other than that move (with or without Mega) is masked, unless the chosen move itself KOs first (`hit >= HP` and (priority > 0, or priority 0 and `outspeeds`)). | Opponent head (switch in top 3), damage engine for all our moves and the KO test vs the assumed set, speed, protocol (foe_attacked, `last_one_each`). | NEEDS the opponent head, damage engine, speed, a little log. Hard. Note the neural.yaml comment "priority_ko stands aside when a switch is in their top 3". |

Opponent-head-dependent: sucker_punch, mega_weakness, priority_ko. Damage-engine (assumed-build) dependent: locked_futile, yawn_switch, mega_weakness, priority_ko. Whole-log dependent: encore_switch (full walk), priority_ko (foe_attacked), sucker_punch (`_team_size`). Pure state/table: no_effect, no_full_heal, useless_status (and the legality half of the others).

All four damage-dependent rules use the pcsa engine `pcsa.battle.damage.damage` (`PCSA\src\pcsa\battle\damage.py:162-195`, Showdown formula at level 50 with integer floors: `base = ((22*power*atk//def)//50)+2`, roll 85-100 then x modifiers (type incl. ability immunity, STAB 1.5/2.0, weather, terrain 1.3, misty dragon 0.5, burn, screens 0.5, Life Orb 1.3, Expert Belt 1.2, type-boost items 1.2, Filter-family 0.75, Ice Scales/Fluffy/Punk Rock 0.5, power modifiers of ~20 abilities, Technician, Sand Force, Choice Band/Specs 1.5 on the stat, Hugepower, Guts/Hustle/Solar Power), variable power from `opponent.move_power`/`tactics.effective_power` (grass knot/low kick weight, heavy slam/heat crash weight ratio, facade, electroball, gyroball, knock off x1.5, triple axel mean power), accuracy including No Guard, Thunder/Hurricane/Blizzard weather rules, expected hits) and the opposing build from `opponent.assumed` (catalog + seen-builds + `Tracker`, `opponent.py:96-160,277-394`) in a form that depends on data files (`assets/sets/top_builds.json`, seen_builds.json) and on which builds were seen on the ladder. This is more than the ll `dmg` group and is not closed-form: it is the "damage engine" the low-latency design excludes.

---

## 6. The max-damage opponent

Training opponent kind `max_damage~` (`LL\configs\v2-league-1116.yaml:21-22`; `Stochastic(max_damage)` built in `RL\rl2\player.py:448-452`); class "scripted" has a 25% share (`opponent_shares`).

### 6.1 Base: `MaxDamagePlayer` (`PCSA\src\pcsa\players\max_damage.py`)

- Decision: among legal orders that are moves (Mega variants included), pick `max(attack_score(...))` (`:54-62`); score for a Mega variant uses the Mega form's stats/types/ability (`mega_form_of`). If no moves legal (forced switch): bench member whose best non-status attack score vs the opposing active is highest (`:63-66`). Never a voluntary switch, never a status/setup move. Python `max` picks the FIRST legal action on ties (iteration order = `valid_orders`, moves in request order, Mega variants after plain). If the opponent active is unknown, falls back to the first legal action.
- Team preview (not used in league-1116, where the scripted seat picks uniformly at random, `rl2/lanes.py:139-158`): the three team members with the highest sum over their opposing six of best attack score, in that descending order (lead = best); `battle.teampreview_opponent_team` else seen opponents.
- `attack_score` (`PCSA\src\pcsa\players\tactics.py:94-125`): `power x STAB(1.5 if move type in attacker types, else 1) x ratio x accuracy x expected_hits x multiplier`, with `ratio = attack_stat x stage_mult(attack) / max(1, defense_stat x stage_mult(defense))`; stats: `current_stats(mon, exact)` (our mon exact via request stats; the opposing unknown mon = midpoint of `stat_ranges` of its level-50 low/high bounds); Foul Play uses the target's Attack and stages; Body Press uses Defense; `multiplier = defender.damage_multiplier(move)` (poke-env type chart on current types), set to 0 if the defender's KNOWN ability (`known_ability`: revealed in log or Mega ability) is an immunity (Levitate/Volt Absorb/Lightning Rod/Motor Drive/Water Absorb/Storm Drain/Dry Skin) or Air Balloon vs Ground; `effective_power`: Grass Knot/Low Kick by weight table, Heavy Slam/Heat Crash by weight ratio, Facade x2 when statused, Electro Ball/Gyro Ball by speed ratio, else `move.base_power`; accuracy: 1.0 under No Guard, Thunder/Hurricane 1.0 in rain, Blizzard 1.0 in snow, 0.5 for Thunder/Hurricane in sun, `move.accuracy x stage multiplier (3+s)/3 or 3/(3-s)` with stage = clamp(acc boost - evasion boost, -6..6), capped 1.0; `move.expected_hits` from poke-env.
- Damage formula needed: none beyond this heuristic (no damage roll, no HP). Needs the species midpoint-stat table, our exact stats, type chart, move table, weight table, hit-count and accuracy rules.

### 6.2 `Stochastic` wrapper (`RL\rl2\stochastic.py`)

Per battle (at `choose_team`) a level cold/warm/hot drawn uniformly (KEEP 0.85/0.65/0.45). Per decision: compute the inner choice; compute the KO move `_ko` = `MaxDamagePlayer` pick if `threat.hit_fraction(...) >= opponent HP fraction` (uses the full damage engine and catalog); with probability `KO_FLOOR` 0.8 take it; else with probability KEEP[level] keep the inner choice if `reasonable` (no heal at >= 0.99 HP, Fake Out/First Impression/Mat Block only on first turn, hazard not already maxed incl. Sticky Web, status moves not into an already-statused foe/yawned foe, setup move only if some raised stat < +6); else draw uniformly among other reasonable moves; with probability 0.05 (or when no other moves) a uniform switch. So the wrapper needs the `threat.hit_fraction` damage engine for the KO check (the same engine as the masks). A Rust reimplementation must either port that engine or define a new documented approximation (distribution drift from training, fully inferred consequence).

### 6.3 `best_matchup_max_damage` (not in league-1116)

`PCSA\src\pcsa\players\best_matchup.py`: max_damage plus a voluntary switch when `HeuristicPlayer.matchup(active, opp) < -2.0` and the best bench member's matchup > 1.0, not back-to-back switching more than twice or after a non-reciprocated switch (`MAX_STREAK = 2`, `opponent_switched` protocol check). Needs `heuristic.py` weights (config/players/heuristic.yaml), not read in this investigation.

---

## 7. Bookkeeping spec (training)

### 7.1 Reward, done, truncation

- Per decision (`RL\rl\player.py:244-250`): `reward[t] = 0.1 x (opposing faints between decision t and t+1 - own faints between them)`; faints counted from `|faint|` lines in the seat's protocol relative to `player_role` (`faint_counts` `:70-80`), recorded at each decision; the final element gets the terminal `+1 / -1` (0 if the result is unknown/tie). `_settle` adds the terminal reward if the player did not see the result (`RL\rl\rollout.py:108-124`).
- Return/advantage: `gae(reward, value, gamma=0.999, lam=0.9)` per episode with value 0 after the last step (`RL\rl\ppo.py:65-75`, `rl2\ppo.py:36-37`); advantages normalised over the update and clipped +-10 (`rl\ppo.py:98-104`, `rl2\ppo.py:40`). Value head: 51 bins over [-1.6, 1.6] two-hot (`rl2/model.py:26`).
- Done = battle finished (win/lose/tie). There is no truncation or bootstrap path in the rl code. Showdown ties the battle when `this.turn > 1000` (`PCSA\.tools\vendor\pokemon-showdown\sim\battle.ts:1855-1860`; verified in the vendored source); the Endless Battle Clause is a separate rule (not inspected). The only other limit is `BATTLE_TIMEOUT_S = 120 s` wall clock (`rl\rollout.py:31`, `drive` `rl\rollout_battle.py:50-73`) which raises BattleFailed: the battle is discarded and counted as `failed` (no partial episode kept).
- Reward is per-seat from that seat's protocol; the learner never sees the opponent's reward.

### 7.2 Per-decision records the trainer consumes (`RL\rl2\player.py:366-394`, `rl2\archive.py:19-23`)

rows, ids, field, last, last_ids, mask (the MASKED one), action (int 0-10), logp (log of the capped, masked probability of the chosen action), value (scalar), reward, probs (the full capped distribution, [T,11]), labels [T,4] int64 = (kind NONE/MOVE/SWITCH = 0/1/2, move id, mega 0/1, bench row 7..11 index or -1) built from the next turn record of the opposing side (`RL\rl2\labels.py:16-41`: label of the decision made while `battle.turn == t` is the opponent's action in segment t; 0 for forced decisions), turn [T], forced [T], plus optional teacher_probs/teacher_mask. Per episode: won (True/False/None), turns (`int(live.turn)`), seat, opponent kind/snapshot, team and opp_team exports, the seat's full protocol text, policy_update, preview_entropy, preview_value, `opp_preview_random`, `rule_masks` counts. Archive stores floats as f16 and ids as i32 (`archive.py:19-23`) - the saved arrays are lower precision than the live ones.
- `sample`: capped probabilities at temperature T (`rl.entropy.capped_probs`, cap 0.99 per the config comment); temperature 0 = argmax. Training `temperature` comes from the run config. Fallback on any exception = first legal action (`_fallback`).

### 7.3 Team sampling and seats

- Format `gen9championsbssregmc` (`PCSA\config\agent.yaml:110`), level 50, bring 3 of 6, `PICKED = 3` (`pcsa\battle\state.py:19`).
- Job: `Job(team_a, team_b, opponent, seed, index, preview_random, ...)` (`RL\rl\rollout.py:72-82`). `index` even: A challenges and sits p1; odd: B challenges, A sits p2 (`rollout.py:217`, `lanes.py:267`). Seeds: `run.seed x 1_000_000_007 + jobs_made` (`rl\train.py:199`); arenas reseeded with `job.seed*2 + salt`, numpy generators with `[job.seed, salt]`.
- Teams: `team_a` = fixed `our_team` (the gold team export), `team_b` drawn from the ladder pool (`team_shares [0,0,0,0,0,1.0]`, `ladder: rl2/data/ladder-20261116.json`, 488 live rosters, `ladder_rho 0.0` = no perturbation; `LL\configs\v2-league-1116.yaml:51-54`) with PFSP by loss rate per roster (`pfsp` block, `rl2.pfsp`) and class PFSP over {self, snapshot, exploiter, scripted} (`rl2.class_pfsp`). `make_jobs` (`rl\train.py:182-204`) keeps adding jobs until expected decisions reach `steps_per_update` (16,384; self-play counts both seats).
- Preview/lead: both seats' trio and lead are drawn uniformly (`preview_random 1.0`, `opponent_preview_random true`); scripted seats use `random_team(rng, 6, 3)` through `_uniform_preview`.
- Both-seat self-play: `kind == "self"` yields two episodes per battle, one per seat (`lanes.py:272-281`, `rollout.py:221-223`); snapshots and scripted opponents yield one (A's). Each seat gets its own policy object sharing the network (batched in `Batcher`, `MAX_BATCH 256`, `YIELDS 24`). The p2 seat sees the mirrored p1 view: `mirror.as_p1` swaps `p1`<->`p2` in protocol and request and a fresh Battle is rebuilt per decision with `replay(...)` (`lanes.py:111-118`), so the observation never contains seat-dependent values. Win/lose is read from the seat's Battle.
- Network memory: the model attends to the field tokens of up to 64 earlier decisions of the game (`rl2/model.py:41,152`); the wrapper must keep per-game history of the observations (not part of one observation).
- New battle: fresh `_Game` per battle tag (also reset when `turn` decreases, `rl2/player.py:222-224`); preview observations are 1-decision games for the critic.

---

## 8. Live-player constraint: phone-side vs engine-side differences

The live player builds a `Battle` per decision from the phone: protocol-like lines produced from the battle log by `Translator` (`PCSA\src\pcsa\battle\state.py:230-273`) plus a request constructed by `battle_request` (`:91-136`). Same observation code, so any column below can differ even for identical hidden state.

Verified in the code:
- Crits: the phone log does not print critical hits (`PCSA\src\pcsa\belief\README.md:37-39`). ll does not read crit lines; `damage_taken` (last-turn token) includes crit damage identically on both paths, but anything that would use crit info is absent. The closed-form damage has no crit term.
- Our HP: exact from the log when possible (`state.py:103-114`), screen OCR otherwise; forced screens prefer the party screen. Our stats/moves/ability/item come from the user's team sheet (`_entry` `:62-75`), not Showdown's request, so values should be identical to the sim for the known team but depend on the sheet being correct.
- Our moves: names from the sheet by slot; PP from the move screen, else `logged_pp` = Champions max minus counted uses (`:221-227`); `disabled` reconstructed by pcsa rules (move-screen flag, PP 0, Encore lock, Choice lock from our last move line, Taunt/Assault Vest blocking status, first-turn-only moves after acting) (`:179-218`) instead of the simulator's request flags. Column 36 ("off") and legal mask can therefore differ.
- Trapped: `is_trapped(...)` heuristic (`:134-135`) vs the request's `trapped`; `maybe_trapped` never set from the phone (inferred; request builder never sets it).
- Opposing preview: species read from preview sprites and corrected from the log (`misread_slots`, `:41-59`); `teampreview_opponent_team` can be wrong/ordered by sprite slot, which changes preview-only rows, their order and `not brought`.
- Opposing HP: percent from the screen/log (integer), rounding may differ from the simulator's `x/100`.
- Opposing item/ability/moves: only what log lines reveal. Items such as Choice Scarf are never named unless the log prints them (in ll the unknown-item upper bound x1.5 covers Scarf in the speed interval, but "item known" stays 0); phone logs may omit `[from] item:` annotations the simulator protocol has (inferred).
- Move order counts (hist cols, `Tracker.first/second`), `moved_first` and `damage_taken` in the last-turn token depend on every `|move|`/`|-damage|` line being present in the same order; the phone's translated log may miss or reorder lines (inferred).
- Turn start of weather/terrain/side-condition (ages) and `first_turn` come from poke-env's parse of the translated log; mismatches would shift cols 61, 11, 16, 18, 30-34/45-49 of the field.
- Mega: `canMegaEvo` comes from the screen (`mega_available`) on the phone, from the request in the sim.
- Max PP of bench and opposing moves uses poke-env's `pp*8//5` (`move.py:461`) on both paths; the simulator's real Champions PP may differ (`pcsa.battle.moves.max_pp` has the Champions values but ll does not use it for non-active rows) - inferred difference, not measured.
- The live path also forbids `belief_crit_lines: true` and any use of hidden info, so ll's "no belief" design is consistent with it.

---

## Open questions / not verified

1. Nothing was run. All timings, parity and accuracy numbers are quoted from the JSON/log files on disk; I did not re-verify them or check that `features_ll.py` imports cleanly (it imports `rl2.turns`, `rl.features`, `rl2.features` from the LL worktree).
2. Why 20 of 4,926 live states differ between the incremental and cold tracker; whether this is an ordering bug or a replay artefact.
3. The Garchomp-target damage misses (section 1.5): wrong in the estimate, or in the reference engine's assumed set?
4. Whether the ll `dmg` group helps a policy: no model was trained on ll; the value of `hist`, `lookup`, `dmg` groups is untested.
5. `docs/low-latency.md` is missing, so the intended column semantics (e.g. whether the hist group is meant to stay on, `PURE = {"hist"}`) are unknown; ll configs "raw" always include hist.
6. Exact poke-env behaviours the columns rely on (status_counter, first_turn `_active_turns`, `possible_abilities`, auto-filled single abilities, `Effect.TRAPPED`/`YAWN` setting, `damage_multiplier`, `is_grounded`, `available_moves_from_request`, how Showdown reorders `side.pokemon` after a switch) were read in part (pokemon.py/battle.py) but not exhaustively; the Rust side must replicate them bit for bit, so a dedicated poke-env semantics inventory is needed.
7. The rule masks use the catalog `Catalog.load(seen=True)` and `seen_builds.json`; the exact assumed builds depend on data files not read here (`assets/sets/top_builds.json`, seen builds).
8. Heuristic matchup weights for best_matchup (not needed by league-1116) were not read.
9. Whether the training loop applies rule masks before or after the matrix head's softmax matters only for the logits path (the recorded mask is the masked one; the learner recomputes). The post-forward ordering is verified; whether the masked mask is also what the learner feeds to the network's `logits` where() was read only through comments (`train_masks.py:3-5`).
10. Endless Battle Clause and the format's other clauses in `gen9championsbssregmc` were not inspected (formats.ts has no entry under that name in the vendored `config/formats.ts`; the Champions format is defined elsewhere).
11. poke-env here is an installed (possibly patched) copy under `RL\.venv` (`rl2/patches.py` installs fixes in each actor); the exact patches (repairs) that alter parsing were not reviewed.
