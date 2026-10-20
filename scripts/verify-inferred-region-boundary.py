#!/usr/bin/env python3
"""Independent oriented-incidence check of a saved native region boundary."""
import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path

if not __debug__:
    raise RuntimeError('Boundary qualification requires Python assertions')
p = argparse.ArgumentParser()
p.add_argument('root', type=Path)
p.add_argument('--boundary', type=Path)
p.add_argument('--output', type=Path)
a = p.parse_args()
source_path = a.root / 'source.json'
boundary_path = a.boundary or a.root / 'boundary.json'
m = json.loads(source_path.read_text())
r = json.loads(boundary_path.read_text())
selected, retained = set(r['selected']), set(r['retained'])
assert len(selected) == len(r['selected']) and r['seed'] in selected
assert len(retained) == len(r['retained']) and not selected & retained
shell = m['shells'][r['shell']]
assert shell['closed'] and 0 < r['topologyVisits'] <= 100000
incidence = defaultdict(list)
for use in shell['faces']:
    f = m['faces'][use['face']]
    for wire in [f['outer']] + f['holes']:
        for c in m['loops'][wire]['coedges']:
            e = m['edges'][c['edge']]
            if e.get('degenerate', False):
                continue
            ports = e['vertices'][:]
            if c['reversed'] != use['reversed']:
                ports.reverse()
            incidence[c['edge']].append((use['face'], ports))
expected = {}
adjacency = defaultdict(set)
for edge, uses in incidence.items():
    assert len(uses) == 2 and uses[0][0] != uses[1][0]
    assert uses[0][1] == uses[1][1][::-1]
    (fa, pa), (fb, pb) = uses
    if (fa in selected) != (fb in selected):
        inner, outer, ports = (fa, fb, pa) if fa in selected else (fb, fa, pb)
        assert outer in retained
        expected[edge] = (inner, outer, ports)
    if fa not in retained and fb not in retained:
        adjacency[fa].add(fb)
        adjacency[fb].add(fa)
reachable = {r['seed']}
while True:
    grown = reachable | {n for f in reachable for n in adjacency[f]}
    if grown == reachable:
        break
    reachable = grown
assert reachable == selected, 'Selected component differs from original shell connectivity'
actual = {}
vertices = set()
for chain in r['loops']:
    assert chain, 'Empty reported boundary loop'
    for k, arc in enumerate(chain):
        edge = arc['edge']
        assert edge not in actual, 'Repeated original boundary edge'
        assert arc['vertices'][1] == chain[(k + 1) % len(chain)]['vertices'][0], 'Open original boundary ports'
        assert arc['vertices'][0] not in vertices, 'Repeated original boundary vertex'
        vertices.add(arc['vertices'][0])
        actual[edge] = (arc['selectedFace'], arc['retainedFace'], arc['vertices'])
assert actual == expected, 'Reported boundary traversal differs from complete original incidence'
assert {x[1] for x in actual.values()} == retained
report = dict(passed=True, selectedFaces=len(selected), loops=len(r['loops']),
              originalEdges=len(actual), originalVertices=len(vertices),
              completeOrientedBoundary=True, sampledGeometryUsed=False,
              newIntersectionDiscoveryClaim=False,
              sha256={str(path.name): hashlib.sha256(path.read_bytes()).hexdigest()
                      for path in [source_path, boundary_path]})
(a.output or a.root / 'independent-boundary-check.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
