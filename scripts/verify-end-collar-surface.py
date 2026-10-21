#!/usr/bin/env python3
"""Independent OpenCascade position/normal/world-curvature samples.

Samples supplement native whole-contact G2 and chart injectivity proofs;
there is no closed-body, arbitrary NURBS end, or radius-law claim.
"""
import json,sys,math,hashlib
from pathlib import Path
from OCP.Geom import Geom_BSplineSurface,Geom_BSplineCurve
from OCP.TColgp import TColgp_Array2OfPnt,TColgp_Array1OfPnt
from OCP.TColStd import TColStd_Array2OfReal,TColStd_Array1OfReal,TColStd_Array1OfInteger
from OCP.gp import gp_Pnt,gp_Vec
root=Path(sys.argv[1]);g=json.loads((root/'geometry.json').read_text());m=json.loads((root/'manifest.json').read_text())
assert m['surfaceQualified'] and m['wholeOriginalContactsG2'] and m['injectiveChart']
assert m['cells']<=100000 and m['exactWork']<=1000000

def knots(values):
    unique=[];counts=[]
    for x in values:
        if unique and x==unique[-1]:counts[-1]+=1
        else:unique.append(x);counts.append(1)
    k=TColStd_Array1OfReal(1,len(unique));n=TColStd_Array1OfInteger(1,len(unique))
    for i,(x,c) in enumerate(zip(unique,counts),1):k.SetValue(i,x);n.SetValue(i,c)
    return k,n

def surface(s):
    pts=s['controlPoints'];a=TColgp_Array2OfPnt(1,len(pts),1,len(pts[0]));w=TColStd_Array2OfReal(1,len(pts),1,len(pts[0]))
    for i,row in enumerate(pts,1):
        for j,p in enumerate(row,1):a.SetValue(i,j,gp_Pnt(*p));w.SetValue(i,j,s['weights'][i-1][j-1])
    ku,mu=knots(s['knotsU']);kv,mv=knots(s['knotsV'])
    return Geom_BSplineSurface(a,w,ku,kv,mu,mv,s['degreeU'],s['degreeV'],False,False)

def curve(c):
    a=TColgp_Array1OfPnt(1,len(c['controlPoints']));w=TColStd_Array1OfReal(1,len(c['weights']))
    for i,(p,v) in enumerate(zip(c['controlPoints'],c['weights']),1):a.SetValue(i,gp_Pnt(*(p if len(p)==3 else p+[0.])));w.SetValue(i,v)
    k,m=knots(c['knots']);return Geom_BSplineCurve(a,w,k,m,c['degree'],False)

def dot(a,b):return sum(x*y for x,y in zip(a,b))
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def norm(a):return math.sqrt(dot(a,a))
def jets(s,u,v):
    p=gp_Pnt();du=gp_Vec();dv=gp_Vec();uu=gp_Vec();vv=gp_Vec();uv=gp_Vec();s.D2(u,v,p,du,dv,uu,vv,uv)
    du=list(du.Coord());dv=list(dv.Coord());n=cross(du,dv);length=norm(n);assert length>1e-12;n=[x/length for x in n]
    e=dot(du,du);f=dot(du,dv);h=dot(dv,dv);det=e*h-f*f;assert det>1e-20
    dual=[[ (h*du[k]-f*dv[k])/det for k in range(3)],[(e*dv[k]-f*du[k])/det for k in range(3)]]
    ii=[[dot(n,uu.Coord()),dot(n,uv.Coord())],[dot(n,uv.Coord()),dot(n,vv.Coord())]]
    tensor=[[sum(ii[i][j]*dual[i][a]*dual[j][b] for i in range(2) for j in range(2)) for b in range(3)] for a in range(3)]
    return list(p.Coord()),n,tensor
skin=surface(g['surface']);supports=list(map(surface,g['supports']));pc=list(map(curve,g['supportPcurves']));sc=list(map(curve,g['skinPcurves']))
maximum=[0.,0.,0.];samples=0
for side in range(2):
    for i in range(129):
        t=i/128;uv=pc[side].Value(t);ab=sc[side].Value(t)
        a,na,ta=jets(supports[side],uv.X(),uv.Y());b,nb,tb=jets(skin,ab.X(),ab.Y())
        sign=1 if dot(na,nb)>=0 else -1
        errors=[norm([x-y for x,y in zip(a,b)]),norm([x-sign*y for x,y in zip(na,nb)]),
            math.sqrt(sum((ta[k][j]-sign*tb[k][j])**2 for k in range(3) for j in range(3)))]
        maximum=[max(x,y) for x,y in zip(maximum,errors)];samples+=1
assert maximum[0]<1e-9 and maximum[1]<1e-9 and maximum[2]<1e-7,maximum
ends=[curve(c).Value(0.) for c in g['contacts']]
axial_error=abs(ends[1].Z()-ends[0].Z()-m['axialSetbackMm'])
radial_error=abs(ends[1].X()-ends[0].X()-m['radialSetbackMm'])
assert axial_error<1e-12 and radial_error<1e-12

report=dict(passed=True,samples=samples,axialSetbackErrorMm=axial_error,radialSetbackErrorMm=radial_error,maximumPositionErrorMm=maximum[0],maximumNormalError=maximum[1],maximumWorldCurvatureTensorError=maximum[2],geometrySha256=hashlib.sha256((root/'geometry.json').read_bytes()).hexdigest(),scope='independent samples supplement native whole-contact G2; surface only, not a closed body')
(root/'independent-occt-surface.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
