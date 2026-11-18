"""Stand-in policies for `champions_gym.neural`: worked examples of the protocol and test doubles.

- `BuiltinPolicy`: a built-in player as a policy (score 1 on its action, 0 elsewhere): the shell must then
  play exactly as that player.
- `UniformPolicy`: every score 0 (argmax: the lowest legal action; sample: uniform).
- `HashPolicy`: scores that are a fixed function of the arrays alone (optionally of the slot's history too),
  for determinism and batched-versus-single checks.
- `FaultyPolicy`: NaN, wrong shape, raises, slow, prefers illegal actions, or a wrong `info`.
"""

from __future__ import annotations

import hashlib
import struct
import time
from typing import Any, Dict, List, Optional, Sequence

from .env import N_ACTIONS, mix_seed
from .neural import _KEY_FIELDS, _row_bytes, contract
from .player import BuiltinPlayer


def _info(name: str, **more: Any) -> Dict[str, Any]:
    i = contract()
    i.update(model_id=name, model_hash=hashlib.sha256(name.encode()).hexdigest()[:16])
    i.update(more)
    return i


class BuiltinPolicy:
    """Built-in player `name` as a policy. `per_request_seed`: seed each decision's source with
    mix_seed(seed, decision) as the live Session does for a built-in name; else one source seeded `seed`
    for all calls, in call order (as `play_games` seeds its built-in players). Asked again after a refusal
    (`requery_refused`), as a built-in player is."""

    def __init__(self, name: str = "tower", seed: int = 0, per_request_seed: bool = False):
        self.name, self.seed, self.per_request = name, int(seed), per_request_seed
        self._p = BuiltinPlayer(name, seed)
        self.info = _info("builtin:" + name, requery_refused=True)
        self.calls = 0

    def decide(self, arrays: Any, slots: Sequence[int], battles: Any) -> Dict[str, Any]:
        out = []
        for r in range(len(slots)):
            b = battles[r]
            p = BuiltinPlayer(self.name, mix_seed(self.seed, int(b.decision) & 0xFFFFFFFFFFFFFFFF)) if self.per_request else self._p
            a = p.action_for(b)
            row = [0.0] * N_ACTIONS
            row[a] = 1.0
            out.append(row)
            self.calls += 1
        return {"scores": out}


class UniformPolicy:
    def __init__(self) -> None:
        self.info = _info("uniform")

    def decide(self, arrays: Any, slots: Sequence[int], battles: Any) -> Dict[str, Any]:
        return {"scores": [[0.0] * N_ACTIONS for _ in slots], "value": [0.0] * len(slots)}


class HashPolicy:
    """score[a] = a number in [0, 1) from a hash of the decision's arrays (mask and decision id left out)
    and a. `stateful`: the slot's number of decisions since its last reset is mixed in, like a model with
    memory. Never ties."""

    def __init__(self, stateful: bool = False, salt: int = 0):
        self.stateful, self.salt = stateful, salt
        self.info = _info("hash%s" % ("-stateful" if stateful else ""))
        self.count: Dict[int, int] = {}
        self.calls = 0

    def reset(self, slots: Sequence[int]) -> None:
        for s in slots:
            self.count[s] = 0

    def observe(self, arrays: Any, slots: Sequence[int], actions: Sequence[Any]) -> None:
        """The memory step of a request answered without asking: the same as `decide` takes."""
        if self.stateful:
            for s in slots:
                self.count[s] = self.count.get(s, 0) + 1
        self.observed = getattr(self, "observed", 0) + len(slots)

    def decide(self, arrays: Any, slots: Sequence[int], battles: Any) -> Dict[str, Any]:
        out, vals = [], []
        for r, s in enumerate(slots):
            h = hashlib.blake2b(b"".join(_row_bytes(arrays, k, r) for k in _KEY_FIELDS), digest_size=16, key=struct.pack("<q", self.salt))
            if self.stateful:
                c = self.count.get(s, 0)
                self.count[s] = c + 1
                h.update(struct.pack("<q", c))
            seed = h.digest()
            row = []
            for a in range(N_ACTIONS):
                d = hashlib.blake2b(seed + bytes([a]), digest_size=8).digest()
                row.append(struct.unpack("<Q", d)[0] / 2.0**64)
            out.append(row)
            vals.append(row[0] * 2 - 1)
            self.calls += 1
        return {"scores": out, "value": vals, "extras": [{"slot": s} for s in slots]}


class FaultyPolicy:
    """`fault`: "nan" (every score NaN), "shape" (rows of 5), "rows" (one row too few), "raise",
    "slow" (sleeps `delay` s, then answers like HashPolicy), "illegal" (+1e9 on illegal actions, -inf on
    legal ones), "illegal_high" (+1e9 on illegal actions, legal ones finite), "info" (a wrong layout hash).
    `every`: fail only on every n-th call (others answer like HashPolicy)."""

    def __init__(self, fault: str, delay: float = 0.0, every: int = 1):
        self.fault, self.delay, self.every = fault, delay, max(1, every)
        self.inner = HashPolicy()
        self.info = _info("faulty-" + fault)
        if fault == "info":
            self.info["layout_hash"] = "0" * 64
        self.calls = 0

    def decide(self, arrays: Any, slots: Sequence[int], battles: Any) -> Dict[str, Any]:
        self.calls += 1
        good = self.inner.decide(arrays, slots, battles)
        if self.calls % self.every:
            return good
        f = self.fault
        n = len(slots)
        if f == "nan":
            return {"scores": [[float("nan")] * N_ACTIONS for _ in range(n)]}
        if f == "shape":
            return {"scores": [[0.0] * 5 for _ in range(n)]}
        if f == "rows":
            return {"scores": good["scores"][:-1]}
        if f == "raise":
            raise RuntimeError("faulty policy")
        if f == "slow":
            time.sleep(self.delay)
            return good
        if f in ("illegal", "illegal_high"):
            out = []
            for r in range(n):
                m = memoryview(arrays.action_mask).cast("B")[r * N_ACTIONS : (r + 1) * N_ACTIONS]
                out.append([(good["scores"][r][a] if f == "illegal_high" else float("-inf")) if m[a] else 1e9 for a in range(N_ACTIONS)])
            return {"scores": out}
        return good


class CountingPolicy:
    """Wraps a policy and records each call's slots and decision ids."""

    def __init__(self, inner: Any):
        self.inner = inner
        self.info = getattr(inner, "info")
        self.calls: List[List[int]] = []
        self.observed: List[List[int]] = []

    def observe(self, arrays: Any, slots: Sequence[int], actions: Sequence[Any]) -> None:
        self.observed.append([int(arrays.decision[r]) for r in range(len(slots))])
        o = getattr(self.inner, "observe", None)
        if callable(o):
            o(arrays, slots, actions)

    def reset(self, slots: Sequence[int]) -> None:
        r = getattr(self.inner, "reset", None)
        if callable(r):
            r(slots)

    def decide(self, arrays: Any, slots: Sequence[int], battles: Any) -> Dict[str, Any]:
        self.calls.append([int(arrays.decision[r]) for r in range(len(slots))])
        return self.inner.decide(arrays, slots, battles)


def make(kind: str = "uniform", **args: Any) -> Any:
    """A stand-in by name (for configs): builtin, uniform, hash, faulty."""
    return {"builtin": BuiltinPolicy, "uniform": UniformPolicy, "hash": HashPolicy, "faulty": FaultyPolicy}[kind](**args)
