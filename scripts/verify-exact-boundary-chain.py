#!/usr/bin/env python3
"""Independent exact Fraction proof for the saved two-patch rational control.
Uses original binary64 values, not residual samples or a filled-solid claim.
"""
import argparse, hashlib, json, math
from fractions import Fraction as F
from pathlib import Path

if not __debug__:
    raise RuntimeError("Exact verification requires Python assertions enabled")

def trim(p):
    while len(p)>1 and p[-1]==0:p.pop()
    return p

def add(a,b):
    return trim([(a[i] if i<len(a) else 0)+(b[i] if i<len(b) else 0) for i in range(max(len(a),len(b)))])

def scale(a,k):return trim([x*k for x in a])

def mul(a,b):
    p=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):p[i+j]+=x*y
    return trim(p)

def power(a,n):
    p=[F(1)]
    for _ in range(n):p=mul(p,a)
    return p

def derivative(p):return [F(0)] if len(p)==1 else [i*p[i] for i in range(1,len(p))]

def net_polynomials(points,weights,a=F(0),b=F(1)):
    n=len(points)-1;t=[a,b-a];u=[1-a,a-b];out=[[F(0)] for _ in range(4)]
    for i,(point,w) in enumerate(zip(points,weights)):
        basis=scale(mul(power(t,i),power(u,n-i)),math.comb(n,i)*w)
        for k in range(4):out[k]=add(out[k],scale(basis,point[k] if k<3 else 1))
    return out

def numbers(values):return [F(v) for v in values]

def natural(s):
    return [[F(s['knotsU'][s['degreeU']]),F(s['knotsU'][len(s['controlPoints'])])],
            [F(s['knotsV'][s['degreeV']]),F(s['knotsV'][len(s['controlPoints'][0])])]]

def bezier(knots,degree,count):
    return count==degree+1 and len(knots)==2*count and all(k==knots[degree] for k in knots[:count]) and all(k==knots[count] for k in knots[count:]) and knots[degree]<knots[count]

