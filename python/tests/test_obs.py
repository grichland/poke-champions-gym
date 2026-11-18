"""The observation arrays through Python. Plain-script mode, no numpy needed:
PYTHONPATH=python python python/tests/test_obs.py"""

import random
import sys

import champions_gym as cg

TEAMS = cg.TEAMS
A = cg.N_ACTIONS


def _random_actions(env, rng):
    m = env._e.masks()
    return [rng.choice([a for a in range(A) if m[g * A + a]]) for g in range(env.games)]


def _list(x):
    return x.tolist()


def _at(x, g):
    """Entry g of a batch array as lists (a memoryview has no sub-views of several dimensions)."""
    return x[g].tolist() if hasattr(x, "dtype") else x.tolist()[g]


def test_shapes_and_dtypes():
    env = cg.BatchEnv(TEAMS, 5, seed=2)
    t = env.observe_tensors()
    for name in cg.Tensors._fields:
        arr = getattr(t, name)
        assert tuple(arr.shape) == (5,) + cg.SHAPES[name], (name, arr.shape)
        fmt = getattr(arr, "format", None) or arr.dtype.char
        assert fmt in {"float32": "f", "int32": "il", "uint8": "B", "int64": "qlL"}[cg.DTYPES[name]], (name, fmt)
    b = env.battle(0).tensors()
    for name in cg.Tensors._fields[:7]:
        assert tuple(getattr(b, name).shape) == cg.SHAPES[name]


def test_layout_covers_every_column():
    for block, width in (("rows", 300), ("field", 66), ("last", 17), ("row_ids", 10), ("last_ids", 4), ("mask", 11)):
        cols = []
        for f in cg.FEATURES.values():
            if f.block == block:
                cols += f.columns()
        assert sorted(cols) == list(range(width)), block
    ids = [f for f in cg.FEATURES.values() if f.block in ("row_ids", "last_ids")]
    assert all(f.vocab in cg.VOCAB and f.vocab_size == cg.VOCAB[f.vocab] for f in ids)


def test_batch_matches_battle_view_and_ranges():
    env = cg.BatchEnv(TEAMS, 16, seed=4)
    rng = random.Random(2)
    out = cg.new_tensors(env.games)
    lo_hi = {}
    for f in cg.FEATURES.values():
        if f.block in ("rows", "field", "last"):
            for c in f.columns():
                lo_hi[(f.block, c)] = (f.lo, f.hi)
    kinds = set()
    for step in range(120):
        t = env.observe_tensors(out=out)
        assert t is out
        masks = env._e.masks()
        seats, k = env._e.pending()
        for g in range(env.games):
            assert t.seat[g] == seats[g] and t.kind[g] == k[g]
            kinds.add(t.kind[g])
            assert _at(t.action_mask, g) == list(masks[g * A : (g + 1) * A])
            if step % 10 == 0:
                b = env.battle(g).tensors()
                for name in cg.Tensors._fields[:7]:
                    assert _at(getattr(t, name), g) == _list(getattr(b, name)), (name, g)
                rows = _at(t.rows, g)
                for r in range(12):
                    for c, x in enumerate(rows[r]):
                        lo, hi = lo_hi[("rows", c)]
                        assert lo <= x <= hi, (r, c, x)
                for c, x in enumerate(_at(t.field, g)):
                    assert lo_hi[("field", c)][0] <= x <= lo_hi[("field", c)][1]
                if t.kind[g] == cg.KIND_PREVIEW:
                    assert not any(_at(t.mask, g))
        env.act(_random_actions(env, rng))
    assert kinds >= {cg.KIND_PREVIEW, cg.KIND_TURN}


def test_same_seed_same_tensors():
    def run(seed):
        env = cg.BatchEnv(TEAMS, 8, seed=seed)
        rng = random.Random(9)
        out = []
        for _ in range(60):
            t = env.observe_tensors()
            out.append([_list(x) for x in t])
            env.act(_random_actions(env, rng))
        return out

    assert run(3) == run(3)
    assert run(3) != run(4)


def test_bad_buffers_are_refused():
    env = cg.BatchEnv(TEAMS, 3, seed=1)
    good = cg.new_tensors(3)
    small = cg.new_tensors(2)
    for i, name in enumerate(cg.Tensors._fields):
        bad = list(good)
        bad[i] = getattr(small, name)
        try:
            env.observe_tensors(out=cg.Tensors(*bad))
        except cg.CGError:
            pass
        else:
            raise AssertionError("accepted a short %s" % name)
    try:
        env._e.observe_into(*([bytes(4 * 3 * 3600)] + list(good[1:])))
    except cg.CGError:
        pass
    else:
        raise AssertionError("accepted read-only rows")
    bad = list(good)
    bad[8] = good[7]  # kind and seat the same array
    try:
        env.observe_tensors(out=cg.Tensors(*bad))
    except cg.CGError:
        pass
    else:
        raise AssertionError("accepted shared memory")


def test_labels_follow_decisions():
    env = cg.BatchEnv(TEAMS, 8, seed=6)
    rng = random.Random(3)
    taken = {}  # (game, decision) -> (seat, action)
    labelled = 0
    for _ in range(300):
        t = env.observe_tensors()
        acts = _random_actions(env, rng)
        for g in range(env.games):
            taken[(g, int(t.decision[g]))] = (int(t.seat[g]), acts[g])
        env.act(acts)
        lab = env.labels()
        valid = _list(lab.valid)
        dec = _list(lab.decision)
        own = _list(lab.own_action)
        oa = _list(lab.opp_action)
        mk = _list(lab.opp_prior)
        for g in range(env.games):
            for s in (0, 1):
                if not valid[g][s]:
                    continue
                labelled += 1
                seat, a = taken[(g, dec[g][s])]
                assert seat == s and own[g][s] == a
                if oa[g][s] >= 0:
                    assert mk[g][s][oa[g][s]] > 0
    assert labelled > 1500


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
