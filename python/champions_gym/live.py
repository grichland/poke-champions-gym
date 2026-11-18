"""Live play: a phone controller's messages (contracts/LIVE.md section 2) in, CG's answers (section 3) out.

    from champions_gym.live import Session
    s = Session(player="tower")             # a built-in name, "module:Class", or a Player object
    for msg in messages:                    # dicts (or JSON text), in the order the controller sends them
        reply = s.handle(msg)               # {"picks": [...]}, {"seq": N, "order": {...}, "notes": {...}}, or None

The Battle is built by CG's Rust Builder, the same code that builds training Battles. `handle` never
raises: a message it cannot read gets `{"error": ...}` and the session stays usable. A restart resends the
battle from `battle_start` (same `battle` tag): requests already answered get the same answers.

Threads: a Session may be made on one thread and used from any other, one call at a time. Calls from
several threads at once are serialised by a lock held for the whole call (a Python Player is called
inside it, on the calling thread, with the GIL as usual).

`{"type": "status"}` (optional `"battle"`) is not part of a battle: it is answered with
`{"status": {...}}` (the battle held, anomalies, corrections, the open request, contract and package
versions), changes nothing and is not written to the record. Every picks or order reply carries the running
counts `notes.anomalies` and `notes.corrections`.

As a process: `python -m champions_gym.live --player tower [--seed N] [--log-dir D] [--player-args JSON]`
reads one JSON message per line on stdin and writes exactly one line per input line on stdout (the reply,
or `null`), flushed per line; diagnostics go to stderr.
"""

from __future__ import annotations

import argparse
import importlib
import json
import os
import re
import sys
import threading
from typing import Any, Dict, List, Optional, Union

from . import _cg

KIND_PREVIEW = 0
KIND_TURN = 1
KIND_REPLACE = 2


def builtin_names() -> List[str]:
    """Built-in players this build knows (the registry may know more names than this list shows)."""
    return [n for n in _cg.BUILTIN_PLAYERS if _cg.live_has_player(n)]


