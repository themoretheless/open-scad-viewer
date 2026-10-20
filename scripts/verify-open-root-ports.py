#!/usr/bin/env python3
"""Independent high precision roots of the saved quadratic control surface.

This verifies four endpoint enclosures for this control. It does not qualify
B-rep edge identities, general NURBS roots, traces, arcs or solid closure.
"""
import hashlib
import json
import sys
from decimal import Decimal as D, localcontext
from pathlib import Path

root=Path(sys.argv[1])
manifest=json.loads((root/'manifest.json').read_text())
source=json.loads((root/'source.json').read_text())
cutter=json.loads((root/'cutter.json').read_text())
s=source['faces'][manifest['sourceFace']]['surface']
c=cutter['faces'][0]['surface']
assert s['degreeU']==s['degreeV']==1 and c['degreeU']==c['degreeV']==2
assert all(w==1 for row in s['weights']+c['weights'] for w in row)
assert s['knotsU']==s['knotsV']==[0,0,1,1]
assert c['knotsU']==c['knotsV']==[0,0,0,1,1,1]
assert len(s['controlPoints'])==2 and all(len(row)==2 for row in s['controlPoints'])
assert len(c['controlPoints'])==3 and all(len(row)==3 for row in c['controlPoints'])
assert all(p==[-0.5+i,-0.5+j,0.2] for i,row in enumerate(s['controlPoints']) for j,p in enumerate(row))
assert all(p[:2]==[-1+i,-1+j] for i,row in enumerate(c['controlPoints']) for j,p in enumerate(row))
checks=[]
with localcontext() as ctx:
    ctx.prec=70
    exact=lambda f:D.from_float(float(f))
    for branch in manifest['proposals']:
        for endpoint,box in enumerate(branch['surfaceBoundaryRootBounds']):
            u=exact(box[0][0]);assert box[0][0]==box[0][1] and u in (0,1)
            x=exact(s['controlPoints'][int(u)][0][0]);target=exact(s['controlPoints'][int(u)][0][2])
            cu=(x+1)/2
            weights=[(1-cu)**2,2*cu*(1-cu),cu**2]
            z=[sum(weights[i]*exact(c['controlPoints'][i][j][2]) for i in range(3)) for j in range(3)]
            a=z[0]-2*z[1]+z[2];b=2*(z[1]-z[0]);cc=z[0]-target
            discriminant=b*b-4*a*cc;assert discriminant>0
            candidates=[(-b-discriminant.sqrt())/(2*a),(-b+discriminant.sqrt())/(2*a)]
            candidates=[v for v in candidates if exact(box[3][0])<=v<=exact(box[3][1])]
            assert len(candidates)==1
            cv=candidates[0];sv=-D('0.5')+2*cv
            actual=[u,sv,cu,cv]
            assert all(exact(lo)<=v<=exact(hi) for v,(lo,hi) in zip(actual,box))
            width=max(float(exact(hi)-exact(lo)) for lo,hi in box)
            assert width<1e-9
            checks.append(dict(component=branch['component'],endpoint=endpoint,parameterWidth=width,
                               pointMm=[float(x),float(-1+2*cv),float(target)]))
report=dict(passed=True,endpointCount=len(checks),checks=checks,
            maximumParameterWidth=max(c['parameterWidth'] for c in checks),
            sourceSha256=hashlib.sha256((root/'source.json').read_bytes()).hexdigest(),
            cutterSha256=hashlib.sha256((root/'cutter.json').read_bytes()).hexdigest(),
            scope='four saved quadratic-control endpoint roots only; no B-rep ownership or solid claim')
assert report['endpointCount']==4
(root/'independent-endpoint-roots.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
