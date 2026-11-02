from fractions import Fraction as Q
import json
source=[[4.,0.],[5.,4.],[4.,8.]]
result=[[4.,0.],[4.623874036455391,2.4954961458215634],[4.334003755927576,5.858517992989387]]
sw=[1.,.7,1.];rw=[1.,.7890327060493011,.8747800728778061]
columns=[[Q(x)*Q(w),Q(y)*Q(w),Q(w)] for (x,y),w in zip(source,sw)]
a=[[columns[j][i] for j in range(3)]+[Q(int(i==j)) for j in range(3)] for i in range(3)]
for i in range(3):
 pivot=a[i][i];a[i]=[x/pivot for x in a[i]]
 for j in range(3):
  if j!=i:
   k=a[j][i];a[j]=[x-k*y for x,y in zip(a[j],a[i])]
inv=[r[3:] for r in a]
def implicit(h):
 b=[sum(x*y for x,y in zip(row,h)) for row in inv]
 return b[1]*b[1]-4*b[0]*b[2]
def point(t):
 b=[(1-t)**2,2*t*(1-t),t*t]
 return [sum(bi*Q(w)*Q(p[k]) for bi,w,p in zip(b,rw,result)) for k in range(2)]+[sum(bi*Q(w) for bi,w in zip(b,rw))]
def sign(q):return (q>0)-(q<0)
for t in [Q(0),Q(1,4),Q(1,2),Q(3,4),Q(1)]:
 b=[(1-t)**2,2*t*(1-t),t*t]
 h=[sum(bi*c[k] for bi,c in zip(b,columns)) for k in range(3)]
 assert implicit(h)==0
assert implicit([Q(4),Q(4),Q(1)])<0<implicit([Q(6),Q(4),Q(1)])
r={'method':'Exact binary64-to-rational conic residual; diagnostic only, not complete region census','interior_reference_sign':sign(implicit([Q(4),Q(4),Q(1)])),'exterior_reference_sign':sign(implicit([Q(6),Q(4),Q(1)])),'result_samples':[]}
for t in [Q(1,4),Q(1,2),Q(3,4)]:
 v=implicit(point(t));r['result_samples'].append({'t':str(t),'residual_sign':sign(v),'numerator':str(v.numerator),'denominator':str(v.denominator)})

# Exact degree-four Bernstein coefficients of the residual on the ENTIRE
# retained segment. A nonnegative Bernstein hull with a strictly positive
# coefficient proves strict positivity throughout the open parameter interval.
from math import comb
hc=[[Q(x)*Q(w),Q(y)*Q(w),Q(w)] for (x,y),w in zip(result,rw)]
bc=[[sum(x*y for x,y in zip(row,h)) for row in inv] for h in hc]
coeff=[]
for k in range(5):
 v=Q(0)
 for i in range(3):
  j=k-i
  if 0<=j<3:
   v+=Q(comb(2,i)*comb(2,j),comb(4,k))*(bc[i][1]*bc[j][1]-4*bc[i][0]*bc[j][2])
 coeff.append(v)
r['whole_interval_bernstein_signs']=[sign(v) for v in coeff]
r['whole_open_interval_exterior_proved']=all(v>=0 for v in coeff) and any(v>0 for v in coeff)
print(json.dumps(r,indent=2))
