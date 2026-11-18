"""The live session (champions_gym.live): the controller's messages in, CG's answers out. Plain-script mode:
PYTHONPATH=python python python/tests/test_live.py"""

import json
import os
import subprocess
import sys
import tempfile
import time

import champions_gym as cg
from champions_gym import _cg
from champions_gym.live import Session

HERE = os.path.dirname(os.path.abspath(__file__))
SMALL = os.path.join(HERE, "data", "live-phone-battle.jsonl")


def selfplay(seed, a="tower", b="max_damage", replace_before_rows=True):
    """Seat 0's record of a game played by CG's engine: messages, and per request (seq, engine Battle,
    action); recorded orders by seq. In the live order (the default) a replace request comes before the
    rows of its turn, as the phone sends it."""
    rec, decs = _cg.live_selfplay(cg.TEAMS, seed, a, b, replace_before_rows)
    lines = [json.loads(x) for x in rec]
    return lines, {q: (bt, act) for q, bt, act in decs}


def messages(lines):
    return [m for m in lines if "type" in m]


def bare(r):
    """A reply without the running counts (they differ between sessions that saw different messages)."""
    if not isinstance(r, dict) or not isinstance(r.get("notes"), dict):
        return r
    notes = {k: v for k, v in r["notes"].items() if k not in ("anomalies", "corrections")}
    out = dict(r, notes=notes)
    if not notes:
        del out["notes"]
    return out


class Recorded:
    """Answers what the engine's game took; checks every Battle it is asked with against the engine's."""

    name = "recorded"

    def __init__(self, decs):
        self.decs = decs
        self.asked = []
        self.diffs = []

    def check(self, battle):
        bt, act = self.decs[battle.decision]
        if not _cg.live_battle_equal(battle, bt):
            self.diffs.append((battle.decision, _cg.live_battle_diff(battle, bt)))
        assert list(battle.mask) == list(bt.mask), (battle.decision, battle.legal_actions(), bt.legal_actions())
        self.asked.append(battle.decision)
        return act

    def teampreview(self, battle):
        return self.check(battle)

    def choose_move(self, battle):
        a = self.check(battle)
        assert a in battle.legal_actions()
        return a


def _replace_before_rows(lines):
    """Replace requests whose turn's rows come after them, and all replace requests."""
    stale = total = 0
    for i, m in enumerate(lines):
        if m.get("type") == "decision_request" and m["kind"] == "replace" and "refused" not in m:
            total += 1
            prev = [x for x in lines[:i] if "type" in x or "order" in x]
            # since the last turn request: no events message (our order and requests only)
            k = max(j for j, x in enumerate(prev) if x.get("type") == "decision_request" and x["kind"] == "turn")
            if not any(x.get("type") == "events" and any(e["k"] != "refused" for e in x["events"]) for x in prev[k:]):
                stale += 1
    return stale, total


def test_selfplay_battles_equal_the_engine_side():
    for live_order in (True, False):
        _selfplay_battles_equal(live_order)


def _selfplay_battles_equal(live_order):
    games = 0
    asked = 0
    stale = replaces = 0
    for seed, (a, b) in enumerate([("tower", "max_damage"), ("random", "random"), ("max_damage", "tower")] * 4):
        lines, decs = selfplay(100 + seed, a, b, live_order)
        st, tot = _replace_before_rows(lines)
        stale += st
        replaces += tot
        p = Recorded(decs)
        s = Session(p)
        orders = [m["order"] for m in lines if "order" in m and "type" not in m]
        sent = []
        for m in messages(lines):
            r = s.handle(m)
            if m["type"] == "decision_request":
                assert r is not None and "order" in r and r["seq"] == m["seq"], r
                assert r["notes"]["action"] in r["notes"]["legal"], r
                sent.append(r["order"])
            elif m["type"] == "preview_request":
                assert set(r) == {"picks", "notes"} and len(set(r["picks"])) == 3, r
                assert set(r["notes"]) == {"anomalies", "corrections"}, r
            else:
                assert r is None, (m, r)
        assert p.diffs == [], p.diffs[:3]
        assert sent == orders, (sent, orders)
        assert not [x for x in s.anomalies if x["source"] != "builder"], s.anomalies
        assert s.ended
        games += 1
        asked += len(p.asked)
    assert games == 12 and asked > 60, asked
    # every replace request comes before its turn's rows in the live order, none in the old one
    assert replaces > 10 and stale == (replaces if live_order else 0), (live_order, stale, replaces)


