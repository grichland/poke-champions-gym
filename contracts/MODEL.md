# CG model contract

Version 0.6, draft, 19 Nov 2026. Owned by CG. What a model receives from CG at each decision and what it
must return. Written for whoever builds the network: it fixes the data, not the architecture.

The same arrays are produced in training (Battles built from the engine's events) and live (Battles built
from the phone's battle log), by the same function, `obs::observe`. A model built against this contract
needs no other code from CG.

## 1. What a decision is

CG asks one seat for one decision at a time. The input describes a **Battle**: what that one player knows.
It is not the engine's full state. Anything the game has not shown that player is encoded as unknown, never
guessed.

A request has a kind:

| Kind | Code | Asked when | Answer |
|---|---|---|---|
| preview | 0 | before the battle, six Pokémon on each side shown | which three to bring, lead first |
| turn | 1 | a normal turn | a move, a move with Mega Evolution, or a switch |
| replace | 2 | our Pokémon fainted or used a pivot move | a switch |

No request is made on a turn where the game gives no choice (locked into a move, charging, recharging).

**What the model sees at a `replace` decision (0.6).** As on the phone, which reads the battle log only on
a turn's action screen, a replace request comes before the rows of the turn it ends: the Battle is the one
of our last turn request plus our answer to it, and holds nothing that happened since (not the opponent's
move, the damage, a faint, a Mega Evolution, a switch, our own move's row for a pivot). So at a replace
decision: `turn` is still that turn's number; the active row (`is_active`) is the Pokémon that was on the
field at the start of the turn, with its HP, status, boosts and volatiles as they were then, whether it
fainted, pivoted or was forced out; the alive counts, the opponent's rows (row 6 is their active of the
turn's start), the field, `opp_prior`, the damage and speed features are those of the turn's start; `last`
and `last_ids` are the turn before (as at the turn request; they meant that in 0.5 too). A second replace
request in the same turn (the Pokémon just sent in fainted or was forced out) shows the same Battle. Only
`mask` tells what changed: the Pokémon being replaced is left out (the active one, or the one our switch
this turn or our last replacement sent in), and every other brought Pokémon not known to have fainted is
offered. A Pokémon that fainted during the unseen rows can therefore be offered; choosing it is refused
and the request comes again without it (`refused`, section 4). In a turn where we chose Revival Blessing
the fainted ones are offered too (the request may be the revival's choice or a plain replacement). The
rows are applied before our next turn request. In training this is `BatchEnv(..., replace_before_rows=True)`,
the default; `False` gives the order of 0.5 (the replace request after the turn's rows).

## 2. Input arrays

Per decision, six arrays. In a batch each gains a leading dimension N.

| Array | Shape | dtype | Contents |
|---|---|---|---|
| `rows` | [12, 300] | float32 | one row per Pokémon: 0–5 ours, 6–11 theirs |
| `row_ids` | [12, 10] | int32 | per row: species, item, ability, 4 moves, 3 possible abilities |
| `field` | [66] | float32 | weather, terrain, rooms, both sides' conditions, tendencies, pending effects |
| `last` | [17] | float32 | what each side did in the last completed turn |
| `last_ids` | [4] | int32 | our move, our switch-in species, their move, their switch-in species |
| `mask` | [11] | uint8 | legal actions for a turn or replace request, 1 = legal |

Plus, with the request: `kind` (uint8, section 1) and `preview_mask` [60] uint8 for a preview request.

One fixed table, not per decision: `learnset` [393, 516] uint8, species id × move id, 1 = can learn. It may
be used as a constant buffer (for example to mask a prediction of the opponent's unseen moves).

All float values are finite and inside the range stated for their feature (mostly 0 to 1; stat stages and a
few others −1 to 1). No further normalisation is needed.

### Row order

- Rows 0–2: the three Pokémon we brought, in pick order (row 0 is our lead). Rows 3–5: the three we left
  behind, in team order. Our active Pokémon is whichever of rows 0–2 has `is_active` (column 141) set. At a
  preview request, before any pick, rows 0–5 are our team in team order.
- Row 6: their active (or the one that just left the field). Then theirs that have been seen, then the
  unseen, each in preview order.
- Which Pokémon came to the battle is also in the row itself: `brought` (column 143; ours: we picked it;
  theirs: it has been seen), `not_brought` (144; theirs only, set on the rest once three have been seen)
  and `was_lead` (145).
- A row with nothing known is all zeros, with ids 0.
- The two seats of one game are exact mirrors: what one sees in rows 0–5 about itself, the other sees in
  rows 6–11 to the extent it has been revealed.

Rows are a set with two marked members (the actives) and a side; position within a side carries no meaning
beyond the order above. The switch actions do not index rows: see section 4.

### Inside a Pokémon row (300 floats)

| Columns | Block | Notes |
|---|---|---|
| 0–23 | identity | types (18, multi-hot), six base stats |
| 24–33 | build | stats where known, Mega and item flags |
| 34–142 | condition | HP fraction, status (one-hot) and its counter, stat stages, 87 volatile bits, turn flags, counters |
| 143–145 | knowledge | what is revealed, not brought |
| 146–149 | matchup | speed and damage relations to the opposing active |
| 150–153 | evidence | observed order of moving and similar |
| 154–297 | moves | 4 move slots × 36 columns: slot s starts at 154 + 36·s |
| 298–299 | timers | turns since Encore and Taunt began |

Each move slot (36 columns) holds the move's facts (type, category, power, accuracy, priority, class
flags), its state (PP fraction, disabled, locked in, revealed) and its estimated damage against the opposing
active. An empty or unknown slot is all zeros with move id 0.

A Pokémon is therefore naturally one token, or one token plus four move tokens: the move columns of a row
and that row's four move ids line up by slot.

### Ids and vocabularies

Every vocabulary reserves 0. Use these sizes for embedding tables.

| Vocabulary | Size | 0 means | Used in |
|---|---|---|---|
| species | 393 | pad / unknown | `row_ids[:, 0]`, `last_ids[1]`, `last_ids[3]` |
| items | 168 | unknown (1 = known to hold nothing) | `row_ids[:, 1]` |
| abilities | 319 | unknown / pad | `row_ids[:, 2]`, `row_ids[:, 7:10]` |
| moves | 516 | empty / none / unknown | `row_ids[:, 3:7]`, `last_ids[0]`, `last_ids[2]` |

`row_ids[:, 7:10]` are the abilities the Pokémon's current form can have (for ours: the known one, then 0,
0). The feature list suggests embedding widths (species and moves 32, items and abilities 16); these are
suggestions only.

The numbering is CG's own and is stable for a given data version (section 6). It is not Showdown's or
poke-env's numbering.

### The exact column list

The authoritative list is the generated layout, not the table above: `champions_gym.FEATURES` in Python
(`obs::layout::FEATURES` in Rust) gives, for every feature id (F1-1, G2-3, ...), its block, offset, size,
encoding and range. It is generated from `obs/features_proposed.csv` (149 rows). Build the model from
`FEATURES` and the array shapes, not from hard-coded column numbers, so that a revision of the feature list
needs no model code change beyond input widths.

## 3. Where the arrays come from

`env.observe_tensors()` gives every game's arrays for the seat that must act, shaped [N, ...], with `kind`,
`seat`, `decision` and `opp_prior` [N, 13]; `env.act(actions)` takes one action index per game (section 4).
`champions_gym.tensors(battle)` gives the same arrays for one Battle. Section 5 has an example. Arrays are plain contiguous
buffers; `torch.from_numpy` works on them without a copy. `BatchEnv(..., threads=K)` steps and observes the
games on K cores; the games played are the same for any K.

For training against or cloning a built-in player: `env.builtin_actions("tower")` gives, per game, the
action (0–70) tower takes at the pending decision, from the same Battle as the observation;
`env.act(actions, opponent="tower")` lets tower answer for every game waiting on the seat that is not the
game's `agent_seat` (set per game in `BatchEnv(..., agent_seat=...)`), so the learner's rows are those
with `seat == agent_seat`. `BatchEnv(..., team_weights=...)` draws teams with given weights per file
index; `env.team_errors` lists teams that could not be read.

## 4. Output: the action space

One fixed space of 71 actions.

| Actions | Meaning | Legal for |
|---|---|---|
| 0–3 | use the move in slot 0–3 of our active Pokémon | turn |
| 4–7 | the same move with Mega Evolution | turn |
| 8–10 | switch to our pick 0, 1 or 2 (in the order brought, lead = pick 0) | turn, replace |
| 11–70 | preview: 11 + lead·10 + pair | preview |

- Actions 0–10 are masked by `mask`; actions 11–70 by `preview_mask`. An action outside the mask is refused.
- Preview: `lead` is the index 0–5 of our team; `pair` numbers the two back picks among the other five
  (10 unordered pairs, in lexicographic order). `champions_gym.preview_picks` and `preview_action` convert.
- When the only possibility is Struggle or a forced move, the one legal action is 0.
- **Policy sizes (user, 18 Nov): 11 actions for us, 13 for the opponent.** Our policy head is over actions
  0–10 with `mask`. Team preview is a separate decision over the 60 preview actions (its own head, or any
  `teampreview` function). The opponent head is over the 13 actions of section 5. CG only needs an action
  index per game: 0–10 for a turn or replace request, 11–70 for a preview request.

Switch actions line up with rows: action 8 + p brings in the Pokémon of row p (MC-1, settled by the user
18 Nov). Move actions refer to the move slots of whichever of rows 0–2 has `is_active` set.

A choice the game refuses despite the mask (a hidden trapping ability, Imprison on an unrevealed move, a
replacement that fainted in rows not yet read) comes back as a new request with that action removed from
the mask. A switch refused for a hidden trapping
ability stays out of the mask while that opposing Pokémon remains on the field.

## 5. Labels: the actions each player took

For every decision CG also gives what both players chose in that game turn. These are **labels for
training, never inputs**: the opponent's choice is not known when the decision is made. They are filled in
once the turn has resolved, so they arrive with the next observation of that game (or with the game's end).

The opponent's choice is expressed as one of **13 actions seen from this player's side**, lined up with the
rows of the observation the label belongs to:

| Opponent action | Meaning |
|---|---|
| 0–3 | their active (row 6) used the move in its move slot 0–3 |
| 4–7 | the same with Mega Evolution |
| 8–12 | they switched to the Pokémon of row 7, 8, 9, 10 or 11 |

Their move slots fill in the order moves are revealed. A move not yet revealed at the decision is labelled
with the slot it takes when it appears: the first empty slot of row 6.

| Array | Shape | dtype | Contents |
|---|---|---|---|
| `own_action` | [] | int16 | the action this seat took: 0–10, or 11–70 at preview |
| `opp_action` | [] | int8 | the opponent's choice as one of the 13 actions above; −1 when it made no choice this step |
| `opp_forced` | [] | uint8 | 1 when the opponent had nothing to choose (locked into a move, charging, recharging); `opp_action` is then the forced move |
| `opp_move` | [] | int32 | move id (vocabulary of 516) of the move chosen or forced; 0 for a switch or no choice |
| `opp_shown` | [] | uint8 | 1 if the game's battle log showed that choice to this player during the turn; 0 if it stayed hidden |
| `opp_prior` | [13] | float32 | how available each opponent action is, from this player's Battle: 0 impossible, 1 certain, between = an estimate (below) |
| `opp_picks` | [3] | int8 | preview only: the three the opponent brought, as indices into its preview order, lead first |

`opp_prior` is computed from the Battle alone, never from the engine's truth, so it exists at inference too
and is an **input**, not a label. It starts as an estimate and becomes certain as the log reveals things:

- **Switches (8–12).** A seen, unfainted bench Pokémon is 1. A fainted one, or one known not brought, is 0.
  An unseen one is (3 − k) / (6 − k), where k is how many of theirs have been seen: 0.4 each after the lead
  alone, 0.25 each after a second, 0 after the third. All switches are 0 while their active is shown trapped.
- **Moves (0–3).** A revealed move is 1, or 0 if the Battle shows it disabled or out of PP. An empty slot is
  1 while fewer than four moves are revealed: an unrevealed move certainly exists. The first empty slot
  stands for "a move not yet seen"; later empty slots are 0.
- **Mega (4–7).** As the plain move, times: 1 if the stone has been shown, 0 if another item has been shown,
  the species has no Mega, or that side has already Mega Evolved, and 0.5 while unknown.

Intended use: add log(`opp_prior`) to the opponent head's logits, so 0 removes an action and a fraction
makes it less likely without forbidding it.

**Truth labels, training only.** What the opponent really has, for a head that learns to predict what the
prior only counts. Indexed by the observation's opponent rows 6–11:

| Array | Shape | dtype | Contents |
|---|---|---|---|
| `opp_true_brought` | [6] | uint8 | 1 for the three the opponent brought |
| `opp_true_moves` | [6, 4] | int32 | each Pokémon's four move ids; revealed moves in their slots, the rest in set order after them |
| `opp_true_item` | [6] | int32 | item id (1 = holds nothing) |
| `opp_true_ability` | [6] | int32 | ability id |

These never exist live and must never be fed back as inputs.

- **When there is no opposing choice.** A `replace` request is answered by one seat alone: `opp_action` is
  −1. At a preview request the opposing label is `opp_picks`. A replace decision's truth labels are indexed
  by its own rows (the Battle of section 1, without the turn's rows); label timing is unchanged in 0.6.
- **A chosen action is not always a performed one.** A player can choose a move and faint, flinch, sleep or
  be dragged out before using it. In training the label is always the choice. `opp_shown` is 0 in those
  cases, because the game's log never revealed it. A switch target the observer had not yet seen still has
  a row (an unseen row becomes seen); action 8–12 names its row in the observation the label belongs to.
- **Use in training and live.** In training every label is exact, from the engine. In recorded phone
  battles only the `opp_shown` = 1 cases can be labelled (from the opponent's `move`, `withdraw` and `enter`
  rows of that turn). To train a head that will later be
  fitted to live games, restrict the loss to `opp_shown` = 1 so both sources mean the same thing.
- **Same turn, both seats.** CG asks seat 0 and then seat 1 within one game turn. Seat 0's `opp_*` labels are
  seat 1's choice of that same turn and the other way round; neither sees the other's choice in its input.
- The game's outcome for value targets is in the environment's results (`done`, `winner`, turns), not here.

**How labels are matched to observations.** Every request has a `decision` number, unique within its game,
returned with the observation. `env.labels()` is called after `env.act(...)` and returns arrays shaped
[N, 2, ...], by game and seat, with `valid` (1 where that seat's decision was completed by this `act`),
`decision` (the number of the observation the label belongs to) and `refused` (1 when the game refused the
action and the seat is being asked again; such a label carries no opposing fields). Both seats choose in the
same game turn, so one `act` can complete both seats' decisions of a game.

**Cases with no opponent action among the 13.** Struggle and a recharge turn have no move slot: `opp_action`
is −1 with `opp_forced` 1 and `opp_move` holding Struggle's id (0 for a recharge). A `replace` decision has
`opp_action` −1 even when both players replace at once.

```python
import champions_gym as cg
env = cg.BatchEnv("teams.json", games=256, seed=1)
obs = env.observe_tensors()        # .rows .row_ids .field .last .last_ids .mask .opp_prior .kind .seat .decision
env.act(actions)
lab = env.labels()                 # [N, 2, ...]; use lab.valid and lab.decision
t = cg.tensors(env.battle(0))      # one Battle -> the same arrays, unbatched
cg.FEATURES["F3-1"].columns(); cg.VOCAB; cg.SHAPES; cg.learnset_table()
```

## 6. What the model must not assume

- No hidden information is present. The opponent's stats, unrevealed moves, item and ability are unknown
  until shown; opposing HP has the resolution of a whole percent; critical hits are not marked.
- Unknown is not zero-valued information: use the knowledge flags (columns 143–145 and the per-move
  `revealed` flag) and id 0 to tell "unknown" from "none".
- Damage and speed features against the opponent use a fixed rule for what is unknown (neutral nature, no
  stat points, no item unless shown). They are estimates, and identical in training and live.
- Rows of Pokémon not brought (ours) or not yet seen (theirs) exist and are mostly empty.
- Live Battles can be incomplete (rows the phone failed to read). Training Battles are not. A model should
  not depend on any single feature being exact.

## 7. Versions

| Item | Value |
|---|---|
| Contract | 0.6 (19 Nov 2026: a `replace` decision before its turn's rows, section 1) |
| Feature list | `obs/features_proposed.csv`, 149 rows, 17 Nov 2026, sha256 `1ac2d02d…acac34` |
| Widths | rows 300, row ids 10, field 66, last 17 + 4, mask 11, preview mask 60 |
| Game data | Showdown `champions` mod at commit c0845f33d, format `gen9championsbssregmc` |
| Live event contract | `contracts/LIVE.md` 0.6 |

A checkpoint should record the feature-list hash and the vocabulary sizes it was trained with; CG exposes
both. A change to the feature list changes widths or meanings and bumps this contract's version.

## 8. Not final

1. The feature list is the user's proposal and still under review: seven features added by the CG review
   (F9-1.1, F9-1.2, G9-1, G9-2.1, G9-2.2, G9-3.1, G9-3.2) and the definitions chosen where a row was not
   exact (ids OB-1 to OB-30, in particular damage bounds OB-6, speed bounds OB-5, move
   classes OB-3, tendency rates OB-13).
2. 26 of the 87 volatile bits and the two Mist columns can never be non-zero in Champions.
3. At a preview request: our six rows in team order with full builds, the opponent's six from the preview
   species only, the field and last-turn blocks empty, `mask` and `opp_prior` all 0 (use `preview_mask`).
4. Reward and episode signals (`done`, `winner`, per-game results) are part of the environment interface,
   not of this contract.
5. Everything in this contract is on master (18 Nov). Other training-only targets (the opponent's hidden set, its unrevealed moves) are not
   defined; CG can supply them from the engine's full state if a model wants them.
