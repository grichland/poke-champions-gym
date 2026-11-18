# 05 - Differential testing: Showdown vs the poke-engine fork

Investigator 5. Read-only study (no code run). "V" = verified by reading the file cited; "I" = inferred.

## Summary (10 lines)

1. An in-process Showdown oracle already exists (`research/pcsa-rl/rl2/sim_oracle.js` + `.py`): a Node child speaking JSON lines, with `create` (rebuild a mid-battle position), `step` (clone, apply both choices, play the turn) and `release`. It returns the seat's protocol lines, both requests and a summary snapshot (V).
2. Its scripted PRNG is NOT a replayable tape. It picks the single most likely outcome by inspecting the JS call stack (accuracy hits, no crits, secondary 100% only, damage roll index 7, tie goes to p1). It cannot force a specific outcome (V, sim_oracle.js:54-94).
3. So today there is no way to force Showdown down a chosen branch and no way to enumerate its outcome distribution. Both are needed; both are small additions to the same PRNG hook (Showdown already routes every random draw through `prng.random/randomChance/sample/shuffle`).
4. poke-engine does not produce 16 damage rolls. It collapses a hit to `trunc(max*0.925)` and only branches on crit and on "kill roll" (V, generate_instructions.rs:2420-2480). Per-HP exact equality with Showdown is impossible without an exact-roll mode in the fork.
5. The 8 Nov damage test compared only the max non-crit roll on 220 random triples: 172 exact, 31 within 1 HP/1%, 6 within 5%, 11 off by more than 5% (Disguise x4, Make It Rain x3, three Showdown-side harness artefacts incl. Imposter, plus Endeavor). No turn-level differential test was ever run (V).
6. Recommendation: Showdown-led design (b) as the core, with a Showdown "path explorer" (a DFS over the PRNG decision tree) to produce full outcome distributions for design (c); (a) is the same machinery in reverse. Use (d) replays for volume and for usage-weighted coverage.
7. Replayable data: ~548 phone battles (536 with a reconstructed `showdown.log`, 8-16 Nov 2026) and ~4,183 sim archive files (about 3.7 GB of seat-view protocol + teams; of the order 10^6 games, I) plus 86 validation game files. None stores a PRNG seed or the choice list. Nothing is exactly re-simulable in Showdown; all are replayable with inferred outcomes.
8. Phone battles lack crits, damage rolls (own HP exact only when we are hit, opponent in whole percent), hidden opponent sets, PP, and speed-tie results. They test Showdown-vs-game as much as engine-vs-Showdown, so they rank gaps by usage rather than gate pass/fail.
9. Match levels: L1 (exact hidden state after each turn, per outcome class) is the gate; L0 legal actions is the cheapest and best first test; L2-L4 are downstream and already covered by `rl2.sim_oracle.compare/feed`.
10. Top risks: damage-roll collapse; PRNG call-order differences (Showdown consumes draws even when the outcome is certain); stack-name keyed scripting being brittle; Showdown versions (vendor 0.11.11 vs the older `research` copy); opponent builds on recorded battles are guesses; hidden counters (sleep/toxic/screens/weather turns).

---

## 1. The existing oracle (what it is exactly)

Files: `C:\Users\user\Code\research\pcsa-rl\rl2\sim_oracle.js` (387 lines), `sim_oracle.py` (528 lines), `rl2/tests/test_sim_oracle.py` (275 lines). Callers: `rl2/search_sim.py:22,266,469` (search over Showdown), `runs/agents-20261115/e_bench.py`, `e_smoke.py` (V, grep).

Showdown build used: `PCSA_SHOWDOWN_SIM` or `C:/Users/user/Code/pcsa/.tools/vendor/pokemon-showdown/dist/sim` (sim_oracle.js:34), package version 0.11.11 (V, package.json). The `research/pcsa-rl/node_modules/pokemon-showdown` the brief mentions does not exist (V, cd failed); the vendor copy has `.ts` sources in `sim/` and `data/mods/champions/` (V). Node is `pcsa/.tools/node/node.exe` (sim_oracle.py:20). The damage test uses the same vendor build (damage_showdown.js:4).

