"""Actual-output tensor Cox-de Boor / Fraction ellipsoid qualification."""
import ast
import copy
import hashlib
import json
import math
import runpy
import sys
from fractions import Fraction as F
from pathlib import Path

source=Path('docs/qualification/nurbs-exact-surface-knot-insertion-2026-11-02/exact_oracle.py')
tree=ast.parse(source.read_text())
tree.body=[n for n in tree.body if isinstance(n,(ast.Import,ast.ImportFrom,ast.FunctionDef))]
scope={}
exec(compile(tree,str(source),'exec'),scope)
homogeneous,product=scope['homogeneous'],scope['product']
conic=runpy.run_path('docs/qualification/nurbs-exact-conics-2026-11-03/exact_oracle.py')
circle={'degree':2,'knots':[0.,0.,0.,.25,.25,.5,.5,.75,.75,1.,1.,1.],
        'controlPoints':[[1.,0.,0.],[1.,1.,0.],[0.,1.,0.],[-1.,1.,0.],[-1.,0.,0.],[-1.,-1.,0.],[0.,-1.,0.],[1.,-1.,0.],[1.,0.,0.]],
        'weights':[1.,1.,2.,1.,1.,1.,2.,1.,1.]}
assert conic['verify']({'center':[0.,0.,0.],'axisU':[1.,0.,0.],'axisV':[0.,1.,0.],'curve':circle})
# The first half is the upper semicircle; positive winding and these canonical
# nonnegative Y controls prove pole-to-pole latitude coverage with R>=0.
assert all(p[1]>=0 for p in circle['controlPoints'][:5])

def add(*polys):
    result={}
    for p in polys:
        for k,v in p.items():result[k]=result.get(k,F(0))+v
    return {k:v for k,v in result.items() if v}

def scale(p,c):
    return {k:v*c for k,v in p.items() if v*c}

def outer(a,b):
    return {(i,j):x*y for i,x in enumerate(a) for j,y in enumerate(b) if x*y}

def cross(a,b):
    return [a[i]*b[j]-a[j]*b[i] for i,j in [(1,2),(2,0),(0,1)]]

def dot(a,b):
    return sum(x*y for x,y in zip(a,b))

def verify(row):
    s=row['surface']
    if (s['degreeU'],s['degreeV'])!=(2,2) or s['knotsU']!=circle['knots'] or s['knotsV']!=[0.,0.,0.,.5,.5,1.,1.,1.]:return False
    if len(s['weights'])!=9 or any(len(r)!=5 or any(w<=0 for w in r) for r in s['weights']):return False
    c=list(map(F,row['center']));a,b,d=(list(map(F,v)) for v in row['axes'])
    cofactors=[cross(b,d),cross(d,a),cross(a,b)];det=dot(a,cofactors[0])
    if not det:return False
    for i in range(4):
        uc=(F(i,4),F(i+1,4))
        xl,yl,_,wl=conic['channels'](circle,*uc)
        for j in range(2):
            vc=(F(j,2),F(j+1,2))
            xm,ym,_,wm=conic['channels'](circle,vc[0]/2,vc[1]/2)
            xm,ym,wm=([x/F(2)**k for k,x in enumerate(p)] for p in [xm,ym,wm])
            *h,w=homogeneous(s,uc,vc)
            t=[add(h[k],scale(w,-c[k])) for k in range(3)]
            projected=[add(*(scale(t[k],cofactor[k]) for k in range(3))) for cofactor in cofactors]
            denominator=scale(w,det)
            if add(*(product(p,p) for p in projected),scale(product(denominator,denominator),-1)):return False
            expected=[outer(xl,ym),outer(yl,ym),scale(outer(wl,xm),-1)]
            expected_w=outer(wl,wm)
            if any(add(product(actual,expected_w),scale(product(reference,denominator),-1)) for actual,reference in zip(projected,expected)):return False
    # Exact source-affine cardinal pole incidence, seam and both collapsed rows.
    for j in range(5):
        if s['controlPoints'][0][j]!=s['controlPoints'][8][j]:return False
    for j,sign in [(0,-1),(4,1)]:
        expected=[c[k]+sign*d[k] for k in range(3)]
        if any(list(map(F,r[j]))!=expected for r in s['controlPoints']):return False
    return True

def rows(path):
    decoder=json.JSONDecoder()
    out=[decoder.raw_decode(line.split('exact-quadric-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-quadric-oracle=' in line]
    return sorted(out,key=lambda r:json.dumps(r,sort_keys=True))

if __name__=='__main__':
    a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert a==b and len(a)==5
    assert all(verify(r) for r in a)
    r=next(r for r in a if r['center']==[0.,0.,0.] and r['axes']==[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]])
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][1][1][0]=math.nextafter(1.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][4][0][0]=math.nextafter(0.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r)
    for weights in wrong['surface']['weights']:weights[2]*=2
    assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['knotsV'][3:5]=[.25,.25];assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['axes'][0][0]=2.;assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['axes'][2]=[1.,1.,0.];assert not verify(wrong)
    scaled=copy.deepcopy(r)
    for weights in scaled['surface']['weights']:
        for j in range(5):weights[j]*=2
    assert verify(scaled)
    print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':6,'globalGaugeControl':True,
        'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),
        'tensorBasisOracleSha256':hashlib.sha256(source.read_bytes()).hexdigest(),
        'scope':'Exact binary64 Fraction tensor coefficients on eight cells: affine quadric identity, canonical rational product parameterization, full circle/upper-semicircle coverage, seam and collapsed poles. No sampling, topology, evaluator-bit or application claim.'},indent=2))
