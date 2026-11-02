#!/usr/bin/env python3
"""Independent exact profile integration versus native and numerical STEP volume."""
import argparse,json,hashlib,math
from fractions import Fraction as F
from pathlib import Path

def add(a,b):
    out=[F(0)]*max(len(a),len(b))
    for i,v in enumerate(a):out[i]+=v
    for i,v in enumerate(b):out[i]+=v
    return out

def mul(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):out[i+j]+=x*y
    return out

def integrate(p,a,b):return sum(v*(b**(i+1)-a**(i+1))/F(i+1) for i,v in enumerate(p))

def removed(h):
    p=[F(0)]*7
    for i,v in enumerate([F(3),F(2),F(1),h,F(1),F(2),F(3)]):
        for j in range(7-i):p[i+j]+=v*math.comb(6,i)*math.comb(6-i,j)*(-1)**j
    out=F(0)
    for a,b,absolute in [(F(0),F(1,2),[F(3),F(-6)]),(F(1,2),F(1),[F(-3),F(6)])]:
        delta=add(p,[-v for v in absolute]);width=add([F(24)],[-v for v in absolute])
        out+=6*integrate(mul(delta,width),a,b)
    return out

def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args();d=args.directory
    native=json.loads((d/'report.json').read_text());occt=json.loads((d/'candidate.occt.json').read_text())
    handles=native.get('handles')
    if handles is None:
        if native.get('requestedMidRadii') is not None:raise ValueError('Prescribed case lacks original handle intent')
        handles=[0.,.25,.5]
    s=F.from_float(native['physicalScale']);hs=[F.from_float(h) for h in handles]
    source=F(10368)*s**3;expected=(F(10368)-sum(removed(h) for h in hs))*s**3
    exact_match=expected==F(native['volumeExactMm3']);error=abs(occt['volumeEstimateMm3']-float(expected))
    digest=hashlib.sha256((d/'candidate.step').read_bytes()).hexdigest()
    passed=exact_match and occt['inputSha256']==digest and occt['basicBrepValid'] and error<=1e-7 and 0<expected<source
    result=dict(passed=passed,inputSha256=digest,expectedVolumeExactMm3=str(expected),expectedVolumeMm3=float(expected),
        sourceVolumeMm3=float(source),removedVolumeMm3=float(source-expected),nativeExactMatch=exact_match,occtVolumeErrorMm3=error,
        scope='Exact independent three-profile weighted integration; OCCT volume is numerical. Not a generic B-rep containment proof.')
    (d/'independent-volume.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
    if not passed:raise SystemExit(1)
if __name__=='__main__':main()
