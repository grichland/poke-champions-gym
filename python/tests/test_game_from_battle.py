"""Tests of `Session.game` / `game_from_battle` (a Game from a live Battle), against self-play truth.
Plain script or pytest: PYTHONPATH=python python python/tests/test_game_from_battle.py"""

import json
import os
import sys

import champions_gym as cg
from champions_gym import _cg
from champions_gym.live import Session

VERBOSE = os.environ.get("CG_VERBOSE") == "1"
SEED = int(os.environ.get("CG_SEED", "0"))
ERRORS = []
PAIRS = [("tower", "max_damage"), ("max_damage", "tower"), ("random", "best_matchup_max_damage")]


class Scripted:
    """Answers each request with the self-play action; at each request it builds a Game and checks it."""

    def __init__(self, decs, truth, check):
        self.by_seq = {seq: a for seq, _, a in decs}
        self.truth = truth
        self.check = check
        self.session = None

    def teampreview(self, battle):
        return self.by_seq[-1]

    def choose_move(self, battle):
        if battle.decision in self.truth:
            try:
                g, guesses, genders, kind = self.truth[battle.decision]
                self.check(self.session, battle, (cg.Game(g), guesses, genders, kind))
            except Exception as e:  # the Session would swallow it
                import traceback

                ERRORS.append("seq %d: %s" % (battle.decision, traceback.format_exc()))
        return self.by_seq[battle.decision]


def same(x, y):
    return [k for k in x._fields if (getattr(x, k) if isinstance(getattr(x, k), int) else getattr(x, k).tobytes()) != (getattr(y, k) if isinstance(getattr(y, k), int) else getattr(y, k).tobytes())]


def play(seed, pair, rbr, check):
    del ERRORS[:]
    lines, decs, truth = _cg.live_selfplay_truth(cg.TEAMS, seed, pair[0], pair[1], rbr)
    p = Scripted(decs, truth, check)
    s = Session(player=p)
    p.session = s
    for line in lines:
        m = json.loads(line)
        if "order" in m and len(m) == 1 or "picks" in m and len(m) == 1:
            continue  # our own answers: the Session makes them
        s.handle(m)
    assert not ERRORS, ERRORS[0]
    return s


def replay(lines, decs, truth, check):
    del ERRORS[:]
    p = Scripted(decs, truth, check)
    s = Session(player=p)
    p.session = s
    for line in lines:
        m = json.loads(line)
        if "order" in m and len(m) == 1 or "picks" in m and len(m) == 1:
            continue
        s.handle(m)
    assert not ERRORS, ERRORS[0]


def illusion_explains(r, guesses):
    """A rejection with true guesses that an opposing Illusion user brought causes: the lead put first
    (the disguise led), or a move the Illusion user knows shown under the disguise's name."""
    illusion = [g for g in guesses if g.get("ability") == "illusion"]
    if not illusion:
        return False
    if r[1] == "brought":
        return True
    m = r[2].split(" was used", 1)
    return r[1] == "moves" and len(m) == 2 and any(m[0] in g["moves"] for g in illusion)


