"""Independent homogeneous tensor de Boor + exact degree-bounded interpolation."""
from fractions import Fraction as F
from math import comb,nextafter,inf
from pathlib import Path
import copy,json,sys,hashlib

def q(x):return F(float(x))
def add(a,b):
    c=a.copy()
    for key,value in b.items():
        c[key]=c.get(key,F(0))+value
        if not c[key]:del c[key]
    return c

def mul(a,b):
    c={}
    for (i,j),x in a.items():
        for (k,l),y in b.items():
            key=(i+k,j+l);c[key]=c.get(key,F(0))+x*y
            if not c[key]:del c[key]
    return c

def partial(a,axis,width):
    out={}
    for key,x in a.items():
        n=key[axis]
        if n:
            k=list(key);k[axis]-=1;out[tuple(k)]=n*x/width
    return out

def de_boor(values,knots,p,k,u):
    d=copy.deepcopy(values)
    for r in range(1,p+1):
        for j in range(p,r-1,-1):
            i=k-p+j;alpha=(u-knots[i])/(knots[i+p-r+1]-knots[i])
            d[j]=[(1-alpha)*a+alpha*b for a,b in zip(d[j-1],d[j])]
    return d[-1]

def evaluate_h(s,ku,kv,u,v):
    pu,pv=s['degreeU'],s['degreeV'];U=list(map(q,s['knotsU']));V=list(map(q,s['knotsV']));columns=[]
    for j in range(kv-pv,kv+1):
        values=[]
        for i in range(ku-pu,ku+1):
            weight=q(s['weights'][i][j]);values.append([q(x)*weight for x in s['controlPoints'][i][j]]+[weight])
        columns.append(de_boor(values,U,pu,ku,u))
    return de_boor(columns,V,pv,kv,v)

def interpolate(values,p):
    # These are exact homogeneous polynomial values at p+1 distinct rational
    # nodes, with a proved source degree bound; not numerical geometric samples.
    nodes=[F(i,p) for i in range(p+1)];diff=values.copy();coeff=[diff[0]]
    for r in range(1,p+1):
        diff=[(diff[i+1]-diff[i])/(nodes[i+r]-nodes[i]) for i in range(len(diff)-1)];coeff.append(diff[0])
    out=[F(0)]*(p+1);basis=[F(1)]
    for i,c in enumerate(coeff):
        for j,b in enumerate(basis):out[j]+=c*b
        if i<p:
            following=[F(0)]*(len(basis)+1)
            for j,b in enumerate(basis):following[j]-=nodes[i]*b;following[j+1]+=b
            basis=following
    return out

def original_power(s,ku,kv):
    pu,pv=s['degreeU'],s['degreeV'];a,b=map(q,s['knotsU'][ku:ku+2]);c,d=map(q,s['knotsV'][kv:kv+2]);samples=[]
    for i in range(pu+1):samples.append([evaluate_h(s,ku,kv,a+(b-a)*F(i,pu),c+(d-c)*F(j,pv)) for j in range(pv+1)])
    out=[]
    for axis in range(4):
        up=[interpolate([samples[i][j][axis] for i in range(pu+1)],pu) for j in range(pv+1)];power={}
        for i in range(pu+1):
            column=interpolate([up[j][i] for j in range(pv+1)],pv)
            for j,x in enumerate(column):
                if x:power[i,j]=x
        out.append(power)
    return out,(b-a,d-c)

def stored_power(s):
    pu,pv=s['degreeU'],s['degreeV'];out=[{} for _ in range(4)]
    for i in range(pu+1):
        for j in range(pv+1):
            weight=q(s['weights'][i][j]);h=[q(x)*weight for x in s['controlPoints'][i][j]]+[weight]
            for a in range(pu-i+1):
                for b in range(pv-j+1):
                    factor=comb(pu,i)*comb(pu-i,a)*comb(pv,j)*comb(pv-j,b)*(-1)**(a+b)
                    for axis,x in enumerate(h):
                        if x:out[axis]=add(out[axis],{(i+a,j+b):x*factor})
    return out

