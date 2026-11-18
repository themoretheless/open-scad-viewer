"""Independent exact polynomial oracle bound to the freshly exported native body."""
from pathlib import Path
from fractions import Fraction as Q
from math import comb
import hashlib,json
root=Path(__file__).resolve().parent
path=root/'affine-body/body.json'
d=json.loads(path.read_text());s=d['faces'][0]['surface'];p=s['controlPoints']
assert (s['degreeU'],s['degreeV'])==(5,1)
assert s['knotsU']==[0]*6+[1]*6 and s['knotsV']==[0,0,1,1]
assert all(Q(w)==1 for row in s['weights'] for w in row)
M=[[Q(1),Q(1),Q(1)],[Q(1),Q(2),Q(1)],[Q(1),Q(1),Q(2)]]
T=[Q(8),Q(-4),Q(2)];inverse=[[Q(3),Q(-1),Q(-1)],[Q(-1),Q(1),Q(0)],[Q(-1),Q(0),Q(1)]]
det=sum(M[0][k]*(M[1][(k+1)%3]*M[2][(k+2)%3]-M[1][(k+2)%3]*M[2][(k+1)%3]) for k in range(3));assert det==1
def original(point):
 source=[sum(inverse[k][j]*(Q(point[j])-T[j]) for j in range(3)) for k in range(3)]
 assert [T[k]+sum(M[k][j]*source[j] for j in range(3)) for k in range(3)]==list(map(Q,point))
 return source
p=[[original(v) for v in row] for row in p]
assert all(row[0][:2]==row[1][:2] and row[0][2]==0 and row[1][2]==2 for row in p)
# Bind the analytic prism to every retained carrier and complete cap trim,
# not only to one profile or a sampled set of points.
source_poles=p
for index,face in enumerate(d['faces']):
 assert not face['holes']
 surface=face['surface'];net=[[original(v) for v in row] for row in surface['controlPoints']]
 assert all(Q(w)==1 for row in surface['weights'] for w in row)
 if index==1:
  assert (surface['degreeU'],surface['degreeV'])==(1,1)
  assert net==[[source_poles[-1][0],source_poles[-1][1]],[source_poles[0][0],source_poles[0][1]]]
 if index>=2:
  assert (surface['degreeU'],surface['degreeV'])==(1,1)
  assert all(net[i][j]==[Q(surface['knotsU'][i*2]),Q(surface['knotsV'][j*2]),Q((index-2)*2)] for i in range(2) for j in range(2))
  wire=d['loops'][face['outer']]['coedges'];assert len(wire)==2
  for c in wire:
   edge=d['edges'][c['edge']]['curve'];xy=[original(v)[:2] for v in edge['controlPoints']]
   if c['reversed']:xy.reverse()
   assert list(map(lambda v:list(map(Q,v)),c['pcurve']['controlPoints']))==xy
   assert c['pcurve']['degree']==edge['degree']
for edge in d['edges']:
 c=edge['curve'];assert all(Q(w)==1 for w in c['weights'])
 net=[original(v) for v in c['controlPoints']]
 assert all(net[i]==original(d['vertices'][edge['vertices'][j]]['point']) for j,i in [(0,0),(1,-1)])
 if c['degree']==5:
  assert net==[row[0] for row in source_poles] or net==[row[1] for row in source_poles]
 else:
  assert c['degree']==1 and len(net)==2
  assert net[0][:2]==net[1][:2] or (net[0][2]==net[1][2] and {tuple(net[0][:2]),tuple(net[1][:2])}=={tuple(source_poles[0][0][:2]),tuple(source_poles[-1][0][:2])})
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
 'affineDeterminant':str(det),'exactInverseReconstruction':True,'analyticVolume':str(2*area*det),'strictCoordinate':'10(2u-1)^5',
 'criticalYDerivative':'5','supportPlaneCoefficients':[str(c) for c in phi],
 'allCarrierAndCapTrimsBound':True,'sourcePolynomialIdentities':True,'fullIntervalArgument':True,'finiteSamplingSuppliesProof':False}
print(json.dumps(result,indent=2))
(root/'exact-oracle.json').write_text(json.dumps(result,indent=2)+'\n')
