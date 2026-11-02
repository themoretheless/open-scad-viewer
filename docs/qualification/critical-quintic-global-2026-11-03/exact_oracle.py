"""Independent exact polynomial oracle bound to the freshly exported native body."""
from pathlib import Path
from fractions import Fraction as Q
from math import comb
import hashlib,json
root=Path(__file__).resolve().parent
path=root/'native-body/body.json'
d=json.loads(path.read_text());s=d['faces'][0]['surface'];p=s['controlPoints']
assert (s['degreeU'],s['degreeV'])==(5,1)
assert s['knotsU']==[0]*6+[1]*6 and s['knotsV']==[0,0,1,1]
assert all(Q(w)==1 for row in s['weights'] for w in row)
assert all(row[0][:2]==row[1][:2] and row[0][2]==0 and row[1][2]==2 for row in p)
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
def scale(a,s):return [x*s for x in a]
def bernstein(values):
 n=len(values)-1;out=[Q(0)]*(n+1)
 for i,x in enumerate(values):
  for j in range(n-i+1):out[i+j]+=Q(x)*comb(n,i)*comb(n-i,j)*(-1)**j
 return out
def equal(a,b):return all(x==y for x,y in zip(a+[Q(0)]*len(b),b+[Q(0)]*len(a)))
q=[Q(-1),Q(2)]
x=bernstein([row[0][0] for row in p]);y=bernstein([row[0][1] for row in p])
assert equal(x,scale(power(q,5),10))
assert equal(y,add(scale(power(q,2),10),scale(q,Q(5,2))))
# The fifth power is strictly increasing even though its derivative has one
# isolated zero. The independent Y derivative equals 5 at u=1/2, so the
# original extrusion has rank two everywhere. These are whole polynomials.
dx=[i*v for i,v in enumerate(x)][1:];dy=[i*v for i,v in enumerate(y)][1:]
value=lambda poly,u:sum(c*u**i for i,c in enumerate(poly))
assert value(dx,Q(1,2))==0 and value(dy,Q(1,2))==5
# Original support-plane numerator: all interior coefficients strictly below
# the planar chord, both complete end rows zero. No point sampling involved.
phi=[Q(row[0][1])-10-Q(row[0][0])/4 for row in p]
assert phi[0]==phi[-1]==0 and all(c<0 for c in phi[1:-1])
# Exact Green integral in q: 500q^4-500q^6+125q^9-125q^5.
area=sum(Q(c,n+1)*(1-(-1)**(n+1)) for n,c in [(4,500),(6,-500),(9,125),(5,-125)])
assert area==Q(400,7)
assert (len(d['faces']),len(d['edges']),len(d['vertices']),len(d['shells']))==(4,6,4,1)
assert d['shells'][0]['closed'] is True
result={'passed':True,'nativeBodySha256':hashlib.sha256(path.read_bytes()).hexdigest(),
 'analyticVolume':str(2*area),'strictCoordinate':'10(2u-1)^5',
 'criticalYDerivative':'5','supportPlaneCoefficients':[str(c) for c in phi],
 'sourcePolynomialIdentities':True,'fullIntervalArgument':True,'finiteSamplingSuppliesProof':False}
print(json.dumps(result,indent=2))
(root/'exact-oracle.json').write_text(json.dumps(result,indent=2)+'\n')
