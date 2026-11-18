"""Team reading (reasons for dropped teams, check_teams), weighted team draws, and the batched built-in
players (builtin_actions, act with an opponent). Plain-script mode:
PYTHONPATH=python python python/tests/test_teams_builtin.py
They need no numpy."""

import hashlib
import json
import math
import os
import random
import sys
import tempfile
import time
import warnings

import champions_gym as cg

TEAMS = cg.TEAMS
A = cg.N_ACTIONS


def _shipped():
    with open(TEAMS, encoding="utf-8") as f:
        return json.load(f)


def _write(teams):
    fd, path = tempfile.mkstemp(suffix=".json", prefix="cg-teams-")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(teams, f)
    return path


def _raises(exc, f, *a, **k):
    try:
        f(*a, **k)
    except exc as e:
        return str(e)
    raise AssertionError("no %s from %s" % (exc.__name__, getattr(f, "__name__", f)))


def test_shipped_pool_accepted_fast():
    t0 = time.perf_counter()
    n, errs = cg.check_teams(TEAMS)
    dt = time.perf_counter() - t0
    assert (n, errs) == (430, []), (n, errs[:3])
    assert dt < 1.0, dt
    with open(TEAMS, encoding="utf-8") as f:
        assert cg.check_teams(f.read()) == (430, [])
    env = cg.BatchEnv(TEAMS, 2)
    assert env.team_count == 430 and env.team_file_count == 430 and env.team_errors == []
    assert env.team_file_index == list(range(430))


def _bad_teams():
    """The shipped first teams with one fault each, and what the reason must say."""
    base = _shipped()
    cases = []

    def bad(why, f):
        t = json.loads(json.dumps(base[len(cases) + 1]))
        cases.append((f(t), why))

    def setkey(i, k, v):
        def f(t):
            if v is None:
                t[i].pop(k, None)
            else:
                t[i][k] = v
            return t
        return f

    bad("set 2 (", setkey(2, "species", "Notamon"))
    bad("unknown species", setkey(1, "species", "Notamon"))
    bad("unknown move", lambda t: setkey(0, "moves", t[0]["moves"][:1] + ["Not A Move"])(t))
    bad("unknown item", setkey(3, "item", "Not An Item"))
    bad("unknown ability", setkey(4, "ability", "Not An Ability"))
    bad("unknown nature", setkey(5, "nature", "Grumpy"))
    bad("more than four moves", lambda t: setkey(0, "moves", (t[0]["moves"] * 5)[:5])(t))
    bad("no moves", setkey(0, "moves", ["", None]))
    bad("moves missing", setkey(0, "moves", None))
    bad("species missing", setkey(0, "species", None))
    bad("ability missing", setkey(0, "ability", None))
    bad("a move is not a string", lambda t: setkey(0, "moves", t[0]["moves"][:1] + [7])(t))
    bad("evs.atk is not a number", setkey(0, "evs", {"hp": 1, "atk": "x"}))
    bad("evs.hp is 300", setkey(0, "evs", {"hp": 300}))
    bad("evs.hp is -1", setkey(0, "evs", {"hp": -1}))
    bad("evs.spe is 2.5", setkey(0, "evs", {"spe": 2.5}))
    bad("evs is not an object", setkey(0, "evs", [1, 2]))
    bad("item is not a string", setkey(0, "item", 3))
    bad("5 sets, need 6", lambda t: t[:5])
    bad("7 sets, need 6", lambda t: t + [t[0]])
    bad("not a list of sets", lambda t: {"sets": t})
    bad("set is not an object", lambda t: t[:5] + ["Pikachu"])
    return base, cases


def _gendered(base):
    """A (team index, set index) whose species can be M or F: a missing gender is an error there."""
    for ti, t in enumerate(base):
        for si, s in enumerate(t):
            probe = json.loads(json.dumps(t))
            probe[si].pop("gender")
            n, errs = cg.check_teams(json.dumps([probe]))
            if n == 0:
                return ti, si, errs[0][1]
    raise AssertionError("no gendered species")


