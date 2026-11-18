# Low-latency observation (feature mode "ll")

Module `rl2/features_ll.py`. Every input is a direct read of the poke-env Battle / Pokemon / Move objects and the
request, or a constant-time table lookup on one. It does not call the damage engine, the belief model, the catalog
or seen builds, `form_of` / `current_stats`, and it never re-reads the protocol: a `Tracker` is fed each new protocol
line once. Benchmarks: `rl2/scripts/ll_bench.py`, run on the laptop only; raw results in `runs/low-latency/`.

## Shape

Same shape as rl2.features' `Obs`: rows [12, 268], ids [12, 7], field [55], last [20] + last_ids [4], mask [11].
Row order is unchanged (0 our active, 1-2 our bench in request order, 3-5 empty, 6 their active, 7-11 their others:
seen first, then preview-only). The legal mask and action indexing are `rl.features.action_index` unchanged.

Three switchable groups (`groups=`): `hist`, `lookup`, `dmg`. A group that is off leaves its columns at zero, so
the width is the same in every configuration. Configurations: `raw` = hist only, `raw+lookup`, `raw+lookup+dmg`.

### Row (268 floats)

| Columns | Width | Group | Content |
|---|---|---|---|
| 0-17 | 18 | raw | current types (form table; poke-env's temporary types when a type change is active) |
| 18-23 | 6 | raw | base stats of the current form / 255 (form table) |
| 24-29 | 6 | raw | ours: exact stats from the request / 400 (max HP, atk, def, spa, spd, spe); theirs 0 |
| 30 | 1 | raw | HP fraction (theirs seen only at preview: 1) |
| 31-37 | 7 | raw | status BRN FRZ PAR PSN SLP TOX, status counter / 5 |
| 38-44 | 7 | raw | boosts atk def spa spd spe accuracy evasion (log2 of the multiplier / 2) |
| 45-50 | 6 | raw | present, ours, active, fainted, revealed in battle, not brought (preview-only once 3 are seen) |
| 51-60 | 10 | raw | Encore, Substitute, Taunt, Yawn, Leech Seed, confusion, trapped, partially trapped, must recharge, preparing |
| 61-62 | 2 | raw | first turn in (active only), protect counter / 3 |
| 63-64 | 2 | raw | is a Mega, can still Mega Evolve (stone table; theirs: stone unknown or matching, their Mega unused) |
| 65-68 | 4 | raw | item known, item known to be gone, ability known, known moves / 4 |
| 69-216 | 4 x 37 | raw | per move: 34 static facts + present, PP fraction, disabled |
| 217-220 | 4 | hist | entries / 4, turns moved first / 5, turns moved second / 5, turns since it came in / 10 (active) |
| 221-223 | 3 | lookup | speed against the opposing active: faster for sure, slower for sure, unsure (Trick Room flips) |
| 224-251 | 4 x 7 | lookup | per move: STAB, type effectiveness against each opposing row (ours: their 6; theirs: our 3) |
| 252-267 | 4 x 4 | dmg | per move on the opposing active: min and max fraction of its current HP, KO certain, KO possible |

Move facts (34, cached per move id): type 18, category 3, base power / 150, accuracy, priority / 5, contact, sound,
pivot, recovery (heal or drain), setup, inflicts status, multi-hit, self-drop, first turn only, protect.
Our moves are the request's four (active: in legal-action slot order) with the request's PP and disabled flag;
their moves are the revealed ones only.

Effectiveness code: 0 no move or no target, -1 immune, else (log2(multiplier) + 3) / 5 (0.25x = 0.2 ... 4x = 1.0).
A known ability or item changes it (Levitate, the absorb abilities, Thick Fat, Air Balloon ...): only when the
ability is revealed, the species' only one, or a Mega's fixed one. Freeze-Dry and the type-changing abilities
(Pixilate ..., Liquid Voice) are handled when known.

Speed: ours is the request's speed x Choice Scarf x stage x paralysis x Tailwind x a weather ability; theirs is
the level-50 formula at 0 points / hindering nature and 32 points / boosting nature, the top x1.5 while the item is
unrevealed and x2 in weather while a weather-speed ability is possible.

ids (7): species of the current form, item, ability, move 1-4. Theirs: item and ability only when known.

### Field (55 floats)

turn / 30, force switch, trapped, maybe trapped, can Mega, our Mega used, their Mega used (7) | weather sun rain sand
snow + age (5) | terrain 4 + age (5) | Trick Room + age, Gravity (3) | side conditions 2 x 15 (10 flags with layers,
5 ages) | our alive / 3, their fainted / 3, their revealed / 3 | legal moves / 4, legal switches / 2.

### History

`features_ll.Tracker` (one per battle; kept on the Battle object, or passed in when the Battle is rebuilt per
decision). It holds rl2.turns' turn records (the last-turn token: each side's move id, switch-in species, moved
first, Mega, damage taken, faints, item / ability revealed; same 20 + 4 layout as rl2.features), each member's
current form (Mega), entries, entry turn and moved first / second counts. The pure-raw configuration keeps the
last-turn token and the 4 `hist` columns per row; nothing else is history (the per-side tendencies and the
"biggest hit" evidence of the old field / rows are dropped).

