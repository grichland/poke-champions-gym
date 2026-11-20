# champions-gym (CG)

CG is a simulator of Pokémon Champions Ranked Singles (Showdown format `[Gen 9 Champions] BSS Reg M-C`:
bring 3 of 6, level 50, Mega Evolution, Item Clause) written in Rust, with a batched Python environment
for reinforcement learning. Every random event goes through one outcome source, so games are reproducible
from a seed, can be forked, and can list every outcome of a step with its probability.

Licence: MIT (`LICENSE`). Third-party notices: `THIRD_PARTY_NOTICES.md`.

## What is in the box

- **Engine** (`engine/`): the full rules of the format for every legal Pokémon, move, item and ability
  (392 species, 515 moves, 166 items, 318 abilities, 82 Megas). The battle state is one plain value of
  about 2.6 KB that can be copied to fork a game.
- **Battle** (`battle/`): what one player knows, built only from battle-log events
  (`contracts/LIVE.md`): its own team in full, of the opponent only what has been shown. Players and the
  observation read a Battle, never the engine's state.
- **Built-in players** (`players/`): `random`, `max_damage` and `tower`, and ports of the older Python
  players: `best_matchup_max_damage`, `disciplined`, `endgame`, `pilot_encore`, `pilot_weather`,
  `pilot_hazard`, `pilot_priority`, and `lookahead` (`env/src/lookahead.rs`), which searches one turn
  ahead on the engine (see "Built-in players" below). All usable as opponents by name.
- **Batched environment** (`env/`, `bindings/`, `python/champions_gym`): N games stepped together with
  strict turn-taking (each call asks one seat of each game for one decision), legal-action masks, results,
  snapshot and fork, outcome enumeration, optional worker threads (the same games for any thread count).
- **Observation and labels** (`obs/`, `contracts/MODEL.md`): per decision, Pokémon rows `[12, 300]`, ids
  `[12, 10]`, a field vector, the last turn, the action mask; and training labels (the opponent's action,
  the true hidden sets) that are never inputs.

The game data is generated from Pokémon Showdown's `champions` mod at commit `c0845f33d`
(`data/SOURCE.json`). Showdown is used only as a test reference by the development tools under `tools/`;
nothing in the engine or the package needs it.

## Requirements