def test_dropped_teams_have_reasons():
    base, cases = _bad_teams()
    ti, si, why = _gendered(base)
    assert "gender missing" in why, why
    probe = json.loads(json.dumps(base[ti]))
    probe[si]["gender"] = "N"
    assert "gender N" in cg.check_teams(json.dumps([probe]))[1][0][1]
    probe[si]["gender"] = "X"
    assert 'gender "X"' in cg.check_teams(json.dumps([probe]))[1][0][1]
    # harmless: empty item, unknown keys, empty move slots, absent nature and stat points
    ok = json.loads(json.dumps(base[0]))
    ok[0]["item"] = ""
    ok[1]["shiny"] = True
    ok[2]["moves"] = ok[2]["moves"] + [""]
    ok[3].pop("nature")
    ok[4].pop("evs")
    ok[5]["level"] = 100
    # file: good team 0, the faulty ones, a good one at the end
    teams = [base[0]] + [t for t, _ in cases] + [ok]
    text = json.dumps(teams)
    n, errs = cg.check_teams(text)
    assert n == 2, (n, errs)
    assert [i for i, _ in errs] == list(range(1, len(cases) + 1)), errs
    for (i, reason), (_, why) in zip(errs, cases):
        assert why in reason, (i, reason, why)
    path = _write(teams)
    try:
        assert cg.check_teams(path) == (n, errs)
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            env = cg.BatchEnv(path, 8, seed=1)
        assert len(w) == 1 and "%d of %d teams not accepted" % (len(cases), len(teams)) in str(w[0].message), [str(x.message) for x in w]
        assert env.team_errors == errs and env.team_count == 2 and env.team_file_count == len(teams)
        assert env.team_file_index == [0, len(teams) - 1]
        ids = env.team_ids()
        assert all(ids[g, s] in (0, 1) for g in range(8) for s in (0, 1))
        msg = _raises(ValueError, cg.BatchEnv, path, 8, strict_teams=True)
        assert "%d of %d teams not accepted" % (len(cases), len(teams)) in msg and "team 1:" in msg, msg
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            cg.BatchEnv(TEAMS, 2, strict_teams=True)
        assert not w
    finally:
        os.remove(path)
    one = _write([base[0]])
    try:
        assert "need at least two valid teams" in _raises(cg.CGError, cg.BatchEnv, one, 2)
    finally:
        os.remove(one)
    _raises(cg.CGError, cg.check_teams, "[1, 2")
    _raises(cg.CGError, cg.check_teams, "no/such/file.json")


# sha256 over (seat, kind, masks, team_ids, results) of 600 random acts, recorded before team weights,
# per-team errors and the agent seat existed
GOLDEN = {
    (7, 1, "real"): "40528b3a8922c7538773366192db9d90257bf4810cef64ececd8e494b635a6cf",
    (7, 4, "real"): "40528b3a8922c7538773366192db9d90257bf4810cef64ececd8e494b635a6cf",
    (123, 1, "real"): "893b74e62aba9bfc3d67718b165bfaa15af780c41ddca06e32bca98c574966a9",
    (5, 1, "stage3"): "2e769d9492fe48d35bc4e57f22655e8068d60e3aa59981957a86caf4682b9930",
}


def _digest(seed, threads, team_mode, **kw):
    env = cg.BatchEnv(TEAMS, 64, seed=seed, threads=threads, team_mode=team_mode, **kw)
    rng = random.Random(3)
    h = hashlib.sha256()
    for _ in range(600):
        s, k, m = env._e.observe()
        h.update(s)
        h.update(k)
        h.update(m)
        h.update(env._e.team_ids())
        acts = []
        for g in range(env.games):
            row = m[g * A : (g + 1) * A]
            acts.append(rng.choice([a for a in range(A) if row[a]]))
        env.act(acts)
        for x in env._e.results():
            h.update(x)
    return h.hexdigest()


# the same in the live order (a replace request before the rows of its turn), recorded 19 Nov 2026
GOLDEN_LIVE = {
    (7, 1, "real"): "d608b51871929b2f2ba701ed990853b253b86182492d9597ae9fa988046af039",
    (7, 4, "real"): "d608b51871929b2f2ba701ed990853b253b86182492d9597ae9fa988046af039",
    (123, 1, "real"): "d4712851af998f9976086ba0c60dd0a00ddbc6c3c70b5cb6f3eb6c11a2fc68bd",
    (5, 1, "stage3"): "47cc64d3bb881516ccebcfc43b0c65195ff7b2e977d028548cba3a829cf3e60a",
}


def test_unweighted_stream_unchanged():
    for (seed, threads, mode), want in GOLDEN.items():
        assert _digest(seed, threads, mode, replace_before_rows=False) == want, (seed, threads, mode)
    for (seed, threads, mode), want in GOLDEN_LIVE.items():
        assert _digest(seed, threads, mode) == want, (seed, threads, mode)
    # an agent seat changes nothing without weights
    assert _digest(7, 1, "real", agent_seat=[g % 2 for g in range(64)], replace_before_rows=False) == GOLDEN[(7, 1, "real")]


