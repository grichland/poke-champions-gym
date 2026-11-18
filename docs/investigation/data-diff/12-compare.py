"""12-compare.py: mechanical comparison of poke-engine data (out/engine.json, from 11-engine_dump.rs)
with Showdown's champions mod (out/showdown-champions.json, from 09-dump_showdown.js).

usage: python 12-compare.py            (run from this folder; writes out/diff.json)

Every difference is a record {cat, id, field, showdown, engine, cls, note}.
cls: "real"            the data disagree
     "representation"  same meaning, different encoding (e.g. never-miss stored as accuracy 100)
     "needs-reading"   Showdown expresses it as a function, or the engine may do it in code; a reader must decide
"""
import json
import os
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
SD = json.load(open(os.path.join(HERE, 'out', 'showdown-champions.json'), encoding='utf-8'))
EN = json.load(open(os.path.join(HERE, 'out', 'engine.json'), encoding='utf-8'))

diffs = []

# move ids referenced by engine code outside the move table (genx modules), for classifying effect differences
import re
ENGINE_SRC = os.path.join(HERE, 'work', 'engine', 'src', 'genx')
CODE_REFS = defaultdict(list)
for fn in sorted(os.listdir(ENGINE_SRC)):
    if fn.endswith('.rs'):
        for mid_ in sorted(set(re.findall(r'Choices::([A-Z0-9]+)', open(os.path.join(ENGINE_SRC, fn), encoding='utf-8').read()))):
            CODE_REFS[mid_].append(fn)
compared = Counter()          # entries compared per category
fields_compared = Counter()   # (cat, field) comparisons made


def add(cat, id_, field, sd, en, cls='real', note=''):
    diffs.append({'cat': cat, 'id': id_, 'field': field, 'showdown': sd, 'engine': en, 'cls': cls, 'note': note})


def toid(s):
    return ''.join(c for c in (s or '').lower() if c.isalnum())


STAT = {'atk': 'atk', 'def': 'def', 'spa': 'spa', 'spd': 'spd', 'spe': 'spe', 'accuracy': 'accuracy', 'evasion': 'evasion'}
STATUS = {'brn': 'BURN', 'par': 'PARALYZE', 'slp': 'SLEEP', 'frz': 'FREEZE', 'psn': 'POISON', 'tox': 'TOXIC'}
SELFISH = {'self', 'adjacentAllyOrSelf', 'allies', 'allyTeam', 'allySide', 'adjacentAlly'}


def tmap(t):
    if t in SELFISH:
        return 'User'
    if t == 'all':
        return 'Field'
    return 'Opponent'


def sd_boosts(b):
    """Showdown boosts dict -> canonical string with nonzero entries only."""
    if not b:
        return None
    b = {k: v for k, v in b.items() if v}
    return ','.join(f'{k}{v:+d}' for k, v in sorted(b.items())) or None


def en_boosts(b):
    if not b:
        return None
    b = {k: v for k, v in b.items() if v}
    return ','.join(f'{k}{v:+d}' for k, v in sorted(b.items())) or None


def frac(x):
    return None if not x else round(x[0] / x[1], 3)


def close(a, b, tol=0.006):
    if a is None or b is None:
        return a is None and b is None
    return abs(a - b) <= tol


# ---------------------------------------------------------------- moves
ENGINE_FLAGS = ['bite', 'bullet', 'charge', 'contact', 'heal', 'powder', 'protect', 'pulse', 'punch',
                'recharge', 'reflectable', 'slicing', 'sound', 'wind']
# engine flags with no same-named Showdown flag, mapped to Showdown move properties
ENGINE_FLAG_PROPS = {'drag': 'forceSwitch', 'pivot': 'selfSwitch'}
MULTIHIT = {'2': 'DoubleHit', '3': 'TripleHit', '[2, 5]': 'TwoToFiveHits', '10': 'PopulationBomb'}