def test_true_guesses_rebuild_the_state(games=60):
    """T-5: with the true hidden sets as guesses, nothing is rejected (but what an opposing Illusion user
    causes, counted as `illusion_exempt`), the legal set equals the Session's,
    Game.tensors(our seat) equals cg.tensors(Session.battle), and the State equals the true one on the
    parts `state_diff` compares (counted by class)."""
    stats = {"turn": 0, "replace": 0, "rejected": 0, "illusion_exempt": 0, "legal": 0, "tensors": 0, "own_mask_true": 0, "both_masks_true": 0}

    def check(session, battle, t):
        true_game, guesses, genders, kind = t
        game, rejected = session.game(guesses, genders=genders)
        key = "turn" if kind == cg.KIND_TURN else "replace"
        stats[key] += 1
        bad = [r for r in rejected if not illusion_explains(r, guesses)]
        stats["illusion_exempt"] += len(bad) < len(rejected)
        stats["rejected"] += bool(bad)
        legal = [a for a in range(cg.N_TURN_ACTIONS) if game.battle(0).mask[a]]
        stats["legal"] += legal == session.legal_actions()
        mine = game.tensors(0)
        stats["tensors"] += not same(mine, cg.tensors(session.battle))
        if kind == cg.KIND_TURN:
            mine_m, true_m = game.masks().tolist(), true_game.masks().tolist()
            stats["own_mask_true"] += mine_m[0] == true_m[0]
            stats["both_masks_true"] += mine_m == true_m
            if mine_m != true_m and VERBOSE:
                print("  opponent's State legal set differs:", [int(x) for x in mine_m[1]], [int(x) for x in true_m[1]], session.battle.opponent_active_pokemon)
        # the Game plays on from here: one step with the first legal actions
        m, ks = game.masks().tolist(), game.kinds()
        a = [next(i for i in range(11) if m[s][i]) if ks[s] > 0 else None for s in (0, 1)]
        game.step(*a)
        assert game.battle(0) is not None
        try:
            game.battle(1)
            raise AssertionError("the opposing seat's Battle must not be known")
        except cg.CGError:
            pass

    for k in range(games):
        pair = PAIRS[k % len(PAIRS)]
        play(SEED + 7000 + k, pair, k % 2 == 0, check)
    if VERBOSE:
        print("T-5", stats)
    n = stats["turn"] + stats["replace"]
    assert stats["turn"] > 300 and stats["replace"] > 30
    assert stats["rejected"] == 0, stats
    assert stats["legal"] == n, stats
    assert stats["tensors"] == n, stats
    assert stats["own_mask_true"] == stats["turn"], stats
    return stats


def test_wrong_guesses_are_rejected_and_the_game_built():
    lines, decs, truth = _cg.live_selfplay_truth(cg.TEAMS, 7101, "tower", "max_damage", True)
    seen = {}

    def check(session, battle, t):
        if seen:
            return
        true_game, guesses, genders, kind = t
        opp = session.battle.opponent_team
        used = [g for g in guesses if any(m for m in opp[g["slot"]].moves)]
        if kind != cg.KIND_TURN or not used:
            return
        g0 = used[0]
        shown = [m for m, _ in opp[g0["slot"]].moves]
        bad = [dict(g) for g in guesses]
        b = next(x for x in bad if x["slot"] == g0["slot"])
        b["moves"] = ["spore", "notamove"] + [m for m in b["moves"] if m not in shown][:2]  # drops a used move
        b["ability"] = "hugepower" if "hugepower" not in b["ability"] else "levitate"
        b["points"] = [40, 40, 0, 0, 0, 0]
        b["item"] = "notanitem"
        game, rejected = session.game(bad, genders=genders)
        fields = sorted({r[1] for r in rejected})
        seen["fields"] = fields
        seen["rejected"] = rejected
        assert {"moves", "ability", "points", "item"} <= set(fields), rejected
        assert any("cannot learn spore" in r[2] or "unknown move" in r[2] for r in rejected)
        assert any("was used" in r[2] for r in rejected), rejected
        # the Game is built and legal for us
        legal = [a for a in range(cg.N_TURN_ACTIONS) if game.battle(0).mask[a]]
        assert legal == session.legal_actions()
        game.tensors(0)

    replay(lines, decs, truth, check)
    assert seen, "no request with a used opposing move"
    if VERBOSE:
        print("rejected", seen["rejected"])


def test_missing_seen_pokemon_raises():
    lines, decs, truth = _cg.live_selfplay_truth(cg.TEAMS, 7102, "tower", "max_damage", True)
    done = []

    def check(session, battle, t):
        if done:
            return
        _, guesses, genders, kind = t
        lead = guesses[0]["slot"]
        others = [i for i in range(6) if i not in [g["slot"] for g in guesses]]
        try:
            session.game([{"slot": others[0]}, {"slot": others[1]}, {"slot": others[2]}])
            raise AssertionError("a seen lead missing must raise")
        except cg.CGError as e:
            assert "seen" in str(e)
        # a free function on the Battle alone gives the same legal set
        game, _ = cg.game_from_battle(session.battle, guesses, genders=genders)
        assert [a for a in range(cg.N_TURN_ACTIONS) if game.battle(0).mask[a]] == session.legal_actions()
        done.append(lead)

    replay(lines, decs, truth, check)
    assert done


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
