"""Low-latency observation (feature mode "ll"): every input is a direct read of the poke-env Battle / request or a
constant-time table lookup on one. No guessed builds, no belief, no damage engine, no log re-read, no catalog.
Row [12, 268]: raw 69 | moves 4 x 37 | hist 4 | lookup 31 | dmg 16. ids [12, 7]. field [55]. The last-turn token
and the legal mask are rl2.features' (the token from an incremental Tracker). Groups "hist", "lookup" and "dmg"
are switchable; a group that is off leaves its columns zero. Column table: docs/low-latency.md."""

from __future__ import annotations

from collections.abc import Sequence
from functools import lru_cache

import numpy as np
from pcsa.battle import game
from pcsa.battle.moves import base_data, reconcile_form
from pcsa.battle.stats import IV, NATURE_DOWN, NATURE_UP, POINTS_MAX
from poke_env.battle import Effect, Field, Move, PokemonType, SideCondition, Status, Weather
from poke_env.data import GenData
from poke_env.data.normalize import to_id_str

from rl import features as f1
from rl.features_vocab import index
from rl2 import features as fx
from rl2 import turns as tt

Obs = fx.Obs
ROWS, IDS, ACTIONS = 12, 7, f1.ACTIONS
LAST, LAST_IDS = fx.LAST, fx.LAST_IDS
RAW, MOVE_FACTS, MOVE_STATE, HIST, LOOKUP, DMG = 69, 34, 3, 4, 31, 16
MOVE = MOVE_FACTS + MOVE_STATE
MOVES_AT = RAW
HIST_AT = MOVES_AT + 4 * MOVE
LOOKUP_AT = HIST_AT + HIST
DMG_AT = LOOKUP_AT + LOOKUP
WIDTH = DMG_AT + DMG
FIELD = 55
HP_COL, ACTIVE_COL, COULD_MEGA_COL = 30, 47, 64
PURE = frozenset({"hist"})
ALL = frozenset({"hist", "lookup", "dmg"})
CONFIGS = {"raw": PURE, "raw+lookup": frozenset({"hist", "lookup"}), "raw+lookup+dmg": ALL}

STATS = ("hp", "atk", "def", "spa", "spd", "spe")
BOOSTS = ("atk", "def", "spa", "spd", "spe", "accuracy", "evasion")
BASE_DIV, STAT_DIV, POWER_DIV, PRIORITY_DIV, DAMAGE_CLIP = 255.0, 400.0, 150.0, 5.0, 2.0
LEVEL = game.LEVEL
SCARF, PARALYSIS, TAILWIND, WEATHER_SPEED = 1.5, 0.5, 2.0, 2.0
ITEM_UNKNOWN_UPPER = 1.5  # an unrevealed opposing item on their damage (Choice Band / Specs)
UNKNOWN_ITEM = GenData.UNKNOWN_ITEM

TYPES = tuple(t for t in PokemonType if t.name not in ("THREE_QUESTION_MARKS", "STELLAR"))
TYPE_INDEX = {t: i for i, t in enumerate(TYPES)}
TYPE_BY_NAME = {t.name.lower(): i for i, t in enumerate(TYPES)}
NONE = len(TYPES)  # "no type": a missing second type, a status move
STATUS_COL = {s: 31 + i for i, s in enumerate((Status.BRN, Status.FRZ, Status.PAR, Status.PSN, Status.SLP, Status.TOX))}
_PARTIAL = ("PARTIALLY_TRAPPED", "BIND", "WRAP", "FIRE_SPIN", "WHIRLPOOL", "SAND_TOMB", "INFESTATION", "MAGMA_STORM",
            "CLAMP", "SNAP_TRAP", "THUNDER_CAGE")