def test_weights_are_validated():
    n = 430
    good = [1.0] * n
    cases = [
        ([1.0] * 5, "5 weights for 430 teams"),
        (good[:-1] + [-1.0], "negative"),
        (good[:-1] + [float("nan")], "team 429 is NaN"),
        (good[:-1] + [float("inf")], "team 429 is inf"),
        ([0.0] * n, "no accepted team has a positive weight"),
        ([0.0] * (n - 1) + [2.0], "only team 429"),
        (["a"] * n, "need numbers"),
        ((good, [0.0] * n), "team_weights[1]: no accepted team"),
        (([1.0] + [0.0] * (n - 1), [1.0] + [0.0] * (n - 1)), "the second seat can draw only team 0"),
        ((good, good, good), "3 sequences"),
    ]
    for w, why in cases:
        msg = _raises(ValueError, cg.BatchEnv, TEAMS, 2, team_weights=w)
        assert why in msg, (why, msg)
    # one team for the agent, any other team for the opponent: possible
    one = [0.0] * n
    one[3] = 1.0
    other = [1.0] * n
    other[3] = 0.0
    env = cg.BatchEnv(TEAMS, 16, team_weights=(one, other))
    ids = env.team_ids()
    assert all(ids[g, 0] == 3 and ids[g, 1] != 3 for g in range(16))
    _raises(ValueError, cg.BatchEnv, TEAMS, 2, agent_seat=[0, 2])
    _raises(ValueError, cg.BatchEnv, TEAMS, 2, agent_seat=[0])


def _z(k, n, p):
    return (k - n * p) / math.sqrt(n * p * (1 - p))


def test_weighted_draws_follow_weights():
    """The agent's seat draws team 5 with weight 0.6 (agent seat alternating by game); the other seat
    draws uniformly among the rest; a second file index map is honoured when teams are dropped."""
    n = 430
    w_agent = [0.4 / (n - 1)] * n
    w_agent[5] = 0.6
    agent = [g % 2 for g in range(2048)]
    hits = draws = other5 = 0
    for seed in range(4):
        env = cg.BatchEnv(TEAMS, 2048, seed=seed, team_weights=(w_agent, [1.0] * n), agent_seat=agent)
        assert list(env.agent_seat) == agent
        ids = env.team_ids()
        for g in range(env.games):
            a, o = ids[g, agent[g]], ids[g, 1 - agent[g]]
            assert a != o
            hits += a == 5
            other5 += o == 5
            draws += 1
    assert abs(_z(hits, draws, 0.6)) < 4.5, hits / draws
    # the other seat: uniform over the teams that are not the agent's (team 5 only when the agent has not)
    assert abs(_z(other5, draws, 0.4 / (n - 1))) < 4.5, other5
    # file indices: drop team 0 of the file; weights still by file index
    teams = _shipped()
    teams[0] = teams[0][:5]
    path = _write(teams)
    try:
        w = [0.0] * n
        w[0], w[1], w[2] = 100.0, 1.0, 1.0
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            env = cg.BatchEnv(path, 512, seed=3, team_weights=w)
        fi = env.team_file_index
        ids = env.team_ids()
        seen = {fi[ids[g, s]] for g in range(512) for s in (0, 1)}
        assert seen == {1, 2}, seen
    finally:
        os.remove(path)


def _legal(m, g):
    return [a for a in range(A) if m[g * A + a]]


def _compare(env, name, seed):
    """builtin_actions against BuiltinPlayer game by game; returns the actions."""
    a = env.builtin_actions(name, seed=seed)
    for g in range(env.games):
        bt = env.battle(g)
        want = cg.BuiltinPlayer(name, cg.mix_seed(cg.mix_seed(seed, g), bt.decision)).action_for(bt)
        assert a[g] == want, (name, seed, g, a[g], want)
    return a


# pools whose hidden trapping abilities and Imprison make the game refuse choices the Battle allowed
POOLS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "bench", "data", "pools")


def test_builtin_actions_after_refusals():
    seen = 0
    for pool in ("hidden_trap.json", "imprison.json"):
        env = cg.BatchEnv(os.path.join(POOLS, pool), 64, seed=8, threads=4)
        rng = random.Random(6)
        for step in range(3000):
            _, _, m = env._e.observe()
            env.act([rng.choice(_legal(m, g)) for g in range(env.games)])
            lab = env.labels()
            if any(lab.refused[g, 0] or lab.refused[g, 1] for g in range(env.games)):
                seen += 1
                for name in cg.BUILTIN_PLAYERS:
                    _compare(env, name, step)
            if seen >= 30:
                break
    assert seen >= 10, seen


