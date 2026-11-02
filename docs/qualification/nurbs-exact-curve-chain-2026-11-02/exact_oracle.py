"""Independent Fraction Cox-de Boor rational curve identities; no CAD imports."""
from fractions import Fraction as F
from functools import lru_cache
import json

def add(a,b):
 out=[F(0)]*max(len(a),len(b))
 for i,x in enumerate(a):out[i]+=x
 for i,x in enumerate(b):out[i]+=x
 return out

def mul(a,b):
 out=[F(0)]*(len(a)+len(b)-1)
 for i,x in enumerate(a):
  for j,y in enumerate(b):out[i+j]+=x*y
 return out

def axis(knots,degree,points,mid):
 knots=list(map(F,knots));mid=F(mid)
 @lru_cache(None)
 def basis(i,p):
  if not p:return [F(int(knots[i]<=mid<knots[i+1]))]
  out=[F(0)]
  d=knots[i+p]-knots[i]
  if d:out=add(out,mul([-knots[i]/d,1/d],basis(i,p-1)))
  d=knots[i+p+1]-knots[i+1]
  if d:out=add(out,mul([knots[i+p+1]/d,-1/d],basis(i+1,p-1)))
  return out
 out=[F(0)]
 for i,x in enumerate(points):out=add(out,[F(x)*v for v in basis(i,degree)])
 return out

def padded(v,n=4):return v+[F(0)]*(n-len(v))

def law(knots,degree,points,weights,mid):
 return [axis(knots,degree,[F(p[k])*F(w) for p,w in zip(points,weights)],mid) for k in range(len(points[0]))]+[axis(knots,degree,weights,mid)]
def residual(a,b):return [add(mul(a[k],b[-1]),[-x for x in mul(b[k],a[-1])]) for k in range(len(a)-1)]

points=[[0,0],[F(3,2),3],[3,3],[F(15,4),3],[F(9,2),F(9,4)],[F(21,4),F(3,2)],[6,0]]
knots=[2]*3+[F(7,2)]*2+[F(17,4)]*2+[5]*3
for mid in [3,4,F(9,2)]:
 a=law(knots,2,points,[1]*7,mid);b=law([2]*3+[5]*3,2,[[0,0],[3,6],[6,0]],[1]*3,mid)
 assert not any(any(row) for row in residual(a,b))
print(json.dumps({'completePolynomialIntervalsEqual':3,'axes':2,'allCoefficientResidualsZero':True},indent=2))
