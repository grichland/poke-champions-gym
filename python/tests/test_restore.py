"""Tests of `BatchEnv.restore`: a snapshot put back into a slot plays as any other game of the batch.
Plain script or pytest: PYTHONPATH=python python python/tests/test_restore.py (CG_SEED=n for other games,
CG_VERBOSE=1 for counts). No numpy needed (arrays are compared as bytes)."""

import json
import os
import random
import sys
import tempfile

import champions_gym as cg

PAIR = ("tower", "max_damage")
VERBOSE = os.environ.get("CG_VERBOSE") == "1"
# added to every seed: CG_SEED=123 runs the same checks on other games
SEED = int(os.environ.get("CG_SEED", "0"))
# label fields fixed before the step's random events (opp_move and opp_shown depend on them)
FIXED_LABELS = ("valid", "decision", "refused", "own_action", "opp_action", "opp_forced", "opp_prior", "opp_picks", "opp_true_brought", "opp_true_moves", "opp_true_item", "opp_true_ability")


def say(*a):
    if VERBOSE:
        print(*a)


def entry(arr, g, n):
    """Entry g of an [n, ...] array as bytes."""
    b = arr.tobytes()
    per = len(b) // n
    return b[g * per : (g + 1) * per]


def diff(x, gx, nx, y, gy, ny, fields=None):
    """Fields of two batch tuples (Tensors, Labels, Results) where entry gx of x differs from entry gy of y."""
    return [k for k in (fields or x._fields) if entry(getattr(x, k), gx, nx) != entry(getattr(y, k), gy, ny)]


def one_vs_batch(t, obs, g, n):
    """Fields where a one-game Tensors (Game.tensors) differs from entry g of a batch."""
    bad = []
    for k in t._fields:
        a, b = getattr(t, k), getattr(obs, k)
        ok = a == b.tolist()[g] if isinstance(a, int) else a.tobytes() == entry(b, g, n)
        if not ok:
            bad.append(k)
    return bad


def pair_actions(env, seed):
    """Each game's action from the built-in player of its pending seat."""
    seats, _ = env.pending()
    a = env.builtin_actions(PAIR[0], seed=seed).tolist()
    b = env.builtin_actions(PAIR[1], seed=seed + 1).tolist()
    return [a[g] if s == 0 else b[g] for g, s in enumerate(seats.tolist())]


def legal(row):
    return [a for a in range(len(row)) if row[a]]


class Snap:
    """A snapshot with what the source showed when it was taken."""

    def __init__(self, env, obs, g, n):
        self.game = env.snapshot(g, views=True)
        self.obs, self.g, self.n = obs, g, n
        self.seat, self.kind, self.decision = obs.seat.tolist()[g], obs.kind.tolist()[g], obs.decision.tolist()[g]
        self.team_ids = env.team_ids().tolist()[g]
        self.agent = env.agent_seat.tolist()[g]


def source(seed, n=16, rbr=True, **kw):
    return cg.BatchEnv(None, n, seed=seed, replace_before_rows=rbr, **kw)


def take(env, count, rng, step0=0, p=0.15):
    """Plays `env` with the built-in pair, snapshotting turn and replace decisions at random: [Snap]."""
    out = []
    step = step0
    while len(out) < count:
        env.act(pair_actions(env, step))
        step += 1
        obs = env.observe_tensors()
        kinds = obs.kind.tolist()
        for g in range(env.games):
            if kinds[g] != cg.KIND_PREVIEW and rng.random() < p and len(out) < count:
                out.append(Snap(env, obs, g, env.games))
    return out