def test_builtin_answers_inside_the_legal_set():
    for name in ("tower", "max_damage", "random"):
        lines, _ = selfplay(7)
        s = Session(name, seed=3)
        last = None
        for m in lines:
            if "type" not in m:
                # what the game took when it was not our answer: the controller reports a fallback
                if "order" in m and last is not None and m["order"] != last["order"]:
                    s.handle({"v": "0.7", "battle": lines[0]["battle"], "type": "events", "events": [], "fallback": {"seq": last["seq"], "order": m["order"]}})
                continue
            r = s.handle(m)
            if m["type"] == "decision_request":
                last = r
                assert r["notes"]["player"] == name and r["notes"]["action"] in r["notes"]["legal"], r
                assert (r["notes"]["action"], r["order"]) in s.legal_orders()
                assert "last_resort" not in r["notes"]


def first_turn(lines):
    """Messages up to and including the first turn request after turn 1."""
    ms = messages(lines)
    k = [i for i, m in enumerate(ms) if m["type"] == "decision_request" and m["kind"] == "turn"][1]
    return ms[: k + 1], ms[k]


def test_refusal_removes_the_order_and_asks_again():
    lines, _ = selfplay(11)
    ms, req = first_turn(lines)
    s = Session("max_damage")
    for m in ms:
        r = s.handle(m)
    first = r["order"]
    before = s.legal_orders()
    again = dict(req, seq=req["seq"] + 1, refused={"order": first})
    r2 = s.handle(again)
    assert r2["seq"] == req["seq"] + 1 and r2["order"] != first, (first, r2)
    after = [o for _, o in s.legal_orders()]
    assert first not in after and after
    # a refused move goes with its Mega form too
    if first["kind"] == "move":
        assert dict(first, mega=not first["mega"]) not in after
    # the `refused` event form does the same
    s2 = Session("max_damage")
    for m in ms:
        s2.handle(m)
    s2.handle({"v": "0.5", "battle": req["battle"], "type": "events", "events": [{"k": "refused", "order": first}]})
    r3 = s2.handle(dict(req, seq=req["seq"] + 1))
    assert r3["order"] != first


class Counting:
    name = "counting"

    def __init__(self):
        self.calls = 0
        self.p = cg.BuiltinPlayer("tower", 1)

    def teampreview(self, battle):
        return [0, 1, 2]

    def choose_move(self, battle):
        self.calls += 1
        return self.p.choose_move(battle)


def test_retry_repeats_the_order_without_asking():
    lines, _ = selfplay(12)
    ms, req = first_turn(lines)
    p = Counting()
    s = Session(p)
    for m in ms:
        r = s.handle(m)
    calls = p.calls
    # the same seq again, flagged retry or not
    assert s.handle(dict(req, retry=True))["order"] == r["order"]
    assert s.handle(dict(req))["order"] == r["order"]
    # a retry with a new seq repeats the last order
    r2 = s.handle(dict(req, seq=req["seq"] + 1, retry=True))
    assert r2["order"] == r["order"] and r2["seq"] == req["seq"] + 1
    assert p.calls == calls
    assert s.battle.decision == req["seq"] + 1


def test_restart_replays_the_same_answers():
    lines, decs = selfplay(13)
    ms = messages(lines)
    half = len(ms) // 2
    s = Session("tower", seed=5)
    first = [s.handle(m) for m in ms[:half]]
    b_half = s.battle
    # the controller restarts and sends everything again from battle_start
    again = [s.handle(m) for m in ms[:half]]
    key = lambda r: bare(r) if r is None or "order" not in r else (r["seq"], r["order"], r["notes"]["action"], r["notes"]["player"])
    assert [key(r) for r in again] == [key(r) for r in first]
    assert all(r["notes"]["repeated"] for r in again if r and "order" in r)
    assert _cg.live_battle_equal(s.battle, b_half), _cg.live_battle_diff(s.battle, b_half)
    assert s.restarts == 1
    for m in ms[half:]:
        s.handle(m)
    # a fresh session over the whole record answers the same
    t = Session("tower", seed=5)
    whole = [t.handle(m) for m in ms]
    assert whole[:half] == first and [key(r) for r in whole[:half]] == [key(r) for r in again]
    assert _cg.live_battle_equal(s.battle, t.battle)
    assert not [x for x in s.anomalies if x["source"] != "builder"], s.anomalies


