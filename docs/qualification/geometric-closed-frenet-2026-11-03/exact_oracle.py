#!/usr/bin/env python3
"""Independent formal-series unit-tangent jets, including irrational speeds."""
from fractions import Fraction as Q
from math import comb,factorial,isqrt
from pathlib import Path
import json,sys,copy,math,runpy
N=3

def trim(p):
    while len(p)>1 and p[-1]==0:p.pop()
    return p

def add(a,b):return trim([(a[i] if i<len(a) else Q(0))+(b[i] if i<len(b) else Q(0)) for i in range(max(len(a),len(b)))])
def mul(a,b):
    p=[Q(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):p[i+j]+=x*y
    return trim(p)
def scale(a,s):return trim([x*s for x in a])
def power(a,n):
    p=[Q(1)]
    for _ in range(n):p=mul(p,a)
    return p
def derivative(a):return trim([i*a[i] for i in range(1,len(a))] or [Q(0)])
def bernstein(a):
    n=len(a)-1;p=[Q(0)]*(n+1)
    for i,x in enumerate(a):
        for j in range(n-i+1):p[i+j]+=x*comb(n,i)*comb(n-i,j)*(-1)**j
    return trim(p)
def homogeneous(c):
    n=c['degree'];assert len(c['controlPoints'])==len(c['weights'])==n+1 and all(Q(w)>0 for w in c['weights'])
    h=[[Q(p[k])*Q(w) for p,w in zip(c['controlPoints'],c['weights'])] for k in range(3)]+[list(map(Q,c['weights']))]
    return [bernstein(a) for a in h]
def composed(p,num,den,degree):
    out=[Q(0)]
    for k,x in enumerate(p):out=add(out,scale(mul(power(num,k),power(den,degree-k)),x))
    return out
def curvature(c):
    h=homogeneous(c);w=h[3];dw=derivative(w)
    a=[add(mul(derivative(p),w),scale(mul(p,dw),-1)) for p in h[:3]]
    p=add(mul(a[0],derivative(a[1])),scale(mul(a[1],derivative(a[0])),-1));n=len(p)-1
    b=[sum(p[k]*Q(comb(i,k),comb(n,k)) for k in range(i+1)) for i in range(n+1)]
    pending=[b];visits=0;lower=None
    while pending:
        b=pending.pop();visits+=1;assert visits<4096
        if min(b)>0:lower=min(b) if lower is None else min(lower,min(b));continue
        left=[b[0]];right=[b[-1]]
        while len(b)>1:b=[(x+y)/2 for x,y in zip(b,b[1:])];left.append(b[0]);right.append(b[-1])
        pending.extend([left,right[::-1]])
    return {'cells':visits,'numeratorLower':str(lower)}
def pieces(c):
    n=c['degree'];count=len(c['controlPoints'])
    if count==n+1:return [c]
    assert count==2*n+1 and c['knots']==[-3]*12+[1]*11+[5]*12
    return [{**c,'controlPoints':c['controlPoints'][i:i+n+1],'weights':c['weights'][i:i+n+1],
        'knots':[a]*(n+1)+[b]*(n+1)} for i,a,b in [(0,-3,1),(n,1,5)]]
def cartesian_series(c,end):
    n=c['degree'];width=Q(c['knots'][-1])-Q(c['knots'][0]);assert width>0
    h=[[Q(v)*Q(w) for v in p]+[Q(w)] for p,w in zip(c['controlPoints'],c['weights'])]
    coefficients=[]
    for order in range(5):
        v=h[-1] if end else h[0];coefficients.append([x*comb(n,order)/width**order for x in v])
        h=[[b-a for a,b in zip(x,y)] for x,y in zip(h,h[1:])]
    result=[]
    for k,hk in enumerate(coefficients):
        result.append([(hk[a]-sum(coefficients[j][3]*result[k-j][a] for j in range(1,k+1)))/coefficients[0][3] for a in range(3)])
    return result

def field(d):
    a,b=isqrt(d.numerator),isqrt(d.denominator);root=Q(a,b) if a*a==d.numerator and b*b==d.denominator else None
    class E:
        def __init__(self,a=0,b=0):
            self.a=Q(a);self.b=Q(b)
            if root is not None:self.a+=self.b*root;self.b=Q(0)
        def coerce(self,x):return x if isinstance(x,E) else E(x)
        def __add__(self,x):x=self.coerce(x);return E(self.a+x.a,self.b+x.b)
        __radd__=__add__
        def __neg__(self):return E(-self.a,-self.b)
        def __sub__(self,x):return self+-self.coerce(x)
        def __rsub__(self,x):return self.coerce(x)+-self
        def __mul__(self,x):x=self.coerce(x);return E(self.a*x.a+self.b*x.b*d,self.a*x.b+self.b*x.a)
        __rmul__=__mul__
        def __truediv__(self,x):
            x=self.coerce(x);norm=x.a*x.a-x.b*x.b*d;assert norm
            return self*E(x.a/norm,-x.b/norm)
        def __eq__(self,x):x=self.coerce(x);return self.a==x.a and self.b==x.b
        def __repr__(self):return str(self.a) if self.b==0 else f'({self.a})+({self.b})*sqrt({d})'
    return E

def smul(a,b,E):return [sum((a[i]*b[k-i] for i in range(k+1)),E()) for k in range(N+1)]
def compose(a,b,E):
    out=[E()]*(N+1);p=[E(1)]+[E()]*N
    for x in a:
        out=[v+x*y for v,y in zip(out,p)];p=smul(p,b,E)
    return out

def tangent_series(pos,E,speed0):
    v=[[(k+1)*pos[k+1][a] for k in range(N+1)] for a in range(3)]
    q=[sum(sum(v[a][i]*v[a][k-i] for i in range(k+1)) for a in range(3)) for k in range(N+1)]
    s=[speed0];assert speed0*speed0==q[0]
    for k in range(1,N+1):s.append((E(q[k])-sum((s[i]*s[k-i] for i in range(1,k)),E()))/(2*s[0]))
    t=[]
    for axis in range(3):
        row=[]
        for k in range(N+1):row.append((E(v[axis][k])-sum((s[j]*row[k-j] for j in range(1,k+1)),E()))/s[0])
        t.append(row)
    # Integrate the speed series and revert it coefficient by coefficient.
    # This is not the native rational chart-derivative formula.
    arc=[E()]+[s[k-1]/k for k in range(1,N+1)];inverse=[E()]*(N+1)
    for k in range(1,N+1):inverse[k]=(E(int(k==1))-compose(arc,inverse,E)[k])/arc[1]
    assert compose(arc,inverse,E)==[E(),E(1),E(),E()]
    raw=[[x*factorial(k) for k,x in enumerate(row)] for row in t]
    geometric=[[x*factorial(k) for k,x in enumerate(compose(row,inverse,E))] for row in t]
    return raw,geometric

def compare(left,right):
    vl=left[1];vr=right[1];axis=next(i for i,x in enumerate(vl) if x)
    ratio=vr[axis]/vl[axis];assert ratio>0 and all(y==ratio*x for x,y in zip(vl,vr))
    d=sum(x*x for x in vl);E=field(d)
    raw_l,geo_l=tangent_series(left,E,E(0,1));raw_r,geo_r=tangent_series(right,E,E(0,ratio))
    return raw_l,raw_r,geo_l,geo_r

def records(path,prefix):return [json.loads(x.split(prefix,1)[1]) for x in Path(path).read_text().splitlines() if prefix in x]
r=records(sys.argv[1],'geometric-frenet-source=');assert {x['sourceClock'] for x in r}=={'mobius','split-mobius'}
results=[]
for x in r:
    assert x['clock']=='arc-length' and x['orientation']=='frenet';c=x['path'];reference=x['reference'];ps=pieces(c)
    targets=[([Q(0),Q(2)],[Q(1),Q(1)])] if len(ps)==1 else [([Q(0),Q(1,2)],[Q(1)]),([Q(1,2),Q(3,2)],[Q(1),Q(1)])]
    href=homogeneous(reference)
    for piece,(num,den) in zip(ps,targets):
        assert homogeneous(piece)==[composed(h,num,den,reference['degree']) for h in href]
    pairs=[(cartesian_series(ps[0],False),cartesian_series(ps[-1],True))]
    if len(ps)>1:pairs.append((cartesian_series(ps[0],True),cartesian_series(ps[1],False)))
    for l,rr in pairs:
        assert l[0]==rr[0] and l[1]!=rr[1]
        a,b,g,h=compare(l,rr);assert g==h and a!=b
    bad=copy.deepcopy(c);bad['controlPoints'][7][2]=math.nextafter(bad['controlPoints'][7][2],math.inf);bp=pieces(bad)
    l,rr=(cartesian_series(bp[0],False),cartesian_series(bp[0],True)) if len(bp)==1 else (cartesian_series(bp[0],True),cartesian_series(bp[1],False))
    _,_,g,h=compare(l,rr);assert all(a[:3]==b[:3] for a,b in zip(g,h)) and g!=h
    results.append({'sourceClock':x['sourceClock'],'exactMobiusIdentity':True,'joinsWithEqualThirdArcTangentJets':len(pairs),
        'rawTangentJetsDiffer':True,'oneUlpThirdJetMismatch':True,'referenceWholeCurvature':curvature(reference)})
material=[]
if len(sys.argv)>2:
    oracle=runpy.run_path(str(Path(__file__).parents[1]/'closed-contact-material-2026-11-03/exact_witness.py'))
    for m in records(sys.argv[2],'geometric-frenet-material-source='):
        assert any(m['path']==x['path'] for x in r)
        assert oracle['verify_binding'](m)==8
        oracle['canonical_ring'](m['trims'][0],[Q(1),Q(4)],Q(2),False)
        oracle['canonical_ring'](m['trims'][1],[Q(1),Q(4)],Q(1,2),True)
        material.append({'originalKnots':len(m['path']['knots']),'exactLiftBindings':8,'nestedRings':2})
    assert material
print(json.dumps({'formalSeriesGeometricJets':results,'materialBindings':material,
    'scope':'Exact image-preserving source clocks and third physical-arc tangent jets; positive full body/material embedding is native atlas evidence.'},indent=2))
