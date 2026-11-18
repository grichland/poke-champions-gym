"""A neural-network player: CG's shell around any model that turns observation arrays into action scores.

    import champions_gym as cg
    from champions_gym.neural import NeuralPlayer, evaluate, load
    p = load(policy="mypkg.cg_policy:load", checkpoint="runs/x/iter-00092.pt", device="cpu")
    cg.play_games(p, "tower", 100)                      # a Player like any other
    Session(player="champions_gym.neural:load", player_args={"policy": "mypkg.cg_policy:load", ...})
    r = evaluate(p.policy, opponent="tower", games=2000) # the same choice code, batched over a BatchEnv

The policy protocol (docs/neural-player.md):

    policy.info -> dict (or a method returning it): "contract" (MODEL.md version), "layout_hash",
        "shapes", "vocab" it was trained with; optional "model_id", "model_hash", "requery_refused".
    policy.decide(arrays, slots, battles) -> {"scores": [B][71], "value": [B] (optional),
        "extras": [B] dicts (optional)}
        arrays: a `champions_gym.Tensors` with a leading batch axis B (what `BatchEnv.observe_tensors()`
        gives), kind per row in arrays.kind. slots: B ints, one per game history (a model with memory
        keeps one history per slot). battles: B Battles, read on access (may be ignored).
        scores: logits in the one action space of 71 (0..10 for turn/replace rows, 11..70 for preview
        rows); a row may also be 11 wide (turn/replace) or 60 wide (preview). Higher is better; -inf is
        "never"; entries outside the legal mask are ignored whatever they hold.
    policy.reset(slots) (optional): a new game starts in these slots.

The shell applies the legal mask itself, takes the argmax (ties: lowest action) or samples
softmax(scores / temperature), and falls back to a built-in player when the policy fails. No numpy or
torch is needed here; numpy is used when installed, as everywhere in the package.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import math
import os
import random
import re
import struct
import sys
import threading
import time
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional, Sequence

from . import _cg
from .env import KIND_NAMES, KIND_PREVIEW, KIND_REPLACE, KIND_TURN, N_ACTIONS, N_TURN_ACTIONS, PREVIEW0, BatchEnv, mix_seed, preview_picks
from .obs import DTYPES, FEATURES, MOVE_STRIDE, SHAPES, VOCAB, Tensors, new_tensors, tensors
from .player import BuiltinPlayer, Player

try:
    import numpy as np
except ImportError:
    np = None

MODEL_CONTRACT = "0.6"  # contracts/MODEL.md section 7
INPUT_BLOCKS = ("rows", "row_ids", "field", "last", "last_ids")
N_PREVIEW = N_ACTIONS - PREVIEW0
# fields that describe the request, not the Battle: they differ between a turn and its replace request,
# and with the mask after a refusal
_REQUEST_FIELDS = ("G1-2.1", "G1-7.1", "G1-7.2")
_KEY_FIELDS = ("rows", "row_ids", "field", "last", "last_ids", "opp_prior", "kind", "seat")
_STATE_FIELDS = ("rows", "row_ids", "field", "last", "last_ids", "opp_prior")
_FMT = {"float32": "f", "int32": "i", "uint8": "B", "int64": "q"}


class ModelMismatch(RuntimeError):
    """The policy was trained on another contract, feature layout, shape or vocabulary."""


# ---------------------------------------------------------------------------------------------- self-check


def layout_hash() -> str:
    """sha256 of where every input feature sits: (id, block, first column, size, repeats, stride) sorted,
    as pcsa_former.layout computes it, so a checkpoint's stored hash compares directly."""
    where = []
    for fid, f in FEATURES.items():
        if f.block not in INPUT_BLOCKS:
            continue
        first = int(f.columns()[0])
        if f.slot:
            where.append((fid, f.block, first, int(f.size), 4, int(MOVE_STRIDE)))
        else:
            where.append((fid, f.block, first, int(f.size), int(f.repeats), 0))
    return hashlib.sha256(repr(sorted(where)).encode()).hexdigest()


def contract() -> Dict[str, Any]:
    """What this CG build gives a model: the policy's `info` must agree with it."""
    return {
        "contract": MODEL_CONTRACT,
        "layout_hash": layout_hash(),
        "shapes": {k: list(v) for k, v in SHAPES.items()},
        "vocab": dict(VOCAB),
    }


def check_info(info: Dict[str, Any]) -> List[str]:
    """The differences between a policy's `info` and this build (empty when they agree)."""
    want = contract()
    out = []
    for key in ("contract", "layout_hash"):
        if str(info.get(key)) != want[key]:
            out.append("%s: model %r, CG %r" % (key, info.get(key), want[key]))
    shapes = info.get("shapes")
    if not isinstance(shapes, dict):
        out.append("shapes: missing")
    else:
        for k, v in shapes.items():
            if k not in want["shapes"] or list(v) != want["shapes"][k]:
                out.append("shape %s: model %r, CG %r" % (k, v, want["shapes"].get(k)))
        for k in INPUT_BLOCKS:
            if k not in shapes:
                out.append("shape %s: missing" % k)
    vocab = info.get("vocab")
    if not isinstance(vocab, dict) or {k: int(v) for k, v in vocab.items()} != want["vocab"]:
        out.append("vocab: model %r, CG %r" % (vocab, want["vocab"]))
    return out


def _info(policy: Any) -> Dict[str, Any]:
    i = getattr(policy, "info", None)
    if callable(i):
        i = i()
    if not isinstance(i, dict):
        raise ModelMismatch("policy has no info dict (contract, layout_hash, shapes, vocab)")
    return dict(i)


# ---------------------------------------------------------------------------------------------- arrays


def _bytes(x: Any) -> bytes:
    if isinstance(x, (int, bool)):
        raise TypeError("scalar")
    return x.tobytes()


def _scalar(name: str, v: int) -> bytes:
    return struct.pack("<" + _FMT[DTYPES[name]], int(v))


def _writable(x: Any) -> memoryview:
    return memoryview(x).cast("B")


