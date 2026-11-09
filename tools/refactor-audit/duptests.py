#!/usr/bin/env python3
"""Find functions whose full source text is identical (modulo whitespace) in two live files of one crate."""
import csv, sys, os, re, collections, json, hashlib
A = sys.argv[1]
files = {r['file']: r for r in csv.DictReader(open(os.path.join(A, 'files.tsv')), delimiter='\t')}
fns = list(csv.DictReader(open(os.path.join(A, 'functions.tsv')), delimiter='\t', quoting=csv.QUOTE_NONE))
cache = {}
def lines(p):
    if p not in cache: cache[p] = open(p, encoding='utf-8', errors='replace').read().split('\n')
    return cache[p]
def text(f):
    L = lines(f['file']); a = int(f['line']) - 1; b = a + int(f['loc'])
    return re.sub(r'\s+', '', '\n'.join(L[a:b]))
groups = collections.defaultdict(list)
for f in fns:
    k = files[f['file']]['kind']
    if k in ('orphan', 'included') or int(f['loc']) < 6: continue
    t = text(f)
    groups[(f['crate'], f['name'], hashlib.sha1(t.encode()).hexdigest())].append(f)
pairs = collections.Counter(); pair_loc = collections.Counter(); pair_tests = collections.Counter()
rows = []
for (crate, name, h), g in groups.items():
    fl = sorted(set(x['file'] for x in g))
    if len(fl) < 2: continue
    is_test = any(x['test'] == '1' for x in g)
    rows.append(dict(crate=crate, name=name, files=fl, loc=int(g[0]['loc']), test=is_test, attr_test=all(x['test'] == '1' for x in g)))
    for i in range(len(fl)):
        for j in range(i + 1, len(fl)):
            pairs[(fl[i], fl[j])] += 1; pair_loc[(fl[i], fl[j])] += int(g[0]['loc'])
            if is_test: pair_tests[(fl[i], fl[j])] += 1
t = [r for r in rows if r['test']]; p = [r for r in rows if not r['test']]
print(f"text-identical functions in 2+ live files: tests={len(t)} (redundant LOC {sum(r['loc']*(len(r['files'])-1) for r in t)}), non-test={len(p)} (redundant LOC {sum(r['loc']*(len(r['files'])-1) for r in p)})")
print("\nfile pairs with the most identical functions (count, of which tests, LOC):")
for (a, b), n in pairs.most_common(40):
    print(f"  {n:4d} {pair_tests[(a,b)]:4d} {pair_loc[(a,b)]:6d}  {a.replace('crates/','')}  ==  {b.replace('crates/','')}")
json.dump(dict(rows=rows, pairs=[dict(a=a, b=b, n=n, tests=pair_tests[(a, b)], loc=pair_loc[(a, b)]) for (a, b), n in pairs.most_common()]), open(os.path.join(A, 'duptests.json'), 'w'), indent=1)
