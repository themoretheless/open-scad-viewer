"""Exact endpoint tangent-plane witnesses for retained station seams.
Nonparallel regular endpoint normals disprove G1. No witness is not a proof.
"""
from fractions import Fraction as F
from pathlib import Path
import json,sys

def vec(a,b):return [F(x)-F(y) for x,y in zip(a,b)]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def normal(s,side):
 p=s['controlPoints'];u=s['degreeU'];v=s['degreeV'];ku=s['knotsU'];kv=s['knotsV']
 if len(p)<2 or len(p[0])<2 or not all(w>0 for row in s['weights'] for w in row):return None
 if ku[:u+1]!=[ku[u]]*(u+1) or not ku[u+1]>ku[u]:return None
 if kv[:v+1]!=[kv[v]]*(v+1) or kv[-v-1:]!=[kv[len(p[0])]]*(v+1):return None
 j=0 if side=='vMin' else len(p[0])-1
 adjacent=1 if j==0 else j-1
 point=p[0][j]
 n=cross(vec(p[1][j],point),vec(p[0][adjacent],point))
 return (point,n) if any(n) else None

def boundary(s,c):
 if c['degree']!=s['degreeU'] or c['knots']!=s['knotsU']:return None
 for side,j in [('vMin',0),('vMax',len(s['controlPoints'][0])-1)]:
  points=[row[j] for row in s['controlPoints']];weights=[row[j] for row in s['weights']]
  if (c['controlPoints']==points and c['weights']==weights) or (c['controlPoints']==list(reversed(points)) and c['weights']==list(reversed(weights))):return side
 return None

manifest=json.loads(Path(sys.argv[1]).read_text());cases=[]
for case in manifest['cases']:
 caps=set(case['capFaces']);uses={}
 surfaces=dict(zip((i for i in range(case['faces']) if i not in caps),case['wallSurfaces']))
 for i,surface in surfaces.items():
  face=case['faceLoops'][i]
  for loop in [face['outer'],*face['holes']]:
   for edge in loop:
    b=boundary(surface,case['edgeCurves'][edge]['curve'])
    if b:uses.setdefault(edge,[]).append((i,b))
 witnesses=[];checked=0
 for edge,pair in uses.items():
  if len(pair)!=2:continue
  a,b=pair;na=normal(surfaces[a[0]],a[1]);nb=normal(surfaces[b[0]],b[1])
  if not na or not nb or na[0]!=nb[0]:continue
  checked+=1
  product=cross(na[1],nb[1])
  if any(product):witnesses.append({'edge':edge,'faces':[a[0],b[0]],'point':na[0],'crossNormals':[str(x) for x in product],'conclusion':'not-G1-at-regular-endpoint'})
 cases.append({'file':case['file'],'checkedEndpoints':checked,'nonparallelWitnesses':witnesses})
report={'method':'exact-binary64-fraction-endpoint-tangent-planes','scope':'station-seam endpoints; absence of a witness does not certify G1/G2','cases':cases}
Path(sys.argv[2]).write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({c['file']:len(c['nonparallelWitnesses']) for c in cases if c['nonparallelWitnesses']},indent=2))
