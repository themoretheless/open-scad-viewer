"""Independent exact Cox-de Boor polynomial oracle on every original/output cell."""
import json, sys, hashlib, copy
from fractions import Fraction as F
from pathlib import Path

def add(a,b):
 c=[F(0)]*max(len(a),len(b))
 for i,x in enumerate(a): c[i]+=x
 for i,x in enumerate(b): c[i]+=x
 return c

def mul(a,b):
 c=[F(0)]*(len(a)+len(b)-1)
 for i,x in enumerate(a):
  for j,y in enumerate(b): c[i+j]+=x*y
 return c

def channels(c,lo,hi):
 k=list(map(F,c['knots']));p=c['degree'];n=len(c['weights']);mid=(lo+hi)/2
 basis=[[F(int(k[i]<=mid<k[i+1]))] for i in range(len(k)-1)]
 for d in range(1,p+1):
  new=[]
  for i in range(len(basis)-1):
   value=[F(0)]
   if k[i+d]!=k[i]: value=add(value,mul(basis[i],[-k[i]/(k[i+d]-k[i]),1/(k[i+d]-k[i])]))
   if k[i+d+1]!=k[i+1]: value=add(value,mul(basis[i+1],[k[i+d+1]/(k[i+d+1]-k[i+1]),-1/(k[i+d+1]-k[i+1])]))
   new.append(value)
  basis=new
 assert len(basis)==n
 out=[]
 for axis in range(len(c['controlPoints'][0])+1):
  value=[F(0)]
  for i,b in enumerate(basis):
   weight=F(c['weights'][i]);h=weight if axis==len(c['controlPoints'][0]) else weight*F(c['controlPoints'][i][axis])
   value=add(value,[x*h for x in b])
  out.append(value)
 return out

def identity(a,b):
 da=[a['knots'][a['degree']],a['knots'][len(a['weights'])]]
 db=[b['knots'][b['degree']],b['knots'][len(b['weights'])]]
 if da!=db:return False
 cuts=sorted({F(x) for x in a['knots']+b['knots'] if da[0]<=x<=da[1]})
 for lo,hi in zip(cuts,cuts[1:]):
  aa,bb=channels(a,lo,hi),channels(b,lo,hi)
  for x,y in zip(aa[:-1],bb[:-1]):
   delta=add(mul(x,bb[-1]),[-t for t in mul(y,aa[-1])])
   if any(delta):return False
 return True

def rows(path):
 result=[];decoder=json.JSONDecoder()
 for line in Path(path).read_text().splitlines():
  if 'insertion-oracle=' in line:result.append(decoder.raw_decode(line.split('insertion-oracle=',1)[1])[0])
 return sorted(result,key=lambda x:json.dumps(x,sort_keys=True))
a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert a==b and len(a)>=7
assert all(identity(r['source'],r['output']) for r in a)
wrong=copy.deepcopy(a[0]);wrong['output']['controlPoints'][1][0]+=0.125
assert not identity(wrong['source'],wrong['output'])
wrong=copy.deepcopy(a[0]);wrong['output']['knots']=[x+1 for x in wrong['output']['knots']]
assert not identity(wrong['source'],wrong['output'])
print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':2,'failed':0,'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),'scope':'Exact rational polynomial cross products on every source/output knot cell; actual binary64 coefficients; default/no-codec outputs match. No runtime or topology claim.'},indent=2))
