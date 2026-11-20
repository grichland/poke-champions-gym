"""Tests of a Game's views (`Game.battle`, `Game.tensors`, `game_tensors`), `Game.readout` and
`Game.sample`. Plain script or pytest: PYTHONPATH=python python python/tests/test_game_views.py
No numpy needed (arrays are compared as bytes)."""

import json
import math
import os
import random
import sys
import tempfile

import champions_gym as cg

A = cg.N_ACTIONS
PAIRINGS = [("tower", "max_damage"), ("random", "best_matchup_max_damage")]
VERBOSE = os.environ.get("CG_VERBOSE") == "1"
# added to every seed: CG_SEED=123 runs the same checks on other games
SEED = int(os.environ.get("CG_SEED", "0"))


def say(*a):
    if VERBOSE:
        print(*a)


def entry(arr, g, n):
    """Entry g of a batch array as bytes."""
    b = arr.tobytes()
    per = len(b) // n
    return b[g * per : (g + 1) * per]


def value(arr, g):
    return arr.tolist()[g]


def same_entry(t, obs, g, n):
    """The fields where a one-game Tensors differs from entry g of a batch."""
    bad = []
    for k in t._fields:
        a, b = getattr(t, k), getattr(obs, k)
        ok = a == value(b, g) if isinstance(a, int) else a.tobytes() == entry(b, g, n)
        if not ok:
            bad.append(k)
    return bad


def same_batch(x, y):
    return [k for k in x._fields if getattr(x, k).tobytes() != getattr(y, k).tobytes()]


def pair_actions(env, pair, seed):
    """Each game's action from the player of its pending seat."""
    seats, _ = env.pending()
    a = env.builtin_actions(pair[0], seed=seed).tolist()
    b = env.builtin_actions(pair[1], seed=seed + 1).tolist()
    return [a[g] if s == 0 else b[g] for g, s in enumerate(seats.tolist())]


def legal(mask_row):
    return [a for a in range(len(mask_row)) if mask_row[a]]


def test_untouched_fork_equals_the_environment(min_points=600):
    """T-1: on a fresh snapshot or fork, Game.tensors of the pending seat is BatchEnv's observation, and
    game_tensors over all the games is the whole batch."""
    total = replace = 0
    for rbr in (True, False):
        for pair in PAIRINGS:
            n = 24
            env = cg.BatchEnv(None, n, seed=SEED + 901 + len(pair[0]) + 7 * rbr, replace_before_rows=rbr)
            points = 0
            step = 0
            while points < min_points:
                env.act(pair_actions(env, pair, step))
                step += 1
                obs = env.observe_tensors()
                kinds = obs.kind.tolist()
                games, seats = [], []
                for g in range(n):
                    if kinds[g] == cg.KIND_PREVIEW:
                        continue
                    seat = value(obs.seat, g)
                    gm = env.snapshot(g, views=True) if g % 2 else env.fork(g, 1234 + g, views=True)
                    bad = same_entry(gm.tensors(seat), obs, g, n)
                    assert not bad, (rbr, pair, step, g, bad)
                    games.append(gm)
                    seats.append(seat)
                    points += 1
                    replace += kinds[g] == cg.KIND_REPLACE
                if games:
                    bt = cg.game_tensors(games, seats, threads=2 if step % 2 else 1)
                    idx = [g for g in range(n) if kinds[g] != cg.KIND_PREVIEW]
                    for i, g in enumerate(idx):
                        for k in bt._fields:
                            assert entry(getattr(bt, k), i, len(idx)) == entry(getattr(obs, k), g, n), (k, g)
            total += points
    say("T-1 decision points", total, "replace", replace)
    assert replace > 0
    return total, replace


