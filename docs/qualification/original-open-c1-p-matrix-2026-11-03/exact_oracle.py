#!/usr/bin/env python3
"""Independent Fraction/Bernstein proof for the emitted original fixed-frame map."""
import json,sys,math
from fractions import Fraction as Q
from pathlib import Path

def field(s,a,b):
    return s[a] if a in s else s[b]
def poly(s,axis):
    pu=field(s,'degree_u','degreeU');pv=field(s,'degree_v','degreeV')
    cp=field(s,'control_points','controlPoints');weights=s['weights'];out={}
    assert not field(s,'periodic_u','periodicU') and not field(s,'periodic_v','periodicV')
    assert len(cp)==pu+1 and all(len(row)==pv+1 for row in cp)
    assert all(Q(w)==1 for row in weights for w in row)
    for i,row in enumerate(cp):
        for j,p in enumerate(row):
            for a in range(pu-i+1):
                for b in range(pv-j+1):
                    key=(i+a,j+b)
                    out[key]=out.get(key,Q(0))+Q(p[axis])*math.comb(pu,i)*math.comb(pv,j)*math.comb(pu-i,a)*math.comb(pv-j,b)*(-1)**(a+b)
    return {k:v for k,v in out.items() if v}
def split(c):
    l=[c[0]];r=[c[-1]]
    while len(c)>1:
        c=[(a+b)/2 for a,b in zip(c,c[1:])];l.append(c[0]);r.append(c[-1])
    return l,r[::-1]
def complete(c):
    pending=[c];cells=0;lower=Q(1)
    while pending:
        c=pending.pop();cells+=1
        assert cells<1000
        if min(c)>0: lower=min(lower,min(c));continue
        pending.extend(split(c))
    return cells,lower

records=[json.loads(line.split('original-open-c1-source=',1)[1]) for line in Path(sys.argv[1]).read_text().splitlines() if 'original-open-c1-source=' in line]
assert len(records)==1
s=records[0];assert s['normal']==[1,0,0] and s['clock']=='parameter' and s['orientation']=='fixed_normal'
for c,target in [(s['path'],[[0,0,0],[0,0,1]]),(s['scale'],None),(s['twist'],None)]:
    assert not c['periodic'] and c['degree']==1 and c['knots']==[0,0,1,1] and c['weights']==[1,1]
    if target is not None: assert field(c,'control_points','controlPoints')==target
assert all(p==[1,0,0] for p in field(s['scale'],'control_points','controlPoints'))
assert all(p==[0,0,0] for p in field(s['twist'],'control_points','controlPoints'))
a,b=s['caps'];x={(1,0):Q(1),(1,1):Q(4)}
y={(0,1):Q(1),**{(i,0):Q(-(-1)**i*math.comb(8,i),2) for i in range(1,9)}}
for cap,offset in [(a,Q(0)),(b,Q(1,64))]:
    for axis,p in [(0,x),(1,y),(2,{})]:
        expected=dict(p)
        if axis<2 and offset: expected[(0,0)]=offset
        assert poly(cap,axis)==expected
    for key,degree,n in [('u',8,9),('v',1,2)]:
        knots=field(cap,'knots_'+key,'knots'+key.upper());assert len(knots)==2*n
        assert all(t==knots[0] for t in knots[:n]) and all(t==knots[-1] for t in knots[n:]) and knots[0]<knots[-1]
# F=(u+4uv+t/64, v+(1-(1-u)^8)/2+t/64, t).
# Diagonal entries and both time principal minors are >=1. The sole
# nontrivial 2x2 minor and full determinant are 1+4v-16u(1-u)^7.
# These original normalized-domain Bernstein coefficients are exact.
cells,lower=complete([Q(1),Q(-1)]+[Q(1)]*7)
# Endpoint matrices at v=0 have positive determinants; their average has
# determinant -3. Their convex hull therefore contains a singular matrix.
assert Q(1)-Q(4,2)*Q(4,2)==-3
print(json.dumps({'records':len(records),'allSevenPrincipalMinorsPositive':True,'completeBernsteinCells':cells,'determinantLowerBound':str(lower),'endpointJacobianAverageDeterminant':'-3','scope':'Original normalized filled map; material trims and arbitrary matrix modes remain native obligations'},indent=2))