VOLATILE_COL = {
    Effect.ENCORE: 51, Effect.SUBSTITUTE: 52, Effect.TAUNT: 53, Effect.YAWN: 54, Effect.LEECH_SEED: 55,
    Effect.CONFUSION: 56, Effect.TRAPPED: 57, **{Effect[n]: 58 for n in _PARTIAL if n in Effect.__members__},
}
BOOST_SCALAR = {s: float(np.log2((2 + s) / 2 if s >= 0 else 2 / (2 - s)) / 2) for s in range(-6, 7)}
STAGE = {s: ((2 + s) / 2 if s >= 0 else 2 / (2 - s)) for s in range(-6, 7)}
NOT_MOVES = frozenset({"struggle", "recharge"})
FIRST_TURN_ONLY = frozenset({"fakeout", "firstimpression", "matblock"})
FIXED_DAMAGE = {"seismictoss": LEVEL, "nightshade": LEVEL}
TARGET_DEF = frozenset({"psyshock", "psystrike", "secretsword"})  # special moves that hit Defense
USER_DEF = frozenset({"bodypress"})  # attacks with the user's Defense
# Known abilities only: type -> multiplier on the hit.
ABILITY_TYPE = {
    "levitate": {"ground": 0.0}, "eartheater": {"ground": 0.0}, "flashfire": {"fire": 0.0},
    "wellbakedbody": {"fire": 0.0}, "waterabsorb": {"water": 0.0}, "stormdrain": {"water": 0.0},
    "dryskin": {"water": 0.0, "fire": 1.25}, "voltabsorb": {"electric": 0.0}, "lightningrod": {"electric": 0.0},
    "motordrive": {"electric": 0.0}, "sapsipper": {"grass": 0.0}, "thickfat": {"fire": 0.5, "ice": 0.5},
    "heatproof": {"fire": 0.5}, "purifyingsalt": {"ghost": 0.5},
}
ITEM_TYPE = {"airballoon": {"ground": 0.0}}
SPEED_WEATHER = {"swiftswim": "rain", "chlorophyll": "sun", "sandrush": "sand", "slushrush": "snow"}
WEATHER_KIND = {
    Weather.SUNNYDAY: "sun", Weather.DESOLATELAND: "sun", Weather.RAINDANCE: "rain", Weather.PRIMORDIALSEA: "rain",
    Weather.SANDSTORM: "sand", Weather.SNOWSCAPE: "snow", Weather.HAIL: "snow",
}
WEATHER_COL = {"sun": 7, "rain": 8, "sand": 9, "snow": 10}
ITEM_ATTACK = {"choiceband": (1.5, 1.0), "choicespecs": (1.0, 1.5), "lifeorb": (1.3, 1.3)}  # (physical, special)
TERRAINS = (Field.ELECTRIC_TERRAIN, Field.GRASSY_TERRAIN, Field.MISTY_TERRAIN, Field.PSYCHIC_TERRAIN)
SIDE_FLAGS = (
    SideCondition.REFLECT, SideCondition.LIGHT_SCREEN, SideCondition.AURORA_VEIL, SideCondition.TAILWIND,
    SideCondition.SAFEGUARD, SideCondition.MIST, SideCondition.STEALTH_ROCK, SideCondition.SPIKES,
    SideCondition.TOXIC_SPIKES, SideCondition.STICKY_WEB,
)
SIDE_LAYERS = {SideCondition.SPIKES: 3.0, SideCondition.TOXIC_SPIKES: 2.0}
SIDE_TIMED = ((SideCondition.REFLECT, 8.0), (SideCondition.LIGHT_SCREEN, 8.0), (SideCondition.AURORA_VEIL, 8.0),
              (SideCondition.TAILWIND, 4.0), (SideCondition.SAFEGUARD, 5.0))
TARGET = np.asarray([6] * 6 + [0] * 6)  # the row a row's moves are aimed at: the opposing active
_R12, _R4 = np.arange(12)[:, None], np.arange(4)[None, :]


# Tables ---------------------------------------------------------------------------------------------------------
@lru_cache(maxsize=1)
def chart() -> np.ndarray:
    """[attacking type, defending type] multiplier; index 18 (no type) is neutral."""
    table = GenData.from_gen(9).type_chart
    out = np.ones((NONE + 1, NONE + 1), dtype=np.float32)
    for a, i in TYPE_INDEX.items():
        for d, j in TYPE_INDEX.items():
            out[i, j] = a.damage_multiplier(d, type_chart=table)
    return out


class Form:
    """Static facts of one species form."""

    __slots__ = ("key", "id", "vec", "types", "lo", "hi", "only_ability", "megas", "is_mega", "weather_speed",
                 "base_id")


@lru_cache(maxsize=4096)
def form(key: str) -> Form | None:
    entry = game.species_entry(key)
    base = entry.get("baseStats")
    if not base:
        return None
    f = Form()
    f.key, f.id = key, index("species", key)
    types = [TYPE_BY_NAME[t.lower()] for t in entry.get("types", []) if t.lower() in TYPE_BY_NAME][:2]
    f.types = (types + [NONE, NONE])[:2]
    f.vec = np.zeros(24, dtype=np.float32)
    f.vec[types] = 1.0
    b = np.asarray([base.get(s, 0) for s in STATS], dtype=np.int64)
    f.vec[18:24] = np.minimum(1.0, b / BASE_DIV)
    f.lo, f.hi = _stats(b, 0, NATURE_DOWN), _stats(b, POINTS_MAX, NATURE_UP)
    abilities = {to_id_str(a) for a in (entry.get("abilities") or {}).values()}
    f.only_ability = next(iter(abilities)) if len(abilities) == 1 else ""
    f.weather_speed = frozenset(SPEED_WEATHER[a] for a in abilities if a in SPEED_WEATHER)
    f.is_mega = "Mega" in (entry.get("forme") or "")
    f.base_id = to_id_str(entry.get("baseSpecies") or entry.get("name") or key)
    f.megas = frozenset(game.megas()[0].get(f.base_id, ()))
    return f


