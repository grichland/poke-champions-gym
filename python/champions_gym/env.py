"""The batched environment and per-game trees over the `_cg` extension.

Arrays come back as numpy arrays when numpy is installed, else as typed `memoryview`s with the same shape
(no per-element Python work either way).

One action space of `N_ACTIONS` = 71 for every decision:

====== =====================================================================
0..3   move slot i
4..7   move slot i with Mega Evolution
8..10  switch to team slot 0..2 (slots are the three picks, in pick order)
11..70 team preview: ``11 + lead * 10 + pair``; see `preview_picks`
====== =====================================================================

When only Struggle or a forced move is possible, the one legal action is 0. Kinds: 0 preview, 1 turn,
2 replace.
"""

from __future__ import annotations

import array
import os
import warnings
from dataclasses import dataclass
from typing import Any, NamedTuple, Optional, Sequence, Tuple

from . import _cg
from .obs import new_tensors

try:
    import numpy as np
except ImportError:  # the package works without numpy, through memoryviews
    np = None

CGError = _cg.CGError
N_ACTIONS: int = _cg.N_ACTIONS
N_TURN_ACTIONS: int = _cg.N_TURN_ACTIONS
PREVIEW0: int = _cg.PREVIEW0
KIND_PREVIEW: int = _cg.KIND_PREVIEW
KIND_TURN: int = _cg.KIND_TURN
KIND_REPLACE: int = _cg.KIND_REPLACE
KIND_NAMES = {KIND_PREVIEW: "preview", KIND_TURN: "turn", KIND_REPLACE: "replace"}

preview_picks = _cg.preview_picks
preview_action = _cg.preview_action
mix_seed = _cg.mix_seed

#: The team pool shipped with the package: 430 six-Pokémon teams in Showdown's set format (JSON).
TEAMS: str = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", "teams.json")

_NP_TYPES = {"?": "bool", "B": "uint8", "b": "int8", "H": "<u2", "Q": "<u8", "I": "<u4", "h": "<i2", "i": "<i4", "q": "<i8", "f": "<f4"}


def _wrap(buf: bytes, fmt: str, shape: Tuple[int, ...]) -> Any:
    """`bytes` as an array of struct format `fmt` and `shape`, without copying."""
    if np is not None:
        return np.frombuffer(buf, dtype=_NP_TYPES[fmt]).reshape(shape)
    return memoryview(buf).cast(fmt, shape)


def _as_actions(actions: Any) -> Any:
    """An int32/int64 C-contiguous buffer for `act`."""
    if np is not None:
        a = np.asarray(actions)
        if a.dtype not in (np.int32, np.int64) or not a.flags.c_contiguous:
            a = np.ascontiguousarray(a, dtype=np.int64)
        return a
    if isinstance(actions, array.array) and actions.typecode in ("i", "q"):
        return actions
    return array.array("q", actions)


class Results(NamedTuple):
    """Games that ended during the last `act`, one entry per game."""

    done: Any  # bool-like uint8[N]
    winner: Any  # int8[N]: 0 or 1; -1 tie or truncated
    turns: Any  # uint16[N]
    truncated: Any  # uint8[N]: stopped at max_steps
    episode: Any  # uint64[N]: the episode that ended (0 based, per game)


class Labels(NamedTuple):
    """Training labels (never inputs) of the decisions the last `act` completed, shaped [N, 2, ...] by game
    and seat. A decision is labelled once its step has resolved (both seats of a turn have chosen, a
    replace is in, both have picked at preview); `decision` matches the `decision` of its observation in
    `observe_tensors`. Opponent actions are 13, seen from this seat: 0-3 their active's (row 6) move slot,
    4-7 the same with Mega, 8-12 a switch to the Pokémon of row 7-11 of that observation."""

    valid: Any  # uint8: a decision of this seat completed in the last act
    decision: Any  # int64
    refused: Any  # uint8: the State refused it; the seat was asked again (no opposing label)
    own_action: Any  # int16: 0-70
    opp_action: Any  # int8: 0-12; -1 none (replace, preview, Struggle, Recharge)
    opp_forced: Any  # uint8: the opponent was locked, charging or recharging
    opp_move: Any  # int32: move id chosen or forced; 0 for a switch, Recharge or none
    opp_shown: Any  # uint8: the battle log showed the choice to this seat
    opp_prior: Any  # float32 [13]: the observation's opp_prior at the decision (an input, repeated)
    opp_picks: Any  # int8 [3]: preview, the opponent's picks in its preview order, lead first
    opp_true_brought: Any  # uint8 [6], by the decision's opponent rows 6-11; truth, training only
    opp_true_moves: Any  # int32 [6, 4]: revealed moves in their slots, the rest of the set after them
    opp_true_item: Any  # int32 [6]: item id (1 holds nothing)
    opp_true_ability: Any  # int32 [6]