def check(model,meta):
    assert len(model['faces'])==2 and len(meta['edges'])==2
    assert meta['boundaryChartAbsenceProven'] and meta['allFacesInjective'] and not meta['filledVolumeClaim']
    assert meta['cells']<=100000 and meta['domainCells']<=1000000 and meta['exactWork']<=1000000
    anchors=meta.get('planeAnchors')
    if meta['coordinate'] is None:
        assert anchors is not None and len(anchors)==3
        anchors=[numbers(p) for p in anchors]
        a,b,c=anchors;u=[b[i]-a[i] for i in range(3)];v=[c[i]-a[i] for i in range(3)]
        normal=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
        assert any(x!=0 for x in normal)
        offset=sum(normal[i]*a[i] for i in range(3))
    else:
        assert 0<=meta['coordinate']<3
        normal=[F(i==meta['coordinate']) for i in range(3)];offset=F(meta['planeValue'])
    height=lambda p:sum(normal[i]*F(p[i]) for i in range(3))-offset
    lifts=0;interior_signs=[]
    for side,fi in enumerate(meta['faces']):
        face=model['faces'][fi];assert not face['holes'];s=face['surface'];poles=s['controlPoints'];weights=s['weights'];domain=natural(s)
        axis,end=meta['naturalSides'][side];assert axis==1 and s['degreeV']==1
        assert len(poles)==s['degreeU']+1 and len(poles[0])==2
        assert not s['periodicU'] and not s['periodicV']
        assert bezier(s['knotsU'],s['degreeU'],len(poles)) and bezier(s['knotsV'],s['degreeV'],len(poles[0]))
        for row,w in zip(poles,weights):
            assert w[0]==w[1] and w[0]>0
            assert row[0][:2]==row[1][:2]
            assert height(row[end])==0
            assert height(row[1-end])*height(poles[0][1-end])>0
        interior_signs.append(height(poles[0][1-end]))
        assert all(height(row[0])==height(poles[0][0]) and height(row[1])==height(poles[0][1]) for row in poles)
        if side==0 and anchors is not None:
            originals=[numbers(row[end]) for row in poles]
            assert all(p in originals for p in anchors)
        # X is a strictly monotone rational coordinate; Z is affine in V.
        # Positive numerator Bernstein coefficients prove injectivity globally.
        profile=[numbers(row[end]) for row in poles];w=[F(row[end]) for row in weights]
        x,_,_,den=net_polynomials(profile,w)
        d=add(mul(derivative(x),den),scale(mul(x,derivative(den)),-1));degree=len(d)-1
        bernstein=[sum(d[i]*F(math.comb(k,i),math.comb(degree,i)) for i in range(k+1)) for k in range(degree+1)]
        assert all(v>0 for v in bernstein)
        intervals=[];uv_polygon=[]
        uses=model['loops'][face['outer']]['coedges']
        for i,c in enumerate(uses):
            pc=c['pcurve'];points=[numbers(p) for p in pc['controlPoints']]
            assert pc['degree']==1 and pc['weights'][0]==pc['weights'][1] and pc['weights'][0]>0
            assert not pc['periodic'] and bezier(pc['knots'],pc['degree'],len(points))
            assert points[-1]==numbers(uses[(i+1)%len(uses)]['pcurve']['controlPoints'][0])
            assert any(points[0][k]==points[1][k] and points[0][k] in domain[k] for k in range(2))
            assert all(domain[k][0]<=p[k]<=domain[k][1] for p in points for k in range(2))
            uv_polygon.append(points[0])
            if c['edge'] not in meta['edges']:continue
            assert all(p[axis]==domain[axis][end] for p in points)
            edge=model['edges'][c['edge']];curve=edge['curve'];cp=[numbers(p) for p in curve['controlPoints']];cw=numbers(curve['weights'])
            assert len(cp)==curve['degree']+1 and all(v>0 for v in cw)
            assert not curve['periodic'] and bezier(curve['knots'],curve['degree'],len(cp))
            if c['reversed']:cp.reverse();cw.reverse()
            actual=net_polynomials(cp,cw)
            a,b=[(p[0]-domain[0][0])/(domain[0][1]-domain[0][0]) for p in points]
            expected=net_polynomials(profile,w,a,b)
            for k in range(3):
                residual=add(mul(actual[k],expected[3]),scale(mul(expected[k],actual[3]),-1))
                assert all(v==0 for v in residual),'Exact lift differs'
            vertices=edge['vertices'][::-1] if c['reversed'] else edge['vertices']
            if a>b:vertices=vertices[::-1]
            intervals.append((min(a,b),max(a,b),*vertices));lifts+=1
        area=sum(uv_polygon[i][0]*uv_polygon[(i+1)%len(uv_polygon)][1]-uv_polygon[(i+1)%len(uv_polygon)][0]*uv_polygon[i][1] for i in range(len(uv_polygon)))/2
        assert area==(domain[0][1]-domain[0][0])*(domain[1][1]-domain[1][0])
        intervals.sort();assert intervals[0][0]==0 and intervals[-1][1]==1
        assert all(a[1]==b[0] and a[3]==b[2] for a,b in zip(intervals,intervals[1:]))
    assert lifts==4 and interior_signs[0]*interior_signs[1]<0
    return dict(exactOriginalLifts=lifts,facesInjective=2,sharedFragments=2,exactHalfspaceSeparation=True,
                boundaryChartAbsenceProven=True,metricSamplesUsed=False,filledVolumeClaim=False,
                supportingPlaneKind='original-anchors' if meta['coordinate'] is None else 'coordinate',roundedNormalUsed=False)

parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);parser.add_argument('--model',default='model.json');parser.add_argument('--expect-lift-refusal',action='store_true');args=parser.parse_args()
model_path=args.directory/args.model;meta=json.loads((args.directory/'manifest.json').read_text());model=json.loads(model_path.read_text())
try:
    report=check(model,meta)
except AssertionError as error:
    if not args.expect_lift_refusal or str(error)!='Exact lift differs':raise
    report=dict(exactLiftRefused=True,metricSamplesUsed=False,filledVolumeClaim=False)
else:
    assert not args.expect_lift_refusal,'Defective exact lift unexpectedly passed'
report['sourceSha256']=hashlib.sha256(model_path.read_bytes()).hexdigest()
name='independent-exact-check.json' if args.model=='model.json' else args.model+'.exact-refusal.json'
(args.directory/name).write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