def test_new_battle_tag_starts_clean():
    a, _ = selfplay(14)
    b, _ = selfplay(15)
    b = [dict(m, battle="other") if "type" in m else m for m in b]
    s = Session("tower")
    for m in messages(a)[:12]:
        s.handle(m)
    out = [s.handle(m) for m in messages(b)]
    t = Session("tower")
    assert [bare(r) for r in out] == [bare(t.handle(m)) for m in messages(b)]
    assert _cg.live_battle_equal(s.battle, t.battle)
    assert s.tag == "other" and s.restarts == 0
    # a message for the old battle is refused and changes nothing
    r = s.handle(messages(a)[12])
    assert "error" in r and _cg.live_battle_equal(s.battle, t.battle)


def test_stale_seq_is_not_applied():
    lines, _ = selfplay(16)
    ms = messages(lines)
    reqs = [m for m in ms if m["type"] == "decision_request"]
    s = Session("tower")
    for m in ms[: ms.index(reqs[2]) + 1]:
        s.handle(m)
    b = s.battle
    r = s.handle(reqs[0])
    assert "error" in r and r["seq"] == reqs[0]["seq"] and "order" in r
    assert _cg.live_battle_equal(s.battle, b)
    assert any("stale" in x["what"] for x in s.anomalies)


def test_garbage_never_raises():
    lines, _ = selfplay(17)
    ms = messages(lines)
    s = Session("tower")
    tag = ms[0]["battle"]
    for bad in ["not json", "", "[1,2]", {}, {"type": "nonsense", "battle": tag, "v": "0.5"}, {"type": "events", "v": "0.5"}, 42, None, b"\xff\xfe"]:
        r = s.handle(bad)
        assert isinstance(r, dict) and "error" in r, (bad, r)
    out = []
    for i, m in enumerate(ms):
        out.append(s.handle(m))
        if i == 3:
            # one unreadable event among good ones: the good ones are applied
            r = s.handle({"v": "0.5", "battle": tag, "type": "events", "events": [{"k": "nonsense"}, {"k": "unread", "text": "x"}]})
            assert "error" in r and r["dropped"][0]["index"] == 0
            assert s.battle is not None
    t = Session("tower")
    ref = [t.handle(m) for m in ms]
    assert [bare(r) for r in out if r and "order" in r] == [bare(r) for r in ref if r and "order" in r]
    # unknown fields are ignored
    u = Session("tower")
    out2 = [u.handle(dict(m, extra_field=1)) for m in ms]
    assert out2 == ref


def test_older_contract_version_is_read():
    lines, _ = selfplay(18)
    s = Session("tower")
    for m in messages(lines):
        s.handle(dict(m, v="0.4"))
    assert not [x for x in s.anomalies if "version" in x["what"]], s.anomalies
    t = Session("tower")
    for m in messages(lines):
        t.handle(dict(m, v="9.9"))
    assert any("version" in x["what"] for x in t.anomalies)


def test_no_legal_action_gets_a_marked_last_resort():
    lines, _ = selfplay(19)
    ms = messages(lines)
    s = Session("tower")
    for m in ms[:3]:
        s.handle(m)
    tag = ms[0]["battle"]
    picks = s.last_reply["picks"]
    team = ms[0]["own_team"]
    # a replace request where every picked Pokemon is refused
    req = {"v": "0.5", "battle": tag, "type": "decision_request", "seq": 50, "kind": "replace"}
    r = s.handle(req)
    for k, p in enumerate(picks):
        r = s.handle(dict(req, seq=51 + k, refused={"order": {"kind": "switch", "mon": team[p]["species"]}}))
    assert r["notes"].get("last_resort") is True and r["order"]["kind"] == "switch", r
    assert s.legal_actions() == []
    assert any("no legal action" in x["what"] for x in s.anomalies)