def _stats(base: np.ndarray, points: int, nature: int) -> list[int]:
    """Level-50 stats of a spread with `points` everywhere and `nature` (percent) on every non-HP stat."""
    raw = (2 * base + IV + 2 * points) * LEVEL // 100
    out = (raw + 5) * nature // 100
    out[0] = raw[0] + LEVEL + 10
    return [int(v) for v in out]


class MoveFacts:
    __slots__ = ("id", "vec", "type", "damaging", "power", "atk", "dfn", "physical", "hits", "contact", "fixed")


def _is_setup(move: Move) -> bool:
    boosts, self_boost = move.boosts or {}, move.self_boost or {}
    target_self = str(getattr(move.target, "name", "")).upper() in ("SELF", "ADJACENT_ALLY_OR_SELF")
    gains = sum(v for v in (boosts if target_self else {}).values() if v > 0)
    gains += sum(v for v in self_boost.values() if v > 0) if move.category.name == "STATUS" else 0
    return gains > 0


@lru_cache(maxsize=4096)
def move_facts(move_id: str) -> MoveFacts | None:
    """type 18 | category 3 | power /150 | accuracy | priority /5 | contact | sound | pivot | recovery | setup |
    inflicts status | multi-hit | self-drop | first turn only | protect."""
    if move_id in NOT_MOVES or move_id not in GenData.from_gen(9).moves:
        return None
    move, data = Move(move_id, gen=9), base_data(move_id)
    m = MoveFacts()
    m.id, m.vec = index("move", move_id), np.zeros(MOVE_FACTS, dtype=np.float32)
    category = (data.get("category") or move.category.name).lower()
    m.type = TYPE_BY_NAME.get((data.get("type") or "").lower(), TYPE_INDEX.get(move.type, NONE))
    m.damaging = category != "status"
    m.physical = category == "physical"
    m.power = float(data.get("power") or 0) if m.damaging else 0.0
    m.atk = 2 if move_id in USER_DEF else (0 if m.physical else 1)  # atk, spa, def
    m.dfn = 0 if m.physical or move_id in TARGET_DEF else 1  # def, spd
    m.hits = tuple(float(h) for h in move.n_hit)
    m.contact = "contact" in move.flags
    m.fixed = float(FIXED_DAMAGE.get(move_id, 0))
    v = m.vec
    if m.type < NONE:
        v[m.type] = 1.0
    v[18 + ("physical", "special", "status").index(category)] = 1.0
    v[21] = min(DAMAGE_CLIP, m.power / POWER_DIV)
    accuracy = data.get("accuracy", 100)
    v[22] = 1.0 if accuracy is True else min(1.0, float(accuracy) / 100.0)
    v[23] = max(-1.5, min(1.5, move.priority / PRIORITY_DIV))
    statuses = move.status is not None or any(s.get("status") for s in move.secondary or ())
    v[24:34] = [
        m.contact, "sound" in move.flags, bool(move.self_switch), move.heal > 0 or move.drain > 0, _is_setup(move),
        statuses, m.hits != (1.0, 1.0), any(x < 0 for x in (move.self_boost or {}).values()),
        move_id in FIRST_TURN_ONLY, bool(move.is_protect_move),
    ]
    return m


@lru_cache(maxsize=1024)
def _type_mult(ability: str, item: str) -> np.ndarray | None:
    """[19] multiplier a known ability / item puts on each attacking type; None when neither does."""
    rules = {**ABILITY_TYPE.get(ability, {}), **ITEM_TYPE.get(item, {})}
    if not rules:
        return None
    out = np.ones(NONE + 1, dtype=np.float32)
    for name, mult in rules.items():
        out[TYPE_BY_NAME[name]] = mult
    return out


