# Test rig contract

The rig checks the engine against Pokémon Showdown (mod `champions`, format `gen9championsbssregmc`, commit
`c0845f33d`). Two programs speak the same JSON-lines protocol on stdin and stdout, one request line in, one
reply line out:

- `tools/showdown-oracle/oracle.js` (Node): Showdown in-process. The reference.
- `engine` binary `rig` (Rust): our engine.

A driver (`tools/rig/`) sends both the same battle and compares what comes back. This file is the contract
between the three. Development only: nothing the simulator ships depends on it (PROJECT.md, G3).

## 1. Conventions

- Sides are `p1` and `p2`.
- A side brings 3 of its 6. **Slot** = position in the brought order, 0, 1, 2; slot 0 is the lead. A slot
  never changes during the battle, whatever is active. (Showdown reorders its list on every switch; the
  oracle translates.)
- Ids are Showdown ids: lower case, letters and digits only (`makeitrain`, `garchompmegaz`, `focussash`).
- All HP and stats are integers. Nothing is rounded for comparison.
- A duration is the number of that side's future end-of-turn steps the effect will still be present for,
  counted after the current turn's end-of-turn step has run. 0 never appears: an effect with nothing left is
  absent. Both programs convert their own counters to this.

## 2. Requests

Every request has `op` and `id` (battle id chosen by the driver). Every reply has `ok`. On failure:
`{"ok": false, "error": "..."}`; the battle is left unchanged.

### `new`
```
{"op":"new","id":"b1","seed":[1,2,3,4],
 "teams":{"p1":[set x6],"p2":[set x6]},
 "picks":{"p1":[4,0,2],"p2":[1,5,3]}}
```
`set` is Showdown's set object: `{species, item, ability, moves:[4 names], nature, evs:{hp,atk,def,spa,spd,spe},
level:50, gender}`. `evs` holds Champions stat points. `picks` are indices into the six, lead first. Reply:
`{"ok":true,"state":S,"options":O}` with the battle at the first decision of turn 1 (after the leads'
entry effects).

### `step`
```
{"op":"step","id":"b1","choices":{"p1":"move 1","p2":"switch 2"},"force":[event,...]}
```
`choices`: `move N` (N = 0-3, position in the set's move list), `move N mega`, `switch S` (S = slot), or
omitted for a side that has no decision (`options` says who decides). `force` is optional: without it the
program draws from its own seeded generator; with it every random event must come from the list, in order
per event key (section 5), and the program fails with `error: "tape"` if it needs an event that is not
there or is left with one it did not use. Reply: `{"ok":true,"events":[...],"state":S,"options":O,"log":[...]}`.
`events` are the random events that happened (section 5). `log` is free text for people (Showdown's protocol
lines; the engine may leave it empty).

A step runs until some side has a decision: a normal turn, or a replacement after a faint or a pivot move.

### `release`
Drops the battle.

## 3. Options (`O`)

```
{"p1":{"kind":"turn","moves":[0,1,3],"mega":true,"switches":[1,2]},
 "p2":{"kind":"wait"}}
```
`kind`: `turn` (normal decision), `replace` (must switch: `switches` only), `wait` (no decision), `done`.
`moves`: positions that may be chosen now (not disabled, has PP). `mega`: every listed move may also be
chosen with Mega Evolution. `switches`: slots that may come in; empty when trapped. Struggle: `moves` is
`["struggle"]`.

## 4. Snapshot (`S`)

```
{"turn": 3, "ended": false, "winner": null,
 "weather": {"id":"sandstorm","left":3},           // id "" when none; left null when permanent
 "terrain": {"id":"","left":0},
 "rooms": {"trickroom":2},                        // also gravity, magicroom, wonderroom
 "sides": {"p1": SIDE, "p2": SIDE}}
```
`winner`: `"p1"`, `"p2"`, `"tie"` or null.

SIDE:
```
{"active": 0,                                      // slot of the active Pokémon
 "megaUsed": false,
 "conditions": {"stealthrock":1,"spikes":2,"toxicspikes":1,"stickyweb":1,
                "reflect":4,"lightscreen":2,"auroraveil":1,"tailwind":3,"safeguard":2,"mist":1},
 "wish": {"left":1,"hp":150},                      // absent when none
 "futuresight": {"left":2},                        // absent when none
 "pokemon": [MON, MON, MON]}                       // by slot
