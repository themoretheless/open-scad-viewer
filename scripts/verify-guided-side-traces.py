#!/usr/bin/env python3
"""Independent Decimal sampling; supplements native full-root proofs."""
import json,sys,hashlib
from decimal import Decimal as D,localcontext
from pathlib import Path
r=Path(sys.argv[1]); m=json.loads((r/'manifest.json').read_text())
source=json.loads((r/'source.json').read_text()); cutter=json.loads((r/'cutter.json').read_text()); bound=json.loads((r/'bound-source.json').read_text())
assert m['cells']<=100000 and m['guideEvaluations']<=4096
exact=lambda x:x if isinstance(x,D) else D.from_float(float(x))
def curve(c,t):
    p=c['degree']; knots=list(map(exact,c['knots'])); cp=c['controlPoints']; w=list(map(exact,c['weights']))
    n=len(cp); k=n-1 if t==knots[n] else next(i for i in range(p,n) if knots[i]<=t<knots[i+1])
    d=[[exact(v)*w[j] for v in cp[j]]+[w[j]] for j in range(k-p,k+1)]
    for a in range(1,p+1):
        for j in range(p,a-1,-1):
            i=k-p+j; q=(t-knots[i])/(knots[i+p-a+1]-knots[i])
            d[j]=[(1-q)*x+q*y for x,y in zip(d[j-1],d[j])]
    return [v/d[p][-1] for v in d[p][:-1]]
def surface(s,u,v):
    rows=[]
    for pts,weights in zip(s['controlPoints'],s['weights']):
        rows.append(curve(dict(degree=s['degreeV'],knots=s['knotsV'],controlPoints=pts,weights=weights),v))
    # This control uses non-rational surfaces; refuse any other family.
    assert all(w==1 for row in s['weights'] for w in row)
    return curve(dict(degree=s['degreeU'],knots=s['knotsU'],controlPoints=rows,weights=[1]*len(rows)),u)
maximum=D(0); samples=0; pieces=0
with localcontext() as ctx:
    ctx.prec=70
    for side in m['sides']:
        assert side['savedSideCurvesQualified'] and side['completeOriginalProduct']
        for chain in side['savedChains']:
            previous=None
            for piece in chain['pieces']:
                c=piece['curve']; pcs=piece['pcurves']; pieces+=1
                if previous:
                    assert previous['curve']['controlPoints'][-1]==c['controlPoints'][0]
                    assert all(previous['pcurves'][k]['controlPoints'][-1]==pcs[k]['controlPoints'][0] for k in range(2))
                for i in range(65):
                    lo=exact(c['knots'][c['degree']]);hi=exact(c['knots'][len(c['controlPoints'])]); t=lo+(hi-lo)*D(i)/64
                    point=curve(c,t)
                    for k,s in enumerate([source['faces'][side['sourceFace']]['surface'],cutter['faces'][0]['surface']]):
                        pc=pcs[k];a=exact(pc['knots'][pc['degree']]);b=exact(pc['knots'][len(pc['controlPoints'])]);uv=curve(pc,a+(b-a)*D(i)/64)
                        q=surface(s,*uv); error=sum((x-y)**2 for x,y in zip(point,q)).sqrt();maximum=max(maximum,error)
                    samples+=1
                previous=piece
            for end,index in enumerate(chain['vertices']):
                piece=chain['pieces'][0 if end==0 else -1]
                assert piece['curve']['controlPoints'][0 if end==0 else -1]==bound['vertices'][index]['point']
assert maximum<D('1e-7')
report=dict(passed=True,pieces=pieces,samples=samples,maximumSampledSupportErrorMm=float(maximum),literalSharedJunctions=True,literalOwnedEndpoints=True,scope='sampled saved side curves on actual control surfaces; native continuous proofs remain primary; no changed solid')
(r/'independent-side-traces.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
