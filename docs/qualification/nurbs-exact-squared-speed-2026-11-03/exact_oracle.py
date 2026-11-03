"""Original homogeneous source: exact full-span squared-speed identity."""
from pathlib import Path
from math import nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];derivative=m['derivative']
def sub(a,b):return add(a,[-x for x in b])
def verify(r):
    try:
        s=r['source'];n=len(s['controlPoints']);dim=len(s['controlPoints'][0]);spans=[k for k in range(s['degree'],n) if s['knots'][k]<s['knots'][k+1]]
        if len(spans)!=len(r['pieces']):return False
        for k,c in zip(spans,r['pieces']):
            d=c['degree'];a,b=s['knots'][k:k+2]
            if not 1<=d<=25 or c['periodic'] or c['knots']!=[a]*(d+1)+[b]*(d+1):return False
            if len(c['controlPoints'])!=d+1 or len(c['weights'])!=d+1 or any(len(x)!=dim for x in c['controlPoints']) or any(q(w)<=0 for w in c['weights']):return False
            h,width=m['source_power'](s,k);actual=m['stored_power'](c);w=h[-1];dw=derivative(w,width);total=[]
            for coordinate in h[:-1]:
                v=sub(mul(derivative(coordinate,width),w),mul(coordinate,dw));total=add(total,mul(v,v))
            w2=mul(w,w);w4=mul(w2,w2)
            if mul(actual[0],w4)!=mul(total,actual[-1]) or any(actual[i] for i in range(1,dim)):return False
        return True
    except (KeyError,ValueError,TypeError,ZeroDivisionError,IndexError,OverflowError):return False

def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-squared-speed-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-squared-speed-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))
if __name__=='__main__':
    records=rows(sys.argv[1]);assert len(records)==17 and all(verify(r) for r in records)
    seed=next(r for r in records if r['source']['controlPoints']==[[0.,0.],[2.,0.]] and r['source']['weights']==[1.,2.] and r['source']['knots']==[-2.,-2.,2.,2.]);multi=next(r for r in records if len(r['pieces'])>1);bad=[]
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][0]=nextafter(r['pieces'][0]['controlPoints'][0][0],inf);bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['weights'][0]*=2;bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][1]=1.;bad.append(r)
    r=copy.deepcopy(seed);r['source']['knots']=[x if x<2 else x+1 for x in r['source']['knots']];bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[16*x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
    r=copy.deepcopy(seed);r['source']['controlPoints'][1][0]+=1;bad.append(r)
    r=copy.deepcopy(multi);r['pieces'].pop();bad.append(r)
    r=copy.deepcopy(multi);r['pieces'][0]['knots'][-1]+=1;bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[-x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
    assert all(not verify(r) for r in bad)
    gauge=copy.deepcopy(seed);gauge['pieces'][0]['weights']=[2*w for w in gauge['pieces'][0]['weights']];assert verify(gauge)
    stationary=next(r for r in records if r['source']['controlPoints']==[[0.25,0.],[-0.25,0.],[0.25,0.]])
    actual=m['stored_power'](stationary['pieces'][0]);assert sum(x*q(0.5)**i for i,x in enumerate(actual[0]))==0 and any(p[0]<0 for p in stationary['pieces'][0]['controlPoints'])
    print(json.dumps({'acceptedSourceRecords':len(records),'outputPieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'positiveGlobalGaugeControl':True,'interiorStationaryPointWithoutRegularityClaim':True,'definitionsSha256':hashlib.sha256(json.dumps(records,sort_keys=True).encode()).hexdigest(),'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original Cox-de Boor source reconstruction, physical-clock velocity norm squared and actual stored full-span scalar-carrier identities. No square root, regularity, topology or application/artifact qualification.'},indent=2))
