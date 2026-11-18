# CG live contract

Version 0.9, draft, 19 Nov 2026. Owned by CG. What a phone controller sends CG during a live battle and what
CG sends back. It is also the definition of what a Battle may contain: in training, CG's engine emits the
same events from its State, limited to this list, so a training Battle and a live Battle are built from the
same input by the same code.

Version 0.2 is aligned with the recorded phone battles (548 battles, 37,420 log rows, 306 message templates:
`docs/investigation/findings/phone-log-inventory.md`, template ids PL-nnn) and the user's review of them.
Version 0.3 adds the controller's review of 0.2 (`pcsa-controller/docs/cg-live-v02-report.md`, ids CT-n).
Version 0.4 adds what replaying the controller's stream of the recorded battles through CG's builder showed
(ids CF-n).
Version 0.5 adds what building the events from CG's own engine showed: fields and values the rows carry
that 0.4 had no place for, and more section-5 rules (ids X-n). Nothing in 0.4 changes meaning.
Version 0.6 answers the controller's questions on 0.5 (ids LV5Q-n, `docs/live-0.5-reply.md`): shapes for rows
the phone writes with fewer names than CG's engine had sent (no PP left, an ability stopping a move, Bug Bite,
Trick, a called move, a Future Sight landing), a `fallback` without an order, a `replace` request before the
rows it answers, and the entry point (section 3). CG's engine now emits these shapes too. Messages of 0.2 to
0.5 are still read unchanged. Later in 0.6 (no message changes): rule 18 covers a second replacement and
Revival Blessing before the rows (LVA-2, LVA-4), and CG's training environment sends replace requests before
the rows of their turn as the phone does (LVA-1).
Version 0.7 answers the controller's questions from building its adapter (ids S8-n, `docs/live-s8-reply.md`):
the three brought in `battle_start.picks` or a preview `fallback`, picks put right from the log (rule 32),
requests replayed after a restart with the order the game took (`replay`, `played`), several fallbacks in
one message, `mega` optional where CG reads an order, and the entry point's arguments (section 3). Nothing
in 0.6 changes meaning; messages of 0.2 to 0.6 are still read.
Version 0.8 answers the controller's questions from running against CG (ids S9-6, S10-n,
`docs/live-s10-reply.md`): the `status` message and the running counts in `notes` (section 3), a preview
fallback later in the battle (section 3, rule 32), the rows putting our recorded choice right and what to
send when a tap is not confirmed (rule 31), and the threads rule (section 3). Only additions; messages of
0.2 to 0.7 are still read.
Version 0.9 adds the model player (section 3): `champions_gym.neural:load` as the `player`, the optional
Player hooks the Session calls (`new_battle`, `battle_end`, `observe`, `status`, `decision_notes`), and two
fields on CG's side: `notes.model` in replies and `player_status` in the `status` answer. Nothing the
controller sends changes; messages of 0.2 to 0.8 are still read.
Changes bump the version; both sides record the version they speak.

## 1. Rules

1. Only what the game shows. An event exists only if the game's battle log (or a screen the controller
   reads) showed it.
2. Unknown is absent. A field the game did not show is left out. It is never sent as a guess, a default or
   zero.
3. One row, one event. A row is sent once: the same row read from two overlapping screen captures is one
   row (CF-13). The controller turns each log row into one event and never merges neighbouring rows
   or fills a field of one row from another. All linking of rows (who a nameless row is about, what caused
   an HP change, who set a weather) is done by CG's Battle builder with the fixed rules of section 5, so it
   is done the same way in training and live.
4. Every row is mapped. A row the controller cannot map is sent as `unread` (section 4) and counted as a
   controller bug; it is never dropped silently.
5. Names, not numbers. Species, moves, items and abilities are Showdown ids as strings (`garchompmegaz`,
   `makeitrain`). Numeric ids are private to each side.
6. Sides are `own` and `opp`. There is no p1 or p2.
7. A Pokémon is named by side and species id as brought (`{"side":"opp","mon":"garchomp"}`). A team cannot
   hold two of a species, and a form change does not change the name it is addressed by. Under Illusion or
   Transform the controller sends the name the row shows; the builder re-addresses it (CT-14). `side` is
   read from the colour of the row's category pill where the row has one (CT-1).
