"""Independent Bernstein-to-power rational identities, no CAD imports."""
from fractions import Fraction as F
from math import comb
import json

def power(values):
    n=len(values)-1
    out=[F(0)]*(n+1)
    for i,a in enumerate(values):
        for j in range(n-i+1):out[i+j]+=a*comb(n,i)*comb(n-i,j)*(-1)**j
    return out

def product(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):out[i+j]+=x*y
    return out

def identity(points,weights,other,other_weights):
    w=power(list(map(F,weights)));v=power(list(map(F,other_weights)));residual=[]
    for axis in range(len(points[0])):
        a=power([F(p[axis])*weight for p,weight in zip(points,weights)])
        b=power([F(p[axis])*weight for p,weight in zip(other,other_weights)])
        l,r=product(a,v),product(b,w)
        residual.append([x-y for x,y in zip(l,r)])
    return residual

polynomial=identity([[0,0,2],[2,4,4],[4,4,6],[6,0,8]],[1]*4,[[0,0,2],[3,6,5],[6,0,8]],[1]*3)
rational=identity([[0,0],[8,0],[7,0],[3,0]],[1,3,4,4],[[0,0],[9,0],[3,0]],[1,4,4])
constant=identity([[5,7]]*3,[1,2,1],[[5,7]]*2,[1,1])
assert all(x==0 for rows in [polynomial,rational,constant] for row in rows for x in row)
perturbed=identity([[1,0],[1,0],[1+F(1,2**52),0]],[1]*3,[[1,0],[1+F(1,2**52),0]],[1]*2)
assert any(x!=0 for row in perturbed for x in row)
print(json.dumps({'polynomialExact':True,'rationalClockExact':True,'constantGaugeExact':True,'nonzeroQuadraticResidual':[str(x) for x in perturbed[0]]},indent=2))
