"""Exact endpoint tangent-plane witnesses; absence of a witness proves nothing."""
import json,sys
from fractions import Fraction as F
from pathlib import Path

def sub(a,b):return [F(x)-F(y) for x,y in zip(a,b)]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
manifest=json.loads(Path(sys.argv[1]).read_text());reports=[]
for case in manifest['cases']:
 smooth=case['nativeProfileSmoothness']
 if smooth['profileG1Certified']:continue
 walls={face:surface for face,surface in zip([i for i in range(case['faces']) if i not in case['capFaces']],case['wallSurfaces'])}
 witnesses=[]
 for edge in smooth['edgeIds']:
  curve=case['edgeCurves'][edge]['curve'];ends=[curve['controlPoints'][0],curve['controlPoints'][-1]];normals=[]
  for face,surface in walls.items():
   loops=case['faceLoops'][face]
   if edge not in loops['outer'] and not any(edge in hole for hole in loops['holes']):continue
   p=surface['controlPoints']
   if any(len(set(surface[key][:degree+1]))!=1 or len(set(surface[key][-degree-1:]))!=1 for key,degree in [('knotsU',surface['degreeU']),('knotsV',surface['degreeV'])]):continue
   for row,neighbor in [(0,1),(-1,-2)]:
    if [p[row][0],p[row][-1]] not in [ends,list(reversed(ends))]:continue
    # Endpoint rational derivatives are positive scalar multiples of adjacent
    # pole differences for clamped positive-weight bases; parallelism is exact.
    if not all(w>0 for line in surface['weights'] for w in line):continue
    normal=cross(sub(p[neighbor][0],p[row][0]),sub(p[row][1],p[row][0]))
    normals.append((face,normal,p[row][0]))
  if len(normals)==2 and normals[0][2]==normals[1][2] and all(any(n) for _,n,_ in normals):
   residual=cross(normals[0][1],normals[1][1])
   if any(residual):witnesses.append({'edge':edge,'faces':[f for f,_,_ in normals],'crossOfTangentPlaneNormals':[str(x) for x in residual]})
 reports.append({'file':case['file'],'endpointNonparallelWitnesses':witnesses,'scope':'Exact authored binary64 pole differences at clamped seam endpoints; no positive smoothness certificate'})
Path(sys.argv[2]).write_text(json.dumps(reports,indent=2)+'\n')
print([(r['file'],len(r['endpointNonparallelWitnesses'])) for r in reports])
