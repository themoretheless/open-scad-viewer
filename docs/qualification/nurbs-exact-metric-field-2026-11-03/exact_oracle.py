"""Actual stored E/F/G fields: exact original-source Gram and area identities."""
from pathlib import Path
from math import nextafter,inf
import copy,json,runpy,sys,hashlib
SOURCE=Path('docs/qualification/nurbs-exact-surface-derivative-2026-11-03/exact_oracle.py')
m=runpy.run_path(str(SOURCE));q=m['q'];mul=m['mul'];add=m['add'];partial=m['partial']
def sub(a,b):return add(a,{k:-v for k,v in b.items()})
def dot(a,b):
    out={}
    for x,y in zip(a,b):out=add(out,mul(x,y))
    return out

def verify(r):
    try:
        s=r['source'];nu=len(s['controlPoints']);nv=len(s['controlPoints'][0]);us=[i for i in range(s['degreeU'],nu) if s['knotsU'][i]<s['knotsU'][i+1]];vs=[j for j in range(s['degreeV'],nv) if s['knotsV'][j]<s['knotsV'][j+1]]
        if len(r['patches'])!=len(us)*len(vs):return False
        for p,(ku,kv) in zip(r['patches'],[(i,j) for i in us for j in vs]):
            pu,pv=p['degreeU'],p['degreeV'];a,b=s['knotsU'][ku:ku+2];c,d=s['knotsV'][kv:kv+2]
            if not(1<=pu<=25 and 1<=pv<=25) or p['periodicU'] or p['periodicV']:return False
            if p['knotsU']!=[a]*(pu+1)+[b]*(pu+1) or p['knotsV']!=[c]*(pv+1)+[d]*(pv+1):return False
            if len(p['controlPoints'])!=pu+1 or len(p['weights'])!=pu+1:return False
            if any(len(row)!=pv+1 for row in p['controlPoints']+p['weights']) or any(len(x)!=3 for row in p['controlPoints'] for x in row) or any(q(w)<=0 for row in p['weights'] for w in row):return False
            h,widths=m['original_power'](s,ku,kv);w=h[3];vectors=[]
            for axis in [0,1]:
                dw=partial(w,axis,widths[axis]);vectors.append([sub(mul(partial(h[i],axis,widths[axis]),w),mul(h[i],dw)) for i in range(3)])
            u,v=vectors;gram=[dot(u,u),dot(u,v),dot(v,v)];w2=mul(w,w);w4=mul(w2,w2);actual=m['stored_power'](p)
            if any(mul(actual[i],w4)!=mul(gram[i],actual[3]) for i in range(3)):return False
            cross=[sub(mul(u[(i+1)%3],v[(i+2)%3]),mul(u[(i+2)%3],v[(i+1)%3])) for i in range(3)]
            determinant=sub(mul(actual[0],actual[2]),mul(actual[1],actual[1]))
            if mul(determinant,mul(w4,w4))!=mul(dot(cross,cross),mul(actual[3],actual[3])):return False
        return True
    except (KeyError,ValueError,TypeError,IndexError,ZeroDivisionError,OverflowError):return False

def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-metric-field-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-metric-field-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))

if __name__=='__main__':
    records=rows(sys.argv[1]);assert len(records)==13 and all(verify(r) for r in records)
    if len(sys.argv)>2:assert records==rows(sys.argv[2])
    plane=next(r for r in records if r['source']['controlPoints']==[[[0.,0.,1.],[2.,4.,1.]],[[3.,0.,1.],[5.,4.,1.]]]);curved=next(r for r in records if r['source']['weights']==[[1.,2.],[2.,4.]] and r['source']['controlPoints'][1][1]==[6.,6.,18.]);multi=next(r for r in records if len(r['patches'])>1);bad=[]
    r=copy.deepcopy(plane);r['patches'][0]['controlPoints']=[[[x[0],x[1]*2,x[2]] for x in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(plane);r['patches'][0]['controlPoints']=[[[x[2],x[1],x[0]] for x in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(plane);r['patches'][0]['controlPoints']=[[[x[0],-x[1],x[2]] for x in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(plane);r['patches'][0]['controlPoints']=[[[1.,0.,1.] for x in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(plane);r['patches'][0]['controlPoints']=[[[16*x[0],32*x[1],64*x[2]] for x in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(curved);r['patches'][0]['weights'][1]=[2*w for w in r['patches'][0]['weights'][1]];bad.append(r)
    r=copy.deepcopy(curved);r['patches'][0]['controlPoints'][0][0][0]=nextafter(r['patches'][0]['controlPoints'][0][0][0],inf);bad.append(r)
    r=copy.deepcopy(curved);r['source']['weights'][0][0]*=2;bad.append(r)
    r=copy.deepcopy(multi);r['patches'].pop();bad.append(r)
    r=copy.deepcopy(curved);r['patches'][0]['periodicU']=True;bad.append(r)
    assert all(not verify(r) for r in bad)
    gauge=copy.deepcopy(curved);gauge['patches'][0]['weights']=[[2*w for w in row] for row in gauge['patches'][0]['weights']];assert verify(gauge)
    print(json.dumps({'acceptedSourceRecords':len(records),'outputPatches':sum(len(r['patches']) for r in records),'negativeControls':len(bad),'positiveGlobalGaugeControl':True,'fullGramDeterminantAreaIdentity':True,'definitionsSha256':hashlib.sha256(json.dumps(records,sort_keys=True).encode()).hexdigest(),'sourceReconstructionSha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'scope':'Exact original homogeneous source reconstruction; actual E/F/G full-cell Gram identities, physical UV clocks and det(metric)=squared area-vector norm. No numerical sampling certification, positive-definite/regularity/body certificate, topology or application delivery credit.'},indent=2))
