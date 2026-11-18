"""The neural player shell (champions_gym.neural) with stand-in policies. Needs neither numpy nor torch.
Plain-script mode: PYTHONPATH=python python python/tests/test_neural.py"""

import json
import os
import random
import re
import subprocess
import sys
import tempfile

import champions_gym as cg
from champions_gym import _cg
from champions_gym import neural as N
from champions_gym import neural_examples as E
from champions_gym.live import Session

HERE = os.path.dirname(os.path.abspath(__file__))
SMALL = os.path.join(HERE, "data", "live-phone-battle.jsonl")
MODEL_MD = os.path.join(HERE, "..", "..", "contracts", "MODEL.md")
# pcsa_former.layout's hash of the 0.6 feature list (runs/league-20261118-f/config.json)
LAYOUT_HASH_06 = "8b8a8fa78150d7cce598e6e91e23689a3d694fab752d0ab3dcd44363225adaad"


def selfplay(seed, a="tower", b="max_damage", live_order=True):
    """Seat 0's messages of an engine-played game, with a battle tag of its own."""
    rec, _ = _cg.live_selfplay(cg.TEAMS, seed, a, b, live_order)
    tag = "sp-%d-%s-%s-%d" % (seed, a, b, live_order)
    return [dict(m, battle=tag) if "battle" in m else m for m in (json.loads(x) for x in rec) if "type" in m]


def player(policy, **kw):
    kw.setdefault("warmup", False)
    return N.NeuralPlayer(policy, **kw)


# ------------------------------------------------------------------------------------------- self-check


def test_layout_hash_and_contract_version():
    assert N.layout_hash() == LAYOUT_HASH_06
    if os.path.isfile(MODEL_MD):
        with open(MODEL_MD, encoding="utf-8") as f:
            m = re.search(r"^Version (\d+\.\d+)", f.read(), re.M)
        assert m and m.group(1) == N.MODEL_CONTRACT, m
    c = N.contract()
    assert c["vocab"] == cg.VOCAB and c["shapes"]["rows"] == [12, 300]
    assert N.check_info(E.UniformPolicy().info) == []


def test_self_check_refuses_a_mismatch_at_construction():
    bad = E.FaultyPolicy("info")
    for make in (lambda: player(bad), lambda: Session(player=player(bad, allow_mismatch=False))):
        try:
            make()
        except N.ModelMismatch as e:
            assert "layout_hash" in str(e)
        else:
            raise AssertionError("mismatch accepted")
    p = player(bad, allow_mismatch=True)
    assert p.shell.mismatch
    for info in ({}, dict(E.UniformPolicy().info, contract="0.5"), dict(E.UniformPolicy().info, vocab={"species": 1})):
        q = E.UniformPolicy()
        q.info = info
        try:
            player(q)
        except N.ModelMismatch:
            pass
        else:
            raise AssertionError(info)

    class NoInfo:
        def decide(self, arrays, slots, battles):
            return {"scores": [[0.0] * 71 for _ in slots]}

    try:
        player(NoInfo())
    except N.ModelMismatch:
        pass
    else:
        raise AssertionError("no info")
    # the warm-up runs one preview and one turn decision; a policy that fails it fails construction
    c = E.CountingPolicy(E.HashPolicy())
    p = N.NeuralPlayer(c)
    assert len(c.calls) == 2 and len(p.warmup_ms) == 2
    for fault in ("shape", "raise", "nan"):
        try:
            N.NeuralPlayer(E.FaultyPolicy(fault))
        except Exception:
            pass
        else:
            raise AssertionError(fault)


# ------------------------------------------------------------------------------------------- choosing


def test_select_masks_ties_and_sampling():
    inf, nan = float("inf"), float("nan")
    s = [0.0] * 71
    s[3] = s[5] = 2.0
    s[1] = 9.0
    assert N.select(s, [3, 5, 8]) == (3, None)  # tie: lowest action; illegal 1 ignored
    s[8] = -inf
    assert N.select(s, [8, 5]) == (5, None)
    assert N.select([-inf] * 71, [2, 4])[0] is None
    assert N.select([nan] * 71, [2, 4])[0] is None
    assert N.select([nan] * 71, [4]) == (4, None)  # one legal action whatever the scores
    t = [0.0] * 71
    t[0], t[1] = 1.0, 0.0
    rng = random.Random(1)
    draws = [N.select(t, [0, 1], "sample", rng, 1.0)[0] for _ in range(4000)]
    share = draws.count(0) / 4000.0
    assert abs(share - 0.7311) < 0.03, share
    rng1, rng2 = random.Random(5), random.Random(5)
    assert [N.select(t, [0, 1], "sample", rng1)[0] for _ in range(50)] == [N.select(t, [0, 1], "sample", rng2)[0] for _ in range(50)]


