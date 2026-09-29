"""Independent exact checks for separable rational line/surface fixtures.
Two disjoint sign changes of a quadratic prove that all U roots are enclosed.
Strictly increasing Bernstein Y controls prove a unique V root.
"""
import json,hashlib,math,sys
from fractions import Fraction as F
from pathlib import Path
source=Path(sys.argv[1]).read_bytes();cases=json.loads(source)['cases'];roots=0
for case in cases:
    s=case['surface'];p,q=s['degreeU'],s['degreeV'];origin=list(map(F,case['origin']));direction=list(map(F,case['direction']))
    assert p==2 and direction[2]==1 and direction[1]==0
    weights=[F(row[0]) for row in s['weights']]
    assert all(all(F(w)==weights[i] for w in row) for i,row in enumerate(s['weights']))
    x=[(F(s['controlPoints'][i][0][0])-origin[0])*weights[i]-(F(s['controlPoints'][i][0][2])-origin[2])*weights[i]*direction[0] for i in range(3)]
    y=[F(s['controlPoints'][0][j][1])-origin[1] for j in range(q+1)]
    assert all(y[j]<y[j+1] for j in range(q))
    assert all(all(F(row[j][1])-origin[1]==y[j] for j in range(q+1)) for row in s['controlPoints'])
    z=[F(s['controlPoints'][i][0][2])*weights[i] for i in range(3)]
    assert all(all(F(point[2])*weights[i]==z[i] for point in row) for i,row in enumerate(s['controlPoints']))
    def bernstein(coeff,t):return sum(F(math.comb(len(coeff)-1,i))*t**i*(1-t)**(len(coeff)-1-i)*v for i,v in enumerate(coeff))
    def normalize(bounds,knots,degree):
        a,b=F(knots[degree]),F(knots[-degree-1]);return [(F(t)-a)/(b-a) for t in bounds]
    assert case['complete'] and len(case['roots'])==2
    intervals=[]
    for root in case['roots']:
        u=normalize(root['uv'][0],s['knotsU'],p);v=normalize(root['uv'][1],s['knotsV'],q)
        assert 0<u[0]<=u[1]<1 and 0<v[0]<=v[1]<1
        assert bernstein(x,u[0])*bernstein(x,u[1])<0
        derivative=[2*(x[i+1]-x[i]) for i in range(2)]
        assert bernstein(derivative,u[0])*bernstein(derivative,u[1])>0
        assert bernstein(y,v[0])<0<bernstein(y,v[1])
        lo,hi=map(F,root['parameter'])
        for t in u:
            parameter=bernstein(z,t)/bernstein(weights,t)-origin[2]
            assert lo<=parameter<=hi
        intervals.append(u);roots+=1
    intervals.sort();assert intervals[0][1]<intervals[1][0]
print(json.dumps({'sourceSha256':hashlib.sha256(source).hexdigest(),'cases':len(cases),'roots':roots,'arithmetic':'exact rational Bernstein signs and degree bounds','maxCells':max(c['cells'] for c in cases),'checks':'passed'},indent=2))