def stack(items: Sequence[Tensors]) -> Tensors:
    """Single-decision `Tensors` (what `cg.tensors(battle)` returns) as one batch [B, ...], of the same
    array type as `BatchEnv.observe_tensors()` (numpy when installed, else memoryviews)."""
    out = new_tensors(len(items))
    for name in Tensors._fields:
        dst = _writable(getattr(out, name))
        size = len(dst) // max(len(items), 1)
        for r, t in enumerate(items):
            v = getattr(t, name)
            b = _scalar(name, v) if isinstance(v, int) else _bytes(v)
            if len(b) != size:
                raise ValueError("%s: %d bytes, want %d" % (name, len(b), size))
            dst[r * size : (r + 1) * size] = b
    return out


def take(batch: Tensors, idx: Sequence[int]) -> Tensors:
    """Rows `idx` of a batch, as a new batch."""
    idx = list(idx)
    if np is not None and isinstance(batch.rows, np.ndarray):
        ix = np.asarray(idx, dtype=np.int64)
        return Tensors(*(np.ascontiguousarray(getattr(batch, k)[ix]) for k in Tensors._fields))
    out = new_tensors(len(idx))
    for name in Tensors._fields:
        src = memoryview(getattr(batch, name)).cast("B")
        dst = _writable(getattr(out, name))
        size = len(src) // len(getattr(batch, "kind"))
        for r, g in enumerate(idx):
            dst[r * size : (r + 1) * size] = src[g * size : (g + 1) * size]
    return out


def _row_bytes(batch: Tensors, name: str, r: int) -> bytes:
    src = memoryview(getattr(batch, name)).cast("B")
    size = len(src) // len(batch.kind)
    return bytes(src[r * size : (r + 1) * size])


def _row_list(x: Any, r: int, width: int) -> List[int]:
    src = memoryview(x).cast("B")
    return list(src[r * width : (r + 1) * width])


def _field_cols(ids: Sequence[str]) -> List[int]:
    return sorted(c for i in ids for c in FEATURES[i].columns())


_REQ_COLS = _field_cols(_REQUEST_FIELDS)


def _state_key(batch: Tensors, r: int) -> bytes:
    """The Battle's arrays without the request-dependent parts (mask, decision id, request fields)."""
    parts = []
    for name in _STATE_FIELDS:
        b = _row_bytes(batch, name, r)
        if name == "field":
            m = bytearray(b)
            for c in _REQ_COLS:
                m[4 * c : 4 * c + 4] = b"\0\0\0\0"
            b = bytes(m)
        parts.append(b)
    return hashlib.blake2b(b"".join(parts), digest_size=16).digest()


def _cache_key(batch: Tensors, r: int) -> bytes:
    """The state key plus the request's kind and seat: equal when only the mask (a refusal) changed."""
    return _state_key(batch, r) + _row_bytes(batch, "kind", r) + _row_bytes(batch, "seat", r)


# ---------------------------------------------------------------------------------------------- choosing


def _rows_of(x: Any, n: int, what: str) -> List[Any]:
    if hasattr(x, "detach"):
        x = x.detach().cpu()
    if hasattr(x, "tolist"):
        x = x.tolist()
    x = list(x)
    if len(x) != n:
        raise ValueError("%s: %d rows for %d decisions" % (what, len(x), n))
    return x


def _full_row(row: Any, kind: int) -> List[float]:
    """A policy's row as 71 floats in the one action space."""
    row = [float(v) for v in row]
    if len(row) == N_ACTIONS:
        return row
    if len(row) == N_TURN_ACTIONS and kind != KIND_PREVIEW:
        return row + [float("-inf")] * (N_ACTIONS - N_TURN_ACTIONS)
    if len(row) == N_PREVIEW and kind == KIND_PREVIEW:
        return [float("-inf")] * PREVIEW0 + row
    raise ValueError("a %s row of %d scores (want 71, or %d)" % (KIND_NAMES.get(kind, kind), len(row), N_PREVIEW if kind == KIND_PREVIEW else N_TURN_ACTIONS))


def select(scores: Sequence[float], legal: Sequence[int], mode: str = "argmax", rng: Optional[random.Random] = None, temperature: float = 1.0):
    """(action, reason): the legal action with the highest score, lowest action on ties ("argmax"), or one
    drawn from softmax(score / temperature) over the legal actions ("sample"). Only legal entries are read.
    NaN or +inf on a legal action, or no finite legal score, gives (None, reason)."""
    if len(legal) == 1:
        return legal[0], None
    best = None
    finite = []
    for a in legal:
        v = scores[a]
        if v != v or v == float("inf"):
            return None, "non-finite score on legal action %d" % a
        if v == float("-inf"):
            continue
        finite.append((a, v))
        if best is None or v > best[1]:
            best = (a, v)
    if best is None:
        return None, "no finite score on a legal action"
    if mode == "argmax":
        return best[0], None
    t = max(float(temperature), 1e-6)
    w = [math.exp((v - best[1]) / t) for _, v in finite]
    u = (rng or random).random() * sum(w)
    acc = 0.0
    for (a, _), x in zip(finite, w):
        acc += x
        if u < acc:
            return a, None
    return finite[-1][0], None