- Windows x64 (the only platform built and tested so far).
- Rust stable for `x86_64-pc-windows-gnu`, installed with rustup (`~/.cargo`, `~/.rustup`).
- [llvm-mingw](https://github.com/mstorsjo/llvm-mingw) (ucrt, x86_64) as the C toolchain and linker.
  `tools/build/env.sh` finds it from `LLVM_MINGW`, else from `x86_64-w64-mingw32-clang` on `PATH`.
- Python 3.12 (x64). numpy is optional (see below).
- Git Bash for the build scripts.
- The crates in `Cargo.lock` (pyo3 and its dependencies) in the cargo cache. The commands below use
  `--offline`; on a machine whose cache is empty, run `cargo fetch` once with network access.

## Build

From the repository root, in Git Bash:

```
source tools/build/env.sh
cargo build --offline --release --workspace
bash tools/build/python-ext.sh
"/c/Program Files/Python312/python.exe" tools/build/wheel.py
```

`python-ext.sh` builds the extension for `C:\Program Files\Python312\python.exe` (or the Python given as
its argument) and copies it into `python/champions_gym/`. `wheel.py` writes
`dist/champions_gym-0.5.0-cp312-cp312-win_amd64.whl`. Install it into a virtual environment:

```
"/c/Program Files/Python312/python.exe" -m venv cg-venv
cg-venv/Scripts/python.exe -m pip install --no-index --no-deps dist/champions_gym-0.5.0-cp312-cp312-win_amd64.whl
```

Without installing, `PYTHONPATH=python` also works from the repository root. `champions_gym.__version__`
tells builds apart (0.5.0: a Game's views, Games from a live Battle, sampled outcomes; 0.4.0: LIVE 0.9,
the model player hooks; 0.3.0: LIVE 0.8; 0.2.0: LIVE 0.7).

numpy is an optional extra (`champions_gym[numpy]`). With numpy every array the package returns is a
numpy array; without it the same data comes back as typed `memoryview`s of the same shape (indexable as
`m[g, a]`, convertible with `.tolist()`), and nothing else is lost.

## Examples

Random legal actions in 64 games at once:

```python
import random
import champions_gym as cg

env = cg.BatchEnv(cg.TEAMS, 64, seed=1)  # 64 games, teams drawn from the shipped pool
rng = random.Random(1)
wins = [0, 0]
while env.games_done < 500:
    masks = env.masks()  # [64, 71]: legal actions of each game's pending decision
    env.act([rng.choice([a for a in range(cg.N_ACTIONS) if masks[g, a]]) for g in range(env.games)])
    r = env.results()  # games that ended during this act; they restart with new teams
    for g in range(env.games):
        if r.done[g] and r.winner[g] >= 0:
            wins[r.winner[g]] += 1
print(env.games_done, "games,", env.decisions, "decisions, wins by seat", wins)
```

The built-in players against each other:

```python
import champions_gym as cg

r = cg.play_games("tower", "max_damage", 2000, seed=7)
print("tower", r.wins[0], "max_damage", r.wins[1], "ties", r.ties, "truncated", r.truncated)
```

A learner against a built-in player, with one team drawn 60% of the time for the learner's seat:

```python
import champions_gym as cg

n, errors = cg.check_teams(cg.TEAMS)  # (430, []): teams accepted, (file index, reason) for each other
w = [1.0] * 430
w[0] = 1.5 * 429  # team 0 carries 60% of the weight
env = cg.BatchEnv(cg.TEAMS, 256, seed=1, team_weights=(w, [1.0] * 430), agent_seat=[g % 2 for g in range(256)])
while env.games_done < 200:
    learner = env.builtin_actions("max_damage")  # stands in for the learner's actions
    env.act(learner, opponent="tower")  # games waiting on the other seat take tower's action
print(env.games_done, "games")
```

At a replace request the seat's Battle does not yet hold the rows of that turn, as on the phone (the log
is read on turn screens only): its rows are applied before its next turn request (`contracts/MODEL.md`
section 1). `BatchEnv(..., replace_before_rows=False)` (`players_run --replace-after-rows`) gives each
step's rows at once instead.

Teams that cannot be read are dropped with one warning and listed in `env.team_errors`
(`strict_teams=True` raises instead). `team_weights` has one weight per team in the file, for both seats,
or two sequences: the learner's seat (`agent_seat`, per game) and the other. Each `act` still asks one seat
per game: with `opponent`, the games waiting on the learner's seat take its action and the others the
built-in player's, so the learner only needs the rows where `seat == agent_seat`.
`env.builtin_actions(name)` gives a built-in player's action for every game's pending decision (computed
in Rust on the worker threads), the same as `cg.BuiltinPlayer` asked game by game.

### Trees and lookahead

