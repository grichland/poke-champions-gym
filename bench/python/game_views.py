"""Timing of a Game's views and the calls around them (one process, the calling thread unless stated):
(a) fork + Game.tensors one game at a time; (b) forks then one game_tensors call into preallocated
arrays; (c) BatchEnv.fork alone and fork + step; (d) Game.outcomes on a sample of positions; (e) a Game
from a live Session (Session.game); (f) Game.sample with n = 100. (c) and (d) also run on a package
without views (an older build, given by PYTHONPATH) for the before/after comparison: there the calls the
older build lacks are skipped. Needs no numpy.

Usage: PYTHONPATH=python python bench/python/game_views.py [--n 1000] [--positions 64] [--seed 1]
       [--label L] [--out FILE]
"""

import argparse
import inspect
import json
import time

import champions_gym as cg


def per_call(f, k):
    t = time.perf_counter()
    for i in range(k):
        f(i)
    return (time.perf_counter() - t) / k * 1e6


def best(f, k, rounds=5):
    return min(per_call(f, k) for _ in range(rounds))


def first_legal(g):
    m, ks = g.masks().tolist(), g.kinds()
    return [next(a for a in range(11) if m[s][a]) if ks[s] > 0 else None for s in (0, 1)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=1000)
    ap.add_argument("--positions", type=int, default=64)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--label", default="")
    ap.add_argument("--out", default=None)
    ap.add_argument("--only-c", action="store_true", help="only (c) fork and fork + step")
    a = ap.parse_args()
    views_api = "views" in inspect.signature(cg.BatchEnv.fork).parameters
    env = cg.BatchEnv(None, a.positions, seed=a.seed)
    for _ in range(40):
        env.act(env.builtin_actions("tower"))
    kinds = env.pending()[1].tolist()
    gs = [g for g in range(a.positions) if kinds[g] != cg.KIND_PREVIEW]
    seats = env.pending()[0].tolist()
    n = a.n
    rows = []

    def put(name, us, **kw):
        r = dict(label=a.label, version=cg.__version__, what=name, us=round(us, 3))
        r["threads"] = kw.pop("threads", 1)
        r.update(kw)
        rows.append(r)
        print(json.dumps(r))

    # (c) fork and fork + step
    put("c env.fork", best(lambda i: env.fork(gs[i % len(gs)], i), n))
    acts = {g: first_legal(env.fork(g, 0)) for g in gs}

    def fork_step(i, **kw):
        g = gs[i % len(gs)]
        x = env.fork(g, i, **kw)
        x.step(*acts[g])

    put("c env.fork+step", best(fork_step, n))
    root = env.fork(gs[0], 1)
    put("c game.fork", best(lambda i: root.fork(i), n))
    put("c game.fork+step", best(lambda i: root.fork(i).step(*acts[gs[0]]), n))
    if views_api:
        put("c env.fork(views=True)", best(lambda i: env.fork(gs[i % len(gs)], i, views=True), n))
        put("c env.fork(views=True)+step", best(lambda i: fork_step(i, views=True), n))
        vroot = env.fork(gs[0], 1, views=True)
        put("c game.fork+step, views carried", best(lambda i: vroot.fork(i).step(*acts[gs[0]]), n))
    if a.only_c:
        return finish(a, rows)
    # (d) outcomes on every position, first legal actions
    pos = [(env.fork(g, 0), acts[g]) for g in gs]
    leaves = sum(p.outcomes(*ac).leaves for p, ac in pos)
    t = time.perf_counter()
    for _ in range(3):
        for p, ac in pos:
            p.outcomes(*ac)
    put("d outcomes per position", (time.perf_counter() - t) / (3 * len(pos)) * 1e6, positions=len(pos), leaves=leaves)
    if views_api:
        vpos = [(env.fork(g, 0, views=True), acts[g]) for g in gs]
        t = time.perf_counter()
        for _ in range(3):
            for p, ac in vpos:
                p.outcomes(*ac)
        put("d outcomes per position, views carried", (time.perf_counter() - t) / (3 * len(vpos)) * 1e6, positions=len(vpos))
    if not views_api:
        return finish(a, rows)
    # (a) fork + tensors, one at a time
    put("a fork(views=True) + tensors", best(lambda i: env.fork(gs[i % len(gs)], i, views=True).tensors(seats[gs[i % len(gs)]]), n))
    put("a fork(views=True) + step + tensors", best(lambda i: _step_tensors(env, gs[i % len(gs)], i, acts), n))
    # (b) forks then one batched call
    for threads in (1, 4):
        out = cg.new_tensors(n)
        games = [env.fork(gs[i % len(gs)], i, views=True) for i in range(n)]
        ss = [seats[gs[i % len(gs)]] for i in range(n)]
        t = time.perf_counter()
        cg.game_tensors(games, ss, out=out, threads=threads)
        put("b game_tensors over fresh forks, per game", (time.perf_counter() - t) / n * 1e6, threads=threads, games=n)
        games = [env.fork(gs[i % len(gs)], i, views=True) for i in range(n)]
        for i, x in enumerate(games):
            x.step(*acts[gs[i % len(gs)]])
        t = time.perf_counter()
        cg.game_tensors(games, [0] * n, out=out, threads=threads)
        put("b game_tensors over stepped forks (one step to emit), per game", (time.perf_counter() - t) / n * 1e6, threads=threads, games=n)
    # (f) sample n = 100
    t = time.perf_counter()
    for p, ac in pos:
        p.sample(ac[0], ac[1], 100, seed=3)
    put("f sample n=100 per position", (time.perf_counter() - t) / len(pos) * 1e6, positions=len(pos))
    # (e) a Game from a live Session
    put("e Session.game", _session_game(), )
    return finish(a, rows)


def _step_tensors(env, g, i, acts):
    x = env.fork(g, i, views=True)
    x.step(*acts[g])
    return x.tensors(0)


def _session_game():
    """Mean microseconds of Session.game at the turn requests of a few self-play games (true sets as
    guesses)."""
    from champions_gym import _cg
    from champions_gym.live import Session

    times = []

    class P:
        def __init__(self, decs, truth):
            self.by = {s: x for s, _, x in decs}
            self.truth = truth
            self.s = None

        def teampreview(self, b):
            return self.by[-1]

        def choose_move(self, b):
            t = self.truth.get(b.decision)
            if t is not None:
                t0 = time.perf_counter()
                self.s.game(t[1], genders=t[2])
                times.append(time.perf_counter() - t0)
            return self.by[b.decision]

    for seed in range(8):
        lines, decs, truth = _cg.live_selfplay_truth(cg.TEAMS, 9000 + seed)
        p = P(decs, truth)
        s = Session(player=p)
        p.s = s
        for line in lines:
            m = json.loads(line)
            if len(m) == 1 and ("order" in m or "picks" in m):
                continue
            s.handle(m)
    return sum(times) / max(1, len(times)) * 1e6


def finish(a, rows):
    if a.out:
        with open(a.out, "a", encoding="utf-8") as f:
            for r in rows:
                f.write(json.dumps(r) + "\n")


if __name__ == "__main__":
    main()