class Session:
    """One live battle at a time. Every argument is a keyword a config can give.

    `player`: a built-in player's name (`builtin_names()`), "module:attr" (a class or factory, called
    with `player_args` as keyword arguments, or a Player object), or a Player object: one with
    `teampreview(battle)` (three roster indices, lead first, or a preview action 11..70) and
    `choose_move(battle)` (an action 0..10), like `champions_gym.Player`. The object gets the same
    `champions_gym.Battle` the rest of the package uses; `champions_gym.tensors(battle)` works on it.
    An unknown name or a broken path raises here, not at the first request.
    `seed`: a built-in player's source for request `seq` is seeded `mix_seed(seed, seq)`.
    `log_dir`: every message and every reply is appended to `<log_dir>/<battle tag>.jsonl`, one JSON
    line each, flushed per line, and closed at `battle_end` (the file reads back with the `replay` tool).
    Other keywords are accepted for later versions, ignored, and listed in `ignored_options`.

    Optional Player hooks (LIVE.md section 3, 0.9), each called when the object has it: `new_battle(tag,
    resend)` at an accepted `battle_start` (`resend`: the battle held, sent again); `battle_end(result)`;
    `observe(battle, action, how)` for a request answered without asking (replayed, `played`, given picks,
    repeated after a resend), with the Battle as it was at that request; `status()` for the `status`
    answer's `player_status`; `decision_notes()` for `notes.model` of the reply it was just asked for.
    """

    def __init__(self, player: Any = "tower", seed: int = 0, log_dir: Optional[str] = None, player_args: Optional[Dict[str, Any]] = None, **options: Any):
        player = load_player(player, player_args)
        if isinstance(player, str):
            self._builtin: Optional[str] = player
            self._player: Any = None
            self.player_name = player
        else:
            if not (callable(getattr(player, "choose_move", None)) or callable(getattr(player, "choose", None))):
                raise TypeError("player needs choose_move(battle) (and teampreview(battle))")
            self._builtin = None
            self._player = player
            self.player_name = str(getattr(player, "name", None) or type(player).__name__)
        self.seed = int(seed)
        self.log_dir = log_dir
        self.ignored_options = sorted(options)
        if options:
            print("champions_gym.live: ignored options %s" % ", ".join(self.ignored_options), file=sys.stderr)
        self.last_reply: Optional[Dict[str, Any]] = None
        self._core = _cg.LiveCore()
        self._errors: List[str] = []
        self._log = None
        self._log_tag: Optional[str] = None
        self._closing = False
        self._lock = threading.RLock()

    # ------------------------------------------------------------------ the protocol

    def handle(self, message: Union[Dict[str, Any], str, bytes]) -> Optional[Dict[str, Any]]:
        """The answer to one message: picks, an order, None (no answer), or {"error": ...}."""
        with self._lock:
            try:
                status = self._status_query(message)
                if status is not None:
                    return status
                reply = self._handle(message)
            except (KeyboardInterrupt, SystemExit, GeneratorExit):
                raise
            except BaseException as e:  # never raise into the controller (a Rust panic is a BaseException)
                self._errors.append("handle: %s: %s" % (type(e).__name__, e))
                reply = {"error": "internal error: %s: %s" % (type(e).__name__, e)}
            if reply is not None:
                self.last_reply = reply
                self._write(reply)
            if self._closing:
                # the battle's record is complete; a later message for it appends again
                self._closing = False
                self.close()
            return reply

    def _handle(self, message: Any) -> Optional[Dict[str, Any]]:
        if isinstance(message, (bytes, bytearray)):
            message = bytes(message).decode("utf-8", errors="replace")
        if isinstance(message, str):
            text = message
            try:
                obj = json.loads(text)
            except ValueError:
                obj = None
        elif isinstance(message, dict):
            obj = message
            text = json.dumps(message, ensure_ascii=False)
        else:
            self._open_log(None)
            self._write({"unparsed": repr(message)})
            return {"error": "message is not a JSON object"}
        tag = obj.get("battle") if isinstance(obj, dict) else None
        if isinstance(obj, dict) and obj.get("type") == "battle_start" or self._log_tag is None:
            self._open_log(tag if isinstance(tag, str) else None)
        self._write(obj if obj is not None else {"unparsed": text})
        prev_tag, prev_restarts = self._core.tag, self._core.restarts
        step, reply, seq, kind = self._core.handle(text)
        typ = obj.get("type") if isinstance(obj, dict) else None
        self._closing = typ == "battle_end"
        if typ == "battle_start" and step == 0:
            resend = prev_tag is not None and prev_tag == self._core.tag and self._core.restarts > prev_restarts
            self._hook("new_battle", self._core.tag, resend=resend)
        elif typ == "battle_end" and step == 0:
            self._hook("battle_end", obj.get("result"))
        if step == 0:
            return None
        if step == 1:
            out = json.loads(reply)
            self._observe()
        else:
            out = json.loads(self._ask(seq, kind))
        if isinstance(out, dict) and "error" not in out and ("order" in out or "picks" in out or "replayed" in out):
            notes = out.get("notes")
            if not isinstance(notes, dict):
                notes = out["notes"] = {}
            a, c = self._core.counts()
            notes["anomalies"] = a + len(self._errors)
            notes["corrections"] = c
            if step == 2 and self._player is not None:
                m = self._hook("decision_notes")
                if m is not None:
                    notes["model"] = m
        return out

    def _hook(self, name: str, *args: Any, **kw: Any) -> Any:
        """Calls the Player's optional hook `name`; a failure is recorded, never raised."""
        f = getattr(self._player, name, None) if self._player is not None else None
        if not callable(f):
            return None
        try:
            return f(*args, **kw)
        except (KeyboardInterrupt, SystemExit, GeneratorExit):
            raise
        except BaseException as e:
            self._errors.append("player %s.%s raised %s: %s" % (self.player_name, name, type(e).__name__, e))
            return None

    def _observe(self) -> None:
        """A request answered without asking the Player: its optional `observe(battle, action, how)` gets
        the Battle as it was at that request and what was played (preview: an action 11..70)."""
        if self._player is None or not callable(getattr(self._player, "observe", None)):
            return
        ans = self._core.answered()
        if ans is None:
            return
        battle, action, picks, how = ans
        if battle.kind == KIND_PREVIEW:
            action = _cg.preview_action(tuple(picks)) if picks else None
        self._hook("observe", battle, action, how)

    def _status_query(self, message: Any) -> Optional[Dict[str, Any]]:
        """The answer to `{"type": "status"}`, else None (a battle message)."""
        obj = message
        if isinstance(message, (str, bytes, bytearray)):
            text = bytes(message).decode("utf-8", errors="replace") if not isinstance(message, str) else message
            if '"status"' not in text:
                return None
            try:
                obj = json.loads(text)
            except ValueError:
                return None
        if not (isinstance(obj, dict) and obj.get("type") == "status"):
            return None
        return {"status": self.status(obj.get("battle"))}

    def status(self, battle: Optional[str] = None) -> Dict[str, Any]:
        """What the Session holds (the `status` message's answer). `held` is false when `battle` names
        another battle than the one held."""
        from . import __version__

        with self._lock:
            o = self._core.open_request()
            kinds = {KIND_PREVIEW: "preview", KIND_TURN: "turn", KIND_REPLACE: "replace"}
            return {
                "battle": self._core.tag,
                "held": battle is None or battle == self._core.tag,
                "ended": self._core.ended,
                "restarts": self._core.restarts,
                "messages": self._core.messages,
                "open_request": None if o is None else {"seq": o[0], "kind": kinds.get(o[1], o[1])},
                "anomalies": self.anomalies,
                "corrections": self.corrections,
                "player": self.player_name,
                "player_status": self._hook("status"),
                "contract": _cg.CONTRACT_VERSION,
                "version": __version__,
            }

    def _ask(self, seq: int, kind: int) -> str:
        if self._builtin is not None:
            return self._core.builtin(self._builtin, self.seed)
        p = self._player
        battle = self._core.battle()
        try:
            if kind == KIND_PREVIEW:
                r = p.teampreview(battle)
                if isinstance(r, int) and not isinstance(r, bool):
                    picks = list(_cg.preview_picks(r))
                else:
                    picks = [int(x) for x in r]
                return self._core.answer_picks(picks, self.player_name)
            f = getattr(p, "choose_move", None) or p.choose
            return self._core.answer(int(f(battle)), self.player_name)
        except (KeyboardInterrupt, SystemExit, GeneratorExit):
            raise
        except BaseException as e:  # the player failed: the first legal action, recorded
            self._errors.append("seq %d: player %s raised %s: %s" % (seq, self.player_name, type(e).__name__, e))
            if kind == KIND_PREVIEW:
                return self._core.answer_picks([], self.player_name)
            return self._core.answer(-1, self.player_name)

    # ------------------------------------------------------------------ what the session holds

    @property
    def battle(self) -> Any:
        """The Battle now, with the open request's `kind` and `mask` (kind 3 and an empty mask with none)."""
        with self._lock:
            return self._core.battle()

    def legal_actions(self) -> List[int]:
        """The open request's legal actions (preview: 11..70)."""
        with self._lock:
            return self._core.legal_actions()

    def legal_orders(self) -> List[tuple]:
        """[(action, order dict)] of the open turn or replace request: what each legal action is sent as."""
        with self._lock:
            return [(a, json.loads(o)) for a, o in self._core.legal_orders()]

    @property
    def open_request(self) -> Optional[tuple]:
        """(seq, kind) of the open request (kind 0 preview, 1 turn, 2 replace), or None."""
        with self._lock:
            return self._core.open_request()

    @property
    def tag(self) -> Optional[str]:
        with self._lock:
            return self._core.tag

    @property
    def ended(self) -> bool:
        with self._lock:
            return self._core.ended

    @property
    def restarts(self) -> int:
        with self._lock:
            return self._core.restarts

    @property
    def corrections(self) -> List[Dict[str, Any]]:
        """What the log put right: the picks (LIVE.md section 5, rule 32, or a later preview fallback)
        and our recorded choice where a row of ours names another (rule 31). Not anomalies."""
        with self._lock:
            return [{"at": at, "what": w} for at, w in self._core.corrections()]

    @property
    def anomalies(self) -> List[Dict[str, Any]]:
        """Everything that did not fit: the Builder's anomalies ("builder"), protocol problems ("session"),
        caught errors ("internal", "python")."""
        with self._lock:
            out = [{"source": s, "at": at, "what": w} for s, at, w in self._core.anomalies()]
            out += [{"source": "python", "at": None, "what": w} for w in self._errors]
            return out

    # ------------------------------------------------------------------ the record

    def _open_log(self, tag: Optional[str]) -> None:
        if not self.log_dir:
            return
        name = re.sub(r"[^A-Za-z0-9._-]", "_", tag) if tag else "unknown"
        if self._log is not None and name == self._log_tag:
            return
        self.close()
        try:
            os.makedirs(self.log_dir, exist_ok=True)
            self._log = open(os.path.join(self.log_dir, name + ".jsonl"), "a", encoding="utf-8", newline="\n")
            self._log_tag = name
        except OSError as e:
            self._errors.append("log: %s" % e)
            self._log = None

    def _write(self, obj: Any) -> None:
        if self._log is None:
            return
        try:
            self._log.write(json.dumps(obj, ensure_ascii=False) + "\n")
            self._log.flush()
        except (OSError, TypeError, ValueError) as e:
            self._errors.append("log: %s" % e)

    def close(self) -> None:
        with self._lock:
            self._close()

    def _close(self) -> None:
        if self._log is not None:
            try:
                self._log.close()
            except OSError:
                pass
        self._log = None
        self._log_tag = None

    def __enter__(self) -> "Session":
        return self

    def __exit__(self, *exc: Any) -> None:
        self.close()