sd_flag_names = Counter()
sd_unmodelled_props = Counter()
# Showdown move properties that the comparison maps to an engine field
MAPPED_PROPS = {'type', 'category', 'basePower', 'accuracy', 'priority', 'target', 'flags', 'secondary', 'secondaries',
                'self', 'selfBoost', 'boosts', 'status', 'volatileStatus', 'sideCondition', 'slotCondition', 'drain',
                'recoil', 'heal', 'critRatio', 'willCrit', 'multihit', 'multiaccuracy', 'hasCrashDamage',
                'selfdestruct', 'forceSwitch', 'selfSwitch', 'pp'}
IGNORED_PROPS = {'name', 'id', 'fullname', 'effectType', 'exists', 'num', 'gen', 'baseMoveType', 'zMove', 'maxMove',
                 'contestType', 'desc', 'shortDesc', 'handlers', 'isNonstandard', 'kind', 'realMove', 'sourceEffect',
                 'noPPBoosts', 'tags', 'isZ', 'isMax', 'condition'}


# Showdown handlers that can apply a status/boost/volatile/side effect (others, e.g. onBasePower or onEffectiveness, cannot)
EFFECT_HANDLERS = {'onHit', 'onAfterHit', 'onAfterSubDamage', 'onHitField', 'onHitSide', 'onAfterMoveSecondarySelf',
                   'self.onHit', 'secondary.onHit', 'secondaries[0].onHit', 'onModifyMove', 'onTryHit', 'onAfterMove'}


def sd_effects(m):
    """Normalise Showdown move effects into comparable groups."""
    tgt = tmap(m['target'])
    eff_tgt = 'User' if tgt == 'User' else 'Opponent'
    out = {}
    # primary boost: engine has a single `boost` (target User or Opponent)
    b = []
    if m.get('boosts'):
        b.append((eff_tgt, sd_boosts(m['boosts'])))
    if (m.get('self') or {}).get('boosts'):
        b.append(('User', sd_boosts(m['self']['boosts'])))
    if (m.get('selfBoost') or {}).get('boosts'):
        b.append(('User', sd_boosts(m['selfBoost']['boosts'])))
    out['boost'] = b[0] if len(b) == 1 else (tuple(b) if b else None)
    out['status'] = (eff_tgt, STATUS.get(m['status'], m['status'])) if m.get('status') else None
    v = []
    if m.get('volatileStatus'):
        v.append((eff_tgt, m['volatileStatus']))
    sv = (m.get('self') or {}).get('volatileStatus')
    if sv and sv != 'mustrecharge':   # mustrecharge is the engine's `recharge` flag
        v.append(('User', sv))
    out['volatile_status'] = v[0] if len(v) == 1 else (tuple(v) if v else None)
    sc = m.get('sideCondition') or m.get('slotCondition')
    if sc:
        out['side_condition'] = ('Opponent' if m['target'] == 'foeSide' else ('User' if tgt == 'User' else 'Opponent'), sc)
    else:
        out['side_condition'] = None
    out['heal'] = (('User' if tgt != 'Opponent' or m['category'] != 'Status' else 'Opponent'), frac(m['heal'])) if m.get('heal') else None
    out['selfdestruct'] = bool(m.get('selfdestruct'))   # 'always' / 'ifHit': value kept in the note
    out['drain'] = frac(m.get('drain'))
    out['recoil'] = frac(m.get('recoil'))
    secs = []
    fn = False
    for s in (m.get('secondaries') or []):
        ch = s.get('chance', 100)
        if s.get('status'):
            secs.append((ch, 'Opponent', 'Status', STATUS.get(s['status'], s['status'])))
        if s.get('volatileStatus'):
            secs.append((ch, 'Opponent', 'VolatileStatus', s['volatileStatus']))
        if s.get('boosts'):
            secs.append((ch, 'Opponent', 'Boost', sd_boosts(s['boosts'])))
        if (s.get('self') or {}).get('boosts'):
            secs.append((ch, 'User', 'Boost', sd_boosts(s['self']['boosts'])))
        if s.get('onHit') or (s.get('self') or {}).get('onHit'):
            fn = True
            secs.append((ch, '?', 'onHit-function', None))
    out['secondaries'] = sorted(secs, key=str) or None
    out['_secondary_fn'] = fn
    return out


