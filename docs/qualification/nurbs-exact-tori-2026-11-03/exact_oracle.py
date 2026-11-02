"""Independent whole-torus quartic and parameter identities on actual output."""
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

def verify(row):
    s=row['surface'];major,minor=F(row['major']),F(row['minor'])
    if not major>minor>0:return False
    if (s['degreeU'],s['degreeV'])!=(2,2) or s['knotsU']!=circle['knots'] or s['knotsV']!=circle['knots']:return False
    if len(s['weights'])!=9 or any(len(r)!=9 or any(w<=0 for w in r) for r in s['weights']):return False
    c=list(map(F,row['center']));a,b,d=(list(map(F,v)) for v in row['axes'])
    cofactors=[cross(b,d),cross(d,a),cross(a,b)];det=dot(a,cofactors[0])
    if not det:return False
    for i in range(4):
        uc=(F(i,4),F(i+1,4))
        xl,yl,_,wl=conic['channels'](circle,*uc)
        for j in range(4):
            vc=(F(j,4),F(j+1,4))
            xm,ym,_,wm=conic['channels'](circle,*vc)
            *h,w=homogeneous(s,uc,vc)
            t=[add(h[k],scale(w,-c[k])) for k in range(3)]
            x,y,z=[add(*(scale(t[k],cofactor[k]) for k in range(3))) for cofactor in cofactors]
            denominator=scale(w,det)
            xy=add(product(x,x),product(y,y));dd=product(denominator,denominator)
            total=add(xy,product(z,z),scale(dd,major*major-minor*minor))
            if add(product(total,total),scale(product(dd,xy),-4*major*major)):return False
            radial=conic['add']([major*q for q in wm],[minor*q for q in xm])
            expected=[outer(xl,radial),outer(yl,radial),scale(outer(wl,ym),minor)]
            expected_w=outer(wl,wm)
            if any(add(product(actual,expected_w),scale(product(reference,denominator),-1)) for actual,reference in zip([x,y,z],expected)):return False
    if s['controlPoints'][0]!=s['controlPoints'][8]:return False
    if any(r[0]!=r[8] for r in s['controlPoints']):return False
    # The independently proved full circle factors, exact product identity,
    # R>r>0 and det!=0 prove a full embedded torus modulo its two seams.
    return True

def rows(path):
    decoder=json.JSONDecoder()
    out=[decoder.raw_decode(line.split('exact-torus-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-torus-oracle=' in line]
    return sorted(out,key=lambda r:json.dumps(r,sort_keys=True))

def outside_domain(row,major,minor):
    r=copy.deepcopy(row);r['major']=major;r['minor']=minor
    # Correct literal canonical horn/spindle/zero-tube parameterization,
    # not merely the old ring output mislabeled with different radii.
    for i,p in enumerate(circle['controlPoints']):
        for j,q in enumerate(circle['controlPoints']):
            r['surface']['controlPoints'][i][j]=[(major+minor*q[0])*p[0],(major+minor*q[0])*p[1],minor*q[1]]
    return r

if __name__=='__main__':
    a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert a==b and len(a)==6
    assert all(verify(r) for r in a)
    r=next(r for r in a if r['center']==[0.,0.,0.] and r['axes']==[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] and r['major']==2.)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][1][1][0]=math.nextafter(3.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r)
    for weights in wrong['surface']['weights']:weights[2]*=2
    assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][8][3][0]=math.nextafter(1.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['controlPoints'][3][8][2]=math.nextafter(0.,math.inf);assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['surface']['knotsV'][3:5]=[.2,.2];assert not verify(wrong)
    wrong=copy.deepcopy(r);wrong['axes'][0][0]=2.;assert not verify(wrong)
    for major,minor in [(1.,1.),(.5,1.),(1.,0.)]:assert not verify(outside_domain(r,major,minor))
    scaled=copy.deepcopy(r)
    for weights in scaled['surface']['weights']:
        for j in range(9):weights[j]*=2
    assert verify(scaled)
    print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':9,'globalGaugeControl':True,
        'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),
        'supportSourceSha256':hashlib.sha256(support_path.read_bytes()).hexdigest(),
        'scope':'Exact Fraction affine quartic and full canonical rational product parameterization on all16 tensor cells, both seams, full embedded ring-torus coverage. Genuine literal horn/spindle/zero-tube cases refused. No sampling, B-rep/solid/volume, evaluator-bit or application claim.'},indent=2))
