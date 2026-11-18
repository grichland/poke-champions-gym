"""Plays N games between two Players with strict turn-taking: each call asks one seat per game. Each Player
gets its seat's Battle."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional, Union

from .env import N_ACTIONS, BatchEnv, mix_seed
from .player import BuiltinPlayer, Player


@dataclass
class GameRecord:
    winner: int  # 0 or 1; -1 tie or truncated
    turns: int
    truncated: bool


@dataclass
class MatchResult:
    games: List[GameRecord] = field(default_factory=list)
    refusals: int = 0  # orders the State refused that the Battle allowed (the seat was asked again)

    @property
    def wins(self) -> List[int]:
        return [sum(g.winner == s for g in self.games) for s in (0, 1)]

    @property
    def ties(self) -> int:
        return sum(g.winner < 0 and not g.truncated for g in self.games)

    @property
    def truncated(self) -> int:
        return sum(g.truncated for g in self.games)


def _player(p: Union[Player, str], seed: int) -> Player:
    return BuiltinPlayer(p, seed) if isinstance(p, str) else p


def play_games(
    p0: Union[Player, str],
    p1: Union[Player, str],
    n_games: int,
    teams: Optional[str] = None,
    seed: int = 0,
    batch: int = 256,
    team_mode: str = "real",
    max_steps: int = 1000,
    rule: str = "battle",
) -> MatchResult:
    """`p0` plays seat 0 and `p1` seat 1 in `n_games` games, `batch` at a time. A Player may be given by
    the name of a built-in one (`BUILTIN_PLAYERS`). Each batch slot plays one game; slots that
    finished early are stepped with their first legal action until the batch is done. `teams`: a teams.json
    path, or None for the pool shipped with the package."""
    players = (_player(p0, mix_seed(seed, 100)), _player(p1, mix_seed(seed, 101)))
    out = MatchResult()
    rnd = 0
    while len(out.games) < n_games:
        b = min(batch, n_games - len(out.games))
        env = BatchEnv(teams, b, mix_seed(seed, rnd), team_mode, max_steps, rule)
        rnd += 1
        finished = [False] * b
        left = b
        actions = [0] * b
        while left:
            # raw bytes: indexable per game with or without numpy
            seat, kind, mask = env._e.observe()
            for g in range(b):
                if finished[g]:
                    actions[g] = mask[g * N_ACTIONS : (g + 1) * N_ACTIONS].index(1)
                    continue
                actions[g] = players[seat[g]].action_for(env._e.battle(g))
            env.act(actions)
            r = env.results()
            for g in range(b):
                if r.done[g] and not finished[g]:
                    finished[g] = True
                    left -= 1
                    out.games.append(GameRecord(int(r.winner[g]), int(r.turns[g]), bool(r.truncated[g])))
        out.refusals += env._e.counters()[0]
    return out
