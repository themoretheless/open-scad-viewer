"""Independent original-source rational-pair differentiation, actual stored bits."""
from pathlib import Path
from math import nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-surface-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];partial=m['partial']
def sub(a,b):return add(a,{k:-v for k,v in b.items()})

def verify(r):
    try:
        orders=r['orders']
        if len(orders)!=2 or any(type(n)!=int or n<0 for n in orders) or not 1<=sum(orders)<=3:return False
        s=r['source'];nu=len(s['controlPoints']);nv=len(s['controlPoints'][0]);us=[i for i in range(s['degreeU'],nu) if s['knotsU'][i]<s['knotsU'][i+1]];vs=[j for j in range(s['degreeV'],nv) if s['knotsV'][j]<s['knotsV'][j+1]]
        if len(r['patches'])!=len(us)*len(vs):return False
        for p,(ku,kv) in zip(r['patches'],[(i,j) for i in us for j in vs]):
            pu,pv=p['degreeU'],p['degreeV'];a,b=s['knotsU'][ku:ku+2];c,d=s['knotsV'][kv:kv+2]
            if not(1<=pu<=25 and 1<=pv<=25) or p['periodicU'] or p['periodicV']:return False
            if p['knotsU']!=[a]*(pu+1)+[b]*(pu+1) or p['knotsV']!=[c]*(pv+1)+[d]*(pv+1):return False
            if len(p['controlPoints'])!=pu+1 or len(p['weights'])!=pu+1:return False
            if any(len(row)!=pv+1 for row in p['controlPoints']+p['weights']) or any(len(x)!=3 for row in p['controlPoints'] for x in row) or any(q(w)<=0 for row in p['weights'] for w in row):return False
            h,widths=m['original_power'](s,ku,kv);actual=m['stored_power'](p)
            for coordinate in range(3):
                numerator,denominator=h[coordinate],h[3]
                # Generic quotient rule squares the current denominator each time;
                # independent of native N/W^k recurrence and native cancellation.
                for axis,n in enumerate(orders):
                    for _ in range(n):
                        numerator=sub(mul(partial(numerator,axis,widths[axis]),denominator),mul(numerator,partial(denominator,axis,widths[axis])))
                        denominator=mul(denominator,denominator)
                if mul(actual[coordinate],denominator)!=mul(numerator,actual[3]):return False
        return True
    except (KeyError,ValueError,TypeError,IndexError,ZeroDivisionError,OverflowError):return False

def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-mixed-partial-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-mixed-partial-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))

if __name__=='__main__':
    records=rows(sys.argv[1]);assert len(records)==27 and all(verify(r) for r in records)
    seed=next(r for r in records if r['orders']==[2,0] and r['source']['weights']==[[1.,2.],[2.,4.]])
    multi=next(r for r in records if len(r['patches'])>1);bad=[]
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints'][0][0][0]=nextafter(r['patches'][0]['controlPoints'][0][0][0],inf);bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['weights'][1]=[2*w for w in r['patches'][0]['weights'][1]];bad.append(r)
    r=copy.deepcopy(seed);r['orders']=[1,1];bad.append(r)
    r=copy.deepcopy(seed);r['source']['knotsU']=[x if x<6 else x+1 for x in r['source']['knotsU']];bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints']=[[[16*x for x in p] for p in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(multi);r['patches'].pop();bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['knotsV'][-1]+=1;bad.append(r)
    r=copy.deepcopy(seed);r['source']['weights'][0][0]*=2;bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['periodicU']=True;bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints']=[[[-x for x in p] for p in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    assert all(not verify(r) for r in bad)
    gauge=copy.deepcopy(seed);gauge['patches'][0]['weights']=[[2*w for w in row] for row in gauge['patches'][0]['weights']];assert verify(gauge)
    third=next(r for r in records if r['orders']==[3,0] and r['source']['weights']==[[1.,2.],[2.,4.]])
    assert any(p[0]!=0 for row in third['patches'][0]['controlPoints'] for p in row)
    print(json.dumps({'acceptedSourceRecords':len(records),'outputPatches':sum(len(r['patches']) for r in records),'negativeControls':len(bad),'positiveGlobalGaugeControl':True,'nonzeroRationalDerivativeAbovePolynomialDegree':True,'definitionsSha256':hashlib.sha256(json.dumps(records,sort_keys=True).encode()).hexdigest(),'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original homogeneous reconstruction; independent generic quotient-pair mixed differentiation, original UV clocks, complete cell cover and actual stored output cross-identities. Not finite differences, global knot smoothness, topology, installed WASM/editor/STEP or whole-application qualification.'},indent=2))