class TensorPlayer:
    """A Python player that reads the observation, as a network would."""

    name = "tensors"

    def __init__(self):
        self.kinds = []

    def teampreview(self, battle):
        t = cg.tensors(battle)
        assert t.kind == 0 and sum(t.action_mask[cg.PREVIEW0:]) == 60
        return cg.PREVIEW0  # an action, not picks

    def choose_move(self, battle):
        t = cg.tensors(battle)
        assert t.kind == battle.kind and t.decision == battle.decision
        assert [i for i in range(cg.N_TURN_ACTIONS) if t.mask[i]] == battle.legal_actions()
        self.kinds.append(int(t.kind))
        return battle.legal_actions()[-1]


def test_custom_python_player_with_tensors():
    seen = set()
    for seed in range(20, 26):
        lines, _ = selfplay(seed)
        p = TensorPlayer()
        s = Session(p)
        for m in messages(lines):
            r = s.handle(m)
            if m["type"] == "preview_request":
                assert r["picks"] == list(cg.preview_picks(cg.PREVIEW0)), r
            if m["type"] == "decision_request":
                assert r["notes"]["player"] == "tensors"
        seen |= set(p.kinds)
    assert seen == {1, 2}, seen


class Failing:
    def teampreview(self, battle):
        raise RuntimeError("no picks")

    def choose_move(self, battle):
        raise RuntimeError("no move")


def test_failing_player_still_answers():
    lines, _ = selfplay(26)
    s = Session(Failing())
    for m in messages(lines):
        r = s.handle(m)
        if m["type"] == "decision_request":
            assert r["notes"]["action"] in r["notes"]["legal"]
    assert sum(1 for x in s.anomalies if x["source"] == "python") > 2


def test_log_dir_keeps_every_message_and_reply():
    lines, _ = selfplay(27)
    ms = messages(lines)
    with tempfile.TemporaryDirectory() as d:
        with Session("tower", log_dir=d) as s:
            replies = [r for r in (s.handle(m) for m in ms) if r is not None]
        path = os.path.join(d, ms[0]["battle"] + ".jsonl")
        with open(path, encoding="utf-8") as f:
            got = [json.loads(x) for x in f]
    assert len(got) == len(ms) + len(replies)
    assert [g for g in got if "type" in g] == ms
    assert [g for g in got if "type" not in g] == replies


def test_small_recorded_stream():
    """The README example's stream: one phone battle, its replace requests where the phone sends them."""
    with open(SMALL, encoding="utf-8") as f:
        ms = [json.loads(x) for x in f]
    s = Session("tower")
    replies = [s.handle(m) for m in ms]
    orders = [r for r in replies if r and "order" in r]
    assert len(orders) == sum(1 for m in ms if m["type"] == "decision_request")
    assert all(r["notes"]["action"] in r["notes"]["legal"] for r in orders)
    assert s.ended and not [x for x in s.anomalies if x["source"] != "builder"], s.anomalies


def run_cli(args, lines, extra_env=None):
    env = dict(os.environ, PYTHONPATH=os.path.join(HERE, "..") + os.pathsep + os.environ.get("PYTHONPATH", ""), PYTHONDONTWRITEBYTECODE="1")
    env.update(extra_env or {})
    p = subprocess.run([sys.executable, "-m", "champions_gym.live"] + args, input="\n".join(lines) + "\n", capture_output=True, text=True, encoding="utf-8", env=env, timeout=120)
    return p


def test_stdin_stdout_process():
    lines, _ = selfplay(28)
    ms = messages(lines)
    text = [json.dumps(m, ensure_ascii=False) for m in ms]
    text.insert(3, "garbage")
    p = run_cli(["--player", "tower", "--seed", "4"], text)
    assert p.returncode == 0, p.stderr
    out = p.stdout.splitlines()
    assert len(out) == len(text), (len(out), len(text))
    got = [json.loads(x) for x in out]
    assert "error" in got[3]
    del got[3]
    s = Session("tower", seed=4)
    assert [bare(r) for r in got] == [bare(s.handle(m)) for m in ms]
    assert "champions_gym.live" in p.stderr


