"""Exact regression data only: an ambient projectivity of a qualified source."""
from pathlib import Path
from fractions import Fraction as Q
from math import lcm
import json
here=Path(__file__).parent
m=json.loads((here.parent/'spatial-poles-2026-10-21/fixtures/nonplanar-rational-pole-tetrahedron.json').read_text())
points=[v['point'] for v in m['vertices']]
for f in m['faces']:points.extend(p for r in f['surface']['controlPoints'] for p in r)
for e in m['edges']:points.extend(e['curve']['controlPoints'])
scale=1
for p in points:scale=lcm(scale,(1+Q(p[1])/8).numerator)
def binary(q):
    x=float(q);assert Q(x)==q,(q,x)
    return x

def transform(p):
    d=1+Q(p[1])/8
    return [binary(scale*Q(x)/d) for x in p],d
for v in m['vertices']:v['point']=transform(v['point'])[0]
for f in m['faces']:
    s=f['surface']
    for i,r in enumerate(s['controlPoints']):
        for j,p in enumerate(r):
            r[j],d=transform(p);s['weights'][i][j]=binary(Q(s['weights'][i][j])*d)
for e in m['edges']:
    c=e['curve']
    for i,p in enumerate(c['controlPoints']):
        c['controlPoints'][i],d=transform(p);c['weights'][i]=binary(Q(c['weights'][i])*d)
(here/'fixtures/nonpolynomial-rational-graph-tetrahedron.json').write_text(json.dumps(m,indent=2)+'\n')
print('exact projective fixture scale',scale)
