# champions-gym: a standalone gym simulator for Pokémon Champions

The single document to read. Last updated 17 Nov 2026. The numbered list of every known difference between
the engine and the real rules is `gaps/INDEX.md`. Everything under `docs/investigation/` is evidence.

## 1. Goal

A standalone gym simulator for the Pokémon Champions game, Ranked Singles (Showdown format
`[Gen 9 Champions] BSS Reg M-C`: bring 3 of 6, level 50, Megas, Item Clause). A Rust engine that plays the
game by the real rules, with a Python gym interface on top. It began from poke-engine's code (commit
`f7b147f`); that code was replaced by a resolver written from Showdown's source and deleted on 18 Nov.

**First milestone:** tower vs max damage playing full games on the engine. A training plan is made after
that, separately.

Why: the bot is stuck near 1700 rating; the target is Master Ball 1 (about 1920). Training on Showdown plus
poke-env runs at about 12 games a second per machine. The strongest known bot in the format trained on about
330 million self-play games on a custom Rust engine.

## 2. What "standalone gym simulator" requires

| Id | Requirement |
|---|---|
| G1 | Self-contained. The repo builds and runs with nothing from `pcsa`, `research`, poke-env or Showdown present. Those are read during development as references only; nothing imports them. |
| G2 | The game's data (species, moves, items, abilities, Megas, type chart, legality) is generated once from Showdown's Champions data and committed in the repo. |
| G3 | Showdown is a development-time test reference only, kept in `tools/`, which the simulator does not depend on, and removable at the end. |
| G4 | A gym interface in Python: `reset`, `step`, observation, legal-action mask, reward, done; many battles per call; both seats, for self-play; and a single-agent form where the other seat is a built-in player. |
| G5 | Built-in players ship in the repo: random, max damage, tower. |
| G6 | Installable as a Python package (Rust core, Python bindings), with a README, its own tests, and the third-party notices that apply (see D17). |
| G7 | Laid out as a simulator: engine, bindings, Python package, tests, tools, docs. Investigation and audit files are development evidence under `docs/`. |

## 3. Decisions (the user's, 16 and 17 Nov)

| Id | Decision |
|---|---|
| D1 | The engine replaces both training and inference for `pcsa`. Showdown is eliminated by the end. |
| D2 | Until then Showdown is the reference: every rule is tested against it. Model evaluation happens on the engine, not on Showdown. |
| D3 | Damage is exact for the roll that is drawn or forced: one calculation per hit, not all 16 rolls. |
| D4 | Only two players are built: tower and max damage. No belief tracker and no guessing of hidden sets of any kind: no build catalogs, no probabilities over sets, no damage ranges. The simplest version. No other scripted players. Extended 19 Nov (user): seven more scripted players ported from pcsa and research on the same terms (a Battle only, CG's fixed rule for unknowns, no catalogs): `best_matchup_max_damage`, `disciplined`, `endgame`, `pilot_encore`, `pilot_weather`, `pilot_hazard`, `pilot_priority`. Round robin and latency: `bench/results/2026-11-19-ports-tables.txt`. Added 19 Nov (the training agent's request): `lookahead` and `lookahead2`, a one- or two-turn search on an engine State built from the player's own Battle by `from_battle`'s fixed rule plus placeholder moves from species data (no belief, no masks, no network), in `env/src/lookahead.rs` because it needs the environment; `env/src/builtin.rs` is now the by-name registry every entry point uses. Results against tower, max damage, disciplined and endgame: `bench/results/2026-11-19-lookahead-results.log` (depth 1 55.5 to 57.7%, depth 2 57.0 to 59.2%, 4,000 games each); speed `2026-11-19-lookahead-speed.txt`. |
| D5 | Inputs are raw reads plus constant-time feature engineering (including damage). |
| D6 | No rule masks in this engine. |
| D7 | The players see their own side fully and only what the opponent has revealed; unknowns are filled by a fixed constant-time rule. |
| D8 | Unused code is deleted from the engine (generations 1 to 8, Terastallization, the tree search). |
| D9 | Compute is allowed on the desktop. The laptop is for the larger jobs (long test runs, benchmarks, anything heavy on cores or memory). |
| D10 | `pcsa` development is finished. No edits there. |
| D11 | Speed: report the measured result; the aim is above 10,000 battles a second (random play, one process; 12,282 was measured on 8 Nov). |
| D12 | First goal: tower vs max damage on the engine. Training is planned after that. |
| D13 | Differences from the real rules are found up front by the read audit, not by test-and-iterate. The audit is checked four ways before anything else moves (step 4). |
| D14 | Scope: every Pokémon legal in the format, in any legal build. |
| D17 | CG is the user's own code, published under the MIT licence (`LICENSE`). The finished repo contains no poke-engine code, so poke-engine's notice is not carried. Done 18 Nov: the old engine, its tests, the rig's old mode and its LICENSE file were deleted together (33 files, 68,576 lines) after a search confirmed nothing in the new code imports, includes or was generated from them. Showdown's MIT notice is a separate matter: CG's data is generated from Showdown, so that notice goes with CG if it is ever given to anyone. |
| D18 | Singles only for now (user, 17 Nov). The State is designed for singles; nothing is added for doubles. |
| D16 | Terminology follows poke-env, and champions-gym is written CG (17 Nov). See "Terms" below. |
| D15 | The repo and package are named `champions-gym` (17 Nov). The folder was renamed from `poke-engine` on 18 Nov, and the engine crate is `cg-engine`. |

