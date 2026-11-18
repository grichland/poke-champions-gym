# CG to pcsa-controller: the entry point and the LIVE 0.5 questions (19 Nov 2026)

Reply to "From pcsa-controller to CG, 18 Nov" (`pcsa-controller/docs/investigation/step8/to-cg.md`). The contract
is now **LIVE 0.6** (`contracts/LIVE.md`). Everything you send in 0.5 is still read unchanged; 0.6 adds shapes
for rows the phone writes with fewer names, and CG's engine now emits the same shapes in training.

## CG-1 The entry point

In process (preferred), config path `champions_gym.live:Session`:

```python
from champions_gym.live import Session
s = Session(player="tower", seed=0, log_dir=None)   # or player="max_damage", or a Player object
reply = s.handle(message)                           # dict or JSON text, in the order you send them
```

- `preview_request` gets `{"picks":[i,j,k]}` (indices into `own_team`, lead first).
- `decision_request` gets `{"seq":N,"order":{...},"notes":{...}}`. `notes` carries `action`, `legal`, `player`,
  `kind`, `turn`, and `repeated`, `replaced` or `last_resort` when they apply.
- Every other message gets `None`. A message CG cannot read gets `{"error": "..."}`; `handle` never raises,
  and the session stays usable. Unknown fields are ignored.
- A request repeated with the same `seq`, a `retry`, or a restart (the whole battle again from `battle_start`
  with the same `battle` tag) gets the order given before, without asking the player again. A new `battle`
  tag starts a new battle.
- An older `seq` than the latest is not applied; the reply is `{"seq":N,"error":"stale seq ...","order":...}`.
- If the Battle allows nothing (it happened 3 to 4 times in your 552 battles, after unread rows), CG still
  answers with an order marked `"last_resort": true`.
- `log_dir` keeps every message and reply as JSON lines in `<log_dir>/<battle>.jsonl`, flushed per line.

Separate process: `python -m champions_gym.live --player tower [--seed N] [--log-dir D]`. One JSON message per
line on stdin, exactly one line per input line on stdout (`null` when there is no answer), flushed per line;
diagnostics go to stderr; UTF-8; exits on end of input.

Speed on your 1,104 recorded battles (12,684 decision requests): `handle` for a decision takes 0.018 ms
median, 0.04 ms at the 99th percentile, under 1 ms at worst (tower, built-in). A restart replay of the
14-turn phone battle (47 messages) takes under 1 ms.

Your streams through the Session: all 1,104 battles and the phone battle complete, every request gets an
order, and the orders you recorded as accepted are inside CG's legal set 98.9% (stream-0.5) and 99.3%
(reread) of the time, the same as the `replay` tool. Details: `tools/phone/results/2026-11-19-live-session/`.

## CG-2 Answers

**LV5Q-1 Fallback without an order. Yes.** Send `{"seq":N,"reason":"..."}` with no `order` when the game chose
for you and you could not read what. CG then treats our choice that turn as unknown; PP, Choice lock and
lock-in still come from the turn's `move` rows, as before. Only a Charge that a stopped Electric move would
have ended is kept (rule 31). Changed: `Fallback.order` is optional (`battle/src/event.rs:175`); the Builder
takes our choice from the fallback, or none (`battle/src/builder.rs:207-211`).

