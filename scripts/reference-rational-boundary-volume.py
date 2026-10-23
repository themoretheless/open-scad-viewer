"""Independent numerical volume oracle for full rational Bezier walls/planar caps.

Uses divergence theorem and Gauss-Legendre quadrature on authored binary64
poles. Does not call kernel evaluators or STEP/OCCT. Convergence is numerical
validation, not an interval certificate or a global embedding proof.
"""
import json
import math
import sys
from pathlib import Path
import numpy as np


def basis(degree, t):
    return np.stack([math.comb(degree,i)*t**i*(1-t)**(degree-i) for i in range(degree+1)],axis=-1)


def rule(order):
    nodes, weights=np.polynomial.legendre.leggauss(order)
    return (nodes+1)/2,weights/2


def homogeneous(points,weights):
    p=np.asarray(points,dtype=float);w=np.asarray(weights,dtype=float)
    assert np.isfinite(p).all() and np.isfinite(w).all() and (w>0).all()
    return np.concatenate((p*w[...,None],w[...,None]),axis=-1)


def surface_flux(s,order):
    p,q=s['degreeU'],s['degreeV']
    assert 1<=p<=6 and 1<=q<=6
    assert s['knotsU']==[0]*(p+1)+[1]*(p+1)
    assert s['knotsV']==[0]*(q+1)+[1]*(q+1)
    assert not s.get('periodicU',False) and not s.get('periodicV',False)
    h=homogeneous(s['controlPoints'],s['weights'])
    assert h.shape==(p+1,q+1,4)
    t,w=rule(order);bu,bv=basis(p,t),basis(q,t)
    value=np.einsum('ai,bj,ijc->abc',bu,bv,h)
    du=np.einsum('ai,bj,ijc->abc',basis(p-1,t),bv,p*np.diff(h,axis=0))
    dv=np.einsum('ai,bj,ijc->abc',bu,basis(q-1,t),q*np.diff(h,axis=1))
    point=value[...,:3]/value[...,3,None]
    ru=(du[...,:3]-point*du[...,3,None])/value[...,3,None]
    rv=(dv[...,:3]-point*dv[...,3,None])/value[...,3,None]
    integrand=np.sum(point*np.cross(ru,rv),axis=-1)/3
    return float(np.einsum('a,b,ab->',w,w,integrand))


def cap_flux(curves,order):
    t,w=rule(order);area=np.zeros(3);points=[];start=previous=None
    for curve in curves:
        p=curve['degree'];assert 1<=p<=6
        assert curve['knots']==[0]*(p+1)+[1]*(p+1) and not curve.get('periodic',False)
        poles=np.asarray(curve['controlPoints'],dtype=float)
        if start is None:start=poles[0].copy()
        else:assert np.array_equal(previous,poles[0]),'Open cap contour'
        previous=poles[-1].copy()
        if np.array_equal(previous,start):start=previous=None
        points.extend(poles)
        h=homogeneous(poles,curve['weights'])
        value=basis(p,t)@h;derivative=basis(p-1,t)@(p*np.diff(h,axis=0))
        point=value[:,:3]/value[:,3,None]
        tangent=(derivative[:,:3]-point*derivative[:,3,None])/value[:,3,None]
        area+=np.einsum('a,ac->c',w,np.cross(point,tangent))/2
    assert start is None and points,'Open/empty cap contour'
    points=np.asarray(points);origin=points[0];normal=area/np.linalg.norm(area)
    assert np.max(np.abs((points-origin)@normal))<=1e-12*max(1.,np.max(np.abs(points))),'Nonplanar cap'
    return float(origin@area/3)


def volume(case,order):
    walls=case['wallSurfaces'];reverse=case['wallFaceReversed']
    assert len(walls)==len(reverse) and all(type(r) is bool for r in reverse)
    return math.fsum([(-1 if r else 1)*surface_flux(s,order) for s,r in zip(walls,reverse)]+[cap_flux(c,order) for c in case['outwardCapContours']])


def main():
    path=Path(sys.argv[1])/'manifest.json';manifest=json.loads(path.read_text())
    for case in manifest['cases']:
        if case.get('volumeReferenceMethod')!='rational-boundary-gauss-reference':continue
        orders=[16,32,64];values=[volume(case,n) for n in orders]
        threshold=1e-12*max(1.,abs(values[-1]))
        assert all(math.isfinite(v) and v>0 for v in values)
        assert max(abs(values[i+1]-values[i]) for i in range(2))<=threshold,'Unconverged rational volume'
        case['expectedVolume']=values[-1]
        case['volumeReference']={'method':case['volumeReferenceMethod'],'orders':orders,'values':values,'agreementThreshold':threshold,
            'scope':'independent rational boundary divergence quadrature; numerical convergence, not interval-certified'}
        print(case['file'],values[-1],values)
    staged = path.with_suffix('.json.writing')
    with staged.open('w') as output:
        json.dump(manifest, output, separators=(',', ':'))
        output.write('\n')
    staged.replace(path)


def self_test():
    # Analytic annular cylinders at two translations exercise rational weights,
    # inner-loop orientation and cancellation between cap/wall fluxes.
    signs=[[(1,0),(1,1),(0,1)],[(0,1),(-1,1),(-1,0)],
           [(-1,0),(-1,-1),(0,-1)],[(0,-1),(1,-1),(1,0)]]
    for z in [0.,7.]:
        walls=[];caps=[[],[]]
        for radius,inner in [(1.,False),(.5,True)]:
            arcs=[[[radius*x,radius*y,z] for x,y in arc] for arc in signs]
            if inner:arcs=[list(reversed(arc)) for arc in reversed(arcs)]
            for arc in arcs:
                top=[[x,y,z+1] for x,y,_ in arc]
                walls.append(dict(degreeU=2,degreeV=1,knotsU=[0,0,0,1,1,1],knotsV=[0,0,1,1],
                    controlPoints=[[a,b] for a,b in zip(arc,top)],weights=[[1,1],[math.sqrt(.5)]*2,[1,1]]))
                caps[1].append(dict(degree=2,knots=[0,0,0,1,1,1],controlPoints=top,weights=[1,math.sqrt(.5),1]))
            caps[0].extend(dict(degree=2,knots=[0,0,0,1,1,1],controlPoints=list(reversed(arc)),weights=[1,math.sqrt(.5),1]) for arc in reversed(arcs))
        case=dict(wallSurfaces=walls,wallFaceReversed=[False]*8,outwardCapContours=caps)
        for order in [16,32,64]:assert abs(volume(case,order)-.75*math.pi)<1e-12
        bad=json.loads(json.dumps(case));bad['outwardCapContours'][1][0]['controlPoints'][1][2]+=.01
        try:volume(bad,16)
        except AssertionError:pass
        else:raise AssertionError('Nonplanar cap accepted')
    print('Rational annular cylinder and translated cylinder: 3*pi/4; nonplanar cap refused')


if __name__=='__main__':
    if sys.argv[1:]==['--self-test']:self_test()
    else:main()