def test_without_numpy():
    code = (
        "import sys; sys.modules['numpy'] = None\n"
        "import json, champions_gym as cg\n"
        "from champions_gym.live import Session\n"
        "assert cg.obs.np is None\n"
        "class P:\n"
        "    def teampreview(self, b): return [0, 1, 2]\n"
        "    def choose_move(self, b):\n"
        "        t = cg.tensors(b); assert t.kind == b.kind; return b.legal_actions()[0]\n"
        "s = Session(P())\n"
        "n = 0\n"
        "for line in open(sys.argv[1], encoding='utf-8'):\n"
        "    r = s.handle(json.loads(line)); n += bool(r and 'order' in r)\n"
        "print(n)\n"
    )
    env = dict(os.environ, PYTHONPATH=os.path.join(HERE, ".."), PYTHONDONTWRITEBYTECODE="1")
    p = subprocess.run([sys.executable, "-c", code, SMALL], capture_output=True, text=True, env=env, timeout=120)
    assert p.returncode == 0, p.stderr
    assert int(p.stdout.strip()) > 10


def test_fallback_without_order_is_read():
    lines, _ = selfplay(29)
    ms = messages(lines)
    tag = ms[0]["battle"]
    s = Session("tower")
    i = [k for k, m in enumerate(ms) if m["type"] == "decision_request"][1]
    for m in ms[: i + 1]:
        s.handle(m)
    r = s.handle({"v": "0.6", "battle": tag, "type": "events", "events": [], "fallback": {"seq": ms[i]["seq"], "reason": "deadline"}})
    assert r is None, r
    for m in ms[i + 1:]:
        s.handle(m)
    assert not [x for x in s.anomalies if x["source"] != "builder"], s.anomalies


class Asked:
    """A player that must never be asked."""

    name = "asked"

    def __init__(self):
        self.n = 0

    def teampreview(self, battle):
        self.n += 1
        return [5, 4, 3]

    def choose_move(self, battle):
        self.n += 1
        return battle.legal_actions()[-1]


def test_restart_into_a_new_session_with_played():
    """S8-1, S8-2: a new CG process gets the battle again with the picks in battle_start and every past
    request marked with the order the game took: no Player is asked and the Battle is the same."""
    lines, _ = selfplay(31)
    ms = messages(lines)
    picks = next(x["picks"] for x in lines if "picks" in x and "type" not in x)
    s = Session("tower", seed=2)
    replies = [s.handle(dict(m, picks=picks) if m["type"] == "battle_start" else m) for m in ms]
    orders = {r["seq"]: r["order"] for r in replies if r and "order" in r}
    sent = []
    for m in ms:
        m = dict(m)
        if m["type"] == "battle_start":
            m["picks"] = picks
        elif m["type"] == "preview_request":
            m["replay"] = True
        elif m["type"] == "decision_request":
            m["played"] = orders[m["seq"]]
        sent.append(m)
    p = Asked()
    t = Session(p)
    again = [t.handle(m) for m in sent]
    assert p.n == 0
    assert {r["seq"]: r["order"] for r in again if r and "order" in r} == orders
    assert all(r["notes"]["replayed"] for r in again if r and "seq" in r)
    assert _cg.live_battle_equal(s.battle, t.battle), _cg.live_battle_diff(s.battle, t.battle)
    assert not [x for x in t.anomalies if x["source"] != "builder"], t.anomalies
    # replayed without `played`: no order, the Player is not asked either
    u = Session(p)
    out = [u.handle(dict(m, played=None, replay=True) if m["type"] == "decision_request" else m) for m in sent]
    assert p.n == 0 and all("order" not in r for r in out if r and "seq" in r)