### Closed-form damage (group "dmg")

Level-50 formula on base stats: ours attack with exact stats into their 0-point hindering / 32-point boosting bounds
(defence and HP); theirs attack from the same bounds into our exact stats, the top x1.5 while their item is
unrevealed. Multipliers: STAB (Adaptability, Libero / Protean), type effectiveness as above, stat stages, burn, sun
and rain, screens, Choice Band / Specs, Life Orb, Huge Power, Technician, the tag abilities (Sharpness, Tough Claws,
Iron Fist ...), Knock Off, Grass Knot / Low Kick by weight, Body Press, Psyshock, Seismic Toss / Night Shade,
multi-hit (min and max hits). Not modelled: crits, other variable-power moves, terrain, sand / snow defence boosts,
defensive items and abilities other than the type ones, Focus Sash / Sturdy, accuracy. One numpy pass over 12 rows x
4 moves.

## Benchmark (laptop AWL, single process, 16 Nov)

States (rebuilt Battles, legal actions from `pcsa.players.mirror.legal_actions`):
- live: gold-team decision files from `pcsa/output/runs` (action + forced), rebuilt with
  `pcsa.battle.state.replay(protocol, request)`: 4,926 states in 440 games (4,213 action, 713 forced); 8 more
  files gave no legal action and are left out.
- simulator: `runs/exploiter-1116/archive/u0003*-u0007*.jsonl.gz` sidecars (team export + the seat's protocol, no
  requests), rebuilt decision by decision with `rl.imitate_rebuild` (request synthesised): 6,009 states in 464
  games (886 forced) + 464 team-preview states. 2 games failed in the rebuild ("team already has 3 pokemons": the
  picks were taken from the send-outs), 2 states had no legal action.
- timing set: the first 2,508 live (230 games) + 2,507 simulator (197 games) states = 5,015 decisions, 3 repeats.

Rebuild (not part of observe), ms per state: live mean 0.57 / median 0.37 / p95 0.71; simulator 0.83 / 0.50 / 0.92.

### Time per decision (ms; mean of the 3 repeats over 5,015 decisions)

| Observation | mean | median | p95 | vs base | vs belief+mega |
|---|---|---|---|---|---|
| current, base (`rl2.features.observe`) | 1.233 | 1.261 | 1.539 | 1.0x | 3.8x |
| current, belief+mega | 4.741 | 4.606 | 8.111 | 0.26x | 1.0x |
| ll raw (1-4 + 7) | 0.108 | 0.107 | 0.135 | 11.4x | 43.8x |
| ll raw+lookup | 0.148 | 0.148 | 0.175 | 8.3x | 31.9x |
| ll raw+lookup+dmg | 0.241 | 0.239 | 0.271 | 5.1x | 19.7x |
| ll raw, cold tracker | 0.210 | 0.198 | 0.343 | 5.9x | 22.6x |
| ll raw+lookup+dmg, cold tracker | 0.357 | 0.342 | 0.502 | 3.5x | 13.3x |

Live and simulator states differ by under 10% in every ll and base row (belief+mega: live 5.1, simulator 4.4;
per-kind numbers in `runs/low-latency/time-*.json`). Caches: the process-wide tables (form, move, type chart,
vocabulary; the engine's and the belief's own caches for the current modes) were warmed on the first 3 games;
repeat 1 has cold per-Battle caches, repeats 2-3 warm ones (repeat 1 is 2-5% slower for the current modes, about
the same for ll). The belief object and the ll tracker are new per game in every repeat and fed incrementally;
"cold tracker" builds a new tracker from the whole protocol at every decision (what a Battle rebuilt per decision
costs without a kept tracker).

These are single-process laptop numbers for the observe call only. They are well below the per-decision parts of
the training-lane profile (67 ms total), and rule masks, team-preview inputs, protocol parsing and the network
forward are outside observe and unchanged here. The effect on training decisions/s was not measured.

### Cost per group (ms, mean over 15,045 timed calls)

| Part | ms |
|---|---|
| raw rows + moves + field (12 rows) | 0.074 |
| hist columns | below the timer noise (4 dict reads per row) |
| tracker feed, incremental (new lines only) | 0.025 |
| tracker feed, cold (whole protocol) | 0.124 |
| lookup (effectiveness, STAB, speed order) | 0.039 |
| dmg (after lookup) | 0.089 |
| dmg (with the lookup columns off) | 0.104 |
| legal mask | 0.005 |
| last-turn token | 0.005 |

