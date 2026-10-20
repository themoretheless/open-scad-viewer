#!/usr/bin/env python3
"""Independent analytic checks for the saved quadratic side-root control.

Endpoint enclosures and sampled root-domain containment supplement native
whole-product and refinement proofs. No saved side-curve or new solid claim.
"""
import hashlib,json,sys
from decimal import Decimal as D,localcontext
from pathlib import Path
root=Path(sys.argv[1]);m=json.loads((root/'manifest.json').read_text())
source=json.loads((root/'source.json').read_text());cutter=json.loads((root/'cutter.json').read_text())
assert m['cells']<=100000 and m['guideEvaluations']<=4096 and len(m['sides'])==2
c=cutter['faces'][0]['surface']
assert c['degreeU']==c['degreeV']==2 and c['knotsU']==c['knotsV']==[0,0,0,1,1,1]
assert all(w==1 for row in c['weights'] for w in row)
assert all(p[:2]==[-1+i,-1+j] for i,row in enumerate(c['controlPoints']) for j,p in enumerate(row))
reports=[];maximum_width=0.;samples=0
with localcontext() as ctx:
    ctx.prec=70;exact=lambda f:D.from_float(float(f))
    for side in m['sides']:
        s=source['faces'][side['sourceFace']]['surface']
        assert s['degreeU']==s['degreeV']==1 and s['knotsU']==s['knotsV']==[0,0,1,1]
        assert all(w==1 for row in s['weights'] for w in row)
        cp=s['controlPoints'];x=exact(cp[0][0][0]);ys=[exact(cp[i][0][1]) for i in range(2)];zs=[exact(cp[0][j][2]) for j in range(2)]
        assert all(list(map(exact,cp[i][j]))==[x,ys[i],zs[j]] for i in range(2) for j in range(2))
        cu=(x+1)/2;weights=[(1-cu)**2,2*cu*(1-cu),cu**2]
        z=[sum(weights[i]*exact(c['controlPoints'][i][j][2]) for i in range(3)) for j in range(3)]
        a=z[0]-2*z[1]+z[2];b=2*(z[1]-z[0]);cc=z[0]-zs[1]
        assert a>0;discriminant=b*b-4*a*cc;assert discriminant>0
        roots=[(-b-discriminant.sqrt())/(2*a),(-b+discriminant.sqrt())/(2*a)]
        ends=[]
        for box in side['surfaceBoundaryRootBounds']:
            candidates=[v for v in roots if exact(box[3][0])<=v<=exact(box[3][1])];assert len(candidates)==1
            cv=candidates[0];u=(-1+2*cv-ys[0])/(ys[1]-ys[0]);point=[u,D(1),cu,cv]
            assert all(exact(lo)<=v<=exact(hi) for v,(lo,hi) in zip(point,box))
            maximum_width=max(maximum_width,max(float(exact(hi)-exact(lo)) for lo,hi in box));ends.append(point)
        lo,hi=sorted([p[0] for p in ends]);assert 0<lo<hi<1
        for i in range(129):
            u=lo+(hi-lo)*D(i)/128;y=ys[0]+u*(ys[1]-ys[0]);cv=(y+1)/2
            value=z[0]*(1-cv)**2+2*z[1]*cv*(1-cv)+z[2]*cv**2
            v=(value-zs[0])/(zs[1]-zs[0])
            if i in (0,128):v=D(1) # Exact authored endpoint plane, independent quadratic root.
            point=[u,v,cu,cv]
            assert any(all(exact(bounds[0])<=value<=exact(bounds[1]) for value,bounds in zip(point,t['domain'])) for t in side['carriers'])
            samples+=1
        assert {t['axis'] for t in side['carriers']}=={'FirstU','FirstV'}
        reports.append(dict(sourceFace=side['sourceFace'],endpointRoots=2,rootSamples=129,carrierCount=len(side['carriers'])))
report=dict(passed=True,sides=reports,rootSamples=samples,maximumEndpointParameterWidth=maximum_width,
            sourceSha256=hashlib.sha256((root/'source.json').read_bytes()).hexdigest(),cutterSha256=hashlib.sha256((root/'cutter.json').read_bytes()).hexdigest(),
            scope='actual quadratic-control roots and carrier samples; native whole-root proofs remain primary; no saved side curves or changed solid')
assert maximum_width<1e-9
(root/'independent-side-roots.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
