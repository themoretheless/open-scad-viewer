"""Exact original source, stored bits and whole-span Euclidean error replay."""
from pathlib import Path
from fractions import Fraction as F
from math import comb,isfinite,nextafter,inf
import copy,hashlib,json,runpy,sys
SOURCE=Path('docs/qualification/nurbs-exact-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];trim=m['trim'];mul=m['mul'];add=m['add'];derivative=m['derivative']
def sub(a,b):return add(a,[-x for x in b])
def bern(a,d=None):
 if d is None:d=max(0,len(a)-1)
 return [sum((x*F(comb(i,j),comb(d,j)) for j,x in enumerate(a[:i+1])),F(0)) for i in range(d+1)]
def divide(a,b):
 a=a[:];out=[F(0)]*max(0,len(a)-len(b)+1)
 while a and len(a)>=len(b):
  i=len(a)-len(b);x=a[-1]/b[-1];out[i]=x
  for j,y in enumerate(b):a[i+j]-=x*y
  trim(a)
 return trim(out),a
def gcd(a,b):
 while b:a,b=b,divide(a,b)[1]
 return [x/a[-1] for x in a] if a else []
def squared_norm(a):
 out=[]
 for x in a:out=add(out,mul(x,x))
 return out
def target(s,k,field):
 h,width=m['source_power'](s,k);w=h[-1];dw=derivative(w,width);dim=len(h)-1
 assert min(bern(w))>0
 def jet(order):
  n=h[:-1]
  for exponent in range(1,order+1):n=[sub(mul(derivative(x,width),w),[exponent*y for y in mul(x,dw)]) for x in n]
  denominator=[F(1)]
  for _ in range(order+1):denominator=mul(denominator,w)
  return n+[denominator]
 if field.startswith('derivative'):
  n=int(field.removeprefix('derivative'));assert 1<=n<=3;return jet(n)
 velocity=jet(1);speed=squared_norm(velocity[:-1])
 if field=='squared_speed':return [speed]+[[] for _ in range(dim-1)]+[mul(velocity[-1],velocity[-1])]
 acceleration=jet(2);cross=[sub(mul(velocity[j],acceleration[z]),mul(velocity[z],acceleration[j])) for j,z in ([(0,1)] if dim==2 else [(1,2),(2,0),(0,1)])]
 if field=='binormal':assert dim==3;return cross+[mul(velocity[-1],acceleration[-1])]
 assert field=='squared_curvature';assert len(speed)<=26 and min(bern(speed))>0
 return [mul(squared_norm(cross),mul(w,w))]+[[] for _ in range(dim-1)]+[mul(mul(speed,speed),speed)]
def reduced(raw):
 common=raw[-1]
 for row in raw[:-1]:common=gcd(common,row)
 out=[]
 for row in raw:
  quotient,remainder=divide(row,common);assert not remainder;out.append(quotient)
 for axis in range(len(raw)-1):assert mul(out[axis],raw[-1])==mul(raw[axis],out[-1])
 if max(bern(out[-1]))<0:out=[[-x for x in row] for row in out]
 assert min(bern(out[-1]))>0
 return out
def verify(record):
 try:
  s=record['source'];dim=len(s['controlPoints'][0]);assert dim in [2,3] and all(q(w)>0 for w in s['weights'])
  spans=[k for k in range(s['degree'],len(s['controlPoints'])) if s['knots'][k]<s['knots'][k+1]]
  assert len(spans)==len(record['pieces'])==len(record['bounds']);tol=q(record['tolerance']);assert tol>0
  for k,c,upper in zip(spans,record['pieces'],record['bounds']):
   d=c['degree'];a,b=s['knots'][k:k+2];assert 1<=d<=25 and not c['periodic'] and c['knots']==[a]*(d+1)+[b]*(d+1)
   assert len(c['controlPoints'])==len(c['weights'])==d+1 and all(len(p)==dim for p in c['controlPoints']) and all(q(w)>0 for w in c['weights'])
   assert isfinite(upper) and 0<=q(upper)<=tol
   raw=target(s,k,record['field']);wanted=reduced(raw);assert d==max(1,max(len(row)-1 for row in wanted));stored=m['stored_power'](c)
   denominator=min(bern(wanted[-1],d))*min(map(q,c['weights']));assert denominator>0
   squared=F(0)
   for axis in range(dim):
    residual=sub(mul(wanted[axis],stored[-1]),mul(stored[axis],wanted[-1]));maximum=max(map(abs,bern(residual)));squared+=(maximum/denominator)**2
   assert q(upper)**2>=squared
   # Additional direct polynomial certificate, independent of the scalar hull.
   # If raw factors obscure coefficient signs, use the exactly cross-checked quotient.
   residuals=[sub(mul(wanted[i],stored[-1]),mul(stored[i],wanted[-1])) for i in range(dim)]
   den=mul(wanted[-1],stored[-1]);nonnegative=sub([q(upper)**2*x for x in mul(den,den)],squared_norm(residuals))
   assert min(bern(nonnegative,max(4*d,len(nonnegative)-1)))>=0
  return True
 except (AssertionError,KeyError,ValueError,TypeError,ZeroDivisionError,IndexError,OverflowError):return False
def rows(path):
 decoder=json.JSONDecoder();return [decoder.raw_decode(line.split('bounded-differential-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'bounded-differential-oracle=' in line]
if __name__=='__main__':
 records=rows(sys.argv[1]);assert len(records)==14 and sum(len(r['pieces']) for r in records)==18 and all(verify(r) for r in records)
 seed=next(r for r in records if r['source']['controlPoints']==[[0.,0.],[.5,0.],[1.,1.]] and r['field']=='squared_curvature');multi=next(r for r in records if len(r['pieces'])>1);bad=[]
 r=copy.deepcopy(seed);r['bounds']=[0.];bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][0]+=.001;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['weights'][1]*=2;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][1]=.001;bad.append(r)
 r=copy.deepcopy(seed);r['field']='squared_speed';bad.append(r)
 r=copy.deepcopy(seed);r['tolerance']=r['bounds'][0]/2;bad.append(r)
 r=copy.deepcopy(seed);r['source']['controlPoints'][1][1]+=.001;bad.append(r)
 r=copy.deepcopy(multi);r['pieces'].pop();r['bounds'].pop();bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
 r=copy.deepcopy(seed);r['source']['controlPoints']=[[0.,0.],[0.,0.],[1.,0.]];r['pieces'][0]={'degree':1,'knots':[0.,0.,1.,1.],'controlPoints':[[0.,0.],[0.,0.]],'weights':[1.,1.],'periodic':False};r['bounds']=[0.];bad.append(r)
 assert all(not verify(r) for r in bad)
 gauge=copy.deepcopy(seed);gauge['pieces'][0]['weights']=[2*x for x in gauge['pieces'][0]['weights']];assert verify(gauge)
 conservative=copy.deepcopy(seed);conservative['bounds'][0]=nextafter(conservative['bounds'][0],inf);assert verify(conservative)
 print(json.dumps({'sourceRecords':len(records),'outputPieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'globalGaugeControl':True,'LargerValidUpperControl':True,'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Independent Fraction original-source reconstruction, uncancelled-to-reduced exact cross identities, actual stored-bit Euclidean residual hulls and full polynomial nonnegative error certificates. No sampled acceptance, topology, serialized-certificate or application authority.'},indent=2))