def en_effects(e):
    out = {}
    out['boost'] = (e['boost']['target'], en_boosts(e['boost']['boosts'])) if e['boost'] and en_boosts(e['boost']['boosts']) else None
    out['_zero_boost'] = bool(e['boost']) and not en_boosts(e['boost']['boosts'])
    out['status'] = (e['status']['target'], e['status']['status']) if e['status'] else None
    out['volatile_status'] = (e['volatile_status']['target'], e['volatile_status']['volatile_status'].lower()) if e['volatile_status'] else None
    out['side_condition'] = (e['side_condition']['target'], e['side_condition']['condition'].lower()) if e['side_condition'] else None
    h = e['heal']
    out['heal'] = (h['target'], round(h['amount'], 3)) if h and h['amount'] != -1 else None
    out['selfdestruct'] = bool(h and h['amount'] == -1)   # engine encodes user-faints as heal -1 on User
    out['drain'] = e['drain']
    out['recoil'] = e['recoil']
    secs = []
    for s in (e['secondaries'] or []):
        k = s['effect']['kind']
        val = s['effect']['value']
        if k == 'VolatileStatus':
            val = val.lower()
        elif k == 'Boost':
            val = en_boosts(val)
        secs.append((round(s['chance'], 2), s['target'], k, val))
    out['secondaries'] = sorted(secs, key=str) or None
    return out


def fmt(x):
    if x is None:
        return None
    if isinstance(x, tuple):
        return [fmt(y) for y in x]
    if isinstance(x, list):
        return [fmt(y) for y in x]
    return x