def _r(x: Any, nd: int = 4) -> Any:
    if isinstance(x, float):
        return round(x, nd) if math.isfinite(x) else None
    if isinstance(x, dict):
        return {k: _r(v, nd) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [_r(v, nd) for v in x]
    if hasattr(x, "tolist"):
        return _r(x.tolist(), nd)
    return x


# ---------------------------------------------------------------------------------------------- the shell


@dataclass
class _Slot:
    """One game history (a seat of one game)."""

    calls: int = 0  # decisions asked in this battle
    last_decision: Optional[int] = None
    last_turn: int = -1
    key: Optional[bytes] = None  # the last decision's arrays (cache)
    legal: Optional[List[int]] = None
    scores: Optional[List[float]] = None
    value: Optional[float] = None
    extras: Any = None
    turn_state: Optional[bytes] = None  # the last turn request's Battle, for the replace check
    slow_run: int = 0
    error_run: int = 0
    disabled: Optional[str] = None  # the fallback plays the rest of the battle, and why
    fallback: Any = None
    log: Any = None
    battle_name: Optional[str] = None


@dataclass
class Choice:
    action: int
    record: Dict[str, Any]


@dataclass
class Stats:
    decisions: int = 0
    policy_calls: int = 0
    cached: int = 0
    fallbacks: int = 0
    slow: int = 0
    observed: int = 0  # requests answered without asking (replayed), given to the policy as memory steps
    replace_stale: int = 0
    replace_fresh: int = 0
    mask_mismatch: int = 0
    policy_ms: float = 0.0
    reasons: Dict[str, int] = field(default_factory=dict)
    ms: List[float] = field(default_factory=list)  # each policy call's wall time

    def inc(self, name: str, k: Any = 1) -> None:
        setattr(self, name, getattr(self, name) + k)

    def summary(self) -> Dict[str, Any]:
        ms = sorted(self.ms)
        return {
            "decisions": self.decisions, "policy_calls": self.policy_calls, "fallbacks": self.fallbacks,
            "fallback_reasons": dict(self.reasons), "slow": self.slow, "reused": self.cached, "observed": self.observed,
            "ms_median": round(ms[len(ms) // 2], 2) if ms else None, "ms_max": round(ms[-1], 2) if ms else None,
        }  # fmt: skip


class _Both:
    """The total and the current battle's Stats, counted together."""

    def __init__(self, a: Stats, b: Stats):
        self.a, self.b = a, b

    def inc(self, name: str, k: Any = 1) -> None:
        self.a.inc(name, k)
        self.b.inc(name, k)

    def reason(self, r: str) -> None:
        for s in (self.a, self.b):
            s.reasons[r] = s.reasons.get(r, 0) + 1

    def ms(self, v: float) -> None:
        self.a.ms.append(v)
        self.b.ms.append(v)


class Shell:
    """The choice logic shared by `NeuralPlayer` (one Battle at a time) and `evaluate` (a BatchEnv)."""

    def __init__(self, policy: Any, mode: str = "argmax", temperature: float = 1.0, seed: int = 0, fallback: Optional[str] = "tower",
                 timeout_s: Optional[float] = 2.0, hard_timeout_s: Optional[float] = 6.0, slow_limit: int = 3, max_errors: int = 3,
                 log_extras: bool = True, allow_mismatch: bool = False):  # fmt: skip
        if mode not in ("argmax", "sample"):
            raise ValueError("mode: 'argmax' or 'sample'")
        if not callable(getattr(policy, "decide", None)):
            raise TypeError("policy needs decide(arrays, slots, battles)")
        if fallback is not None and fallback not in _cg.BUILTIN_PLAYERS:
            raise ValueError("fallback %r is not a built-in player (%s)" % (fallback, ", ".join(_cg.BUILTIN_PLAYERS)))
        self.policy = policy
        self.info = _info(policy)
        self.mismatch = check_info(self.info)
        if self.mismatch and not allow_mismatch:
            raise ModelMismatch("policy does not match this CG build: " + "; ".join(self.mismatch))
        self.mode, self.temperature, self.seed = mode, float(temperature), int(seed)
        self.fallback_name = fallback
        self.timeout_s, self.hard_timeout_s = timeout_s, hard_timeout_s
        self.slow_limit, self.max_errors = int(slow_limit), int(max_errors)
        self.log_extras = log_extras
        self.requery_refused = bool(self.info.get("requery_refused", False))
        self.model = {"id": self.info.get("model_id"), "hash": self.info.get("model_hash")}
        self.slots: Dict[int, _Slot] = {}
        self.stats = Stats()
        self.battle_stats = Stats()
        self._count = _Both(self.stats, self.battle_stats)

    def slot(self, s: int) -> _Slot:
        st = self.slots.get(s)
        if st is None:
            st = self.slots[s] = _Slot()
        return st

    def begin(self, slots: Sequence[int]) -> None:
        """New games in these slots: their state is cleared and the policy is told."""
        for s in slots:
            old = self.slots.get(s)
            self.slots[s] = _Slot(log=old.log if old else None, battle_name=old.battle_name if old else None)
        reset = getattr(self.policy, "reset", None)
        if callable(reset) and slots:
            reset(list(slots))

    def _fallback(self, s: int, battle: Any, legal: List[int]) -> int:
        if self.fallback_name is None or battle is None:
            return legal[0]
        st = self.slot(s)
        if st.fallback is None:
            st.fallback = BuiltinPlayer(self.fallback_name, mix_seed(self.seed, s))
        try:
            a = st.fallback.action_for(battle)
        except Exception:  # the fallback must not fail either
            return legal[0]
        return a if a in legal else legal[0]

    def decide(self, batch: Tensors, slots: Sequence[int], battles: Any = None, turns: Optional[Sequence[int]] = None) -> List[Choice]:
        """One choice per row of `batch`: `slots[r]` names its game history; `battles[r]` (read only when the
        policy or the fallback needs it) its Battle; `turns[r]` its turn number for the log."""
        n = len(slots)
        kinds = [int(batch.kind[r]) for r in range(n)]
        legal = []
        for r in range(n):
            m = _row_list(batch.action_mask, r, N_ACTIONS)
            legal.append([a for a in range(N_ACTIONS) if m[a]])
        recs: List[Dict[str, Any]] = [{} for _ in range(n)]
        scores: List[Optional[List[float]]] = [None] * n
        values: List[Optional[float]] = [None] * n
        extras: List[Any] = [None] * n
        keys = [_cache_key(batch, r) for r in range(n)]
        ask = []
        for r in range(n):
            st = self.slot(slots[r])
            k = kinds[r]
            rec = recs[r]
            if k == KIND_REPLACE:
                if st.turn_state is not None:
                    stale = _state_key(batch, r) == st.turn_state
                    rec["replace"] = "stale" if stale else "fresh"
                    if stale:
                        self._count.inc("replace_stale")
                    else:
                        self._count.inc("replace_fresh")
            elif k == KIND_TURN:
                st.turn_state = _state_key(batch, r)
            if not legal[r]:
                rec["fallback"] = "no legal action"
                continue
            if st.disabled:
                rec["fallback"] = st.disabled
                continue
            if (not self.requery_refused and st.key == keys[r] and st.legal is not None and st.scores is not None
                    and set(legal[r]) <= set(st.legal)):  # fmt: skip
                scores[r], values[r], extras[r] = st.scores, st.value, st.extras
                rec["cached"] = True
                self._count.inc("cached")
                continue
            ask.append(r)
        ms = None
        if ask:
            sub = batch if len(ask) == n else take(batch, ask)
            sub_battles = _Pick(battles, ask)
            t0 = time.perf_counter()
            try:
                out = self.policy.decide(sub, [slots[r] for r in ask], sub_battles)
                ms = (time.perf_counter() - t0) * 1000.0
                got = _rows_of(out["scores"], len(ask), "scores")
                val = out.get("value")
                val = _rows_of(val, len(ask), "value") if val is not None else [None] * len(ask)
                ext = out.get("extras")
                ext = _rows_of(ext, len(ask), "extras") if ext is not None else [None] * len(ask)
                for j, r in enumerate(ask):
                    try:
                        scores[r] = _full_row(got[j], kinds[r])
                    except (TypeError, ValueError) as e:
                        recs[r]["fallback"] = "bad output: %s" % e
                    values[r] = None if val[j] is None else float(val[j])
                    extras[r] = ext[j]
                err = None
            except Exception as e:  # any failure of the policy: the fallback answers
                ms = (time.perf_counter() - t0) * 1000.0
                err = "policy raised %s: %s" % (type(e).__name__, e)
                for r in ask:
                    recs[r]["fallback"] = err
            self._count.inc("policy_calls")
            self._count.inc("policy_ms", ms)
            self._count.ms(ms)
            for r in ask:
                st = self.slot(slots[r])
                recs[r]["ms"] = round(ms, 2)
                st.error_run = st.error_run + 1 if err else 0
                if err and st.error_run >= self.max_errors:
                    st.disabled = "policy failed %d times in a row" % st.error_run
                slow = self.timeout_s is not None and ms > 1000.0 * self.timeout_s
                if slow:
                    recs[r]["slow"] = True
                    self._count.inc("slow")
                st.slow_run = st.slow_run + 1 if slow else 0
                if self.hard_timeout_s is not None and ms > 1000.0 * self.hard_timeout_s:
                    st.disabled = "policy took %.0f ms (hard limit %.0f ms)" % (ms, 1000.0 * self.hard_timeout_s)
                elif slow and st.slow_run >= self.slow_limit:
                    st.disabled = "policy slower than %.1f s %d times in a row" % (self.timeout_s, st.slow_run)
        out_choices = []
        for r in range(n):
            s = slots[r]
            st = self.slot(s)
            rec = recs[r]
            action = None
            if scores[r] is not None and "fallback" not in rec:
                rng = random.Random(mix_seed(mix_seed(self.seed, s), st.calls)) if self.mode == "sample" else None
                action, why = select(scores[r], legal[r], self.mode, rng, self.temperature)
                if why:
                    rec["fallback"] = why
            if action is None:
                if legal[r]:
                    action = self._fallback(s, battles[r] if battles is not None else None, legal[r])
                else:
                    action = 0 if kinds[r] != KIND_PREVIEW else PREVIEW0
                rec["fallback"] = {"reason": rec.get("fallback", "unknown"), "player": self.fallback_name}
                self._count.inc("fallbacks")
                reason = re.sub(r"\d+(\.\d+)?", "N", str(rec["fallback"]["reason"]))[:80]
                self._count.reason(reason)
            st.calls += 1
            st.key, st.legal = keys[r], legal[r]
            st.scores, st.value, st.extras = scores[r], values[r], extras[r]
            self._count.inc("decisions")
            k = kinds[r]
            lo, hi = (PREVIEW0, N_ACTIONS) if k == KIND_PREVIEW else (0, N_TURN_ACTIONS)
            full = {
                "turn": None if turns is None else turns[r],
                "decision": int(batch.decision[r]),
                "kind": KIND_NAMES.get(k, k),
                "legal": legal[r],
                "scores": _r(scores[r][lo:hi]) if scores[r] is not None else None,
                "action": action,
                "value": _r(values[r]),
                "ms": rec.get("ms"),
                "cached": rec.get("cached", False),
                "slow": rec.get("slow", False),
                "fallback": rec.get("fallback"),
                "model": self.model,
            }
            if "replace" in rec:
                full["replace"] = rec["replace"]
            if self.log_extras and extras[r] is not None:
                full["extras"] = _r(extras[r])
            out_choices.append(Choice(action, full))
        return out_choices


def _shell_observe(self: Shell, batch: Tensors, slots: Sequence[int], actions: Sequence[Optional[int]]) -> List[bool]:
    """Requests answered without asking: the policy takes its memory step (`policy.observe`) for each, unless
    the slot's last request had the same Battle and a larger mask (a refusal: no step when asked either).
    Returns per row whether a step was taken."""
    n = len(slots)
    out = []
    ask = []
    for r in range(n):
        st = self.slot(slots[r])
        k = int(batch.kind[r])
        m = _row_list(batch.action_mask, r, N_ACTIONS)
        legal = [a for a in range(N_ACTIONS) if m[a]]
        key = _cache_key(batch, r)
        same = st.key == key and st.legal is not None and set(legal) <= set(st.legal)
        if k == KIND_TURN:
            st.turn_state = _state_key(batch, r)
        st.key, st.legal = key, legal
        st.scores, st.value, st.extras = None, None, None
        st.calls += 1
        out.append(not same)
        if not same:
            ask.append(r)
    f = getattr(self.policy, "observe", None)
    if ask and callable(f):
        sub = batch if len(ask) == n else take(batch, ask)
        f(sub, [slots[r] for r in ask], [actions[r] for r in ask])
    self._count.inc("observed", len(ask))
    return out


Shell.observe = _shell_observe


class _Pick:
    """battles[idx[j]] on access."""

    def __init__(self, battles: Any, idx: List[int]):
        self.b, self.idx = battles, idx

    def __len__(self) -> int:
        return len(self.idx)

    def __getitem__(self, j: int) -> Any:
        if self.b is None:
            raise LookupError("no Battle given for this decision")
        return self.b[self.idx[j]]


class _One:
    def __init__(self, battle: Any):
        self.battle = battle

    def __len__(self) -> int:
        return 1

    def __getitem__(self, j: int) -> Any:
        return self.battle


def _order(battle: Any, action: int) -> Optional[Dict[str, Any]]:
    """What the action is on the screen (the Session sends the same)."""
    try:
        if action >= PREVIEW0:
            return {"picks": list(preview_picks(action))}
        if action < 8:
            return {"kind": "move", "move": battle.available_moves[action % 4], "mega": action >= 4}
        return {"kind": "switch", "mon": battle.available_switches[action - 8]}
    except Exception:
        return None


# ---------------------------------------------------------------------------------------------- the player


_ALLOC_LOCK = threading.Lock()
_ALLOC: Dict[int, Any] = {}  # id(policy) -> (policy, {key: slot}): slots of one policy object are never shared


def _alloc_slot(policy: Any, key: Any) -> int:
    """A slot of `policy` for `key` (a player, game and seat): the same key gets the same slot, two keys never
    share one, and slots stay small (a policy with memory sizes its store by the largest slot)."""
    with _ALLOC_LOCK:
        ent = _ALLOC.get(id(policy))
        if ent is None or ent[0] is not policy:
            ent = _ALLOC[id(policy)] = (policy, {})
        m = ent[1]
        if key not in m:
            used = set(m.values())
            m[key] = next(i for i in range(len(m) + 1) if i not in used)
        return m[key]


def _free_slots(policy: Any, owner: Any) -> None:
    """Gives back the slots whose key starts with `owner`."""
    with _ALLOC_LOCK:
        ent = _ALLOC.get(id(policy))
        if ent is not None and ent[0] is policy:
            for k in [k for k in ent[1] if isinstance(k, tuple) and k and k[0] == owner]:
                del ent[1][k]


class NeuralPlayer(Player):
    """A `cg.Player` that asks a policy (module docstring). Usable in `Session(player=...)`, `play_games`,
    and anywhere a Player object is accepted.

    `mode` "argmax" (default; a pure function of the Battles seen and the checkpoint) or "sample"
    (softmax(scores / `temperature`), drawn from `seed`, the slot and the decision's number in the game).
    `fallback`: a built-in player that answers when the policy raises, returns a wrong shape, NaN or +inf
    on a legal action, or no finite legal score. Time: a call over `timeout_s` is used and logged as slow;
    `slow_limit` slow calls in a row, or one over `hard_timeout_s`, or `max_errors` failures in a row hand
    the rest of that battle to the fallback. `log_dir`: one JSON line per decision, a file per battle
    (`<tag>.neural.jsonl` under a Session). The policy's `info` is checked against this CG build at
    construction (ModelMismatch unless `allow_mismatch`), then one warm-up preview and turn decision run.

    Session hooks (LIVE.md section 3): `new_battle(tag, resend)`, `battle_end(result)`,
    `observe(battle, action, how)` for a request answered without asking (a restart's replay), `status()`
    and `decision_notes()`. Outside a Session (`play_games`) a new battle is detected from a preview request
    or a step back in the decision number or turn. One NeuralPlayer serves one battle at a time; its methods
    take a lock, and two NeuralPlayers never share a slot of the same policy object.
    """

    def __init__(self, policy: Any, mode: str = "argmax", temperature: float = 1.0, seed: int = 0, fallback: Optional[str] = "tower",
                 timeout_s: Optional[float] = 2.0, hard_timeout_s: Optional[float] = 6.0, slow_limit: int = 3, max_errors: int = 3,
                 log_dir: Optional[str] = None, log_extras: bool = True, allow_mismatch: bool = False, warmup: bool = True,
                 name: Optional[str] = None):  # fmt: skip
        self.shell = Shell(policy, mode, temperature, seed, fallback, timeout_s, hard_timeout_s, slow_limit, max_errors, log_extras, allow_mismatch)
        self.policy = policy
        self.info = self.shell.info
        self.allow_mismatch = allow_mismatch
        self.log_dir = log_dir
        mid = self.info.get("model_id")
        self.name = name or ("neural:%s" % os.path.basename(str(mid)) if mid else "neural")
        self.last: Optional[Dict[str, Any]] = None
        self.warmup_ms: Optional[List[float]] = None
        self._lock = threading.RLock()
        self._battles = 0
        self._tag: Optional[str] = None
        self._session = False  # told of battles by a Session: no guessing
        self._resent = False
        self._seen: Dict[int, set] = {}  # slot -> decision numbers that took a memory step this battle
        if self.shell.mismatch:
            print("champions_gym.neural: running despite: %s" % "; ".join(self.shell.mismatch), file=sys.stderr)
        if warmup:
            self.warmup_ms = warm_up(self.shell, _alloc_slot(policy, (id(self), "warmup")))
            _free_slots(policy, id(self))

    @property
    def stats(self) -> Stats:
        return self.shell.stats

    @staticmethod
    def from_path(policy: str, **args: Any) -> "NeuralPlayer":
        return load(policy, **args)

    # the battle -------------------------------------------------------------------------------------------
    def new_battle(self, tag: Optional[str] = None, resend: bool = False) -> None:
        """A battle starts (Session hook). `resend`: the controller sends the same battle again from
        `battle_start` (a restart): the memory is cleared and rebuilt from the replayed requests (`observe`),
        the log is appended to, and a battle handed to the fallback stays so. Otherwise a new battle."""
        with self._lock:
            self._session = True
            resend = bool(resend) and tag == self._tag
            if not resend:
                self._end_logs()
                self.shell.battle_stats.__init__()
            handed = {s: st.disabled for s, st in self.shell.slots.items()} if resend else {}
            self._tag = tag
            self._resent = resend
            slots = list(self.shell.slots)
            self.shell.begin(slots)
            for s, why in handed.items():
                self.shell.slot(s).disabled = why
            self._seen = {}
            if resend:
                for s in slots:
                    self._header(s, "resend")

    def battle_end(self, result: Optional[str] = None) -> None:
        """The battle ended (Session hook): the record gets an end line and is closed."""
        with self._lock:
            for st in self.shell.slots.values():
                if st.log is not None:
                    self._line(st, {"type": "end", "t": round(time.time(), 3), "result": result, "battle": self.shell.battle_stats.summary()})
            self._end_logs()

    def close(self) -> None:
        with self._lock:
            self._end_logs()

    def _end_logs(self) -> None:
        for st in self.shell.slots.values():
            if st.log is not None:
                try:
                    st.log.close()
                except OSError:
                    pass
            st.log, st.battle_name = None, None

    def _slot_of(self, battle: Any) -> int:
        s = _alloc_slot(self.policy, (id(self), int(battle.game), int(battle.seat)))
        st = self.shell.slots.get(s)
        d, turn = int(battle.decision), int(battle.turn)
        if st is None:
            self.shell.begin([s])
        elif not self._session:
            last = st.last_decision
            new = battle.kind == KIND_PREVIEW or (last is not None and (turn < st.last_turn or (d >= 0 and last >= 0 and d < last)))
            if new and st.calls:
                self._close_slot_log(s)
                self.shell.battle_stats.__init__()
                self._seen.pop(s, None)
                self.shell.begin([s])
        st = self.shell.slot(s)
        st.last_decision, st.last_turn = d, turn
        return s

    def _close_slot_log(self, s: int) -> None:
        st = self.shell.slots.get(s)
        if st is not None and st.log is not None:
            try:
                st.log.close()
            except OSError:
                pass
            st.log, st.battle_name = None, None

    def _header(self, s: int, typ: str) -> None:
        st = self.shell.slot(s)
        if self._open(s, st):
            self._line(st, {"type": typ, "t": round(time.time(), 3), "battle": self._tag, "player": self.name})

    def _open(self, s: int, st: _Slot) -> bool:
        if not self.log_dir:
            return False
        if st.log is not None:
            return True
        try:
            os.makedirs(self.log_dir, exist_ok=True)
            self._battles += 1
            if self._session and self._tag:
                base = re.sub(r"[^A-Za-z0-9._-]", "_", self._tag)
                st.battle_name = base if len(self.shell.slots) <= 1 else "%s-s%d" % (base, s)
            else:
                st.battle_name = "battle-%s-%d-s%d" % (time.strftime("%Y%m%dT%H%M%S"), self._battles, s)
            path = os.path.join(self.log_dir, st.battle_name + ".neural.jsonl")
            fresh = not os.path.exists(path)
            st.log = open(path, "a", encoding="utf-8", newline="\n")
            if fresh or not self._resent:
                head = {"type": "start", "t": round(time.time(), 3), "battle": self._tag, "player": self.name, "model": self.info,
                        "config": {"mode": self.shell.mode, "temperature": self.shell.temperature, "seed": self.shell.seed,
                                   "fallback": self.shell.fallback_name, "timeout_s": self.shell.timeout_s,
                                   "hard_timeout_s": self.shell.hard_timeout_s, "slow_limit": self.shell.slow_limit},
                        "mismatch": self.shell.mismatch, "allow_mismatch": self.allow_mismatch, "warmup_ms": self.warmup_ms}  # fmt: skip
                self._line(st, head)
            return True
        except OSError as e:
            print("champions_gym.neural: log: %s" % e, file=sys.stderr)
            return False

    def _line(self, st: _Slot, rec: Dict[str, Any]) -> None:
        try:
            st.log.write(json.dumps(rec, ensure_ascii=False, default=str) + "\n")
            st.log.flush()
        except (OSError, AttributeError) as e:
            print("champions_gym.neural: log: %s" % e, file=sys.stderr)

    def _write(self, s: int, rec: Dict[str, Any]) -> None:
        st = self.shell.slot(s)
        if self._open(s, st):
            if self._resent:
                rec["after_resend"] = True
            self._line(st, rec)

    # the Player interface ---------------------------------------------------------------------------------
    def action_for(self, battle: Any) -> int:
        with self._lock:
            s = self._slot_of(battle)
            t = tensors(battle)
            rec_extra = {}
            if int(t.kind) != int(battle.kind):
                rec_extra["anomaly"] = "arrays kind %d, request kind %d" % (int(t.kind), int(battle.kind))
            if bytes(t.action_mask) != bytes(battle.mask) or (battle.kind != KIND_PREVIEW and bytes(t.mask) != bytes(battle.mask)[:N_TURN_ACTIONS]):
                rec_extra["anomaly"] = "arrays mask differs from the request's"
                self.shell._count.inc("mask_mismatch")
            c = self.shell.decide(stack([t]), [s], _One(battle), [int(battle.turn)])[0]
            if not c.record.get("cached"):
                self._seen.setdefault(s, set()).add(int(battle.decision))
            a = c.action
            if a not in battle.legal_actions():  # cannot happen: the shell picks from the arrays' mask
                legal = battle.legal_actions()
                a = legal[0] if legal else a
                rec_extra["anomaly"] = "chosen action outside the request's mask"
            rec = dict(c.record, t=round(time.time(), 3), order=_order(battle, a), **rec_extra)
            rec["action"] = a
            self.last = rec
            self._write(s, rec)
            return a

    def teampreview(self, battle: Any) -> List[int]:
        return list(preview_picks(self.action_for(battle)))

    def choose_move(self, battle: Any) -> int:
        return self.action_for(battle)

    def observe(self, battle: Any, action: Optional[int] = None, how: str = "played") -> None:
        """A request answered without asking (Session hook): a restart's replayed request (`how` "played",
        "unknown" without `played`, "given" picks) or one repeated from the Session's record. `battle` is the
        Battle as it was at that request; `action` what was played (None: unknown). The policy takes the
        memory step it would have taken when asked (`policy.observe`), once per request and battle."""
        with self._lock:
            s = self._slot_of(battle)
            d = int(battle.decision)
            seen = self._seen.setdefault(s, set())
            rec = {"type": "observed", "how": how, "t": round(time.time(), 3), "turn": int(battle.turn), "decision": d,
                   "kind": KIND_NAMES.get(int(battle.kind), int(battle.kind)), "action": action}  # fmt: skip
            if d in seen:
                rec["step"] = False  # already in the memory (a repeat within this battle)
            else:
                rec["step"] = self.shell.observe(stack([tensors(battle)]), [s], [action])[0]
                seen.add(d)
            self._write(s, rec)

    # what the controller sees -------------------------------------------------------------------------------
    def decision_notes(self) -> Optional[Dict[str, Any]]:
        """The last decision in short, for the reply's `notes.model`."""
        with self._lock:
            r = self.last
            if r is None:
                return None
            fb = r.get("fallback")
            return {"fallback": fb["reason"] if isinstance(fb, dict) else False, "ms": r.get("ms"), "reused": bool(r.get("cached")),
                    "slow": bool(r.get("slow")), "value": r.get("value")}  # fmt: skip

    def status(self) -> Dict[str, Any]:
        """For the Session's `status` answer."""
        with self._lock:
            handed = [st.disabled for st in self.shell.slots.values() if st.disabled]
            return {
                "name": self.name,
                "model_id": self.info.get("model_id"),
                "model_hash": self.info.get("model_hash"),
                "checkpoint": self.info.get("checkpoint", self.info.get("model_id")),
                "contract": self.info.get("contract"),
                "layout_ok": not self.shell.mismatch,
                "mismatch": list(self.shell.mismatch),
                "allow_mismatch": self.allow_mismatch,
                "fallback_player": self.shell.fallback_name,
                "mode": self.shell.mode,
                "battle": self.shell.battle_stats.summary(),
                "total": self.shell.stats.summary(),
                "handed_to_fallback": handed[0] if handed else False,
                "warmup_ms": self.warmup_ms,
            }


def warm_up(shell: Shell, slot: int = 0) -> List[float]:
    """One preview and one turn decision on a fixed game, through the shell's checks; the slot is cleared
    after. Returns the two times in ms. Raises when the policy fails or returns the wrong shape."""
    env = BatchEnv(None, 1, seed=1, threads=1)
    times = []
    for want in (KIND_PREVIEW, KIND_TURN):
        while int(env.observe()[1][0]) != want:
            env.act(env.builtin_actions("max_damage"))
        t = env.observe_tensors()
        t0 = time.perf_counter()
        out = shell.policy.decide(t, [slot], _One(env.battle(0)))
        times.append(round((time.perf_counter() - t0) * 1000.0, 1))
        row = _full_row(_rows_of(out["scores"], 1, "scores")[0], want)
        legal = [a for a in range(N_ACTIONS) if _row_list(t.action_mask, 0, N_ACTIONS)[a]]
        a, why = select(row, legal)
        if a is None:
            raise ModelMismatch("warm-up %s decision: %s" % (KIND_NAMES[want], why))
    reset = getattr(shell.policy, "reset", None)
    if callable(reset):
        reset([slot])
    return times


# ---------------------------------------------------------------------------------------------- loading


def _resolve(spec: Any) -> Any:
    if not isinstance(spec, str):
        return spec
    if ":" not in spec:
        raise ValueError("policy %r: give 'module:attr'" % spec)
    mod, attr = spec.split(":", 1)
    obj: Any = importlib.import_module(mod)
    for part in attr.split("."):
        obj = getattr(obj, part)
    return obj


def make_policy(policy: Any, checkpoint: Optional[str] = None, device: Optional[str] = None, policy_args: Optional[Dict[str, Any]] = None) -> Any:
    """A policy object from an object, or from "module:attr" (a class or factory called with `checkpoint`,
    `device` when given, and `policy_args`)."""
    obj = _resolve(policy)
    if callable(getattr(obj, "decide", None)) and not isinstance(obj, type):
        return obj
    args = dict(policy_args or {})
    if checkpoint is not None:
        args["checkpoint"] = checkpoint
    if device is not None:
        args["device"] = device
    return obj(**args)


def load(policy: Any, checkpoint: Optional[str] = None, device: Optional[str] = None, policy_args: Optional[Dict[str, Any]] = None, **player: Any) -> NeuralPlayer:
    """A NeuralPlayer from a config: `champions_gym.neural:load` as the Session's `player`, these as
    `player_args`. Every other keyword goes to NeuralPlayer (mode, fallback, timeout_s, log_dir, ...)."""
    return NeuralPlayer(make_policy(policy, checkpoint, device, policy_args), **player)


# ---------------------------------------------------------------------------------------------- evaluation


def wilson(k: float, n: int, z: float = 1.96) -> List[float]:
    """The Wilson score interval of k successes in n."""
    if n <= 0:
        return [0.0, 1.0]
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return [round((c - h) / d, 4), round((c + h) / d, 4)]


@dataclass
class EvalResult:
    opponent: str
    games: int
    wins: int
    losses: int
    ties: int
    truncated: int
    win_rate: float  # (wins + ties / 2) / games
    ci95: List[float]
    by_seat: Dict[str, List[int]]  # seat -> [wins, games]
    mean_turns: float
    decisions: int
    policy_calls: int
    refusals: int  # our orders the game refused (asked again)
    fallbacks: int
    fallback_reasons: Dict[str, int]
    cached: int
    decisions_per_s: float
    policy_ms_per_decision: float
    seconds: float
    stopped_early: bool
    replace_counters: List[int]
    model: Dict[str, Any]
    actions: Optional[List[List[int]]] = None  # [slot, episode, decision, action] per policy decision, with record=True

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def evaluate(policy: Any, opponent: str = "tower", games: int = 10000, seed: int = 0, threads: int = 8, batch: int = 256,
             teams: Optional[str] = None, team_weights: Any = None, team_mode: str = "real", max_steps: int = 1000,
             replace_before_rows: bool = True, mode: str = "argmax", temperature: float = 1.0, fallback: Optional[str] = "tower",
             log_dir: Optional[str] = None, record: bool = False, allow_mismatch: bool = False, time_limit_s: Optional[float] = None,
             policy_args: Optional[Dict[str, Any]] = None) -> EvalResult:  # fmt: skip
    """The policy against built-in player `opponent` over a BatchEnv of `batch` game slots. Slot g plays an
    equal share of `games` (counted games only; the rest of a slot's games are played with its first legal
    action and not counted) in seat g % 2, so seats are balanced and short games are not over-counted.
    Choices go through the same `Shell.decide` as `NeuralPlayer`; time limits are not applied (a batch call
    is not one live decision). `log_dir`: one JSON line per policy decision in one file. `time_limit_s`:
    stop when reached and report the games finished."""
    policy = make_policy(policy, policy_args=policy_args) if isinstance(policy, str) else policy
    shell = Shell(policy, mode, temperature, seed, fallback, None, None, 1, 1 << 30, False, allow_mismatch)
    n = max(1, min(batch, games))
    quota = [games // n + (1 if g < games % n else 0) for g in range(n)]
    seats = [g % 2 for g in range(n)]
    env = BatchEnv(teams, n, seed, team_mode, max_steps, "battle", threads, False, team_weights, seats, replace_before_rows)
    counted = [0] * n
    episode = [0] * n
    res = {"wins": 0, "losses": 0, "ties": 0, "truncated": 0}
    by_seat = {"0": [0, 0], "1": [0, 0]}
    turns: List[int] = []
    refusals = 0
    actions_log: List[List[int]] = [] if record else None
    log = None
    if log_dir:
        os.makedirs(log_dir, exist_ok=True)
        log = open(os.path.join(log_dir, "eval-%s-%s.jsonl" % (opponent, time.strftime("%Y%m%dT%H%M%S"))), "a", encoding="utf-8", newline="\n")
        log.write(json.dumps({"type": "start", "model": shell.info, "opponent": opponent, "games": games, "seed": seed, "batch": n,
                              "replace_before_rows": replace_before_rows, "mode": mode}, default=str) + "\n")  # fmt: skip
    slot_of = [_alloc_slot(policy, (id(shell), g, seats[g])) for g in range(n)]
    shell.begin(slot_of)
    buf = None
    acts = [0] * n
    t0 = time.perf_counter()
    stopped = False

    class _EnvBattles:
        def __getitem__(self, g: int) -> Any:
            return env.battle(g)

    battles = _EnvBattles()
    while any(counted[g] < quota[g] for g in range(n)):
        if time_limit_s is not None and time.perf_counter() - t0 > time_limit_s:
            stopped = True
            break
        buf = env.observe_tensors(out=buf)
        ours = []
        for g in range(n):
            if int(buf.seat[g]) != seats[g]:
                continue
            if counted[g] >= quota[g]:
                acts[g] = _row_list(buf.action_mask, g, N_ACTIONS).index(1)
            else:
                ours.append(g)
        if ours:
            sub = buf if len(ours) == n else take(buf, ours)
            chosen = shell.decide(sub, [slot_of[g] for g in ours], _Pick(battles, ours))
            for g, c in zip(ours, chosen):
                acts[g] = c.action
                if record:
                    actions_log.append([g, episode[g], c.record["decision"], c.action])
                if log is not None:
                    log.write(json.dumps(dict(c.record, game=g, episode=episode[g], seat=seats[g]), default=str) + "\n")
        env.act(acts, opponent=opponent)
        lab = env.labels()
        for g in range(n):
            s = seats[g]
            if lab.valid[g, s] and lab.refused[g, s] and counted[g] < quota[g]:
                refusals += 1
        r = env.results()
        ended = []
        for g in range(n):
            if not r.done[g]:
                continue
            if counted[g] < quota[g]:
                w = int(r.winner[g])
                s = seats[g]
                if r.truncated[g]:
                    res["truncated"] += 1
                if w == s:
                    res["wins"] += 1
                    by_seat[str(s)][0] += 1
                elif w == 1 - s:
                    res["losses"] += 1
                else:
                    res["ties"] += 1
                by_seat[str(s)][1] += 1
                turns.append(int(r.turns[g]))
                counted[g] += 1
            episode[g] += 1
            ended.append(slot_of[g])
        if ended:
            shell.begin(ended)
    secs = time.perf_counter() - t0
    _free_slots(policy, id(shell))
    if log is not None:
        log.close()
    played = res["wins"] + res["losses"] + res["ties"]
    st = shell.stats
    score = res["wins"] + 0.5 * res["ties"]
    return EvalResult(
        opponent=opponent, games=played, wins=res["wins"], losses=res["losses"], ties=res["ties"], truncated=res["truncated"],
        win_rate=round(score / max(played, 1), 4), ci95=wilson(score, played), by_seat=by_seat,
        mean_turns=round(sum(turns) / max(len(turns), 1), 2), decisions=st.decisions, policy_calls=st.policy_calls,
        refusals=refusals, fallbacks=st.fallbacks, fallback_reasons=dict(st.reasons), cached=st.cached,
        decisions_per_s=round(st.decisions / max(secs, 1e-9), 1), policy_ms_per_decision=round(st.policy_ms / max(st.decisions, 1), 3),
        seconds=round(secs, 1), stopped_early=stopped, replace_counters=list(env.replace_counters()), model=shell.model, actions=actions_log,
    )  # fmt: skip


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(prog="python -m champions_gym.neural", description="Evaluate a policy against built-in players.")
    ap.add_argument("--policy", required=True, help="module:attr, a policy class or factory")
    ap.add_argument("--policy-args", default=None, help="JSON keyword arguments for it (checkpoint, device, ...)")
    ap.add_argument("--opponents", default="tower,max_damage")
    ap.add_argument("--games", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--teams", default=None)
    ap.add_argument("--replace-after-rows", action="store_true")
    ap.add_argument("--log-dir", default=None)
    ap.add_argument("--time-limit", type=float, default=None)
    a = ap.parse_args(argv)
    pol = make_policy(a.policy, policy_args=json.loads(a.policy_args) if a.policy_args else None)
    for k, name in enumerate(a.opponents.split(",")):
        r = evaluate(pol, name, a.games, mix_seed(a.seed, k), a.threads, a.batch, a.teams, replace_before_rows=not a.replace_after_rows,
                     log_dir=a.log_dir, time_limit_s=a.time_limit)  # fmt: skip
        print("EVAL " + json.dumps(r.to_dict(), default=str), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
