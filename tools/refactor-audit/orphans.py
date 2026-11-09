#!/usr/bin/env python3
"""Classify .rs files that no Cargo target reaches: stale copy of a compiled file, or unique content.
usage: orphans.py <audit_dir> <repo_root>   (writes orphans.json, orphan-decision.json, delete-rs.txt)"""
import csv, sys, os, re, json, hashlib, collections, subprocess
A, ROOT = sys.argv[1], sys.argv[2]
files = list(csv.DictReader(open(os.path.join(A, 'files.tsv')), delimiter='\t'))
fns = list(csv.DictReader(open(os.path.join(A, 'functions.tsv')), delimiter='\t', quoting=csv.QUOTE_NONE))
mods = list(csv.DictReader(open(os.path.join(A, 'moddecls.tsv')), delimiter='\t', quoting=csv.QUOTE_NONE))
git = lambda *a: subprocess.run(['git', '-C', ROOT, *a], capture_output=True, text=True).stdout
tracked = set(git('ls-files', 'crates').split('\n'))
status = {l[3:].strip(): l[:2] for l in git('status', '--porcelain').split('\n') if l}
refd = set(re.findall(r'crates/[A-Za-z0-9_./-]+\.rs', git('grep', '-hoE', r'crates/[A-Za-z0-9_./-]+\.rs', '--', '.', ':!docs/qualification', ':!crates/*/src/**', ':!crates/*/tests/**')))
OUTSIDE = {'modelgraph-runtime', 'independent-modeling', 'tensor-core', 'math-core', 'raster-core', 'modelgraph-text', 'compute-core'}
TRIV = re.compile(r'^[\s{}()\[\];,]*$')
def norm(path):
    try: text = open(os.path.join(ROOT, path), encoding='utf-8', errors='replace').read()
    except OSError: return ''
    return '\n'.join(re.sub(r'\s+', ' ', t) for t in (l.strip() for l in text.split('\n')) if t and not t.startswith('//') and not TRIV.match(t))
live, sigs = {}, {}
for f in files:
    if f['kind'] in ('orphan', 'included'): continue
    n = norm(f['file']); sigs.setdefault(hashlib.sha1(n.encode()).hexdigest(), f['file']); live[f['file']] = set(n.split('\n'))
live_names = collections.defaultdict(set); ofns = collections.defaultdict(list)
kind = {f['file']: f['kind'] for f in files}
for f in fns:
    if kind[f['file']] == 'orphan': ofns[f['file']].append(f['name'])
    elif kind[f['file']] != 'included': live_names[f['crate']].add(f['name'])
rows = []
for f in files:
    if f['kind'] != 'orphan': continue
    p = f['file']; n = norm(p); lines = set(n.split('\n')); h = hashlib.sha1(n.encode()).hexdigest()
    if h in sigs: sim, twin = 1.0, sigs[h]
    else:
        sim, twin = 0.0, ''
        for x, lx in live.items():
            if x.split('/')[1] != p.split('/')[1]: continue
            s = len(lines & lx) / max(len(lines), 1)
            if s > sim: sim, twin = s, x
    reasons = []
    if f['crate'] in OUTSIDE: reasons.append('crate dir outside workspace')
    if p not in tracked or status.get(p, '').strip(): reasons.append('untracked or modified (work in progress)')
    missing = [x for x in ofns[p] if x not in live_names[f['crate']]]
    if missing: reasons.append(f"{len(missing)} fn(s) with no live namesake: {', '.join(missing[:3])}")
    if sim < 0.8: reasons.append(f"only {sim:.2f} of lines exist in a compiled file")
    if p in refd: reasons.append('path referenced from tracked non-Rust files')
    rows.append(dict(file=p, crate=f['crate'], loc=int(f['loc']), sim=round(sim, 3), twin=twin, reasons=reasons))
delete = {r['file']: r for r in rows if not r['reasons']}; keep = {r['file']: r for r in rows if r['reasons']}
bymod = collections.defaultdict(list)
for m in mods: bymod[m['file']].append(m)
def children(f):
    d, stem = os.path.dirname(f), os.path.basename(f)[:-3]
    for m in bymod.get(f, []):
        inl = m['inline'].split('::') if m['inline'] else []
        for base in (d, os.path.join(d, stem)):
            if m['path_attr']: yield os.path.normpath(os.path.join(base, *inl, m['path_attr']))
            else:
                yield os.path.normpath(os.path.join(base, *inl, m['mod'] + '.rs')); yield os.path.normpath(os.path.join(base, *inl, m['mod'], 'mod.rs'))
moved = True
while moved:
    moved = False
    for k in list(keep):
        for c in children(k):
            if c in delete:
                r = delete.pop(c); r['reasons'] = [f'child module of kept file {k}']; keep[c] = r; moved = True
json.dump(rows, open(os.path.join(A, 'orphans.json'), 'w'), indent=1)
json.dump(dict(delete=list(delete.values()), keep=list(keep.values())), open(os.path.join(A, 'orphan-decision.json'), 'w'), indent=1)
open(os.path.join(A, 'delete-rs.txt'), 'w').write(''.join(p + '\n' for p in sorted(delete)))
print(f"unreachable .rs files: {len(rows)} ({sum(r['loc'] for r in rows)} LOC); provably stale copies: {len(delete)} ({sum(r['loc'] for r in delete.values())} LOC); kept for a decision: {len(keep)}")
