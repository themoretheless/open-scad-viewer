"""Independent Fraction Cox-de Boor polynomial oracle, no CAD imports."""
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
source_knots=[2]*4+[F(7,2)]+[5]*4
source_points=[[0,0,2],[1,2,3],[3,4,5],[5,2,7],[6,0,8]]
result_points=[[0,0,2],[3,6,5],[6,0,8]]
expected=[[-4,2],[-F(40,3),F(28,3),-F(4,3)],[-2,2]]
for mid in [3,4]:
 for k in range(3):
  a=axis(source_knots,3,[p[k] for p in source_points],mid)
  b=axis([2]*3+[5]*3,2,[p[k] for p in result_points],mid)
  assert padded(a)==padded(b)==padded(expected[k])
perturbed=[p[:] for p in source_points];perturbed[2][1]=F(4)+F(1,2**50)
residual=add(axis(source_knots,3,[p[1] for p in perturbed],3),[-x for x in expected[1]])
assert any(residual)
print(json.dumps({'exactPiecewiseSourceAndGlobalCandidate':True,'spans':2,'axes':3,'independentPolynomialLaws':[[str(x) for x in p] for p in expected],'oneUlpPerturbationNonzeroResidual':[str(x) for x in residual]},indent=2))
