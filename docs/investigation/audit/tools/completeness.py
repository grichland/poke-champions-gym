"""Completeness checks 4c and 4d: every legal move, item and ability against its audit row.
usage: python completeness.py <docs/investigation dir> <out.json>
Flags rows to re-read; it judges nothing about the rules themselves."""
import csv, glob, json, os, re, sys

root, out = sys.argv[1], sys.argv[2]
dump = json.load(open(f'{root}/data-diff/out/showdown-champions.json', encoding='utf-8'))
desc = json.load(open(f'{root}/data-diff/out/descriptions.json', encoding='utf-8'))
VERDICTS = ('MATCH', 'PLAIN', 'DATA', 'LOGIC', 'PARTIAL', 'MISSING', 'CORE', 'UNSURE')
NO_EFFECT = re.compile(r'^(No additional effect|No competitive use)', re.I)
ENGINE_REF = re.compile(r'\b[a-z_]+\.rs\b|:\d{2,5}\b|\b[A-Z]{1,3}:\d+')

report, flagged, counts = {}, [], {}
for kind in ('moves', 'abilities', 'items'):
    ids = [r['id'] for r in csv.DictReader(open(f'{root}/coverage/03-coverage-{kind}.csv', encoding='utf-8'))]
    used = {r['id']: r.get('used', '') for r in csv.DictReader(open(f'{root}/coverage/03-coverage-{kind}.csv', encoding='utf-8'))}
    idset = set(ids)
    rows = {}
    for p in sorted(glob.glob(f'{root}/audit/{kind}/*.md')):
        for line in open(p, encoding='utf-8'):
            if not line.startswith('|'):
                continue
            cells = [c.strip() for c in line.strip().strip('|').split('|')]
            key = re.sub(r'[^a-z0-9]', '', cells[0].strip('`* ').lower().split('(')[0])
            if key in idset and key not in rows:
                verdict = next((v for c in cells[1:4] for v in VERDICTS if re.search(rf'\b{v}\b', c)), '?')
                rows[key] = {'file': os.path.basename(p), 'verdict': verdict, 'cells': cells}
    c = {'legal': len(ids), 'rows': len(rows), 'no_row': 0, 'verdict_unread': 0}
    for i in ids:
        sd, d, row = dump[kind].get(i, {}), desc[kind].get(i, {}), rows.get(i)
        handlers = sd.get('handlers', [])
        text = d.get('shortDesc') or d.get('desc') or ''
        has_effect = bool(text) and not NO_EFFECT.match(text)
        why = []
        if row is None:
            c['no_row'] += 1
            why.append('no audit row')
        else:
            v, cells = row['verdict'], row['cells']
            engine_cell = ' '.join(cells[3:6]) if len(cells) > 5 else ' '.join(cells)
            cites = bool(ENGINE_REF.search(engine_cell))
            if v == '?':
                c['verdict_unread'] += 1
                why.append('verdict not parsed')
            if v == 'PLAIN' and (handlers or (kind != 'moves' and has_effect)):
                why.append('PLAIN but Showdown has handlers: ' + ', '.join(handlers[:6]))
            if v == 'PLAIN' and kind == 'moves' and has_effect and (sd.get('secondary') or sd.get('secondaries') or sd.get('self') or sd.get('boosts') or sd.get('status') or sd.get('volatileStatus') or sd.get('sideCondition') or sd.get('drain') or sd.get('recoil') or sd.get('heal') or sd.get('multihit') or sd.get('selfSwitch') or sd.get('weather') or sd.get('terrain') or sd.get('pseudoWeather') or sd.get('forceSwitch')):
                why.append('PLAIN but the move has an effect field')
            if v == 'MATCH' and has_effect and not cites:
                why.append('MATCH with no engine file or line cited')
            if v == 'MATCH' and len(handlers) >= 3 and len(' '.join(cells)) < 220:
                why.append(f'MATCH on {len(handlers)} handlers with a very short row')
        entry = {'id': i, 'kind': kind, 'used': used.get(i, ''), 'verdict': row['verdict'] if row else None,
                 'file': row['file'] if row else None, 'handlers': handlers, 'desc': text, 'flags': why}
        report[f'{kind}/{i}'] = entry
        if why:
            flagged.append(entry)
    for v in VERDICTS + ('?',):
        c[v] = sum(1 for i in ids if rows.get(i, {}).get('verdict') == v)
    c['with_effect_desc'] = sum(1 for i in ids if (desc[kind].get(i, {}).get('shortDesc') or '') and not NO_EFFECT.match(desc[kind][i].get('shortDesc') or ''))
    c['handler_pairs'] = sum(len(dump[kind].get(i, {}).get('handlers', [])) for i in ids)
    c['flagged'] = sum(1 for f in flagged if f['kind'] == kind)
    counts[kind] = c
json.dump({'counts': counts, 'flagged': flagged, 'all': report}, open(out, 'w', encoding='utf-8'), indent=1)
print(json.dumps(counts, indent=1))
for f in flagged:
    print(f"{f['kind']:9} {f['id']:22} used={f['used']:1} {f['verdict']}  {f['file']}  :: {'; '.join(f['flags'])[:110]}")