def test_builtin_actions_match_builtin_player():
    """builtin_actions gives BuiltinPlayer's action game by game, for every kind of decision, and the same
    for any thread count."""
    kinds = {}
    for name in cg.BUILTIN_PLAYERS:
        envs = [cg.BatchEnv(TEAMS, 48, seed=17, threads=t) for t in (1, 8)]
        rng = random.Random(2)
        for step in range(220):
            seed = 1000 + step
            a = envs[0].builtin_actions(name, seed=seed)
            b = envs[1].builtin_actions(name, seed=seed)
            assert list(a) == list(b), (name, step)
            env = envs[0]
            seat, kind = env._e.pending()
            assert list(_compare(env, name, seed)) == list(a)
            for g in range(env.games):
                kinds[kind[g]] = kinds.get(kind[g], 0) + 1
            # mostly the player's own actions, some random ones for variety
            _, _, m = env._e.observe()
            acts = [a[g] if rng.random() < 0.8 else rng.choice(_legal(m, g)) for g in range(env.games)]
            for e in envs:
                e.act(acts)
    assert kinds.get(cg.KIND_PREVIEW, 0) > 100 and kinds.get(cg.KIND_TURN, 0) > 1000 and kinds.get(cg.KIND_REPLACE, 0) > 100, kinds
    # the int64 and int32 out buffers; an unknown name
    env = cg.BatchEnv(TEAMS, 4, seed=1)
    import array

    o32 = array.array("i", [0] * 4)
    o64 = array.array("q", [0] * 4)
    env.builtin_actions("tower", out=o32)
    env.builtin_actions("tower", out=o64)
    assert list(o32) == list(o64) == list(env.builtin_actions("tower"))
    _raises(cg.CGError, env.builtin_actions, "nobody")
    _raises(cg.CGError, env.builtin_actions, "tower", out=array.array("i", [0] * 3))


def test_act_with_opponent():
    """act(actions, opponent=...) equals the hand-written pattern, ignores the learner's entries for the
    opponent's games, and plays whole games."""
    n = 64
    agent = [g % 2 for g in range(n)]
    e1 = cg.BatchEnv(TEAMS, n, seed=9, agent_seat=agent, threads=4)
    e2 = cg.BatchEnv(TEAMS, n, seed=9, agent_seat=agent)
    rng = random.Random(5)
    ours = 0
    for _ in range(600):
        seat, _ = e1._e.pending()
        _, _, m = e1._e.observe()
        learner = [rng.choice(_legal(m, g)) if seat[g] == agent[g] else 999 for g in range(n)]
        ours += sum(seat[g] == agent[g] for g in range(n))
        e1.act(learner, opponent="tower", opponent_seed=4)
        opp = e2.builtin_actions("tower", seed=4)
        s2, _ = e2._e.pending()
        e2.act([learner[g] if s2[g] == agent[g] else opp[g] for g in range(n)])
        assert e1._e.observe() == e2._e.observe() and e1._e.results() == e2._e.results() and e1._e.labels() == e2._e.labels()
    assert e1.games_done > 50 and ours > 0
    # an illegal learner action is still refused
    seat, _ = e1._e.pending()
    g = next(g for g in range(n) if seat[g] == agent[g])
    bad = [0] * n
    bad[g] = 999
    _raises(cg.CGError, e1.act, bad, opponent="tower")
    _raises(cg.CGError, e1.act, [0] * n, opponent="nobody")


PORTED = ("best_matchup_max_damage", "disciplined", "endgame", "pilot_encore", "pilot_weather", "pilot_hazard", "pilot_priority")


def test_ported_players_everywhere():
    """Each ported player by name: play_games, builtin_player, builtin_actions (any thread count) and
    act(opponent=) agree and finish games."""
    n = 32
    agent = [g % 2 for g in range(n)]
    for name in PORTED:
        assert name in cg.BUILTIN_PLAYERS
        r = cg.play_games(name, "tower", 16, TEAMS, seed=2, batch=16)
        assert len(r.games) == 16 and r.truncated == 0, (name, r.truncated)
        p = cg.builtin_player(name, 3)
        assert p.name == name
        e1 = cg.BatchEnv(TEAMS, n, seed=11, agent_seat=agent, threads=4)
        e2 = cg.BatchEnv(TEAMS, n, seed=11, agent_seat=agent)
        rng = random.Random(7)
        for step in range(300):
            seat, _ = e1._e.pending()
            _, _, m = e1._e.observe()
            learner = [rng.choice(_legal(m, g)) if seat[g] == agent[g] else 999 for g in range(n)]
            e1.act(learner, opponent=name, opponent_seed=5)
            opp = e2.builtin_actions(name, seed=5)
            s2, _ = e2._e.pending()
            e2.act([learner[g] if s2[g] == agent[g] else opp[g] for g in range(n)])
            assert e1._e.observe() == e2._e.observe() and e1._e.results() == e2._e.results(), (name, step)
        assert e1.games_done > 20, (name, e1.games_done)


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