def load_player(spec: Any, player_args: Optional[Dict[str, Any]] = None) -> Any:
    """A built-in name (checked), "module:attr" (a class or factory is called with `player_args`), or an
    object, returned as it is."""
    args = dict(player_args or {})
    if not isinstance(spec, str):
        if args:
            raise TypeError("player_args need a player given as a name or a 'module:attr' path")
        return spec
    if ":" not in spec:
        if args:
            raise TypeError("built-in player %r takes no player_args" % spec)
        if not _cg.live_has_player(spec):
            raise ValueError("no built-in player %r (built-in: %s; or give 'module:attr')" % (spec, ", ".join(builtin_names())))
        return spec
    mod, attr = spec.split(":", 1)
    obj: Any = importlib.import_module(mod)
    for part in attr.split("."):
        obj = getattr(obj, part)
    if isinstance(obj, type) or not (callable(getattr(obj, "choose_move", None)) or callable(getattr(obj, "choose", None))):
        if not callable(obj):
            raise TypeError("%s is neither a Player, a class nor a factory" % spec)
        return obj(**args)
    if args:
        raise TypeError("%s is a Player object; player_args need a class or factory" % spec)
    return obj


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(prog="python -m champions_gym.live", description="CG live session: JSON lines on stdin, one reply line per input line on stdout.")
    ap.add_argument("--player", default="tower", help="a built-in player's name, or module:Class (or module:factory)")
    ap.add_argument("--player-args", default=None, help="JSON object of keyword arguments for module:Class")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--log-dir", default=None)
    a = ap.parse_args(argv)
    stdin = open(sys.stdin.fileno(), "r", encoding="utf-8", errors="replace", newline=None, closefd=False)
    stdout = open(sys.stdout.fileno(), "w", encoding="utf-8", newline="\n", closefd=False)
    try:
        session = Session(a.player, seed=a.seed, log_dir=a.log_dir, player_args=json.loads(a.player_args) if a.player_args else None)
    except Exception as e:
        print("champions_gym.live: %s" % e, file=sys.stderr)
        return 2
    print("champions_gym.live: player %s, seed %d, log %s" % (session.player_name, a.seed, a.log_dir), file=sys.stderr)
    with session:
        for line in stdin:
            line = line.rstrip("\r\n")
            reply = session.handle(line) if line.strip() else {"error": "empty line"}
            stdout.write(json.dumps(reply, ensure_ascii=False) + "\n")
            stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
