#!/usr/bin/env python3
"""Independent sampled check of stored collar coefficients, not a corner solid.

OCP constructs its own splines from actual saved coefficients. Native original-
interval proofs remain primary; these samples do not prove whole-seam G2.
"""
import argparse
import hashlib
import itertools
import json
import math
from pathlib import Path
import OCP
from OCP.Geom import Geom_BSplineCurve, Geom_BSplineSurface
from OCP.GeomLProp import GeomLProp_SLProps
from OCP.TColgp import TColgp_Array1OfPnt, TColgp_Array2OfPnt
from OCP.TColStd import TColStd_Array1OfInteger, TColStd_Array1OfReal, TColStd_Array2OfReal
from OCP.gp import gp_Dir, gp_Pnt


def array(values, kind):
    result = kind(1, len(values))
    for i, value in enumerate(values, 1):
        result.SetValue(i, value)
    return result


def knots(values):
    groups = [(key, len(list(group))) for key, group in itertools.groupby(values)]
    return array([g[0] for g in groups], TColStd_Array1OfReal), array([g[1] for g in groups], TColStd_Array1OfInteger)


def surface(data):
    points = data['controlPoints']
    n, m = len(points), len(points[0])
    poles, weights = TColgp_Array2OfPnt(1,n,1,m), TColStd_Array2OfReal(1,n,1,m)
    for i in range(n):
        for j in range(m):
            poles.SetValue(i+1,j+1,gp_Pnt(*points[i][j]))
            weights.SetValue(i+1,j+1,data['weights'][i][j])
    uk, um = knots(data['knotsU']); vk, vm = knots(data['knotsV'])
    return Geom_BSplineSurface(poles,weights,uk,vk,um,vm,data['degreeU'],data['degreeV'],False,False)


def curve(data):
    poles = array([gp_Pnt(*(p if len(p)==3 else [*p,0.])) for p in data['controlPoints']], TColgp_Array1OfPnt)
    k, m = knots(data['knots'])
    return Geom_BSplineCurve(poles,array(data['weights'],TColStd_Array1OfReal),k,m,data['degree'],False)


def xyz(p):
    return [p.X(),p.Y(),p.Z()]


def shape(s,u,v):
    props = GeomLProp_SLProps(s,u,v,2,1e-12)
    assert props.IsNormalDefined() and props.IsCurvatureDefined()
    normal = xyz(props.Normal())
    if props.IsUmbilic():
        curvature = (props.MaxCurvature()+props.MinCurvature())/2
        tensor = [[curvature*((1. if i==j else 0.)-normal[i]*normal[j]) for j in range(3)] for i in range(3)]
    else:
        maximum,minimum=gp_Dir(),gp_Dir()
        props.CurvatureDirections(maximum,minimum)
        a,b=xyz(maximum),xyz(minimum)
        tensor=[[props.MaxCurvature()*a[i]*a[j]+props.MinCurvature()*b[i]*b[j] for j in range(3)] for i in range(3)]
    return normal,tensor


def verify(path):
    data=json.loads(path.read_text());assert len(data['boundaries'])==6
    rows=[]
    for index,boundary in enumerate(data['boundaries']):
        c,pc=curve(boundary['curve']),curve(boundary['originalPcurve'])
        s,original=surface(boundary['surface']),surface(boundary['originalSurface'])
        position_error=normal_error=tensor_error=0.
        for i in range(33):
            t=i/32
            p=c.Value(c.FirstParameter()+t*(c.LastParameter()-c.FirstParameter()))
            uv=pc.Value(pc.FirstParameter()+t*(pc.LastParameter()-pc.FirstParameter()))
            original_point=original.Value(uv.X(),uv.Y()); actual=s.Value(t,0.)
            position_error=max(position_error,p.Distance(original_point),p.Distance(actual))
            n,a=shape(original,uv.X(),uv.Y());m,b=shape(s,t,0.)
            sign=1 if sum(x*y for x,y in zip(n,m))>=0 else -1
            normal_error=max(normal_error,math.dist(n,[sign*x for x in m]))
            tensor_error=max(tensor_error,math.sqrt(sum((a[i][j]-sign*b[i][j])**2 for i in range(3) for j in range(3))))
        assert position_error<=1e-7 and normal_error<=1e-5 and tensor_error<=1e-4,(index,position_error,normal_error,tensor_error)
        rows.append(dict(boundary=index,samples=33,positionErrorMm=position_error,normalError=normal_error,shapeTensorError=tensor_error))
    report=dict(passed=True,occtVersion=OCP.__version__,inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(),scope='sampled six stored collars against original spline supports; not STEP or a filled corner',closedBodyQualified=False,cases=rows)
    path.with_name('collars-occt-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('path',type=Path)
    verify(parser.parse_args().path)