def test_stack_and_take_give_the_batch_arrays():
    env = cg.BatchEnv(None, 6, seed=3)
    for _ in range(9):
        env.act(env.builtin_actions("tower"))
    t = env.observe_tensors()
    one = N.stack([cg.tensors(env.battle(g)) for g in range(6)])
    for k in cg.Tensors._fields:
        assert getattr(one, k).tobytes() == getattr(t, k).tobytes(), k
    sub = N.take(t, [4, 1])
    for k in cg.Tensors._fields:
        assert N._row_bytes(sub, k, 0) == N._row_bytes(t, k, 4) and N._row_bytes(sub, k, 1) == N._row_bytes(t, k, 1), k


# ------------------------------------------------------------------------------------------- reproducing a built-in player


class Recorder(cg.Player):
    def __init__(self, inner):
        self.inner = inner
        self.actions = []

    def action_for(self, battle):
        a = self.inner.action_for(battle)
        self.actions.append((battle.game, battle.kind, a))
        return a


def test_builtin_policy_reproduces_play_games():
    kinds = {0: 0, 1: 0, 2: 0}
    for k, (name, other) in enumerate([("tower", "max_damage"), ("max_damage", "tower"), ("random", "tower"), ("disciplined", "tower"), ("pilot_weather", "endgame")]):
        seed = 40 + k
        ref = Recorder(cg.BuiltinPlayer(name, cg.mix_seed(seed, 100)))
        a = cg.play_games(ref, other, 240, seed=seed, batch=80)
        neu = Recorder(player(E.BuiltinPolicy(name, cg.mix_seed(seed, 100))))
        b = cg.play_games(neu, other, 240, seed=seed, batch=80)
        assert ref.actions == neu.actions, name
        assert [(g.winner, g.turns) for g in a.games] == [(g.winner, g.turns) for g in b.games], name
        assert neu.inner.stats.fallbacks == 0
        for _, kd, _ in neu.actions:
            kinds[kd] += 1
    assert kinds[0] >= 1000 and kinds[1] >= 3000 and kinds[2] >= 500, kinds


class Lexicographic:
    """Built-in player `name` as the Session seeds it (mix_seed(seed, seq) per request), with its two back
    picks in the preview action's order."""

    def __init__(self, name, seed):
        self.name, self.seed = name, seed

    def _p(self, battle):
        return cg.BuiltinPlayer(self.name, cg.mix_seed(self.seed, battle.decision & 0xFFFFFFFFFFFFFFFF))

    def teampreview(self, battle):
        return list(cg.preview_picks(cg.preview_action(tuple(self._p(battle).teampreview(battle)))))

    def choose_move(self, battle):
        return self._p(battle).choose_move(battle)


def test_builtin_policy_reproduces_session_streams():
    kinds = {0: 0, 1: 0, 2: 0}
    for k in range(30):
        name = ("tower", "max_damage", "random", "endgame")[k % 4]
        ms = selfplay(300 + k, "tower" if k % 2 else "random", "max_damage")
        s = Session(Lexicographic(name, 9))
        p = player(E.BuiltinPolicy(name, 9, per_request_seed=True))
        t = Session(p)
        for m in ms:
            r1, r2 = s.handle(m), t.handle(m)
            if r1 and "order" in r1:
                assert r1["order"] == r2["order"], (k, m.get("seq"), r1, r2)
                kinds[{"turn": 1, "replace": 2}[m["kind"]]] += 1
            elif r1 and "picks" in r1:
                # a preview action names the two back picks as a pair: their order is lexicographic
                assert r1["picks"] == r2["picks"], (r1, r2)
                kinds[0] += 1
        assert p.stats.fallbacks == 0
    assert kinds[0] == 30 and kinds[1] > 200 and kinds[2] > 40, kinds


# ------------------------------------------------------------------------------------------- mask and fallbacks


def test_the_mask_is_always_applied():
    p = player(E.FaultyPolicy("illegal_high"))
    r = cg.play_games(p, "tower", 60, seed=2)  # an illegal action would raise in act
    assert len(r.games) == 60 and p.stats.fallbacks == 0 and p.stats.decisions > 300
    q = player(E.FaultyPolicy("illegal"))
    cg.play_games(q, "tower", 20, seed=2)
    forced = q.stats.decisions - q.stats.fallbacks
    assert q.stats.fallbacks > 50 and list(q.stats.reasons) == ["no finite score on a legal action"], q.stats.reasons
    assert forced >= 0