def test_picks_of_the_phone_battle():
    """S8-1: the phone battle brought team indices 0, 1, 2. Tower picks others: the Builder corrects the
    picks from the log instead of the 'not picked' anomalies; with battle_start.picks nothing to correct."""
    with open(SMALL, encoding="utf-8") as f:
        ms = [json.loads(x) for x in f]
    s = Session("tower")
    for m in ms:
        s.handle(m)
    assert sorted(s.battle.picks) == [0, 1, 2] and s.battle.picks[0] == 0
    assert len(s.corrections) >= 1, s.corrections
    assert not [x for x in s.anomalies if "not picked" in x["what"]], s.anomalies
    t = Session("tower")
    replies = [t.handle(dict(m, picks=[0, 1, 2]) if m["type"] == "battle_start" else m) for m in ms]
    assert replies[1] == {"picks": [0, 1, 2], "notes": {"given": True, "anomalies": 0, "corrections": 0}}
    # tower's orders are not the recorded ones: the rows put our recorded choice right, not the picks
    assert not [c for c in t.corrections if "picks" in c["what"]] and not t.anomalies, (t.corrections, t.anomalies)


def test_player_by_path_and_arguments():
    """S8-10: every argument is a keyword from the config; a player is a built-in name, module:attr with
    player_args, or an object; a bad one fails at construction."""
    assert len(Session("tower").player_name) and len(cg.live.builtin_names()) == 10
    s = Session(player="champions_gym.player:BuiltinPlayer", player_args={"name": "max_damage", "seed": 3}, seed=1)
    assert s.player_name == "max_damage"
    s = Session(player="champions_gym.player:builtin_player", player_args={"name": "random"})
    assert s.player_name == "random"
    for bad, err in [("nonsense", ValueError), ("champions_gym.nope:X", ImportError), ("champions_gym.player:Nope", AttributeError)]:
        try:
            Session(player=bad)
        except err:
            pass
        else:
            raise AssertionError(bad)
    try:
        Session(player="tower", player_args={"x": 1})
    except TypeError:
        pass
    else:
        raise AssertionError("player_args with a built-in name")
    s = Session(player="tower", some_later_option=1)
    assert s.ignored_options == ["some_later_option"]
    lines, _ = selfplay(32)
    ms = messages(lines)
    text = [json.dumps(m) for m in ms]
    p = run_cli(["--player", "champions_gym.player:BuiltinPlayer", "--player-args", '{"name": "tower", "seed": 0}'], text)
    assert p.returncode == 0, p.stderr
    assert len(p.stdout.splitlines()) == len(text)
    p = run_cli(["--player", "nonsense"], text)
    assert p.returncode == 2 and "no built-in player" in p.stderr


def test_log_is_closed_at_battle_end():
    """S8-5: the final events and battle_end are kept; the record is closed at battle_end."""
    lines, _ = selfplay(33)
    ms = messages(lines)
    assert ms[-1]["type"] == "battle_end"
    with tempfile.TemporaryDirectory() as d:
        s = Session("tower", log_dir=d)
        for m in ms:
            s.handle(m)
        assert s.ended and s._log is None
        path = os.path.join(d, ms[0]["battle"] + ".jsonl")
        with open(path, encoding="utf-8") as f:
            got = [json.loads(x) for x in f]
        assert got[-1] == ms[-1]
        # a late message for the battle appends to the same record
        s.handle({"v": "0.7", "battle": ms[0]["battle"], "type": "events", "events": []})
        s.close()
        with open(path, encoding="utf-8") as f:
            assert len(f.readlines()) == len(got) + 1


# ---------------------------------------------------------------- S10: threads, status, counts

import threading


def _on_thread(f):
    """f() on a new thread; its result, or the exception it raised (BaseException included)."""
    box = {}

    def run():
        try:
            box["r"] = f()
        except BaseException as e:  # a Rust panic is a BaseException
            box["e"] = e

    t = threading.Thread(target=run)
    t.start()
    t.join(120)
    if "e" in box:
        raise AssertionError("raised on another thread: %s: %s" % (type(box["e"]).__name__, box["e"]))
    return box.get("r")