def test_stepped_views_follow_the_environment(min_games=125):
    """T-2: a snapshot played forward with the environment's actions shows, at every later decision, the
    environment's observation (same seed state, so the same continuation)."""
    done = compared = dropped = auto_steps = 0
    for rbr in (True, False):
        for pair in PAIRINGS:
            n = 16
            env = cg.BatchEnv(None, n, seed=SEED + 77 + 3 * rbr + len(pair[1]), replace_before_rows=rbr)
            tracked = {}  # g -> [Game, actions given since its last step]
            prev = {}  # g -> (seat, action) of the last act
            finished = 0
            step = 0
            while finished < min_games:
                obs = env.observe_tensors()
                kinds, seats = obs.kind.tolist(), obs.seat.tolist()
                for g in range(n):
                    if g not in tracked and kinds[g] == cg.KIND_TURN and random.Random(step * 31 + g).random() < 0.3:
                        # taken between the seats' answers: seat 0's answer is already in
                        # (seat 0 was asked right before: the decision ids follow)
                        p = prev.get(g)
                        given = {0: p[1]} if seats[g] == 1 and p and p[0] == 0 and p[2] + 1 == value(obs.decision, g) else {}
                        tracked[g] = [env.snapshot(g, views=True), given]
                for g, tr in tracked.items():
                    bad = same_entry(tr[0].tensors(seats[g]), obs, g, n)
                    assert not bad, (rbr, pair, step, g, bad)
                    compared += 1
                acts = pair_actions(env, pair, step)
                for g, tr in tracked.items():
                    tr[1][seats[g]] = acts[g]
                dec = obs.decision.tolist()
                prev = {g: (seats[g], acts[g], dec[g]) for g in range(n)}
                refusals = env.counters()[0]
                env.act(acts)
                step += 1
                if env.counters()[0] != refusals:
                    # a refusal (the State refused what the Battle allowed): Game.step takes State-legal
                    # actions only, so the games followed are dropped
                    dropped += len(tracked)
                    tracked = {}
                res = env.results()
                done_g = res.done.tolist()
                for g in list(tracked):
                    gm, given = tracked[g]
                    now = env.snapshot(g, views=False) if not done_g[g] else None
                    if now is not None and now.same_state(gm):
                        continue  # the step is not resolved yet
                    m = gm.masks().tolist()
                    ks = gm.kinds()
                    a = [None, None]
                    for s in (0, 1):
                        if ks[s] > 0:
                            a[s] = given.get(s, legal(m[s])[0])  # a locked seat: its one action
                    gm.step(a[0], a[1])
                    given.clear()
                    # both seats locked: the environment takes the next step too, asking no one
                    for _ in range(6):
                        if gm.ended or (now is not None and now.same_state(gm)) or any(gm.tensors(x).kind != cg.KIND_NONE for x in (0, 1)):
                            break
                        m, ks = gm.masks().tolist(), gm.kinds()
                        gm.step(*[legal(m[x])[0] if ks[x] > 0 else None for x in (0, 1)])
                        auto_steps += 1
                    if done_g[g]:
                        assert gm.ended, (g, step)
                        for s in (0, 1):
                            assert gm.tensors(s).kind == cg.KIND_NONE
                            assert gm.battle(s).finished
                        del tracked[g]
                        finished += 1
                    else:
                        assert now.same_state(gm), (rbr, pair, step, g)
            done += finished
    say("T-2 games to the end", done, "decisions compared", compared, "dropped at a refusal", dropped, "unasked steps", auto_steps)
    return done, compared


def test_hidden_information_does_not_show(games=40):
    """T-3: two games that differ only in what seat 0 cannot see (the opponent's three Pokémon not
    brought: nature, stat points, move order) give seat 0 the same Game.tensors at every decision."""
    with open(cg.TEAMS, encoding="utf-8") as f:
        teams = json.load(f)
    natures = ["Hardy", "Bold", "Timid", "Modest", "Jolly", "Calm"]
    points = 0
    with tempfile.TemporaryDirectory() as d:
        for k in range(games):
            pool = [teams[(2 * k) % len(teams)], teams[(2 * k + 1) % len(teams)]]
            p = os.path.join(d, "a%d.json" % k)
            with open(p, "w", encoding="utf-8") as f:
                json.dump(pool, f)
            paths = [p]
            first = cg.BatchEnv(p, 1, seed=5000 + k)
            j = first.team_ids().tolist()[0][1]  # the pool team seat 1 plays
            twin = json.loads(json.dumps(pool))
            for i, m in enumerate(twin[j][3:]):
                m["nature"] = natures[(k + i) % len(natures)]
                ev = m["evs"]
                ev["hp"], ev["spe"] = ev["spe"], ev["hp"]
                m["moves"] = list(reversed(m["moves"]))
            p = os.path.join(d, "b%d.json" % k)
            with open(p, "w", encoding="utf-8") as f:
                json.dump(twin, f)
            paths.append(p)
            envs = [first, cg.BatchEnv(p, 1, seed=5000 + k)]
            # the same draw for the same seed and pool size
            assert envs[0].team_ids().tolist() == envs[1].team_ids().tolist()
            gms = [None, None]
            while envs[0].games_done == 0:
                obs = [e.observe_tensors() for e in envs]
                seat, kind = value(obs[0].seat, 0), value(obs[0].kind, 0)
                assert (seat, kind) == (value(obs[1].seat, 0), value(obs[1].kind, 0))
                if kind == cg.KIND_PREVIEW:
                    a = cg.PREVIEW0  # picks 0, 1, 2 for both seats
                else:
                    if gms[0] is None:
                        gms = [e.snapshot(0, views=True) for e in envs]
                    if seat == 0:
                        x, y = gms[0].tensors(0), gms[1].tensors(0)
                        assert not [f for f in x._fields if (getattr(x, f) if isinstance(getattr(x, f), int) else getattr(x, f).tobytes()) != (getattr(y, f) if isinstance(getattr(y, f), int) else getattr(y, f).tobytes())], (k,)
                        points += 1
                    a = legal(obs[0].action_mask.tobytes())[0]
                for e in envs:
                    e.act([a])
                if gms[0] is not None:
                    now = envs[0].snapshot(0, views=False) if envs[0].games_done == 0 else None
                    if now is None or not now.same_state(gms[0]):
                        for gm in gms:
                            m = gm.masks().tolist()
                            ks = gm.kinds()
                            gm.step(*[legal(m[s])[0] if ks[s] > 0 else None for s in (0, 1)])
    say("T-3 seat-0 decisions compared", points)
    assert points > 100
    return points