### Damage estimate against the engine

Engine: `pcsa.battle.damage.damage` with `pcsa.players.threat.fighters` (our exact set, their assumed set, as the KO
check uses), roll midpoint x expected hits, in % of the target's max HP. Estimate: its own (min + max) / 2.

| | live, ours -> theirs | live, theirs -> ours | sim, ours -> theirs | sim, theirs -> ours |
|---|---|---|---|---|
| (move, state) pairs | 14,693 | 2,923 | 6,019 | 2,571 |
| midpoint abs error, mean | 7.8 | 8.6 | 6.3 | 9.2 |
| median / p95 | 4.5 / 26.4 | 5.6 / 26.8 | 3.9 / 19.1 | 5.5 / 29.3 |
| engine value inside [min, max] | 96.4% | 94.4% | 95.8% | 98.3% |
| mean width of [min, max] | 38.5 | 32.4 | 34.2 | 44.3 |
| engine 0, estimate > 0 | 36 | 0 | 84 | 0 |
| estimate 0, engine > 0 | 4 | 0 | 0 | 2 |

KO flags on the decisions where `threat._ko_moves` says a legal move of ours KOs their active (the logic of
`runs/ko-check-1116/ko_check.py`, without its "moves first" filter). Live = all 4,926 live states; simulator = 3,004
states of `u0003*`.

| | live | simulator |
|---|---|---|
| action decisions with both actives up | 4,199 | 2,550 |
| decisions with a legal KO move (engine) | 1,969 | 985 |
| KO moves (engine) | 3,378 | 1,649 |
| KO-possible recall (moves) | 3,375 = 99.9% | 1,635 = 99.2% |
| KO-possible recall (decisions, any KO move flagged) | 1,969 = 100% | 978 = 99.3% |
| KO-certain recall (moves) | 1,890 = 56.0% | 887 = 53.8% |
| KO-certain recall (decisions) | 1,051 = 53.4% | 509 = 51.7% |
| legal damaging moves that do not KO (engine) | 10,288 | 4,729 |
| of those flagged KO-possible (false positives) | 1,664 = 16.2% | 559 = 11.8% |
| of those flagged KO-certain (false positives) | 5 = 0.05% | 28 = 0.6% |
| for reference, not a column: midpoint >= current HP | recall 91.5%, false positives 3.6% | 91.3%, 2.1% |

Where the two disagree on "no damage at all", the engine uses something the battle has not shown, or a stale read:
- the assumed set's ability on an unrevealed ability (Water Absorb Vaporeon, Dry Skin Heliolisk, Flash Fire
  Ceruledge ...): the estimate does not guess, by design.
- Mega Garchomp Z: the pinned species table says Dragon only with Levitate; the engine's Fighter keeps poke-env's
  base-form Dragon / Ground types (and, in a plain poke-env Battle, the ability Sand Force), so it reports Electric
  moves as immune and Ice moves as 4x. The 3 live KO moves the estimate misses are all Ice Shard into a full-HP
  Garchomp; the one inspected was a Mega Garchomp Z.
- a used-up or knocked-off item: poke-env sets the item to None, `pcsa.players.opponent.revealed_item` reads None
  as "unknown", and the assumed set's item comes back (a popped Air Balloon still blocks Ground moves in the
  engine: 36 of 42 such states in one simulator sample). The estimate reads None as "holds nothing".

The last two look like engine-side faults in pcsa; nothing in pcsa was changed.

### Parity (one code path)

Every state ran through `features_ll.observe` in all three configurations (shapes, finite values, |x| <= 2, a
non-empty mask): live 4,926 of 4,926 ok, simulator 6,009 of 6,009 ok, 0 observe errors;
`preview_observations` 464 of 464 simulator preview states ok.

Incremental tracker against a tracker rebuilt from the whole protocol: identical on 6,009 of 6,009 simulator
states and on 4,906 of 4,926 live states. The 20 live differences are decisions where the phone protocol was
rewritten before its last line (the tracker only checks the last line it consumed; without that check it was 259).
So the live player builds a fresh tracker each decision (+0.1 ms); training lanes keep one per battle.

### Not checked

- No model was trained on these inputs; nothing says how much strength the dropped inputs cost.
- Training throughput (decisions/s in a lane) with the new observation.
- `preview_observations` timing, and team-preview states from live games (only simulator previews ran).
- The estimate against real damage rolls in the protocols (only against the engine).
- Whether the pinned species table or the engine is right about Mega Garchomp Z's typing.
- Live against simulator on the same situation, value by value: both kinds go through the same function; no pair
  of states of one situation was compared.
- Tests: `rl2/tests/test_features_ll.py` (6 tests: tables, tracker) pass on the laptop; the rest of the suite was
  not run.

## What else must change to train with it

