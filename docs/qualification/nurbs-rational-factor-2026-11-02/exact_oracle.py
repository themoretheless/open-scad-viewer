"""Independent Fraction polynomial identities from actual Rust-returned bits."""
import json,sys,re,hashlib
from fractions import Fraction as F
from math import comb
from pathlib import Path

def rows(path):
 result={};decoder=json.JSONDecoder()
 for line in Path(path).read_text().splitlines():
  if 'factor-oracle=' in line:
   r,_=decoder.raw_decode(line.split('factor-oracle=',1)[1]);assert r['name'] not in result;result[r['name']]=r
 return result

def power(values):
 p=len(values)-1;a=[F(0)]*(p+1)
 for i,b in enumerate(values):
  for j in range(p-i+1):a[i+j]+=b*comb(p,i)*comb(p-i,j)*(-1)**j
 return a

def multiply(a,b):
 c=[F(0)]*(len(a)+len(b)-1)
 for i,x in enumerate(a):
  for j,y in enumerate(b):c[i+j]+=x*y
 return c

def homogeneous(c):
 p=c['degree'];n=p+1;assert len(c['controlPoints'])==len(c['weights'])==n
 assert len(c['knots'])==2*n and c['knots']==[c['knots'][0]]*n+[c['knots'][-1]]*n and c['knots'][0]<c['knots'][-1]
 weights=[F.from_float(float(w)) for w in c['weights']];assert all(w>0 for w in weights)
 coordinates=list(zip(*c['controlPoints']));return [power([F.from_float(float(x))*w for x,w in zip(axis,weights)]) for axis in coordinates]+[power(weights)]

def identity(a,b):
 assert a['knots'][0]==b['knots'][0] and a['knots'][-1]==b['knots'][-1]
 aa,bb=homogeneous(a),homogeneous(b);assert len(aa)==len(bb)
 residual=[]
 for x,y in zip(aa[:-1],bb[:-1]):residual.extend(u-v for u,v in zip(multiply(x,bb[-1]),multiply(y,aa[-1])))
 return all(x==0 for x in residual)

actual=rows(sys.argv[1]);other=rows(sys.argv[2]);assert len(actual)==len(other)==6
canonical=lambda r:json.dumps(r,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
assert canonical(actual)==canonical(other),'Feature modes emitted different source/output bit roundtrips'
checks={name:identity(r['source'],r['output']) for name,r in actual.items()}
wrong=dict(actual['rational-clock']['output']);wrong['weights']=[1.,4.]
checks['wrong-clock-detected']=not identity(actual['rational-clock']['source'],wrong)
source={'degree':3,'knots':[-2.]*4+[6.]*4,'controlPoints':[[x,0.] for x in [0.,.25,5.5,1.]],'weights':[12.,4.,1.,18.]}
wrong={'degree':2,'knots':[-2.]*3+[6.]*3,'controlPoints':[[0.,0.],[-.5,0.],[1.,0.]],'weights':[6.,1.5,6.]}
checks['per-pole-weight-sign-change-detected']=not identity(source,wrong)
assert all(checks.values()),checks
print(json.dumps({'passed':len(checks),'failed':0,'checks':checks,'acceptedDefinitionsSha256':hashlib.sha256(canonical(actual)).hexdigest(),'scope':'Exact full polynomial cross-products of actual binary64 source/output definitions, same original clocks; no sampled geometry, topology, global qualification or formal compiler claim.'},indent=2))