def play_same(env, slots, rng, check=True):
    """Plays the restored games in `slots` with the same random actions until the first ends; with `check`,
    every observation, label and result must be equal across the slots. Returns (decisions, seats seen)."""
    n = env.games
    first = slots[0]
    points = 0
    seats = set()
    for _ in range(2000):
        obs = env.observe_tensors()
        for g in slots[1:]:
            assert not diff(obs, first, n, obs, g, n), (first, g, diff(obs, first, n, obs, g, n))
        seats.add(obs.seat.tolist()[first])
        a = rng.choice(legal(entry(obs.action_mask, first, n)))
        acts = [a if g in slots else legal(entry(obs.action_mask, g, n))[0] for g in range(n)]
        env.act(acts)
        points += 1
        lab, res = env.labels(), env.results()
        for g in slots[1:]:
            assert not diff(lab, first, n, lab, g, n), diff(lab, first, n, lab, g, n)
            assert not diff(res, first, n, res, g, n), diff(res, first, n, res, g, n)
        if res.done.tolist()[first]:
            return points, seats
    raise AssertionError("restored game did not end")


def test_same_seed_same_game(per_order=40):
    """R-1: one snapshot restored into two slots with the same seed and fed the same actions: identical
    observations (every field, both seats as they come up), labels and results to the end."""
    total = seat1 = replace = 0
    seats = set()
    for rbr in (True, False):
        rng = random.Random(SEED * 7 + rbr)
        snaps = take(source(SEED + 11 + rbr, rbr=rbr), per_order, rng)
        target = cg.BatchEnv(None, 2, seed=SEED + 5, replace_before_rows=rbr)
        for i, s in enumerate(snaps):
            seed = SEED * 1000 + i
            target.restore(0, s.game, seed)
            target.restore(1, s.game, seed)
            k, sn = play_same(target, [0, 1], rng)
            total += k
            seats |= sn
            seat1 += s.seat == 1
            replace += s.kind == cg.KIND_REPLACE
    say("R-1 snapshots", 2 * per_order, "decisions compared", total, "taken at seat 1", seat1, "at replace", replace)
    assert seats == {0, 1} and seat1 > 0 and replace > 0 and total > 500
    return total


def test_other_seeds_differ_by_luck(count=40):
    """R-2: the same snapshot with two seeds and the same actions: the first observation is identical (no
    random event yet), later ones differ in most games."""
    rng = random.Random(SEED + 2)
    snaps = take(source(SEED + 21), count, rng)
    differ = 0
    for i, s in enumerate(snaps):
        env = cg.BatchEnv(None, 2, seed=SEED + 6)
        env.restore(0, s.game, SEED * 100 + 2 * i)
        env.restore(1, s.game, SEED * 100 + 2 * i + 1)
        obs = env.observe_tensors()
        assert not diff(obs, 0, 2, obs, 1, 2)
        seen = False
        for _ in range(400):
            obs = env.observe_tensors()
            m0, m1 = legal(entry(obs.action_mask, 0, 2)), legal(entry(obs.action_mask, 1, 2))
            seen = seen or bool(diff(obs, 0, 2, obs, 1, 2))
            a = rng.choice(m0)
            env.act([a, a if a in m1 else m1[0]])
            r = env.results().done.tolist()
            if r[0] or r[1]:
                seen = seen or r[0] != r[1] or bool(diff(env.results(), 0, 2, env.results(), 1, 2))
                break
        differ += seen
    say("R-2 games", count, "that differ", differ)
    assert differ >= count * 0.6, differ
    return differ


def test_restored_slot_shows_the_original(count=60):
    """R-3: right after restore the slot's observation is the original's at the snapshot (every field) and
    the snapshot's Game.tensors; team_ids and agent_seat are the original's; with the original's actions,
    the labels' fields fixed before the step's luck are the original's."""
    rng = random.Random(SEED + 3)
    n = 16
    src = source(SEED + 31, n=n, agent_seat=[(g + 1) % 2 for g in range(n)])
    step = 0
    checked = labelled = 0
    while checked < count:
        src.act(pair_actions(src, step))
        step += 1
        obs = src.observe_tensors()
        kinds = obs.kind.tolist()
        cand = [g for g in range(n) if kinds[g] != cg.KIND_PREVIEW]
        if not cand or rng.random() > 0.3:
            continue
        g = rng.choice(cand)
        s = Snap(src, obs, g, n)
        t = cg.BatchEnv(None, 1, seed=SEED + 9)
        t.restore(0, s.game, SEED + step)
        o = t.observe_tensors()
        assert not diff(o, 0, 1, obs, g, n), diff(o, 0, 1, obs, g, n)
        assert not one_vs_batch(s.game.tensors(s.seat), o, 0, 1)
        assert t.team_ids().tolist()[0] == s.team_ids
        assert t.agent_seat.tolist()[0] == s.agent
        # the original's actions until the step resolves
        for _ in range(3):
            if t.pending()[1].tolist()[0] == cg.KIND_PREVIEW or not src.snapshot(g).same_state(t.snapshot(0)):
                break
            o, so = t.observe_tensors(), src.observe_tensors()
            assert not diff(o, 0, 1, so, g, n)
            acts = pair_actions(src, step)
            step += 1
            src.act(acts)
            t.act([acts[g]])
            la, lb = t.labels(), src.labels()
            assert not diff(la, 0, 1, lb, g, n, FIXED_LABELS), diff(la, 0, 1, lb, g, n, FIXED_LABELS)
            labelled += sum(la.valid.tolist()[0])
            if src.results().done.tolist()[g]:
                break
        checked += 1
    say("R-3 restores compared", checked, "labels compared", labelled)
    assert labelled > count
    return checked


