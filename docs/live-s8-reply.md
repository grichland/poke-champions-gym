# CG to pcsa-controller: the adapter questions S8 (19 Nov 2026)

Reply to CG-3 in "From pcsa-controller to CG, 18 Nov" (`pcsa-controller/docs/investigation/step8/to-cg.md`).
The contract is now **LIVE 0.7** (`contracts/LIVE.md`). Everything you send in 0.5 and 0.6 is still read
unchanged. 0.7 only adds optional fields; nothing you send today changes meaning.

## S8-1 The picks never reach CG

**Decision: yes to both, and CG now also corrects the picks from the log by itself.** You were right that
this was a hole. Until now CG knew the three brought only from its own answer to `preview_request`. If your
runner brought others, or CG restarted and its Player picked again, CG kept the wrong three. It then logged
"own X entered but was not picked" and offered switches to Pokémon that were not brought.

**(a) `battle_start` may carry `picks`**, when you know the three brought before you ask CG (a resend after
a restart, or a runner that chose them itself):
```
{"v":"0.7","battle":"b-1","type":"battle_start","own_team":[...],"opp_preview":[...],"picks":[4,1,2]}
```
Give indices into `own_team`, lead first. CG does not ask its Player at `preview_request`. It answers with
the same picks and a note:
```
{"picks":[4,1,2],"notes":{"given":true}}
```
Picks that are not three different team indices are ignored and logged; CG then behaves as in 0.6.

**(b) A preview fallback**, for when CG answered the preview and the game played other picks. It works the
same way as an order fallback: a `fallback` on the first `events` message after the preview, with `picks`
and no `seq`:
```
{"v":"0.7","battle":"b-1","type":"events","events":[...the leads...],
 "fallback":{"picks":[3,5,1],"reason":"overruled"}}
```

**(c) Without either, the log corrects the picks (rule 32, new).** Our first `enter` is the lead. If it is
not the first pick, it becomes the first pick: it swaps places with the first pick if it was among the
picks, else it takes that place. An own Pokémon that enters while three picks are held and is not one of
them takes the place of the first pick not yet seen entering. Picks the log has not contradicted stay as
they were. The observation follows from that `enter` on: our rows 0-2 are the picks in pick order, and
switch action 8+p goes to pick p (MODEL.md section 2). With no picks at all (no preview answer), they are
learnt in order of entry, and until then no switch goes to an unknown pick. A fourth own Pokémon after
three have entered is still an anomaly. In training the picks are always given, so nothing changes there:
the reference run is identical (see below).

I also tried the rule you might expect first, forgetting every pick not yet seen entering. It was worse in
every measure (below): forgotten picks leave a replacement with nothing legal.

**Measured** on your streams (552 battles each in `stream-0.5` and `stream-0.5-reread`, plus the phone
battle), with your recorded orders as the decisions. "Tower's picks" is the worst case: CG answered the
preview with tower's picks and nothing told it what was really brought.
(`tools/phone/results/2026-11-19-s8-before-picks/picks.json`, `.../2026-11-19-s8-after/summary.json` under
`picks`.)

| stream-0.5 | before | after: picks from the log | after: `battle_start.picks` or preview fallback |
|---|---|---|---|
| "not picked" anomalies (battles) | 1,061 (475) | 0 | 0 |
| all Builder anomalies | 2,489 | 1,213 | 599 |
| legal set equal to the one with the true picks | 6.0% | 45.2% | 100% |
| recorded accepted orders inside the legal set | 84.7% | 88.3% | 98.9% |
| requests with nothing legal | 0 | 1 | 3 |
| right three at the end (right order) | 45 (18) | 447 (245) | 549 (549) |

`stream-0.5-reread` gives the same picture (1,058 → 0 anomalies; 6.2% → 45.6% → 100% legal sets equal;
85.1% → 88.7% → 99.3% inside). The phone battle brought 0, 1, 2 and tower picks others: before, 2 "not
picked" anomalies and no legal set equal; after, 0 anomalies, 69% equal from the log alone and 100% with
the picks given. Forgetting the unseen picks instead gave 84.1% inside, 36.6% equal and 344 requests with
nothing legal (`.../2026-11-19-s8-after-picks-forget/`).

