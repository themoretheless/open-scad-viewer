#!/usr/bin/env python3
"""Independent exact polynomial meridian integral for rounded annular STEP QA.

The design uses quintic corners with endpoint speed 5*extent/4 and zero
second endpoint jets. V = pi * integral(r(t)^2 * z'(t), t) around the CCW
meridian. Fractions retain the authored binary64 dimensions exactly.
"""
import argparse
from fractions import Fraction as F
import json
from math import comb, pi
from pathlib import Path

def power(values):
    n=len(values)-1
    out=[F(0) for _ in values]
    for i,p in enumerate(values):
        for j in range(n-i+1):
            out[i+j]+=p*comb(n,i)*comb(n-i,j)*(-1)**j
    return out

def multiply(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):out[i+j]+=x*y
    return out

def volume(parameters):
    r,ri,h,e=(F.from_float(float(parameters[k])) for k in ['outer','inner','height','extent'])
    extents=[F.from_float(float(x)) for x in parameters.get('extents',[float(e)]*4)]
    corners=[(r,F(0)),(r,h),(ri,h),(ri,F(0))]
    directions=[(0,1),(-1,0),(0,-1),(1,0)]
    integral=F(0)
    for i,c in enumerate(corners):
        e=extents[i]
        incoming,outgoing=directions[(i-1)%4],directions[i]
        a=tuple(c[k]-e*incoming[k] for k in range(2))
        b=tuple(c[k]+e*outgoing[k] for k in range(2))
        controls=[a,tuple(a[k]+e*incoming[k]/4 for k in range(2)),tuple(a[k]+e*incoming[k]/2 for k in range(2)),
                  tuple(b[k]-e*outgoing[k]/2 for k in range(2)),tuple(b[k]-e*outgoing[k]/4 for k in range(2)),b]
        stop=tuple(corners[(i+1)%4][k]-extents[(i+1)%4]*outgoing[k] for k in range(2))
        for points in [controls,[b,stop]]:
            radial=power([p[0] for p in points]);height=power([p[1] for p in points])
            dz=[v*i for i,v in enumerate(height)][1:]
            integrand=multiply(multiply(radial,radial),dz)
            integral+=sum((v/F(i+1) for i,v in enumerate(integrand)),F(0))
    assert integral>0
    return integral,float(integral)*pi

def main(directory):
    native=json.loads((directory/'native.json').read_text())
    parts=[]
    for case in native['cases']:
        assert case['profileG2'] and case['stationG2'] and case['globalProven'] and case['allPairsClassified']
        assert case['coveredPairs']==case['totalPairs'] and case['nextPair'] is None
        integral,mass=volume(case['parameters']);r=case['parameters']['outer'];h=case['parameters']['height']
        scale=case['parameters'].get('scale',[1,1,1]);mass*=scale[0]*scale[1]*scale[2]
        parts.append({'name':case['name'],'file':case['file'],'sha256':case['sha256'],
                      'referenceMeridianIntegral':{'numerator':str(integral.numerator),'denominator':str(integral.denominator)},
                      'expected':{'volumeMm3':mass,'boundsMm':[[-r*scale[0],-r*scale[1],0],[r*scale[0],r*scale[1],h*scale[2]]]}})
    (directory/'manifest.json').write_text(json.dumps({'schema':'cad-roadmap-step/1','units':'mm','toleranceMm':1e-6,
        'relativeVolumeTolerance':1e-8,'reference':'independent exact Fraction polynomial meridian integral', 'parts':parts},indent=2)+'\n')

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);main(parser.parse_args().directory)