Model input width and ids
- `rl2/model.py`: `pokemon_net` input `fx.WIDTH + emb` -> `ll.WIDTH` (268) + 192 (species 32, item 16, ability 16,
  4 moves x 32; the 3 possible-ability embeddings `ids[..., 7:10]` go); `field_net` 126 -> `ll.FIELD` (55);
  `HP_COL` 34 -> `ll.HP_COL` (30); `COULD_MEGA_COL` -> `ll.COULD_MEGA_COL` (64); ids [12, 7]. Behind a `features`
  field on `Config` so old checkpoints still load.
- `rl2/belief_features.py` (`MODES`): add "ll".
- `rl2/player.py` (`_observe`, `_preview_observations`, the game state): dispatch on "ll"; one `Tracker` per battle
  in the game state (a mirrored p2 seat rebuilds its Battle every decision); no belief object.
- `rl2/train.py`, `rl2/ppo.py`, `rl2/archive.py`: run config `features: ll` + `ll_groups`; the feature mode written
  into every archive sidecar (archives of different modes do not mix).
- `rl2/search_sim.py`, `rl2/search_player.py`, `rl2/matrix2.py`, `rl2/teacher.py`: call `rl2.features._encode`
  directly; not ported, search stays off in this mode.
- `rl2/scripts/live_finetune.py`, `label_sft.py`, `label_targets.py`, `value_calibration.py`, `validate_ckpt.py`,
  `check_features.py`: the observe call per mode.

Rule masks on the cheap flags (`rl2/train_masks.py`, new `rl2/masks_ll.py`; live: `pcsa/src/pcsa/players/
neural_rules.py`, `no_effect.py`, `sucker_punch.py`, `encore_switch.py`, `mega_weakness.py`,
`src/pcsa/rl_training/rule_masks.py`)

| Rule | Needs | Group |
|---|---|---|
| no_effect | effectiveness -1 against their active; first-turn-only flag + first turn in; status move against a known Good as Gold; a small table for status-move type immunities (Thunder Wave on Ground, powder on Grass) | lookup + raw |
| no_full_heal | recovery flag, HP = 1 | raw |
| useless_status | inflicts-status flag vs their status / Substitute; setup at +6; side conditions at max layers; weather / terrain already up; Taunt on us | raw + field |
| locked_futile | one legal move (disabled flags) and its effectiveness -1 | lookup |
| sucker_punch | move id + the opponent head's kept actions (network output) | none |
| encore_switch | Encore volatile + the tracker's turn records for the pair (who moved first) | hist |
| mega_weakness | the Mega form's types (form table) x revealed opposing move types (type chart) | lookup |
| yawn_switch | Yawn volatile, status, ability, terrain, first turn in; its "a move KOs" exception needs a KO flag | raw + dmg |
| priority_ko | priority > 0, KO flag, speed flags | dmg + lookup |

Without `dmg`: drop priority_ko and yawn_switch's KO exception, or let them read the KO-possible flag only when
`dmg` is on.

Cheap max-damage opponent
- new `rl2/opponents/max_power.py`: the legal move with the highest power x STAB x effectiveness
  (`ll.move_facts`, `ll.chart`), Mega when offered, forced switch to the bench member with the best such score;
  registered in `rl2/opponents/__init__.py`, `rl2/opponents/pilots.py`, the run configs' opponent kinds. Replaces
  `pcsa.players.max_damage` (engine `attack_score`) in the pool.

Distilling the live model (`C:\Users\user\Code\pcsa\assets\models\poke-transformer-league-20261116-1145.pt`)
- `rl2/distill.py`, `rl2/scripts/collect_distill.py`: teacher on its belief+mega inputs, student on ll inputs:
  policy, opponent-head and value targets from the teacher. Two sources of pairs: new teacher rollouts that store
  both observations (`rl2/archive.py`: `rows_ll`, `ids_ll`, `field_ll`), or the existing archives
  (`runs/*/archive/u*.npz` + sidecar protocols) with the ll observation rebuilt offline as `ll_bench.sim_games`
  does (rl.imitate_rebuild); the decision count per episode must be checked against the archive.
- then the usual pipeline on the student: `rl2/scripts/live_finetune.py` (heads), `label_sft.py`,
  `value_calibration.py`.

Live player's mirror in pcsa
- `src/pcsa/rl_training/features_ll.py` (copy; imports `pcsa.rl_training.features`, `turns`, `features_vocab`),
  `former_features.py` (dispatch), `former.py` / `model.py` (widths, ids), `former_player.py` (a fresh `Tracker`
  every decision: the phone protocol is revised between decisions, see the parity result), `rule_masks.py`,
  `players/neural.py`, `players/neural_modes.py`, `config/players/neural.yaml` (a new version entry),
  `value_calibration.py` refit; `rl2/scripts/check_features.py` extended to compare the two copies.
