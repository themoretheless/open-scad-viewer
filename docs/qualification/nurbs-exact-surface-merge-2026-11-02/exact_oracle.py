"""Independent exact bivariate Cox-de Boor identity; no CAD imports."""
from fractions import Fraction as F
from functools import lru_cache
import json

def add(a,b):
 out=dict(a)
 for k,v in b.items():out[k]=out.get(k,F(0))+v
 return {k:v for k,v in out.items() if v}
def mul(a,b):
 out={}
 for (i,j),x in a.items():
  for (k,l),y in b.items():out[i+k,j+l]=out.get((i+k,j+l),F(0))+x*y
 return {k:v for k,v in out.items() if v}
def scale(a,s):return {k:v*s for k,v in a.items() if v*s}
def basis(knots,degree,mid,axis):
 knots=list(map(F,knots));unit=(1,0) if axis==0 else (0,1)
 @lru_cache(None)
 def at(i,p):
  if not p:return {(0,0):F(1)} if knots[i]<=mid<knots[i+1] else {}
  out={};d=knots[i+p]-knots[i]
  if d:out=add(out,mul({(0,0):-knots[i]/d,unit:1/d},at(i,p-1)))
  d=knots[i+p+1]-knots[i+1]
  if d:out=add(out,mul({(0,0):knots[i+p+1]/d,unit:-1/d},at(i+1,p-1)))
  return out
 return [at(i,degree) for i in range(len(knots)-degree-1)]
def tensor(knots_u,points,weights,mid):
 u=basis(knots_u,1,F(mid),0);v=basis([0,0,1,1],1,F(1,2),1);out=[{} for _ in range(4)]
 for i,row in enumerate(points):
  for j,p in enumerate(row):
   b=scale(mul(u[i],v[j]),F(weights[i][j]))
   for k,x in enumerate([*p,1]):out[k]=add(out[k],scale(b,F(x)))
 return out
def residual(a,b):return [add(mul(a[k],b[3]),scale(mul(b[k],a[3]),-1)) for k in range(3)]
points=[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]],[[3,0,0],[3,1,0]]];weights=[[1,3],[2,6],[4,12]];knots=[-2,-2,2,8,8]
for source_knots,indices,mid in [([-2,-2,2,2],[0,1],0),([2,2,8,8],[1,2],5)]:
 source=tensor(source_knots,[points[i] for i in indices],[weights[i] for i in indices],mid)
 merged=tensor(knots,points,weights,mid)
 assert not any(residual(source,merged))
source=tensor([2,2,8,8],points[1:],[[4,12],[8,24]],5)
wrong=tensor(knots,points,[[1,3],[2,6],[8,24]],5)
r=residual(source,wrong);assert any(r)
print(json.dumps({'completeOriginalRectanglesEqual':2,'cartesianAxes':3,'denominatorCrossProductsExact':True,'inconsistentSeamGaugeNonzeroResidual':[[[list(k),str(v)] for k,v in row.items()] for row in r]},indent=2))