### API (JSON lines on stdin/stdout, one reply line per request line; sim_oracle.js:3-22, 364-387)
- `create {format, turn, ours, sides:{p1,p2:{name, team:[PokemonSet + state], megaUsed}}, field}` builds a `Battle` (format id default `gen9championsbssregmc`, seed [1,2,3,4], sim_oracle.js:153), auto-picks team preview as the whole team in order (164-169), then patches every mon to the given state: `state` = hp, hpFrac, status, statusCounter, boosts, fainted, pp, item, mega form, activeTurns, moveActions, lastMove, volatiles (substitute, leechseed, choicelock, encore, confusion, taunt, yawn); `field` = weather(+turns), terrain(+turns), pseudo-weather (trick room, gravity...), side conditions with turns/layers (196-300).
- `step {bid, p1, p2, auto}` clones the parent through `State.serializeBattle -> JSON -> deserializeBattle` (302-307), applies both choices (invalid ones fall back to `autoChoose`, listed in `invalid`), plays the turn until a side we decide for has a request. `auto` sides pick their own forced replacement (highest-weight bench member) (329-362).
- `release`, `reset`, `summary`, `ping` (returns draw counters), `debug` (adds omniscient `lines_all`).
- Reply `view`: `lines` (our channel's new protocol lines via `extractChannelMessages`), `requests` {p1,p2 = `side.activeRequest`}, `ended`, `winner`, `summary`, `invalid`, `pending` (137-149).
- `summary`: turn, weather, terrain, pseudo-weather, per side: per mon {name, species, hp, maxhp, status, fainted, active, item, ability, boosts, moves [id,pp], volatiles (keys only), canMega} + `left` + side conditions with layers/duration (110-127). This is a partial L1 snapshot: it omits status counters (sleep time, toxic stage), volatile counters/data, PRNG-free `lastMove`, `activeTurns`, side-condition source, `weatherState.duration` is only via `duration` of side conditions, not weather.

### The scripted PRNG (sim_oracle.js:39-95)
`script(battle)` replaces `battle.prng` with `ScriptedPRNG` (also done again after `deserializeBattle` in `clone`). It overrides exactly Showdown's `PRNG` public methods:
- `randomChance(n, d)`: true if n >= d; true if the caller (stack frame 3) is `hitStepAccuracy` or `hitStepMoveHitLoop`; otherwise false. Hence crits, paralysis full-stop, confusion self-hit, thaw, Shed Skin, Quick Claw, ability/item procs, Protect-counter chain all return false (Protect with counter 1 is `n>=d` = true).
- `random(m[,n])`: no args -> 0.5. If the caller is `secondaries` or `selfDrops` -> `hi-1` (so only chance=100 secondaries fire, since `roll < chance`). If caller is `sort`/`insertChoice`/`speedSort` -> `lo` (first candidate). Otherwise the median `floor((lo+hi-1)/2)`: damage `random(16)` -> 7 (roll factor 93), confusion `random(2,6)` -> 3, magnitude, psywave, partial trapping `random(5,7)` -> 5.
- `sample(items)`: most frequent item; numeric ties -> nearest the mean (2-5 hits [2x7,3x7,4x3,5x3] -> tie 2/3 -> mean 3.1 -> 3), else first item. Champions sleep `sample([2,3,3])` -> 3 turns.
- `shuffle`: identity (speed ties: p1 first).
- `getSeed/clone/setSeed`: no-ops. It never consumes a real stream, so call count and order are irrelevant to it.

Hence a "script entry" is not an entry at all: it is a fixed policy keyed by (method, calling function name). There is no per-call tape. Hooking by `new Error().stack` is slow and fragile (regex `at (?:[\w$]+\.)?([\w$]+) `; depends on function names in the dist build; Champions overrides `hitStepMoveHitLoop` in `data/mods/champions/scripts.ts:426` which happens to keep the name).

### Speed
No dedicated micro-benchmark. From the engine-backed search bench (`runs/agents-20261115/e/bench-d2-s128.json`, 200 positions): median 181 ms per decision with ~14-21 `step` calls per decision (rows show `steps` 14-21) -> roughly 8-13 ms per `step` including JSON clone and Python round trip (I); first `create` 266 ms (warmup), later ~3 ms. Compare the poke-engine native loop: 314k steps/s per process (poke-engine-eval/out/scale.jsonl, V). So the oracle is 1,000x slower than the engine, adequate for a differential harness (about 100 steps/s/process; a 5k-leaf enumeration is about a minute).

### How it is tested
`rl2/tests/test_sim_oracle.py` (V): `create` reproduces a position (7 assertions on turn, weather, terrain, side conditions, HP/status/boosts, fainted, requests); clone independence and determinism (same inputs -> identical lines and summary); a known-damage turn (Endeavor, Leftovers, 3x identical Gunk Shot with no crit/miss/poison); KO replacement and terminal; pivot waits for our switch; choice-lock carried; first-impression follows move actions; a live round trip (`test_live_root_and_round_trip`, 157-188): rebuild the root from a live decision's `protocol` + `request` (via `pcsa.battle.state.replay`), `compare()` actives/HP/status/boosts/field against the poke-env battle, step one move, feed the oracle's lines back into a forked poke-env battle (`feed`), and compare again. `e_bench` fidelity run: 200 live positions, `fidelity_exact 200, mismatch 0` (V, bench-d2-s128-final.json summary). This checks that Showdown plus poke-env parsing agree with a rebuilt root; it never compares a second engine.

### Limits
- One outcome per (state, actions); no enumeration, no forcing.
- Rebuilt roots are approximate (their stats, unrevealed items, HP +-1%, remaining weather turns, PP, Illusion; sim_oracle.py:3-5).
- `summary` omits counters; volatiles are names only.
- `compare()` checks only actives/HP/status/boosts/field/species, not PP, items, volatile counters.
- Team preview is hard-wired to "bring all, in order".
- Format/ruleset `Gen9 Champions BSS Reg M-C` is selected by the format id; mods load from vendor `data/mods/champions`.

---

## 2. Showdown random-call inventory (vendor 0.11.11, mod `champions`)

All draws go through `Battle.random/randomChance/sample` (`sim/battle.ts:347-358`) to `PRNG` (`sim/prng.ts:91-155`). Important PRNG facts (V, prng.ts): `randomChance(n,d) = random(d) < n` always consumes one draw even if n>=d; `sample(items)` = `random(len)`; `shuffle` = Fisher-Yates, one `random(start,end)` per position (`prng.ts:150-157`); `Battle.forceRandomChance` exists but only in debug mode and only for `randomChance` (`battle.ts:214,352`). The generator is the Gen5 LCG or sodium depending on seed (`prng.ts:58-68`).

### Core, in the order of a turn (singles)
| # | Where | Call | Consumed even when outcome certain? |
|---|---|---|---|
| 1 | Turn start `battle.ts:1805-1816` | none for gen9 (Quick Claw rolls are gen2/3 only). | n/a |
| 2 | Action order: `battle-queue.ts:426` -> `battle.speedSort` `battle.ts:434-462` -> `prng.shuffle(list, sorted, sorted+k)` | Only when two actions compare equal (same priority, fractional priority, speed): k-1 draws `random(start,end)`. Singles: one `random(0,2)`. Note: ties are over the whole action list including switches and mega-evolution actions. | Only on ties |
| 3 | `battle-queue.ts:406` `insertChoice` | `random(firstIndex,lastIndex+1)` when inserting an action (e.g. after a switch-in, Instruct/After You style inserts) among equal-priority existing actions | only when range > 1 |
| 4 | Quick Claw `data/items.ts:5017` (`onFractionalPriority`) | `randomChance(1,5)` when `priority <= 0` | yes (called every action for the holder) |
| 5 | Quick Draw `data/abilities.ts:3739` | `randomChance(3,10)` for damaging moves | yes |
| 6 | Before-move conditions: Paralysis `data/mods/champions/conditions.ts:5` `randomChance(1,8)`; Freeze `:49` `time<=0 || randomChance(1,4)` (short-circuits when the 3-turn timer is up); Sleep timer is deterministic after start; Confusion `data/conditions.ts:187` `!randomChance(33,100)` (after decrementing the counter); Attract `data/moves.ts:745` `randomChance(1,2)`; Disable/Taunt/Encore none. | yes |
| 7 | Confusion self-hit damage `battle-actions.ts:1914` `randomizer` -> `random(16)` | yes (when the self-hit happens) |
| 8 | Protect/Detect/Endure chain `data/conditions.ts:456` `randomChance(1, counter)` (counter 3,9,27...) | yes, also when counter is 1 (`random(1)<1`) |
| 9 | Accuracy `battle-actions.ts:800` `randomChance(accuracy,100)`; skipped only when accuracy `=== true` (alwaysHit, self-targeting status). A numeric 100 still consumes a draw. Multi-accuracy per hit `:997` and Champions override `data/mods/champions/scripts.ts:507`. | numeric accuracy 100 draws |
| 10 | Multi-hit count `battle-actions.ts:929-940` (Champions override `scripts.ts:439-450`): 2-5 hit moves `sample([2x7,3x7,4x3,5x3])` (35/35/15/15), Loaded Dice `5 - random(2)` or `10 - random(7)`; other ranges `random(lo,hi+1)` | n/a |
| 11 | Crit `battle-actions.ts:1695` `randomChance(1, critMult[ratio])`, crit table `[0,24,8,2,1]` for gen 7+ (`:1685`), only when `move.willCrit === undefined` and `critRatio > 0` (always, for damaging moves). Drawn BEFORE `CriticalHit` event (Battle Armor/Shell Armor/Lucky Chant cancel after the draw). | yes (per damaging hit, per target) |
| 12 | Damage roll `battle-actions.ts:1809` -> `battle.ts:2407-2410` `trunc(trunc(baseDamage * (100 - random(16))) / 100)`: 16 equally likely roll factors 100..85; `random(16)` = 0 is the max roll, 15 the min. Skipped for fixed-damage / OHKO moves that return before `modifyDamage`. Not skipped for 1-damage cases. | yes |
| 13 | Secondary effects `battle-actions.ts:1399` `random(100)` PER secondary, `roll < chance`; drawn even when `chance` is 100 or undefined (the draw is before the check). Self-boost secondaries `selfDrops` `:1381`: `random(100)` only when `moveData.self.boosts` and not `isSecondary`. Filtering for Shield Dust/Sheer Force happens before the draw (`ModifySecondaries`). Order = order of the secondaries array, per target, after the damage of each hit. | yes |
| 14 | After-hit abilities/items: Static, Flame Body, Poison Point, Poison Touch, Cute Charm, Cursed Body, Toxic Chain (`abilities.ts` 788,801,1320,3348,3376,4540,5097: all `randomChance(3,10)`), Effect Spore `abilities.ts:1164` `random(100)`, Focus Band `items.ts:2273` `randomChance(1,10)` (only evaluated if the hit would KO), Shed Skin `abilities.ts:4192` `randomChance(33,100)`, Healer `:1823`, Harvest `:1805` (sun short-circuits), Moody `:2715-2726` samples, Pickup `:3272`, Trace `:5145`, Forewarn `:1526`, Champions Healer variant `data/mods/champions/abilities.ts:43`. | yes (each holder's trigger evaluates its draw when the event fires) |
| 15 | Move-internal draws (`data/moves.ts`): Metronome 11803, Assist 632, Sleep Talk 16887, Present 13926, Magnitude 10903, Psywave 14294, Ally Switch 343, Tri Attack 19853, Dire Claw 3641, Fickle Beam 5231, Shell Side Arm 16232, Conversion 2 2839, Acupressure 155, Star Fruit item 6036 | per move |
| 16 | Status durations: Sleep `data/mods/champions/conditions.ts:23` `sample([2,3,3])` (1/3 wakes at the second turn, 2/3 third); Freeze 3-turn timer (`:30`, 1/4 thaw per turn); Confusion duration `data/conditions.ts:174` `random(min,6)` (min 3 for Axe Kick, else 2) -> 2-5 turns (counter decrements before the check); Partial trapping `:227` `random(5,7)`; Locked move (Outrage) `:265` `random(2,4)`; Two-turn `:305` only for random-target (doubles) | duration draws are at status/volatile start |
| 17 | Random target / switch: `battle.ts:1586` `getRandomSwitchable` (`this.sample(canSwitchIn)`), `battle.ts:2523-2532` (doubles-only), `side.ts:372` | rare in singles |
| 18 | Gender at team construct `pokemon.ts:361` `sample(['M','F'])` when the set has none and the species has no fixed gender | construct time |

Other facts: Champions HP shared to opponents is `floor(100*hp/maxhp)` min 1, with `y/r/g` colour suffix at exactly 20% and 50% (`pokemon.ts:2087-2095`), own side exact `hp/maxhp`. Spread modifier 0.75 only for doubles (`battle-actions.ts:1788`), Parental Bond second hit 0.25. Damage order (V, `modifyDamage` lines 1780-1880): `baseDamage = floor(floor(floor(2L/5+2)*bp*atk/def)/50)`, `+2`, spread/PB, weather, crit x1.5 (trunc), random roll (trunc), STAB via `modify`, type effectiveness (x2 loop or trunc /2 loop), burn x0.5, `finalModify`, min 1.

Practical consequence for "scripted alignment": in a seed-replay world, every draw must be consumed in the same order or the stream desyncs; in a semantic-tape world (below) call order doesn't matter, but the enumerator has to know which draws are "meaningful" (e.g. `random(100)` for a 100% secondary has one equivalence class, not 100).

---

## 3. Comparison design options

All options need the same Showdown-side primitive, so build it first: a **ControlledPRNG** replacing `ScriptedPRNG`, driven by a decision tape, labelled by semantics instead of stack names.

ControlledPRNG sketch (Showdown side, ~150 lines of JS):
- Every call is tagged `{kind: random|chance|sample|shuffle, args, site}` where `site` is `battle.effect.id` / `battle.event.id` / `target` / `source` (available at call time; no stack inspection) plus the ordinal inside the turn.
- Modes: `record(seed)` runs a real seeded PRNG and logs `{site, kind, args, result}`; `force(tape)` returns scripted results keyed by ordinal or semantic key and fails loudly on a mismatch; `explore` = DFS enumerator: unseen call -> take alternative 0 and remember the alternative count and weight; after the turn, re-run from a cloned root for each unexplored sibling. Weights: `random(n)` uniform 1/n per value (merge equivalent values at `secondaries`: roll<chance vs not, weights chance/100); `randomChance(n,d)` weights n/d, 1-n/d; `sample` empirical frequencies; `shuffle` permutations (singles: 2 orderings).
- Canonicalisation: merge leaves with identical canonical L1 state (adding weights), keep the set of outcome tags for the report.

### (a) Branch-forcing (engine branch -> Showdown)
Take the engine's instruction branches for `(state, a1, a2)`, translate each branch to a tape (crit yes/no, hit yes/no, secondary yes/no, speed order, kill-roll vs average), run Showdown on that tape, compare states.
- Detects: wrong state change given that outcome class; wrong ordering; missing mechanics; wrong counters.
- Cannot detect: branches the engine fails to emit (missing outcome classes), wrong probabilities, wrong set of legal outcomes.
- Needs from the engine: branch metadata (outcome tags), and exact roll semantics (otherwise HP differs by up to +-7.5% from the average-roll collapse, see section 4).
- Difficulty: medium; translation table from tags to Showdown call sites is the work.

### (b) Showdown-led (Showdown -> engine)
Run Showdown with a seed (or random legal play), `record` mode produces a tape of every semantic outcome (hit/miss, crit, roll index 0-15, secondary yes/no, status draw, tie result, multi-hit count, sleep length...). Feed the same outcomes to the engine via a forced-outcome API, compare states after each turn.
- Detects: wrong state change for every outcome that Showdown actually produced, including rare combinations; finds missing mechanics and wrong order. Real games hit common outcomes; rare ones appear with volume.
- Cannot detect: wrong probabilities; outcome classes the engine could produce but Showdown wouldn't (engine over-branching).
- Needs: `step_forced(state, a1, a2, outcome_script) -> state'` (or an outcome-selector that overrides each chance node), plus an exact-roll mode.
- Difficulty: low-medium once ControlledPRNG exists. Highest bug-finding value per unit of effort.

### (c) Distribution comparison
For one `(state, a1, a2)`: Showdown's `explore` gives the full leaf set with weights; the engine's `generate_instructions_from_move_pair` gives `Vec<StateInstructions{percentage, instruction_list}>` (V, instruction.rs:12-15, generate_instructions.rs:4065). Compare as multisets of (canonical state, probability).
- Detects: probability errors (crit rate 1/24 vs 1/16 style, secondary chance, speed-tie 50/50, hit probabilities, sleep-turn mix 1/3-2/3), missing/extra outcome classes, wrong branch merge.
- Cannot detect: nothing about the states beyond what the enumerated cases contain; combinatorial blow-up (16 rolls x crit x hit x secondaries for both moves is of the order 10^3-10^4 leaves).
- Needs: engine option to enumerate rolls exactly (or a coarse Showdown classifier that collapses rolls into classes: min/median/max + the KO boundary roll); a total-variation distance tolerance (e.g. 1e-6).
- Difficulty: medium-high; do for short targeted cases (mechanic taxonomy below), not for fuzzing at scale. Roll collapse: set the explorer's roll alternatives to {0, 7, 15 + every roll where HP outcome changes class (KO / sash / sitrus threshold)} and compare engine under `Max/Avg/Min` modes.

### (d) Replay recorded battles
Walk a protocol log turn by turn, rebuild the state (rebuild-root exists: `root_op`), infer the random outcomes from the next turn's lines, force the engine (and Showdown, for a ground-truth check), compare.
- Detects: realistic state combinations, real teams and moves (usage-weighted), the actual distribution of mechanics seen in play. Phone battles also reveal Showdown-vs-game errors.
- Cannot detect: anything not visible in logs (crits on phone, exact opponent HP, hidden sets, PP, counters); ambiguity in damage roll for opponent HP.
- Needs: outcome inference (log -> tape) and the same forced-outcome API as (b).
- Difficulty: medium-high (inference + rebuild). Rebuild approximations (their sets, HP) lead to false failures, so run (d) first against Showdown (does the rebuilt tape reproduce the logged lines?) to measure replay-noise before blaming the engine.

### Recommendation
1. Build ControlledPRNG in Showdown (record / force / explore) and a Python driver `diffcheck` that talks to it like `SimOracle` does now.
2. Core gate = (b) with random legal play in Showdown (millions of turns, tags recorded), engine given the same outcome tape; mismatch triage by tag.
3. (c) for a hand-built list of ~200-400 mechanic cases (taxonomy 6) to verify probabilities and branch completeness; also run as a regression on every engine change.
4. (a) = (b) in reverse; implement only as a cheap mode of (c).
5. (d) for usage-weighted coverage and as "real game" sanity; run on sim archives first, phone later.

---

## 4. What "match" means

| Level | Definition | Notes |
|---|---|---|
| L0 | Legal action sets per side equal (move slots incl. disabled/choice-lock/encore/taunt/no PP, switches incl. trapped/maybeTrapped, Mega option, forced switch, team preview picks) | Compare engine `get_all_options` with Showdown `request.active[0].moves[].disabled`, `side.pokemon[].condition`, `canMegaEvo`, `forceSwitch`, `trapped`. Cheap, runs on every state, finds many bugs. Choice-lock and PP-exhaustion details count. |
| L1 | Canonical full hidden state after each turn per outcome class (below) | Primary gate. HP exact (integer), counters exact. |
| L2 | Protocol lines + request JSON | Only needed if the engine claims to be a protocol emulator. Not required for training; skip, but keep Showdown-side lines as evidence. |
| L3 | poke-env `Battle` built from engine output | Needed only if pcsa features read poke-env; reuse `rl2.sim_oracle.feed/compare`. |
| L4 | Observation arrays | Reuse existing feature code; compare arrays from the two engines' state exports. |

### Canonical snapshot (JSON; both engines export; compare field by field)
Top: `turn`, `ended`, `winner`, `weather` {id, turnsLeft}, `terrain` {id, turnsLeft}, `pseudo` {id: turnsLeft} (trick room, gravity, magic/wonder room), `speedOrderTieFlags`.
Per side (sorted by slot): `sideConditions` {stealthrock, spikes layers, toxicspikes layers, stickyweb, reflect/lightscreen/auroraveil/tailwind/safeguard/mist turnsLeft}, `megaUsed`, `lastMoveUsedBySide`, `pokemonLeft`.
Per mon: `species` (incl. forme), `types`, `ability` (current + base), `item` (current; "" consumed/knocked; `itemUsed` for Sitrus/Berry history), `level`, `maxhp`, `hp` (exact), `status` (+ `statusCounter`: sleep turns left, toxic stage, freeze timer), `boosts` (7 stats incl. accuracy/evasion), `moves` [{id, pp, maxpp, disabled}], `active`, `fainted`, `lastMove`, `activeTurns`, `moveActions`, `volatiles`: [{id, duration/counter, data}] (substitute hp, leechseed target, confusion turns left, taunt/encore/disable turns + move, yawn turns, protect counter, choicelock move, locked-move turns, partiallytrapped turns, perish count, torment, flashfire, charge, destinybond, focusenergy, ingrain, aquaring, curse, nightmare, lastBerryUsed...), `transformed/illusion` flags, `canMega`, `forme details`, `stats` (computed at level 50 for all six; used for speed ties).
Canonicalise: sort volatiles by id, normalise `turnsLeft` as "end-of-turn remaining" (the two engines count differently), strip PRNG data, round nothing.
Showdown side: `Pokemon.toJSON()` via `State.serializeBattle` plus the extractor already in `sim_oracle.js::summary/monSummary` (extend with counters/volatile data).

### Damage-roll exactness
- Showdown: 16 discrete rolls `trunc(trunc(base*(100-r))/100)`, r in 0..15, then STAB/type/burn/final through 4096-fixed `modify` chains (`battle.ts:2285-2344`), min 1; own-side HP exact, opponent shared HP `floor(100*hp/maxhp)||1` (Champions, `pokemon.ts:2089`).
- Engine: computes the max roll then `average = trunc(max*0.925)`, `min = trunc(max*0.85)`; with `branch_on_damage` it branches only crit vs non-crit and kill vs non-kill (V, generate_instructions.rs:2419-2480; `calculate_damage_rolls` uses `DamageRolls::Max`). This cannot equal any Showdown roll exactly: for max damage 100 the engine gives 92, Showdown rolls give 85,86,...100 (roll factor 93 -> 93).
- Therefore the fork must provide an exact-roll mode (roll index 0-15 -> integer) and the damage pipeline (every `pokeRound`/`trunc` stage) must match Showdown's chain. Then the match criterion for L1 HP is exact integer equality for the roll index the tape specifies. The 8 Nov test shows today's engine gets only 172/220 exact even at the max roll.
- Until then use bracket matching: engine damage must lie within `[min roll, max roll]` of Showdown for the same class, and the engine's `average` should equal Showdown's roll with factor 93 only if the fork keeps the average semantics (document the intended tolerance, do not hide it).
- HP-rounding issues: Showdown applies integer HP; healing `trunc(maxhp/16)`; Leftovers, Sitrus (`trunc(maxhp/4)`), recoil `round`, Substitute `floor(maxhp/4)`. Opponent HP in protocol is floored percent (Champions) so percent-only comparisons collapse 1-2% differences: compare internal integer HP wherever possible, percent only for replay inference.

---

## 5. Recorded-battle inventory

| Source | Count / range | Record format | Teams / sets | Seed? | Replayable exactly in Showdown (i) | Replayable in engine with inferred outcomes (ii) |
|---|---|---|---|---|---|---|
| Phone live battles, `C:\Users\user\Code\pcsa\output\runs\*\battle-N` | 548 battle dirs in 51 runs (`20261108-170344-run` .. `20261116-133224-run`), started 2026-11-08 17:04 to 2026-11-16 14:29 (V); 536 have `showdown.log` + `battle_final.txt`; outcomes 184 win, 260 loss, 71 opponent_forfeit, 29 unknown; 5,378 `decisions/*-action.json` | `showdown.log` is a reconstructed Showdown protocol (own seat only, ~70 lines/game, no `|request|`): our HP `hp/maxhp`, opponent `hp/100`; plus `battle_log.txt` (OCR text of the game log with `[Move]`, `[HP Lost]`), per-decision JSON with `protocol`, synthesized `request`, `legal`, `chosen`, `belief`, `order` (V) | Our team full (`own_team.txt`, EVs/nature/moves; V). Opponent: species only as seen/previewed; moves/items/abilities as revealed. Opponent Mega/forme lines exist | `seed` in manifest is the bot run seed (1583502531), NOT the game's RNG | No (real game, not Showdown; no seed) | Partial. Missing: `|-crit|` (0 hits in `showdown.log`, 0 "crit" in `battle_log.txt`), `|-miss|` present (109 lines), `|cant|` present (140), no damage rolls (only our HP exact), no PP, no opponent hidden stats/items/ability until revealed, no speed-tie info, 28 unmapped rows remain after reconciliation (live-reconciled/9c5d1a0/report.md) |
| Live reconciled, `research/pcsa-rl/runs/live-reconciled/<commit>/pcsa/<run>/battle-N` | 10 snapshots (16efc43 .. 9c5d1a0 + `-2` variants); latest `9c5d1a0`: 375 battles, 4,291 records, 4,171 reconciled; `16efc43`: 298 battles (V, reports) | same shape as above after a translator pass (`battle.json`, `battle_final.txt`, `decisions`, `showdown.log`), unmapped rows 206 -> 28 | as above | no | no | partial; prefer these over the raw phone logs (more protocol lines added: 1,145 added / 453 removed) |
| Sim RL archives `research/pcsa-rl/runs/*/archive/u*.jsonl.gz` | 4,183 `jsonl.gz` files (about 3.68 GB), dirs: s1 1,323; v2-scratch 801; team3 287; bench-hard 153; v2-belief 150; v2-gold 122; v2-league-1115 113; others 100-1 each; file dates 2026-11-08 to 2026-11-16 (V find). Games per file: `v2-gold` update 854 log row shows `games 1502`, `battles 1210` (V) -> order 10^6 games in total (I) | One JSON row per episode: `opponent`, `snapshot`, `seat`, `won`, `turns`, `team`, `opp_team`, `protocol` (the seat's full Showdown protocol, one `|`-joined message per line; `rl/rollout.py:64`, `rl2/archive.py:46-50`) plus a sibling `.npz` of decisions (`rows, field, last, ids, mask, action, logp, value, reward, turn, forced`; `rl2/archive.py:15-17,26-37`) | Team export text for both sides (sim opponents are our own bots/scripts/teams so full sets are present, I) | No. Showdown servers randomise seeds (no `seed`/inputLog written; `servers/actorN.log` has only startup; `vendor/logs` has no battle logs) | Not exact | Yes (ii): protocol is Showdown's own, so crit/miss/status/damage lines are real. Seat view only: opponent HP in floored percent; own HP exact; their EVs unknown but their `opp_team` set is given (I); choices can be read from `|move|`/`|switch|` lines; damage roll for own mon identifiable (16 values), for theirs partly |
| Validation game files `runs/**/validate/*.games.jsonl.gz` | 86 files (19.4 MB) | one row per game: `opponent, seat, won, turns, protocol` (`rl2/train.py:518-531`, V) | no team fields (lighter than archives) | no | no | yes (ii), small |
| `runs/live-parity/20261109-205551-run` | 1 `obs.npz` + `report.json` | observation parity data | n/a | n/a | n/a | n/a |

Phone vs engine: the phone game is the real Pokemon Champions client, not Showdown, so mismatches there may be Showdown-vs-game rather than fork bugs. Use them for usage and for sanity checks (our HP loss on a hit, speed order, status/ability presence), not as pass/fail.

---

## 6. Test-case taxonomy

Layers, in order of cost:
1. **Unit probes, L0:** legal-action sets for crafted states: choice lock, taunt, encore, disable, PP 0, trapped (Shadow Tag/Arena Trap/Magnet Pull), forced switch, Mega availability once per side, Sucker Punch/Fake Out first-turn legality.
2. **Per-mechanic outcome-class tests with (c):** one state per mechanic, enumerate and compare distributions. List: speed order (priority, Trick Room, Tailwind, paralysis x0.5, Choice Scarf, Quick Claw, speed ties 50/50), accuracy/evasion boosts, OHKO moves, crit (rates 1/24, 1/8, 1/2, always; Battle/Shell Armor; Focus Energy; Scope Lens), 16 damage rolls and KO boundary, STAB/type chart/immunities (Levitate, Flash Fire), weather/terrain residuals and turn counters, status application and duration (sleep `[2,3,3]`, freeze 3-turn timer + 1/4, confusion 2-5, paralysis 1/8, burn/toxic residual stages), secondary effect chances (10/20/30/50/100), Sheer Force, Shield Dust, multi-hit (2-5 distribution, Triple Axel, Skill Link, Loaded Dice), Protect chain (1/3, 1/9...), substitute, Leech Seed, Yawn timeline, Perish Song, Wish, hazards on entry (Stealth Rock, Spikes layers, Toxic Spikes absorption), screens with Light Clay (8 turns), Mega Evolution ordering (acts after mega but base speed rules in Champions), switch-in abilities (Intimidate, Drizzle/Drought/Sand Stream/Snow Warning, Electric Surge), pivot moves (U-turn/Flip Turn/Volt Switch/Parting Shot/Teleport) with Eject Button, Emergency Exit, Red Card, Eject Pack; forced switch after KO.
3. **Per-move, per-item, per-ability cases:** generated from Champions data: for each move in the Champions movepool a 1-on-1 state with the move (target types spanning immunity), run (b) with ~50 seeds; for each item (Leftovers, Life Orb, Sitrus, Choice items, Focus Sash, Assault Vest, Rocky Helmet, Light Clay, Terrain Extender, weather rocks, mega stones, Air Balloon, White Herb ...) a state where it triggers; for each ability a trigger case. Engine unimplemented list from 8 Nov (`poke-engine-eval/out/impl.json`): abilities Stance Change, Cursed Body, Symbiosis, Imposter, Super Luck, Frisk, Cute Charm, Illusion, Electromorphosis, Moody, Harvest, Rivalry, Unnerve, Early Bird; items Light Clay, Terrain Extender, Damp Rock, Scope Lens, Eject Button, Leek, Heat Rock, Smooth Rock, Red Card, Quick Claw, Zoom Lens, Bright Powder, Big Root, King's Rock (V).
4. **Fuzzing (b):** random legal play in Showdown from random teams (teams from catalog builds) with `record` tape; step the engine forced each turn; compare L0+L1; shrink failing trace to the shortest prefix. Weight team generation by opponent usage from `seen_builds.json` and live logs.
5. **Replays (d):** sim archive protocols first (Showdown reproduces own lines: calibrates inference), then phone logs.
6. **Regression packs:** every confirmed bug becomes a state + tape + expected snapshot file.

### Ranking gaps in the diff report
Rank by (frequency of the feature in recorded play) x (severity), where frequency = count over the 548 phone battles and the sim archives (moves, items, abilities, status, weather, hazards, formes) and the existing ladder usage numbers in `impl.json`; severity = {illegal action set > wrong ordering > wrong HP (outside 1 HP) > wrong status/boost > wrong counter > wrong probability within 1%}. Output per gap: minimal state, tape, expected vs actual snapshot diff, usage count, whether it also occurs in Showdown-vs-game (phone) data.

---

## 7. Risks and engine hooks

### Risks
1. Damage-roll collapse (section 4): without exact-roll semantics most turns diverge in HP. The 8 Nov test: 11/220 off by more than 5%, 6 within 5%, 31 within 1 HP; the off-by-more-than-5% cases were (a) Disguise ignored: engine 0 vs Showdown 44/90/60/60 (Mimikyu as defender: Fire Fang, Earthquake, Rock Slide, Surf); (b) Make It Rain: engine 49/62/53 vs Showdown 62/78/66 (about x0.79 - looks like a spread-move modifier being applied in singles, I); (c) Showdown-side harness artefacts that the engine and pcsa agree on: Rock Tomb into Ditto Imposter (Showdown 21 vs 70), Grassy Glide into Ditto (43 vs 133), Hyper Voice into Sneasler (53 vs 94; cause unclear, unexplained), Endeavor (Showdown -9, engine 0, pcsa 2: both engines mis-handle the harness's full-HP setup). The six 1-5% cases were Shadow Claw x2, Flip Turn, Shadow Ball, Aqua Jet, Crabhammer: rounding-chain stage differences (V, `out/damage_diff.json` engine_divergent list).
2. PRNG order: Showdown consumes draws when the outcome is certain (secondary 100%, accuracy 100, Protect counter 1, crit for armor targets), so seed-level replay of a recorded stream into the engine is impractical; use a semantic tape.
3. Stack-name PRNG scripting is brittle; replace with effect/event tags.
4. Showdown version drift: vendor build 0.11.11 (pcsa) is the only one on disk; pin and record the commit in the test report. The `research/.../node_modules/pokemon-showdown` path mentioned in the brief is absent.
5. Counters: weather/terrain/screen/Trick Room turns, sleep time, toxic stage differ in off-by-one conventions between engines; canonicalise to a single convention and test each with explicit multi-turn cases.
6. Replay noise: root rebuild uses top-1 belief for their stats/items; failures from this are not engine bugs (calibrate with Showdown replay first).
7. Mega evolution timing and Champions-specific changes (Reg M-C: mega rules, Omni Ring, `par` 1/8, sleep 2-3 turns, freeze 3-turn, Z-moves absent, HP shown as percentage with colours) must all be in the oracle (they are, via mod) and in the engine.
8. Phone data have no crits and no rolls; do not treat their HP deltas as exact.
9. Machine load: the desktop is for live play; run harness on the second laptop or after hours.

### Minimal engine hooks needed for testability
1. `generate_instructions(state, a1, a2, mode)` where `mode` selects roll semantics: `Exact(roll 0-15)`, `Avg`, `AllRolls` (16 branches) and crit forced or enumerated.
2. `step_forced(state, a1, a2, outcome: Vec<OutcomeTag>)` returning the one resulting state; outcome tags cover: speed tie winner, hit/miss per target and per hit, crit per hit, damage roll index per hit, secondary fired per secondary, status roll (par full stop, thaw, confusion self-hit, attract), sleep length, confusion length, multi-hit count, ability/item proc, Protect success, Quick Claw. Return `OutcomeTagMissing` if the engine needs a draw that the tape lacks.
3. `enumerate(state, a1, a2) -> Vec<(prob, Vec<OutcomeTag>, State)>` without merging, so merged and unmerged distributions can both be compared.
4. State export: `to_snapshot_json()` exactly the canonical fields of section 4, including counters and volatile data; and `from_snapshot_json()` so one can create a state mid-battle (existing `State::from_string`/`to_string` exist in the Python bindings, V poke-engine-py/src/lib.rs:141-145).
5. Legal action export `options(state, side) -> Vec<String>` in Showdown-style ids (`move earthquake`, `switch 3`, `move x mega`).
6. Deterministic RNG seed input for any internal randomness; none if the engine is fully enumerative.
7. Version stamps: engine commit, Showdown commit, format id; written into each diff report.

---

## Open questions / not verified
- Whether `e.protocol` in sim archives is the raw server message stream or poke-env's `_replay_data` after split handling, and whether it contains `|request|` lines (not decompressed; counts and per-file game numbers are inferred from the log row, not from reading the files).
- Exact number of games across the 4,183 archive files (only the v2-gold update 854 row: 1,502 games; others unknown). Whether `opp_team` is always a full set export (assumed from the opponent being a known bot/team).
- Per-step latency of the oracle (derived from search bench ratios, no direct measure). Whether Node build in `.tools/node` is fast enough for 10^6 steps (about 3 hours/core at 100 steps/s).
- Why Hyper Voice into Sneasler (53 vs 94) and Endeavor disagree; I attribute Ditto cases to Imposter's transformation inside the harness, not checked.
- Whether the engine's Make It Rain deficit (about 0.79) is a spread modifier or a self-drop calculation.
- Whether the game's real Champions mechanics differ from Showdown's mod in ways the phone data would show (not analysed).
- Whether `poke-engine` fork will keep the instruction-list architecture; hooks assume it does (another agent is studying the engine).
- Whether oracle `create` fidelity (0 mismatches on 200 live positions) holds for the recorded sim positions with hidden opponent sets.