def cmp_moves():
    E = EN['moves']
    for mid in SD['legal']['moves']:
        m = SD['moves'][mid]
        compared['moves'] += 1
        for k, v in m.items():
            if v in (None, False, [], {}, '') or k in MAPPED_PROPS or k in IGNORED_PROPS:
                continue
            sd_unmodelled_props[k] += 1
        for f in (m.get('flags') or {}):
            sd_flag_names[f] += 1
        e = E.get(mid.upper())
        fields_compared['moves', 'exists'] += 1
        if e is None:
            add('moves', mid, 'exists', True, False)
            continue

        def chk(field, sdv, env, cls='real', note='', eq=None):
            fields_compared['moves', field] += 1
            same = eq(sdv, env) if eq else sdv == env
            if not same:
                add('moves', mid, field, fmt(sdv), fmt(env), cls, note)

        chk('type', m['type'].upper(), e['move_type'])
        chk('category', m['category'], e['category'])
        chk('basePower', m.get('basePower', 0), e['base_power'],
            *(('needs-reading', 'Showdown computes power in a callback') if m.get('basePowerCallback') or m.get('damageCallback') or m.get('damage') or m.get('ohko') else ()))
        acc = m['accuracy']
        fields_compared['moves', 'accuracy'] += 1
        if acc is True:
            if e['accuracy'] == 100:
                add('moves', mid, 'accuracy', True, 100, 'representation', 'never-miss stored as 100')
            else:
                add('moves', mid, 'accuracy', True, e['accuracy'], 'real', 'never-miss in Showdown, can miss in engine')
        elif acc != e['accuracy']:
            add('moves', mid, 'accuracy', acc, e['accuracy'])
        chk('priority', m.get('priority', 0), e['priority'])
        t = tmap(m['target'])
        fields_compared['moves', 'target'] += 1
        if t == 'Field':
            add('moves', mid, 'target', m['target'], e['target'], 'representation', 'field-wide move; engine has only User/Opponent')
        elif m['target'] == 'adjacentAlly':
            if e['target'] != 'User':
                add('moves', mid, 'target', m['target'], e['target'], 'representation', 'ally-target move (no ally in singles)')
        elif t != e['target']:
            add('moves', mid, 'target', m['target'], e['target'])
        flags = m.get('flags') or {}
        for f in ENGINE_FLAGS:
            chk('flag.' + f, bool(flags.get(f)), bool(e['flags'][f]))
        chk('flag.drag(forceSwitch)', bool(m.get('forceSwitch')), e['flags']['drag'])
        pv_refs = CODE_REFS.get(mid.upper())
        chk('flag.pivot(selfSwitch)', m.get('selfSwitch') or False, e['flags']['pivot'],
            'needs-reading' if pv_refs and m.get('selfSwitch') and not e['flags']['pivot'] else 'real',
            f"selfSwitch={m.get('selfSwitch')}" + (('; engine has move-specific code in ' + ','.join(pv_refs)) if pv_refs else ''),
            eq=lambda a, b: bool(a) == b)
        # crit
        chk('critRatio', m.get('critRatio', 1), 2 if e['increased_crit_ratio'] else 1)
        chk('willCrit', bool(m.get('willCrit')), e['guaranteed_crit'])
        # multihit
        mh = m.get('multihit')
        sdmh = MULTIHIT.get(str(mh), str(mh)) if mh is not None else None
        if mid == 'tripleaxel' and mh == 3:
            sdmh = 'TripleAxel'
        chk('multihit', sdmh, e['multi_hit'], *(('representation', 'engine approximates') if mid == 'populationbomb' else ()))
        fields_compared['moves', 'multiaccuracy'] += 1
        if m.get('multiaccuracy'):
            add('moves', mid, 'multiaccuracy', True, e['multi_hit'], 'representation',
                'engine has no per-hit accuracy; multi_accuracy() exists but multi_hit() approximation is used')
        chk('crash(hasCrashDamage)', bool(m.get('hasCrashDamage')), e['crash'] is not None,
            note=f"engine crash fraction {e['crash']}")
        se, ee = sd_effects(m), en_effects(e)
        # a 100%-chance secondary in one source may be a primary effect in the other: normalise, record as representation
        for kind, prim in (('VolatileStatus', 'volatile_status'), ('Status', 'status'), ('Boost', 'boost')):
            for a, b, who in ((se, ee, 'showdown'), (ee, se, 'engine')):
                for sec in list(a['secondaries'] or []):
                    if sec[0] >= 100 and sec[2] == kind and a[prim] is None and b[prim] == (sec[1], sec[3]):
                        a['secondaries'].remove(sec)
                        a['secondaries'] = a['secondaries'] or None
                        a[prim] = (sec[1], sec[3])
                        fields_compared['moves', 'effect placement'] += 1
                        add('moves', mid, 'effect placement', 'secondary 100%' if who == 'showdown' else 'primary',
                            'secondary 100%' if who == 'engine' else 'primary', 'representation',
                            f'{prim} {sec[3]} stored as a 100% secondary on the {who} side')
        if m.get('mindBlownRecoil') and ee['heal'] == ('User', -0.5):
            fields_compared['moves', 'recoil(special)'] += 1
            add('moves', mid, 'recoil(special)', 'mindBlownRecoil', 'heal -0.5 (User)', 'representation',
                'half-max-HP self damage stored as negative heal')
            ee['heal'] = None
        elif m.get('mindBlownRecoil') or m.get('struggleRecoil'):
            fields_compared['moves', 'recoil(special)'] += 1
            add('moves', mid, 'recoil(special)', 'mindBlownRecoil' if m.get('mindBlownRecoil') else 'struggleRecoil',
                ee['recoil'], 'needs-reading', 'Showdown special recoil (half max HP); engine field empty')
        sd_fn = [h for h in (m.get('handlers') or []) if h in EFFECT_HANDLERS]
        refs = CODE_REFS.get(mid.upper())

        def eff_cls(sdv, env):
            if sdv is None and sd_fn:
                return 'needs-reading', 'Showdown applies effects in ' + ','.join(sd_fn)
            if env is None and refs:
                return 'needs-reading', 'engine has move-specific code in ' + ','.join(refs)
            return 'real', ''
        for f in ['boost', 'status', 'volatile_status', 'side_condition', 'heal', 'selfdestruct']:
            cls, note = eff_cls(se[f], ee[f])
            if f == 'boost' and se[f] and 'evasion' in str(se[f]) and ee['boost'] is None:
                cls, note = 'real', 'engine StatBoosts has no evasion field' + (' (engine boost is all zeros)' if ee['_zero_boost'] else '')
            if f == 'selfdestruct':
                cls, note = eff_cls(se[f] or None, ee[f] or None)
                note = (note + '; ' if note else '') + f"Showdown selfdestruct={m.get('selfdestruct')}; engine field = heal -1 on User"
            chk('effect.' + f, se[f], ee[f], cls, note)
        chk('drain', se['drain'], ee['drain'], eq=close)
        chk('recoil', se['recoil'], ee['recoil'], eq=close)

        def sec_eq(a, b):
            if a is None or b is None:
                return a is None and b is None
            if len(a) != len(b):
                return False
            for x, y in zip(sorted(a, key=lambda t: (t[2], str(t[3]))), sorted(b, key=lambda t: (t[2], str(t[3])))):
                if not (abs(x[0] - y[0]) < 0.01 and x[1:] == y[1:]):
                    return False
            return True
        if se['_secondary_fn']:
            cls, note = 'needs-reading', 'Showdown secondary is an onHit function'
        else:
            cls, note = eff_cls(se['secondaries'], ee['secondaries'])
        chk('secondaries', se['secondaries'], ee['secondaries'], cls, note, eq=sec_eq)
        # numeric fields: flag engine code references for readers, class unchanged
        if refs:
            for d in diffs:
                if d['cat'] == 'moves' and d['id'] == mid and d['field'] in ('basePower', 'type', 'accuracy', 'priority', 'category') and not d['note']:
                    d['note'] = 'engine also has move-specific code in ' + ','.join(refs)