def test_every_fallback_path():
    for fault, reason in [("nan", "non-finite"), ("shape", "bad output"), ("rows", "policy raised"), ("raise", "policy raised")]:
        p = player(E.FaultyPolicy(fault, every=2), max_errors=1000)
        r = cg.play_games(p, "tower", 30, seed=4)
        assert len(r.games) == 30
        st = p.stats
        assert st.fallbacks > 20 and all(reason in k for k in st.reasons), (fault, st.reasons)
        assert st.fallbacks < st.decisions
    # a failure's record
    p = player(E.FaultyPolicy("raise"))
    ms = selfplay(61)
    s = Session(p)
    for m in ms:
        r = s.handle(m)
        if r and "order" in r:
            assert r["notes"]["action"] in r["notes"]["legal"]
    fb = p.last["fallback"]
    assert fb["player"] == "tower" and ("policy raised" in fb["reason"] or "in a row" in fb["reason"]), fb
    # three failures in a row: the policy is not asked again in that battle
    c = E.CountingPolicy(E.FaultyPolicy("raise"))
    p = player(c)
    s = Session(p)
    asked = 0
    for m in ms:
        s.handle(m)
        asked += m["type"] in ("decision_request", "preview_request")
    assert len(c.calls) == 3 and asked > 6 and p.stats.fallbacks == asked, (len(c.calls), asked)
    # the next battle asks it again
    for m in selfplay(62):
        s.handle(m)
    assert len(c.calls) == 6


def test_slow_policy():
    ms = selfplay(63)
    # three slow calls in a row: the rest of the battle goes to the fallback
    c = E.CountingPolicy(E.FaultyPolicy("slow", delay=0.03))
    p = player(c, timeout_s=0.01, hard_timeout_s=5.0, slow_limit=3)
    s = Session(p)
    n = 0
    for m in ms:
        r = s.handle(m)
        n += bool(r and ("order" in r or "picks" in r))
    assert len(c.calls) == 3 and p.stats.slow == 3 and p.stats.fallbacks == n - 3, (len(c.calls), p.stats)
    assert "in a row" in p.last["fallback"]["reason"]
    # one call over the hard limit is enough; the slow answer itself is still used
    c = E.CountingPolicy(E.FaultyPolicy("slow", delay=0.05))
    p = player(c, timeout_s=0.01, hard_timeout_s=0.02)
    s = Session(p)
    for m in ms:
        s.handle(m)
    assert len(c.calls) == 1 and p.stats.fallbacks == n - 1
    # slow now and then, never three in a row: always the policy's answer
    c = E.CountingPolicy(E.FaultyPolicy("slow", delay=0.03, every=2))
    p = player(c, timeout_s=0.01, hard_timeout_s=5.0, slow_limit=3)
    s = Session(p)
    for m in ms:
        s.handle(m)
    assert p.stats.fallbacks == 0 and p.stats.slow > 2


# ------------------------------------------------------------------------------------------- refusals, retries, restarts


def first_turn(ms):
    k = [i for i, m in enumerate(ms) if m["type"] == "decision_request" and m["kind"] == "turn"][1]
    return ms[: k + 1], ms[k]


def test_refusal_reuses_the_scores():
    ms = selfplay(64)
    head, req = first_turn(ms)
    c = E.CountingPolicy(E.HashPolicy())
    p = player(c)
    s = Session(p)
    for m in head:
        r = s.handle(m)
    calls = len(c.calls)
    scores = next(iter(p.shell.slots.values())).scores
    legal = list(r["notes"]["legal"])
    first = r["order"]
    r2 = s.handle(dict(req, seq=req["seq"] + 1, refused={"order": first}))
    assert len(c.calls) == calls and p.last["cached"], "asked again with an unchanged Battle"
    left = [a for a in r2["notes"]["legal"]]
    assert r["notes"]["action"] not in left and set(left) < set(legal)
    best = max(left, key=lambda a: (scores[a], -a))
    assert r2["notes"]["action"] == best, (r2["notes"]["action"], best)
    # the next request (a changed Battle) asks the policy
    for m in ms[len(head):]:
        s.handle(m)
        if m["type"] == "decision_request":
            break
    assert len(c.calls) == calls + 1
    # a policy that wants to be asked again is asked again
    c = E.CountingPolicy(E.BuiltinPolicy("max_damage"))
    p = player(c)
    s = Session(p)
    for m in head:
        r = s.handle(m)
    calls = len(c.calls)
    s.handle(dict(req, seq=req["seq"] + 1, refused={"order": r["order"]}))
    assert len(c.calls) == calls + 1


