"""Independent exact source reconstruction and full-span binormal identities."""
from pathlib import Path
import copy,json,runpy,sys,hashlib
from math import nextafter,inf
SOURCE=Path('docs/qualification/nurbs-exact-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];derivative=m['derivative']
def sub(a,b):return add(a,[-x for x in b])
def verify(r):
 try:
  s=r['source'];n=len(s['controlPoints']);spans=[k for k in range(s['degree'],n) if s['knots'][k]<s['knots'][k+1]]
  if len(spans)!=len(r['pieces']) or any(len(p)!=3 for p in s['controlPoints']):return False
  for k,c in zip(spans,r['pieces']):
   d=c['degree'];a,b=s['knots'][k:k+2]
   if not 1<=d<=25 or c['periodic'] or c['knots']!=[a]*(d+1)+[b]*(d+1):return False
   if len(c['controlPoints'])!=d+1 or len(c['weights'])!=d+1 or any(len(x)!=3 for x in c['controlPoints']) or any(q(w)<=0 for w in c['weights']):return False
   h,width=m['source_power'](s,k);actual=m['stored_power'](c);w=h[-1];dw=derivative(w,width)
   v=[sub(mul(derivative(x,width),w),mul(x,dw)) for x in h[:-1]]
   acc=[sub(mul(derivative(x,width),w),[2*y for y in mul(x,dw)]) for x in v]
   w5=mul(mul(mul(w,w),mul(w,w)),w)
   for i,(j,k2) in enumerate([(1,2),(2,0),(0,1)]):
    target=sub(mul(v[j],acc[k2]),mul(v[k2],acc[j]))
    if mul(actual[i],w5)!=mul(target,actual[-1]):return False
  return True
 except (KeyError,ValueError,TypeError,ZeroDivisionError,IndexError,OverflowError):return False
def rows(path):
 d=json.JSONDecoder()
 return [d.raw_decode(line.split('exact-binormal-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-binormal-oracle=' in line]
if __name__=='__main__':
 records=rows(sys.argv[1]);assert len(records)==11 and sum(len(r['pieces']) for r in records)==15 and all(verify(r) for r in records)
 seed=next(r for r in records if r['source']['weights']==[1.,2.,4.]);multi=next(r for r in records if len(r['pieces'])>1);bad=[]
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][2]=nextafter(r['pieces'][0]['controlPoints'][0][2],inf);bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['weights'][0]*=2;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[-x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
 r=copy.deepcopy(seed);r['source']['knots']=[2*x for x in r['source']['knots']];bad.append(r)
 r=copy.deepcopy(seed);r['source']['controlPoints'][1][0]+=1;bad.append(r)
 r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
 r=copy.deepcopy(multi);r['pieces'].pop();bad.append(r)
 r=copy.deepcopy(multi);r['pieces'][0]['knots'][-1]+=1;bad.append(r)
 assert all(not verify(r) for r in bad)
 gauge=copy.deepcopy(seed);gauge['pieces'][0]['weights']=[2*x for x in gauge['pieces'][0]['weights']];assert verify(gauge)
 print(json.dumps({'acceptedSourceRecords':len(records),'outputPieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'positiveGlobalGaugeControl':True,'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original source reconstruction and actual stored full-span cross-product identities. No normalization, regularity, topology or application qualification.'},indent=2))
