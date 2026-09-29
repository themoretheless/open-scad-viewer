"""Independent exact Fraction check of shared-boundary certificates.
No Rust predicates, geometry evaluator or tessellation are used.
"""
import json
import sys
from fractions import Fraction as F
from itertools import combinations

def sub(a,b): return [x-y for x,y in zip(a,b)]
def cross(a,b): return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def bezier(c):
    n=c['degree']+1
    return len(c['controlPoints'])==n and c['knots']==[c['knots'][0]]*n+[c['knots'][-1]]*n and c['knots'][0]<c['knots'][-1]

def boundary(model,face,edge):
    f=model['faces'][face];s=f['surface'];e=model['edges'][edge]['curve']
    assert bezier(e)
    points=s['controlPoints'];weights=s['weights'];sizes=[len(points),len(points[0])]
    for loop in [f['outer']]+f['holes']:
        for use in model['loops'][loop]['coedges']:
            if use['edge']!=edge: continue
            p=use['pcurve'];assert p['degree']==1 and bezier(p)
            for axis in range(2):
                free=1-axis;ks=[s['knotsU'],s['knotsV']];ds=[s['degreeU'],s['degreeV']]
                assert all(k==[k[0]]*(d+1)+[k[-1]]*(d+1) for k,d in zip(ks,ds))
                for row,fixed in [(0,ks[axis][0]),(sizes[axis]-1,ks[axis][-1])]:
                    pc=p['controlPoints']
                    if not all(q[axis]==fixed for q in pc):continue
                    if sorted(q[free] for q in pc)!=[ks[free][0],ks[free][-1]]:continue
                    if ds[free]!=e['degree']:continue
                    indices=[(row,i) if axis==0 else (i,row) for i in range(sizes[free])]
                    cp=[points[u][v] for u,v in indices];w=[weights[u][v] for u,v in indices]
                    if (cp==e['controlPoints'] and w==e['weights']) or (cp==e['controlPoints'][::-1] and w==e['weights'][::-1]):return axis,row
    raise AssertionError('edge is not the authored full natural boundary')

cases=json.load(open(sys.argv[1]))['cases'];assert len(cases)==24
count=0
for c in cases:
    m=c['model'];certs=c['certificates'];assert len(certs)==12
    assert len({tuple(c['faces']) for c in certs})==12
    for cert in certs:
        a,b=cert['faces'];plane=cert['planarFace'];side=cert['sidedFace']
        assert a<b and {plane,side}=={a,b}
        for i in [a,b]:
            s=m['faces'][i]['surface'];assert not s['periodicU'] and not s['periodicV']
            assert all(F(w)>0 for row in s['weights'] for w in row)
            boundary(m,i,cert['edge'])
        s=m['faces'][plane]['surface'];points=[list(map(F,p)) for row in s['controlPoints'] for p in row]
        for x,y,z in combinations(points,3):
            n=cross(sub(y,x),sub(z,x))
            if any(n):break
        else:raise AssertionError('degenerate plane')
        assert all(dot(n,sub(p,x))==0 for p in points)
        axis,row=boundary(m,side,cert['edge']);signs=set()
        for u,ps in enumerate(m['faces'][side]['surface']['controlPoints']):
            for v,p in enumerate(ps):
                d=dot(n,sub(list(map(F,p)),x))
                if (u,v)[axis]==row:assert d==0
                else:assert d!=0;signs.add(d>0)
        assert len(signs)==1
        count+=1
print(json.dumps({'models':len(cases),'exactSharedBoundaryCertificates':count}))