```
Hazards give their layer count; timed conditions give their duration; absent conditions are omitted.

MON:
```
{"species":"garchompmegaz", "baseSpecies":"garchomp",
 "types":["dragon"], "ability":"levitate", "baseAbility":"roughskin",
 "item":"garchompitez",                           // "" when none or used up
 "hp":183, "maxhp":183, "fainted": false,
 "stats":{"atk":182,"def":115,"spa":100,"spd":105,"spe":169},   // before stages and modifiers
 "status":"slp",                                  // "", brn, par, slp, frz, psn, tox
 "statusData":{"turns":2},                        // slp: turns of sleep left; tox: stage; frz: attempts left
 "moves":[{"id":"earthquake","pp":10,"maxpp":10,"disabled":false}, ...],
 "boosts":{"atk":0,"def":0,"spa":0,"spd":0,"spe":0,"accuracy":0,"evasion":0},   // active only, else zeros
 "volatiles":{"substitute":{"hp":45},"encore":{"left":2,"move":"swordsdance"},"confusion":{"left":3},
              "leechseed":{}, "taunt":{"left":2}, "choicelock":{"move":"earthquake"}, ...},
 "lastMove":"earthquake",                         // "" when none since entry
 "activeTurns": 2}                                 // 0 for benched
```
Volatiles are keyed by Showdown's condition id. Each program lists every volatile it holds on the active
Pokémon, with `left` where the condition has a duration and the extra fields Showdown's condition state
carries when they affect play (`hp`, `move`, `layers`, `counter`). Bookkeeping that never affects play is
omitted. A program that has no concept of a volatile simply never lists it: the comparer reports that as a
difference, which is the point.

## 5. Events

A random event the battle consumed, in the order consumed:
```
{"key":"accuracy","side":"p1","move":"focusblast","hit":0,"value":false}
```
`side` is the side whose action or Pokémon the event belongs to. `hit` is the hit number of a multi-hit
move, from 0. `value` is the outcome.

| key | value | when |
|---|---|---|
| `speedtie` | side that goes first | two actions tie in order |
| `accuracy` | true = hits | a move with a numeric accuracy is checked |
| `crit` | true = critical hit | each damaging hit |
| `roll` | 0-15; damage factor is `100 - value` percent | each damaging hit that rolls |
| `hits` | number of hits | a move with a variable number of hits |
| `secondary` | true = happens; with `index` (position in the move's secondary list) | each chance effect of a move |
| `fullpara` | true = cannot move | paralysed Pokémon acts |
| `thaw` | true = thaws | frozen Pokémon acts |
| `sleepturns` | number of turns | sleep starts |
| `confusionturns` | number of turns | confusion starts |
| `confusionhit` | true = hits itself | confused Pokémon acts |
| `lockturns` | number of turns | Outrage-style lock starts |
| `trapturns` | number of turns | partial trap starts |
| `protect` | true = succeeds | Protect-family move used in a row |
| `proc` | true = triggers; with `effect` (id of the ability, item or move) | any other chance trigger (Static, Quick Claw, Shed Skin, Effect Spore, ...) |
| `pick` | chosen value; with `effect` | any other random choice (random switch-in for Roar, Dire Claw's status, ...) |

Showdown draws a number even when the outcome is certain (a 100% secondary, accuracy 100, the first Protect).
The oracle reports only events with more than one possible outcome, and when forced it answers the certain
ones itself. An event the oracle cannot classify is reported as
`{"key":"raw","effect":"<current effect id>","kind":"random|chance|sample|shuffle","args":[...],"value":...}`
so nothing is lost; raw events are a to-do list for the classifier, not a failure.

Each event also carries `p`, the probability of its value, when the program knows it. The comparer uses `p`
to check probabilities (step 7's randomness check).

## 6. What the comparer reports

Per step: options equal or not (per side, per field); the snapshot fields that differ, with both values;
events one program reported and the other did not. Per run: the summary line of `tools/rig/driver.py`
(battles and steps identical, failures by kind), and with `--rows` the battle-log row comparison
(`tools/rig/rows.py`, `tools/rig/rows_report.py`).

## 7. Settled after the first build (17 Nov)

Both programs were built to sections 1 to 6. These points were unclear or could not be met as written; this
is what holds now. Where this section and an earlier one disagree, this one wins.

1. `new` accepts `force` and returns `events` and `log`, like `step` (lead entry can draw: speed ties, procs).
2. A tape error carries `detail`. In force mode `move`, `hit`, `index` and `effect` must match as well as the
   key and side.
3. `speedtie` has `side: null` and `value` = the side that goes first. A tie between two effects on one side
   gives `effect` = the id of the one that goes first.
4. A move's own guaranteed stat change that Showdown rolls for is `secondary` with `index: "self"`. A
   confusion self-hit's damage roll is `roll` with `move: "confusion"`.
5. Durations: each program reports its own raw counter. The resolver keeps Showdown's counters; its export
   is in `rig/src/newsim.rs`.
   `sleepturns` and `statusData.turns` follow Showdown's counter (the value minus one is the number of
   turns the Pokémon cannot move).
6. No weather or terrain is `{"id":"","left":0}`: the one place a 0 appears.
7. `rooms` lists every field-wide condition by Showdown id. `futuresight` sits on the target's side and
   carries `move`. Other slot conditions (Healing Wish, Lunar Dance) go into `conditions`.
8. A fainted Pokémon has status `""`. `baseSpecies` is the species it was brought as.
9. `statusData`: `turns` for sleep and freeze, `stage` for bad poison.
10. `disabled` on a move is true for any reason it cannot be chosen (Disable, Taunt, Encore, Choice lock,
    no PP). When a Pokémon is locked into Recharge or Struggle, `options.moves` is `["recharge"]` or
    `["struggle"]` and the choice is `move recharge` / `move struggle`.
11. Revival Blessing: the `replace` options list fainted slots.
12. Who owns a duration event: sleep, confusion and trap turns belong to the affected Pokémon's side; lock
    turns to the user's side.
13. The engine rig also answers `options` (current options only).
14. (Removed: `turn` and `activeTurns` were kept by the rig for the first engine, which did not track them.)

## 8. Settled after the resolver's stage 2 (17 Nov)

1. A tie between two effects on one side: `effect` is `"a+b"` (the two ids in Showdown's list order) and
   `value` is the id of the one that goes first. This replaces 7.3's wording.
2. A tie of three or more is reported as Showdown's raw shuffle draws with absolute indices. CG reproduces
   Showdown's handler list for that event, including statuses of fainted active Pokémon, so the indices mean
   the same thing in both.
3. Confusion `left` is Showdown's counter. `pick` values may be strings (an id). A move the Pokémon is locked
   into that is not one of its four moves is given by id in `options.moves`.
4. Imprison is not expressed: Showdown hides which moves it disables, offers them, and rejects the choice.
   Until the contract has a "choice refused" reply, Imprison stays out of the test pools. (CG needs that
   reply anyway: see PROJECT.md, "A choice the Battle allows but the game refuses".)
5. The snapshot does not hold everything that drives rules or draws (speed as modified, who attacked whom
   this turn, last move's result, stat-change flags, newly-switched flags, Substitute HP at a half point).
   Both programs keep these internally; a difference in them shows up one step later in fields the snapshot
   does hold, or as a tape error. Fields are added to the snapshot when a difference proves hard to trace.

## 9. Choices the game offers and refuses (stage 4, 17 Nov)

Showdown's request hides some restrictions until a player runs into them: a trap by an ability (Shadow Tag,
Arena Trap, Magnet Pull) leaves the switches offered, and Imprison leaves the moves it disables offered. Choosing
one is refused; the request is updated to show it unavailable, and the player chooses again. The game does the
same (PROJECT.md, "A choice the Battle allows but the game refuses"). Both programs express it this way:

1. `options` stays the truth: `moves` and `switches` list what is accepted now. A side with a move decision may
   also carry `"hidden": {"switches": [slots], "moves": [positions]}`: choices the request offers that will be
   refused. It is absent when both lists are empty.
2. A `step` whose choice for a side is in that side's `hidden` replies `{"ok": true, "refused": {"p1":
   "trapped"}, "events": [], "state": S, "options": O, "log": []}` (`"disabled"` for a move). Nothing is
   played, for either side: both choose again. Only what was refused is revealed: a refused switch removes the
   hidden switches, a refused move removes that move from `hidden.moves` (the others stay hidden). A new
   request (the next turn) hides them again.
3. A locked move is never hidden. When every move is disabled, some of them hidden, any move choice is
   Struggle (Showdown's chooseMove): `moves` is `["struggle"]` and `hidden.moves` is empty.
4. A move slot's `disabled` in the snapshot is the truth (7.10), hidden disables included.
5. The driver draws a hidden choice now and then (`--stage4`, `--focus4`); both programs must refuse it with the
   same reason and leave the same state and options.

## 10. Items and Mega Evolution (stage 5, 17 Nov)

1. Item volatiles carry the state that affects play, as Showdown keeps it: `choicelock` its `move`, `metronome`
   its `lastMove` (`""` before the first move) and `numConsecutive`, `gem` its `left` (it ends with the turn);
   `fling` lasts only until the next Update and is never seen at a decision.
2. The `pick` of a forced switch has `effect: "redcard"` when the attacker is the one dragged out (the oracle's
   dragIn site), otherwise the move, or `"dragin"` when no move is running (a Red Card used by Future Sight's
   hit at the end of the turn).
3. The engine rig's `coverage` reply also carries `items`: per item id, the handler runs that changed something.
4. The driver's `--stage5`, `--focus5` and `--real` battles (tools/rig/driver.py) and the scripted battles of
   `tools/rig/scenarios.py` use only the requests above.