# ---------------------------------------------------------------- megas / species
def cmp_species():
    sp = SD['species']
    legal = SD['legal']['species']
    names = set(EN['pokemon_names'])
    emegas = {m['mega']: m for m in EN['megas']}
    bs_table = EN['base_stats']
    statk = ['hp', 'atk', 'def', 'spa', 'spd', 'spe']
    for sid in legal:
        s = sp[sid]
        compared['species(name enum)'] += 1
        fields_compared['species', 'exists in PokemonName'] += 1
        if sid.upper() not in names:
            add('species', sid, 'exists in PokemonName', True, False)
        if not s['isMega']:
            if sid.upper() in bs_table:
                compared['species(base stats held by engine)'] += 1
                fields_compared['species', 'baseStats'] += 1
                sdv = [s['baseStats'][k] for k in statk]
                if sdv != bs_table[sid.upper()]:
                    add('species', sid, 'baseStats', sdv, bs_table[sid.upper()])
            continue
        compared['megas'] += 1
        fields_compared['megas', 'exists'] += 1
        m = emegas.get(sid.upper())
        if m is None:
            add('megas', sid, 'exists', s['requiredItem'], False)
            continue
        fields_compared['megas', 'requiredItem'] += 1
        req = toid(s['requiredItem']).upper()
        if req != m['item']:
            add('megas', sid, 'requiredItem', req, m['item'])
        fields_compared['megas', 'changesFrom'] += 1
        bases = {x['base'] for x in EN['megas'] if x['mega'] == m['mega']}
        cf = toid(s.get('changesFrom') or s['baseSpecies']).upper()
        if bases != {cf}:
            add('megas', sid, 'changesFrom', cf, sorted(bases), 'real', 'species the stone is used on')
        fields_compared['megas', 'types'] += 1
        sdt = [t.upper() for t in s['types']]
        ent = [t for t in m['types'] if t != 'TYPELESS']
        if sdt != ent:
            add('megas', sid, 'types', sdt, ent, 'real' if set(sdt) != set(ent) else 'representation',
                '' if set(sdt) != set(ent) else 'type order only')
        fields_compared['megas', 'ability'] += 1
        sda = toid(s['abilities'].get('0')).upper()
        if sda != m['ability']:
            add('megas', sid, 'ability', sda, m['ability'])
        sdv = [s['baseStats'][k] for k in statk]
        for i, k in enumerate(statk):
            fields_compared['megas', 'baseStats.' + k] += 1
            if sdv[i] != m['base_stats'][i]:
                add('megas', sid, 'baseStats.' + k, sdv[i], m['base_stats'][i], 'real', 'mega_evolve_target() data')
            tb = m['base_stats_table']
            fields_compared['megas', 'baseStats(table).' + k] += 1
            if tb is None:
                if i == 0:
                    add('megas', sid, 'baseStats(table)', sdv, None, 'real', 'PokemonName::base_stats() panics for this form')
            elif sdv[i] != tb[i]:
                add('megas', sid, 'baseStats(table).' + k, sdv[i], tb[i], 'real', 'PokemonName::base_stats() (used for stat recalculation)')
    # engine megas not legal in Showdown (information only)
    legal_set = set(legal)
    extra = sorted({m['mega'] for m in EN['megas'] if m['mega'].lower() not in legal_set})
    return extra