@dataclass
class Outcome:
    """One outcome of a step: its probability, the forced path that reproduces it, and the game after it."""

    p: float
    path: list
    game: "Game"


@dataclass
class Outcomes:
    outcomes: list
    covered: float  # probability of the outcomes listed (1 unless capped)
    capped: bool
    leaves: int  # replays run


class Game:
    """One game for trees: both seats choose together and the engine resolves the step.

    Actions are turn/replace actions 0..10, or None for a seat with no decision.
    """

    __slots__ = ("_g",)

    def __init__(self, g: Any):
        self._g = g

    @property
    def turn(self) -> int:
        return self._g.turn

    @property
    def ended(self) -> bool:
        return self._g.ended

    @property
    def winner(self) -> Optional[int]:
        """0 or 1 once ended; None while playing or for a tie."""
        return self._g.winner

    def kinds(self) -> Tuple[int, int]:
        """Each seat's decision: 1 turn, 2 replace, -1 none."""
        return self._g.kinds()

    def masks(self) -> Any:
        """bool[2, 11]: each seat's legal turn/replace actions."""
        return _wrap(self._g.masks(), "?", (2, N_TURN_ACTIONS))

    def snapshot(self) -> "Game":
        """An exact copy: same seed state, same continuation."""
        return Game(self._g.snapshot())

    def fork(self, seed: int) -> "Game":
        """A copy with its own seed: same State, independent continuation."""
        return Game(self._g.fork(seed))

    def step(self, a0: Optional[int], a1: Optional[int]) -> None:
        self._g.step(a0, a1)

    def step_forced(self, a0: Optional[int], a1: Optional[int], path: Sequence[int]) -> None:
        """One step with every random event forced from `path` (an `Outcome.path`)."""
        self._g.step_forced(a0, a1, list(path))

    def outcomes(self, a0: Optional[int], a1: Optional[int], merge: bool = True, max_branches: int = 100_000) -> Outcomes:
        """Every outcome of the next step with these actions, with its probability."""
        lst, covered, capped, leaves = self._g.outcomes(a0, a1, merge, max_branches)
        return Outcomes([Outcome(p, path, Game(g)) for p, path, g in lst], covered, capped, leaves)

    def same_state(self, other: "Game") -> bool:
        """Whether both games hold the same State (seed state not compared)."""
        return self._g.same_state(other._g)


def check_teams(path_or_text: str) -> Tuple[int, list]:
    """Reads a teams file without building an environment: ``(accepted, [(index in the file, reason), ...])``.
    `path_or_text` is a path, or the JSON text itself when it starts with ``[``."""
    return _cg.check_teams(str(path_or_text))


def _is_seq(x: Any) -> bool:
    return not isinstance(x, (str, bytes)) and hasattr(x, "__len__")


def _team_weights(w: Any) -> Optional[list]:
    """None, one sequence, or two sequences, as a list of lists of floats."""
    if w is None:
        return None
    try:
        rows = list(w)
        if rows and all(_is_seq(x) for x in rows):
            return [[float(v) for v in x] for x in rows]
        return [[float(v) for v in rows]]
    except (TypeError, ValueError) as e:
        raise ValueError("team_weights: need numbers, one per team in the file (%s)" % e) from None