def test_session_used_from_another_thread():
    """S9-6: a Session made on one thread and used from another, built-in and Python players alike; the
    same answers as on one thread."""
    lines, decs = selfplay(41)
    ms = messages(lines)
    ref = Session("tower")
    want = [ref.handle(m) for m in ms]
    s = Session("tower")
    assert _on_thread(lambda: [s.handle(m) for m in ms]) == want
    assert _on_thread(lambda: (s.ended, s.restarts, s.tag, s.open_request, len(s.anomalies), s.corrections, s.legal_actions(), s.legal_orders())) is not None
    assert _on_thread(lambda: cg.tensors(s.battle)) is not None
    # a Python player made on this thread, called on another
    p = Recorded(decs)
    t = _on_thread(lambda: Session(p))
    replies = _on_thread(lambda: [t.handle(m) for m in ms])
    assert p.diffs == [] and p.asked and all("error" not in (r or {}) for r in replies)
    # a RustPlayer and a Battle handed between threads
    rp = _cg.RustPlayer("tower", 0)
    b = ref.battle
    assert _on_thread(lambda: rp.teampreview(b)) is not None


def test_two_threads_in_turn():
    """Two threads calling one Session in turn (each waits for the other): the same as one thread."""
    lines, _ = selfplay(42)
    ms = messages(lines)
    ref = Session("max_damage")
    want = [ref.handle(m) for m in ms]
    s = Session("max_damage")
    got = [None] * len(ms)
    turns = [threading.Event(), threading.Event()]
    turns[0].set()

    def worker(k):
        for i in range(k, len(ms), 2):
            turns[k].wait(60)
            turns[k].clear()
            got[i] = s.handle(ms[i])
            turns[1 - k].set()

    ts = [threading.Thread(target=worker, args=(k,)) for k in (0, 1)]
    for t in ts:
        t.start()
    for t in ts:
        t.join(120)
    assert got == want


class Slow:
    """A Python player that sleeps while choosing (it lets other threads run)."""

    name = "slow"

    def __init__(self):
        self.inside = 0
        self.overlap = 0

    def _enter(self):
        self.inside += 1
        self.overlap = max(self.overlap, self.inside)
        time.sleep(0.002)
        self.inside -= 1

    def teampreview(self, battle):
        self._enter()
        return [0, 1, 2]

    def choose_move(self, battle):
        self._enter()
        return battle.legal_actions()[0]


def test_concurrent_calls_are_serialised():
    """Calls from several threads at once wait for each other: the player is never inside twice, nothing
    raises, every reply is a reply."""
    lines, _ = selfplay(43)
    ms = messages(lines)
    p = Slow()
    s = Session(p)
    errors = []

    def hammer():
        try:
            for _ in range(30):
                s.status()
                s.legal_orders()
                s.anomalies
                s.battle
        except BaseException as e:
            errors.append(e)

    replies = []

    def play():
        for m in ms:
            replies.append(s.handle(m))

    ts = [threading.Thread(target=hammer) for _ in range(4)] + [threading.Thread(target=play), threading.Thread(target=play)]
    for t in ts:
        t.start()
    for t in ts:
        t.join(120)
    assert not errors, errors
    assert p.overlap == 1
    assert len(replies) == 2 * len(ms)
    assert not [r for r in replies if r and "error" in r and "stale" not in r["error"]], [r for r in replies if r and "error" in r][:2]
    # two threads sending the same request at once: serialised, the second gets the same answer
    s2 = Session("tower")
    k = next(i for i, m in enumerate(ms) if m["type"] == "decision_request")
    for m in ms[:k]:
        s2.handle(m)
    out = []
    ts = [threading.Thread(target=lambda: out.append(s2.handle(ms[k]))) for _ in range(2)]
    for t in ts:
        t.start()
    for t in ts:
        t.join(60)
    assert len(out) == 2 and out[0]["order"] == out[1]["order"], out


def test_batchenv_from_another_thread():
    """BatchEnv: made on one thread, stepped on another (one call at a time)."""
    env = cg.BatchEnv(cg.TEAMS, 4, seed=7)
    assert _on_thread(lambda: env.legal_actions(0))
    obs = _on_thread(lambda: env.observe())
    assert obs is not None
    # two threads at once: a call that finds another under way is an ordinary RuntimeError, never a panic
    seen = []

    def stepper():
        for _ in range(300):
            try:
                env.act([env.legal_actions(g)[0] for g in range(4)])
            except BaseException as e:
                seen.append(e)

    ts = [threading.Thread(target=stepper) for _ in range(2)]
    for t in ts:
        t.start()
    for t in ts:
        t.join(120)
    assert all(type(e) is RuntimeError for e in seen), [repr(e) for e in seen[:3]]


