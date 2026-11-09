#!/usr/bin/env python3
"""Module dependency graphs from audit-rs refs: per-crate top-level module graph (SCCs, fan-in/out),
cross-crate usage, unused dependency candidates."""
import csv, sys, os, json, collections
A = sys.argv[1]
def rows(n): return list(csv.DictReader(open(os.path.join(A, n), newline=''), delimiter='\t', quoting=csv.QUOTE_NONE))
files, refs, rootuse = rows('files.tsv'), rows('refs.tsv'), rows('rootuse.tsv')
deps = [l.rstrip('\n').split('\t') for l in open(os.path.join(A, '..', 'deps.tsv'))]
top_loc = collections.defaultdict(collections.Counter)   # crate -> top module -> prod loc
top_files = collections.defaultdict(collections.Counter)
for f in files:
    if f['kind'] != 'lib' or f['test'] == '1': continue
    top = f['module'].split('::')[0] if f['module'] else '(root)'
    top_loc[f['crate']][top] += int(f['loc']); top_files[f['crate']][top] += 1
alias = collections.defaultdict(dict)
for u in rootuse:
    if u['kind'] != 'lib' or u['glob'] == '1': continue
    p = u['path'].split('::')
    if p and p[0] in ('crate', 'self'): p = p[1:]
    if p: alias[u['crate']][u['alias']] = p[0]
def scc(nodes, adj):
    index = {}; low = {}; onst = set(); st = []; out = []; counter = [0]
    for root in nodes:
        if root in index: continue
        work = [(root, iter(adj.get(root, ())))]
        index[root] = low[root] = counter[0]; counter[0] += 1; st.append(root); onst.add(root)
        while work:
            v, it = work[-1]
            advanced = False
            for w in it:
                if w not in index:
                    index[w] = low[w] = counter[0]; counter[0] += 1; st.append(w); onst.add(w)
                    work.append((w, iter(adj.get(w, ())))); advanced = True; break
                elif w in onst: low[v] = min(low[v], index[w])
            if advanced: continue
            work.pop()
            if work: low[work[-1][0]] = min(low[work[-1][0]], low[v])
            if low[v] == index[v]:
                comp = []
                while True:
                    w = st.pop(); onst.discard(w); comp.append(w)
                    if w == v: break
                out.append(comp)
    return out
result = {}
edges_all = collections.defaultdict(collections.Counter)
unresolved_root = collections.defaultdict(collections.Counter)
for r in refs:
    if r['tkind'] != 'crate' or r['kind'] != 'lib' or r['test'] == '1': continue
    c = r['crate']; frm = r['from_module'].split('::')[0] if r['from_module'] else '(root)'
    to = r['t0']
    if not to: continue
    if to not in top_loc[c]:
        if to in alias[c] and alias[c][to] in top_loc[c]: to = alias[c][to]
        else:
            unresolved_root[c][to] += int(r['count']); to = '(root)'
    if frm != to: edges_all[c][(frm, to)] += int(r['count'])
for c, ed in edges_all.items():
    nodes = sorted(top_loc[c]); adj = collections.defaultdict(set); radj = collections.defaultdict(set)
    for (a, b), n in ed.items():
        if a == '(root)' or b == '(root)': continue      # the root re-export hub is not a design dependency
        adj[a].add(b); radj[b].add(a)
    comps = [x for x in scc(nodes, adj) if len(x) > 1]
    comps.sort(key=len, reverse=True)
    in_cycle = set(x for comp in comps for x in comp)
    fan_in = sorted(((len(radj[n]), n) for n in nodes), reverse=True)[:15]
    fan_out = sorted(((len(adj[n]), n) for n in nodes), reverse=True)[:15]
    leaves = [n for n in nodes if not adj[n] and n != '(root)']
    isolated = [n for n in nodes if not adj[n] and not radj[n] and n != '(root)']
    via_root = sum(n for (a, b), n in ed.items() if b == '(root)')
    result[c] = dict(modules=len(nodes), edges=len([1 for (a, b) in ed if a != '(root)' and b != '(root)']),
                     scc=[dict(size=len(x), loc=sum(top_loc[c][m] for m in x), members=sorted(x, key=lambda m: -top_loc[c][m])) for x in comps],
                     scc_count=len(comps), modules_in_cycles=len(in_cycle), loc_in_cycles=sum(top_loc[c][m] for m in in_cycle),
                     loc=sum(top_loc[c].values()), fan_in=fan_in, fan_out=fan_out, leaves=len(leaves), isolated=len(isolated),
                     isolated_loc=sum(top_loc[c][m] for m in isolated), refs_via_root=via_root,
                     unresolved_root=unresolved_root[c].most_common(12), root_loc=top_loc[c].get('(root)', 0))
