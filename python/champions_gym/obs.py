"""The observation: the model's input arrays from a Battle (what one seat knows), laid out as in
`obs/features_proposed.csv`.

    t = champions_gym.tensors(battle)       # or battle.tensors(); one game
    t = env.observe_tensors()               # every game of a BatchEnv, its pending seat

`Tensors` fields (a batch adds a leading N axis):

=========== ============ ======= ===============================================================
rows        [12, 300]    float32 Pokémon rows: 0..6 ours, 6..12 the opponent's
row_ids     [12, 10]     int32   per row: species, item, ability, 4 moves, 3 possible abilities
field       [66]         float32 field vector
last        [17]         float32 last completed turn
last_ids    [4]          int32   our move, our switch-in species, their move, their switch-in species
mask        [11]         uint8   legal turn/replace actions from the Battle (1 = legal); 0 at preview
action_mask [71]         uint8   what `act` accepts, in the one action space (11..71 preview)
seat        scalar / [N] uint8
kind        scalar / [N] uint8   0 preview, 1 turn, 2 replace
decision    scalar / [N] int64   the decision's id: the key of its label in `BatchEnv.labels()`
opp_prior   [13]         float32 how available each opponent action is (0 impossible, 1 certain): 0-3
                                 row 6's move slots, 4-7 with Mega, 8-12 switch to rows 7-11
=========== ============ ======= ===============================================================

At team preview the Battle holds our six and the opponent's six preview species only; see
`env/src/observe.rs` for what each array holds then. `FEATURES` maps each feature id to its array
(`block`), columns and range; `VOCAB` gives the embedding sizes (0 is reserved in each).

Arrays are numpy arrays when numpy is installed, else typed `memoryview`s over `bytearray`s with the same
shape. Either way the extension writes into them directly; pass `out=` to reuse arrays across calls.
"""

from __future__ import annotations

from typing import Any, Dict, List, NamedTuple, Optional, Tuple

from . import _cg

try:
    import numpy as np
except ImportError:  # works without numpy, through memoryviews
    np = None

SIZES: Dict[str, int] = dict(_cg.layout_sizes())
VOCAB: Dict[str, int] = {k: SIZES[k] for k in ("species", "moves", "items", "abilities")}
N_ROWS: int = SIZES["n_rows"]
MOVE_BASE: int = SIZES["move_base"]
MOVE_STRIDE: int = SIZES["move_stride"]

SHAPES: Dict[str, Tuple[int, ...]] = {
    "rows": (N_ROWS, SIZES["row"]),
    "row_ids": (N_ROWS, SIZES["row_ids"]),
    "field": (SIZES["field"],),
    "last": (SIZES["last"],),
    "last_ids": (SIZES["last_ids"],),
    "mask": (SIZES["mask"],),
    "action_mask": (SIZES["actions"],),
    "seat": (),
    "kind": (),
    "decision": (),
    "opp_prior": (SIZES["opp_actions"],),
}
DTYPES: Dict[str, str] = {
    "rows": "float32",
    "row_ids": "int32",
    "field": "float32",
    "last": "float32",
    "last_ids": "int32",
    "mask": "uint8",
    "action_mask": "uint8",
    "seat": "uint8",
    "kind": "uint8",
    "decision": "int64",
    "opp_prior": "float32",
}
_FMT = {"float32": ("f", 4), "int32": ("i", 4), "uint8": ("B", 1), "int64": ("q", 8)}


class Feature(NamedTuple):
    """One feature of the layout. `block` names the `Tensors` field it lives in ("table" for the fixed
    learnset table). Move-slot features (`slot`) repeat in each of the four move slots."""

    id: str
    name: str
    block: str
    offset: int
    size: int
    repeats: int
    slot: bool
    encoding: str
    scale: str
    lo: float
    hi: float
    vocab: Optional[str]  # for id features: the key into VOCAB

    def columns(self) -> List[int]:
        """Indices on the last axis of its block (all four move slots for a move-slot feature)."""
        if self.slot:
            return [MOVE_BASE + MOVE_STRIDE * s + self.offset + k for s in range(4) for k in range(self.size)]
        return list(range(self.offset, self.offset + self.size * self.repeats))

    @property
    def vocab_size(self) -> Optional[int]:
        return VOCAB[self.vocab] if self.vocab else None


FEATURES: Dict[str, Feature] = {f[0]: Feature(*f) for f in _cg.features()}


class Tensors(NamedTuple):
    rows: Any
    row_ids: Any
    field: Any
    last: Any
    last_ids: Any
    mask: Any
    action_mask: Any
    seat: Any
    kind: Any
    decision: Any
    opp_prior: Any


def _empty(name: str, lead: Tuple[int, ...]) -> Any:
    shape = lead + SHAPES[name]
    dtype = DTYPES[name]
    if np is not None:
        return np.zeros(shape, dtype=dtype)
    fmt, size = _FMT[dtype]
    n = 1
    for d in shape:
        n *= d
    return memoryview(bytearray(n * size)).cast(fmt, shape)


def new_tensors(games: int) -> Tensors:
    """Arrays for `games` games, to pass as `out=` to `BatchEnv.observe_tensors`."""
    return Tensors(*(_empty(k, (games,)) for k in Tensors._fields))


def battle_tensors(battle: Any, damage: bool = True) -> Tensors:
    """The observation of one Battle (`champions_gym.Battle`)."""
    parts = [_empty(k, ()) for k in ("rows", "row_ids", "field", "last", "last_ids", "mask", "opp_prior")]
    battle.tensors_into(*parts, damage=damage)
    m = bytearray(battle.mask)
    am = np.frombuffer(m, dtype=np.uint8) if np is not None else memoryview(m)
    return Tensors(*parts[:6], am, battle.seat, battle.kind, battle.decision, parts[6])


tensors = battle_tensors


def learnset_table() -> Any:
    """uint8 [species vocab, move vocab]: 1 where the species can learn the move (fixed, M2-1)."""
    shape = (VOCAB["species"], VOCAB["moves"])
    if np is not None:
        t = np.zeros(shape, dtype=np.uint8)
    else:
        t = memoryview(bytearray(shape[0] * shape[1])).cast("B", shape)
    _cg.learnset_into(t)
    return t
