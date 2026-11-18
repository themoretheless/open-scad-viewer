"""Independent exact rational identity bound to every authored loft carrier."""
from pathlib import Path
from fractions import Fraction as Q
from math import comb
import hashlib,json,sys
root=Path(sys.argv[1]);path=root/'body.json';d=json.loads(path.read_text())
def add(a,b):
 out=[Q(0)]*max(len(a),len(b))
 for i,x in enumerate(a):out[i]+=x
 for i,x in enumerate(b):out[i]+=x
 return out
def mul(a,b):
 out=[Q(0)]*(len(a)+len(b)-1)
 for i,x in enumerate(a):
  for j,y in enumerate(b):out[i+j]+=x*y
 return out
def power(a,n):
 out=[Q(1)]
 for _ in range(n):out=mul(out,a)
 return out
def scale(a,x):return [x*c for c in a]
def bernstein(values):
 n=len(values)-1;out=[Q(0)]*(n+1)
 for i,x in enumerate(values):
  for j in range(n-i+1):out[i+j]+=Q(x)*comb(n,i)*comb(n-i,j)*(-1)**j
 return out
def equal(a,b):return all(x==y for x,y in zip(a+[Q(0)]*len(b),b+[Q(0)]*len(a)))
def value(a,x):return sum(c*x**i for i,c in enumerate(a))
def derivative(a):return [i*c for i,c in enumerate(a)][1:]
N=[Q(-2),Q(3)];D=[Q(2),Q(-1)]
profile=[[Q(-10 if i%2==0 else 10),Q(2*i*i)-9*i+Q(15,2)]for i in range(6)]
weights=[Q(2)**(5-i)for i in range(6)]
den=bernstein(weights)
nx=bernstein([p[0]*w for p,w in zip(profile,weights)])
ny=bernstein([p[1]*w for p,w in zip(profile,weights)])
assert equal(den,power(D,5))
assert equal(nx,scale(power(N,5),10))
assert equal(ny,scale(add(mul(power(N,2),power(D,3)),scale(mul(N,power(D,4)),Q(1,4))),10))
critical=Q(2,3)
dx=value(derivative(nx),critical)*value(den,critical)-value(nx,critical)*value(derivative(den),critical)
dy=(value(derivative(ny),critical)*value(den,critical)-value(ny,critical)*value(derivative(den),critical))/value(den,critical)**2
assert dx==0 and dy==Q(45,8)
assert len(d['faces'])==8 and len(d['edges'])==15 and len(d['vertices'])==9
for index,face in enumerate(d['faces']):
 assert not face['holes'];s=face['surface'];p=s['controlPoints']
 if index<6:
  interval,span=divmod(index,3)
  if span==0:
   assert (s['degreeU'],s['degreeV'])==(5,1)
   assert s['knotsU']==[0]*6+[1]*6 and s['knotsV']==[0,0,1,1]
   assert [[list(map(Q,v))for v in row]for row in p]==[[xy+[Q(interval+j)]for j in range(2)]for xy in profile]
   assert list(map(lambda row:list(map(Q,row)),s['weights']))==[[w,w]for w in weights]
  else:
   ends=([profile[-1],[Q(0),Q(10)]]if span==1 else [[Q(0),Q(10)],profile[0]])
   assert [[list(map(Q,v))for v in row]for row in p]==[[xy+[Q(interval+j)]for j in range(2)]for xy in ends]
   assert all(w==1 for row in s['weights']for w in row)
 else:
  assert all(p[i][j]==[s['knotsU'][2*i],s['knotsV'][2*j],2*(index-6)]for i in range(2)for j in range(2))
  wire=d['loops'][face['outer']]['coedges'];assert len(wire)==3
  for c in wire:
   edge=d['edges'][c['edge']]['curve'];xy=[v[:2]for v in edge['controlPoints']]
   if c['reversed']:xy.reverse()
   assert c['pcurve']['controlPoints']==xy
   assert c['pcurve']['degree']==edge['degree'] and c['pcurve']['knots']==edge['knots'] and not c['pcurve']['periodic']
   assert c['pcurve']['weights']==(list(reversed(edge['weights']))if c['reversed']else edge['weights'])
for e in d['edges']:
 c=e['curve'];p=c['controlPoints']
 assert p[0]==d['vertices'][e['vertices'][0]]['point']and p[-1]==d['vertices'][e['vertices'][1]]['point']
 assert c['degree']in [1,5] and not c['periodic']
 assert c['knots']==[0]*(c['degree']+1)+[1]*(c['degree']+1)
 if c['degree']==5:
  assert [list(map(Q,v[:2]))for v in p]==profile and list(map(Q,c['weights']))==weights
 else:
  assert len(p)==2 and len(c['weights'])==2 and c['weights'][0]==c['weights'][1] and c['weights'][0]>0
  assert p[0][:2]==p[1][:2]or (p[0][2]==p[1][2]and Q(p[0][1])==10+Q(p[0][0])/4 and Q(p[1][1])==10+Q(p[1][0])/4)
area=sum(Q(c,n+1)*(1-(-1)**(n+1))for n,c in [(4,500),(6,-500),(9,125),(5,-125)])
assert area==Q(400,7)
result=dict(passed=True,nativeBodySha256=hashlib.sha256(path.read_bytes()).hexdigest(),analyticVolume=str(2*area),exactDenominator='(2-u)^5',positiveDenominatorOnWholeInterval=True,clock='u/(2-u)',inverseClock='2s/(1+s)',nonDyadicCriticalParameter=str(critical),criticalYDerivative=str(dy),allCarriersAndCapTrimsBound=True,finiteSamplingSuppliesProof=False)
(root/'exact-oracle.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