# ---------------------------------------------------------------- items / abilities
def cmp_enum(cat, legal, enum):
    es = set(enum)
    for x in legal:
        compared[cat] += 1
        fields_compared[cat, 'exists in enum'] += 1
        if x.upper() not in es:
            add(cat, x, 'exists in enum', True, False)
    lu = {x.upper() for x in legal}
    return sorted(e for e in enum if e not in lu)


# ---------------------------------------------------------------- type chart / natures
CODE = {0: 1.0, 1: 2.0, 2: 0.5, 3: 0.0}


def cmp_types():
    tc = SD['typechart']
    types = [t for t in tc if t != 'Stellar']
    for d in types:
        for a in types:
            compared['typechart pairs'] += 1
            fields_compared['typechart', 'multiplier'] += 1
            sdv = CODE[tc[d]['damageTaken'][a]]
            env = EN['typechart'][a.upper()][d.upper()]
            if sdv != env:
                add('typechart', f'{a} -> {d}', 'multiplier', sdv, env)


def cmp_natures():
    for nid, n in SD['natures'].items():
        compared['natures'] += 1
        e = EN['natures'].get(nid.upper())
        fields_compared['natures', 'plus/minus'] += 1
        if e is None:
            add('natures', nid, 'exists', True, False)
            continue
        if (n['plus'], n['minus']) != (e['plus'], e['minus']):
            add('natures', nid, 'plus/minus', [n['plus'], n['minus']], [e['plus'], e['minus']])


cmp_moves()
extra_megas = cmp_species()
extra_items = cmp_enum('items', SD['legal']['items'], EN['items'])
extra_abil = cmp_enum('abilities', SD['legal']['abilities'], EN['abilities'])
cmp_types()
cmp_natures()

engine_only_flags = sorted(set(EN['moves']['TACKLE']['flags']) - set(ENGINE_FLAGS) - set(ENGINE_FLAG_PROPS))
summary = {
    'compared': dict(compared),
    'fields_compared': {f'{c}:{f}': n for (c, f), n in fields_compared.items()},
    'diff_by_cat_cls': dict(Counter(f"{d['cat']}:{d['cls']}" for d in diffs)),
    'diff_by_cat_field': dict(Counter(f"{d['cat']}:{d['field']}" for d in diffs)),
    'showdown_flags_not_in_engine': {k: v for k, v in sd_flag_names.most_common() if k not in ENGINE_FLAGS},
    'showdown_move_props_not_compared': dict(sd_unmodelled_props.most_common()),
    'engine_megas_not_legal_in_showdown': extra_megas,
    'engine_items_not_legal_in_showdown': extra_items,
    'engine_abilities_not_legal_in_showdown': extra_abil,
    'engine_moves_total': len(EN['moves']),
}
json.dump({'summary': summary, 'diffs': diffs}, open(os.path.join(HERE, 'out', 'diff.json'), 'w', encoding='utf-8'), indent=1)
print(json.dumps({k: summary[k] for k in ['compared', 'diff_by_cat_cls']}, indent=1))