Not part of this project (own plans later): training on the simulator, following a battle from the phone,
team-preview playouts, removing Showdown and poke-env from `pcsa` and research.

## 4. Standing rules

- **Agents:** at most 5 at once. Sonnet or Haiku for reading, checking and verifying; Opus for writing code.
  An agent's claim is checked before it is accepted.
- **Findings and questions carry ids** that never change (see `gaps/INDEX.md`).
- **Machines:** small jobs on the desktop (builds, unit tests, data dumps, single-battle checks). Larger
  jobs on the laptop: `ssh -i ~/.ssh/id_ed25519 -o IdentitiesOnly=yes -o BatchMode=yes USER@LAPTOP_HOST`
  (Windows 11, 24 cores, 32 GB, RTX 5070 8 GB; everything under `C:\Users\grich\Code\pcsa`). Over ssh, write
  a script file, copy it and run the file; inline PowerShell breaks on quoting. No reboot or settings change
  on the laptop without asking.
- **Limits:** under 28 GB of memory per machine; benchmarks capped at 10 minutes; any task longer than a few
  seconds runs in its own visible PowerShell window with a log.
- **Processes:** check a process's full command line before stopping it; never stop one we did not start.
- **Keep** every generated result, log and test output. Commit early and often: files have vanished from
  this machine before.
- **No model training** without the user's say.
- **Code:** short comments, no dated history in code. Finished side branches are merged and removed.
- **Our own code (D17):** new CG code is written from Showdown's source and the audit, never copied or
  adapted from poke-engine's code. foul-play is GPL-3.0: never copy from it either.
- **Ask first** before anything hard to undo or outward-facing (pushing, deleting outside this repo). Never
  open `.env` files. No phone or adb. `pcsa` is not edited.

## 5. Where things are

