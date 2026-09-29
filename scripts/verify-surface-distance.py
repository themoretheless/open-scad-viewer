"""Independent exact-rational tensor-basis evaluation of native distance enclosures.

The checker uses Cox basis recursion with exact fractions, not native de Boor/blossom
restriction. Binary64 input values are converted exactly. Sampled enclosure
checks supplement the interval construction; they are not an exhaustive proof.
"""
from fractions import Fraction as D
from decimal import Decimal, getcontext
from pathlib import Path
import json, sys, hashlib
getcontext().prec=90
source=Path(sys.argv[1])
data=json.loads(source.read_text(),parse_float=lambda x:D.from_float(float(x)))
def dec(x):return x if isinstance(x,D) else D(x)
def basis(knots,p,n,t):
    knots=list(map(dec,knots));t=dec(t)
    b=[D(int(knots[i]<=t<knots[i+1])) for i in range(len(knots)-1)]
    if t==knots[n]:
        b=[D(0)]*len(b);span=n-1
        while knots[span]==t:span-=1
        b[span]=D(1)
    for order in range(1,p+1):
        result=[]
        for i in range(len(b)-1):
            left=knots[i+order]-knots[i];right=knots[i+order+1]-knots[i+1]
            result.append(((t-knots[i])*b[i]/left if left else D(0))+((knots[i+order+1]-t)*b[i+1]/right if right else D(0)))
        b=result
    return b[:n]
def evaluate(s,uv):
    nu=len(s['controlPoints']);nv=len(s['controlPoints'][0])
    u=basis(s['knotsU'],s['degreeU'],nu,uv[0]);v=basis(s['knotsV'],s['degreeV'],nv,uv[1])
    h=[D(0)]*3;w=D(0)
    for i in range(nu):
        for j in range(nv):
            f=u[i]*v[j]*dec(s['weights'][i][j]);w+=f
            for k in range(3):h[k]+=f*dec(s['controlPoints'][i][j][k])
    return [x/w for x in h]
checked=0
for case in data['rectangles']:
    domain=[[dec(x) for x in axis] for axis in case['domain']]
    for i in range(11):
        for j in range(11):
            uv=[domain[k][0]+(domain[k][1]-domain[k][0])*D([i,j][k])/10 for k in range(2)]
            p=evaluate(case['surface'],uv)
            assert all(dec(case['bounds'][k][0])<=p[k]<=dec(case['bounds'][k][1]) for k in range(3)),(case,uv,p)
            checked+=1
case=data['distance'];r=case['result']
for index,key in enumerate(['a','b']):
    p=evaluate(case[key],r['parameters'][index])
    assert all(dec(r['pointEnclosures'][index][k][0])<=p[k]<=dec(r['pointEnclosures'][index][k][1]) for k in range(3)), (index,p,r['pointEnclosures'][index])
exact=Decimal(450).sqrt()-10
to_decimal=lambda x:Decimal(x.numerator)/Decimal(x.denominator)
assert to_decimal(r['distanceIntervalMm'][0])<=exact<=to_decimal(r['distanceIntervalMm'][1])
assert r['converged'] and r['distanceIntervalMm'][1]-r['distanceIntervalMm'][0]<=r['toleranceMm']
print(json.dumps({'sourceSha256':hashlib.sha256(source.read_bytes()).hexdigest(),'arithmetic':'exact rational basis evaluation','rectangles':len(data['rectangles']),'sampledPoints':checked,'witnesses':2,'analyticDistanceMm':str(exact),'cells':r['cells'],'independentEnclosureChecks':'passed'},indent=2))