def test_retry_restart_and_replay_do_not_ask():
    ms = selfplay(65)
    head, req = first_turn(ms)
    c = E.CountingPolicy(E.HashPolicy(stateful=True))
    p = player(c)
    s = Session(p)
    for m in head:
        r = s.handle(m)
    calls = len(c.calls)
    assert s.handle(dict(req, retry=True))["order"] == r["order"]
    assert s.handle(dict(req, seq=req["seq"] + 1, retry=True))["order"] == r["order"]
    assert len(c.calls) == calls
    # a restart sends everything again: answered requests are not asked
    s2 = Session(player(c))
    c.calls.clear()
    first = [s2.handle(m) for m in ms]
    n = len(c.calls)
    again = [s2.handle(m) for m in ms]
    assert len(c.calls) == n and s2.restarts == 1
    assert [x and x.get("order") for x in again] == [x and x.get("order") for x in first]
    # a new process given the played orders asks nothing
    orders = {x["seq"]: x["order"] for x in first if x and "order" in x}
    picks = next(x["picks"] for x in first if x and "picks" in x)
    c.calls.clear()
    s3 = Session(player(c))
    for m in ms:
        m = dict(m)
        if m["type"] == "battle_start":
            m["picks"] = picks
        elif m["type"] == "preview_request":
            m["replay"] = True
        elif m["type"] == "decision_request":
            m["played"] = orders[m["seq"]]
        s3.handle(m)
    assert c.calls == []


# ------------------------------------------------------------------------------------------- determinism and equivalence


def test_argmax_is_deterministic_and_sampling_is_seeded():
    streams = [selfplay(70 + k) for k in range(4)]

    def run(mode, seed):
        out = []
        p = player(E.HashPolicy(stateful=True), mode=mode, seed=seed, temperature=0.05)
        for ms in streams:
            s = Session(p)
            out += [r["notes"]["action"] for r in (s.handle(m) for m in ms) if r and "order" in r]
        return out

    assert run("argmax", 1) == run("argmax", 2)
    assert run("sample", 3) == run("sample", 3)
    assert run("sample", 3) != run("sample", 4)


