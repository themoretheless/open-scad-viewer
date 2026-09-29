"""Independent rational tensor-product evaluation of exported periodic surfaces."""
import hashlib
import json
import math
from pathlib import Path
import sys

folder=Path(sys.argv[1])
browser=json.loads((folder/'periodic-surface-browser.json').read_text())

def basis(knots,degree,count,t):
    values=[float(knots[i]<=t<knots[i+1]) for i in range(len(knots)-1)]
    if t==knots[count]:
        values=[0.]*(len(knots)-1)
        span=count-1
        while knots[span]==t:
            span-=1
        values[span]=1.
    for p in range(1,degree+1):
        result=[]
        for i in range(len(values)-1):
            a,b=knots[i+p]-knots[i],knots[i+p+1]-knots[i+1]
            result.append(((t-knots[i])/a*values[i] if a else 0.)
                          +((knots[i+p+1]-t)/b*values[i+1] if b else 0.))
        values=result
    return values[:count]

def evaluate(surface,u,v):
    points,weights=surface['controlPoints'],surface['weights']
    bu=basis(surface['knotsU'],surface['degreeU'],len(points),u)
    bv=basis(surface['knotsV'],surface['degreeV'],len(points[0]),v)
    h=[0.,0.,0.,0.]
    for i,row in enumerate(points):
        for j,p in enumerate(row):
            w=bu[i]*bv[j]*weights[i][j]
            for k in range(3):
                h[k]+=w*p[k]
            h[3]+=w
    assert h[3]>0
    return [x/h[3] for x in h[:3]]

reports=[]
for item in browser['axes']:
    axis=item['axis']
    path=folder/f'surface-{axis}-committed.json'
    source=json.loads((folder/f'surface-{axis}-canceled.json').read_text())['surfaces'][0]['surface']
    target=json.loads(path.read_text())['surfaces'][0]['surface']
    assert target['periodicU'] and target['periodicV']
    for suffix,count in [('U',len(target['controlPoints'])),('V',len(target['controlPoints'][0]))]:
        degree=target['degree'+suffix]
        assert [target['knots'+suffix][degree],target['knots'+suffix][count]]==[2,6]
    maximum=0.
    for i in range(65):
        for j in range(65):
            u,v=2+4*i/64,2+4*j/64
            maximum=max(maximum,math.dist(evaluate(source,u,v),evaluate(target,u,v)))
    gap=0.
    tangent_gap=0.
    h=1e-4
    for i in range(65):
        t=2+4*i/64
        for direction in ['u','v']:
            sample=lambda s:evaluate(target,s,t) if direction=='u' else evaluate(target,t,s)
            a,a1,a2=sample(2),sample(2+h),sample(2+2*h)
            b,b1,b2=sample(6),sample(6-h),sample(6-2*h)
            gap=max(gap,math.dist(a,b))
            da=[(-3*x+4*y-z)/(2*h) for x,y,z in zip(a,a1,a2)]
            db=[(3*x-4*y+z)/(2*h) for x,y,z in zip(b,b1,b2)]
            tangent_gap=max(tangent_gap,math.dist(da,db))
    assert maximum<=item['deviationUpperMm']<=.2
    assert gap<1e-12 and tangent_gap<1e-5
    reports.append({'axis':axis,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                    'samples':65*65,'maxSampledDeviationMm':maximum,
                    'reportedDeviationUpperMm':item['deviationUpperMm'],
                    'maxSeamPositionResidualMm':gap,
                    'maxFiniteDifferenceSeamTangentResidual':tangent_gap})
report={'oracle':'Independent Python Cox-de Boor evaluation and second-order one-sided differences',
        'independentContinuousCertificate':False,'axes':reports}
(folder/'independent-periodic-surface-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