# History --------------------------------------------------------------------------------------------------------
class Tracker:
    """Per-battle history, fed each protocol line once: turn records (rl2.turns), each member's current form,
    entries and move order. Keep one per battle (observe() keeps it on the Battle unless one is passed)."""

    def __init__(self) -> None:
        self.reset()

    def reset(self) -> None:
        self.n, self.turn = 0, 0
        self.turns = tt._Reader()
        self.form: dict[str, str] = {}
        self.entries: dict[str, int] = {}
        self.entered: dict[str, int] = {}
        self.first: dict[str, int] = {}
        self.second: dict[str, int] = {}
        self._order: list[str] = []

    def feed(self, protocol: list[list[str]]) -> Tracker:
        n = len(protocol)
        if n < self.n:
            self.reset()
        for i in range(self.n, n):
            self.line(protocol[i])
        self.n = n
        return self

    def line(self, parts: list[str]) -> None:
        self.turns.line(parts)
        if len(parts) < 3:
            return
        kind = parts[1]
        if kind == "turn":
            order = self._order
            if len(order) == 2 and order[0][:2] != order[1][:2]:
                self.first[order[0]] = self.first.get(order[0], 0) + 1
                self.second[order[1]] = self.second.get(order[1], 0) + 1
            self._order = []
            self.turn = int(parts[2]) if parts[2].isdigit() else self.turn + 1
            return
        if kind == "move":
            if len(parts) > 3 and not any(p.startswith("[from]") for p in parts[4:]):
                k = _key(parts[2])
                if k not in self._order:
                    self._order.append(k)
        elif kind in ("switch", "drag", "replace", "detailschange", "-formechange") and len(parts) > 3:
            k = _key(parts[2])
            candidate = to_id_str(parts[3].split(",", 1)[0])
            current = self.form.get(k)
            if kind in ("switch", "drag", "replace"):
                self.entries[k] = self.entries.get(k, 0) + 1
                self.entered[k] = self.turn
                now = form(current) if current else None
                then = form(candidate)
                if now is not None and (now.is_mega or current == "mimikyubusted") and (
                        then is None or then.base_id == now.base_id):
                    return  # send-out rows name the base species after a persistent form change
            self.form[k] = reconcile_form(current, candidate) if current else candidate
        elif kind == "-mega" and len(parts) > 4:
            resolved = game.mega_form_for_stone(parts[3], parts[4])
            if resolved:
                self.form[_key(parts[2])] = to_id_str(resolved)


def _key(ident: str) -> str:
    """'p1a: Name' -> 'p1: Name' (the team dict key)."""
    side, _, name = ident.partition(": ")
    return f"{side[:2]}: {name}"


def tracker_of(battle) -> Tracker:
    """The battle's own tracker (created on first use), brought up to date."""
    try:
        tracker = battle.__dict__.setdefault("_ll_tracker", Tracker())
    except AttributeError:
        tracker = Tracker()
    return tracker.feed(battle._replay_data)


# Rows -----------------------------------------------------------------------------------------------------------
def _their_members(battle, tracker: Tracker) -> list[tuple[str | None, object]]:
    """rl.features._their_members with the tracker's entry turns: active first, then seen, then preview-only."""
    seen = [(k, m) for k, m in battle.opponent_team.items() if ": " in k]
    active = battle.opponent_active_pokemon
    if active is None and seen:
        active = max(seen, key=lambda km: tracker.entered.get(km[0], -1))[1]
    seen.sort(key=lambda km: km[1] is not active)
    bases = set()
    for _, m in seen:
        bases.add(m.species)
        bases.add(game.base_name(m.species))
    preview = [(None, m) for m in battle.teampreview_opponent_team
               if m.species not in bases and game.base_name(m.species) not in bases]
    return (seen + preview)[:6]


def _item(mon, ours: bool) -> tuple[bool, str]:
    """(known, item id); ours always known, theirs once revealed ("" = known to be gone)."""
    item = mon.item
    if item is None or item == UNKNOWN_ITEM:
        return ours, ""
    return True, item


def _speed(mon, fm: Form | None, ours: bool, item_known: bool, item: str, ability: str, weather: str, tailwind: bool):
    """(low, high) speed now: ours exact from the request, theirs from the level-50 formula's bounds."""
    exact = (mon.stats or {}).get("spe") if ours else None
    if exact:
        lo = hi = float(exact)
    elif fm is not None:
        lo, hi = float(fm.lo[5]), float(fm.hi[5])
    else:
        return 0.0, 0.0
    if item == "choicescarf":
        lo, hi = lo * SCARF, hi * SCARF
    elif not item_known:
        hi *= SCARF
    if weather:
        if ability:
            if SPEED_WEATHER.get(ability) == weather:
                lo, hi = lo * WEATHER_SPEED, hi * WEATHER_SPEED
        elif fm is not None and weather in fm.weather_speed:
            hi *= WEATHER_SPEED
    stage = STAGE[max(-6, min(6, int(mon.boosts.get("spe", 0))))]
    scale = stage * (PARALYSIS if mon.status is Status.PAR else 1.0) * (TAILWIND if tailwind else 1.0)
    return float(int(lo * stage)) * scale / stage, float(int(hi * stage)) * scale / stage


def _elapsed(turn: int, start, div: float) -> float:
    try:
        return max(0.0, min(1.0, (turn - int(start)) / div))
    except (TypeError, ValueError):
        return 0.0