def manual_play(policy, opponent, games, seed, batch, threads=2, **kw):
    """One Battle at a time through NeuralPlayer.action_for, mirroring evaluate's slots and quotas."""
    p = player(policy, **kw)
    n = min(batch, games)
    quota = [games // n + (1 if g < games % n else 0) for g in range(n)]
    seats = [g % 2 for g in range(n)]
    env = cg.BatchEnv(None, n, seed, "real", 1000, "battle", threads, False, None, seats, True)
    counted, episode, log = [0] * n, [0] * n, []
    while any(counted[g] < quota[g] for g in range(n)):
        seat, _ = env.pending()
        acts = [0] * n
        for g in range(n):
            if seat[g] != seats[g]:
                continue
            b = env.battle(g)
            if counted[g] >= quota[g]:
                acts[g] = b.legal_actions()[0] if b.kind else cg.PREVIEW0
                continue
            acts[g] = p.action_for(b)
            log.append([g, episode[g], b.decision, acts[g]])
        env.act(acts, opponent=opponent)
        r = env.results()
        for g in range(n):
            if r.done[g]:
                counted[g] += counted[g] < quota[g]
                episode[g] += 1
    return log, p


def test_evaluate_equals_one_battle_at_a_time():
    for stateful in (False, True):
        res = N.evaluate(E.HashPolicy(stateful=stateful), "tower", games=120, seed=11, threads=2, batch=40, record=True)
        log, p = manual_play(E.HashPolicy(stateful=stateful), "tower", 120, 11, 40)
        assert res.actions == log, (stateful, len(res.actions), len(log))
        assert res.games == 120 and res.fallbacks == 0 and len(log) > 1000
        assert res.by_seat["0"][1] == res.by_seat["1"][1] == 60
    # built-in stand-in: evaluate with it equals the built-in player as the learner
    res = N.evaluate(E.BuiltinPolicy("max_damage"), "tower", games=60, seed=12, threads=2, batch=30)
    assert res.fallbacks == 0 and res.games == 60


def test_evaluate_reports():
    r = N.evaluate(E.UniformPolicy(), "random", games=50, seed=13, batch=25, threads=2)
    d = r.to_dict()
    assert r.games == 50 and r.wins + r.losses + r.ties == 50
    assert r.ci95[0] <= r.win_rate <= r.ci95[1] and r.decisions_per_s > 0
    assert r.replace_counters[0] > 0 and json.dumps(d)
    lo, hi = N.wilson(50, 100)
    assert abs(lo - 0.4038) < 1e-3 and abs(hi - 0.5962) < 1e-3
    # a broken policy still finishes, every decision from the fallback
    r = N.evaluate(E.FaultyPolicy("raise"), "tower", games=20, seed=14, batch=10, threads=2)
    assert r.games == 20 and r.fallbacks == r.decisions


# ------------------------------------------------------------------------------------------- the replace decision


def test_replace_sees_the_stale_battle():
    for live_order, want in ((True, "stale"), (False, "fresh")):
        p = player(E.HashPolicy())
        for k in range(6):
            s = Session(p)
            for m in selfplay(80 + k, live_order=live_order):
                s.handle(m)
        st = p.stats
        if want == "stale":
            assert st.replace_stale > 5 and st.replace_fresh == 0, st
        else:
            assert st.replace_fresh > 5, st
    r = N.evaluate(E.HashPolicy(), "tower", games=40, seed=15, batch=20, threads=2)
    assert r.replace_counters[1] > 0


# ------------------------------------------------------------------------------------------- the phone battle and the log


def test_phone_battle_through_load_with_log():
    with open(SMALL, encoding="utf-8") as f:
        ms = [json.loads(x) for x in f]
    with tempfile.TemporaryDirectory() as d:
        s = Session(player="champions_gym.neural:load", player_args={"policy": "champions_gym.neural_examples:HashPolicy", "log_dir": d, "policy_args": {"salt": 3}})
        replies = [s.handle(m) for m in ms]
        orders = [r for r in replies if r and "order" in r]
        assert len(orders) == sum(1 for m in ms if m["type"] == "decision_request")
        assert all(r["notes"]["action"] in r["notes"]["legal"] and r["notes"]["player"] == "neural:hash" for r in orders)
        assert not [x for x in s.anomalies if x["source"] != "builder"], s.anomalies
        s._player.close()
        files = os.listdir(d)
        assert len(files) == 1 and files[0].endswith(".neural.jsonl"), files
        with open(os.path.join(d, files[0]), encoding="utf-8") as f:
            lines = [json.loads(x) for x in f]
    head, decs, end = lines[0], lines[1:-1], lines[-1]
    assert head["type"] == "start" and head["model"]["layout_hash"] == N.layout_hash() and head["battle"] == ms[0]["battle"]
    assert files[0] == ms[0]["battle"] + ".neural.jsonl" and end["type"] == "end" and end["result"] == ms[-1].get("result")
    assert len(decs) == len(orders) + 1  # and the preview
    for rec, r in zip(decs[1:], orders):
        assert rec["action"] == r["notes"]["action"] and rec["order"] == r["order"], (rec, r)
        assert rec["kind"] in ("turn", "replace") and len(rec["scores"]) == 11 and rec["fallback"] is None
        assert set(rec) >= {"turn", "decision", "legal", "scores", "action", "order", "value", "ms", "model", "t"}
    assert decs[0]["kind"] == "preview" and len(decs[0]["scores"]) == 60 and len(decs[0]["order"]["picks"]) == 3


def test_engine_streams_with_refusals_and_a_log():
    key = lambda b: N._cache_key(N.stack([cg.tensors(b)]), 0)  # noqa: E731
    with tempfile.TemporaryDirectory() as d:
        p = player(E.HashPolicy(stateful=True), log_dir=d)
        refused = same = 0
        try:
            for k in range(8):
                ms = selfplay(90 + k)
                s = Session(p)
                seqs = [m["seq"] for m in ms if m["type"] == "decision_request"] + [1 << 30]
                for m in ms:
                    r = s.handle(m)
                    if r and "order" in r and m["kind"] == "turn" and seqs[seqs.index(m["seq"]) + 1] > m["seq"] + 1 and len(r["notes"]["legal"]) > 1:
                        k1 = key(s.battle)
                        r2 = s.handle(dict(m, seq=m["seq"] + 1, refused={"order": r["order"]}))
                        assert r2["order"] != r["order"] and r2["notes"]["action"] in r2["notes"]["legal"]
                        refused += 1
                        # asked again only when the refusal changed the Battle (a trap, a move ruled out) or
                        # the mask grew (the last move refused: Struggle as action 0)
                        unchanged = key(s.battle) == k1 and set(r2["notes"]["legal"]) <= set(r["notes"]["legal"])
                        assert p.last["cached"] == unchanged, (r, r2)
                        same += unchanged
        finally:
            p.close()
        files = sorted(os.listdir(d))
        assert len(files) == 8, files
        assert refused > 20 and p.stats.cached == same > 10 and p.stats.fallbacks == 0, (refused, same, p.stats)


# ------------------------------------------------------------------------------------------- no numpy, no torch


def test_without_numpy_or_torch():
    code = (
        "import sys; sys.modules['numpy'] = None; sys.modules['torch'] = None\n"
        "import json, champions_gym as cg\n"
        "from champions_gym import neural as N, neural_examples as E\n"
        "from champions_gym.live import Session\n"
        "assert cg.obs.np is None and N.np is None\n"
        "s = Session(player='champions_gym.neural:load', player_args={'policy': 'champions_gym.neural_examples:HashPolicy'})\n"
        "n = 0\n"
        "for line in open(sys.argv[1], encoding='utf-8'):\n"
        "    r = s.handle(json.loads(line)); n += bool(r and 'order' in r)\n"
        "r = N.evaluate(E.HashPolicy(stateful=True), 'tower', games=30, batch=10, threads=2, record=True)\n"
        "print(n, r.games, r.fallbacks)\n"
    )
    env = dict(os.environ, PYTHONPATH=os.path.join(HERE, ".."), PYTHONDONTWRITEBYTECODE="1")
    p = subprocess.run([sys.executable, "-c", code, SMALL], capture_output=True, text=True, env=env, timeout=300)
    assert p.returncode == 0, p.stderr
    n, games, fb = map(int, p.stdout.split())
    assert n > 10 and games == 30 and fb == 0


# ------------------------------------------------------------------------------------------- Session hooks and restarts


def play(p, ms):
    s = Session(p)
    return [s.handle(m) for m in ms]


def _answers(replies):
    return [(r.get("seq"), r.get("order"), r.get("picks")) for r in replies if r and ("order" in r or "picks" in r)]


def resent_messages(ms, upto, replies):
    """What a restarted controller sends: the battle again from battle_start, with the picks, the preview
    marked replayed and each answered request marked with the order played; up to message `upto`."""
    picks = next((r["picks"] for r in replies if r and "picks" in r), None)
    orders = {r["seq"]: r["order"] for r in replies if r and "order" in r}
    out = []
    for m in ms[:upto]:
        m = dict(m)
        if m["type"] == "battle_start" and picks:
            m["picks"] = picks
        elif m["type"] == "preview_request":
            m["replay"] = True
        elif m["type"] == "decision_request":
            if m["seq"] in orders:
                m["played"] = orders[m["seq"]]
            else:
                m["replay"] = True
        out.append(m)
    return out


class NoObserve:
    """A NeuralPlayer seen without its `observe` hook (how CG behaved before the hook existed)."""

    def __init__(self, p):
        self.p = p
        self.name = p.name

    def teampreview(self, b):
        return self.p.teampreview(b)

    def choose_move(self, b):
        return self.p.choose_move(b)

    def new_battle(self, tag, resend=False):
        return self.p.new_battle(tag, resend)

    def decision_notes(self):
        return self.p.decision_notes()


def restart_run(ms, cut, make, hooks=True, in_process=False, values=False):
    """Answers after a restart at message `cut`: the whole battle resent, then the rest of the stream."""
    s = Session(make())
    first = [s.handle(m) for m in ms[:cut]]
    if in_process:
        t = s  # the controller restarts; CG keeps its Session
        for m in ms[:cut]:
            t.handle(m)
    else:
        p = make()
        t = Session(p if hooks else NoObserve(p))
        for m in resent_messages(ms, cut, first):
            t.handle(m)
    rest = [t.handle(m) for m in ms[cut:]]
    if values:
        return _answers(rest), [r["notes"].get("model", {}).get("value") for r in rest if r and "order" in r]
    return _answers(rest)


def test_restart_rebuilds_the_memory():
    """A model with memory gives the same answers after a restart as without one: the replayed requests
    reach `observe` and take the memory steps the model took when asked."""
    rng = random.Random(5)
    same = differ = before_differ = inproc_differ = 0
    for k in range(16):
        ms = selfplay(200 + k, ("tower", "random")[k % 2], "max_damage")
        make = lambda: player(E.HashPolicy(stateful=True))  # noqa: E731
        straight = _answers(play(make(), ms))
        reqs = [i for i, m in enumerate(ms) if m["type"] == "decision_request"]
        if len(reqs) < 3:
            continue
        cut = rng.choice(reqs[1:])
        n_after = len([1 for m in ms[cut:] if m["type"] == "decision_request"])
        want = straight[-n_after:]
        got = restart_run(ms, cut, make)
        same += got == want
        differ += sum(1 for a, b in zip(got, want) if a != b)
        before_differ += sum(1 for a, b in zip(restart_run(ms, cut, make, hooks=False), want) if a != b)
        inproc_differ += sum(1 for a, b in zip(restart_run(ms, cut, make, in_process=True), want) if a != b)
    assert differ == 0 and inproc_differ == 0 and same >= 10, (same, differ, inproc_differ)
    assert before_differ > 10, before_differ  # without the hook the memory starts empty and answers change


class Hooked:
    """Records the Session's hook calls."""

    name = "hooked"

    def __init__(self):
        self.calls = []

    def new_battle(self, tag, resend=False):
        self.calls.append(("new_battle", tag, resend))

    def battle_end(self, result=None):
        self.calls.append(("battle_end", result))

    def observe(self, battle, action, how):
        self.calls.append(("observe", battle.kind, battle.decision, action, how))

    def teampreview(self, b):
        self.calls.append(("ask", 0, b.decision))
        return [0, 1, 2]

    def choose_move(self, b):
        self.calls.append(("ask", b.kind, b.decision))
        return b.legal_actions()[0]


def test_session_hooks():
    ms = selfplay(230)
    p = Hooked()
    s = Session(p)
    replies = [s.handle(m) for m in ms]
    tag = ms[0]["battle"]
    assert p.calls[0] == ("new_battle", tag, False) and p.calls[-1] == ("battle_end", ms[-1].get("result")), p.calls
    asked = [c for c in p.calls if c[0] == "ask"]
    assert len(asked) == len(_answers(replies)) and not [c for c in p.calls if c[0] == "observe"]
    # the same battle again in this Session: a resend, every answer repeated from the record and observed
    p.calls.clear()
    for m in ms:
        s.handle(m)
    obs = [c for c in p.calls if c[0] == "observe"]
    assert p.calls[0] == ("new_battle", tag, True) and not [c for c in p.calls if c[0] == "ask"]
    assert [(c[1], c[2]) for c in obs] == [(c[1], c[2]) for c in asked] and all(c[4] == "repeated" for c in obs), obs
    assert [c[3] for c in obs[1:]] == [r["notes"]["action"] for r in replies if r and "order" in r]
    # a new process: the battle resent with picks, replay and played
    q = Hooked()
    t = Session(q)
    for m in resent_messages(ms, len(ms), replies):
        t.handle(m)
    obs2 = [c for c in q.calls if c[0] == "observe"]
    assert q.calls[0] == ("new_battle", tag, False) and [(c[1], c[2], c[3]) for c in obs2] == [(c[1], c[2], c[3]) for c in obs]
    assert obs2[0][4] == "given" and all(c[4] == "played" for c in obs2[1:])
    # the Battle handed to observe is the one the Player was asked with
    class Arrays(Hooked):
        def __init__(self):
            super().__init__()
            self.seen = {}

        def observe(self, battle, action, how):
            self.seen[battle.decision] = N._cache_key(N.stack([cg.tensors(battle)]), 0)

        def teampreview(self, b):
            self.seen[b.decision] = N._cache_key(N.stack([cg.tensors(b)]), 0)
            return [0, 1, 2]

        def choose_move(self, b):
            self.seen[b.decision] = N._cache_key(N.stack([cg.tensors(b)]), 0)
            return b.legal_actions()[0]

    a, b = Arrays(), Arrays()
    replies = play(a, ms)
    tb = Session(b)
    for m in resent_messages(ms, len(ms), replies):
        tb.handle(m)
    assert a.seen == b.seen and len(a.seen) > 3


def test_unknown_played_and_refusals_in_a_replay():
    """`replay` without `played` is observed with no action; a refused request replayed takes no extra step."""
    ms = selfplay(231)
    p = player(E.CountingPolicy(E.HashPolicy(stateful=True)))
    replies = play(p, ms)
    q = Hooked()
    t = Session(q)
    for m in resent_messages(ms, len(ms), []):  # no picks, nothing played: all unknown
        t.handle(m)
    obs = [c for c in q.calls if c[0] == "observe"]
    assert obs and all(c[3] is None and c[4] == "unknown" for c in obs), obs
    assert len(_answers(replies)) > 2


# ------------------------------------------------------------------------------------------- status and notes


def test_status_and_notes():
    ms = selfplay(232)
    p = player(E.HashPolicy())
    s = Session(p)
    replies = [s.handle(m) for m in ms]
    orders = [r for r in replies if r and "order" in r]
    for r in orders:
        m = r["notes"]["model"]
        assert m["fallback"] is False and m["reused"] is False and isinstance(m["ms"], float), m
        assert "anomalies" in r["notes"] and "corrections" in r["notes"]
    st = s.handle({"type": "status"})["status"]
    ps = st["player_status"]
    assert ps["model_id"] == "hash" and ps["layout_ok"] and ps["mismatch"] == [] and ps["fallback_player"] == "tower"
    assert ps["battle"]["decisions"] == len(orders) + 1 and ps["total"]["decisions"] == ps["battle"]["decisions"]
    assert ps["battle"]["ms_max"] >= ps["battle"]["ms_median"] and ps["handed_to_fallback"] is False
    # a failing model shows in the reply that carries the fallback
    f = player(E.FaultyPolicy("raise"))
    s = Session(f)
    rs = [s.handle(m) for m in ms]
    o = [r for r in rs if r and "order" in r]
    assert "policy raised" in o[0]["notes"]["model"]["fallback"], o[0]
    st = s.status()["player_status"]
    assert st["handed_to_fallback"] and st["battle"]["fallbacks"] == len(o) + 1, st
    assert Session("tower").status()["player_status"] is None


def test_status_and_notes_through_the_process():
    ms = selfplay(233)
    text = [json.dumps(m) for m in ms] + ['{"type":"status"}']
    env = dict(os.environ, PYTHONPATH=os.path.join(HERE, ".."), PYTHONDONTWRITEBYTECODE="1")
    args = ["--player", "champions_gym.neural:load", "--player-args", json.dumps({"policy": "champions_gym.neural_examples:HashPolicy"})]
    p = subprocess.run([sys.executable, "-m", "champions_gym.live"] + args, input="\n".join(text) + "\n", capture_output=True, text=True, encoding="utf-8", env=env, timeout=300)
    assert p.returncode == 0, p.stderr
    out = [json.loads(x) for x in p.stdout.splitlines()]
    assert len(out) == len(text)
    assert all(r["notes"]["model"]["fallback"] is False for r in out if r and "order" in r)
    assert out[-1]["status"]["player_status"]["total"]["decisions"] > 2


# ------------------------------------------------------------------------------------------- threads


def test_two_sessions_on_two_threads():
    """Two Sessions, each with its NeuralPlayer, driven in turn from two threads; the players share one
    policy object with memory. Each gives the answers it gives alone."""
    import threading

    a, b = selfplay(240), selfplay(241, "random")
    alone = [_answers(play(player(E.HashPolicy(stateful=True)), x)) for x in (a, b)]
    shared = E.HashPolicy(stateful=True)
    sessions = [Session(player(shared)), Session(player(shared))]
    got = [[], []]
    turn = threading.Condition()
    state = {"next": 0, "done": [False, False]}

    def run(i, ms):
        for m in ms:
            with turn:
                turn.wait_for(lambda: state["next"] == i or state["done"][1 - i])
            got[i].append(sessions[i].handle(m))
            with turn:
                state["next"] = 1 - i
                turn.notify_all()
        with turn:
            state["done"][i] = True
            turn.notify_all()

    ts = [threading.Thread(target=run, args=(i, x)) for i, x in enumerate((a, b))]
    for t in ts:
        t.start()
    for t in ts:
        t.join(120)
    assert [_answers(g) for g in got] == alone
    # free for all: both threads at once
    sessions = [Session(player(shared)), Session(player(shared))]
    got = [[], []]

    def free(i, ms):
        for m in ms:
            got[i].append(sessions[i].handle(m))

    ts = [threading.Thread(target=free, args=(i, x)) for i, x in enumerate((a, b))]
    for t in ts:
        t.start()
    for t in ts:
        t.join(120)
    assert [_answers(g) for g in got] == alone


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
