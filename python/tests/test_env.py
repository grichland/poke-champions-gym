"""Tests of the champions_gym Python interface. Run with pytest, or as a plain script:
PYTHONPATH=python python python/tests/test_env.py
They need no numpy (they read the raw bytes the extension returns)."""

import random
import sys

import champions_gym as cg

TEAMS = cg.TEAMS
A = cg.N_ACTIONS


def random_actions(env, rng):
    """One uniformly random legal action per game, from the raw mask bytes."""
    _, _, m = env._e.observe()
    out = []
    for g in range(env.games):
        row = m[g * A : (g + 1) * A]
        legal = [a for a in range(A) if row[a]]
        assert legal, "game %d has no legal action" % g
        out.append(rng.choice(legal))
    return out


def test_preview_layout():
    seen = set()
    for a in range(cg.PREVIEW0, A):
        p = cg.preview_picks(a)
        assert len(set(p)) == 3 and all(0 <= x < 6 for x in p)
        assert cg.preview_action(p) == a
        assert cg.preview_action((p[0], p[2], p[1])) == a
        seen.add((p[0], frozenset(p[1:])))
    assert len(seen) == 60


def test_same_seed_same_games():
    def run(seed):
        env = cg.BatchEnv(TEAMS, 16, seed=seed)
        rng = random.Random(5)
        trace = []
        for _ in range(400):
            trace.append(env._e.observe())
            env.act(random_actions(env, rng))
            trace.append(env._e.results())
        return trace, env.games_done

    a, da = run(7)
    b, db = run(7)
    c, _ = run(8)
    assert da > 0 and da == db
    assert a == b
    assert a != c


def test_threads_same_games():
    """Stepping and observing on worker threads gives exactly the single-thread results."""

    def run(threads):
        env = cg.BatchEnv(TEAMS, 200, seed=13, threads=threads)
        assert env.threads == threads
        rng = random.Random(9)
        out = cg.new_tensors(env.games)
        trace = []
        for i in range(150):
            env.observe_tensors(out=out)
            if i % 25 == 0:
                trace.append([bytes(memoryview(x).cast("B")) for x in out])
            env.act(random_actions(env, rng))
            trace.append(env._e.results())
            trace.append(env._e.labels())
        return trace, env.games_done

    one, d1 = run(1)
    assert d1 > 0
    for t in (4, 16):
        many, dt = run(t)
        assert dt == d1 and many == one, "%d threads differ" % t


def test_100k_random_decisions_inside_mask():
    env = cg.BatchEnv(TEAMS, 256, seed=11)
    rng = random.Random(1)
    calls = 0
    while env.decisions < 100_000:
        acts = random_actions(env, rng)
        env.act(acts)
        calls += 1
    assert env.decisions >= 100_000
    assert env.games_done > 1000
    # an action outside the mask is refused and nothing moves
    _, _, m = env._e.observe()
    for g in range(env.games):
        row = m[g * A : (g + 1) * A]
        lo, hi = (cg.PREVIEW0, A) if env._e.pending()[1][g] == cg.KIND_PREVIEW else (0, cg.N_TURN_ACTIONS)
        bad = [a for a in range(lo, hi) if not row[a]]
        if bad:
            acts = random_actions(env, rng)
            acts[g] = bad[0]
            before = env._e.observe()
            try:
                env.act(acts)
            except cg.CGError:
                pass
            else:
                raise AssertionError("illegal action accepted")
            assert env._e.observe() == before
            break


def test_errors_are_exceptions():
    env = cg.BatchEnv(TEAMS, 4, seed=1)
    for bad in ([0, 0, 0], [1000] * 4, [-1] * 4):
        try:
            env.act(bad)
        except cg.CGError:
            pass
        else:
            raise AssertionError("accepted %r" % (bad,))
    try:
        env.snapshot(0)  # in team preview
    except cg.CGError:
        pass
    else:
        raise AssertionError("snapshot during preview")
    try:
        env.snapshot(9)
    except IndexError:
        pass
    else:
        raise AssertionError("bad index")
    try:
        cg.BatchEnv("no/such/teams.json", 4)
    except cg.CGError:
        pass
    else:
        raise AssertionError("missing team file")