`env.snapshot(g)` and `env.fork(g, seed)` copy one game into a `cg.Game`: both seats choose together
(`Game.step(a0, a1)`, None for a seat with no decision, `Game.masks()` the State's legal actions),
`Game.outcomes(a0, a1)` enumerates every outcome of a step with its probability and forced path
(`Game.step_forced`), and `Game.sample(a0, a1, n, seed)` draws `n` outcomes with the engine's
probabilities when enumeration is too large (weight 1/n per draw, equal States merged). `Game.readout()`
gives the State's HP fractions, faints, statuses and active slots of both sides.

With `views=True` the Game also carries each seat's Battle: `Game.battle(seat)` and `Game.tensors(seat)`
are what `BatchEnv` would show that seat had it played the same actions with the same random outcomes
(the env's `replace_before_rows` order included; on a fresh fork exactly `observe_tensors` for the
pending seat). A seat with no open request (not deciding, locked, already answered, the game over) shows
its Battle as it last saw it with `kind == cg.KIND_NONE`, decision -1 and an empty mask. Forks, steps and
outcome children keep the views at no cost; the rows are emitted the first time a view is read.
`cg.game_tensors(games, seats, out=None, threads=1)` writes many Games' observations into one [N, ...]
batch. Views cost a copy of both Battles (a few microseconds) at `env.fork`, so they are asked for.

`env.restore(slot, game, seed)` puts a copy of `env.snapshot(g, views=True)` (not stepped since; it can be
restored again, into any number of slots) back into the batch as an ordinary game, waiting on its pending
turn or replace decision, its luck from then on drawn from `seed` alone (as `game.fork(seed)`), so the same
snapshot and seed give the same game in any slot for any `threads`. Both Battles, decision ids, the label
records, `team_ids` and `agent_seat` come with it; no preview is asked and no start reported; its turn
count and step cap count from the original game's first turn, and `results()` reports it once, under the
slot's episode number, before the slot restarts as usual. It goes only into a slot whose game has not
started (just ended, as `results()` shows, or a new environment), from an environment with the same rule,
team mode, `replace_before_rows` and teams; anything else raises CGError. About 2 microseconds (10 from
another environment, whose teams are compared); the snapshot with views costs about 1.5 more for it.

```python
import champions_gym as cg

env = cg.BatchEnv(cg.TEAMS, 64, seed=1)
saved = []
while env.games_done < 200:
    env.act(env.builtin_actions("tower"))
    kind = env.pending()[1].tolist()
    if kind[0] == cg.KIND_TURN and len(saved) < 8:
        saved.append(env.snapshot(0, views=True))
    for g, done in enumerate(env.results().done.tolist()):
        if done and saved:
            env.restore(g, saved[g % len(saved)], seed=g)  # another continuation of a saved position
```

A Game can also start from a live battle: `Session.game(opponent)` (or `cg.game_from_battle(battle,
opponent)`) builds the engine State from the Session's Battle at its open request plus the caller's
guesses for the opponent's three brought (unseen moves, item, ability, nature, stat points). CG only
checks the guesses (learnset, ability, Mega Stone, stat point limits, nothing the Battle revealed is
contradicted) and returns the ones it could not use with the reason; `cg.env.UNKNOWN` lists what the
Battle cannot tell and the rule used. Our seat's views are the live Battle, continued with the rows of
each step; the opponent's Battle is not known.

```python
import champions_gym as cg

def lookahead(session, opponent, a1_choices, n=50):
    """Mean HP fraction we keep after one turn, per our action, against a guessed opponent action set."""
    game, rejected = session.game(opponent)        # opponent: [{"slot": 2, "moves": [...], ...}, ...]
    mine = [a for a in range(cg.N_TURN_ACTIONS) if game.battle(0).mask[a]]
    value = {}
    for a0 in mine:
        total = 0.0
        for a1 in a1_choices:
            for o in game.sample(a0, a1, n, seed=1).outcomes:
                total += o.p * sum(o.game.readout().hp[0]) / 3
        value[a0] = total / len(a1_choices)
    return value
```

## Built-in players

`cg.BUILTIN_PLAYERS` lists them; each name works in `cg.builtin_player(name)`, `cg.play_games(name, ...)`,
`env.builtin_actions(name)`, `env.act(..., opponent=name)` and `players_run --a/--b name`. Each sees only its
own Battle; unknown opposing stats, items, abilities and moves follow one fixed rule (neutral nature, no stat
points, no item, revealed moves and abilities only), with no build catalogs.

| Name | What it does |
|---|---|
| `random` | uniform over the legal actions |
| `max_damage` | the move with the highest damage estimate; switches only when forced |
| `tower` | the Battle Tower trainer AI: KOs (priority first), then a random pick among sensible moves |
| `best_matchup_max_damage` | max damage, plus a voluntary switch out of a really bad type and speed matchup |
| `disciplined` | max damage plus switching, anti-setup, recovery and "ahead: attack only" rules |
| `endgame` | disciplined until both sides have two or fewer left, then a 1v1 duel chain picks stay, switch or send |
| `pilot_encore`, `pilot_weather`, `pilot_hazard`, `pilot_priority` | tower plus one signature rule taken 85% of the time when it applies; bring their gimmick members first |
| `lookahead`, `lookahead2` | one-turn (two-turn) search on an engine State built from its Battle: every legal action against every opposing action, outcomes sampled, positions scored; keeps the action whose worst replies are best. Preview: the three with the best worst-case matchup |

`lookahead` builds the State with `game_from_battle`'s fixed rule and fills unrevealed opposing moves with
placeholders from public species data (a STAB move per type and one coverage move); it reads nothing
else. Each position is scored by Pokémon left and their HP, statuses, boosts, who moves first and
hazards. Options follow the name after `@`: `lookahead@depth=2,samples=4,replies=2,seed=7,fill=1`
(`depth` 1 or 2, `samples` draws per pair of actions, default 4; `replies` worst opposing replies averaged,
default 2, 1 is maximin; `seed` mixed into the draws; `fill` 2 (default) or 1, without the coverage move).
It is deterministic for a seed. About 0.3 ms per decision at depth 1 and 1.4 to 2 ms at depth 2 on one worker
thread; results against the other built-ins: `bench/results/2026-11-19-lookahead-results.log`.

Their rules, thresholds and sources are in each module's header under `players/src/` and `env/src/lookahead.rs`. Team pools that fit the
pilots are in `bench/data/pools/pilot_<style>.json` (converted with `tools/pools/showdown_to_cg.py`);
`players_run --teams-a FILE` gives player A its own pool.

A Player of your own subclasses `cg.Player` and implements `teampreview(battle)` (three roster indices,
lead first) and `choose_move(battle)` (an action 0..10 from `battle.legal_actions()`); `play_games`
accepts it in place of a name. `env.observe_tensors()` returns every game's observation for its pending
seat and `env.labels()` the labels of the decisions the last `act` completed; both are described in
`contracts/MODEL.md` and in the docstrings of `champions_gym.obs` and `champions_gym.env`.

The action space has 71 actions for every decision: 0-3 a move, 4-7 the same move with Mega Evolution,
8-10 a switch to the first, second or third pick (the active one is never legal), 11-70 the 60 ordered
team-preview picks.

## Tests

```
source tools/build/env.sh
cargo test --offline --release --workspace
for t in python/tests/*.py; do PYTHONPATH=python "/c/Program Files/Python312/python.exe" $t; done
```

The Python tests also run under pytest and need no numpy. Against an installed wheel, run them from a
virtual environment without `PYTHONPATH`. They include 100,000 random decisions all inside the legal
mask, same seed giving the same games for any thread count, and enumerated outcomes summing to one.

Longer checks (the built-in players with every Battle checked against the engine's state; numbers are
printed as counters that must be zero):

```
./target/release/players_run.exe --a tower --b max_damage --games 20000 --seed 5512 --threads 8 --cap 300
```

## Speed

Measured on 18 Nov 2026 on an Intel i9-14900HX with 4,096 games in one `BatchEnv`, through Python:

| Threads | Decisions/s without observations | Decisions/s with observations |
|---|---|---|
| 1 | 81,000 | 41,000 |
| 24 | 484,000 | 353,000 |

## Live play

`champions_gym.live.Session` plays a live battle for a phone controller (`contracts/LIVE.md`): it takes the
controller's messages in order, builds the Battle with the same Builder as training, and answers team
preview with picks and each decision request with an order. Every argument is a keyword a config can pass:
`player` is a built-in name (`champions_gym.live.builtin_names()`: twelve), a `"module:attr"` path to a class or
factory (called with `player_args`, a dict), or any object with `teampreview(battle)` and
`choose_move(battle)` (it gets the same `Battle` as `cg.Player`); an unknown player fails when the Session is
made. `seed` seeds the built-in players; `log_dir` keeps every message and reply as JSON lines per battle,
closed at `battle_end`. It never raises on bad input: it answers `{"error": ...}`. After a restart the
controller sends the battle again with the picks in `battle_start.picks` and each answered request marked
with the order the game took (`"played"`): the Player is not asked again (LIVE.md section 2, "Restart").
A Session may be made on one thread and used from any other, one call at a time; calls from several
threads at once wait for each other (a lock held for the whole call). `BatchEnv` may also be used from any
thread, one call at a time; a call while another thread's call is under way raises `RuntimeError`. Every
picks or order reply carries `notes.anomalies` and `notes.corrections` (running counts), and the message
`{"type": "status"}` is answered with what the Session holds (`Session.status()`; LIVE.md section 3), in
process or over stdin and stdout.

```python
import json
from champions_gym.live import Session

s = Session(player="tower")    # or Session(player="mypkg.net:NetPlayer", player_args={"ckpt": "a.pt"})
for line in open("python/tests/data/live-phone-battle.jsonl", encoding="utf-8"):  # one recorded phone battle
    reply = s.handle(json.loads(line))
    if reply: print(reply.get("picks") or reply["order"])
```

As a separate process: `python -m champions_gym.live --player tower [--player-args JSON] [--seed N]
[--log-dir D]` reads one message per line on stdin and writes one reply line per input line on stdout
(`null` when there is none).

## Neural player

`champions_gym.neural` puts a model behind the Player interface: the model supplies a small policy
(`decide(arrays, slots, battles)` returning scores over the 71 actions, an `info` dict naming the contract and
layout it was trained on, optionally `reset(slots)` and `observe(arrays, slots, actions)` for a model with
memory), and CG's shell does the rest: the arrays of
`cg.tensors(battle)`, the legal mask, argmax or seeded sampling, a built-in fallback when the policy fails or
is too slow, scores reused after a refused choice, a check of `info` against this build when the player is
made, a warm-up, and a JSON-lines decision log (`<log_dir>/<battle tag>.neural.jsonl`). Under a Session it
is told when a battle starts, is resent or ends, and is shown every request answered without asking it, so
a model with memory answers the same after a restart; replies carry `notes.model` (fallback, ms, reused) and
the `status` answer `player_status` (LIVE.md 0.9). The controller selects it with `player:
"champions_gym.neural:load"` and `player_args: {policy: "pkg.mod:Factory", checkpoint: ..., device: "cpu"}`.
`champions_gym.neural.evaluate(policy, opponent="tower", games=2000)` plays the same choice code over a
`BatchEnv`. Neither needs numpy or torch; the policy may. Protocol, guarantees, log format and the checklist
before a model plays on the phone: `docs/neural-player.md`; an adapter for pcsa-former:
`docs/examples/pcsa_former_policy.py`.

## Contracts

- `contracts/MODEL.md`: the observation, the action space and the labels: everything a model needs.
- `contracts/LIVE.md`: the battle-log events a Battle is built from, in training (emitted by the engine)
  and live (from a phone controller).

## Layout

| Path | What |
|---|---|
| `engine/` | Rust: the rules, the battle state, the outcome source, the generated data tables |
| `battle/` | Rust: the Battle and its builder from events; legal actions from a Battle |
| `emit/` | Rust: turns the engine's log rows into each seat's events |
| `players/` | Rust: the scripted built-in players (random, max damage, tower and the seven ports) |
| `obs/` | Rust: the observation and the learnset table |
| `env/` | Rust: the batched environment, labels, outcome enumeration, team pool; the `lookahead` player and the by-name registry of every built-in (`env/src/builtin.rs`) |
| `bindings/` | Rust + pyo3: the `_cg` extension |
| `python/champions_gym/` | the Python package; `data/teams.json` is the shipped pool of 430 teams |
| `python/tests/` | Python tests |
| `bench/` | benchmarks and long checks, with recorded results |
| `rig/`, `tools/` | development only: the Showdown comparison rig, data generators, build scripts |
| `data/` | generated game data and the pinned Showdown commit |
| `contracts/` | the model and live contracts |
| `docs/`, `gaps/` | investigation notes and the register of rule differences (development evidence) |
| `PROJECT.md` | the project's goals, decisions and status |
