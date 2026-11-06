"""Original source quotient-pair jets and full-domain torsion certificates."""
from pathlib import Path
from fractions import Fraction as F
from math import comb,isfinite,nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];add=m['add'];mul=m['mul'];derivative=m['derivative']
def sub(a,b):return add(a,[-x for x in b])
def bern(a,d=None):
 if d is None:d=max(0,len(a)-1)
 return [sum((x*F(comb(i,j),comb(d,j)) for j,x in enumerate(a[:i+1])),F(0)) for i in range(d+1)]
def target(source,k):
 h,width=m['source_power'](source,k);assert min(bern(h[-1]))>0
 numerators,denominator=h[:-1],h[-1];jets=[]
 for _ in range(3):
  numerators=[sub(mul(derivative(n,width),denominator),mul(n,derivative(denominator,width))) for n in numerators];denominator=mul(denominator,denominator);jets.append((numerators,denominator))
 (v,dv),(a,da),(j,dj)=jets;cross=[sub(mul(v[x],a[y]),mul(v[y],a[x])) for x,y in [(1,2),(2,0),(0,1)]];norm=[];det=[]
 for x,y in zip(cross,j):norm=add(norm,mul(x,x));det=add(det,mul(x,y))
 # This proves the original, not a cancelled, binormal is nonzero.
 assert min(bern(norm))>0
 return mul(det,mul(dv,da)),mul(dj,norm)
def verify(r):
 try:
  s=r['source'];assert all(len(p)==3 for p in s['controlPoints']);assert all(q(w)>0 for w in s['weights'])
  spans=[k for k in range(s['degree'],len(s['controlPoints'])) if s['knots'][k]<s['knots'][k+1]];assert len(spans)==len(r['pieces']);assert r['mode'] in ['exact','bounded']
  if r['mode']=='bounded':assert len(r['bounds'])==len(spans) and q(r['tolerance'])>0
  for index,(k,p) in enumerate(zip(spans,r['pieces'])):
   d=p['degree'];a,b=s['knots'][k:k+2];assert 1<=d<=25 and not p['periodic'] and p['knots']==[a]*(d+1)+[b]*(d+1);assert len(p['controlPoints'])==len(p['weights'])==d+1 and all(len(x)==3 for x in p['controlPoints']) and all(q(w)>0 for w in p['weights'])
   n,den=target(s,k);stored=m['stored_power'](p);assert stored[1]==stored[2]==[];residual=sub(mul(n,stored[3]),mul(stored[0],den))
   if r['mode']=='exact':assert not residual
   else:
    upper=r['bounds'][index];assert isfinite(upper) and 0<=q(upper)<=q(r['tolerance']);product=mul(den,stored[3]);polynomial=sub([q(upper)**2*x for x in mul(product,product)],mul(residual,residual));degree=2*(max(len(n),len(den))-1+d);assert min(bern(polynomial,degree))>=0
  return True
 except (AssertionError,KeyError,ValueError,TypeError,IndexError,ZeroDivisionError,OverflowError):return False
def rows(path):
 decoder=json.JSONDecoder();return [decoder.raw_decode(line.split('torsion-field-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'torsion-field-oracle=' in line]
if __name__=='__main__':
 records=rows(sys.argv[1]);assert len(records)==13 and sum(len(r['pieces']) for r in records)==16 and all(verify(r) for r in records)
 seed=next(r for r in records if r['mode']=='bounded' and r['source']['controlPoints']==[[0.,0.,0.],[1.,0.,0.],[2.,1.,0.],[3.,3.,3.]]);flat=next(r for r in records if r['mode']=='exact');multi=next(r for r in records if len(r['pieces'])>1);bad=[]
 r=copy.deepcopy(seed);r['bounds']=[0.];bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][0]+=.01;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['weights'][1]*=2;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][1]=.01;bad.append(r)
 r=copy.deepcopy(seed);r['mode']='exact';bad.append(r)
 r=copy.deepcopy(seed);r['tolerance']=r['bounds'][0]/2;bad.append(r)
 r=copy.deepcopy(seed);r['source']['controlPoints']=[[0.,0.,0.],[1.,0.,0.],[2.,0.,0.],[3.,0.,0.]];r['pieces'][0]['controlPoints']=[[0.,0.,0.] for _ in r['pieces'][0]['controlPoints']];r['bounds']=[0.];bad.append(r)
 r=copy.deepcopy(multi);r['pieces'].pop();r['bounds'].pop();bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
 r=copy.deepcopy(flat);r['pieces'][0]['controlPoints'][0][0]=nextafter(0.,inf);bad.append(r)
 assert all(not verify(r) for r in bad)
 gauge=copy.deepcopy(seed);gauge['pieces'][0]['weights']=[2*w for w in gauge['pieces'][0]['weights']];assert verify(gauge)
 larger=copy.deepcopy(seed);larger['bounds'][0]=nextafter(larger['bounds'][0],inf);assert verify(larger)
 print(json.dumps({'sourceRecords':len(records),'outputPieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'globalGaugeControl':True,'largerValidBoundControl':True,'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original Cox-de Boor reconstruction and independent generic quotient-pair jets; original nonzero binormal, full polynomial stored-bit identities or nonnegative squared-error certificates. No sampled acceptance, Frenet frame, topology or application authority.'},indent=2))