8. HP is sent as the game shows it (whole numbers for own Pokémon, percent for the opponent's). The Battle
   holds percent only, for both sides: CG divides own HP by the max HP of the set in `battle_start`.
9. Events are sent in the order the game showed them, and each battle's events form one growing list. CG can
   rebuild a Battle from the full list of messages at any time (the events with the requests and fallbacks
   in their places, and what we played: rules 10, 18, 31, 32), so a restart needs nothing but the list
   again, sent as section 2 says under "Restart".
10. CG decides what is legal, from the Battle. The controller never filters choices. When the game refuses a
    choice, the controller reports the refusal (section 3) and CG asks its Player again.

## 2. Messages from the controller to CG

All are JSON objects with `v` (contract version) and `battle` (a tag that is stable across a restart).

**`battle_start`**
```
{"v":"0.8","battle":"b-20261117-0042","type":"battle_start",
 "own_team":[SET x6],            // as read from the game, in team-select order
 "opp_preview":["glimmora","garchomp", ...],   // six species ids as shown at preview; null where unread
 "picks":[4,1,2]}                // optional (0.7): the three brought, when already known
```
`picks` are indices into `own_team`, lead first, sent only when the three brought are known before CG is
asked: a resend after a restart, or a runner that chose them itself (S8-1). CG then does not ask its Player at
`preview_request` and answers with these picks. Picks that are not three different indices are ignored and
logged.
`SET`: `{species, item, ability, moves:[4 ids], nature, points:{hp,atk,def,spa,spd,spe},
stats:{hp,atk,def,spa,spd,spe}}`. `stats` are the numbers the game shows. `points` and `nature` are included
only when read from the game; if only `stats` were read, they are omitted (rule 2).

**`preview_request`**: `{..., "type":"preview_request", "deadline_ms": 45000}`. CG answers with picks.

**`events`**: `{..., "type":"events", "events":[EVENT, ...]}`. May be sent any number of times; each call
appends.

**`decision_request`**
```
{..., "type":"decision_request", "seq": 7,
 "kind":"turn",                  // "turn" or "replace" (a forced switch: a faint or a pivot move)
 "deadline_ms": 12000,
 "refused": {"order": ORDER}}    // only after a refusal
```
`seq` rises with every request; an answer to an older `seq` is ignored. A request is sent at its place in
the game: a `turn` request after that turn's `turn` event, a `replace` request after the faint or pivot row
it answers (CF-9, CF-10), or, when the log is not read on the replacement screen, as soon as that screen
shows: the rows of the turn then come after the answer, in their own order (LV5Q-7, rule 18). `refused` is
set only when the game refused the choice; when the controller is repeating a tap that did not register, or
the same replacement screen came back, it sets `"retry": true` instead (CF-8, LV5Q-6). The game shows no
text for a refusal: the controller knows it because the action screen came back in the same turn. CG
answers a repeated request (the same `seq`, a `retry`, or the replay after a restart) with the order it gave
before, without asking its Player again.

**`battle_end`**: `{..., "type":"battle_end", "result":"win"|"loss"|"tie"|"forfeit"|"unknown"}`. Read from
the result or forfeit screen, not from the log. `forfeit` means the opponent forfeited: the screen names no
side and the controller never forfeits (CT-9). Before it, the controller sends an `events` message with
every row it could still read after the last request, and the `end` event; a final turn it could not read
is simply missing (section 7 item 5, S8-5).

**Restart (0.7, S8-2).** When CG restarts (a new process, a new `Session`), the controller sends the battle
again from `battle_start` with the same `battle` tag: the events, fallbacks and requests in their places, as
first sent, with the picks in `battle_start.picks` when known. Requests that were already answered are
marked:
```
{..., "type":"preview_request", "replay": true}
{..., "type":"decision_request", "seq": 7, "kind":"turn", "replay": true, "played": ORDER}
```
`played` is the order the game took for that request (CG's answer, or the controller's own fallback); it is
left out when not known, and also when the controller is not sure its tap registered (rule 31, S10-4). A request with `played` implies `replay`. CG records `played` as our choice and
does not ask its Player; it answers with that order, `"replayed": true` in `notes`. Without `played`, CG uses
the answer it gave before if it has one (a session that did not restart), else our choice for that request
is unknown, as for a fallback without an order (rule 31), and the reply is `{"seq":N,"replayed":true}` with
no order. A replayed `preview_request` may carry `"played":{"picks":[i,j,k]}`, read like
`battle_start.picks`. It is answered from `battle_start.picks` or `played`, else from CG's earlier
answer, else with `{"replayed":true}` and no picks (they then come from the log, rule 32). The controller
ignores every answer to a replayed request. The request after the replayed ones is a live request again.
Unmarked requests are read as in 0.6: a request CG has answered before gets the same answer, otherwise its
Player is asked. Marked or not, nothing is applied twice: `battle_start` rebuilds the Battle from nothing,
and a request with an older `seq` than the latest, without a new `battle_start`, is not applied.

**`status`** (0.8, S10-5): `{"type":"status"}`, optionally with `"battle"` (and `v`). Not part of a battle:
it may be sent at any time, in either transport, changes nothing and is not written to the `log_dir` record.
It is answered as section 3 says.

Optional, never required: `screen_offer` inside a `decision_request`, listing the move rows and switch
targets the screen showed when the controller happened to read them. CG uses it only to log a disagreement
with its own legal set, with one exception (X7): while our Pokémon is transformed, `screen_offer` may be
`{"moves":[ids in screen order], "ability": id}`, and the Battle takes its copied moves and ability from
it, because the log names neither.

## 3. Messages from CG to the controller

The entry point is `champions_gym.live.Session(player="tower", seed=0, log_dir=None)`: `handle(message)`
takes each message of section 2 (a dict or JSON text) in order and returns the answer below, `None` when a
message has none, or `{"error": ...}` for a message it cannot read (it never raises). As a process,
`python -m champions_gym.live --player tower [--seed N] [--log-dir D]` reads one message per line on stdin
and writes exactly one line per input line on stdout (`null` when there is no answer). `player` is a
built-in player's name or an object with `teampreview(battle)` and `choose_move(battle)`.

**Threads (0.8, S9-6).** A Session may be made on one thread and used from any other, one call at a time.
Calls from several threads at once are serialised: each waits for the one under way (a lock held for the
whole call, a Python Player's choice included; the Player is called on the calling thread). No call
raises because of the thread it comes from. The same holds for the objects a live user touches through the
Session: the `Battle` handed to a Player, `champions_gym.tensors(battle)` and `RustPlayer`. `BatchEnv` (the
training environment) may also be used from any thread, one call at a time; a call made while another
thread's call is under way raises an ordinary `RuntimeError` ("Already mutably borrowed"). CG before 0.8
(package 0.2.0) panicked on any call from another thread than the one that made the object.

**Picks** (answer to `preview_request`): `{"picks":[4,1,2]}`: indices into `own_team`, lead first.

**Order** (answer to `decision_request`):
```
{"seq": 7, "order": {"kind":"move","move":"earthquake","mega":false}}
{"seq": 7, "order": {"kind":"switch","mon":"garchomp"}}
```
A move is named by id, a switch target by species id. CG always writes `mega`; where CG reads an order
(`refused`, `fallback`, `played`) `mega` may be left out and means false (S8-3). The controller maps these to a screen row or a party
entry. `notes` (free JSON for the run record and the dashboard) may accompany an order; CG's carry `action`
(0..10), `legal` (the legal actions), `player`, `kind`, `turn`, and `repeated`, `replayed`, `replaced` or
`last_resort` when they apply. When the Battle allows nothing, CG still sends an order, marked
`"last_resort": true`. Picks given in `battle_start` are answered `{"picks":[...],"notes":{"given":true}}`.
Every picks or order reply (a replayed reply without an order too) carries in `notes` the running counts
`anomalies` and `corrections` (0.8, S10-5): the lengths of the lists the `status` answer gives.

**Status** (answer to `status`, 0.8):
```
{"status": {"battle": "b-20261117-0042", "held": true, "ended": false, "restarts": 0, "messages": 41,
            "open_request": {"seq": 7, "kind": "turn"},
            "anomalies": [{"source": "builder", "at": 120, "what": "..."}],
            "corrections": [{"at": 88, "what": "..."}],
            "player": "tower", "player_status": null, "contract": "0.9", "version": "0.4.0"}}
```
`battle` is the battle the Session holds (null before any `battle_start`); `held` is false when the query
named another battle. `open_request` is null when none is open; its `kind` is `preview`, `turn` or
`replace`. `anomalies` (`source`: `builder`, `session`, `internal`, `python`) and `corrections` (picks
put right, rule 32; our recorded choice put right, rule 31) are the same lists as `Session.anomalies` and
`Session.corrections`. `contract` is the version CG speaks, `version` the package version. `player_status`
(0.9) is what the Player reports of itself (its optional `status()`; null for a built-in player); for the
model player see "Model player" below.

If CG cannot answer in time, the controller acts on its own fallback and tells CG what it did with a
`fallback` field on the next `events` message: `{"seq":N,"order":ORDER,"reason":"..."}`. When the game chose
for us and the order could not be read, `order` is left out: `{"seq":N,"reason":"..."}` (LV5Q-1); the
turn's rows still show what was done. When the team brought differs from CG's picks (the runner overruled
them, or chose without CG), the fallback is `{"picks":[i,j,k],"reason":"..."}` with no `seq` (0.7, S8-1),
on the first `events` message after the preview. A later preview fallback may follow at any time in the
battle when the controller learns the three brought better (0.8, S10-3): it replaces the picks if it holds
the lead first and every own Pokémon that has entered so far; otherwise it is an anomaly and the picks held
stay. Send it as soon as it is known (a party screen read), not only when the Pokémon enters (rule 32). `fallback` may be one object or a list (0.7, S8-3); a list
applies in its order, all before the message's events, as if each were sent alone on an empty `events`
message just before it. Each fallback is sent once, on the first `events` message after the request it
answers. When the controller is not sure a tap registered (the screen did not confirm it), it sends a
fallback without an order for that request (`{"seq":N,"reason":"tap not confirmed"}`), before the next
request: as an `events` message with no events if the next message would be a request (rule 31).

The entry point's arguments (0.7, S8-10) are keywords a config can give: `player` (a built-in name: `random`,
`max_damage`, `tower`, `best_matchup_max_damage`, `disciplined`, `endgame`, `pilot_encore`, `pilot_weather`,
`pilot_hazard`, `pilot_priority`; or `"module:attr"`, a class or factory called with `player_args`; or a
Player object), `player_args` (a dict), `seed`, `log_dir`. An unknown player fails when the Session is made.
Keywords CG does not know yet are ignored and listed in `Session.ignored_options`. The process takes the same
as `--player`, `--player-args JSON`, `--seed`, `--log-dir`. The `log_dir` record of a battle is closed at its
`battle_end`.

**Model player (0.9).** A neural network plays through `player: "champions_gym.neural:load"` with
`player_args: {policy: "pkg.mod:Factory", checkpoint: "...", device: "cpu", fallback: "tower", log_dir: "..."}`
(`docs/neural-player.md`); no controller code changes between models. CG's shell around the model applies
the legal mask, falls back to a built-in player when the model fails or is too slow, and keeps a decision
record `<log_dir>/<battle tag>.neural.jsonl` beside the Session's `<battle tag>.jsonl`. Each order reply it
answers carries `notes.model`:
```
"notes": {..., "model": {"fallback": false, "ms": 7.4, "reused": false, "slow": false, "value": 0.21}}
```
`fallback` is false or the reason the fallback answered (the reply is still a legal order); `ms` the model's
time; `reused` the scores of the refused request before were reused; `slow` the call took longer than its
limit. `status.player_status` holds the model's id and hash, the checkpoint, the contract check (`layout_ok`,
`mismatch`, `allow_mismatch`), the fallback player, counts for the battle and in total (`decisions`,
`fallbacks`, `fallback_reasons`, `slow`, `reused`, `observed`, `ms_median`, `ms_max`) and
`handed_to_fallback` (false, or why the rest of the battle is the fallback's).

**Player hooks (0.9).** A Player object may have these; the Session calls each one it has, and a failure in
one is recorded as an anomaly (`python`), never raised:
- `new_battle(tag, resend)`: at a `battle_start` it accepts. `resend` is true when it is the battle held,
  sent again from the start (a restart).
- `battle_end(result)`: at `battle_end`, with its `result`.
- `observe(battle, action, how)`: for each request answered without asking the Player (a replayed request,
  one with `played`, picks given in `battle_start`, or a request repeated from the Session's record after a
  resend): `battle` is the Battle as it was at that request (preview: before any picks), `action` the action
  played (0..10, or 11..70 at preview; None when unknown) and `how` one of `played`, `given`, `repeated`,
  `unknown`. A Player with memory takes the step it would have taken when asked, so its answers after a
  restart are the answers it would have given without one.
- `status()`: a JSON object for `status.player_status`.
- `decision_notes()`: a JSON object for `notes.model` of the reply to the request it was just asked.
A fallback the controller reports, and our recorded choice that the log puts right (rule 31), are not
passed to the Player: they change the Battle, which the next request shows.

## 4. Events

Every event has `k` (kind). Fields in brackets are sent only when the row showed them. `side`, `mon` in
brackets means the row may name no Pokémon; section 5 says how the builder fills it.

### From the controller's own screens (not log rows)

| k | Fields | When |
|---|---|---|
| `turn` | `n` | a new turn began. Sent for every turn of the game, including turns where the game shows no action screen because our Pokémon is locked into a move, charging or recharging: the log screen shows one page per turn headed "Turn N", so when the heading jumps the controller pages back, reads each missed turn and sends `turn` and that turn's rows in order. |
| `gap` | `what` (`turn`, `rows`), [`n`] | rows of the battle were not read: a whole turn (`what: turn`, with its number) or part of a page. CG keeps building and marks the Battle as incomplete from that point. |
| `refused` | `order` | the game refused our choice (mirrors `decision_request.refused`) |
| `end` | `result` | the battle ended (mirrors `battle_end`) |

### From log rows

| k | Fields | Row |
|---|---|---|
| `enter` | `side`, `mon`, [`forced`: true when dragged in] | "Go! X!", "TRAINER sent out X!". Carries no HP. |
| `withdraw` | `side`, `mon`, [`pivot`: true] | A Pokémon was called back. `pivot` is set for the wording the game uses after U-turn, Volt Switch, Flip Turn, Parting Shot and the like. |
| `move` | `side`, `mon`, `move`, [`priority`: signed], [`called_by`: move or ability id] | `Move:` row. `priority` when the row category showed `+N` or `-N Priority`. `called_by` is the move that called this one (Metronome, Sleep Talk, Copycat) or the ability that did (Dancer, Magic Bounce) (X4); the phone does not show it, so it is left out and the builder takes the caller from the row before (rule 29, LV5-8). Still read when sent. |
| `prepare` | `side`, `mon`, [`move`] | the charging turn of a two-turn move |
| `cant` | [`side`, `mon`], `reason` (a status, condition, move or ability id; `recharge`; `nopp`; `other`) | a Pokémon could not move. An ability id when its own ability stopped it (Truant). "X cannot use MOVE!" (another Pokémon's Damp, Dazzling, Queenly Majesty, Armor Tail) names X, the Pokémon that could not use the move, with the move id as `reason`, after the holder's `ability` row (rule 30, LV5Q-2). "But there was no PP left for the move!" names nobody: `reason` `nopp` with no `side` and `mon`, right after the `move` row it is about (rule 1, LV5Q-2, LV5-9). An `ability` event for an ability comes just before a `cant`, `fail` or `blocked` it caused. |
| `protect` | `side`, `mon` | "X protected itself!" |
| `miss` | `side`, `mon` (the Pokémon that avoided the attack) | "X avoided the attack!" |
| `fail` | [`side`, `mon`], [`reason`] | "But it failed!" `reason` only when the row itself says why: `hpfull` ("X's HP is full!"), or the id of the move, ability, item, status or condition named (X1). |
| `result` | `how` (`super`, `extreme`, `resisted`, `mostly_ineffective`, `immune`, `ohko`), [`side`, `mon`] | effectiveness rows. Only the immune row names a Pokémon. |
| `hits` | `n` | "The Pokémon was hit N times!" |
| `hp` | `side`, `mon`, `dir` (`lost`, `restored`), own: `before`, `after` (numbers); opp: `before_pct`, `after_pct` | `HP Lost:` and `HP Restored:` rows. No max HP, no cause. |
| `cause` | `effect` (the mechanic when the wording names one alone: `sandstorm`, `brn`, `psn`, `stealthrock`, `spikes`, `recoil`, `leechseed`, `curse`, `wish`, `healingwish` (Lunar Dance too), `saltcure`, `futuresight`, `pain_split`, a trapping move's id; otherwise the wording: `energy_drained`, `hp_restored`, `crashed`, `was_hurt`, `lost_some_hp`), [`side`, `mon`], [`of`: side and mon] | a row that says why HP changed: weather, a hazard, poison, burn, recoil, a crash, Leech Seed, "had its energy drained", "had its HP restored", "was hurt!", Pain Split |
| `pp` | `side`, `mon`, `move`, `lost` (number) | "X lost N PP from MOVE!" (Spite, Eerie Spell) |
| `faint` | `side`, `mon` | `Fainted:` row |
| `status` | `side`, `mon`, `status` (brn par slp frz psn tox), [`effect`] | a status began. `effect: "slept_and_restored"` for Rest's row. |
| `cure` | `side`, `mon`, `status`, [`item`] | a status ended |
| `volatile` | `side`, `mon`, `effect`, [`detail`: a move id or a number] | a condition began or its shown count changed (Substitute, Encore, Taunt, Leech Seed, confusion, a trapping move, Disable, Smack Down, a Perish count with `detail` N, `endure` for "braced itself", `type_power_rose` with the type as `detail`: Flash Fire is `fire`) (X8) |
| `volatile_end` | `side`, `mon`, `effect` | a condition ended (includes Illusion wearing off) |
| `volatile_hit` | `effect`, [`side`, `mon`] | a condition acted (the Substitute took the hit, "hurt itself in its confusion", "endured the hit"). Effect ids: `substitute`, `confusion_self_hit`, `endured_hit`, `confusion` ("X is confused!" before it acts), `attract`. |
| `boost` | `side`, `mon`, `stats` (a list; one row can name several), `by` (signed) | a stat stage changed. Accuracy and evasion included. |
| `boost_limit` | `side`, `mon`, `stats` (a list), `dir` (`up`, `down`) | "X's STAT won't go any higher / lower!" |
| `boost_reset` | [`side`, `mon`], `what` (`all`, `negative`, `positive`, `copy`, `swap`, `invert`), [`of`], [`stats`] | Haze (no Pokémon named: both sides), Clear Smog, White Herb (`negative`), Psych Up (`copy` with `of`), Heart Swap, Power Swap, Guard Swap (`swap` with `of`, and `stats` for Power Swap and Guard Swap), Topsy-Turvy (`invert`). `positive` is reserved: CG's engine never writes it. Speed Swap, Power Split and Guard Split change stats, not stages, and are `once` rows (X2). |
| `blocked` | `effect` (what did the blocking), [`side`, `mon`], [`what`] | something was prevented: a stat drop, a status or priority move by terrain, Berries by Unnerve. `effect` is `unnerve` (with `what: berries`, no Pokémon), `stat_drop` (an ability stopped a stat drop and the row names no cause), or the id of the ability, item, terrain, side effect or condition that blocked; `what` is the id of the thing blocked when the row names it (X6). |
| `ability` | `side`, `mon`, `ability` | `Ability:` row, or a plain sentence that names an ability (Pressure, Mold Breaker) |
| `ability_change` | `side`, `mon`, `from`, `to` | `Ability Changed: X: OLD → NEW` |
| `item` | [`side`, `mon`], `item`, `what` (`activated`, `used`, `removed`, `received`, `found`), [`of`] | `activated`: a `Held Item:` row, which does not say whether the item is spent. `used`, `removed`, `received`: plain rows that say so (Focus Sash, Air Balloon popped, Knock Off, Red Card). `found`: Frisk and Harvest. A Gem's row names only a move, so it has no side or mon. "X stole and ate its target's BERRY!" (Bug Bite, Pluck) is `removed` with `of` = X and no side or mon; X's `Held Item:` row for that Berry names X only (rule 28, LV5Q-5). Trick's "X obtained ITEM." is `received` on X with no `of` (rule 23, LV5Q-4). |
| `mega_stone` | `side`, `mon`, `stone` | "X's STONE is reacting to TRAINER's Omni Ring!" |
| `mega` | `side`, `mon`, [`form`] (species id of the Mega) | `Mega Evolution:` row. `form` is left out when the species has more than one Mega: the row does not say which. |
| `form` | `side`, `mon`, `form` | any other form change (Disguise busted, Zero to Hero) |
| `transform` | `side`, `mon`, `into` (species id) | "transformed into X" |
| `types` | `side`, `mon`, [`type`], [`lose`], [`add`] | "X transformed into the T type!"; `lose` for a type burned out or used up (Burn Up, Double Shock); `add` for a type added to the others (Forest's Curse, Trick-or-Treat) (X3) |
| `weather` | `weather` (id), `what` (`start`, `end`) | weather began or ended. The row names no setter. |
| `field` | `effect` (terrain or room id), `what` (`start`, `end`) | a field-wide effect |
| `side` | `side`, `effect` (hazard, screen, Tailwind, Safeguard, ...), `what` (`start`, `end`, `acted`), [`mon`] with `acted` | a side effect began, ended, or acted on a Pokémon coming in (Sticky Web) |
| `once` | `effect`, [`side`, `mon`] | a one-off announcement with no other kind (Perish Song's "all Pokémon that heard the song": `perishsong`; Struggle's "no moves left": `no_moves_left`; Chilly Reception's line; Destiny Bond taking the attacker down: `took_attacker_down`, followed by that Pokémon's `hp` row; Skill Swap: `skillswap`, followed by one `ability_change` per Pokémon; Future Sight and Doom Desire: `futuresight_start` naming the user when the move is used, and `futuresight` naming the target when it lands (two different sentences on the phone; the landing's row is the cause of the HP row after it, with no `cause` row, LV5Q-3); Trick and Switcheroo: `trick` naming the user ("switched items with its target!"), followed by a `received` row per Pokémon that got an item (LV5Q-4); `speedswap` and the two Splits; a charge of power) (X9, X10) |
| `revive` | `side`, `mon` | Revival Blessing |
| `unread` | `text`, [`category`] | a row the controller could not map (rule 4) |

## 5. What CG's Battle builder derives

These are done by CG from the event list, by the same code in training and live. The controller does none
of them.

1. **Nameless rows.** `fail`, `hits`, `result`, a nameless `once`, `blocked` or `cant`: the row belongs to the
   most recent `move` event above it (its user, and the Pokémon it was used on). A nameless `cant` `nopp`
   right after that `move` row undoes it as a use (no lock, Choice lock or last move) and leaves the move
   with no PP. A nameless
   `volatile_hit` ("It hurt itself in its confusion!") belongs to the last Pokémon named in a row above
   it, which is the confused one, not the last to move.
2. **Cause of an HP change.** An `hp` event takes the nearest `cause` event beside it in the same step; with
   none, the cause is the most recent `move`.
3. **Who set a weather, terrain or side effect.** The Pokémon of the most recent `ability` or `move` event
   above the row.
4. **A stat at its limit** (`boost_limit`) is tied to the most recent `move` or `ability` above it.
5. **Weather, terrain, room and screen turns.** The Battle counts turns from the `start` row. An `end` row
   ends the effect; a new weather replaces the old one and restarts the count. The game writes no
   "continuing" row and never says how many turns remain.
6. **PP.** A move's use is never shown as a PP change. The Battle counts uses per move from `move` events (two per use while an opposing
   Pressure has been shown), against the move's maximum in the Champions data, and subtracts what a `pp` row says was lost.
7. **Items.** Our own items are known from `battle_start`; the opponent's only from events. `item` with
   `what: activated` is resolved from a table in CG's data: an item that is spent when it acts (a Berry,
   Focus Sash, White Herb) is marked gone; one that is not (Leftovers, Rocky Helmet, an Air Balloon's
   entry line) is marked shown.
8. **Own HP as percent** (rule 8). Entering Pokémon have no HP row: the Battle keeps the percent it last
   held for that Pokémon (100 if never seen).
9. **Switches.** `withdraw` then `enter` on a side is a switch. `withdraw` with `pivot` is a pivot move's switch,
   normally after that side's `move` in the same turn (accepted without one: the Move row can be lost); the `replace` decision for our side comes between the
   two. `enter` with `forced` and no `withdraw` is a drag (Red Card included). Eject Button's `item` row
   followed by an `enter` is a switch with no `withdraw` row.
10. **Locked moves.** Outrage and the like last two or three turns and the game does not say which. The
    lock is assumed to continue until a row shows otherwise (the fatigue confusion row, a different move,
    a switch). For our own side a `decision_request` of kind `turn` also ends it, because the game shows
    no action screen while we are locked.
11. **Silent effects of a known ability** (Regenerator, Natural Cure on switching out) are applied when
    the ability is known: ours always, the opponent's once shown.
12. **Layers and stat kinds.** Spikes and Toxic Spikes layers are counted from repeated `start` rows.
    `boost` covers accuracy and evasion as well as the five battle stats.
13. **Mega form.** When `mega` has no `form`, it is the Mega of the stone in the `mega_stone` event just
    above; for our own side it is also known from the set.
14. **Illusion and Transform.** Rows under a disguise carry the shown name. When Illusion ends the active
    slot is re-addressed to the real Pokémon. A transformed Pokémon keeps its own name.
15. **Preview is a hint.** An opposing Pokémon that enters and matches none of the six preview species
    takes an unseen preview entry, and the Battle records a preview misread (CF-12).
16. **No withdraw row.** Baton Pass, Shed Tail, Emergency Exit and Wimp Out write none: the `enter` that
    follows is the switch (CF-2).
17. **Critical hits.** Not derived. The log has no row for them and CG does not guess them from damage.
18. **End of turn before a replacement.** A `decision_request` of kind `replace` that follows our faint or
    end-of-turn rows means the turn's end has passed: effects that expire at the end of a turn (Endure,
    a Yawn count, Throat Chop, a trap whose user left) are expired before the request is answered. A
    weather, terrain, room or side effect started by a Pokémon that came in after the end of the turn
    counts its turns from the next turn (X14). A `replace` request sent before the rows of its turn
    (LV5Q-7) is answered from what the Battle holds then: the legal set is the same (the active Pokémon is
    never a replacement; when our order that turn was a switch, the Pokémon it named is the one being
    replaced, though no row shows it yet), and the rows that follow are applied as they come: the faint, the expiries at
    the next `turn` event, the replacement's `enter`. The Battle at the next request is the same as with the
    request after the rows; only the Player's view at the replacement lacks that turn's rows. Our answer to
    a replacement does not change what we chose for the turn. A second `replace` request before any row
    (not a `retry`, not refused) replaces the Pokémon our last answer sent in; the one it replaced is offered,
    since it may have pivoted out, and the game refuses it if it fainted (LVA-2). In a turn we chose Revival
    Blessing the request may be the revival's choice or a replacement before we moved: fainted and healthy
    Pokémon are both offered and the game refuses the wrong kind (LVA-4); an answer naming a fainted one
    sends no one in. When no one else can be offered, the Pokémon taken to be on the field is (it was
    dragged out by Red Card, Roar or Dragon Tail in the unread rows). A refusal at a replace request holds
    for that screen (and a `retry` of it); a new replace request is a new question and may offer it again. In training CG asks replacements in
    this order too (`replace_before_rows`, the default): a seat's rows reach its Battle at its next `turn`
    request, and at the end of the game before `battle_end` (CG applies the final turn's rows, which the
    phone usually cannot read, section 7 item 5).
19. **Regenerator inferred.** A Pokémon known to have Regenerator gets a third of its HP back when it
    leaves, and that HP is marked as an estimate until its next HP row. For an opposing Pokémon whose
    ability is not yet shown, the next HP row after it returns corrects the Battle's HP (X11).
20. **A lock ends** also when the locked move misses, is blocked or the user hurts itself in confusion. A
    confusion caused by the other Pokémon's move during a lock turn does not end it.
21. **A partial trap ends** at the end of the turn in which the Pokémon that set it left the field, or at
    the end of the next turn if the trap's damage for this turn had already been dealt.
22. **Charge** ends when its holder next uses an Electric move (not one called by another move), and also
    when that move is stopped before it is used. For the opponent that second case is not shown, so their Charge is then marked uncertain.
23. **Item exchanges.** Trick and Switcheroo write their `once` `trick` row, then a `received` row per
    Pokémon that got an item; both active Pokémon gave theirs, so one named in no `received` row holds
    nothing. A theft (Thief, Covet, Pickpocket, Magician) writes one
    `received` row with `of`: the giver holds nothing. `activated` with `of` is another Pokémon's Berry
    acting on this one (Fling, Bug Bite, Pluck): it is `of`'s item that is gone (X13).
24. **Hidden trapping ability.** A switch the game refused stays unavailable while the same opposing
    Pokémon remains on the field, not only for that one request.
25. **Our own Illusion.** Rows about our disguised Pokémon carry its real name in training and are expected
    to on the phone, which shows our own party truthfully; the opponent's carry the shown name (rule 7) (X12).
26. **Healing Wish and Lunar Dance** write their row when the replacement comes in, even at full HP. Both
    arrive as `healingwish`, so the PP Lunar Dance restores is not credited.
27. **A delayed attack** (Future Sight, Doom Desire) landing is not its user's move that turn: the rows
    after it do not change what that user is charging, locked into or recharging from.
28. **A Berry eaten by another Pokémon** (Bug Bite, Pluck): `item` `removed` with only `of` (the eater)
    takes the Berry from the other active Pokémon; the eater's `activated` or `cure` rows for that Berry
    that follow, until the next `move` or `turn`, are not about the eater's own item.
29. **A called move without `called_by`.** A `move` row right after a calling move's row of the same
    Pokémon (Sleep Talk, Copycat, Metronome, Assist, Me First, Mirror Move, Nature Power) is the called
    move; one right after the same Pokémon's Magic Bounce or Dancer `ability` row is that ability's.
30. **An ability stopping another Pokémon's move.** A `cant` whose `reason` is a move id, right after the
    `ability` row of another Pokémon's Damp, Dazzling, Queenly Majesty or Armor Tail, is that move stopped
    (even where a condition shares the move's name, such as Taunt).
31. **A fallback.** The order in a `fallback` is what we chose that turn; with no order, it is unknown
    (Charge is then not ended by a move we could not use, and is marked uncertain). The turn's rows still
    show the move. A replayed request's `played` is our choice the same way, and a replayed request without
    one leaves it unknown. Our choice is used for three things only: Charge (rule 22), the Pokémon our
    switch named when a `replace` request comes before its rows (rule 18), and the order repeated to a
    `retry`. Everything else about our side comes from the rows. The rows win over the recorded choice
    (0.8, S10-4): our active's own `move` or `prepare` row (not a called move), a `cant` that names the move,
    or a switch by choice (a `withdraw` row, then the `enter`) to another Pokémon than our recorded switch,
    replaces it and is recorded as a correction (not an anomaly); an unknown choice is learnt from those
    rows without a correction. A recorded move followed by a switch is left as it is (Emergency Exit writes
    the same rows; the choice is not used once the rows show the switch). The rows come too late for the
    three uses: a `cant` for a reason other than the move names no move, and a `replace` request before its
    rows and a `retry` have no row yet. So a `played` or fallback order the game did not take is wrong in
    them, while an unknown one is the safer guess: send an order only when the game confirmed it.
32. **Picks from the log.** The three brought are known from `battle_start.picks`, a preview fallback, or
    CG's answer to the preview; with none of these, from what enters (lead first, then in order of first
    entry; until then the others are unknown, and no switch action goes to them). The log corrects held
    picks: when our first `enter` is not the first pick, the lead becomes the first pick (it swaps places
    with it when it was among the picks, else it takes its place); when an own Pokémon enters that full
    picks do not hold, it takes the place of the first pick not yet seen entering. Picks the log has not
    contradicted stay as they were: the rows (picks first, in pick order) and switch actions (switch to pick
    p is row p) follow the corrected picks from that `enter` on. A fourth own Pokémon after three have
    entered is still an anomaly. A preview fallback later in the battle (0.8, S10-3) replaces the picks
    when the log agrees with it (the lead is its first pick, every own Pokémon that entered is among them);
    the rows, the switch actions and an open request's legal set follow it from then on, a refused switch
    stays refused for its Pokémon, and it is recorded as a correction. One that contradicts the log is an
    anomaly and rule 32's picks stay. In training the picks are always given and never corrected.

## 6. What never appears

Because the game's log does not show it, no event carries: a critical hit; the damage roll; a Choice Scarf or any unrevealed
item or ability; the opponent's stats, nature, points or unused moves; PP left or spent by using a move; max HP; how many turns of sleep,
confusion, a trap, a lock, a weather or a screen remain; who attacked on a miss; who set a weather; the
cause of an HP change inside the HP row; the order two simultaneous effects were resolved in beyond the
order of the log.

CG's engine, when it produces events for training, obeys the same list.

## 7. Still to settle (before version 1.0)

1. (Settled: critical hits are not in the Battle. See sections 5 and 6.)
2. Rows never seen in the recordings, whose wording is still unknown: Electric and Misty Terrain, Gravity,
   Magic Room, hazard removal, our own side's screens, Tailwind and Safeguard on our side, items passed by
   Trick or Thief, a freeze, Forewarn. Each needs a recorded example before version 1.0.
3. (Settled: the effect ids are in section 4 under `cause`; also `volatile_hit` `endured_hit` and
   `volatile` `type_power_rose` with the type as `detail`.) A name damaged in reading is sent as the single
   nearest legal id (CT-15); with no unique nearest, the row is `unread`.
4. (Settled: turns come from the "Turn N" heading of each log page.) Still to confirm on a phone: that
   paging back with the arrows works mid-battle and older pages stay readable.
5. (Settled: the `gap` event.) Open: the battle's final turn. The log cannot be opened after the last
   move, so 542 of 548 recorded battles end without their final turn's rows. The Battle is then one turn
   short at `end`; the result still comes from the result screen. CG wants whatever rows were read after
   the last request in an `events` message (with `end`) before `battle_end` (S8-5), and needs nothing more.
6. (Settled, CT-9.) The wording of a tie is unknown.
7. Phone wording not yet recorded for rows the engine produces (ids LG-n): a Berry flung or eaten by Bug
   Bite or Pluck; an item taken by Pickpocket or Magician and whether an `Ability:` row precedes it; where
   White Herb and Air Balloon rows fall relative to Trick's "received" row; whether a disguised own
   Pokémon is named by its real name (rule 25); the 0.6 shapes taken from Showdown's sentences ("cannot
   use", "no PP left", "stole and ate", "switched items", "obtained", "took the Future Sight attack"), and
   whether the Damp holder's `Ability:` row comes first. None of the six 0.6 wordings appears in the
   controller's 549 recorded battle logs (LVA-3, 19 Nov); the controller sends the first phone recording of
   each. CG's emitter uses the order and fields of sections 4 and 5 until a recording says otherwise.
8. Known limits, where the log gives too little and the Battle can differ from the game until a later row
   corrects it: an opposing Illusion (the disguise's name is all that is shown); an unshown Regenerator or
   Natural Cure; an unshown trapping ability until the first refusal; a lock that ends while the Pokémon
   is already confused; Imprison against moves not yet seen.

## 8. Conformance

- CG: every recorded phone battle, converted to events, builds a Battle without error; and a battle played
  by CG's engine, emitted as events and read back, gives the same Battle as the one built alongside it.
- Controller: every one of the 306 recorded templates maps to a kind in section 4; every event it emits
  validates against section 4; nothing in section 6 ever appears; the share of `unread` rows is reported
  per battle.