def run_slots(threads, snaps, into, calls=300):
    """A 48-game batch played by the built-in pair, restoring the snapshots in turn into the slots `into`
    as their games end: per slot, everything it showed after each act."""
    n = 48
    env = cg.BatchEnv(None, n, seed=SEED + 77, threads=threads)
    trace = [[] for _ in range(n)]
    k = 0
    for call in range(calls):
        env.act(pair_actions(env, call))
        done = env.results().done.tolist()
        for g in into:
            if done[g]:
                env.restore(g, snaps[k % len(snaps)].game, SEED + k)
                k += 1
        obs, lab, res, ids = env.observe_tensors(), env.labels(), env.results(), env.team_ids()
        for g in range(n):
            trace[g].append((tuple(entry(getattr(obs, f), g, n) for f in obs._fields), tuple(entry(getattr(lab, f), g, n) for f in lab._fields), tuple(entry(getattr(res, f), g, n) for f in res._fields), entry(ids, g, n)))
    return trace, k


def test_threads_and_other_slots():
    """R-4: the same run with restores on 1 and 8 threads is identical; every slot not restored into is
    identical to a run without restores."""
    snaps = take(source(SEED + 41), 6, random.Random(SEED + 4))
    into = [5, 20, 41]
    one, k = run_slots(1, snaps, into)
    eight, _ = run_slots(8, snaps, into)
    assert one == eight
    plain, _ = run_slots(8, snaps, [])
    for g in range(48):
        assert (plain[g] == one[g]) == (g not in into), g
    say("R-4 restores", k)
    assert k >= 6
    return k


def test_builtin_opponents_to_the_end(games=60):
    """R-5: restored games play to the end with act(opponent=...): no preview while they run, their
    team_ids and agent seat are the snapshot's, each result is reported once under the slot's next episode,
    then the slot restarts with a preview and its own agent seat."""
    n = 32
    snaps = take(source(SEED + 51, n=16, agent_seat=[(g + 1) % 2 for g in range(16)]), 30, random.Random(SEED + 5))
    env = cg.BatchEnv(None, n, seed=SEED + 52, agent_seat=[g % 2 for g in range(n)])
    rng = random.Random(SEED + 53)
    live = {}  # g -> (Snap, episode expected)
    ended = restored = 0
    last_ep = [-1] * n
    while ended < games:
        before = env.games_done
        env.act(env.builtin_actions("max_damage"), opponent="tower")
        res = env.results()
        done, ep = res.done.tolist(), res.episode.tolist()
        assert env.games_done - before == sum(done)
        kinds = env.pending()[1].tolist()
        agent, ids = env.agent_seat.tolist(), env.team_ids().tolist()
        for g in range(n):
            if g in live:
                s, want = live[g]
                if done[g]:
                    assert ep[g] == want, (ep[g], want)
                    assert kinds[g] == cg.KIND_PREVIEW and agent[g] == g % 2
                    del live[g]
                    ended += 1
                else:
                    assert kinds[g] != cg.KIND_PREVIEW and agent[g] == s.agent and ids[g] == s.team_ids
            if done[g]:
                last_ep[g] = ep[g]
            if done[g] and g not in live and rng.random() < 0.5:
                s = rng.choice(snaps)
                r0 = [x.tobytes() for x in env.results()]
                env.restore(g, s.game, rng.randrange(1 << 62))
                # no start reported, no preview asked: the results stay the last act's
                assert [x.tobytes() for x in env.results()] == r0 and env.games_done == before + sum(done)
                o = env.observe_tensors()
                assert o.kind.tolist()[g] == s.kind and o.decision.tolist()[g] == s.decision and o.seat.tolist()[g] == s.seat
                live[g] = (s, last_ep[g] + 1)
                restored += 1
    say("R-5 restored", restored, "played to the end", ended)
    return ended


