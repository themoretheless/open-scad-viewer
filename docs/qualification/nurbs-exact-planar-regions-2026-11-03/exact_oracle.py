"""Exact plane, radial/angle identities and disk/annulus coverage of actual output."""
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
homogeneous,product,add,scale,outer,cross=(support[k] for k in ['homogeneous','product','add','scale','outer','cross'])

def verify(row,require_clock=True,require_plane=True):
    s=row['surface'];inner,outer_radius=map(F,row['radii'])
    if not 0<=inner<outer_radius:return False
    if row['kind']!=('filledDisk' if not inner else 'annulus'):return False
    if (s['degreeU'],s['degreeV'])!=(2,1) or s['knotsU']!=circle['knots'] or s['knotsV']!=[0.,0.,1.,1.]:return False
    if len(s['weights'])!=9 or any(len(r)!=2 or any(w<=0 for w in r) for r in s['weights']):return False
    c=list(map(F,row['center']));a,b=(list(map(F,v)) for v in row['axes'])
    normal=cross(a,b);choices=[(k,i,j) for k,(i,j) in enumerate([(1,2),(2,0),(0,1)]) if normal[k]]
    if not choices:return False
    k,i,j=choices[0];det=normal[k];radius=[inner,outer_radius-inner]
    for cell in range(4):
        uc=(F(cell,4),F(cell+1,4));vc=(F(0),F(1))
        xc,yc,_,wc=conic['channels'](circle,*uc)
        *h,w=homogeneous(s,uc,vc)
        t=[add(h[q],scale(w,-c[q])) for q in range(3)]
        if require_plane and add(*(scale(t[q],normal[q]) for q in range(3))):return False
        x=add(scale(t[i],b[j]),scale(t[j],-b[i]))
        y=add(scale(t[j],a[i]),scale(t[i],-a[j]));denominator=scale(w,det)
        rr=outer([1],radius)
        if add(product(x,x),product(y,y),scale(product(product(denominator,denominator),product(rr,rr)),-1)):return False
        expected=[outer(xc,radius),outer(yc,radius)];expected_w=outer(wc,[1])
        if require_clock and any(add(product(actual,expected_w),scale(product(reference,denominator),-1)) for actual,reference in zip([x,y],expected)):return False
    if s['controlPoints'][0]!=s['controlPoints'][8] or s['weights'][0]!=s['weights'][8]:return False
    if not inner and any(list(map(F,r[0]))!=c for r in s['controlPoints']):return False
    # Only the default mode proves the entire source-defined region:
    # plane, full canonical circle, linear radius, injectivity modulo seam/center.
    # Partial modes below ONLY demonstrate weaker checks missing damage.
    return True

def rows(path):
    decoder=json.JSONDecoder()
    out=[decoder.raw_decode(line.split('exact-planar-region-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-planar-region-oracle=' in line]
    return sorted(out,key=lambda r:json.dumps(r,sort_keys=True))

def outside_domain(row,radii):
    r=copy.deepcopy(row);r['radii']=radii
    for i,p in enumerate(circle['controlPoints']):
        for j in range(2):r['surface']['controlPoints'][i][j]=[radii[j]*p[0],radii[j]*p[1],0.]
    return r

if __name__=='__main__':
    a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert a==b and len(a)==10
    assert all(verify(r) for r in a)
    r=next(r for r in a if r['center']==[0.,0.,0.] and r['axes']==[[1.,0.,0.],[0.,1.,0.]] and r['radii']==[0.,1.])
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][1][1][2]=math.nextafter(0.,math.inf)
    assert verify(wrong,require_plane=False) and not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][1][1][0]=math.nextafter(1.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][3][0][0]=math.nextafter(0.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r)
    for weights in wrong['surface']['weights']:weights[1]*=2
    assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['knotsV']=[0.,0.,2.,2.];assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['axes'][0][0]=2.;assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['kind']='annulus';assert not verify(wrong)
    for radii in [[0.,0.],[1.,1.],[2.,1.],[-1.,1.]]:assert not verify(outside_domain(r,radii))
    rotated=copy.deepcopy(r)
    for point in (p for row in rotated['surface']['controlPoints'] for p in row):point[0],point[1]=-point[1],point[0]
    assert verify(rotated,require_clock=False) and not verify(rotated)
    scaled=copy.deepcopy(r)
    for weights in scaled['surface']['weights']:
        for j in range(2):weights[j]*=2
    assert verify(scaled)
    print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':12,'globalGaugeControl':True,
        'minorAloneMissesOutOfPlaneDamage':True,'radialIdentityAloneMissesAngularPhaseChange':True,
        'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),
        'supportSourceSha256':hashlib.sha256(support_path.read_bytes()).hexdigest(),
        'scope':'Exact Fraction plane and canonical angular/linear-radius identities on all4 cells; full disk/annulus coverage, seam, boundary/center classification and nonzero exact plane minor. No sampling, B-rep/shell/solid, numeric-normal/evaluator-bit or application claim.'},indent=2))
