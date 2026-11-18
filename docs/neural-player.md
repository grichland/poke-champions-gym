# Neural player

`champions_gym.neural` puts any model behind CG's Player interface. CG owns the **shell** (everything that is
the same for every model: arrays, masks, choosing, fallback, timing, the decision log, the self-check, the
batched evaluator). The model side supplies a small **policy**: arrays in, scores out, and loading its
checkpoint. CG's package imports and runs without torch or numpy; only the policy needs them.

Files: `python/champions_gym/neural.py` (shell), `python/champions_gym/neural_examples.py` (stand-in
policies), `python/tests/test_neural.py`, `docs/examples/pcsa_former_policy.py` (the pcsa-former adapter),
`bench/python/neural_pcsa_former.py` (the checks below on a real checkpoint).

## 1. The policy protocol

```python
class MyPolicy:
    info = {                                  # or a method info() returning the dict
        "contract": "0.6",                    # contracts/MODEL.md version the model was trained on
        "layout_hash": "...",                 # champions_gym.neural.layout_hash() at training time
        "shapes": {"rows": [12, 300], "row_ids": [12, 10], "field": [66], "last": [17], "last_ids": [4]},
        "vocab": {"species": 393, "items": 168, "abilities": 319, "moves": 516},
        "model_id": "runs/x/iter-00092.pt",   # optional: logged with every decision
        "model_hash": "001fc79980c7e7b7",     # optional
        "requery_refused": False,             # optional, see 2.4
    }

    def decide(self, arrays, slots, battles):
        # arrays: champions_gym.Tensors, every field with a leading batch axis B (exactly what
        #   BatchEnv.observe_tensors() returns; numpy arrays when numpy is installed, else memoryviews).
        #   arrays.kind[r]: 0 preview, 1 turn, 2 replace. arrays.action_mask[r]: the 71 legal actions.
        # slots: B ints. One slot = one seat of one game. A model with memory keeps one history per slot.
        # battles: B champions_gym.Battle objects, built on access; most models ignore them.
        return {"scores": scores,             # [B, 71]: 0..10 turn/replace, 11..70 preview (a row may be 11 or 60 wide)
                "value": value,               # optional [B]
                "extras": extras}             # optional: B JSON-able dicts, logged

    def reset(self, slots):                   # optional: a new game starts in these slots
        ...

    def observe(self, arrays, slots, actions):  # optional: requests answered without asking (a restart's
        ...                                     # replay): take the memory step decide would have taken
```

- **Scores are logits.** Higher is better. `-inf` means "never". Entries outside the request's legal mask are
  never read, whatever they hold. `scores`/`value` may be lists, numpy arrays or torch tensors (`tolist()`
  is used).
- `layout_hash()` is the sha256 of `repr(sorted((id, block, first column, size, repeats, stride)))` over the
  input features of `champions_gym.FEATURES`. It is pcsa-former's own layout hash (`layout.py:87`), so a
  pcsa-former checkpoint's stored hash compares directly: `8b8a8fa7…5adaad` for the 0.6 feature list.
- One call per request in live play (B = 1); in `evaluate` one call per environment step for all games
  waiting on our seat.
- Slots: each NeuralPlayer and each `evaluate` run gets its own slots of a policy object (small integers,
  handed out per policy), so two players on one policy object never share a game history. A policy object
  used from several threads must lock its own state (the pcsa-former adapter does).

## 2. What the shell guarantees

Each point has its test in `python/tests/test_neural.py`.

1. **The same arrays as training.** Live: `cg.tensors(battle)`, stacked to a batch of one (`neural.stack`).
   Evaluation: `env.observe_tensors()`. Both come from `obs::observe`; stacking reproduces the batch bytes
   exactly (`test_stack_and_take_give_the_batch_arrays`). The arrays' `kind` and mask are checked against the
   request (a difference is logged as an `anomaly`). At a replace request the shell compares the arrays with
   those of the same slot's last turn request, without the request fields G1-2.1, G1-7.1, G1-7.2: equal means
   the stale Battle of MODEL.md 0.6, and the log marks `"replace": "stale"` or `"fresh"`
   (`test_replace_sees_the_stale_battle`: all stale in the live order, fresh in the old one).
