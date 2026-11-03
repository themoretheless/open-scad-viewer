#!/usr/bin/env python3
"""Independent exact source-jet/curvature/alias and material-binding oracle."""
from fractions import Fraction as F
from math import comb,factorial
from pathlib import Path
import json,sys,runpy,copy,math,hashlib

def clean(p):
    while len(p)>1 and p[-1]==0:p.pop()
    return p

def add(a,b):
    return clean([(a[i] if i<len(a) else F(0))+(b[i] if i<len(b) else F(0)) for i in range(max(len(a),len(b)))])
def scale(a,s):return clean([v*s for v in a])
def mul(a,b):
    p=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):p[i+j]+=x*y
    return clean(p)
def derivative(p):return clean([i*p[i] for i in range(1,len(p))] or [F(0)])
def evaluate(p,x):
    v=F(0)
    for a in reversed(p):v=v*x+a
    return v
def bernstein_power(c):
    n=len(c)-1;p=[F(0)]*(n+1)
    for i,x in enumerate(c):
        for j in range(n-i+1):p[i+j]+=x*comb(n,i)*comb(n-i,j)*(-1)**j
    return clean(p)
def homogeneous(c):
    p=c['controlPoints'];w=list(map(F,c['weights']));assert len(p)==len(w)==c['degree']+1 and all(x>0 for x in w)
    return [bernstein_power([F(v[k])*weight for v,weight in zip(p,w)]) for k in range(3)]+[bernstein_power(w)]
def jets(h,x,order,width):
    hd=[]
    for p in h:
        row=[]
        for k in range(order+1):row.append(evaluate(p,x)/width**k);p=derivative(p)
        hd.append(row)
    result=[]
    for k in range(order+1):
        result.append([(hd[a][k]-sum(F(comb(k,j))*hd[3][j]*result[k-j][a] for j in range(1,k+1)))/hd[3][0] for a in range(3)])
    return result
def endpoint_jets(c,order):
    p=c['degree'];k=list(map(F,c['knots']));assert not c['periodic'] and k[:p+1]==[k[p]]*(p+1) and k[p+1:]==[k[-1]]*(p+1)
    h=homogeneous(c);width=k[-1]-k[0];assert width>0
    return jets(h,F(0),order,width),jets(h,F(1),order,width)
def split(c):
    left=[c[0]];right=[c[-1]]
    while len(c)>1:
        c=[(a+b)/2 for a,b in zip(c,c[1:])];left.append(c[0]);right.append(c[-1])
    return left,right[::-1]
def positive_polynomial(p):
    n=len(p)-1;c=[sum(p[k]*F(comb(i,k),comb(n,k)) for k in range(i+1)) for i in range(n+1)]
    pending=[c];visits=0;lower=None
    while pending:
        c=pending.pop();visits+=1;assert visits<4096,'No complete positive Bernstein cover'
        if min(c)>0:lower=min(c) if lower is None else min(lower,min(c))
        else:pending.extend(split(c))
    return {'cells':visits,'numeratorLower':str(lower),'degree':n}
def curvature_cover(c):
    h=homogeneous(c);w=h[3];dw=derivative(w)
    a=[add(mul(derivative(p),w),scale(mul(p,dw),-1)) for p in h[:3]]
    cross_z=add(mul(a[0],derivative(a[1])),scale(mul(a[1],derivative(a[0])),-1))
    # (P' x P'')_z = (A x A')_z/W^4 in normalized source units.
    return positive_polynomial(cross_z)
def records(path,prefix):
    return [json.loads(line.split(prefix,1)[1]) for line in Path(path).read_text().splitlines() if prefix in line]

r={x['label']:x for x in records(sys.argv[1],'closed-frenet-source=')}
assert set(r)=={'positive-c4','c3-only','twist-c1-only','zero-curvature','double-cover'}
assert all(x['orientation']=='frenet' and x['clock']=='parameter' for x in r.values())
for key in ['positive-c4','zero-curvature','twist-c1-only']:
    a,b=endpoint_jets(r[key]['path'],4);assert a==b
p=r['positive-c4']['path'];positive=curvature_cover(p)
a,b=endpoint_jets(r['c3-only']['path'],4);assert a[:4]==b[:4] and a[4]!=b[4]
a,b=endpoint_jets(r['twist-c1-only']['twist'],2);assert a[:2]==b[:2] and a[2]!=b[2]
z=r['zero-curvature']['path'];j=jets(homogeneous(z),F(1,2),2,F(z['knots'][-1])-F(z['knots'][0]));assert any(j[1]) and j[2]==[0,0,0]
d=r['double-cover'];c=d['path'];n=c['degree'];assert n==11
assert c['knots']==[0]*12+[1]*11+[2]*12 and len(c['controlPoints'])==23
assert c['controlPoints'][:n+1]==c['controlPoints'][n:] and c['weights'][:n+1]==c['weights'][n:]
for law in ['scale','twist']:
    target=[1,0,0] if law=='scale' else [0,0,0]
    assert all(q==target for q in d[law]['controlPoints'])
first={**c,'knots':[0]*12+[1]*12,'controlPoints':c['controlPoints'][:12],'weights':c['weights'][:12]}
a,b=endpoint_jets(first,4);assert a==b and curvature_cover(first)
# The entire source path and Frenet frame repeat on distinct normalized
# stations 0 and 1/2, with identical scale/twist. Hence EVERY source cap point
# has the same ideal world image at those two distinct S1 parameters.
result={'exactC4EndpointIdentity':True,'positiveWholeCurvature':positive,'c3DoesNotSupplyC4':True,
    'c1TwistDoesNotSupplyC2':True,'zeroInteriorAccelerationWithRegularTangent':True,
    'actualDoubleCoverCollision':{'normalizedStations':['0','1/2'],'sourcePointScope':'every cap parameter'},
    'scope':'Exact source jets, whole curvature direction and negative alias; positive global volume embedding remains native atlas evidence.'}
if len(sys.argv)>2:
    material=records(sys.argv[2],'closed-frenet-material-source=');assert len(material)==1;m=material[0];assert m['path']==p
    oracle=runpy.run_path(str(Path(__file__).parents[1]/'closed-contact-material-2026-11-03/exact_witness.py'))
    assert oracle['verify_binding'](m)==8
    oracle['canonical_ring'](m['trims'][0],[F(1),F(4)],F(2),False)
    oracle['canonical_ring'](m['trims'][1],[F(1),F(4)],F(1,2),True)
    for name in ['scale','twist','axes','center']:
        a,b=endpoint_jets(m[name],2);assert a==b
    bad=copy.deepcopy(m['trims'][1][0]);bad['controlPoints'][1][0]=math.nextafter(bad['controlPoints'][1][0],math.inf)
    assert not oracle['lift_equal'](m['profiles'][4],bad,m['surface'])
    result['exactMaterialLiftBindings']=8;result['canonicalNestedMaterialRings']=2;result['periodicC2MotionLaws']=True
    result['oneUlpBindingMismatchRejected']=True
result['sourcePayloadSha256']=hashlib.sha256(json.dumps(r,sort_keys=True).encode()).hexdigest()
print(json.dumps(result,indent=2))
