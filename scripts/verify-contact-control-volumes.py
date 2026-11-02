#!/usr/bin/env python3
"""Exact Bernstein integrals for the two declared controls versus OCCT STEP volume."""
import argparse
import hashlib
import json
from fractions import Fraction as F
from math import comb
from pathlib import Path


def product_integral(a,b):
    n,m=len(a)-1,len(b)-1
    return sum(a[i]*b[j]*F(comb(n,i)*comb(m,j),comb(n+m,i+j)*(n+m+1))
               for i in range(n+1) for j in range(m+1))



def assert_bezier(surface):
    for axis,count in [('U',len(surface['controlPoints'])),('V',len(surface['controlPoints'][0]))]:
        degree=surface['degree'+axis]
        knots=surface['knots'+axis]
        assert count==degree+1 and len(knots)==2*count
        assert knots[:count]==[knots[0]]*count and knots[count:]==[knots[-1]]*count
        assert knots[0]<knots[-1]


def pyramid(root,model):
    data=json.loads((root/'construction.json').read_text())
    assert len(model['faces'])==10
    height,width=F(19),F(3)
    volume=4*(height-2*width)**3/6
    for arm,index in enumerate(data['stripFaces']):
        s=model['faces'][index]['surface']
        assert_bezier(s)
        assert s['degreeU']==6 and s['degreeV']==1
        assert all(w==1 for row in s['weights'] for w in row)
        poles=[[[F(v) for v in p] for p in row] for row in s['controlPoints']]
        cross,free=(0,1) if arm%2==0 else (1,0)
        sign=-1 if arm in [0,3] else 1
        values=[]
        for i,(near,far) in enumerate(poles):
            assert near[cross]==far[cross]==-width+i
            assert near[free]==sign*width and far[2]==0
            assert far[free]==sign*(width+near[2])
            values.append(near[2])
        volume+=width*product_integral(values,values)
    center=model['faces'][data['centralFace']]['surface']
    assert_bezier(center)
    assert center['degreeU']==center['degreeV']==6
    assert all(w==1 for row in center['weights'] for w in row)
    for i,row in enumerate(center['controlPoints']):
        for j,p in enumerate(row):
            assert p[0]==-3+i and p[1]==-3+j
    volume+=F(36,49)*sum(F(p[2]) for row in center['controlPoints'] for p in row)
    return volume,2*height**3/3


def prism(root,model):
    assert len(model['faces'])==7
    candidates=[f['surface'] for f in model['faces'] if f['surface']['degreeU']==5]
    assert len(candidates)==1
    s=candidates[0]
    assert_bezier(s)
    assert s['degreeV']==2 and len(s['controlPoints'])==6
    assert all(row==s['weights'][0] for row in s['weights'])
    assert s['weights'][0]==[1.,0.875,1.]
    cp=[[[F(x) for x in p] for p in row] for row in s['controlPoints']]
    for v in range(3):
        for u in range(6):
            assert cp[u][v][2]==cp[0][v][2]
            for k in range(2):
                assert cp[u][v][k]-cp[0][v][k]==cp[u][0][k]-cp[0][0][k]
    x=[row[0][0] for row in cp]
    y=[row[0][1] for row in cp]
    assert (x[0],y[0],x[-1],y[-1])==(1,0,0,1)
    dx=[5*(x[i+1]-x[i]) for i in range(5)]
    dy=[5*(y[i+1]-y[i]) for i in range(5)]
    removed=(product_integral(x,dy)-product_integral(y,dx))/2
    assert 0<removed<1
    span=cp[0][-1][2]-cp[0][0][2]
    assert span==10
    return (16-removed)*span,F(160)


def verify(root,kind):
    body=json.loads((root/'body.json').read_text())
    expected,stock=(pyramid if kind=='pyramid' else prism)(root,body['model'])
    occt=json.loads((root/'candidate.occt.json').read_text())
    sha=hashlib.sha256((root/'candidate.step').read_bytes()).hexdigest()
    assert sha==occt['inputSha256']
    error=abs(occt['volumeEstimateMm3']-float(expected))
    report=dict(passed=error<=1e-5 and 0<expected<stock,stepSha256=sha,
        expectedVolumeMm3=float(expected),exactNumerator=str(expected.numerator),
        exactDenominator=str(expected.denominator),occtVolumeMm3=occt['volumeEstimateMm3'],
        absoluteErrorMm3=error,numericalToleranceMm3=1e-5,sourceVolumeMm3=float(stock),
        scope='Exact Bernstein integral of the declared control family versus independent numerical STEP mass properties; containment and global embedding are separate native gates.')
    (root/'independent-volume.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
    return 0 if report['passed'] else 1


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    parser.add_argument('--kind',choices=['prism','pyramid'],required=True)
    args=parser.parse_args()
    raise SystemExit(verify(args.root,args.kind))
