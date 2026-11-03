"""Independent tensor source reconstruction and global squared-error polynomial."""
from pathlib import Path
from fractions import Fraction as F
from math import comb,isfinite,nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-surface-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];partial=m['partial']
def sub(a,b):return add(a,{k:-x for k,x in b.items()})
def degree(p,axis):return max((k[axis] for k in p),default=0)
def bernstein(p,pu,pv):
 out=[]
 for i in range(pu+1):
  for j in range(pv+1):out.append(sum((x*F(comb(i,a),comb(pu,a))*F(comb(j,b),comb(pv,b)) for (a,b),x in p.items() if a<=i and b<=j),F(0)))
 return out
def target(s,ku,kv,field):
 h,widths=m['original_power'](s,ku,kv);assert min(bernstein(h[3],s['degreeU'],s['degreeV']))>0
 def jet(orders):
  denominator=h[3];numerators=h[:3]
  # Generic rational-pair quotient rule, independent of native W^k recurrence.
  for axis,n in enumerate(orders):
   for _ in range(n):
    numerators=[sub(mul(partial(p,axis,widths[axis]),denominator),mul(p,partial(denominator,axis,widths[axis]))) for p in numerators];denominator=mul(denominator,denominator)
  return numerators+[denominator]
 if field in ['partial_u','partial_v']:return jet([1,0] if field=='partial_u' else [0,1])
 if field.startswith('mixed_'):
  orders=list(map(int,field.split('_')[1:]));assert len(orders)==2 and min(orders)>=0 and 1<=sum(orders)<=3;return jet(orders)
 u,v=jet([1,0]),jet([0,1]);denominator=mul(u[3],v[3])
 if field=='area_normal':return [sub(mul(u[j],v[k]),mul(u[k],v[j])) for j,k in [(1,2),(2,0),(0,1)]]+[denominator]
 assert field=='metric';out=[]
 for a,b in [(u,u),(u,v),(v,v)]:
  total={}
  for i in range(3):total=add(total,mul(a[i],b[i]))
  out.append(total)
 return out+[denominator]
def verify(record):
 try:
  source=record['source'];us=[i for i in range(source['degreeU'],len(source['controlPoints'])) if source['knotsU'][i]<source['knotsU'][i+1]];vs=[j for j in range(source['degreeV'],len(source['controlPoints'][0])) if source['knotsV'][j]<source['knotsV'][j+1]]
  assert len(record['patches'])==len(record['bounds'])==len(us)*len(vs);assert q(record['tolerance'])>0
  for p,upper,(ku,kv) in zip(record['patches'],record['bounds'],[(u,v) for u in us for v in vs]):
   pu,pv=p['degreeU'],p['degreeV'];assert 1<=pu<=25 and 1<=pv<=25 and not p['periodicU'] and not p['periodicV'];a,b=source['knotsU'][ku:ku+2];c,d=source['knotsV'][kv:kv+2]
   assert p['knotsU']==[a]*(pu+1)+[b]*(pu+1) and p['knotsV']==[c]*(pv+1)+[d]*(pv+1)
   assert len(p['controlPoints'])==len(p['weights'])==pu+1 and all(len(row)==pv+1 for row in p['controlPoints']+p['weights']) and all(len(x)==3 for row in p['controlPoints'] for x in row) and all(q(w)>0 for row in p['weights'] for w in row)
   assert isfinite(upper) and 0<=q(upper)<=q(record['tolerance'])
   wanted=target(source,ku,kv,record['field']);stored=m['stored_power'](p);denominator=mul(wanted[3],stored[3]);poly={k:q(upper)**2*x for k,x in mul(denominator,denominator).items()}
   for axis in range(3):
    residual=sub(mul(wanted[axis],stored[3]),mul(stored[axis],wanted[3]));poly=sub(poly,mul(residual,residual))
   du=2*(max(degree(x,0) for x in wanted)+pu);dv=2*(max(degree(x,1) for x in wanted)+pv)
   assert min(bernstein(poly,du,dv))>=0
  return True
 except (AssertionError,ValueError,KeyError,TypeError,IndexError,ZeroDivisionError,OverflowError):return False
def rows(path):
 decoder=json.JSONDecoder();return [decoder.raw_decode(line.split('bounded-surface-differential-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'bounded-surface-differential-oracle=' in line]
if __name__=='__main__':
 records=rows(sys.argv[1]);assert len(records)==25 and sum(len(r['patches']) for r in records)==50 and all(verify(r) for r in records)
 seed=next(r for r in records if r['field']=='partial_u' and r['source']['weights']==[[1.,2.],[3.,5.]]);bad=[]
 r=copy.deepcopy(seed);r['bounds']=[0.];bad.append(r)
 r=copy.deepcopy(seed);r['patches'][0]['controlPoints'][0][0][0]+=.01;bad.append(r)
 r=copy.deepcopy(seed);r['patches'][0]['weights'][1]=[2*x for x in r['patches'][0]['weights'][1]];bad.append(r)
 r=copy.deepcopy(seed);r['source']['weights'][0][0]*=2;bad.append(r)
 r=copy.deepcopy(seed);r['field']='partial_v';bad.append(r)
 r=copy.deepcopy(seed);r['patches'][0]['periodicV']=True;bad.append(r)
 r=copy.deepcopy(seed);r['tolerance']=r['bounds'][0]/2;bad.append(r)
 r=copy.deepcopy(seed);r['source']['knotsU']=[2*x for x in r['source']['knotsU']];r['patches'][0]['knotsU']=[2*x for x in r['patches'][0]['knotsU']];bad.append(r)
 multi=next(r for r in records if len(r['patches'])>1)
 r=copy.deepcopy(multi);r['patches'].pop();r['bounds'].pop();bad.append(r)
 r=copy.deepcopy(multi);r['patches'][0],r['patches'][1]=r['patches'][1],r['patches'][0];bad.append(r)
 assert all(not verify(r) for r in bad)
 gauge=copy.deepcopy(seed);gauge['patches'][0]['weights']=[[2*x for x in row] for row in gauge['patches'][0]['weights']];assert verify(gauge)
 larger=copy.deepcopy(seed);larger['bounds'][0]=nextafter(larger['bounds'][0],inf);assert verify(larger)
 print(json.dumps({'sourceRecords':len(records),'outputPatches':sum(len(r['patches']) for r in records),'negativeControls':len(bad),'wholeNetGaugeControl':True,'largerValidBoundControl':True,'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original homogeneous tensor reconstruction and generic quotient differentiation. Full UV Bernstein nonnegative squared-error polynomials against actual stored bits; no numerical samples, native factor-cancellation trust, topology or application authority.'},indent=2))
