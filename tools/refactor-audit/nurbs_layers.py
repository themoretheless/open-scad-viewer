#!/usr/bin/env python3
"""Upward (layer-violating) references between the domain directories of nurbs-core. usage: nurbs_layers.py <audit_dir>"""
import csv, sys, os, collections, json
A = sys.argv[1]
def rows(n): return list(csv.DictReader(open(os.path.join(A, n), newline=''), delimiter='\t', quoting=csv.QUOTE_NONE))
files, refs, rootuse = rows('files.tsv'), rows('refs.tsv'), rows('rootuse.tsv')
C = 'nurbs-core'
# Bottom -> top. Same number = same layer. Change here and re-run to evaluate another ordering.
LAYER = {'numerics': 0, 'core': 1, 'foundation': 2, 'certificates': 2, 'distance': 2, 'analysis': 3, 'trim': 3, 'intersection': 4, 'offset': 4, 'join': 4,
         'editing': 4, 'construct': 5, 'sweeps': 5, 'tessellation': 5, 'transport': 6, 'domains': 7}
tops = collections.Counter()
for f in files:
    if f['crate'] == C and f['kind'] == 'lib' and f['test'] == '0': tops[f['module'].split('::')[0] if f['module'] else '(root)'] += int(f['loc'])
alias = {}
for u in rootuse:
    if u['crate'] == C and u['kind'] == 'lib' and u['glob'] == '0':
        p = u['path'].split('::')
        if p and p[0] in ('crate', 'self'): p = p[1:]
        if p: alias[u['alias']] = p[0]
ed = collections.Counter(); ex = collections.defaultdict(collections.Counter)
for r in refs:
    if r['crate'] != C or r['tkind'] != 'crate' or r['kind'] != 'lib' or r['test'] == '1': continue
    a = r['from_module'].split('::')[0] if r['from_module'] else '(root)'; b = r['t0']
    if b not in tops: b = alias.get(b, '(root)')
    if b not in tops or '(root)' in (a, b) or a == b: continue
    ed[(a, b)] += int(r['count']); ex[(a, b)][r['file']] += int(r['count'])
up = sorted(((a, b, n) for (a, b), n in ed.items() if a in LAYER and b in LAYER and LAYER[b] > LAYER[a]), key=lambda x: -x[2])
flat = {t: v for t, v in tops.items() if t not in LAYER and t != '(root)'}
json.dump(dict(layer=LAYER, up=[dict(a=a, b=b, refs=n, files=len(ex[(a, b)]), examples=[f for f, _ in ex[(a, b)].most_common(5)]) for a, b, n in up], flat=flat), open(os.path.join(A, 'nurbs-layers.json'), 'w'), indent=1)
print(f"domain dirs: {sum(tops[k] for k in LAYER)} LOC of {sum(tops.values())}; flat top-level modules outside them: {len(flat)} ({sum(flat.values())} LOC)")
print(f"upward edges: {len(up)} with {sum(n for *_, n in up)} references")
for a, b, n in up: print(f"  {a:12s} -> {b:13s} refs={n:4d} files={len(ex[(a, b)]):3d}  {', '.join(f.replace('crates/nurbs-core/src/', '') for f, _ in ex[(a, b)].most_common(3))}")