def _field(battle, legal, mine: list, theirs: list, weather: str) -> np.ndarray:
    """global 7 | weather 4 + age | terrain 4 + age | Trick Room + age, Gravity | side conditions 2 x 15 |
    our alive, their fainted, their revealed | legal moves, legal switches."""
    f = np.zeros(FIELD, dtype=np.float32)
    turn = int(battle.turn or 0)
    f[0:7] = [min(1.0, turn / 30.0), bool(battle.force_switch), bool(battle.trapped), bool(battle.maybe_trapped),
              bool(battle.can_mega_evolve), bool(battle.used_mega_evolve), bool(battle.opponent_used_mega_evolve)]
    if weather:
        f[WEATHER_COL[weather]] = 1.0
        f[11] = _elapsed(turn, next(iter(battle.weather.values())), 8.0)
    fields = battle.fields
    if fields:
        for i, t in enumerate(TERRAINS):
            if t in fields:
                f[12 + i], f[16] = 1.0, _elapsed(turn, fields[t], 8.0)
        if Field.TRICK_ROOM in fields:
            f[17], f[18] = 1.0, _elapsed(turn, fields[Field.TRICK_ROOM], 5.0)
        f[19] = float(Field.GRAVITY in fields)
    for at, conditions in ((20, battle.side_conditions), (35, battle.opponent_side_conditions)):
        if not conditions:
            continue
        for i, c in enumerate(SIDE_FLAGS):
            if c in conditions:
                f[at + i] = min(1.0, conditions[c] / SIDE_LAYERS[c]) if c in SIDE_LAYERS else 1.0
        for i, (c, div) in enumerate(SIDE_TIMED):
            if c in conditions:
                f[at + 10 + i] = _elapsed(turn, conditions[c], div)
    f[50] = sum(1 for m in mine if not m.fainted) / 3.0
    f[51] = sum(1 for k, m in theirs if k is not None and m.fainted) / 3.0
    f[52] = min(1.0, sum(1 for k, _ in theirs if k is not None) / 3.0)
    moves = switches = 0
    for a in legal:
        kind = a.action.kind
        if kind == "switch":
            switches += 1
        elif not a.action.mega:
            moves += 1
    f[53], f[54] = min(1.0, moves / 4.0), min(1.0, switches / 2.0)
    return f


