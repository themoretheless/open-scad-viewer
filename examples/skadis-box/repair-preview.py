import struct,math,json
from pathlib import Path
d=Path(__file__).parent
polys=[]
for name in ['box','hook']:
 p=d/('dovetail-'+name+'.stl');b=p.read_bytes()
 tris=[[tuple(round(v,5) for v in struct.unpack_from('<fff',b,84+50*t+12+12*i)) for i in range(3)] for t in range(struct.unpack_from('<I',b,80)[0])]
 points=list(set(v for tri in tris for v in tri));fixed=[]
 for tri in tris:
  ring=[]
  for a,c in zip(tri,tri[1:]+tri[:1]):
   delta=[c[k]-a[k] for k in range(3)];ll=sum(v*v for v in delta);hits=[]
   for v in points:
    t=sum((v[k]-a[k])*delta[k] for k in range(3))/ll
    if -1e-8<=t<1-1e-8 and sum((v[k]-a[k]-t*delta[k])**2 for k in range(3))<1e-10:hits.append((t,v))
   ring.extend(v for _,v in sorted(hits))
  center=tuple(sum(v[k] for v in tri)/3 for k in range(3))
  fixed.extend((center,a,c) for a,c in zip(ring,ring[1:]+ring[:1]))
 out=bytearray(80)+struct.pack('<I',len(fixed))
 for tri in fixed:out+=struct.pack('<12fH',0,0,0,*sum((list(v) for v in tri),[]),0)
 p.write_bytes(out)
 pts=[];ids={};faces=[]
 for tri in fixed:
  f=[]
  for v in tri:
   if v not in ids:ids[v]=len(pts);pts.append(v)
   f.append(ids[v])
  faces.append(f[::-1])
 poly='polyhedron(points='+json.dumps(pts)+',faces='+json.dumps(faces)+');'
 if name=='box':polys.append('color([0.18,0.55,0.64]) '+poly)
 else:
  for x in [20,100]:polys.append('color([0.95,0.65,0.22]) translate(['+str(x)+',0,42]) rotate([0,90,0]) translate([-10,0,-2.2]) '+poly)
 print(name,len(fixed),'triangles repaired')
(d/'dovetail-preview.scad').write_text('\n'.join(polys))
