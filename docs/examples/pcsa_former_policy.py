"""A pcsa-former checkpoint as a `champions_gym.neural` policy. Example for the model side: copy it into
`pcsa_former` (for instance as `pcsa_former/cg_policy.py`) and name it in the controller's config:

    player: "champions_gym.neural:load"
    player_args: {policy: "pcsa_former.cg_policy:PcsaFormerPolicy", checkpoint: "runs/.../iter-00092.pt",
                  device: "cpu", fallback: "tower", log_dir: "..."}

It does what pcsa-former's own evaluation does (tools/evaluate.py:43, model.Net.act): one step of the
network per request with that game's memory (model.Memory), the expectation over the opponent with the
legal mask after it (model.py:317-329, decide.py), and returns log(pi) and log(preview_pi): the shell's
argmax is then the greedy action of decide.choose, and sampling at temperature 1 is the training policy.

A request answered without asking (a restart's replayed requests, LIVE.md section 3) comes to `observe`:
the same forward step on the Battle of that request, so the memory after a restart is the memory the
model had (pcsa-former's memory holds each decision's input tokens only, never the action taken).

Threads: one lock per policy object around every call (the memory store is not safe to touch from two
threads); torch's own thread count is process-wide (`threads`, set at construction).

Needs torch and numpy (the model's own requirements); CG's package does not.
"""

from __future__ import annotations

import hashlib
import os
import threading
from typing import Any, Dict, Optional, Sequence

import numpy as np
import torch

from pcsa_former import cg_adapter
from pcsa_former import checkpoint as ckpt
from pcsa_former import layout as L
from pcsa_former import model

VOCAB_NAMES = {"species": "species", "item": "items", "ability": "abilities", "move": "moves"}  # ours -> CG's


def _tensor(x: Any, device: str) -> torch.Tensor:
    """CG's array (numpy, or a memoryview without numpy) as a tensor, without a copy where possible."""
    if isinstance(x, np.ndarray):
        return torch.from_numpy(np.ascontiguousarray(x)).to(device)
    return torch.from_numpy(np.asarray(x)).to(device)


def _file_hash(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()[:16]


class PcsaFormerPolicy:
    """`checkpoint`: a .pt saved by pcsa_former.checkpoint.save. `device`: "cpu" for live play.
    `threads`: torch CPU threads. `requery_refused`: ask the network again after a refused choice (as
    pcsa-former's training and evaluation do: every request is a step of the memory) instead of the shell
    reusing the first answer's scores. `log_matrix`: add the 11 x 13 matrix to each logged decision.
    `allow_contract`: load a checkpoint made under another contract version (pcsa_former.checkpoint.load
    refuses it otherwise); CG's own check still reports it unless the player has `allow_mismatch`."""

    def __init__(self, checkpoint: str, device: str = "cpu", threads: int = 4, requery_refused: bool = False, log_matrix: bool = False,
                 allow_contract: bool = False):  # fmt: skip
        torch.set_num_threads(int(threads))
        self.device = device
        lay = cg_adapter.layout()  # pcsa-former's layout from champions_gym.FEATURES, checked against CG
        extra = {"allow_contract": True} if allow_contract else {}
        self.net, ck = ckpt.load(checkpoint, lay, cg_adapter.learnset(), device, **extra)  # LayoutMismatch on a difference
        self.net.eval()
        self.memory: Optional[model.Memory] = None
        self.lock = threading.Lock()
        self.log_matrix = log_matrix
        lt = ck["layout"]
        w = lt["widths"]
        self.info: Dict[str, Any] = {
            "contract": lt.get("contract"),
            "layout_hash": lt["hash"],
            "shapes": {"rows": [L.ROWS, w["rows"]], "row_ids": [L.ROWS, w["row_ids"]], "field": [w["field"]], "last": [w["last"]],
                       "last_ids": [w["last_ids"]], "mask": [L.OWN_ACTIONS], "opp_prior": [L.OPP_ACTIONS]},  # fmt: skip
            "vocab": {VOCAB_NAMES[k]: int(v) for k, v in lt["vocab"].items()},
            "model_id": os.path.abspath(checkpoint),
            "checkpoint": os.path.abspath(checkpoint),
            "model_hash": _file_hash(checkpoint),
            "requery_refused": bool(requery_refused),
            "run": ck.get("extra", {}).get("run"),
            "iteration": ck.get("extra", {}).get("iteration"),
            "memory": self.net.cfg.memory,
        }

    def _grow(self, need: int) -> model.Memory:
        m = self.memory
        if m is None or m.hist.shape[0] < need:
            new = model.Memory(self.net, max(need, 2 * (m.hist.shape[0] if m is not None else 1)), self.device)
            if m is not None:
                k = m.hist.shape[0]
                new.hist[:k], new.pos[:k], new.count[:k] = m.hist, m.pos, m.count
            self.memory = m = new
        return m

    def reset(self, slots: Sequence[int]) -> None:
        with self.lock:
            if self.memory is not None:
                ok = [s for s in slots if s < self.memory.hist.shape[0]]
                if ok:
                    self.memory.reset(torch.as_tensor(ok, dtype=torch.long, device=self.device))

    def _forward(self, arrays: Any, slots: Sequence[int]) -> dict:
        """One step of each slot's game: the forward pass, which also adds this decision to the memory."""
        b = {k: _tensor(getattr(arrays, k), self.device) for k in ("rows", "row_ids", "field", "last", "last_ids", "mask", "opp_prior")}
        b["preview_mask"] = _tensor(arrays.action_mask, self.device)[:, L.PREVIEW_OFFSET :]
        mem = self._grow(max(slots) + 1)
        s = torch.as_tensor(list(slots), dtype=torch.long, device=self.device)
        return self.net.forward(b, memory=mem, slots=s, beliefs=False)

    @torch.no_grad()
    def observe(self, arrays: Any, slots: Sequence[int], actions: Sequence[Optional[int]] = ()) -> None:
        """Requests answered without asking: the memory step only (the action does not enter the memory)."""
        with self.lock:
            self._forward(arrays, slots)

    @torch.no_grad()
    def decide(self, arrays: Any, slots: Sequence[int], battles: Any = None) -> Dict[str, Any]:
        with self.lock:
            out = self._forward(arrays, slots)
        scores = torch.cat([decide_log(out["pi"]), decide_log(out["preview_pi"])], dim=1)  # [B, 71]
        kind = _tensor(arrays.kind, self.device)
        extras = []
        p_opp, score, matrix = out["p_opp"].cpu(), out["score"].cpu(), out["matrix"].cpu()
        for r in range(len(slots)):
            if int(kind[r]) == L.KIND_PREVIEW:
                extras.append(None)
                continue
            e = {"score": score[r].tolist(), "p_opp": p_opp[r].tolist()}
            if self.log_matrix:
                e["matrix"] = matrix[r].tolist()
            extras.append(e)
        return {"scores": scores.cpu(), "value": out["value"].cpu(), "extras": extras}


def decide_log(p: torch.Tensor) -> torch.Tensor:
    """log p with -inf where p is 0 (pcsa_former.decide.safe_log)."""
    return torch.where(p > 0, p.clamp_min(1e-38).log(), torch.full_like(p, float("-inf")))