def test_repeated_and_errors():
    """R-6: one snapshot restored again and again (and its copies), after the source moved on, gives the
    same game; what restore cannot take raises CGError."""
    rng = random.Random(SEED + 6)
    src = source(SEED + 61)
    s = [x for x in take(src, 20, rng) if x.kind == cg.KIND_TURN][0]

    def trace(game, slots=1):
        env = cg.BatchEnv(None, slots, seed=SEED + 62)
        for g in range(slots):
            env.restore(g, game, 4242)
        r = random.Random(1)
        out = []
        while True:
            obs = env.observe_tensors()
            for g in range(1, slots):
                assert not diff(obs, 0, slots, obs, g, slots)
            out.append(tuple(entry(getattr(obs, f), 0, slots) for f in obs._fields))
            a = r.choice(legal(entry(obs.action_mask, 0, slots)))
            env.act([a] * slots)
            if env.results().done.tolist()[0]:
                return out + [tuple(entry(x, 0, slots) for x in env.results())]

    first = trace(s.game)
    for k in range(40):
        src.act(pair_actions(src, 1000 + k))
    assert trace(s.game, 3) == first
    assert trace(s.game) == first
    assert s.game.restorable and s.game.snapshot().restorable and s.game.fork(9).restorable
    assert trace(s.game.snapshot()) == first and trace(s.game.fork(9)) == first

    def refused(f, why=None):
        try:
            f()
        except cg.CGError as e:
            assert why is None or why in str(e), (why, str(e))
            return
        raise AssertionError("restore should raise CGError")

    t = cg.BatchEnv(None, 2, seed=1)
    g = s.g
    while src.pending()[1].tolist()[g] == cg.KIND_PREVIEW:
        src.act(pair_actions(src, 7))
    plain = src.snapshot(g)
    assert not plain.restorable
    refused(lambda: t.restore(0, plain, 1), "views=True")
    refused(lambda: t.restore(0, src.fork(g, 1, views=True), 1), "views=True")
    stepped = s.game.snapshot()
    m, ks = stepped.masks().tolist(), stepped.kinds()
    stepped.step(*[legal(m[x])[0] if ks[x] > 0 else None for x in (0, 1)])
    assert not stepped.restorable
    refused(lambda: t.restore(0, stepped, 1), "do not step")
    child = s.game.outcomes(*[legal(m[x])[0] if ks[x] > 0 else None for x in (0, 1)]).outcomes[0].game
    refused(lambda: t.restore(0, child, 1))
    import itertools

    with open(cg.TEAMS, encoding="utf-8") as f:
        teams = json.load(f)
    b = src.battle(g)
    opp = teams[src.team_file_index[src.team_ids().tolist()[g][1 - b.seat]]]
    live_game = None
    for trio in itertools.permutations(range(6), 3):  # the first guess the Battle accepts
        try:
            live_game, _ = cg.game_from_battle(b, [{"slot": i, "moves": opp[i]["moves"]} for i in trio], seat=b.seat)
            break
        except cg.CGError:
            continue
    assert not live_game.restorable
    refused(lambda: t.restore(0, live_game, 1), "views=True")
    # a running slot, a slot in preview with picks given
    t.restore(0, s.game, 1)
    refused(lambda: t.restore(0, s.game, 2), "being played")
    obs = t.observe_tensors()
    t.act([legal(entry(obs.action_mask, 0, 2))[0], cg.PREVIEW0])
    refused(lambda: t.restore(1, s.game, 2), "being played")
    try:
        t.restore(5, s.game, 1)
        raise AssertionError("slot out of range")
    except IndexError:
        pass
    # other settings, other teams; the same teams from the same file are accepted
    for kw in ({"replace_before_rows": False}, {"rule": "state"}, {"team_mode": "stage3"}):
        refused(lambda: cg.BatchEnv(None, 1, **kw).restore(0, s.game, 1), "other settings")
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "few.json")
        with open(p, "w", encoding="utf-8") as f:
            json.dump(teams[:10], f)
        refused(lambda: cg.BatchEnv(p, 1).restore(0, s.game, 1), "other teams")
    cg.BatchEnv(cg.TEAMS, 1, seed=3, max_steps=50, team_weights=[1.0] * len(teams)).restore(0, s.game, 1)
    return len(first)


