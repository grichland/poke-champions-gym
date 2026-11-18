"""Check 4a: compare the second read with the first, row by row.
usage: python compare_reads.py <docs/investigation dir> [second-read file ...]
Agreement is on the class: fine (MATCH, PLAIN, CORE) against not fine (DATA, LOGIC, PARTIAL, MISSING)."""
import csv, glob, os, re, sys

root = sys.argv[1]
files = sys.argv[2:] or sorted(glob.glob(f'{root}/audit/second-read/*.md'))
VERDICTS = ('MATCH', 'PLAIN', 'DATA', 'LOGIC', 'PARTIAL', 'MISSING', 'CORE', 'UNSURE')
FINE = {'MATCH', 'PLAIN', 'CORE'}


def table(paths, ids):
    rows = {}
    for p in paths:
        for line in open(p, encoding='utf-8'):
            if not line.startswith('|'):
                continue
            cells = [c.strip() for c in line.strip().strip('|').split('|')]
            key = re.sub(r'[^a-z0-9]', '', cells[0].strip('`* ').lower().split('(')[0])
            if key in ids and key not in rows:
                v = next((v for c in cells[1:4] for v in VERDICTS if re.search(rf'\b{v}\b', c)), '?')
                rows[key] = (v, ' | '.join(cells[1:])[:400])
    return rows


total = agree = 0
for f in files:
    kind = os.path.basename(f).split('-')[0]
    ids = [r['id'] for r in csv.DictReader(open(f'{root}/coverage/03-coverage-{kind}.csv', encoding='utf-8'))]
    first = table(sorted(glob.glob(f'{root}/audit/{kind}/*.md')), set(ids))
    second = table([f], set(ids))
    n = a = 0
    out = []
    for i in ids:
        if i not in second:
            continue
        n += 1
        v1, v2 = first[i][0], second[i][0]
        if v1 == 'UNSURE' or v2 == 'UNSURE' or (v1 in FINE) != (v2 in FINE):
            out.append((i, v1, v2))
        else:
            a += 1
    total += n
    agree += a
    print(f'{os.path.basename(f)}: {n} rows, agree {a}, disagree {n - a} ({100 * (n - a) / max(n, 1):.0f}%)')
    for i, v1, v2 in out:
        print(f'   {i:20} first {v1:8} second {v2:8}')
        print(f'      1: {first[i][1][:230]}')
        print(f'      2: {second[i][1][:230]}')
print(f'ALL: {total} rows, agree {agree}, disagree {total - agree}')