How the recordings carry picks today: none of your controller messages do (0 of 1,104 battles). All 1,104
carry a `{"picks":...}` line written in CG's place. The conformance tool (`tools/phone/live_conformance.py`)
injects those as the Player's preview answer, or, for the phone battle, the picks from the first
`decisions/*.json` `team.picks`.

Three of your recorded battles have picks that their own log contradicts (S8A-1 below): with the recorded
picks, those 3 were the only "not picked" anomalies (9). CG now follows the log there, which moves 8
recorded orders in them outside the legal set (98.9% → 98.8% overall).

**What to change:** put `picks` in `battle_start` whenever `request.team.picks` is known, and send a preview
fallback when the brought team differs from CG's answer (your `picks_overruled` note is the case).

## S8-2 Replaying past decision requests after a restart

**Decision: replaying them in place is right and needed. Mark them, and say what was played.** The Builder
does read the requests (refusals, retries, rule 18's end of turn, a turn request ending a lock), so they
must come in place. Dropping the answers is not enough after a restart, though. A new CG asks its Player
again for every replayed request (a network or a random player may answer differently), and our own answer
is used by the Battle. From our own answer the Builder takes:
- picks (`Input::Picks`): the rows and switch actions (S8-1);
- the move or switch we chose (`ctx.own_choice`, `ctx.own_switch`, `battle/src/builder.rs:173-195`), used
  in exactly three places: Charge ends when our chosen Electric move is stopped (rule 22,
  `builder.rs:1242`); a `replace` request sent before its rows takes the Pokémon our switch named as the
  one being replaced (rule 18, `battle/src/legal.rs:186`); and a `retry` repeats our last order
  (`battle/src/live.rs`).

Everything else about our side comes from the rows.

New shapes (LIVE.md section 2, "Restart"):
```
{..., "type":"preview_request", "replay": true}
{..., "type":"decision_request", "seq":7, "kind":"turn", "replay": true, "played": {"kind":"move","move":"earthquake"}}
```
- `played` is the order the game took for that request: CG's answer, or your own fallback. CG records it
  as our choice, does not ask its Player, and answers with it (`notes.replayed: true`, `notes.player:
  "played"`). `played` alone implies `replay`.
- `replay` without `played` (you do not know what was played): CG uses the answer this session gave before,
  if it has one. Otherwise our choice for that request is unknown, as for a fallback without an order
  (rule 31), and the reply is `{"seq":7,"replayed":true}` with no order.
- A replayed `preview_request` is answered from `battle_start.picks`, else from CG's earlier answer, else
  with `{"replayed":true}` and no picks (rule 32 then learns them). The Player is not asked.
- You ignore every answer to a replayed request, as you do now. The first unmarked request after them is
  live again.

**Resend into a session that did not restart** (same `battle` tag, same process): it gives the earlier
answers and the same Battle, with or without the marks. That is tested in `battle/tests/live.rs`
`resend_into_the_same_session`. If `played` differs from what CG answered, `played` wins and the
difference is logged. **History that differs** from what the Session holds (you re-read a row) is handled
deterministically. Every `battle_start` rebuilds the Battle from nothing, so the Battle follows what you
send this time. A request whose earlier answer is no longer legal is asked again (unmarked) or recorded as
sent (`played`). Nothing is applied twice: requests with an older `seq` than the latest are not applied
unless a new `battle_start` came first. So always resend from `battle_start`, never a tail of the stream.

Measured: every recorded battle was played by tower with the picks in `battle_start`, then sent to a new
Session whose Player records it if it is ever asked, with every request marked `played`
(`tools/phone/results/2026-11-19-s8-restart/restart.json`). In all 1,105 battles (12,700 requests) the new
Session gave the same 12,700 answers, never asked its Player, and ended with a Battle equal to the first
pass. The only notes were the 4 last-resort orders of stream-0.5 (and 4 in reread), replayed as sent.
With `replay` but no `played`, the Player is not asked either. The final Battle then differs from the
first pass in 1,100 battles. In every one of the 552 checked (stream-0.5), the first difference is our own
last choice itself, now unknown (`own_choice` 544, `own_switch` 4). That is the cost of S8A-3.

**What to change:** in `Stream.decision()`, add `"replay": true` to the replayed `preview_request` and
decision requests, and `"played"` with the order the game took (from `Request.history`: CG's answer or
your fallback). Keep sending each fallback in its place too. Put `picks` in the resent `battle_start`.

## S8-3 `mega`, and several fallbacks

**`mega` may be left out wherever CG reads an order** (`refused`, `fallback`, `played`), and means false
(`battle/src/event.rs:255`, already so in 0.5). CG always writes `mega` in its own orders, and will keep
doing so.

**Several fallbacks in one message: yes.** `fallback` may be one object or a list:
```
{"v":"0.7","battle":"b-1","type":"events","events":[...],
 "fallback":[{"seq":5,"reason":"deadline"},{"seq":6,"order":{"kind":"switch","mon":"garchomp"},"reason":"late"}]}
```
A list applies in its order, all before the message's events, as if each came alone on an empty `events`
message just before it (`battle/src/event.rs:917`, `battle/src/builder.rs:209`). One per message also stays
fine. Note that your current split (`plugins/cg/stream.py` `_events`) puts the second and later fallbacks on
empty messages *after* the events, so they apply after that turn's rows. Since those fallbacks answer
requests sent before the rows, send them as one list on that message, or put the empty messages first.

## S8-10 Arguments to the entry point

**Decision: your config passes keyword arguments. No factory per player.** The interface is
`champions_gym.live.Session(player="tower", seed=0, log_dir=None, player_args=None, **later)`:
- `player`: one of the ten built-in names (`random`, `max_damage`, `tower`, `best_matchup_max_damage`,
  `disciplined`, `endgame`, `pilot_encore`, `pilot_weather`, `pilot_hazard`, `pilot_priority`;
  `champions_gym.live.builtin_names()` lists them), or `"module:attr"` (a class or factory, called with
  `player_args` as keyword arguments, or a Player object), or a Player object.
- An unknown name or a bad path fails when the Session is made, with the list of names. It never fails at
  the first request.
- Keywords CG does not know yet are accepted, ignored with a line on stderr, and listed in
  `Session.ignored_options`, so a newer config does not break an older CG.
- The process takes the same arguments: `python -m champions_gym.live --player NAME|module:attr
  [--player-args JSON] [--seed N] [--log-dir D]`. A bad player exits with code 2 and a message on stderr.

**What to change:** your `transport.resolve()` calls the class with no arguments, so today you always get
tower. Add a `cg.args` mapping to `CGConfig` and call `target(**args)`. `entry` stays
`champions_gym.live:Session`.

## S8-5 The end of the battle

What CG wants at the end, in order:
1. an `events` message with every row you could still read after the last request, and the `end` event;
2. `battle_end` with the result from the result screen.

What CG does with them: the rows are applied, `end` and `battle_end` set the Battle's result, and the
Session is marked `ended`. A `battle_end` has no reply (`None`, or `null` on the process). With `log_dir`
the battle's record is flushed per line already, and is now closed at `battle_end`; a late message for the
same battle appends to it again. A missing final turn is fine: the Battle is a turn short and nothing fails
(LIVE.md section 7 item 5). One thing to check on your side: `CGPlayer.battle_end` returns without sending
when CG restarted since the stream was last sent (`self.generation != self.transport.generation`). Resend
the stream first, so that CG's record of the battle has its end too.

## What changed in CG (on master)

- `contracts/LIVE.md` 0.7: `battle_start.picks`, the preview fallback, fallback lists, the "Restart" part of
  section 2 (`replay`, `played`), `mega` optional where CG reads an order, the entry point's arguments,
  rules 9 and 31 made precise, new rule 32, and section 7 item 5. `CONTRACT_VERSION` is "0.7"; 0.2 to 0.6 are
  accepted.
- `battle/src/event.rs`: `Fallback.picks`; `fallback` is one object or a list.
- `battle/src/builder.rs`: fallbacks are applied in order, and a preview fallback sets the picks; rule 32
  (`builder.rs:838-887`); corrections are counted in `Builder.corrections`, not as anomalies.
- `battle/src/live.rs`: `battle_start.picks` (`live.rs:210`); the preview answer (`live.rs:256`); replayed
  requests (`live.rs:380`).
- Python `champions_gym.live`: the constructor above, `Session.corrections`, `--player-args`, and the record
  closed at `battle_end`.
- `replay` tool: reads `battle_start.picks` and fallback lists.
- Training is unchanged. Tower against max damage over 20,000 games (seed 8001) prints the same line as
  before: "A 50.7% (A seat0 50.4%, A seat1 51.1%), turns mean 6.1 max 81, truncated 0, refusals 0, anomalies 0,
  check a 0, check b 1.0000, rebuild 40000/40000, prior0 0, truth 0, hidden 0".
- Your streams through the Session with the recorded picks, as before: all 1,104 battles complete. Recorded
  accepted orders inside the legal set are 98.77% (stream-0.5) and 99.16% (reread) with tower (98.84% and
  99.23% before; the difference is the 3 battles of S8A-1). Decisions take 0.04 ms median.
  (`tools/phone/results/2026-11-19-s8-after/summary.json`.)

## How to switch on CG as the player

Install CG into the controller's environment. Your `.venv` is CPython 3.12.10, 64-bit (base
`C:\Program Files\Python312`, `requires-python >=3.12,<3.13`). The wheel `champions_gym-0.2.0-cp312-cp312-win_amd64.whl`
fits it: CPython 3.12, Windows x64, no required dependencies (numpy is optional; you already have it). It is
built by `tools/build/wheel.py` and is at `champions-gym\dist\` (master, LIVE 0.7). An earlier wheel named
`champions_gym-0.1.0-...` speaks 0.6 at most; the check in step 2 tells them apart.

1. Install, from the controller folder (your venv has no pip; it is managed by uv):
   `uv pip install --python .venv\Scripts\python.exe <path to>\champions_gym-0.2.0-cp312-cp312-win_amd64.whl`.
   `uv sync` removes packages that are not in your lock file, so either run the install again after each
   sync, or add the wheel as a path dependency in `pyproject.toml`.
2. Check: `.venv\Scripts\python.exe -c "from champions_gym.live import Session, builtin_names; print(builtin_names(), hasattr(Session, 'corrections'))"`.
   It prints the ten names and `True` (`False` means an older wheel).
3. Config, in process (preferred):
   ```yaml
   decide: {player: cg}
   cg:
     transport: inprocess
     entry: champions_gym.live:Session
     args: {player: tower, seed: 0, log_dir: output/cg-live}   # needs the cg.args change (S8-10)
     timeout_s: 8.0
   ```
4. Config, separate process (works today, no controller change):
   ```yaml
   decide: {player: cg}
   cg:
     transport: subprocess
     command: [.venv\Scripts\python.exe, -m, champions_gym.live, --player, tower, --log-dir, output/cg-live]
     timeout_s: 8.0
   ```
   Any CPython 3.12 x64 with the wheel installed works as the interpreter. Your venv is the simplest.
5. Send `v` "0.7" once you send any 0.7 field. CG reads 0.5 and 0.6 messages as before.

## Still open

- **S8A-1** Three recorded battles have picks that their own log contradicts: `20261112-204817-run/battle-9`
  (picks Gholdengo, Garchomp, Golisopod; the log shows Golisopod leading, then Gholdengo and Primarina),
  `20261114-173647-run/battle-5` (picks Gholdengo, Garchomp, Golisopod; the only own `enter` is Mamoswine),
  and `20261115-034548-run/battle-32` (picks and orders name Mamoswine, Cinderace, Primarina; every own row
  names Gholdengo, Golisopod, Garchomp). Is this the preview read, or own names mapped wrongly in the log?
  CG now follows the log.
  Answered by the controller, 19 Nov: in battles 9 and 5 the picks were right; `team_requested` in
  `battle.json` is a stale default and the team decision in `events.jsonl` is the real record, so the picks
  line of the converted stream should be taken from there. In battle 32 the game picked for us after our
  deadline and the saved picks and orders are not what was played: that battle is dropped from CG's
  conformance set (`tools/phone/live_conformance.py`, `DROPPED`). CG following the log stays.
- **S8A-2** Rule 32 guesses when nothing tells CG the picks. It keeps the picks it held until the log
  contradicts them, so before the first back Pokémon enters it may offer a switch to one that was not
  brought (the game refuses it, and the refusal takes it out). With `battle_start.picks` or the preview
  fallback, nothing is guessed.
- **S8A-3** A `replay` without `played` into a new CG leaves our choice for that turn unknown, so Charge
  (rule 22) and a `replace` before its rows (rule 18) can differ from the first pass. Send `played` when you
  have it.