Everything is under `C:\Users\user\Code\pokemon_champions\`.

| Path | What |
|---|---|
| `champions-gym\` | this repo |
| `research\pcsa-rl\poke-engine-eval\` | the 8 Nov evaluation: poke-engine checkout at `f7b147f`, a working Windows Rust toolchain under `_dl`, a pyo3 extension, damage comparison outputs |
| `pcsa\.tools\vendor\pokemon-showdown\` | Showdown at commit `c0845f33d`, mod `champions`: the reference |
| `research\pcsa-rl\rl2\opponents\tower.py` | the tower player's rules today (Python, poke-env) |
| `pcsa\src\pcsa\players\max_damage.py` | the max-damage player's rules today |
| `research\pcsa-rl\rl2\sim_oracle.js` | in-process Showdown; starting point for the test recorder |
| `research\pcsa-rl\runs\team-gold\team.txt` | the gold team: Gholdengo, Garchomp (Mega Z), Golisopod (Mega), Primarina, Mamoswine, Cinderace |
| `docs\investigation\low-latency-recovered\` | the raw observation's code and column table: the specification of the gym's observation |

Target layout of this repo (folders are created when first needed):

```
README.md  PROJECT.md  Cargo.toml
engine/                 Rust: state, rules, turn resolution, outcome source, data tables
players/                Rust: random, max damage, tower, revealed-information view
bindings/               Rust + pyo3: batched env, arrays in and out
python/<package>/       gym interface, wrappers
data/                   generated game data + the pinned Showdown commit
tests/                  Rust and Python tests; regression cases
tools/showdown-oracle/  development only: recorder, comparer, data generator (Node)
gaps/                   the gap register
docs/investigation/     findings, coverage, audit, laptop, data-diff
bench/                  benchmarks and recorded results
```

## 6. Steps and status

| Step | What | Status |
|---|---|---|
| 1 | Record the decisions; one project document | done 17 Nov |
| 2 | Finish the read audit | done 17 Nov: 515 moves, 317 abilities, 166 items, 392 species, three mechanics slices; three rows the readers skipped were found by program and audited |
| 3 | Data comparison by program (every number, type and flag) | done 17 Nov (`data-diff/09-data-diff.md`): engine compiled and dumped; 23 moves, 6 Megas, 22 items and 1 ability have real data errors; the first read had found all but one (X4) |
| 4 | Check the audit four ways (below) | done 17 Nov (`docs/investigation/audit/completeness.md`): 657 used rows read twice; 3 real faults and about 14 details added (SR ids); every entry named by `pcsa`, by Showdown's handlers and by the descriptions has a row. Not done: the 10% sample of unused rows |
| 5 | Gap register: grouped by cause, sized, ranked by usage; shown to the user before code changes | done 17 Nov (`gaps/INDEX.md`, "Work list by cause": 30 groups W1 to W30 over 291 findings); waiting for the user's review |
| 6 | Set up the build: import at `f7b147f`, build on the desktop, toolchain to the laptop, speed baseline, take the upstream fixes, delete unused code | done 17 Nov: imported (commit 05c549a); builds on both machines; 956 engine tests pass; toolchain on the laptop under `C:\Users\grich\Code\pcsa\tools\rust`; the four upstream pull requests read, none taken as a patch (X5 to X8 in the register); baseline about 22,600 battles/s on the desktop and 26,300 on the laptop, one process (`bench/results/`); generations 1 to 8, Tera branches and the tree search deleted with tests, benchmark counts and data dump unchanged |
| 7 | Test rig: Showdown recorder with forced outcomes, state snapshot, comparer, data check | built 17 Nov (`tools/rig/CONTRACT.md`, `tools/showdown-oracle/`, `rig/`, `tools/rig/driver.py`, `report.py`, `triage.py`). First run, 500 battles: 48.4% of steps exact, 29.1% damage only, 19.0% other, with the PP shortcut set aside. Triage: 69 causes already in the register, 12 missed (N1 to N13). Open: five rig faults from the triage, probabilities recorded but not compared, no driver timeout |
| 8 | Core rewrite: one outcome per turn, exact damage, turn pipeline and end-of-turn order, statuses and field | running. Stage 1 done 17 Nov (`engine/src/sim/`): generated tables, copyable State (1,660 bytes), outcome source, turn skeleton, about 70 hook points. Plain battles: 3,000 battles, 64,510 steps, all identical to Showdown with outcomes forced; damage fixture 4,800 cases exact; about 27,900 battles/s. Stage 2 done 17 Nov: statuses, volatiles, 152 moves with their own rules, the end-of-turn order; 433 of 515 legal moves inside; 5,000 + 2,000 + 2,400 battles all identical, every pool move exercised at least 130 times; State 2,064 bytes; about 31,000 battles/s plain and 13,800 on stage-2 battles. Deferred: Skill Swap (needs abilities), Imprison (needs a "choice refused" reply). Stage 3 done 17 Nov (weather, terrain, rooms, screens, hazards, Wish, Future Sight, Revival Blessing): 492 of 515 moves inside; agent's runs 11,800 battles and the manager's fresh-seed check 5,000 battles (161,584 steps) all identical, 0 tape errors; 994 tests; State 2,240 bytes; about 28,700 battles/s plain, 13,100 stage-2, 11,300 stage-3 battles. Left for step 9: 14 moves that need items, 6 that need abilities, Skill Swap, Imprison. Stage 4 done 17 Nov (abilities): 300 of 318 abilities, the ability-changing moves, Imprison, refused choices in the rig; agent's runs 12,000 battles and the manager's fresh-seed check 5,400 battles (161,115 steps) all identical, 0 tape errors; 1,007 tests; State 2,464 bytes; 6 to 12% slower than stage 3 on the same pools. Stage 5 done 18 Nov (items, Mega Evolution, item moves, remaining abilities): 515 of 515 moves, 166 of 166 items, 82 Megas, 306 of 318 abilities (the 12 left are on no legal species); manager's fresh-seed check 15,100 battles all identical, of which 10,200 on the 430 real teams as written (213,748 steps), 0 tape errors, 0 unimplemented; 1,037 tests; State 2,608 bytes; about 4,300 battles/s on real teams on a busy desktop. Battle-log record done 18 Nov (`engine/src/sim/log.rs`, 37 row kinds, optional, under 1% cost): about 56,000 fresh battles with states and rows equal to Showdown's, 208 scripted battles identical. Old engine deleted 18 Nov (D17); 299 tests remain, all of the new code; the manager's check after the deletion: 2,500 battles and 208 scripted battles identical to Showdown with rows, 20,000 milestone games with every counter zero |
| 9 | Close the gaps in rank order | done 18 Nov as stages 4 and 5 of step 8: the new resolver was written from Showdown's source, so the register's gaps in the old engine no longer apply |
| 10 | The two players, the revealed-information view, the runner; **milestone** | done 18 Nov, on master (`emit/`, `battle/`, `players/`, `bench/src/players_run.rs`). Milestone reached on real teams with full rules: manager's check on new seeds, 100,000 tower vs max damage games, tower 50.5%, mean 6.0 turns, none unfinished, 200,000 of 200,000 Battles rebuilt from their events, about 24,000 battles/s on 24 threads with every check on. Tower 93.0% and max damage 92.5% against random. Over 295,200 games (real teams and 12 focused pools, 13.1M checked decisions) every counter is zero: Battle against State, legal choices, anomalies, prior-0 actions, truth labels, hidden-information replay. What the log cannot show is counted in named classes (opposing Illusion, unshown Regenerator, Natural Cure or trapping ability, and so on: `contracts/LIVE.md` section 7.8). Results: `bench/results/2026-11-18-rem-verify*.jsonl`, `2026-11-18-zero-target.jsonl` |
| 11 | The gym interface, packaging, standalone check | interface done 18 Nov, on master (`env/`, `obs/`, `bindings/`, `python/champions_gym`): BatchEnv with strict turn-taking, snapshot and fork, outcome enumeration, the observation of `contracts/MODEL.md` 0.5, labels, threaded stepping (`threads=`, same games for any count). Speed at 4,096 games (decisions/s, without and with observations): Rust 85k and 41k on 1 thread, 852k and 435k on 24; through Python 81k and 41k on 1 thread, 484k and 353k on 24. 1,262 Rust tests and 16 Python tests pass. Packaging done 18 Nov: `README.md` (every command and example run as written), `tools/build/wheel.py` writing `dist/champions_gym-0.1.0-cp312-cp312-win_amd64.whl` (2 MB, team pool inside, numpy optional), and the standalone check: a fresh clone outside the project folder built from scratch, the wheel alone installed in a new virtual environment, README examples, 16 Python tests, 100,096 random decisions all inside the mask, same seed same games. The build needs from outside the clone only the Rust toolchain, the cargo cache, llvm-mingw (still stored under `research`; set `LLVM_MINGW` once it is moved) and Python 3.12. Probability check done 18 Nov (`bench/src/prob_check.rs`, `tools/prob/`, results `bench/results/2026-11-18-prob-*`): 72 situations over 38 classes of random event, 200,000 seeded trials each against Showdown's source numbers, exact enumeration equal to the reference in all 72, Showdown's own measured frequencies (40,000 trials) agreeing on the 41 compared, all 515 moves' chance data equal, generator checks pass, 20,000 random whole games on each engine with no aggregate apart. Known limit: a tie when inserting an action among three or more tied positions takes the first position (`engine/src/sim/battle.rs:338`), believed unreachable in singles. To do: nothing Added 19 Nov: the live entry point `champions_gym.live.Session` (contracts/LIVE.md 0.7; replies to the controller in `docs/live-0.5-reply.md` and `docs/live-s8-reply.md`); training asks for a replacement before the turn's rows as the phone does (`replace_before_rows`, default on, user's decision LVA-1; MODEL.md 0.6); team errors reported, weighted team draw, batched built-in players; wheel 0.2.0. Manager's check: 388 Rust and 46 Python tests, 525,200 games across ten players and 12 pools with no unexplained counter, old order still identical game for game. |

**Step 4, the four checks.** Steps 5 onward wait for all four and for the user to see the result.

- **4a. Second read.** A different agent re-reads every row we use and every gold-team row, plus about 10%
  of the rest weighted to MATCH and PLAIN. A slice with more than 10% disagreement is redone.
- **4b. Completeness against `pcsa`'s feature code.** Every volatile, status, stat stage, weather, terrain,
  room, side condition, damage special case, trapping source and battle-log message that `pcsa` knows must
  have engine code and an audit row. Sources: `rl_training/features_rows.py`, `features_field.py`,
  `features_moves.py`, `features_log.py`, `turns.py`, `battle/damage.py`, `belief/calc.py`,
  `battle/trapping.py`, `players/no_effect.py`, `players/disciplined.py`, `assets/game/battle_text.json`
  with `battle/log_patterns.py`, the recovered `features_ll.py`, and `tower.py`.
- **4c. Completeness against Showdown's handlers, by program.** Every handler on every legal move, item,
  ability and condition maps to an audit row.
- **4d. Completeness against the descriptions.** For every legal move, ability and item whose description
  states an effect, each stated effect points to engine logic (file and line) or is recorded as MISSING.
- Output: `docs/investigation/audit/completeness.md`; every NOT COVERED entry is audited before the register
  is built.

**Trees of games (user, 17 Nov: all four pieces).**
- Step 8: the whole battle state is one plain value that can be copied, with no pointers into shared
  structures, so a battle can be forked at any point for the cost of a small memory copy.
- Step 8: every random event goes through one outcome source with three kinds: seeded (play), forced (tests
  and "what if"), and enumerating (lists every outcome of an event with its probability).
- Step 11: `snapshot` and `fork` in the Python interface, each copy with its own seed or its own forced
  outcomes; and a call that returns every outcome of a step with its probability.
- A tree grown from the engine uses both real teams, so it is a training and analysis tool, not live play.
- Added 19 Nov (0.5.0): a Game carries each seat's Battle when asked (`fork(g, seed, views=True)`):
  `Game.battle(seat)`, `Game.tensors(seat)` and `game_tensors` give what `BatchEnv` would show that seat
  after the same actions and outcomes, kept lazily through steps, forced steps and outcome children
  (`env/src/views.rs`). `Game.sample` draws outcomes with the engine's probabilities; `Game.readout` reads
  HP, faints, statuses and actives. `Session.game` / `game_from_battle` build a Game from a live Battle
  plus the caller's guesses for the hidden opposing sets (`env/src/from_battle.rs`; CG checks the guesses,
  never samples them; `UNKNOWN` lists what the Battle cannot tell), so a tree can also start in live play.
  `BatchEnv.restore(slot, game, seed)` puts a `snapshot(g, views=True)` back into a slot whose game just
  ended, as an ordinary batch game with its own seed (`env::batch::SlotSave`, tests `env/tests/restore.rs`, `python/tests/test_restore.py`).

**Step 8 design.**
- A new turn resolver in `engine/src/sim/`, built beside the old one. The old resolver and its tests
  were deleted on 18 Nov once the new one passed the rig on full teams.
- It changes the state directly and plays one outcome, in Showdown's order of steps. Effects of moves,
  abilities, items and conditions attach at named points (Showdown's events: before the move, on base power,
  on hit, on damage, at end of turn, and so on), dispatched by id.
- All game data comes from tables generated from Showdown (`tools/data/`), not from the old hand-written
  move table.
- Built in stages, each passing the rig with forced outcomes before the next begins:
  1. Skeleton: action order, switches, plain attacks with exact damage, PP and Struggle, fainting and
     replacements, the win rule. Rig battles restricted to plain moves, no abilities, no items: every
     legal-action set and every state identical to Showdown's.
  2. Statuses, volatile conditions, the end-of-turn sequence.
  3. Field: weather, terrain, rooms, screens, hazards.
  4. Abilities and items at their proper points, in usage order (this is step 9).
- Speed is measured after every stage against the baseline.

**Step 8 notes.** Damage takes the roll as an input and computes one hit with Showdown's integer arithmetic;
`audit/mechanics-formulas.md` part A is the specification. The turn pipeline follows
`audit/mechanics-turn.md`; statuses and field effects follow `audit/mechanics-conditions.md`.

**Terms (D16).** CG uses poke-env's names for the same things, so that talking about CG and about today's
code needs no translation.

| Term | Meaning in CG | poke-env equivalent |
|---|---|---|
| State | The engine's full truth: both complete teams and the field. CG only; poke-env has no such thing because Showdown holds it | none (Showdown's server-side battle) |
| Battle | What one player knows of a battle: own team in full, and of the opponent only what has been revealed. Earlier notes call this a "view" | `Battle` |
| `battle.team`, `battle.opponent_team` | Own Pokémon; the opponent's as revealed | same |
| `battle.active_pokemon`, `battle.opponent_active_pokemon` | The two Pokémon on the field | same |
| `battle.available_moves`, `battle.available_switches` | What may be chosen now | same |
| `battle.force_switch`, `battle.can_mega_evolve`, `battle.trapped` | Decision flags | same |
| `battle.weather`, `battle.fields`, `battle.side_conditions`, `battle.opponent_side_conditions` | Field state | same |
| `battle.turn`, `battle.finished`, `battle.won` | Progress and result | same |
| Pokemon, Move | One Pokémon and one move as the player sees them (`species`, `current_hp_fraction`, `status`, `boosts`, `effects`, `moves`, `item`, `ability`, `types`; `base_power`, `type`, `category`, `current_pp`) | `Pokemon`, `Move` |
| Player | Something that chooses: `choose_move(battle)` and `teampreview(battle)` | `Player` |
| Order | A choice: a move (optionally with Mega Evolution) or a switch | `BattleOrder` |
| Effect, Status, Weather, Field, SideCondition | The enumerations of conditions | same names |

Where CG has something poke-env lacks, it gets a plain name of its own and is listed here.

**State and Battle (user, 17 Nov).** The engine holds the State and always needs two complete teams to run.
The model never sees the State. In training and in live play it sees a Battle, and the two must be 1:1, as
close as possible.

- **Definition.** A Battle holds exactly what the Pokémon Champions battle log tells a player, and nothing
  more, even where the engine knows more. So: no critical hits, a miss in the game's form, no Choice Scarf
  reveal, no Protect activation, the opponent's HP as a percentage, hit counts without a name. The list of
  what the game's log shows is the definition; it comes from the phone side (`pcsa/docs/quirks.md` and the
  controller's findings).
- **What the game shows, not what today's translator manages.** Rows the phone translator cannot map, and
  its inferences, are controller faults to fix, not something training imitates.
- **One builder.** A Battle is built from a stream of battle events by one builder. In training the engine
  emits those events from the State, limited to what the game's log shows. Live, the phone's log is turned
  into the same events. Players and the observation builder take a Battle, never the State.
- **The event format is CG's own.** It does not have to be Showdown's line format. Showdown-style lines are
  kept only where they are convenient (the readable text log, the controller's own regression tests); if
  they get in the way of 1:1 they are dropped.

**Step 10 notes.** Unknown opposing stats use one fixed rule (neutral nature, no stat points, no item). One
constant-time damage estimate is shared by both players. Tower's optional pivot-switch rule is left out
unless asked for. Milestone: 10,000 tower vs max damage games on the laptop with no crash and no illegal
state; report win rate, game length and battles a second.

**Step 11 decisions (user, 17 Nov).**
- Team preview is a function passed to the environment, so different ways of choosing the three can be
  tested. It receives both rosters as each side may see them and returns the picks, lead first. Built-in
  ones ship (random, max damage, a policy over the 60 ordered picks).
- Strict turn-taking: every step asks exactly one seat for one decision. The engine resolves the turn only
  when it has both, so the turn logic is the same whatever drives it. This is slower per call than asking
  both seats at once; the speed is reported.
- An optional text log of each battle in Showdown-like lines, off by default, so a game can be read.

**Observation features (reviewed 17 Nov).** The proposed feature list is
`C:\Users\user\Code\pokemon_champions\pcsa-former\features_proposed.csv`: 12 rows of 300 floats (all 87 volatile conditions kept, plus the seven additions from the CG review) and
10 ids, a field vector of 66, a last-turn vector of 17 with 4 ids, the 11-action mask, and a fixed learnset table.
It replaces the low-latency layout named elsewhere in this file (12 x 268, 7 ids, field 55) as the working
list. It is a proposal the user is still revising, not a frozen specification: CG's job is to be able to
produce every kind of feature in it quickly, and the final list is fixed before step 11's observation is
built.

Decided (user, 17 Nov):
- Ids use CG's own compact numbering (392 species, 515 moves, 166 items, 318 abilities, plus padding), not
  `pcsa`'s vocabulary. The model is trained fresh on CG.
- What a player may choose is worked out by one rule from the Battle, the same in training and live:
  disabled moves, locked-in moves, trapped, the action mask and the legal counts. It is never taken from the
  State's truth, and live it does not depend on reading the move list off the screen. If CG is trustworthy
  the move-check step is not needed.
- A choice the Battle allows but the game refuses (a switch while trapped by an ability not yet revealed)
  is handled as the game handles it: the choice is refused, the Battle learns why, and the player is asked
  again. CG does the same in training, so both sides see the same thing.
- Damage features are computed from the Battle only: revealed items and abilities, and species bounds.

Every feature in the list can be produced without belief, catalogs or search, in five ways:

| How | Features | What CG must have |
|---|---|---|
| Table lookup | types, base stats, move type, category, power, accuracy, priority, the three move-class flags, possible abilities, the ids, the learnset table | the generated tables; a learnset table (not generated yet); the id numbering the model uses |
| Direct read of the Battle | HP, status, stages, the eight volatiles, item and ability known or gone, active, Mega, brought, PP, field, side conditions and their elapsed turns | the Battle holds each one, updated as events arrive |
| Running counters | turns on field, was lead, biggest hit taken, moved first and second counts, entry count, the four tendency rates per side, the whole last-turn block | the Battle builder keeps them as events arrive; the log is never re-read |
| Bounds arithmetic | surely faster, surely slower | each species' lowest and highest possible speed, precomputed |
| Damage calculation | each move's mean and maximum damage and the two KO flags, and each row's incoming hit | one damage function over a Battle, using only revealed information and species bounds. Roughly 60 to 100 calculations per decision: the only costly part. Cached per attacker, move and defender, and recomputed only when one of them changes |

Speed is measured when it is built: decisions a second with and without the damage features.

**Step 11 notes.** Batched environment, N battles, arrays out (observation, legal mask, reward, done, acting
seat). Actions: 4 moves, 4 moves with Mega, 3 switches, plus the 60 preview picks. Observation: the raw
inputs (rows 12 x 268, ids 12 x 7, field 55, last turn 20 + 4 ids), only what the acting side has seen.
Reward: win or loss; per-faint shaping optional and off by default. Two-seat API and a Gymnasium-style
single-agent wrapper. Builds to a wheel.

## 7. What must pass

- **Audit:** every row covered (515 moves, 317 abilities, 166 items, 392 species); second read done with the
  disagreement rate per slice; `completeness.md` has no NOT COVERED entry.
- **Data:** the program diff against Showdown reports zero differences.
- **Rules:** on 10,000 random battles with Showdown's outcomes forced on the engine, every legal-action set
  and every turn's state is identical. A gap the tests find that the audit missed is counted as an audit
  miss.
- **Randomness:** each event class has Showdown's probabilities; the sampler passes a chi-square test.
- **Speed:** the 8 Nov benchmark rerun after steps 6, 8, 9 and 10, reported against the 10,000 a second aim.
- **Milestone:** the 10,000-game run, plus a sample of those games replayed turn by turn on Showdown.
- **Gym:** same seed, same battles; 100,000 random decisions all inside the legal mask with no error; the
  observation never contains anything the acting side has not seen.
- **Standalone:** a fresh clone on a machine with no `pcsa`, `research` or Showdown installs and runs the
  README example and the tests; no import of or path into them outside `tools/` and `docs/`.

## 8. Evidence index (`docs/investigation/`)

| Folder | What |
|---|---|
| `findings/` | one report per question: 01 engine internals, 02 format support, 04 what our code reads from poke-env and Showdown, 05 how to compare two engines, 07 licence and upstream, 08 the raw observation; `-check` files are the checker's results |
| `coverage/` | every legal species, move, item and ability with a `used` column: the audit's row lists |
| `audit/` | the engine read against Showdown row by row; `INSTRUCTIONS.md` is the readers' brief |
| `laptop/` | what can be built and run on the laptop, with the scripts and their outputs |
| `data-diff/` | the Showdown data dump script for step 3 |
| `low-latency-recovered/` | the raw observation's Python code, benchmark and column table |

Reports written on 16 Nov name the old locations (`C:\Users\user\Code\...`, `investigation\...`).
