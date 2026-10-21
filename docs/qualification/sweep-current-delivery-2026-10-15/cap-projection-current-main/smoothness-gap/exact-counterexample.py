from pathlib import Path
from fractions import Fraction as F
import json,hashlib
source=Path('/private/tmp/osv-sweep-cap-projection-artifact-qa-20261015/c2-exact-planar-caps.json')
data=json.loads(source.read_text());cases=[]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def normal(s,uv):
 assert s['degreeU']==2 and s['degreeV']==1
 assert len(s['controlPoints'])==3 and all(len(row)==2 for row in s['controlPoints'])
 du=s['knotsU'][s['degreeU']],s['knotsU'][len(s['controlPoints'])]
 dv=s['knotsV'][s['degreeV']],s['knotsV'][len(s['controlPoints'][0])]
 u=(uv[0]-F(du[0]))/(F(du[1])-F(du[0]));v=(uv[1]-F(dv[0]))/(F(dv[1])-F(dv[0]));assert u in [0,1]
 h=[[[F(p[k])*F(s['weights'][i][j]) for k in range(3)]+[F(s['weights'][i][j])] for j,p in enumerate(row)] for i,row in enumerate(s['controlPoints'])]
 rows=[[row[0][k]*(1-v)+row[1][k]*v for k in range(4)] for row in h]
 endpoint=0 if u==0 else 2;other=1
 p=rows[endpoint]
 pu=[2*(rows[other][k]-p[k])*(1 if endpoint==0 else -1) for k in range(4)]
 pv=[h[endpoint][1][k]-h[endpoint][0][k] for k in range(4)]
 a=[pu[k]*p[3]-p[k]*pu[3] for k in range(3)];b=[pv[k]*p[3]-p[k]*pv[3] for k in range(3)]
 n=cross(a,b);assert any(n);return n
for c in data['artifacts'][1]['cases']:
 model=c['body']['model'];walls=c['body']['retainedSmoothness']['walls'];unresolved=walls['g1Audit']['unresolvedSeams'];witnesses=[]
 for index in unresolved:
  edge=walls['edgeIds'][index];owners=[]
  for fid,face in enumerate(model['faces']):
   for loop in [face['outer'],*face['holes']]:
    for use in model['loops'][loop]['coedges']:
     if use['edge']==edge:owners.append((fid,use))
  assert len(owners)==2
  normals=[]
  for fid,use in owners:
   p=use['pcurve'];assert p['degree']==1 and len(p['controlPoints'])==2 and p['weights'][0]==p['weights'][1]
   uv=[(F(p['controlPoints'][0][k])+F(p['controlPoints'][1][k]))/2 for k in range(2)]
   normals.append(normal(model['faces'][fid]['surface'],uv))
  determinant=cross(*normals)
  witnesses.append({'profileSeam':index,'edge':edge,'faces':[o[0] for o in owners],'exactMidpointNormalsParallel':not any(determinant),'crossNormal':[str(x) for x in determinant]})
 cases.append({'spacing':c['spacing'],'witnesses':witnesses})
result={'schema':'independent-exact-c2-planar-profile-g1-counterexample/1','sourceSha256':hashlib.sha256(source.read_bytes()).hexdigest(),'geometryWasmSha256':data['artifacts'][1]['sha256'],'cases':cases,'scope':'Exact rational normal comparison at the same edge midpoint of actual retained quadratic/linear patches. A nonzero cross-normal disproves geometric G1 even with variable reparametrization. Zero at a sample does not prove a full seam. No rounded evaluations or tolerance used.'}
Path('/private/tmp/osv-c2-planar-profile-g1-counterexample-20261015.json').write_text(json.dumps(result,indent=2)+'\n')
for c in cases:print(c['spacing'],[(w['profileSeam'],w['exactMidpointNormalsParallel']) for w in c['witnesses']])