def verify(record):
    try:
        s=record['source'];axis={'u':0,'v':1}[record['axis']];nu=len(s['controlPoints']);nv=len(s['controlPoints'][0]);us=[i for i in range(s['degreeU'],nu) if s['knotsU'][i]<s['knotsU'][i+1]];vs=[j for j in range(s['degreeV'],nv) if s['knotsV'][j]<s['knotsV'][j+1]]
        if len(record['patches'])!=len(us)*len(vs):return False
        for patch,(ku,kv) in zip(record['patches'],[(i,j) for i in us for j in vs]):
            pu,pv=patch['degreeU'],patch['degreeV'];a,b=s['knotsU'][ku:ku+2];c,d=s['knotsV'][kv:kv+2]
            if not(1<=pu<=25 and 1<=pv<=25) or patch['periodicU'] or patch['periodicV']:return False
            if patch['knotsU']!=[a]*(pu+1)+[b]*(pu+1) or patch['knotsV']!=[c]*(pv+1)+[d]*(pv+1):return False
            if len(patch['controlPoints'])!=pu+1 or len(patch['weights'])!=pu+1:return False
            if any(len(row)!=pv+1 for row in patch['controlPoints']+patch['weights']) or any(len(p)!=3 for row in patch['controlPoints'] for p in row) or any(q(w)<=0 for row in patch['weights'] for w in row):return False
            h,widths=original_power(s,ku,kv);actual=stored_power(patch);w=h[3];dw=partial(w,axis,widths[axis]);squared=mul(w,w)
            for coordinate in range(3):
                target=add(mul(partial(h[coordinate],axis,widths[axis]),w),{k:-x for k,x in mul(h[coordinate],dw).items()})
                if mul(actual[coordinate],squared)!=mul(target,actual[3]):return False
        return True
    except (KeyError,ValueError,TypeError,IndexError,ZeroDivisionError,OverflowError):return False

def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-surface-derivative-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-surface-derivative-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))

if __name__=='__main__':
    records=rows(sys.argv[1]);assert len(records)==17 and all(verify(r) for r in records)
    if len(sys.argv)>2:assert records==rows(sys.argv[2])
    seed=next(r for r in records if r['axis']=='u' and r['source']['weights']==[[1.,2.],[2.,8.]]);multi=next(r for r in records if len(r['patches'])>1);bad=[]
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints'][0][0][0]=nextafter(r['patches'][0]['controlPoints'][0][0][0],inf);bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['weights'][1]=[w*2 for w in r['patches'][0]['weights'][1]];bad.append(r)
    r=copy.deepcopy(seed);r['axis']='v';bad.append(r)
    r=copy.deepcopy(seed);r['source']['knotsU']=[x if x<6 else x+1 for x in r['source']['knotsU']];bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints']=[[[4*x for x in p] for p in row] for row in r['patches'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(multi);r['patches'].pop();bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['knotsV'][-1]+=1;bad.append(r)
    r=copy.deepcopy(seed);r['source']['weights'][0][0]*=2;bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['periodicU']=True;bad.append(r)
    r=copy.deepcopy(multi);r['patches']=r['patches'][::-1];bad.append(r)
    assert all(not verify(r) for r in bad)
    print(json.dumps({'acceptedDefinitions':len(records),'patches':sum(len(r['patches']) for r in records),'negativeControls':len(bad),'definitionsSha256':hashlib.sha256(json.dumps(records,sort_keys=True).encode()).hexdigest(),'scope':'Exact homogeneous de Boor values reconstructed by degree-bounded rational tensor interpolation, then full bivariate derivative cross-identities against actual output bits and original absolute UV clocks/cell cover. No numerical sample certification, topology, JSON/WASM/editor claim.'},indent=2))