def _in_turn(env, rng, g, steps):
    """Plays random actions until game g has gone `steps` decisions past preview and waits on seat 0."""
    n = 0
    while True:
        seat, kind = env._e.pending()
        if kind[g] != cg.KIND_PREVIEW and seat[g] == 0:
            n += 1
            if n > steps:
                return
        env.act(random_actions(env, rng))


def test_fork_reproduces():
    rng = random.Random(3)
    env = cg.BatchEnv(TEAMS, 8, seed=21)
    checked = 0
    for trial in range(40):
        g = trial % env.games
        _in_turn(env, rng, g, rng.randrange(6))
        snap = env.snapshot(g)
        k = snap.kinds()
        sm = snap.masks()
        # the env asks the deciding seats in order; the snapshot steps once with both. Under the Battle
        # rule a seat locked into a move is not asked (it takes the State's first legal action), and the
        # step may resolve after seat 0 alone.
        chosen = [None, None]
        ended = resolved = False
        for s in (0, 1):
            if k[s] < 0:
                continue
            if resolved or (s == 1 and not snap.same_state(env.snapshot(g))):
                chosen[s] = _legal(sm, s)[0]
                continue
            seat, _ = env._e.pending()
            assert seat[g] == s
            acts = random_actions(env, rng)
            # an action the State also allows, so that the env does not refuse it and ask again
            row = env._e.masks()[g * A : (g + 1) * A]
            both = [x for x in _legal(sm, s) if row[x]]
            if both:
                acts[g] = rng.choice(both)
            chosen[s] = acts[g]
            env.act(acts)
            if env._e.results()[0][g]:
                ended = resolved = True
        snap.step(chosen[0], chosen[1])
        if ended:
            assert snap.ended
            continue
        assert snap.same_state(env.snapshot(g))
        checked += 1
        # a snapshot replays the same continuation; forks with one seed agree with each other
        a, b = snap.snapshot(), snap.snapshot()
        f1, f2 = snap.fork(99), snap.fork(99)
        for x, y in ((a, b), (f1, f2)):
            for _ in range(30):
                if x.ended:
                    break
                m = x.masks()
                acts = [rng.choice(_legal(m, s)) if kk >= 0 else None for s, kk in enumerate(x.kinds())]
                x.step(*acts)
                y.step(*acts)
                assert x.same_state(y)
    assert checked >= 10


def _mask_at(m, s, x):
    return bool(m[s, x])


def _legal(m, s):
    return [x for x in range(cg.N_TURN_ACTIONS) if _mask_at(m, s, x)]


def test_enumerate_sums_to_one():
    rng = random.Random(4)
    env = cg.BatchEnv(TEAMS, 8, seed=31)
    done = 0
    multi = 0
    for trial in range(60):
        g = trial % env.games
        _in_turn(env, rng, g, rng.randrange(8))
        game = env.snapshot(g)
        m = game.masks()
        acts = [rng.choice(_legal(m, s)) if k >= 0 else None for s, k in enumerate(game.kinds())]
        for merge in (True, False):
            res = game.outcomes(acts[0], acts[1], merge=merge, max_branches=20_000)
            if res.capped:
                continue
            total = sum(o.p for o in res.outcomes)
            assert abs(total - 1.0) < 1e-9, total
            assert abs(res.covered - 1.0) < 1e-9
            multi += len(res.outcomes) > 1
            # each outcome is reproduced by forcing its path
            for o in res.outcomes[:20]:
                h = game.snapshot()
                h.step_forced(acts[0], acts[1], o.path)
                assert h.same_state(o.game)
        done += 1
    assert done >= 40 and multi >= 10


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
