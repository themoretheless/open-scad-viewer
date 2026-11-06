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
qpoly=[Q(-1),Q(2)]
weights=list(map(Q,[1,2,2,2,2,1]))
profile=[[Q(-10 if i%2==0 else 10)/w,(Q(2*i*i)-9*i+Q(15,2))/w]for i,w in enumerate(weights)]
den=bernstein(weights)
nx=bernstein([p[0]*w for p,w in zip(profile,weights)])
ny=bernstein([p[1]*w for p,w in zip(profile,weights)])
assert equal(den,add([Q(2)],scale(add(power([Q(1),Q(-1)],5),power([Q(0),Q(1)],5)),-1)))
assert equal(nx,scale(power(qpoly,5),10))
assert equal(ny,scale(add(power(qpoly,2),scale(qpoly,Q(1,4))),10))
critical=Q(1,2)
dx=value(derivative(nx),critical)*value(den,critical)-value(nx,critical)*value(derivative(den),critical)
dy=(value(derivative(ny),critical)*value(den,critical)-value(ny,critical)*value(derivative(den),critical))/value(den,critical)**2
assert dx==0 and dy==Q(80,31)
# The original positive Bernstein denominator is >=1 on the whole interval.
# This quotient has a genuine degree-four denominator, not a geometric clock.
assert den[-1]==0 and den[-2]!=0 and value(den,critical)==Q(31,16)
assert weights[2]*weights[0]!=weights[1]*weights[1]
both=d['faces'][0]['surface']['degreeV']==2
station_weights=list(map(Q,[1,2,1]if both else [1,1]));nv=len(station_weights)
assert len(d['faces'])==8 and len(d['edges'])==15 and len(d['vertices'])==9
for index,face in enumerate(d['faces']):
 assert not face['holes'];s=face['surface'];p=s['controlPoints']
 if index<6:
  interval,span=divmod(index,3)
  if span==0:
   assert (s['degreeU'],s['degreeV'])==(5,nv-1)
   assert s['knotsU']==[0]*6+[1]*6 and s['knotsV']==[0]*nv+[1]*nv
   assert [[list(map(Q,v))for v in row]for row in p]==[[xy+[Q(interval)+Q(j,nv-1)]for j in range(nv)]for xy in profile]
   assert list(map(lambda row:list(map(Q,row)),s['weights']))==[[w*v for v in station_weights]for w in weights]
  else:
   ends=([profile[-1],[Q(0),Q(10)]]if span==1 else [[Q(0),Q(10)],profile[0]])
   assert [[list(map(Q,v))for v in row]for row in p]==[[xy+[Q(interval)+Q(j,nv-1)]for j in range(nv)]for xy in ends]
   assert list(map(lambda row:list(map(Q,row)),s['weights']))==[station_weights,station_weights]
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
 assert c['degree']in [1,2,5] and not c['periodic']
 assert c['knots']==[0]*(c['degree']+1)+[1]*(c['degree']+1)
 if c['degree']==5:
  assert [list(map(Q,v[:2]))for v in p]==profile and list(map(Q,c['weights']))==weights
 elif c['degree']==2:
  assert both and p[0][:2]==p[1][:2]==p[2][:2] and Q(p[1][2])==(Q(p[0][2])+Q(p[2][2]))/2
  assert c['weights'][0]==c['weights'][2]>0 and c['weights'][1]==2*c['weights'][0]
 else:
  assert len(p)==2 and len(c['weights'])==2 and c['weights'][0]==c['weights'][1] and c['weights'][0]>0
  assert p[0][:2]==p[1][:2]or (p[0][2]==p[1][2]and Q(p[0][1])==10+Q(p[0][0])/4 and Q(p[1][1])==10+Q(p[1][0])/4)
# Exact independent Green integral enclosure. W(q)=(31/16)(1-a(q)),
# a(q)=(10q^2+5q^4)/31 in [0,15/31]. Expand (1-a)^-3 with a
# fixed 129-term rational sum, and bound the entire nonnegative tail.
W=[Q(31,16),Q(0),Q(-10,16),Q(0),Q(-5,16)]
assert equal(den,add([Q(31,16)],add(scale(power(qpoly,2),Q(-10,16)),scale(power(qpoly,4),Q(-5,16)))))
q=[Q(0),Q(1)]
P=scale(mul(power(q,4),mul(add(W,scale(power(q,2),-1)),add(scale(W,5),scale(mul(q,derivative(W)),-1)))),100)
assert equal(add(W,scale(power(q,2),-1)),scale(mul([Q(1),Q(0),Q(-1)],[Q(31),Q(0),Q(5)]),Q(1,16)))
assert equal(add(scale(W,5),scale(mul(q,derivative(W)),-1)),[Q(155,16),Q(0),Q(-30,16),Q(0),Q(-5,16)])
assert Q(155)-30-5>0
a=[Q(0),Q(0),Q(10,31),Q(0),Q(5,31)]
apow=[Q(1)];lower=Q(0);factor=Q(16,31)**3
for k in range(129):
 term=mul(P,apow)
 integral=sum(2*c/Q(i+1)for i,c in enumerate(term)if i%2==0)
 lower+=2*factor*comb(k+2,2)*integral
 apow=mul(apow,a)
r=Q(15,31);M=129
remainder=r**M/2*((M*M+3*M+2)/(1-r)+(2*M+3)*r/(1-r)**2+r*(1+r)/(1-r)**3)
# P>=0: W-q^2=(1-q^2)(31+5q^2)/16 and
# 5W-qW'=(155-30q^2-5q^4)/16 >= 120/16.
# Whole-interval P <= 100*(31/16)*(155/16); length two, volume factor two.
width=4*Q(100*31*155,16*16)*factor*remainder
assert width>0 and width<Q(1,10**30)
upper=lower+width
from decimal import Decimal,localcontext
with localcontext()as ctx:
 ctx.prec=90
 dec=lambda x:str(Decimal(x.numerator)/Decimal(x.denominator))
 text=dec((lower+upper)/2)
result=dict(passed=True,nativeBodySha256=hashlib.sha256(path.read_bytes()).hexdigest(),highPrecisionVolume=text,volumeLowerFraction=str(lower),volumeUpperFraction=str(upper),exactVolumeIntervalWidth=str(width),exactGreenIntegralEnclosed=True,seriesTerms=129,exactDenominator='2-(1-u)^5-u^5',positiveDenominatorOnWholeInterval=True,weightSequenceNotGeometric=True,nonMobiusStationClock=both,criticalParameter=str(critical),criticalYDerivative=str(dy),allCarriersAndCapTrimsBound=True,finiteSamplingSuppliesProof=False)
(root/'exact-oracle.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items()if k not in ['volumeLowerFraction','volumeUpperFraction','exactVolumeIntervalWidth']},indent=2))
