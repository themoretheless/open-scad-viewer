#!/usr/bin/env python3
"""Type-1 clone detector: maximal runs of identical normalized code lines across files.
usage: dupes.py <repo_root> <out.json> <min_lines> <glob-ish suffixes comma separated> <dir>..."""
import sys, os, re, json, collections, hashlib
root, out_path, MIN = sys.argv[1], sys.argv[2], int(sys.argv[3])
suffixes = tuple(sys.argv[4].split(','))
dirs = sys.argv[5:]
SKIP_DIRS = {'target', 'vendor', 'node_modules', '.git', 'generated', 'dist'}
TRIVIAL = re.compile(r'^[\s{}()\[\];,]*$')
def norm_lines(path):
    out = []  # (lineno, normalized)
    try: text = open(path, encoding='utf-8', errors='replace').read()
    except OSError: return out
    depth = 0
    for i, line in enumerate(text.split('\n'), 1):
        t = line.strip()
        if depth > 0:
            depth += t.count('/*') - t.count('*/'); depth = max(depth, 0); continue
        if not t or t.startswith('//'): continue
        if t.startswith('/*'):
            depth += t.count('/*') - t.count('*/'); depth = max(depth, 0); continue
        if TRIVIAL.match(t): continue
        if t.startswith(('use ', 'pub use ', 'import ', 'export {', 'mod ', 'pub mod ', '#[', '#![')): continue
        out.append((i, re.sub(r'\s+', ' ', t)))
    return out
files = []
for d in dirs:
    for dp, dn, fn in os.walk(os.path.join(root, d)):
        dn[:] = [x for x in dn if x not in SKIP_DIRS]
        for f in fn:
            if f.endswith(suffixes): files.append(os.path.join(dp, f))
files.sort()
data = []; total_sig = 0
for p in files:
    nl = norm_lines(p); data.append(nl); total_sig += len(nl)
# hash windows
H = lambda s: hash(s)
index = collections.defaultdict(list)
for fi, nl in enumerate(data):
    hs = [H(x[1]) for x in nl]
    data[fi] = (nl, hs)
    for i in range(0, len(hs) - MIN + 1):
        index[hash(tuple(hs[i:i + MIN]))].append((fi, i))
covered = [bytearray(len(nl)) for nl, _ in data]
clones = []  # maximal clone pairs
seen_pairs = set()
MAXOCC = 40
for key, occ in index.items():
    if len(occ) < 2: continue
    for fi, i in occ:
        for k in range(i, i + MIN): covered[fi][k] = 1
    if len(occ) > MAXOCC: continue
    for a in range(len(occ)):
        for b_ in range(a + 1, len(occ)):
            (f1, i1), (f2, i2) = occ[a], occ[b_]
            if f1 == f2 and abs(i1 - i2) < MIN: continue
            h1, h2 = data[f1][1], data[f2][1]
            # only start at maximal beginnings
            if i1 > 0 and i2 > 0 and h1[i1 - 1] == h2[i2 - 1]: continue
            n = 0
            while i1 + n < len(h1) and i2 + n < len(h2) and h1[i1 + n] == h2[i2 + n]: n += 1
            pk = (f1, i1, f2, i2)
            if pk in seen_pairs: continue
            seen_pairs.add(pk)
            clones.append((n, f1, i1, f2, i2))
clones.sort(reverse=True)
rel = lambda p: os.path.relpath(p, root)
per_file = []
for fi, p in enumerate(files):
    sig = len(data[fi][0]); dup = sum(covered[fi])
    if sig: per_file.append(dict(file=rel(p), sig=sig, dup=dup))
def group_of(path):
    parts = path.split('/')
    if parts[0] == 'crates' and len(parts) > 2: return 'crates/' + parts[1]
    return '/'.join(parts[:2]) if len(parts) > 2 else parts[0]
groups = collections.defaultdict(lambda: [0, 0])
for r in per_file:
    g = groups[group_of(r['file'])]; g[0] += r['sig']; g[1] += r['dup']
pair_groups = collections.Counter()
for n, f1, i1, f2, i2 in clones:
    a, b_ = sorted((group_of(rel(files[f1])), group_of(rel(files[f2]))))
    pair_groups[(a, b_)] += n
top = []
for n, f1, i1, f2, i2 in clones[:400]:
    top.append(dict(lines=n, a=rel(files[f1]), a_line=data[f1][0][i1][0], a_end=data[f1][0][i1 + n - 1][0], b=rel(files[f2]), b_line=data[f2][0][i2][0], b_end=data[f2][0][i2 + n - 1][0]))
# whole-file duplicates
byhash = collections.defaultdict(list)
for fi, p in enumerate(files):
    nl = data[fi][0]
    if len(nl) >= 15:
        byhash[hashlib.sha1('\n'.join(x[1] for x in nl).encode()).hexdigest()].append((rel(p), len(nl)))
identical = sorted([v for v in byhash.values() if len(v) > 1], key=lambda v: -v[0][1] * (len(v) - 1))
res = dict(min_lines=MIN, files=len(files), sig_lines=total_sig, dup_lines=sum(sum(c) for c in covered),
           groups={k: dict(sig=v[0], dup=v[1], pct=round(100 * v[1] / v[0], 1) if v[0] else 0) for k, v in groups.items()},
           pair_groups=[dict(a=k[0], b=k[1], lines=v) for k, v in pair_groups.most_common(60)],
           top=top, identical_files=identical[:200], identical_file_sets=len(identical),
           identical_redundant_lines=sum(v[0][1] * (len(v) - 1) for v in identical),
           per_file=sorted(per_file, key=lambda r: -r['dup']))
json.dump(res, open(out_path, 'w'), indent=1)
print(f"files={res['files']} significant_lines={total_sig} duplicated_lines={res['dup_lines']} ({100*res['dup_lines']/max(total_sig,1):.1f}%) clone_pairs={len(clones)} identical_file_sets={len(identical)} redundant_lines_in_identical_files={res['identical_redundant_lines']}")