def test_outcome_children_equal_forced_steps(positions=60):
    """T-4: each child of outcomes() shows what a Game stepped with step_forced on its path shows."""
    env = cg.BatchEnv(None, 12, seed=SEED + 4242)
    rng = random.Random(3)
    checked = 0
    for step in range(400):
        env.act(pair_actions(env, PAIRINGS[0], step))
        if step % 7:
            continue
        for g in range(12):
            if checked >= positions * 4 or env.pending()[1].tolist()[g] == cg.KIND_PREVIEW:
                continue
            gm = env.snapshot(g, views=True)
            m = gm.masks().tolist()
            ks = gm.kinds()
            a = [rng.choice(legal(m[s])) if ks[s] > 0 else None for s in (0, 1)]
            out = gm.outcomes(a[0], a[1], max_branches=3000)
            for o in out.outcomes[:6]:
                h = gm.snapshot()
                h.step_forced(a[0], a[1], o.path)
                assert h.same_state(o.game)
                for s in (0, 1):
                    x, y = o.game.tensors(s), h.tensors(s)
                    assert not same_entry_one(x, y), (step, g, s)
                    assert cg._cg.live_battle_equal(o.game.battle(s), h.battle(s))
                checked += 1
    say("T-4 children checked", checked)
    assert checked > 100
    return checked


def same_entry_one(x, y):
    return [k for k in x._fields if (getattr(x, k) if isinstance(getattr(x, k), int) else getattr(x, k).tobytes()) != (getattr(y, k) if isinstance(getattr(y, k), int) else getattr(y, k).tobytes())]


def test_views_off_and_readout():
    env = cg.BatchEnv(None, 4, seed=8)
    for step in range(12):
        env.act(env.builtin_actions("tower"))
    g = env.fork(0, 5, views=False)
    assert not g.has_views
    try:
        g.tensors(0)
        raise AssertionError("no views: tensors must raise")
    except cg.CGError:
        pass
    r = env.snapshot(1, views=True).readout()
    hp = r.hp.tolist()
    assert all(0.0 <= x <= 1.0 for row in hp for x in row)
    act = r.active.tolist()
    assert all(-1 <= a <= 2 for a in act)
    fainted = r.fainted.tolist()
    for s in (0, 1):
        for i in range(3):
            assert (hp[s][i] == 0.0) == bool(fainted[s][i])


def chi2_upper(df, x):
    """P(chi-square with `df` degrees of freedom >= x): the regularized upper incomplete gamma Q(df/2, x/2)."""
    a, x = df / 2.0, x / 2.0
    if x <= 0:
        return 1.0
    lead = math.exp(-x + a * math.log(x) - math.lgamma(a))
    if x < a + 1:  # series for the lower part
        t = s = 1.0 / a
        ap = a
        for _ in range(100000):
            ap += 1
            t *= x / ap
            s += t
            if abs(t) < abs(s) * 1e-15:
                break
        return max(0.0, 1.0 - s * lead)
    b, c, d = x + 1 - a, 1e300, 1.0 / (x + 1 - a)  # continued fraction for the upper part
    h = d
    for i in range(1, 100000):
        an = -i * (i - a)
        b += 2
        d = an * d + b
        d = d if abs(d) > 1e-300 else 1e-300
        c = b + an / c
        c = c if abs(c) > 1e-300 else 1e-300
        d = 1.0 / d
        h *= d * c
        if abs(d * c - 1) < 1e-15:
            break
    return lead * h


