#!/usr/bin/env python3
"""Reachability of src/ modules from real entry points. usage: ts_reach.py <audit_dir>"""
import json, sys, os, collections
A = sys.argv[1]
d = json.load(open(os.path.join(A, 'ts.json'))); F = {f['file']: f for f in d['files']}
adj = collections.defaultdict(set)
for e in d['edges']:
    if e['to']: adj[e['from']].add(e['to'])
def reach(roots):
    seen, st = set(), [r for r in roots if r in F]
    while st:
        x = st.pop()
        if x not in seen: seen.add(x); st.extend(adj[x] - seen)
    return seen
src = [f for f in F if f.startswith('src/')]
prod = reach(['src/main.ts', 'src/mcp/server.ts'])
tool = reach([f for f in F if f.split('/')[0] in ('scripts', 'benchmarks', 'tools', 'examples')])
test = reach([f for f in F if f.startswith('tests/')])
out = dict(dead=sorted(f for f in src if f not in prod and f not in tool and f not in test),
           test_only=sorted(f for f in src if f not in prod and f not in tool and f in test),
           tool_only=sorted(f for f in src if f not in prod and f in tool))
json.dump(out, open(os.path.join(A, 'ts-reach.json'), 'w'), indent=1)
L = lambda fs: sum(F[f]['loc'] for f in fs)
print(f"src files={len(src)} reachable from app/mcp={len([f for f in src if f in prod])} test-only={len(out['test_only'])} ({L(out['test_only'])} LOC) unreachable={len(out['dead'])} ({L(out['dead'])} LOC)")
