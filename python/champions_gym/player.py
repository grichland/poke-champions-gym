"""Players, in poke-env's naming: `teampreview(battle)` and `choose_move(battle)` (alias `choose`).

A Player receives the seat's `Battle` (`_cg.Battle`, read-only): what that seat knows, built from the
battle log events alone, plus the request (`game`, `seat`, `kind`, `mask`, `legal_actions()`). See
`bindings/src/view.rs` for every field. `Request` is the old name of the same object.
"""

from __future__ import annotations

import random
from typing import Any, Optional, Sequence, Union

from . import _cg
from .env import KIND_NAMES, KIND_PREVIEW, N_ACTIONS, PREVIEW0, preview_action

Battle = _cg.Battle
Request = Battle
BUILTIN_PLAYERS = tuple(_cg.BUILTIN_PLAYERS)

Picks = Sequence[int]


def kind_name(battle: Any) -> str:
    return KIND_NAMES[battle.kind]


class Player:
    """Something that chooses. Subclasses implement `teampreview` and `choose_move`.

    `teampreview` returns three roster indices 0..5 (lead first) or a preview action 11..70.
    `choose_move` returns a turn/replace action 0..10.
    """

    def teampreview(self, battle: Battle) -> Union[Picks, int]:
        raise NotImplementedError

    def choose_move(self, battle: Battle) -> int:
        raise NotImplementedError

    def choose(self, battle: Battle) -> int:
        return self.choose_move(battle)

    def action_for(self, battle: Battle) -> int:
        """The action in the one action space for this request."""
        if battle.kind == KIND_PREVIEW:
            r = self.teampreview(battle)
            return int(r) if isinstance(r, int) else preview_action(tuple(int(x) for x in r))
        return int(self.choose(battle))


class RandomPlayer(Player):
    """Uniform over the legal actions (preview: uniform over the 60 ordered picks)."""

    def __init__(self, seed: Optional[int] = None):
        self.rng = random.Random(seed)

    def teampreview(self, battle: Battle) -> int:
        return PREVIEW0 + self.rng.randrange(N_ACTIONS - PREVIEW0)

    def choose_move(self, battle: Battle) -> int:
        return self.rng.choice(battle.legal_actions())


class BuiltinPlayer(Player):
    """One of CG's Rust Players by name (`BUILTIN_PLAYERS`: "random", "max_damage", "tower",
    "best_matchup_max_damage", "disciplined", "endgame", "pilot_encore", "pilot_weather", "pilot_hazard",
    "pilot_priority"), reading the same Battle.

    An action outside the request's mask (possible only when the environment runs the State rule) is
    replaced by the first legal one and counted in `outside_mask`.
    """

    def __init__(self, name: str, seed: int = 0):
        self._p = _cg.RustPlayer(name, seed)
        self.name = name
        self.outside_mask = 0

    def teampreview(self, battle: Battle) -> Picks:
        return self._p.teampreview(battle)

    def choose_move(self, battle: Battle) -> int:
        a = self._p.choose_move(battle)
        if not battle.mask[a]:
            self.outside_mask += 1
            return battle.legal_actions()[0]
        return a


def builtin_player(name: str, seed: int = 0) -> BuiltinPlayer:
    return BuiltinPlayer(name, seed)
