"""Exact rational test-data construction, not a production geometry path."""
from fractions import Fraction as F
from math import comb
from pathlib import Path
import json
here=Path(__file__).parent
m=json.loads((here.parent/'holed-atlas-2026-10-21/fixtures/collapsed-boundary-tetrahedron.json').read_text())
old=[tuple(v['point']) for v in m['vertices']]
new=[tuple(int(x) for x in p) for p in old]
lookup=dict(zip(old,new))
def binary(q):
    q=F(q);v=float(q)
    assert F(v)==q, q
    return v

def product(a,b):
    n,p=len(a)-1,len(a[0])-1;r,s=len(b)-1,len(b[0])-1
    c=[[F(0) for _ in range(p+s+1)] for _ in range(n+r+1)]
    for i in range(n+1):
        for j in range(p+1):
            for k in range(r+1):
                for l in range(s+1):
                    c[i+k][j+l]+=a[i][j]*b[k][l]*F(comb(n,i)*comb(r,k),comb(n+r,i+k))*F(comb(p,j)*comb(s,l),comb(p+s,j+l))
    return c
for f in m['faces']:
    s=f['surface'];h=[[lookup[tuple(p)] for p in row] for row in s['controlPoints']]
    c=[product([[F(p[k]) for p in row] for row in h],[[F(1)]*2 for _ in range(2)]) for k in range(3)]
    yy=product([[F(p[1]) for p in row] for row in h],[[F(p[1]) for p in row] for row in h])
    s.update(degreeU=2,degreeV=2,knotsU=[0,0,0,1,1,1],knotsV=[0,0,0,1,1,1],weights=[[1]*3 for _ in range(3)],controlPoints=[[[binary(c[k][i][j]+(yy[i][j]/64 if k==2 else 0)) for k in range(3)] for j in range(3)] for i in range(3)])
for e in m['edges']:
    a,b=[new[i] for i in e['vertices']]
    h=[a,tuple(F(a[k]+b[k],2) for k in range(3)),b]
    points=[]
    for i,p in enumerate(h):
        z=F(p[2])+([F(a[1]**2),F(a[1]*b[1]),F(b[1]**2)][i])/64
        points.append([binary(p[0]),binary(p[1]),binary(z)])
    e['curve'].update(degree=2,knots=[0,0,0,1,1,1],controlPoints=points,weights=[1]*3)
for v,p in zip(m['vertices'],new):v['point']=[p[0],p[1],binary(F(p[2])+F(p[1]**2,64))]
# Non-monomial rational angular clock on the curved first source face.
s=m['faces'][0]['surface'];p=s['controlPoints'];s['controlPoints']=[p[0],p[0],p[2]];s['weights']=[[w]*3 for w in [2,2,1]]
outer=m['faces'][0]['outer'];edge=m['loops'][outer]['coedges'][0]['edge']
e=m['edges'][edge]['curve'];a,b=e['controlPoints'][0],e['controlPoints'][-1];e.update(controlPoints=[a,a,b],weights=[2,2,1])
for li,loop in enumerate(m['loops']):
    if li==outer:continue
    for ce in loop['coedges']:
        if ce['edge']!=edge:continue
        p=ce['pcurve'];a,b=p['controlPoints'][0],p['controlPoints'][-1]
        p.update(degree=2,knots=[0,0,0,1,1,1],controlPoints=[a,b,b] if ce['reversed'] else [a,a,b],weights=[1,2,2] if ce['reversed'] else [2,2,1])
(here/'fixtures/nonplanar-rational-pole-tetrahedron.json').write_text(json.dumps(m,indent=2)+'\n')
