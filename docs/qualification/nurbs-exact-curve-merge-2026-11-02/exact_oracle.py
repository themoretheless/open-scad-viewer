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
points=[[0,2,4],[1,4,6],[3,8,10]];weights=[1,2,4];knots=[-2,-2,2,8,8]
for sk,ids,mid in [([-2,-2,2,2],[0,1],0),([2,2,8,8],[1,2],5)]:
 assert not any(any(row) for row in residual(law(sk,1,[points[i] for i in ids],[weights[i] for i in ids],mid),law(knots,1,points,weights,mid)))
poly=[[0,0],[F(3,2),3],[3,3],[F(9,2),3],[6,0]]
for mid in [3,4]:assert not any(any(row) for row in residual(law([2]*3+[F(7,2)]*2+[5]*3,2,poly,[1]*5,mid),law([2]*3+[5]*3,2,[[0,0],[3,6],[6,0]],[1]*3,mid)))
wrong=law(knots,1,points,[1,2,8],5);right=law([2,2,8,8],1,points[1:],[4,8],5)
r=residual(right,wrong);assert any(any(row) for row in r)
print(json.dumps({'originalRationalIntervalsEqual':2,'rationalAxes':3,'polynomialSpansEqual':2,'incorrectGaugeResidualNonzero':[[str(x) for x in row] for row in r]},indent=2))