def encode(battle, legal: Sequence, tracker: Tracker, groups: frozenset = ALL, members: list | None = None,
           detail: dict | None = None) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """(rows, ids, field). `members`: our trio with the lead first (team preview); `detail` receives the damage
    estimate's arrays (benchmarks)."""
    rows = np.zeros((ROWS, WIDTH), dtype=np.float32)
    ids = np.zeros((ROWS, IDS), dtype=np.int64)
    hist, lookup, dmg = "hist" in groups, "lookup" in groups, "dmg" in groups
    mine = (members if members is not None else f1._request_members(battle))[:3]
    theirs = _their_members(battle, tracker)
    revealed = sum(1 for k, _ in theirs if k is not None)
    turn = int(battle.turn or 0)
    weather = next((WEATHER_KIND[w] for w in battle.weather if w in WEATHER_KIND), "") if battle.weather else ""
    request = battle.last_request or {}
    state = {m.get("id"): m for m in ((request.get("active") or [{}])[0].get("moves") or ())} if members is None else {}
    legal_moves = {a.order.order.id for a in legal
                   if a.action.kind == "move" and a.order is not None and isinstance(a.order.order, Move)}
    role = battle.player_role or "p1"
    their_active = battle.opponent_active_pokemon
    used = (battle.used_mega_evolve, battle.opponent_used_mega_evolve)
    tailwind = (SideCondition.TAILWIND in battle.side_conditions,
                SideCondition.TAILWIND in battle.opponent_side_conditions)
    stones = game.megas()[1]

    # Per-row facts the lookups need (index = row).
    t1, t2 = np.full(ROWS, NONE), np.full(ROWS, NONE)
    alive = np.zeros(ROWS, dtype=bool)
    mtype = np.full((ROWS, 4), NONE)
    damaging = np.zeros((ROWS, 4), dtype=bool)
    resist = None
    speeds: list = [None] * ROWS
    calc: list = [None] * ROWS  # (mon, form, ours, item known, item, ability, move facts)

    entries = [(mon, f"{role}: {mon.name}", True, i) for i, mon in enumerate(mine)]
    entries += [(mon, k, False, 6 + i) for i, (k, mon) in enumerate(theirs)]
    for mon, key, ours, i in entries:
        r = rows[i]
        shown = ours or key is not None
        fm = form((tracker.form.get(key) if key else None) or mon.species) or form(mon.species)
        fainted = bool(mon.fainted)
        active = not fainted and (i == 0 if ours else (shown and mon is their_active))
        temporary = mon._temporary_types
        if fm is not None:
            r[0:24] = fm.vec
            types = fm.types
        if temporary or fm is None:
            types = ([TYPE_INDEX.get(t, NONE) for t in (temporary or mon.types) if t is not None] + [NONE, NONE])[:2]
            r[0:18] = 0.0
            for t in types:
                if t < NONE:
                    r[t] = 1.0
        t1[i], t2[i] = types
        if ours:
            stats = mon.stats or {}
            r[24] = min(2.0, (mon.max_hp or 0) / STAT_DIV)
            r[25:30] = [min(2.0, (stats.get(s) or 0) / STAT_DIV) for s in STATS[1:]]
        hp = max(0.0, min(1.0, float(mon.current_hp_fraction or 0.0))) if shown else 1.0
        r[30] = hp
        alive[i] = not fainted and hp > 0
        status = mon.status
        if status in STATUS_COL:
            r[STATUS_COL[status]] = 1.0
            r[37] = min(1.0, (mon.status_counter or 0) / 5.0)
        boosts = mon.boosts
        if any(boosts.values()):
            r[38:45] = [BOOST_SCALAR[max(-6, min(6, boosts.get(s, 0)))] for s in BOOSTS]
        r[45], r[46], r[47], r[48], r[49] = 1.0, ours, active, fainted, shown
        r[50] = float(not shown and revealed >= 3)
        effects = mon.effects
        if effects:
            for e in effects:
                c = VOLATILE_COL.get(e)
                if c is not None:
                    r[c] = 1.0
        r[59], r[60] = bool(mon.must_recharge), bool(mon.preparing)
        r[61] = float(active and mon.first_turn)
        r[62] = min(1.0, (mon.protect_counter or 0) / 3.0)
        item_known, item = _item(mon, ours)
        ability = mon.ability or (fm.only_ability if fm is not None else "") or ""
        mega = fm is not None and fm.is_mega
        r[63] = mega
        if not fainted and not mega and not used[not ours] and fm is not None and fm.megas:
            r[64] = float(stones.get(item) in fm.megas) if item_known else 1.0
        r[65], r[66], r[67] = item_known, item_known and not item, bool(ability)
        if ours:
            moves = f1._slot_moves(mon, legal) if i == 0 and members is None else list(mon.moves.values())[:4]
        else:
            moves = [m for k, m in mon.moves.items() if move_facts(k) is not None][:4] if shown else []
        facts: list = [None] * 4
        count = 0
        for j, move in enumerate(moves):
            mf = move_facts(move.id) if move is not None else None
            if mf is None:
                continue
            count += 1
            facts[j] = mf
            at = MOVES_AT + j * MOVE
            r[at : at + MOVE_FACTS] = mf.vec
            row = state.get(move.id) if i == 0 and ours else None
            if row is not None and row.get("maxpp"):
                pp = max(0.0, min(1.0, row.get("pp", 0) / row["maxpp"]))
            else:
                top = move.max_pp
                pp = max(0.0, min(1.0, move.current_pp / top)) if top else 1.0
            off = bool(row.get("disabled")) if row is not None else False
            if i == 0 and ours and legal_moves and move.id not in legal_moves:
                off = True
            r[at + MOVE_FACTS : at + MOVE] = [1.0, pp, off]
            ids[i, 3 + j] = mf.id
            mtype[i, j], damaging[i, j] = mf.type, mf.damaging
        r[68] = count / 4.0
        ids[i, 0] = fm.id if fm is not None else index("species", mon.species)
        ids[i, 1] = index("item", item) if item else 0
        ids[i, 2] = index("ability", ability) if ability else 0
        if hist and key is not None:
            entered = tracker.entered.get(key, -1)
            r[HIST_AT : HIST_AT + 4] = [
                min(1.0, tracker.entries.get(key, 0) / 4.0), min(1.0, tracker.first.get(key, 0) / 5.0),
                min(1.0, tracker.second.get(key, 0) / 5.0),
                min(1.0, max(0, turn - entered) / 10.0) if active and entered >= 0 else 0.0,
            ]
        if lookup or dmg:
            mult = _type_mult(ability, item)
            if mult is not None:
                if resist is None:
                    resist = np.ones((NONE + 1, ROWS), dtype=np.float32)
                resist[:, i] = mult
            calc[i] = (mon, fm, ours, item_known, item, ability, facts)
            if lookup and alive[i]:
                speeds[i] = _speed(mon, fm, ours, item_known, item, ability, weather, tailwind[not ours])

    if lookup or dmg:
        present = rows[:, 45] > 0
        c = chart()
        eff = c[mtype[:, :, None], t1[None, None, :]] * c[mtype[:, :, None], t2[None, None, :]]  # [12, 4, 12]
        if resist is not None:
            eff = eff * resist[mtype]
        stab = ((mtype == t1[:, None]) | (mtype == t2[:, None])) & damaging
        for i in range(ROWS):  # Libero / Protean: every move gets STAB
            if calc[i] is not None and calc[i][5] in ("libero", "protean"):
                stab[i] = damaging[i]
    if lookup:
        code = np.where(eff > 0, (np.clip(np.log2(np.maximum(eff, 1e-3)), -2.0, 2.0) + 3.0) / 5.0, -1.0)
        code = code * (damaging[:, :, None] & present[None, None, :])
        out = rows[:, LOOKUP_AT + 3 : LOOKUP_AT + LOOKUP].reshape(ROWS, 4, 7)
        out[:, :, 0] = stab
        out[:6, :, 1:] = code[:6, :, 6:]
        out[6:, :, 1:] = code[6:, :, :6]
        trick_room = Field.TRICK_ROOM in battle.fields
        for i in range(ROWS):
            a, b = speeds[i], speeds[TARGET[i]]
            if a is None or b is None:
                continue
            faster, slower = a[0] > b[1], a[1] < b[0]
            if trick_room:
                faster, slower = slower, faster
            rows[i, LOOKUP_AT : LOOKUP_AT + 3] = [faster, slower, not (faster or slower)]
    if dmg:
        _damage(battle, rows, calc, eff, stab, mtype, alive, weather, detail)
    return rows, ids, _field(battle, legal, mine, theirs, weather)


