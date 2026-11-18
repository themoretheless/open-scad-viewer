"""Independent Cox-de Boor source basis and actual-output derivative identities."""
from fractions import Fraction as F
from functools import lru_cache
from math import comb,nextafter,inf
from pathlib import Path
import copy,json,sys,hashlib

def trim(a):
    while a and not a[-1]:a.pop()
    return a

def add(a,b):
    c=[F(0)]*max(len(a),len(b))
    for i,x in enumerate(a):c[i]+=x
    for i,x in enumerate(b):c[i]+=x
    return trim(c)

def mul(a,b):
    if not a or not b:return []
    c=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):c[i+j]+=x*y
    return trim(c)

def derivative(a,width):return trim([i*x/width for i,x in enumerate(a) if i])
def q(x):return F(float(x))

def source_power(s,k):
    knots=list(map(q,s['knots']));a=knots[k];width=knots[k+1]-a;p=s['degree']
    @lru_cache(None)
    def basis(i,d):
        if not d:return (F(1),) if i==k else ()
        out=[];den=knots[i+d]-knots[i]
        if den:out=mul([(a-knots[i])/den,width/den],list(basis(i,d-1)))
        den=knots[i+d+1]-knots[i+1]
        if den:out=add(out,mul([(knots[i+d+1]-a)/den,-width/den],list(basis(i+1,d-1))))
        return tuple(out)
    dim=len(s['controlPoints'][0]);h=[[] for _ in range(dim+1)]
    for i in range(k-p,k+1):
        b=list(basis(i,p));w=q(s['weights'][i])
        for axis,x in enumerate(s['controlPoints'][i]+[1.]):
            value=q(x);h[axis]=add(h[axis],[value*qv*w for qv in b])
    return h,width

def stored_power(c):
    dim=len(c['controlPoints'][0]);p=c['degree'];h=[]
    for axis in range(dim+1):
        values=[q(w)*(q(point[axis]) if axis<dim else 1) for point,w in zip(c['controlPoints'],c['weights'])];out=[F(0)]*(p+1)
        for i,x in enumerate(values):
            for j in range(p-i+1):out[i+j]+=x*comb(p,i)*comb(p-i,j)*(-1)**j
        h.append(trim(out))
    return h

def verify(r):
    try:
        s=r['source'];p=s['degree'];n=len(s['controlPoints']);dim=len(s['controlPoints'][0]);spans=[k for k in range(p,n) if s['knots'][k]<s['knots'][k+1]]
        if len(spans)!=len(r['pieces']):return False
        for k,c in zip(spans,r['pieces']):
            d=c['degree'];a,b=s['knots'][k:k+2]
            if not 1<=d<=25 or c['periodic'] or c['knots']!=[a]*(d+1)+[b]*(d+1):return False
            if len(c['controlPoints'])!=d+1 or len(c['weights'])!=d+1 or any(len(x)!=dim for x in c['controlPoints']) or any(q(w)<=0 for w in c['weights']):return False
            source,width=source_power(s,k);actual=stored_power(c);w=source[-1];dw=derivative(w,width);squared=mul(w,w)
            for axis in range(dim):
                target=add(mul(derivative(source[axis],width),w),[-x for x in mul(source[axis],dw)])
                if mul(actual[axis],squared)!=mul(target,actual[-1]):return False
        return True
    except (KeyError,ValueError,TypeError,ZeroDivisionError,IndexError):return False

def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-derivative-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-derivative-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))

if __name__=='__main__':
    records=rows(sys.argv[1]);assert len(records)==12 and all(verify(r) for r in records)
    if len(sys.argv)>2:assert records==rows(sys.argv[2])
    seed=next(r for r in records if r['source']['weights']==[1.,2.]);multi=next(r for r in records if len(r['pieces'])>1);bad=[]
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints'][0][0]=nextafter(r['pieces'][0]['controlPoints'][0][0],inf);bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['weights'][0]*=2;bad.append(r)
    r=copy.deepcopy(seed);r['source']['knots']=[x if x<2 else x+1 for x in r['source']['knots']];bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['controlPoints']=[[4*x for x in p] for p in r['pieces'][0]['controlPoints']];bad.append(r)
    r=copy.deepcopy(seed);r['pieces'][0]['periodic']=True;bad.append(r)
    r=copy.deepcopy(seed);r['source']['controlPoints'][1][0]+=1;bad.append(r)
    r=copy.deepcopy(multi);r['pieces'].pop();bad.append(r)
    r=copy.deepcopy(multi);r['pieces'][0]['knots'][-1]+=1;bad.append(r)
    assert all(not verify(r) for r in bad)
    print(json.dumps({'acceptedDefinitions':len(records),'pieces':sum(len(r['pieces']) for r in records),'negativeControls':len(bad),'definitionsSha256':hashlib.sha256(json.dumps(records,sort_keys=True).encode()).hexdigest(),'scope':'Exact full-domain polynomial derivative identities from independently reconstructed original Cox-de Boor basis, absolute clock, actual stored output bits and complete one-sided span cover. No sampling, topology, JSON/WASM/editor qualification.'},indent=2))