def test_hidden_information_does_not_show(games=30):
    """R-7: two games that differ only in what seat 0 cannot see (the opponent's three Pokémon not brought:
    nature, stat points, move order), snapshotted at the same point and restored with the same seed into
    environments of their own pools, give seat 0 the same observation at every decision."""
    with open(cg.TEAMS, encoding="utf-8") as f:
        teams = json.load(f)
    natures = ["Hardy", "Bold", "Timid", "Modest", "Jolly", "Calm"]
    points = 0
    with tempfile.TemporaryDirectory() as d:
        for k in range(games):
            pool = [teams[(2 * k + SEED) % len(teams)], teams[(2 * k + 1 + SEED) % len(teams)]]
            paths = [os.path.join(d, "a%d.json" % k), os.path.join(d, "b%d.json" % k)]
            with open(paths[0], "w", encoding="utf-8") as f:
                json.dump(pool, f)
            first = cg.BatchEnv(paths[0], 1, seed=SEED + 5000 + k)
            j = first.team_ids().tolist()[0][1]  # the pool team seat 1 plays
            twin = json.loads(json.dumps(pool))
            for i, m in enumerate(twin[j][3:]):
                m["nature"] = natures[(k + i) % len(natures)]
                ev = m["evs"]
                ev["hp"], ev["spe"] = ev["spe"], ev["hp"]
                m["moves"] = list(reversed(m["moves"]))
            with open(paths[1], "w", encoding="utf-8") as f:
                json.dump(twin, f)
            envs = [first, cg.BatchEnv(paths[1], 1, seed=SEED + 5000 + k)]
            # play both (picks 0, 1, 2) to a seat-0 turn a few steps in, snapshot both there
            steps = 0
            while True:
                obs = envs[0].observe_tensors()
                kind, seat = obs.kind.tolist()[0], obs.seat.tolist()[0]
                if kind == cg.KIND_TURN and seat == 0 and steps >= 2 + k % 5:
                    break
                a = cg.PREVIEW0 if kind == cg.KIND_PREVIEW else legal(obs.action_mask.tobytes())[0]
                for e in envs:
                    e.act([a])
                steps += 1
                if envs[0].games_done:
                    break
            if envs[0].games_done:
                continue
            snaps = [e.snapshot(0, views=True) for e in envs]
            back = [cg.BatchEnv(p, 1, seed=SEED + 1) for p in paths]
            for e, sn in zip(back, snaps):
                e.restore(0, sn, SEED * 31 + k)
            r = random.Random(k)
            while not back[0].results().done.tolist()[0]:
                o = [e.observe_tensors() for e in back]
                assert (o[0].seat.tolist(), o[0].kind.tolist()) == (o[1].seat.tolist(), o[1].kind.tolist())
                if o[0].seat.tolist()[0] == 0:
                    assert not diff(o[0], 0, 1, o[1], 0, 1), (k, diff(o[0], 0, 1, o[1], 0, 1))
                    points += 1
                a = r.choice(legal(o[0].action_mask.tobytes()))
                for e in back:
                    e.act([a])
    say("R-7 seat-0 decisions compared", points)
    assert points > 100
    return points


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
