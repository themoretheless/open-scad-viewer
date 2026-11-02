"""Independent exact bivariate rational identity over every tensor knot cell."""
import json,sys,hashlib,copy
from fractions import Fraction as F
from pathlib import Path

def add(a,b):
 c=[F(0)]*max(len(a),len(b))
 for i,x in enumerate(a):c[i]+=x
 for i,x in enumerate(b):c[i]+=x
 return c

def mul(a,b):
 c=[F(0)]*(len(a)+len(b)-1)
 for i,x in enumerate(a):
  for j,y in enumerate(b):c[i+j]+=x*y
 return c

def basis(knots,p,lo,hi):
 k=list(map(F,knots));mid=(lo+hi)/2
 b=[[F(int(k[i]<=mid<k[i+1]))] for i in range(len(k)-1)]
 for d in range(1,p+1):
  out=[]
  for i in range(len(b)-1):
   value=[F(0)]
   if k[i+d]!=k[i]:value=add(value,mul(b[i],[-k[i]/(k[i+d]-k[i]),1/(k[i+d]-k[i])]))
   if k[i+d+1]!=k[i+1]:value=add(value,mul(b[i+1],[k[i+d+1]/(k[i+d+1]-k[i+1]),-1/(k[i+d+1]-k[i+1])]))
   out.append(value)
  b=out
 return b

def homogeneous(s,ucell,vcell):
 bu=basis(s['knotsU'],s['degreeU'],*ucell);bv=basis(s['knotsV'],s['degreeV'],*vcell)
 assert len(bu)==len(s['weights']) and len(bv)==len(s['weights'][0])
 out=[{} for _ in range(4)]
 for i,a in enumerate(bu):
  for j,b in enumerate(bv):
   w=F(s['weights'][i][j]);h=[F(x)*w for x in s['controlPoints'][i][j]]+[w]
   for k,x in enumerate(a):
    for l,y in enumerate(b):
     for c in range(4):out[c][k,l]=out[c].get((k,l),F(0))+x*y*h[c]
 return out

def product(a,b):
 out={}
 for (i,j),x in a.items():
  for (k,l),y in b.items():out[i+k,j+l]=out.get((i+k,j+l),F(0))+x*y
 return {k:v for k,v in out.items() if v}

def cuts(a,b,axis):
 key='knots'+axis;p=a['degree'+axis];q=b['degree'+axis]
 da=[a[key][p],a[key][-p-1]];db=[b[key][q],b[key][-q-1]]
 if da!=db:return None
 return sorted({F(x) for x in a[key]+b[key] if da[0]<=x<=da[1]})

def identity(a,b):
 u,v=cuts(a,b,'U'),cuts(a,b,'V')
 if u is None or v is None:return False
 for uc in zip(u,u[1:]):
  for vc in zip(v,v[1:]):
   aa,bb=homogeneous(a,uc,vc),homogeneous(b,uc,vc)
   for c in range(3):
    if product(aa[c],bb[3])!=product(bb[c],aa[3]):return False
 return True

def rows(path):
 out=[];decoder=json.JSONDecoder()
 for line in Path(path).read_text().splitlines():
  if 'surface-insertion-oracle=' in line:out.append(decoder.raw_decode(line.split('surface-insertion-oracle=',1)[1])[0])
 return sorted(out,key=lambda x:json.dumps(x,sort_keys=True))
a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert len(a)>=8 and a==b
assert all(identity(r['source'],r['output']) for r in a)
r=next(x for x in a if x['source']['weights']==[[1.,3.],[2.,6.]] and len(x['output']['weights'])==3)
wrong=copy.deepcopy(r['output'])
for row in wrong['weights']:row[1]*=2
assert not identity(r['source'],wrong),'Independent row/column gauges must change tensor law'
wrong=copy.deepcopy(r['output']);wrong['controlPoints'][1][1][2]+=0.125
assert not identity(r['source'],wrong)
wrong=copy.deepcopy(r['output']);wrong['knotsU']=[x+1 for x in wrong['knotsU']]
assert not identity(r['source'],wrong)
print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':3,'failed':0,'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),'scope':'Exact full bivariate homogeneous cross-products on every original/output tensor knot cell. Actual binary64 data; default/no-codec equality. No sampled geometry, runtime, topology or compiler claim.'},indent=2))
