from pathlib import Path
from fractions import Fraction as F
import json,math
out=Path(__file__).resolve().parent;m=json.loads((out/'v6-provider-geometry-probes/rotated-intersection.json').read_text());bad=[]
for fi,f in enumerate(m['faces']):
 cs=m['loops'][f['outer']]['coedges'];ids=[m['edges'][c['edge']]['vertices'][1 if c['reversed'] else 0] for c in cs];p=[c['pcurve']['controlPoints'][0] for c in cs];pts=[m['vertices'][i]['point'] for i in ids];ring=list(range(len(p)));orient=lambda a,b,c:(F(p[b][0])-F(p[a][0]))*(F(p[c][1])-F(p[a][1]))-(F(p[b][1])-F(p[a][1]))*(F(p[c][0])-F(p[a][0]));area=sum(p[i][0]*p[(i+1)%len(p)][1]-p[(i+1)%len(p)][0]*p[i][1] for i in range(len(p)))/2
 if area<0:ring.reverse()
 cursor=0;tris=[]
 while len(ring)>3:
  selected=None
  for off in range(len(ring)):
   i=(cursor+off)%len(ring);a,b,c=ring[i-1],ring[i],ring[(i+1)%len(ring)]
   if orient(a,b,c)<=0:continue
   lo=[min(p[t][j] for t in [a,b,c]) for j in range(2)];hi=[max(p[t][j] for t in [a,b,c]) for j in range(2)]
   if any(p[v] not in [p[a],p[b],p[c]] and all(lo[j]<=p[v][j]<=hi[j] for j in range(2)) and orient(a,b,v)>=0 and orient(b,c,v)>=0 and orient(c,a,v)>=0 for v in ring):continue
   selected=i,[a,b,c];break
  assert selected is not None,fi
  i,t=selected;tris.append(t);ring.pop(i);cursor=i%len(ring)
 assert orient(*ring)>0,fi
 tris.append(ring)
 for t in tris:
  a,b,c=[pts[i] for i in t];u=[b[i]-a[i] for i in range(3)];v=[c[i]-a[i] for i in range(3)];n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
  if math.hypot(*n)<=2**-52*max(1,math.hypot(*u)*math.hypot(*v)):bad.append({'face':fi,'localTriangle':t,'faceTriangles':tris,'vertices':ids,'points':pts,'uv':p,'exactUvArea':str(orient(*t))})
print(json.dumps(bad,indent=2));(out/'v7-display-ear-replay.json').write_text(json.dumps({'scope':'Independent diagnostic replay of simple display-ear ordering using rational binary64 orientation, NOT native runtime proof','bad':bad},indent=2)+'\n')