def _damage(battle, rows, calc, eff, stab, mtype, alive, weather: str, detail: dict | None) -> None:
    """Closed-form level-50 damage of every row's moves on the opposing active: per move min and max fraction of
    the target's current HP, KO certain, KO possible. Ours attack with exact stats into their zero- and
    max-investment bounds; theirs attack from their bounds (x1.5 at the top while the item is unrevealed)."""
    power, fixed = np.zeros((ROWS, 4)), np.zeros((ROWS, 4))
    acat, dcat = np.zeros((ROWS, 4), dtype=np.int64), np.zeros((ROWS, 4), dtype=np.int64)
    hits = np.ones((ROWS, 4, 2))
    a_lo, a_hi = np.ones((ROWS, 3)), np.ones((ROWS, 3))  # atk, spa, def (stages applied)
    d_lo, d_hi = np.ones((ROWS, 2)), np.ones((ROWS, 2))  # def, spd
    hp_lo, hp_hi, top_lo, top_hi = np.ones(ROWS), np.ones(ROWS), np.ones(ROWS), np.ones(ROWS)
    mod, upper = np.ones((ROWS, 4)), np.ones(ROWS)
    burn = np.ones((ROWS, 2))
    stab_mult = np.full(ROWS, 1.5)
    for i, entry in enumerate(calc):
        if entry is None:
            continue
        mon, fm, ours, item_known, item, ability, facts = entry
        boosts = mon.boosts
        stats = mon.stats if ours else None
        if stats and stats.get("atk"):
            lo = hi = [mon.max_hp or 1, stats["atk"], stats["def"], stats["spa"], stats["spd"]]
            hp_lo[i] = hp_hi[i] = mon.current_hp or 1
        elif fm is not None:
            lo, hi = fm.lo, fm.hi
            frac = rows[i, HP_COL]
            hp_lo[i], hp_hi[i] = max(1.0, frac * lo[0]), max(1.0, frac * hi[0])
        else:
            continue
        top_lo[i], top_hi[i] = lo[0], hi[0]
        sa, sd = STAGE[boosts.get("atk", 0)], STAGE[boosts.get("def", 0)]
        sc, sf = STAGE[boosts.get("spa", 0)], STAGE[boosts.get("spd", 0)]
        a_lo[i] = (lo[1] * sa, lo[3] * sc, lo[2] * sd)
        a_hi[i] = (hi[1] * sa, hi[3] * sc, hi[2] * sd)
        d_lo[i] = (lo[2] * sd, lo[4] * sf)
        d_hi[i] = (hi[2] * sd, hi[4] * sf)
        if mon.status is Status.BRN and ability != "guts":
            burn[i, 0] = 0.5
        if not item_known:
            upper[i] = ITEM_UNKNOWN_UPPER
        boost = ITEM_ATTACK.get(item)
        huge = 2.0 if ability in ("hugepower", "purepower") else 1.0
        if ability == "adaptability":
            stab_mult[i] = 2.0
        for j, mf in enumerate(facts):
            if mf is None or not mf.damaging:
                continue
            p = mf.power
            if ability == "technician" and p <= 60:
                p *= 1.5
            if mf.id == _KNOCK_OFF and calc[TARGET[i]] is not None and (
                    calc[TARGET[i]][4] or not calc[TARGET[i]][3]):
                p *= 1.5  # the target holds (or may hold) an item
            power[i, j], fixed[i, j] = p, mf.fixed
            acat[i, j], dcat[i, j] = mf.atk, mf.dfn
            hits[i, j] = mf.hits
            m = 1.0
            if boost is not None:
                m *= boost[0] if mf.physical else boost[1]
            if mf.physical:
                m *= huge
            if mf.contact and ability == "toughclaws":
                m *= 1.3
            mod[i, j] = m
    e = eff[_R12, _R4, TARGET[:, None]]  # [12, 4] against the row's target
    tw = np.ones(NONE + 1)
    if weather == "sun":
        tw[TYPE_BY_NAME["fire"]], tw[TYPE_BY_NAME["water"]] = 1.5, 0.5
    elif weather == "rain":
        tw[TYPE_BY_NAME["water"]], tw[TYPE_BY_NAME["fire"]] = 1.5, 0.5
    screens = np.ones((ROWS, 2))  # on the target's side, per (def, spd)
    for sl, conditions in ((slice(0, 6), battle.opponent_side_conditions), (slice(6, 12), battle.side_conditions)):
        if conditions:
            veil = SideCondition.AURORA_VEIL in conditions
            screens[sl, 0] = 0.5 if veil or SideCondition.REFLECT in conditions else 1.0
            screens[sl, 1] = 0.5 if veil or SideCondition.LIGHT_SCREEN in conditions else 1.0
    physical = acat != 1
    mod = mod * e * np.where(stab, stab_mult[:, None], 1.0) * tw[mtype]
    mod = mod * np.where(physical, burn[:, :1], 1.0) * np.take_along_axis(screens, dcat, 1)
    att_lo, att_hi = np.take_along_axis(a_lo, acat, 1), np.take_along_axis(a_hi, acat, 1)
    def_lo = np.take_along_axis(d_lo[TARGET], dcat, 1)
    def_hi = np.take_along_axis(d_hi[TARGET], dcat, 1)
    level = 2 * LEVEL // 5 + 2
    low = np.floor((np.floor(level * power * att_lo / def_hi) // 50 + 2) * mod * 0.85) * hits[:, :, 0]
    high = np.floor((np.floor(level * power * att_hi / def_lo) // 50 + 2) * mod * upper[:, None]) * hits[:, :, 1]
    flat = (fixed > 0) & (e > 0)
    low, high = np.where(flat, fixed, low), np.where(flat, fixed, high)
    valid = (power > 0) & (e > 0) & alive[:, None] & alive[TARGET][:, None]
    low, high = low * valid, high * valid
    f_lo, f_hi = low / hp_hi[TARGET][:, None], high / hp_lo[TARGET][:, None]
    out = rows[:, DMG_AT : DMG_AT + DMG].reshape(ROWS, 4, 4)
    out[:, :, 0], out[:, :, 1] = np.minimum(DAMAGE_CLIP, f_lo), np.minimum(DAMAGE_CLIP, f_hi)
    out[:, :, 2], out[:, :, 3] = f_lo >= 1.0, f_hi >= 1.0
    if detail is not None:  # fractions of the target's max HP, as the engine reports them
        detail["low"], detail["high"] = low / top_hi[TARGET][:, None], high / top_lo[TARGET][:, None]
        detail["current_low"], detail["current_high"], detail["valid"] = f_lo, f_hi, valid


_KNOCK_OFF = index("move", "knockoff")


# Observations ---------------------------------------------------------------------------------------------------
def observe(battle, legal: Sequence, tracker: Tracker | None = None, groups: frozenset = ALL,
            detail: dict | None = None) -> Obs:
    """The observation of one decision. `tracker`: this battle's Tracker when the Battle object is rebuilt per
    decision (a mirrored p2 seat, the live player); else the one kept on the Battle."""
    tracker = tracker_of(battle) if tracker is None else tracker.feed(battle._replay_data)
    rows, ids, field = encode(battle, legal, tracker, groups, detail=detail)
    mask = np.zeros(ACTIONS, dtype=bool)
    for a in legal:
        mask[f1.action_index(a, battle)] = True
    last, last_ids = fx.last_turn(battle, tracker.turns.turns)
    forced = bool((battle.last_request or {}).get("forceSwitch")) or not mask[: f1.SWITCH_AT].any()
    return Obs(rows, ids, field, last, last_ids, mask, int(battle.turn or 0), forced)


def preview_observations(battle, picks: Sequence[tuple[int, int, int]], tracker: Tracker | None = None,
                         groups: frozenset = ALL) -> list[Obs]:
    """Turn-0 observations, one per (lead, second, third) pick: our trio with the lead active, their six."""
    tracker = tracker_of(battle) if tracker is None else tracker.feed(battle._replay_data)
    team = list(battle.team.values())
    last, last_ids = fx.last_turn(battle, tracker.turns.turns)
    out = []
    for pick in picks:
        rows, ids, field = encode(battle, (), tracker, groups, members=[team[i] for i in pick])
        out.append(Obs(rows, ids, field, last.copy(), last_ids.copy(), np.zeros(ACTIONS, dtype=bool), 0, True))
    return out