2. **Actions.** Preview: the 60-way row through `cg.preview_picks` (the two back picks come out in the preview
   action's order). Turn/replace: an action 0..10.
3. **The legal mask is always applied**, so an illegal action is never returned whatever the policy outputs
   (`test_the_mask_is_always_applied`). The fallback (`fallback="tower"`, any built-in name, or None for the
   first legal action) answers that one decision and the log records why, when the policy raises, returns
   the wrong number of rows or a row of the wrong width, NaN or +inf on a legal action, or no finite score on
   any legal action (`test_every_fallback_path`). After `max_errors` (3) failures in a row the fallback plays
   the rest of the battle.
   **Time.** A call cannot be interrupted, so its answer is always used. A call over `timeout_s` (2 s) is
   logged `"slow"`. `slow_limit` (3) slow calls in a row, or one call over `hard_timeout_s` (6 s), hand the
   rest of that battle to the fallback (`test_slow_policy`). The next battle starts fresh.
4. **A refused choice.** When the request comes again with a smaller mask and the Battle's arrays are
   otherwise unchanged (request fields left out), the shell takes the best remaining legal action from the
   same scores without calling the policy (`"cached": true`). The same holds for a second replace request in
   one turn, which MODEL.md 0.6 gives the same Battle with a smaller mask. It calls the policy again when the Battle
   changed (a refusal can teach the Battle that we are trapped, or rule a move out), when the mask grew (the
   last move refused: Struggle becomes action 0), or when the policy sets `info["requery_refused"]`
   (`test_refusal_reuses_the_scores`, `test_engine_streams_with_refusals_and_a_log`).
5. **Retries, restarts and replayed requests are the Session's.** It answers a repeated `seq`, a `retry`, a
   restart resending the battle and a new process given `played` orders without asking the player; with the
   neural player in place the policy is not called for any of them (`test_retry_restart_and_replay_do_not_ask`).
6. **Determinism.** `mode="argmax"` (default) takes the highest legal score, the lowest action on ties: a pure
   function of the Battles of the game so far and the checkpoint (the game history matters only for a model
   with memory). `mode="sample"` draws from softmax(score / `temperature`) over the legal actions with a
   source seeded by `mix_seed(mix_seed(seed, slot), n)`, n the decision's number in the game, so the draw
   does not depend on batching (`test_argmax_is_deterministic_and_sampling_is_seeded`).
7. **The decision log.** `log_dir`: one file per battle, flushed per line (format in section 4). Under a
   Session it is `<log_dir>/<battle tag>.neural.jsonl`, beside the Session's own `<battle tag>.jsonl`; a
   resend appends to it (a `resend` line, then `observed` lines for the replayed requests, and every later
   decision marked `after_resend`); `battle_end` writes an `end` line with the result and the battle's
   counts and closes it. Outside a Session: `battle-<time>-<n>-s<slot>.neural.jsonl`.
8. **Self-check at construction.** `info` is compared with this build (`neural.contract()`: contract
   version, layout hash, shapes, vocabulary sizes); a difference raises `ModelMismatch` when the player is
   made, not at the first request, unless `allow_mismatch=True` (then printed and kept in
   `player.shell.mismatch`). Then one preview and one turn decision run on a fixed game (`warmup_ms`); a policy
   that fails them fails construction (`test_self_check_refuses_a_mismatch_at_construction`).
9. **No numpy or torch in the shell** (`test_without_numpy_or_torch` runs the Session and `evaluate` with both
   blocked). Without numpy, `cg.tensors` and `observe_tensors` return typed memoryviews of the same shape;
   an adapter turns either into tensors without a copy: `torch.from_numpy(a)` for numpy, or
   `torch.frombuffer(memoryview(m).cast("B"), dtype=...).reshape(shape)` for a memoryview.

10. **Battles and restarts (Session hooks, LIVE.md 0.9).** Under a Session the player is told: `new_battle(tag,
   resend)` at each accepted `battle_start`, `battle_end(result)`, and `observe(battle, action, how)` for every
   request the Session answers without asking it (a restart's `replay` / `played` requests, picks given in
   `battle_start`, requests repeated from the Session's record after a resend). It then no longer guesses
   battle boundaries (that stays only for `play_games`). A new battle clears the policy's slot; a resend clears
   it too and the replayed requests rebuild it: `observe` passes the Battle of each request, as it was when
   asked, to `policy.observe`, once per request (a repeat of a request already stepped, and a refused request
   whose Battle did not change, take no step, as when asked). So a model with memory answers the same after a
   restart as without one: `test_restart_rebuilds_the_memory` (a new Session given the battle with picks,
   `replay` and `played`, and the same Session resent) and `test_session_hooks`. Measured with the stateful
   stand-in over 40 streams restarted at a random request: 204 answers after the restart, 134 different without
   the hook, 0 with it; with the pcsa-former checkpoint see section 7.
   Not passed to the player: a `fallback` the controller reports (the game played another order) and our
   recorded choice put right by the log (rule 31). pcsa-former's memory holds each decision's input tokens,
   never the action, and the next request's Battle already shows what was played, so neither changes a
   memory step.
11. **Threads.** The Session holds its lock for each call; a NeuralPlayer also takes its own lock, and the
    pcsa-former adapter one per policy object. Two Sessions with two NeuralPlayers may run on two threads,
    also on one shared policy object (`test_two_sessions_on_two_threads`, in turn and at once). torch's
    thread count is process-wide (`policy_args.threads`).
12. **What the controller sees.** Each reply the model answered carries `notes.model`: `{"fallback": false or
    the reason, "ms", "reused", "slow", "value"}`; the `status` answer carries `player_status` (section 3b).

**Evaluation is the same code.** `evaluate(policy, opponent, games, seed, threads, batch, teams,
team_weights, replace_before_rows=True, mode, temperature, fallback, log_dir, record, time_limit_s)` drives a
`BatchEnv` with `observe_tensors()` and `env.act(actions, opponent=...)`, and chooses with the same
`Shell.decide` as the live player. Slot g plays an equal share of the games in seat g % 2 (seats balanced,
short games not over-counted). It returns win rate ((wins + ties/2)/games) with a Wilson 95% interval, wins
by seat, mean turns, refusals of our orders, fallbacks with reasons, decisions per second and policy ms per
decision. `test_evaluate_equals_one_battle_at_a_time` plays the same games batched and one Battle at a time
through `NeuralPlayer.action_for` and gets identical actions; `test_builtin_policy_reproduces_play_games` and
`..._session_streams` show that a policy scoring a built-in player's action plays exactly as that player
(5 players x 240 games through `play_games`, 30 Session streams).

As a command: `python -m champions_gym.neural --policy mod:Factory --policy-args '{"checkpoint": "..."}'
--opponents tower,max_damage --games 2000`.

## 3. Selecting it from the controller

No controller code changes. In the controller's config:

```yaml
player: "champions_gym.neural:load"
player_args:
  policy: "pcsa_former.cg_policy:PcsaFormerPolicy"   # class or factory, called with checkpoint, device, policy_args
  checkpoint: "C:/.../runs/<run>/evals/iter-00092.pt"
  device: "cpu"
  policy_args: {threads: 4}
  fallback: "tower"
  timeout_s: 2.0
  hard_timeout_s: 6.0
  log_dir: "C:/.../runs/<run>"     # the same folder as the Session's log_dir: <tag>.neural.jsonl beside <tag>.jsonl
  # mode: "argmax"   allow_mismatch: false   seed: 0
log_dir: "C:/.../runs/<run>"       # the Session's own record
```

Read `notes.model.fallback` in each reply (false, or why the fallback answered) and `status.player_status`
(section 3b) for the run record. On a restart, resend the battle from `battle_start` with `picks`, `replay`
and `played` as LIVE.md section 2 says: the model's memory is rebuilt from it.

The package holding the policy must be importable by the controller's Python (installed, or on
`PYTHONPATH`), with torch. In code: `NeuralPlayer(policy, ...)`, `load(policy=..., checkpoint=..., **kw)` or
`NeuralPlayer.from_path("mod:Factory", checkpoint=...)`.

## 3b. `status.player_status`

`name`, `model_id`, `model_hash`, `checkpoint`, `contract` (the model's), `layout_ok`, `mismatch` (list),
`allow_mismatch`, `fallback_player`, `mode`, `battle` and `total` (each: `decisions`, `policy_calls`,
`fallbacks`, `fallback_reasons`, `slow`, `reused`, `observed`, `ms_median`, `ms_max`), `handed_to_fallback`
(false or the reason), `warmup_ms`. Tested in `test_status_and_notes` and, over stdin/stdout,
`test_status_and_notes_through_the_process`.

## 4. The decision log

Line 1 of each file: `{"type": "start", "t", "battle", "player", "model": <info>, "config": {mode, temperature, seed,
fallback, timeout_s, hard_timeout_s, slow_limit}, "mismatch", "allow_mismatch", "warmup_ms"}`. Other lines with a
`type`: `resend` (a resend began), `observed` (`how`, `turn`, `decision`, `kind`, `action`, `step`: whether
the policy took a memory step), `end` (`result`, `battle`: the battle's counts). Then one line per decision:

| Field | Meaning |
|---|---|
| `t` | Unix time |
| `turn`, `decision`, `kind` | the request (`decision` is the live `seq`; -1 at preview) |
| `legal` | the legal actions |
| `scores` | the policy's 11 (or 60 at preview) scores, 4 decimals, null for non-finite |
| `action`, `order` | the chosen action and what it is (`{"kind":"move","move","mega"}`, `{"kind":"switch","mon"}`, `{"picks"}`) |
| `value` | the policy's value, if given |
| `ms` | wall time of the policy call |
| `cached` | scores reused after a refusal |
| `slow` | the call was over `timeout_s` |
| `fallback` | null, or `{"reason", "player"}` |
| `after_resend` | the decision came after a resend of the battle |
| `replace` | at a replace request: `"stale"` or `"fresh"` (2.1) |
| `model` | `{"id", "hash"}` from `info` |
| `extras` | the policy's extras (for pcsa-former: `score` [11], `p_opp` [13]; the matrix with `log_matrix`) |

The Session's own `log_dir` keeps every message and reply; the two records join on `decision` = `seq`.

## 5. The model side: what to provide

1. A class or factory taking `checkpoint`, `device` and its own keywords, returning an object with `info`,
   `decide` and, for a model with memory, `reset`.
2. `info` from the checkpoint: the contract version and `layout_hash` it was trained with, shapes,
   vocabulary sizes. Store `champions_gym.neural.contract()` in every checkpoint when it is saved.
3. `decide` returns logits over the 71 actions. To match the training policy exactly, return log of the
   policy's own probabilities (argmax = the greedy choice; sampling at temperature 1 = the training
   distribution).

The adapter for pcsa-former is `docs/examples/pcsa_former_policy.py` (copy it into `pcsa_former`): it loads
with `checkpoint.load` (which refuses another layout), runs `Net.forward` with a `model.Memory` that grows to
the slots used, and returns `log(pi)` and `log(preview_pi)` as pcsa-former's evaluation chooses from them.

## 6. Before a model plays on the phone

1. The self-check passes without `allow_mismatch` (contract 0.6, layout hash, shapes, vocabularies).
2. `evaluate` against all ten built-in players, 2,000 games each, live order (`replace_before_rows=True`),
   seats balanced; the numbers agree with the training side's own evaluation within the intervals.
3. The equivalence check: batched and one-at-a-time actions identical on the same games
   (`bench/python/neural_pcsa_former.py --part equiv`).
4. Timing on the phone machine's CPU: the recorded phone battle and 20 engine-played streams through the
   Session; worst request well under the controller's budget (8 s), no fallback (`--part live`).
5. A restart in the middle of a battle gives the same answers afterwards (`--part restart`: 0 different with
   the hooks).
6. The fallback tested with the faulty stand-ins (the test suite) and `log_dir` set; `notes.model` and
   `status.player_status` read by the controller.

## 7. Measured with a pcsa-former checkpoint (CPU)

`runs/league-20261118-f/evals/iter-00092.pt`, trained under MODEL 0.5 (the replace request after the turn's
rows), so it loads only with `allow_mismatch=True` (and the adapter's `allow_contract=True`). Results in
`bench/results/2026-11-19-neural2-*` (`bench/python/neural_pcsa_former.py`):

| Check | Result |
|---|---|
| self-check | refused without `allow_mismatch`: model contract 0.5, CG 0.6; layout hash, shapes, vocabularies equal |
| `evaluate` vs tower, 2,000 games, live order | 0.715 [0.695, 0.734]; old order 0.7125 |
| `evaluate` vs max_damage, 2,000 games | 0.732 [0.712, 0.751]; old order 0.736 |
| batched vs one at a time (200 games each vs tower and max_damage) | 2,247 / 2,247 and 2,107 / 2,107 actions identical |
| phone battle + 20 engine streams through the Session | every answer legal, no fallback; per request median 6.5 ms, p99 10.0 ms, worst 13.7 ms |
| restart at a random request (phone battle + 20 streams, 91 answers after the restarts) | with the hooks: 0 answers and 0 values different; without `observe`: 1 answer and 90 values different; same Session resent: 0 |
