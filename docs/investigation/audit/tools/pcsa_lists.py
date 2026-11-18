"""Check 4b: every move, ability, item and condition that pcsa's feature and damage code names, against the audit.
usage: python pcsa_lists.py <docs/investigation dir> <pokemon_champions dir>"""
import json, os, re, sys

root, base = sys.argv[1], sys.argv[2]
rows = json.load(open(f'{root}/audit/completeness-rows.json', encoding='utf-8'))['all']
dump = json.load(open(f'{root}/data-diff/out/showdown-champions.json', encoding='utf-8'))
legal = {k: set(dump['legal'][k]) for k in ('moves', 'abilities', 'items')}
every = {k: set(dump[k]) for k in ('moves', 'abilities', 'items')}
FINE = {'MATCH', 'PLAIN', 'CORE'}
SOURCES = [
    'pcsa/src/pcsa/rl_training/features_rows.py', 'pcsa/src/pcsa/rl_training/features_field.py',
    'pcsa/src/pcsa/rl_training/features_moves.py', 'pcsa/src/pcsa/rl_training/features_log.py',
    'pcsa/src/pcsa/rl_training/turns.py', 'pcsa/src/pcsa/battle/damage.py', 'pcsa/src/pcsa/belief/calc.py',
    'pcsa/src/pcsa/belief/keys.py', 'pcsa/src/pcsa/battle/trapping.py', 'pcsa/src/pcsa/battle/stats.py',
    'pcsa/src/pcsa/players/no_effect.py', 'pcsa/src/pcsa/players/disciplined.py', 'pcsa/src/pcsa/players/tactics.py',
    'pcsa/src/pcsa/players/threat.py', 'pcsa/src/pcsa/players/max_damage.py',
    'research/pcsa-rl/rl2/opponents/tower.py', 'research/pcsa-rl/rl2/opponents/switching.py',
    'poke-engine/docs/investigation/low-latency-recovered/rl2/features_ll.py',
]
# plain English words that are also ids; only counted when quoted as a whole string
total = {}
for src in SOURCES:
    p = os.path.join(base, src)
    if not os.path.exists(p):
        print('MISSING SOURCE', src)
        continue
    text = open(p, encoding='utf-8').read()
    words = set(re.findall(r'["\']([a-z0-9]{3,})["\']', text))
    for lit in re.findall(r'["\']([a-z0-9 ]{8,})["\']', text):  # "a b c".split() lists
        words |= set(lit.split())
    found = {'legal': [], 'illegal': []}
    for w in sorted(words):
        for k in ('moves', 'abilities', 'items'):
            if w in every[k]:
                found['legal' if w in legal[k] else 'illegal'].append((k, w))
    bad = [(k, w, rows[f'{k}/{w}']['verdict']) for k, w in found['legal'] if rows[f'{k}/{w}']['verdict'] not in FINE]
    ok = len(found['legal']) - len(bad)
    print(f'{src.split("/")[-1]:22} names {len(found["legal"]):3} legal entries: {ok:3} fine in the engine, {len(bad):3} already in the register as wrong; {len(found["illegal"])} not legal')
    for k, w in found['legal']:
        total[(k, w)] = rows[f'{k}/{w}']['verdict']
no_row = [kw for kw in total if total[kw] is None]
print('distinct legal entries named by pcsa:', len(total), '| without an audit row:', len(no_row))
by = {}
for (k, w), v in total.items():
    by.setdefault(v, []).append(w)
for v in sorted(by, key=lambda x: -len(by[x])):
    print(f'  {v:8} {len(by[v]):3}  ' + ' '.join(sorted(by[v]))[:260])