class BatchEnv:
    """N games with strict turn-taking: each game waits for exactly one decision (seat, kind).

    A game that ends is reported by `results()` after that `act` and restarts with new teams drawn from the
    pool. `teams` is a teams.json path (None: the pool shipped with the package, `TEAMS`); `team_mode` is
    "real" (the sets as written) or "stage3" (No Ability, no item, stage-3 moves).
    `rule` is "battle" (the default: each seat's legal actions come from its own Battle; a seat locked
    into a move is not asked; an order the State refuses is refused and the seat asked again) or "state"
    (the State's options, every deciding seat asked; for comparison).

    Teams that cannot be read (not six valid sets) are dropped with one `warnings.warn`; `team_errors`
    lists them as (index in the file, reason). `strict_teams=True` raises ValueError instead.
    Pool indices (`team_ids`) number the accepted teams 0..team_count-1; `team_file_index[i]` is pool
    team i's index in the file.

    `team_weights`: None draws both teams of a game uniformly (two different teams). Otherwise one
    sequence of non-negative weights, one per team in the file (weights of dropped teams are ignored),
    for both seats; or two sequences ``(w_agent, w_other)``: each game's agent seat (`agent_seat`) draws
    from `w_agent`, then the other seat from `w_other` without the agent's team. The two teams of a game
    are always different, so the agent's draw follows its weights exactly and the other seat's is its
    weights given that. Draws are exact (cumulative table, one 53-bit uniform each), come from the
    game's episode seed, and do not depend on `threads`. Weights change the random stream: without them
    the games are the same as before weights existed. Weights that cannot give two different teams raise
    ValueError.

    `agent_seat`: 0, 1, or one 0/1 per game: the learner's seat in that game. It decides which seat
    draws from `w_agent` and which seat `act(..., opponent=...)` leaves to the learner. ``[g % 2 for g in
    range(games)]`` balances seats.

    `replace_before_rows` (default True, the live order): a seat's battle-log rows reach its Battle only at
    its next turn request (or the game's end), so at a replace request the Battle, the observation and the
    mask do not yet hold the rows of that turn, as on the phone (contracts/LIVE.md rule 18,
    contracts/MODEL.md section 1). False delivers each step's rows at once.
    """

    def __init__(
        self,
        teams: Optional[str],
        games: int,
        seed: int = 0,
        team_mode: str = "real",
        max_steps: int = 1000,
        rule: str = "battle",
        threads: int = 1,
        strict_teams: bool = False,
        team_weights: Any = None,
        agent_seat: Any = 0,
        replace_before_rows: bool = True,
    ):
        path = str(TEAMS if teams is None else teams)
        seats = [int(x) for x in agent_seat] if _is_seq(agent_seat) else [int(agent_seat)] * games
        self._e = _cg.BatchEnv(
            path, games, seed, team_mode, max_steps, rule, threads, strict_teams, _team_weights(team_weights), seats, bool(replace_before_rows)
        )
        self.games = games
        self.rule = rule
        self.replace_before_rows = bool(replace_before_rows)
        errs = self._e.team_errors
        if errs:
            warnings.warn(
                "%s: %d of %d teams not accepted (first: team %d: %s); see BatchEnv.team_errors, or strict_teams=True to raise"
                % (path, len(errs), self._e.team_file_count, errs[0][0], errs[0][1]),
                stacklevel=2,
            )

    @property
    def team_errors(self) -> list:
        """Teams in the file that were not accepted: [(index in the file, reason)]."""
        return self._e.team_errors

    @property
    def team_count(self) -> int:
        """Teams accepted; pool indices are 0..team_count-1."""
        return self._e.team_count

    @property
    def team_file_count(self) -> int:
        """Teams in the file, accepted or not."""
        return self._e.team_file_count

    @property
    def team_file_index(self) -> list:
        """For each pool index, the team's index in the file (the identity when nothing was dropped)."""
        return self._e.team_file_index

    @property
    def agent_seat(self) -> Any:
        """uint8[N]: each game's agent seat."""
        return _wrap(self._e.agent_seat(), "B", (self.games,))

    def builtin_actions(self, name: str, seed: Optional[int] = None, out: Any = None) -> Any:
        """int32[N]: for every game, the action (one action space, preview included) that built-in player
        `name` (one of `BUILTIN_PLAYERS`) takes at the pending decision: the same Battle and the
        same answer as ``BuiltinPlayer(name, s).action_for(env.battle(g))`` with
        ``s = mix_seed(mix_seed(seed, g), env.battle(g).decision)``. `seed` None uses the environment's
        seed. Computed in Rust on the environment's workers; the same for any thread count. `out`: a
        writable int32 or int64 array of length N to write into (returned)."""
        if out is None:
            out = np.empty(self.games, dtype=np.int32) if np is not None else memoryview(bytearray(4 * self.games)).cast("i")
        self._e.builtin_into(name, out, seed)
        return out

    @property
    def threads(self) -> int:
        """Worker threads `act` and `observe_tensors` step the games on; the results do not depend on it."""
        return self._e.threads

    @threads.setter
    def threads(self, n: int) -> None:
        self._e.threads = n

    def battle(self, g: int, seat: Optional[int] = None) -> Any:
        """What the pending seat of game g (or `seat`) knows: its `Battle`, read-only."""
        return self._e.battle(g, seat)

    def counters(self) -> Tuple[int, int, int]:
        """(refusals, decisions that fell back to the State's options, seats not asked while locked)."""
        return self._e.counters()

    def replace_counters(self) -> Tuple[int, int, int]:
        """(replace requests asked, of those asked before rows the seat had not been given, events held back)."""
        return self._e.replace_counters()

    def __len__(self) -> int:
        return self.games

    @property
    def decisions(self) -> int:
        return self._e.decisions

    @property
    def games_done(self) -> int:
        return self._e.games_done

    def pending(self) -> Tuple[Any, Any]:
        """(seat uint8[N], kind uint8[N])."""
        s, k = self._e.pending()
        return _wrap(s, "B", (self.games,)), _wrap(k, "B", (self.games,))

    def masks(self) -> Any:
        """bool[N, 71] over the one action space."""
        return _wrap(self._e.masks(), "?", (self.games, N_ACTIONS))

    def observe(self) -> Tuple[Any, Any, Any]:
        """(seat, kind, masks) in one call."""
        s, k, m = self._e.observe()
        n = self.games
        return _wrap(s, "B", (n,)), _wrap(k, "B", (n,)), _wrap(m, "?", (n, N_ACTIONS))

    def observe_tensors(self, out: Any = None, threads: Optional[int] = None, damage: bool = True) -> Any:
        """Every game's observation for its pending seat: a `Tensors` of [N, ...] arrays (see
        `champions_gym.obs`). `out`: arrays from `new_tensors(N)` (or an earlier result) to write into
        instead of new ones. `threads`: None uses the environment's workers, else that many threads."""
        if out is None:
            out = new_tensors(self.games)
        self._e.observe_into(*out, threads=threads, damage=damage)
        return out

    def act(self, actions: Any, opponent: Optional[str] = None, opponent_seed: Optional[int] = None) -> None:
        """One action per game (any int sequence or array of length N). Raises CGError on an illegal one
        before any game moves.

        `opponent`: a built-in player's name. Each game still waits on one seat; where that seat is not
        the game's `agent_seat`, the game takes the built-in player's action (as `builtin_actions(opponent,
        opponent_seed)` gives it, computed in Rust) and its entry in `actions` is ignored (any int). A batch
        where some games wait on the learner and some on the opponent is driven by one call: observe,
        fill `actions` for the games with ``seat == agent_seat`` (the others' rows are the opponent's
        observations), act. Results and labels are as without `opponent`; the learner's labels are
        ``labels()[g, agent_seat[g]]``."""
        self._e.act(_as_actions(actions), opponent, opponent_seed)

    def results(self) -> Results:
        d, w, t, tr, ep = self._e.results()
        n = (self.games,)
        return Results(_wrap(d, "B", n), _wrap(w, "b", n), _wrap(t, "H", n), _wrap(tr, "B", n), _wrap(ep, "Q", n))

    def labels(self) -> Labels:
        """Labels of the decisions the last `act` completed (see `Labels`)."""
        n = (self.games, 2)
        v, d, r, oa, pa, pf, pm, ps, pr, pk, tb, tm, ti, ta = self._e.labels()
        return Labels(
            _wrap(v, "B", n), _wrap(d, "q", n), _wrap(r, "B", n), _wrap(oa, "h", n), _wrap(pa, "b", n),
            _wrap(pf, "B", n), _wrap(pm, "i", n), _wrap(ps, "B", n), _wrap(pr, "f", n + (13,)), _wrap(pk, "b", n + (3,)),
            _wrap(tb, "B", n + (6,)), _wrap(tm, "i", n + (6, 4)), _wrap(ti, "i", n + (6,)), _wrap(ta, "i", n + (6,)),
        )

    def team_ids(self) -> Any:
        """uint32[N, 2]: pool indices of each game's two teams."""
        return _wrap(self._e.team_ids(), "I", (self.games, 2))

    def legal_actions(self, g: int) -> list:
        return self._e.legal_actions(g)

    def snapshot(self, g: int) -> Game:
        """An exact copy of game g (not during team preview)."""
        return Game(self._e.snapshot(g))

    def fork(self, g: int, seed: int) -> Game:
        """A copy of game g with its own seed."""
        return Game(self._e.fork(g, seed))