def test_status_message_and_counts():
    """S10-5: `status` answers what the Session holds and changes nothing; picks and order replies carry
    the running counts."""
    lines, _ = selfplay(44)
    ms = messages(lines)
    s = Session("tower")
    st = s.handle({"type": "status"})["status"]
    assert st["battle"] is None and st["contract"] == _cg.CONTRACT_VERSION == "0.9" and st["version"] == cg.__version__
    replies = [s.handle(m) for m in ms[:6]]
    before = s.battle
    st = s.handle('{"type":"status","battle":"%s"}' % ms[0]["battle"])["status"]
    assert _cg.live_battle_equal(before, s.battle)
    assert st["battle"] == ms[0]["battle"] and st["held"] and not st["ended"] and st["restarts"] == 0
    assert st["anomalies"] == s.anomalies and st["corrections"] == s.corrections
    assert s.handle({"type": "status", "battle": "other"})["status"]["held"] is False
    assert any(r and "order" in r for r in replies)
    for r in replies:
        if r and ("order" in r or "picks" in r):
            assert r["notes"]["anomalies"] == 0 and r["notes"]["corrections"] == 0, r
    # an anomaly shows in the next reply's counts
    s.handle({"v": "0.8", "battle": ms[0]["battle"], "type": "events", "events": [{"k": "turn", "n": 1}]})
    n = len(s.anomalies)
    assert n >= 1, s.anomalies
    for m in ms[6:]:
        r = s.handle(m)
        if r and "order" in r:
            assert r["notes"]["anomalies"] == len(s.anomalies) >= n, r
            break
    else:
        raise AssertionError("no order after the anomaly")


def test_status_through_the_process():
    """S10-5: the same through stdin and stdout; a status line is not written to the record."""
    lines, _ = selfplay(45)
    ms = messages(lines)
    text = [json.dumps({"type": "status"})] + [json.dumps(m) for m in ms] + [json.dumps({"type": "status", "battle": ms[0]["battle"]})]
    with tempfile.TemporaryDirectory() as d:
        p = run_cli(["--player", "tower", "--log-dir", d], text)
        assert p.returncode == 0, p.stderr
        out = [json.loads(x) for x in p.stdout.splitlines()]
        assert len(out) == len(text)
        assert out[0]["status"]["battle"] is None
        last = out[-1]["status"]
        assert last["battle"] == ms[0]["battle"] and last["ended"] and last["contract"] == "0.9"
        assert isinstance(last["anomalies"], list) and isinstance(last["corrections"], list)
        orders = [r for r in out if r and "order" in r]
        assert orders and all({"anomalies", "corrections"} <= set(r["notes"]) for r in orders)
        with open(os.path.join(d, ms[0]["battle"] + ".jsonl"), encoding="utf-8") as f:
            assert not any('"status"' in x for x in f)


def test_stream_labelled_0_7_with_0_6_shapes():
    """S10-6: a stream labelled 0.7 that uses only 0.6 shapes is read without complaint."""
    with open(SMALL, encoding="utf-8") as f:
        ms = [json.loads(x) for x in f]
    base = Session("tower")
    for m in ms:
        base.handle(m)
    for v in ("0.6", "0.7", "0.8", "0.9"):
        s = Session("tower")
        for m in ms:
            s.handle(dict(m, v=v) if "v" in m else m)
        assert not [a for a in s.anomalies if "version" in a["what"]], s.anomalies
        assert len(s.anomalies) == len(base.anomalies) and _cg.live_battle_equal(s.battle, base.battle)


if __name__ == "__main__":
    tests = [(n, f) for n, f in sorted(globals().items()) if n.startswith("test_") and callable(f)]
    failed = 0
    for name, f in tests:
        try:
            f()
            print("PASS", name)
        except Exception as e:  # report every test
            failed += 1
            print("FAIL", name, type(e).__name__, e)
    print("%d passed, %d failed" % (len(tests) - failed, failed))
    sys.exit(1 if failed else 0)
