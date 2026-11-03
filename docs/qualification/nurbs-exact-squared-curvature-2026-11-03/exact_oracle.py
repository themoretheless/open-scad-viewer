"""Independent Fraction source reconstruction, regularity and curvature identity."""
from pathlib import Path
from fractions import Fraction as F
from math import comb,nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];derivative=m['derivative']
def sub(a,b):return add(a,[-x for x in b])
def verify(r):
 try:
  s=r['source'];n=len(s['controlPoints']);dim=len(s['controlPoints'][0]);spans=[k for k in range(s['degree'],n) if s['knots'][k]<s['knots'][k+1]]
  if dim not in [2,3] or len(spans)!=len(r['pieces']):return False
  for k,c in zip(spans,r['pieces']):
   d=c['degree'];a,b=s['knots'][k:k+2]
   if not 1<=d<=25 or c['periodic'] or c['knots']!=[a]*(d+1)+[b]*(d+1):return False
   if len(c['controlPoints'])!=d+1 or len(c['weights'])!=d+1 or any(len(x)!=dim for x in c['controlPoints']) or any(q(w)<=0 for w in c['weights']):return False
   h,width=m['source_power'](s,k);actual=m['stored_power'](c);w=h[-1];dw=derivative(w,width)
   v=[sub(mul(derivative(x,width),w),mul(x,dw)) for x in h[:-1]]
   acc=[sub(mul(derivative(x,width),w),[2*y for y in mul(x,dw)]) for x in v]
   norm=[]
   for x in v:norm=add(norm,mul(x,x))
   degree=max(0,len(norm)-1)
   if degree>25 or any(sum((norm[j]*F(comb(i,j),comb(degree,j)) for j in range(min(i+1,len(norm)))),F(0))<=0 for i in range(degree+1)):return False
   cross_norm=[]
   for j,k2 in ([(0,1)] if dim==2 else [(1,2),(2,0),(0,1)]):
    cross=sub(mul(v[j],acc[k2]),mul(v[k2],acc[j]));cross_norm=add(cross_norm,mul(cross,cross))
   numerator=mul(cross_norm,mul(w,w));denominator=mul(mul(norm,norm),norm)
   if mul(actual[0],denominator)!=mul(numerator,actual[-1]) or any(actual[i] for i in range(1,dim)):return False
  return True
 except (KeyError,ValueError,TypeError,ZeroDivisionError,IndexError,OverflowError):return False
def rows(path):
 d=json.JSONDecoder();return [d.raw_decode(line.split('exact-squared-curvature-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-squared-curvature-oracle=' in line]
if __name__=='__main__':
 records=rows(sys.argv[1]);assert len(records)==11 and all(verify(r) for r in records)
 seed=next(r for r in records if r['source']['controlPoints']==[[1.,0.],[1.,1.],[0.,1.]]);multi=next(r for r in records if len(r['pieces'])>1);bad=[]
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][0]=nextafter(1.,inf);bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][1]=1.;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[.25*x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
 r=copy.deepcopy(seed);r['source']['controlPoints'][0][0]+=.125;bad.append(r)
 r=copy.deepcopy(seed);r['source']['weights'][-1]=3.;bad.append(r)
 r=copy.deepcopy(multi);r['pieces'].pop();bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
 r=copy.deepcopy(seed);r['source']={'degree':2,'knots':[-2.]*3+[2.]*3,'controlPoints':[[0.,0.],[0.,0.],[1.,0.]],'weights':[1.,1.,1.],'periodic':False};r['pieces'][0]['controlPoints']=[[0.,0.]]*2;bad.append(r)
 assert all(not verify(r) for r in bad)
 gauge=copy.deepcopy(seed);gauge['pieces'][0]['weights']=[2*x for x in gauge['pieces'][0]['weights']];assert verify(gauge)
 clock=copy.deepcopy(seed);clock['source']['knots']=[2*x+1 for x in clock['source']['knots']]
 for c in clock['pieces']:c['knots']=[2*x+1 for x in c['knots']]
 assert verify(clock)
 print(json.dumps({'acceptedSourceRecords':len(records),'outputPieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'globalGaugeControl':True,'clockInvarianceControl':True,'stationarySourceRejectedDespiteZeroQuotient':True,'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Original-source sufficient regularity plus full-span stored-bit squared-curvature identities. No complete regularity solver, topology or application qualification.'},indent=2))
