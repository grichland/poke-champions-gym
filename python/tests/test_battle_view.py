"""The Battle view Python Players receive, and CG's Rust Players as built-in opponents. Plain-script mode:
PYTHONPATH=python python python/tests/test_battle_view.py"""

import sys

import champions_gym as cg

TEAMS = cg.TEAMS


def test_battle_view_fields():
    env = cg.BatchEnv(TEAMS, 4, seed=5, rule="battle")
    p = cg.RandomPlayer(1)
    seen_turn = 0
    for _ in range(400):
        acts = []
        for g in range(env.games):
            b = env.battle(g)
            assert b.game == g and b.kind in (0, 1, 2)
            assert len(b.team) == 6 and len(b.opponent_team) == 6
            assert len([s for s in b.opponent_preview if s]) == 6
            if b.kind == cg.KIND_TURN:
                seen_turn += 1
                assert b.active_pokemon is not None
                a = b.active_pokemon
                assert 0.0 < a.current_hp_fraction <= 1.0 and not a.fainted
                assert set(a.boosts) == {"atk", "def", "spa", "spd", "spe", "accuracy", "evasion"}
                assert a.revealed and a.item is not None and a.ability is not None
                assert len(b.available_moves) == 4 and len(b.available_switches) == 3
                assert set(b.legal_actions()) <= set(range(cg.N_TURN_ACTIONS))
                o = b.opponent_active_pokemon
                assert o is not None and o.revealed
                # nothing unrevealed: the opponent's item and ability are unknown in this scope
                assert all(m[1] is None or m[1] >= 0 for m in o.moves)
                assert isinstance(b.weather, dict) and isinstance(b.side_conditions, dict)
            acts.append(p.action_for(b))
        env.act(acts)
    assert seen_turn > 100


def test_builtin_players_play():
    assert list(cg.BUILTIN_PLAYERS) == ["random", "max_damage", "tower", "best_matchup_max_damage", "disciplined", "endgame",
                                        "pilot_encore", "pilot_weather", "pilot_hazard", "pilot_priority"]
    r = cg.play_games("max_damage", "random", 40, TEAMS, seed=3, batch=20)
    assert len(r.games) == 40
    assert r.wins[0] > r.wins[1], r.wins
    r2 = cg.play_games(cg.builtin_player("tower", 1), cg.RandomPlayer(2), 20, TEAMS, seed=4, batch=10)
    assert len(r2.games) == 20
    try:
        cg.builtin_player("nobody")
    except cg.CGError:
        pass
    else:
        raise AssertionError("unknown player accepted")


def test_state_rule_still_available():
    r = cg.play_games("random", "random", 20, TEAMS, seed=6, batch=10, rule="state")
    assert len(r.games) == 20


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
