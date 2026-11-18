"""Independent source-basis reconstruction and generic rational-pair derivatives."""
from pathlib import Path
from math import nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];derivative=m['derivative']
def sub(a,b):return add(a,[-x for x in b])
def verify(r):
    try:
        order=r['order']
        if type(order)!=int or not 1<=order<=3:return False
        s=r['source'];n=len(s['controlPoints']);dim=len(s['controlPoints'][0]);spans=[k for k in range(s['degree'],n) if s['knots'][k]<s['knots'][k+1]]
        if len(spans)!=len(r['pieces']):return False
        for k,c in zip(spans,r['pieces']):
            d=c['degree'];a,b=s['knots'][k:k+2]
            if not 1<=d<=25 or c['periodic'] or c['knots']!=[a]*(d+1)+[b]*(d+1):return False
            if len(c['controlPoints'])!=d+1 or len(c['weights'])!=d+1 or any(len(x)!=dim for x in c['controlPoints']) or any(q(w)<=0 for w in c['weights']):return False
            source,width=m['source_power'](s,k);actual=m['stored_power'](c)
            for axis in range(dim):
                numerator,denominator=source[axis],source[-1]
                # Squaring the current denominator avoids mirroring the native
                # recurrence in powers of the original source denominator.
                for _ in range(order):
                    numerator=sub(mul(derivative(numerator,width),denominator),mul(numerator,derivative(denominator,width)))
                    denominator=mul(denominator,denominator)
                if mul(actual[axis],denominator)!=mul(numerator,actual[-1]):return False
        return True
    except (KeyError,ValueError,TypeError,ZeroDivisionError,IndexError,OverflowError):return False

def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-curve-order-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-curve-order-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))
if __name__=='__main__':
    records=rows(sys.argv[1]);assert len(records)==20 and all(verify(r) for r in records)
    seed=next(r for r in records if r['order']==2 and r['source']['controlPoints']==[[0.,0.],[2.,0.]] and r['source']['weights']==[1.,2.]);multi=next(r for r in records if len(r['pieces'])>1);bad=[]
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][0]=nextafter(r['pieces'][0]['controlPoints'][0][0],inf);bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['weights'][0]*=2;bad.append(r)
    r=copy.deepcopy(seed);r['order']=3;bad.append(r)
    r=copy.deepcopy(seed);r['source']['knots']=[x if x<2 else x+1 for x in r['source']['knots']];bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[16*x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
    r=copy.deepcopy(seed);r['source']['controlPoints'][1][0]+=1;bad.append(r)
    r=copy.deepcopy(multi);r['pieces'].pop();bad.append(r)
    r=copy.deepcopy(multi);r['pieces'][0]['knots'][-1]+=1;bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[-x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
    assert all(not verify(r) for r in bad)
    gauge=copy.deepcopy(seed);gauge['pieces'][0]['weights']=[2*w for w in gauge['pieces'][0]['weights']];assert verify(gauge)
    third=next(r for r in records if r['order']==3 and r['source']['controlPoints']==[[0.,0.],[2.,0.]] and r['source']['weights']==[1.,2.]);assert any(x[0]!=0 for x in third['pieces'][0]['controlPoints'])
    print(json.dumps({'acceptedSourceRecords':len(records),'outputPieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'positiveGlobalGaugeControl':True,'nonzeroRationalDerivativeAbovePolynomialDegree':True,'definitionsSha256':hashlib.sha256(json.dumps(records,sort_keys=True).encode()).hexdigest(),'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original Cox-de Boor polynomial reconstruction, generic rational-pair differentiation and actual stored output full-span identities on original absolute clocks. Not finite differences, global knot smoothness, topology or installed application qualification.'},indent=2))
