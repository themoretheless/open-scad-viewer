"""Whole exact affine cylinder/frustum/cone identities from actual Rust output."""
import copy
import hashlib
import json
import math
import runpy
import sys
from fractions import Fraction as F
from pathlib import Path

support_path=Path('docs/qualification/nurbs-exact-quadrics-2026-11-03/exact_oracle.py')
support=runpy.run_path(str(support_path))
circle,conic=(support[k] for k in ['circle','conic'])
homogeneous,product,add,scale,outer,cross,dot=(support[k] for k in ['homogeneous','product','add','scale','outer','cross','dot'])

def verify(row,require_clock=True):
    s=row['surface'];r0,r1=map(F,row['radii'])
    if min(r0,r1)<0 or max(r0,r1)<=0:return False
    collapsed=0 if not r0 else 1 if not r1 else None
    if row['collapsedEnd']!=collapsed:return False
    if (s['degreeU'],s['degreeV'])!=(2,1) or s['knotsU']!=circle['knots'] or s['knotsV']!=[0.,0.,1.,1.]:return False
    if len(s['weights'])!=9 or any(len(r)!=2 or any(w<=0 for w in r) for r in s['weights']):return False
    c=list(map(F,row['center']));a,b,d=(list(map(F,v)) for v in row['axes'])
    cofactors=[cross(b,d),cross(d,a),cross(a,b)];det=dot(a,cofactors[0])
    if not det:return False
    for i in range(4):
        uc=(F(i,4),F(i+1,4));vc=(F(0),F(1))
        xc,yc,_,wc=conic['channels'](circle,*uc)
        *h,w=homogeneous(s,uc,vc)
        t=[add(h[k],scale(w,-c[k])) for k in range(3)]
        x,y,z=[add(*(scale(t[k],cofactor[k]) for k in range(3))) for cofactor in cofactors]
        denominator=scale(w,det)
        radius=add(scale(denominator,r0),scale(z,r1-r0))
        if add(product(x,x),product(y,y),scale(product(radius,radius),-1)):return False
        expected=[outer(xc,[r0,r1-r0]),outer(yc,[r0,r1-r0]),outer(wc,[0,1])]
        expected_w=outer(wc,[1])
        if require_clock and any(add(product(actual,expected_w),scale(product(reference,denominator),-1)) for actual,reference in zip([x,y,z],expected)):return False
    if s['controlPoints'][0]!=s['controlPoints'][8] or s['weights'][0]!=s['weights'][8]:return False
    if collapsed is not None:
        expected=[c[k]+collapsed*d[k] for k in range(3)]
        if any(list(map(F,r[collapsed]))!=expected for r in s['controlPoints']):return False
    # With require_clock=True, the fresh full-circle theorem and positive
    # interior radius prove complete injective side coverage modulo seam/apex.
    # False is ONLY a negative-control check of the weaker quadric identity.
    return True

def rows(path):
    decoder=json.JSONDecoder()
    out=[decoder.raw_decode(line.split('exact-frustum-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-frustum-oracle=' in line]
    return sorted(out,key=lambda r:json.dumps(r,sort_keys=True))

def outside_domain(row,radii):
    r=copy.deepcopy(row);r['radii']=radii
    for i,p in enumerate(circle['controlPoints']):
        for j in range(2):r['surface']['controlPoints'][i][j]=[radii[j]*p[0],radii[j]*p[1],float(j)]
    return r

if __name__=='__main__':
    a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert a==b and len(a)==9
    assert all(verify(r) for r in a)
    r=next(r for r in a if r['center']==[0.,0.,0.] and r['axes']==[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] and r['radii']==[2.,1.])
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][1][1][0]=math.nextafter(1.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r)
    for weights in wrong['surface']['weights']:weights[1]*=2
    assert verify(wrong,require_clock=False),'Quadric membership alone misses this UV-clock change'
    assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][8][0][0]=math.nextafter(2.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['knotsV']=[0.,0.,2.,2.];assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['axes'][0][0]=2.;assert not verify(wrong)
    cone=next(r for r in a if r['radii']==[2.,0.]);wrong=copy.deepcopy(cone);wrong['surface']['controlPoints'][3][1][0]=math.nextafter(0.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(cone);wrong['collapsedEnd']=None;assert not verify(wrong)
    for radii in [[0.,0.],[-1.,1.],[1.,-1.]]:assert not verify(outside_domain(r,radii))
    scaled=copy.deepcopy(r)
    for weights in scaled['surface']['weights']:
        for j in range(2):weights[j]*=2
    assert verify(scaled)
    print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':10,'globalGaugeControl':True,
        'membershipAloneMissesColumnClockChange':True,
        'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),
        'supportSourceSha256':hashlib.sha256(support_path.read_bytes()).hexdigest(),
        'scope':'Exact Fraction affine quadric and original angular/linear-height parameterization on all4 cells; complete side coverage, seam and collapsed end, correct intrinsic cone-apex metadata. No sampling, caps/B-rep/solid/volume, smooth apex, evaluator-bit or application claim.'},indent=2))