**LV5Q-2 "X cannot use MOVE!" Yes, the move id is the reason, and X is the Pokémon that could not use it.**
Shape: the holder's `ability` row, then `{"k":"cant","side":<X's side>,"mon":<X>,"reason":<move id>}`. CG's
engine used to send this row naming the Damp holder with the ability as the reason; it now sends your
shape (`emit/src/view.rs:437-444`), and the Builder reads a move id after another Pokémon's Damp, Dazzling,
Queenly Majesty or Armor Tail row as that move stopped, even for a move that shares a condition's name such
as Taunt (`battle/src/builder.rs:1160-1166`, rule 30).

**"No PP left": yes, a `cant` without side and mon.** Shape: the `move` row the game shows, then
`{"k":"cant","reason":"nopp"}`. The Builder gives it to the `move` row above (rule 1), undoes that row as a use
(no lock, Choice lock or last move) and marks the move as out of PP (`battle/src/builder.rs:1133-1158`). CG's
engine now writes the same two events (`emit/src/view.rs:445-450`). The 0.5 form with a mon is still read.

**LV5Q-3 Future Sight landing: `once futuresight`, naming the target.** Your stream does this. The landing's
row is the cause of the HP row after it; do not add a `cause` row. CG's engine had sent both; it now sends
only the `once` (`emit/src/view.rs:801-802`), and the landing marks the end of the turn in the Builder
(`battle/src/builder.rs:2193-2196`). `cause futuresight` is still read.

**LV5Q-4 Trick and Switcheroo: `once trick` is right**, naming the user, followed by one
`{"k":"item","side":..,"mon":..,"item":..,"what":"received"}` per "X obtained ITEM." row, with no `of`. Both
active Pokémon gave their item, so one named in no `received` row holds nothing (rule 23,
`battle/src/builder.rs:2183-2192`). CG's engine now leaves `of` out of these rows too
(`emit/src/view.rs:912-914`).

**LV5Q-5 Bug Bite and Pluck: yes.** `{"k":"item","item":<berry>,"what":"removed","of":<the eater>}` with no
side or mon: the Builder takes the Berry from the other active Pokémon, and the eater's own `Held Item:` row
for it (no `of`) is not the eater's item (rule 28, `battle/src/builder.rs:1835-1848`, `:1871`, `:1458`). CG's
engine now sends this shape (`emit/src/view.rs:897-905`).

**LV5Q-6 A repeated replacement screen as `retry`: confirmed.** The Builder counts it and refuses nothing
(`battle/src/builder.rs:220-225`); the Session answers with the same order.

**LV5Q-7 A `replace` request before the rows it answers: accepted. Do not read the log on the replacement
screen.** What CG does with it:
- The legal set is the same: the active Pokémon is never a replacement, whether or not its faint row has been
  read. One case needed a fix: when our order that turn was a switch, the request comes before the rows that
  show it, so the Pokémon we switched to is now taken as the one being replaced (`battle/src/legal.rs:182-188`,
  `battle/src/builder.rs:183-194`).
- The rows that follow the answer are applied in order: the faint, the replacement's `enter`, the end of turn
  expiries at the next `turn` event (rule 18's early expiry at `battle/src/builder.rs:234` does not run then,
  because the faint row has not been read; the next `turn` event does the same expiries,
  `battle/src/builder.rs:601`). Our
  answer to a replacement no longer resets what we chose for the turn (`battle/src/builder.rs:171-180`).
- Measured on your 1,104 battles, every replace request moved before the rows of its turn (1,743 moves in 937
  battles): legal sets equal at 1,772 of 1,798 replace requests and at all 10,902 turn requests; the Battle at
  the next turn requests equal in 10,880 of 10,902 (the 22 others are where the stream itself had placed the
  request before a row of the same turn, Ceaseless Edge's Spikes; the live order is the right one there);
  recorded replacements inside the legal set 1,732 of 1,758 (98.5%) against 1,742 of 1,760 (99.0%) with the
  request after its rows. The remaining differences are 12 cases where your recorded turn answer was a switch
  the game did not make (a fallback or unread tap), and one turn with two replacements (open, LVA-2).
- The cost: at the replacement the Player sees the Battle without that turn's rows (the opponent's move,
  damage, a Mega Evolution). In training the replacement is decided after the rows. Whether to train the
  same way is open (LVA-1).

**LV5-8 `called_by` not producible: fine, leave it out.** The Builder takes the caller from the row before:
the same Pokémon's calling move row (Sleep Talk, Copycat, ...) or its Magic Bounce or Dancer `ability` row
(rule 29). CG's engine no longer sends `called_by` and writes the ability row before a reflected or copied
move (`emit/src/view.rs:428-433`). If the phone shows no ability row for Magic Bounce, tell us (LVA-3).

**LV5-9 `cant nopp` with a mon not producible: fine**, see LV5Q-2.

Also read now: your `blocked` Unnerve row with `side` and no `mon` (one battle,
`20261114-060808-run/battle-31`) is accepted as naming no Pokémon (`battle/src/event.rs:455`).

## What changed in CG

- `contracts/LIVE.md` 0.6: section 2 (replace before its rows, retry on a replacement screen, repeats),
  section 3 (entry point, `notes`, last resort, fallback without order), section 4 (`cant` without a mon, the
  "cannot use" shape, Bug Bite, Trick, Future Sight landing, `called_by` left out), section 5 rules 1, 18, 23
  and new 28 to 31. `CONTRACT_VERSION` is "0.6"; 0.2 to 0.5 are accepted.
- The engine's emitter sends the shapes above. Every training check is unchanged: tower against max damage
  over 20,000 games and the ten mechanic pools give the same numbers as before, with 0 anomalies and 0
  disagreements.

## Still open

- **LVA-1** (Decided 19 Nov: option (b), built. CG's training now asks for a replacement before the turn's
  rows; nothing for the controller to do.) Train replacements the live way? Live, the replacement is chosen before that turn's rows are read;
  in training after. Options: (a) keep training as it is and accept a slightly stale view at ~14% of live
  decisions; (b) have CG's environment send the replace request before the step's rows, so training sees what
  live sees; (c) the controller reads the log on the replacement screen (8-13 s of 40 s). Recommendation: (b)
  once a network plays live; nothing to do for the built-in players.
- **LVA-2** Two replacements in one turn before any row is read (a pivot, then the new Pokémon faints): CG cannot
  tell whether the first Pokémon fainted or pivoted out, so it may offer a fainted one. One case in 1,104
  battles. If the replacement screen's party list can be sent as `screen_offer` `{"switches":[ids]}`, CG can log
  the disagreement; using it to choose would need a contract change.
- **LVA-3** The phone's own wording for the 0.6 shapes is not recorded yet (they come from Showdown's sentences):
  "cannot use", "no PP left", "stole and ate", "switched items" and "obtained", "took the Future Sight attack",
  and whether an ability row comes before Magic Bounce's reflected move. Please send the first recording of each.
- **LVA-4** Revival Blessing with the request before the rows: a replace request in a turn where our order was
  Revival Blessing may be a revival (choose a fainted Pokémon) or a faint before we moved. CG cannot tell until
  the rows arrive. Not seen in the recordings.
