"""Independent sampled homogeneous Coons oracle; no continuous certificate."""
import json,sys,math,hashlib
from pathlib import Path
prepared='--prepared' in sys.argv[2:]
raw=Path(sys.argv[1]).read_bytes();d=json.loads(raw);curves=[c['curve'] for c in d['curves']];s=d['surfaces'][0]['surface']
def basis(k,p,n,t):
 if t==k[n]:return [0.]*(n-1)+[1.]
 b=[float(k[i]<=t<k[i+1]) for i in range(len(k)-1)]
 for degree in range(1,p+1):
  b=[((t-k[i])*b[i]/(k[i+degree]-k[i]) if k[i+degree]!=k[i] else 0.)+((k[i+degree+1]-t)*b[i+1]/(k[i+degree+1]-k[i+1]) if k[i+degree+1]!=k[i+1] else 0.) for i in range(len(b)-1)]
 return b[:n]
def homogeneous(c,t):
 p=c['degree'];n=len(c['weights']);a,b=c['knots'][p],c['knots'][n];bs=basis(c['knots'],p,n,a+(b-a)*t)
 return [sum(bs[i]*c['weights'][i]*(c['controlPoints'][i][j] if j<3 else 1.) for i in range(n)) for j in range(4)]
# Independent constant scaling makes the four homogeneous corners coincide.
factors=[1/curves[0]['weights'][0],0,0,0]
factors[2]=1/curves[2]['weights'][0]
factors[3]=curves[0]['weights'][-1]*factors[0]/curves[3]['weights'][0]
factors[1]=curves[2]['weights'][-1]*factors[2]/curves[1]['weights'][0]
def h(edge,t):
 factor=((1-t)/curves[edge]['weights'][0]+t/curves[edge]['weights'][-1]) if prepared else factors[edge]
 return [x*factor for x in homogeneous(curves[edge],t)]
corners=[h(0,0),h(0,1),h(1,0),h(1,1)]
def surface(u,v):
 nu,nv=len(s['weights']),len(s['weights'][0]);bu=basis(s['knotsU'],s['degreeU'],nu,u);bv=basis(s['knotsV'],s['degreeV'],nv,v)
 h=[sum(bu[i]*bv[j]*s['weights'][i][j]*(s['controlPoints'][i][j][k] if k<3 else 1.) for i in range(nu) for j in range(nv)) for k in range(4)]
 return [x/h[3] for x in h[:3]]
error=0.;minimum=float('inf')
for i in range(101):
 for j in range(101):
  u,v=i/100,j/100;bottom,top,left,right=h(0,u),h(1,u),h(2,v),h(3,v)
  expected=[(1-v)*bottom[k]+v*top[k]+(1-u)*left[k]+u*right[k]-((1-u)*(1-v)*corners[0][k]+u*(1-v)*corners[1][k]+(1-u)*v*corners[2][k]+u*v*corners[3][k]) for k in range(4)]
  assert expected[3]>0;minimum=min(minimum,expected[3]);actual=surface(u,v)
  error=max(error,max(abs(actual[k]-expected[k]/expected[3]) for k in range(3)))
assert error<2e-12,error
print(json.dumps({'preparedBoundaryWeights':prepared,'samples':10201,'maxCoordinateError':error,'minimumSampledWeight':minimum,'continuousCertificate':False,'sha256':hashlib.sha256(raw).hexdigest(),'oracle':'Independent Cox-de Boor and homogeneous Coons formula'},indent=2))
