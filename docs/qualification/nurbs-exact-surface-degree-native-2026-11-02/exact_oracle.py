"""Independent exact bivariate Bernstein coefficient comparison, no CAD imports."""
from fractions import Fraction as F
from math import comb
import json

def power(values):
    n=len(values)-1
    out=[F(0)]*(n+1)
    for i,a in enumerate(values):
        for j in range(n-i+1):out[i+j]+=F(a)*comb(n,i)*comb(n-i,j)*(-1)**j
    return out

def tensor(net):
    nu,nv=len(net),len(net[0])
    columns=[power([net[i][j] for i in range(nu)]) for j in range(nv)]
    return [power([columns[j][i] for j in range(nv)]) for i in range(nu)]

def padded_equal(a,b):
    for i in range(max(len(a),len(b))):
        for j in range(max(len(a[0]),len(b[0]))):
            av=a[i][j] if i<len(a) and j<len(a[0]) else 0
            bv=b[i][j] if i<len(b) and j<len(b[0]) else 0
            assert av==bv

x=[0,2,4,6];z=[0,4,4,0];q=[0,3,6];r=[0,6,0]
for axis in range(3):
    a=[[(x[i] if axis==0 else x[j] if axis==1 else z[i]+z[j]) for j in range(4)] for i in range(4)]
    b=[[(q[i] if axis==0 else q[j] if axis==1 else r[i]+r[j]) for j in range(3)] for i in range(3)]
    padded_equal(tensor(a),tensor(b))
assert power([1,3,4,4])==power([1,4,4])+[F(0)]
assert power([0,24,28,12])==power([0,36,12])+[F(0)]
source_weight=sum(F(v)*b for v,b in zip([1,2,1],[F(1,4),F(1,2),F(1,4)]))
source_y=F(1)/(source_weight+1)
assert source_y==F(2,5) and source_y!=F(1,2)
print(json.dumps({'tensorPolynomialExact':True,'rationalNumeratorAndDenominatorExact':True,'adversarialSourceY':str(source_y),'naiveReducedY':'1/2'},indent=2))
