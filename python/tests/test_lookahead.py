"""The lookahead built-in player through every by-name entry point: BuiltinPlayer, play_games,
builtin_actions (any thread count), act(opponent=...), a live Session; names with options; same seed, same
games. Plain-script mode: PYTHONPATH=python python python/tests/test_lookahead.py (no numpy needed).
CG_SEED=<n> changes the seeds."""

import json
import os
import random
import sys

import champions_gym as cg
from champions_gym.live import Session

TEAMS = cg.TEAMS
SEED = int(os.environ.get("CG_SEED", "0"))
DATA = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")


def _legal(mask, g):
    row = mask[g * cg.N_ACTIONS : (g + 1) * cg.N_ACTIONS]
    return [a for a in range(cg.N_ACTIONS) if row[a]]


def _raises(exc, f, *a, **k):
    try:
        f(*a, **k)
    except exc as e:
        return str(e)
    raise AssertionError("no %s from %s" % (exc.__name__, getattr(f, "__name__", f)))


def test_names():
    assert "lookahead" in cg.BUILTIN_PLAYERS and "lookahead2" in cg.BUILTIN_PLAYERS
    assert cg.BUILTIN_PLAYERS[:10] == ("random", "max_damage", "tower", "best_matchup_max_damage", "disciplined", "endgame",
                                       "pilot_encore", "pilot_weather", "pilot_hazard", "pilot_priority")
    for name in ("lookahead", "lookahead2", "lookahead@samples=2,replies=1", "lookahead@depth=2,seed=4,fill=1"):
        assert cg.builtin_player(name, 1).name == name
        assert cg._cg.live_has_player(name)
    for bad in ("lookahead@depth=5", "lookahead@x=1", "lookahead@fill=0", "lookahead3"):
        _raises(cg.CGError, cg.builtin_player, bad)
        assert not cg._cg.live_has_player(bad)
    _raises(cg.CGError, cg.BatchEnv(TEAMS, 2, seed=1).builtin_actions, "lookahead@samples=99")


def test_play_games_and_seed():
    r = cg.play_games("lookahead", "tower", 24, TEAMS, seed=SEED + 3, batch=12)
    assert len(r.games) == 24 and r.truncated == 0
    again = cg.play_games("lookahead", "tower", 24, TEAMS, seed=SEED + 3, batch=12)
    assert [(g.winner, g.turns) for g in r.games] == [(g.winner, g.turns) for g in again.games]
    r2 = cg.play_games("max_damage", "lookahead@depth=2,samples=2", 8, TEAMS, seed=SEED + 4, batch=8)
    assert len(r2.games) == 8 and r2.truncated == 0


def test_builtin_actions_any_threads():
    """builtin_actions equals BuiltinPlayer game by game and is the same for 1 and 6 threads, at preview,
    turn and replace decisions."""
    kinds = {}
    for name in ("lookahead", "lookahead@samples=2,replies=3"):
        envs = [cg.BatchEnv(TEAMS, 40, seed=SEED + 21, threads=t) for t in (1, 6)]
        rng = random.Random(SEED)
        for step in range(150):
            a = envs[0].builtin_actions(name, seed=SEED + step)
            b = envs[1].builtin_actions(name, seed=SEED + step)
            assert list(a) == list(b), (name, step)
            env = envs[0]
            seat, kind = env._e.pending()
            _, _, m = env._e.observe()
            for g in range(env.games):
                kinds[kind[g]] = kinds.get(kind[g], 0) + 1
                assert m[g * cg.N_ACTIONS + a[g]], (name, step, g, a[g])
                s = cg.mix_seed(cg.mix_seed(SEED + step, g), env.battle(g).decision)
                assert cg.BuiltinPlayer(name, s).action_for(env.battle(g)) == a[g]
            acts = [a[g] if rng.random() < 0.8 else rng.choice(_legal(m, g)) for g in range(env.games)]
            for e in envs:
                e.act(acts)
    assert kinds.get(cg.KIND_PREVIEW, 0) > 50 and kinds.get(cg.KIND_TURN, 0) > 500 and kinds.get(cg.KIND_REPLACE, 0) > 50, kinds


def test_act_with_opponent():
    n = 32
    agent = [g % 2 for g in range(n)]
    e1 = cg.BatchEnv(TEAMS, n, seed=SEED + 9, agent_seat=agent, threads=4)
    e2 = cg.BatchEnv(TEAMS, n, seed=SEED + 9, agent_seat=agent)
    rng = random.Random(SEED + 5)
    for _ in range(250):
        seat, _ = e1._e.pending()
        _, _, m = e1._e.observe()
        learner = [rng.choice(_legal(m, g)) if seat[g] == agent[g] else 999 for g in range(n)]
        e1.act(learner, opponent="lookahead", opponent_seed=4)
        opp = e2.builtin_actions("lookahead", seed=4)
        s2, _ = e2._e.pending()
        e2.act([learner[g] if s2[g] == agent[g] else opp[g] for g in range(n)])
        assert e1._e.observe() == e2._e.observe() and e1._e.results() == e2._e.results()
    assert e1.games_done > 20
    # the learner (random) rarely beats it
    assert e1.counters()[0] <= e1.decisions // 50


def test_live_session():
    """A recorded phone battle answered by lookahead: every reply an order or picks, no error."""
    s = Session(player="lookahead", seed=SEED + 1)
    assert s.player_name == "lookahead"
    replies = 0
    for line in open(os.path.join(DATA, "live-phone-battle.jsonl"), encoding="utf-8"):
        r = s.handle(json.loads(line))
        if r:
            assert "error" not in r, r
            assert r.get("picks") or r.get("order"), r
            replies += 1
    assert replies > 3
    s2 = Session(player="lookahead@depth=2,samples=2", seed=SEED + 1)
    assert s2.player_name == "lookahead@depth=2,samples=2"


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
