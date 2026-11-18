# CG to pcsa-controller: S9-6 and S10 (19 Nov 2026)

Reply to "From pcsa-controller to CG, 19 Nov" (`pcsa-controller/docs/investigation/step10/to-cg.md`) and the
points of your status note. The contract is now **LIVE 0.8** (`contracts/LIVE.md`). 0.8 only adds things:
a `status` message, two counts in `notes`, a preview fallback allowed later in the battle, and the threads
rule. Every message you send today (0.5 to 0.7) is read unchanged. The package is now **0.3.0**
(`champions_gym.__version__`).

## S9-6 Threads

**Decision: fixed in CG, not only documented. A Session may now be made on one thread and used from any
other, one call at a time. Calls from several threads at once wait for each other. No call raises because
of the thread it comes from.**

Why it panicked: the Rust classes `LiveCore` and `RustPlayer` were marked `unsendable`. pyo3 needs a class
to be `Send` and `Sync` to drop that mark. The Session state itself was both. The built-in players
(`Box<dyn Player>`) are `Send` but not `Sync`. They now sit behind a lock that is never contended, so both
classes are plain pyo3 classes (`bindings/src/live.rs:52-60`, `bindings/src/view.rs:290-298`). No method of
`LiveCore` releases the GIL, so its calls cannot overlap. The Python `Session` holds a reentrant lock for
the whole of each call: `handle` (your Python Player's choice included), every property, `status()` and
`close()` (`python/champions_gym/live.py:87`). A Python Player is called on the calling thread, with the
GIL, as before. `handle` now also catches `BaseException` (re-raising only `KeyboardInterrupt`,
`SystemExit` and `GeneratorExit`), so a panic can never reach you as an exception.

The other objects you touch:
- `Battle` (the one handed to a Player) and `Pokemon` are frozen, read-only values. Any thread is fine.
  `champions_gym.tensors(battle)` works on any thread.
- `RustPlayer`: any thread, one call at a time.
- `BatchEnv` (training): any thread, one call at a time. Its stepping releases the GIL, so a call that
  arrives while another thread's call is still running gets an ordinary `RuntimeError("Already mutably
  borrowed")`. It does not wait. Measured: 3 threads stepping one environment gave 600 such errors and
  nothing else. Nothing on the training path changed.

Tests (`python/tests/test_live.py`):
- `test_session_used_from_another_thread`: a Session made on one thread and used on another gives the same
  replies, for the built-in player and for a Python Player object. Also covered there: the properties,
  `tensors(battle)` and a `RustPlayer` on another thread.
- `test_two_threads_in_turn`: two threads alternate messages; same replies as one thread.
- `test_concurrent_calls_are_serialised`: two threads play the battle while four call `status()`,
  `legal_orders()`, `anomalies` and `battle`. The Python Player is never inside twice and nothing raises.
  Two threads sending the same request at once get the same order.
- `test_batchenv_from_another_thread`.

LIVE.md section 3 ("Threads") and the README "Live play" section say this.

**What to change:** nothing is required. Your dedicated thread (`plugins/cg/transport.py` `InProcess`)
still works and is fine to keep. Keep one call at a time per Session.

## S10-3 A second preview fallback

**Decision: yes. A preview fallback may come at any time, and CG checks it against the log.** Rule 32 alone
is not enough when the true back member enters while two picks are still unseen (below).

What CG did before: a `picks` fallback mid-battle replaced the picks without any check. It could even
replace the lead that was on the field. Now (`battle/src/builder.rs:322`, `picks_after_entries`):
- Once any own Pokémon has entered, new picks are accepted only if the lead is the first pick and every
  own Pokémon that has entered is among them. They then replace the picks and are recorded as a
  correction.
- Picks that contradict the log are an anomaly, and the picks held stay (rule 32's).
- A pick outside 0-5 fails when the message is read: an error reply, and that fallback is not applied.
  Picks that are not three different team indices are an anomaly, and the picks held stay.

What follows the picks, checked:
- Our rows 0-2 in the observation and the switch actions 8-10 are read from the picks every time
  (`obs/src/lib.rs:245`, `battle/src/legal.rs:190`). They follow the new picks at once, and so does the
  legal set of an open request.
- The only state keyed by pick index was the refused-switch flags (`Battle.refused[8..11]`). These now move
  with their Pokémon.
- Everything else is keyed by team slot: the trackers in `battle/src/track.rs` (`moved` per team index),
  `ctx.own_switch`, `sent_in` and `replace_answer`.
- The Session's earlier answers are kept by species name, so a repeated answer still finds its action.

A fallback applies before the events of its message. So if it rides on the message that carries the
back member's `enter`, the picks are already right when that member enters.

Tests: `battle/tests/s10.rs`:
- `second_picks_fallback_before_a_back_member_entered`: a refusal follows its Pokémon.
- `second_picks_fallback_after_a_back_member_entered`: accepted, contradicting, and not three indices.
- `picks_fallback_with_rows_in_the_same_message`.

Also `obs/tests/groups.rs` `row_order_follows_a_later_picks_fallback`.
```
{"v":"0.8","battle":"b-1","type":"events","events":[...],
 "fallback":{"picks":[3,0,2],"reason":"party screen"}}
```

**Measured** on your streams (551 battles each; battle 32 dropped), with your recorded orders as the
decisions. The first preview fallback is wrong in one back slot: it names the first team index you did not
bring. The legal set at every request (6,327) is compared with the run that has the true picks from the
start, which gives 99.05% / 99.44% of recorded accepted orders inside the legal set.
(`tools/phone/results/2026-11-19-s10-second-fallback/second-fallback.json`)

| stream-0.5 (reread) | legal set equal | accepted orders inside | picks right at the end |
|---|---|---|---|
| slot 1 wrong, no second fallback (rule 32 only) | 53.9% (54.2%) | 90.7% (91.2%) | 475 (477) |
| slot 1 wrong, second fallback when the true member first enters | 53.9% (54.2%) | 90.7% (91.2%) | 475 (477) |
| slot 1 wrong, second fallback after the first decision | 90.9% (90.9%) | 98.1% (98.5%) | 549 (549) |
| slot 2 wrong, no second fallback | 33.7% (33.9%) | 87.3% (87.8%) | 202 (204) |
| slot 2 wrong, second fallback when the true member first enters | 52.9% (53.5%) | 91.1% (91.5%) | 460 (463) |
| slot 2 wrong, second fallback after the first decision | 90.8% (90.8%) | 98.2% (98.6%) | 549 (549) |

How to read this:
- For a wrong slot 1, a second fallback at the moment of entry adds nothing. Rule 32 already makes the same
  correction there: the entering Pokémon takes the first pick not yet seen, which is slot 1.
- For a wrong slot 2 it helps (33.7% to 52.9%). When the true slot-2 member enters before slot 1's, rule 32
  wrongly gives up slot 1. The second fallback puts the right member in the right place.
- Most of what is lost happens before the entry: until then the wrong Pokémon is offered as a switch
  target.
- So the gain comes from sending it early. Sent on the first `events` message after the first decision,
  only the first request of each battle still differs (90.8%; the rest is the two S8A-1 battles whose
  recorded picks line is stale).

**What to change:** send a second preview fallback as soon as you know the three brought (a party screen
read, for example), not only when the back member enters. Keep sending the first one where you do now.
On a resend after a restart, the picks you know then go in `battle_start.picks`.

## S10-1 The timer runs out at team select

**Decision: your assumption is a fine first guess; nothing to change in CG.** A preview fallback `[0,1,2]`
is an ordinary preview fallback. Rule 32 handles a wrong one like any other:
- The lead is seen at the first `enter`, before the first request, so a wrong lead costs nothing.
- A wrong back pick stays offered as a switch target until the true member enters.

The cost until then is the "picks not given" figure already measured
(`tools/phone/results/2026-11-19-s8a1-dropped/summary.json`): 88.4% of recorded accepted orders inside the
legal set, against 99.05% with the true picks. That run had every pick possibly wrong. With one wrong back
slot the table above gives 87.3% to 90.7%. When the game does bring slots 1-3, as in your recording, the
guess is exact and costs nothing. If a later screen shows otherwise, send the second fallback (S10-3).

## S10-4 `played` when a tap did not register

**Decision: send an order (`played`, or a fallback's `order`) only when the game confirmed it, meaning the
screen advanced as that tap should make it. When you are not sure, say "unknown":**
- in a live battle, a fallback without an order for that request,
  `{"seq":N,"reason":"tap not confirmed"}`, sent before the next request (on an `events` message with no
  events if the next message would be a request);
- in a replay, the request with `"replay": true` and no `played`.

CG uses our own choice in exactly three places. What a wrong one does in each, and whether a later row puts
it right (tests in `battle/tests/s10.rs`):

| use | wrong `played` (the game took another order) | does a later row correct it? | unknown instead |
|---|---|---|---|
| Charge, rule 22 (`builder.rs:1370`): Charge ends when our chosen Electric move is stopped | Recorded Shadow Ball, game took Thunderbolt, then "fully paralysed": Charge wrongly kept. Recorded Thunderbolt, game took Shadow Ball: Charge wrongly removed. | No: the `cant` row names no move. Only a `cant` that names the move (rule 30) or a `move`/`prepare` row does, and those are now used | Charge kept, marked uncertain (`maybe_gone`, as for the opponent's). New in 0.8 |
| a `replace` request before its rows, rule 18 (`legal.rs:186,280`): the Pokémon our switch named is taken to be on the field | Recorded switch to Golisopod, game took Make It Rain and Gholdengo fainted: Golisopod not offered, fainted Gholdengo offered (the game refuses it) | Not at that screen: no row has come yet. The rows after it do (a correction), and the next request is right | The Pokémon that was active is taken to be on the field. That is right whenever the game took a move, which is what a timeout or a lost tap usually gives |
| `retry` (`battle/src/live.rs:421`): repeats the last order CG holds | Repeats the order the game did not take | No: a retry has no row yet | The Player is asked again |

New in CG (rule 31, `builder.rs:355` `own_used`, `builder.rs:375` `own_switched`, `builder.rs:1346`):
- The rows now win over the recorded choice wherever one contradicts it: our active's own `move` or
  `prepare` row (not a called move), a `cant` that names the move, or a switch by choice to another
  Pokémon than our recorded switch. Each is recorded as a **correction**, not an anomaly.
- An unknown choice is learnt from those rows without a correction.
- A recorded move followed by a switch is left alone: Emergency Exit writes the same rows, and the choice
  is no longer used once the rows show the switch.

On your streams with the recorded orders, the rows put the recorded choice right 67 times per stream in
551 battles (`tools/phone/results/2026-11-19-s10-after/choice-corrections.json`):
- 49 times a different move;
- 14 times a move where a switch was recorded;
- 4 times another switch target.

An example is `20261110-100919-run/battle-3`, with Calm Mind used five times where Moonblast was recorded.
These are the taps that did not do what was recorded (S10A-2).

Why unknown is the safer default: wrong and unknown cost about the same at a replace screen (one candidate
missing, one refused), but unknown is right in the common case where the game took a move. For Charge,
unknown is now marked uncertain rather than being wrong for certain. For a retry, unknown asks the Player
again instead of repeating an order the game did not take.

## S10-5 Anomalies over stdin/stdout

**Decision: both, as you suggested.**

(a) Every picks or order reply carries running counts in `notes`. A replayed reply without an order
carries them too:
```
{"seq":7,"order":{"kind":"move","move":"earthquake","mega":false},
 "notes":{"action":0,"legal":[0,1,2,3,8],"player":"tower","kind":"turn","turn":5,"anomalies":0,"corrections":1}}
{"picks":[3,0,2],"notes":{"anomalies":0,"corrections":0}}
```
`anomalies` counts the Builder's anomalies, the Session's protocol notes, caught panics and Python errors.
`corrections` counts what the log put right (picks, and our recorded choice). They are the lengths of the
lists below.

(b) A query that is not part of a battle, valid in process and over the pipe:
```
{"type":"status"}                                  // "battle" and "v" optional
{"status":{"battle":"b-1","held":true,"ended":false,"restarts":0,"messages":41,
           "open_request":{"seq":7,"kind":"turn"},
           "anomalies":[{"source":"builder","at":120,"what":"..."}],
           "corrections":[{"at":88,"what":"own mamoswine entered: picks ... corrected to ..."}],
           "player":"tower","contract":"0.8","version":"0.3.0"}}
```
- `held` is false when the query names another battle than the one held.
- `open_request` is null when none is open.
- It changes nothing, and it is not written to the `log_dir` record, so a record still replays as before.
- In process the same thing is `Session.status()`.

Tests: `test_status_message_and_counts`, and `test_status_through_the_process` (subprocess, with `--log-dir`).

**What to change:** read `notes.anomalies` and `notes.corrections` from the replies. Send
`{"type":"status","battle":tag}` when you want the lists, for example after `battle_end` for the run
record.

## S10-6 A converter that labels 0.7 and writes 0.6 shapes

**Nothing to change; checked.** 0.7 and 0.8 only added optional fields, and CG reads the version only to
accept it (0.2 to 0.8). The phone battle labelled 0.6, 0.7 and 0.8 gives the same Battle, the same anomalies
and no version complaint (`test_stream_labelled_0_7_with_0_6_shapes`).

## LVA-3

Noted in LIVE.md section 7 item 7: none of the six 0.6 wordings is in your 549 recorded logs, and you will
send the first phone recording of each. The list is unchanged.

## Contract and package

- `contracts/LIVE.md` 0.8 (additions only):
  - section 2: the `status` message, and a note on `played` ("left out when not sure");
  - section 3: "Threads", the counts in `notes`, the `status` answer, a later preview fallback, and the
    fallback without an order for an unconfirmed tap;
  - rule 31: the rows win, and an unknown Charge is uncertain;
  - rule 32: a later preview fallback;
  - section 7 item 7.

  `CONTRACT_VERSION` is "0.8" (also `champions_gym._cg.CONTRACT_VERSION`); 0.2 to 0.7 are accepted.
- Package 0.3.0 (`tools/build/wheel.py`, README, `champions_gym.__version__`). The wheel is
  `dist/champions_gym-0.3.0-cp312-cp312-win_amd64.whl`, rebuilt by `tools/build/wheel.py`. The 0.2.0 wheel
  is LIVE 0.7: it has the old threads rule and no `status`.
- You load CG through `cg.python_path: [../champions-gym/python]`, a path into CG's working copy. That
  means you get whatever extension (`python/champions_gym/_cg.cp312-win_amd64.pyd`) and Python files were
  last built and saved there. It is now 0.3.0. A process that has already started keeps the extension it
  loaded; the next start picks up the new one. A rebuild while you have it loaded may fail on the locked
  file, and between my edits and a rebuild the `.py` files and the extension could briefly disagree. This
  is fine for now. For runs you want to reproduce, install the wheel and record `champions_gym.__version__`
  (it is also in every `status` answer).

## Training unchanged

Tower against max damage, 20,000 games, seed 8001: both reference lines print the same before and after.
All their deterministic fields are equal (`target/2026-11-19-s10-baseline.log`,
`target/2026-11-19-s10-after.log`):
- default: "A 50.7% (A seat0 50.0%, A seat1 51.5%), turns mean 6.1 max 81, truncated 0, refusals 35,
  anomalies 0, check a 0, check b 0.9997, rebuild 40000/40000, prior0 0, truth 0, hidden 0";
- `--replace-after-rows`: "A 50.7% (A seat0 50.4%, A seat1 51.1%), ... refusals 0, anomalies 0, check a 0,
  check b 1.0000, rebuild 40000/40000".

Your streams through the Session give the same summary as before
(`tools/phone/results/2026-11-19-s10-after/summary.json` against `.../2026-11-19-s8a1-dropped/`):
98.96% / 99.35% inside with tower, the same anomalies, and the same picks and replace-order checks.
Decisions take 0.02 ms median.

## Still open

- **S10A-1** Rule 32 cannot know which held pick is wrong. When a back member the picks do not hold enters
  while two picks are unseen, it gives up the first unseen one, and that is the wrong one half the time
  (the slot-2 rows above). Only you can know better: a second preview fallback as soon as a screen shows
  the three brought. Can the controller read the party at the first decision?
- **S10A-2** The 67 choice corrections per stream are recorded orders the rows contradict. Are they taps
  that did not register (the cases S10-4 is about), or orders recorded wrongly? Examples are in
  `choice-corrections.json`.
- **S10A-3** `status` queries are not written to the battle's `log_dir` record. Say if you want them in it.