# cross-crate usage
ext = collections.defaultdict(collections.Counter); ext_mod = collections.defaultdict(collections.Counter); ext_any = collections.defaultdict(collections.Counter)
for r in refs:
    if r['tkind'] != 'extern': continue
    scope = 'prod' if (r['kind'] in ('lib', 'bin') and r['test'] == '0') else 'dev'
    ext_any[(r['crate'], scope)][r['t0']] += int(r['count'])
    if scope == 'prod':
        ext[r['crate']][r['t0']] += int(r['count']); ext_mod[r['t0']][(r['crate'], r['t1'])] += int(r['count'])
unused = []
for d in deps:
    if len(d) < 3 or d[2] == 'self': continue
    crate, ident, kind = d[0], d[1], d[2]
    used_prod = ext_any[(crate, 'prod')][ident]; used_dev = ext_any[(crate, 'dev')][ident]
    if kind == 'normal' and used_prod == 0: unused.append(dict(crate=crate, dep=ident, kind=kind, used_in_tests=used_dev, optional=(d[4] if len(d) > 4 else '')))
    if kind == 'dev' and used_dev == 0 and used_prod == 0: unused.append(dict(crate=crate, dep=ident, kind=kind, used_in_tests=0, optional=''))
json.dump(dict(graphs=result, extern={k: dict(v) for k, v in ext.items()}, unused_deps=unused,
               top_loc={c: dict(v) for c, v in top_loc.items()},
               ext_mod={k: [dict(crate=a, item=b, count=n) for (a, b), n in v.most_common(400)] for k, v in ext_mod.items()}),
          open(os.path.join(A, 'graph.json'), 'w'), indent=1)
for c in ['brep-core', 'nurbs-core', 'geometry-bridge', 'cad-predicates', 'polygon-core', 'photogrammetry-core', 'planar-geometry', 'cad-step', 'openscad-core', 'nurbs-intersect', 'brep-intersect']:
    g = result.get(c)
    if not g: continue
    print(f"\n### {c}: top-level modules={g['modules']} edges={g['edges']} prodLOC={g['loc']} rootLOC={g['root_loc']} | cyclic groups={g['scc_count']} modules in cycles={g['modules_in_cycles']} ({g['loc_in_cycles']} LOC, {100*g['loc_in_cycles']//max(g['loc'],1)}%) | leaves={g['leaves']} isolated={g['isolated']} ({g['isolated_loc']} LOC) | refs via crate root={g['refs_via_root']}")
    for s in g['scc'][:3]: print(f"   SCC size={s['size']} loc={s['loc']}: {', '.join(s['members'][:18])}{' ...' if s['size'] > 18 else ''}")
    print("   fan-in :", ', '.join(f"{n}({k})" for k, n in g['fan_in'][:10]))
    print("   fan-out:", ', '.join(f"{n}({k})" for k, n in g['fan_out'][:10]))
    if g['unresolved_root']: print("   crate::X not resolved to a module:", g['unresolved_root'][:8])
print("\n### unused dependency candidates:", len(unused))
for u in unused: print("  ", u)