def pooled_chi2(expected, observed):
    """Pearson's chi-square with every outcome expected fewer than 5 times pooled into one bin:
    (statistic, degrees of freedom, p-value)."""
    ex, ob, pe, po = [], [], 0.0, 0
    for e, o in zip(expected, observed):
        if e >= 5:
            ex.append(e)
            ob.append(o)
        else:
            pe += e
            po += o
    if pe > 0:
        ex.append(pe)
        ob.append(po)
    stat = sum((o - e) ** 2 / e for e, o in zip(ex, ob))
    df = len(ex) - 1
    return stat, df, chi2_upper(df, stat) if df > 0 else 1.0


#: a position fails T-6 when its pooled chi-square p-value is below this: with about 12 positions a run,
#: a correct sampler fails a run with probability about 12 * 1e-6
P_FAIL = 1e-6


def test_sample_frequencies_match_enumeration(n=4000):
    """T-6: on steps whose enumeration is complete, `sample` draws with the enumerated probabilities: a
    pooled chi-square per position (outcomes expected fewer than 5 times in one bin) must have p >=
    `P_FAIL`. Exact checks: draws k use distinct seeds `mix_seed(seed, k)`, merged draws count n in all,
    every sampled State is one enumerated State, every path forces its State again, and the same seed gives
    the same draws."""
    env = cg.BatchEnv(None, 8, seed=SEED + 31)
    rng = random.Random(9)
    positions = 0
    pmin = 1.0
    for step in range(300):
        env.act(pair_actions(env, PAIRINGS[0], step))
        if step % 11 or positions >= 12:
            continue
        for g in range(8):
            if positions >= 12 or env.pending()[1].tolist()[g] == cg.KIND_PREVIEW:
                continue
            gm = env.snapshot(g, views=True)
            m = gm.masks().tolist()
            ks = gm.kinds()
            a = [rng.choice(legal(m[s])) if ks[s] > 0 else None for s in (0, 1)]
            e = gm.outcomes(a[0], a[1], max_branches=20000)
            if e.capped or len(e.outcomes) < 2:
                continue
            seed = SEED * 1000 + step * 100 + g
            assert len({cg.mix_seed(seed, k) for k in range(n)}) == n, "repeated draw seeds"
            s = gm.sample(a[0], a[1], n, seed=seed)
            assert s.sampled and s.leaves == n and sum(o.leaves for o in s.outcomes) == n
            assert abs(sum(o.p for o in s.outcomes) - 1) < 1e-9
            count = [0] * len(e.outcomes)
            for o in s.outcomes:
                hit = [i for i, x in enumerate(e.outcomes) if x.game.same_state(o.game)]
                assert len(hit) == 1, "a sampled State not among the enumerated ones"
                count[hit[0]] += o.leaves
                h = gm.snapshot()
                h.step_forced(a[0], a[1], o.path)
                assert h.same_state(o.game)
                assert not same_entry_one(h.tensors(0), o.game.tensors(0))
            stat, df, pv = pooled_chi2([n * x.p for x in e.outcomes], count)
            assert pv >= P_FAIL, (step, g, len(e.outcomes), stat, df, pv)
            pmin = min(pmin, pv)
            P_VALUES.append(pv)
            s2 = gm.sample(a[0], a[1], n, seed=seed)
            assert [(o.p, o.path) for o in s2.outcomes] == [(o.p, o.path) for o in s.outcomes]
            positions += 1
    say("T-6 positions", positions, "smallest chi-square p-value %.3g" % pmin)
    assert positions >= 8
    return positions, pmin


P_VALUES = []


if __name__ == "__main__":
    tests = [(n, f) for n, f in sorted(globals().items()) if n.startswith("test_") and callable(f)]
    failed = 0
    for name, f in tests:
        try:
            f()
            print("PASS", name)
        except Exception as e:  # report every test
            failed += 1
            import traceback

            traceback.print_exc()
            print("FAIL", name, type(e).__name__, e)
    print("%d passed, %d failed" % (len(tests) - failed, failed))
    sys.exit(1 if failed else 0)
