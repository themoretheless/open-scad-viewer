#!/usr/bin/env python3
"""Sample stored central NURBS against original supports using independent OCCT.

A passing sampled result is not whole-seam G2 or closed-solid qualification.
"""
import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import OCP
from OCP.GeomAPI import GeomAPI_IntCS
from OCP.Geom import Geom_Plane
from OCP.gp import gp_Pnt, gp_Dir
spec = importlib.util.spec_from_file_location('collar_qa', Path(__file__).with_name('verify-trihedral-collars-occt.py'))
qa = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qa)


def sampling_sites(pc, surface_data):
    first, last = pc.FirstParameter(), pc.LastParameter()
    roots = [0.,1.]
    for axis, key in enumerate(('knotsU','knotsV')):
        for knot in sorted(set(surface_data[key])):
            if not 0. < knot < 1.:
                continue
            origin = [0.,0.,0.]; origin[axis] = knot
            normal = [0.,0.,0.]; normal[axis] = 1.
            intersection = GeomAPI_IntCS(pc, Geom_Plane(gp_Pnt(*origin),gp_Dir(*normal)))
            assert intersection.IsDone(), ('incomplete OCCT knot intersection', axis, knot)
            for index in range(1,intersection.NbPoints()+1):
                parameter = intersection.Parameters(index)[-1]
                assert abs(qa.xyz(pc.Value(parameter))[axis]-knot)<=1e-6
                t=(parameter-first)/(last-first)
                if -1e-10<=t<=1.+1e-10: roots.append(min(1.,max(0.,t)))
    roots.sort()
    unique=[]
    for t in roots:
        if not unique or t-unique[-1]>1e-10: unique.append(t)
    sites={i/256 for i in range(257)}
    sites.update(unique)
    for a,b in zip(unique,unique[1:]):
        sites.update(a+(b-a)*i/18 for i in range(1,18))
    return sorted(sites)

def exploratory_grid(surface, boundaries, resolution=25):
    if not 5 <= resolution <= 201:
        raise ValueError('Interior grid resolution must be in 5..201')
    polygon=[]
    for boundary in boundaries:
        pc=qa.curve(boundary['candidatePcurve'])
        for i in range(256):
            p=pc.Value(pc.FirstParameter()+i/256*(pc.LastParameter()-pc.FirstParameter()))
            polygon.append((p.X(),p.Y()))
    def inside(u,v):
        hit=False
        for (a,b),(c,e) in zip(polygon,polygon[1:]+polygon[:1]):
            if (b>v)!=(e>v) and u<(c-a)*(v-b)/(e-b)+a: hit=not hit
        return hit
    points=[]
    for i in range(1,resolution):
        for j in range(1,resolution):
            u,v=i/resolution,j/resolution
            if inside(u,v):points.append(dict(uv=[u,v],xyz=qa.xyz(surface.Value(u,v))))
    return dict(scope='Exploratory interior grid with polygonal membership; not a trim or solid certificate',
        resolution=resolution,samples=len(points),bounds=[[min(p['xyz'][k] for p in points),max(p['xyz'][k] for p in points)] for k in range(3)],
        minimumByAxis=[min(points,key=lambda p:p['xyz'][k]) for k in range(3)],
        lowestByAxis=[sorted(points,key=lambda p:p['xyz'][k])[:12] for k in range(3)],
        largest=max(points,key=lambda p:max(abs(x) for x in p['xyz'])))

def verify(path, interior_grid=25):
    data = json.loads(path.read_text())
    assert len(data['boundaries']) == 6
    surface = qa.surface(data['surface'])
    cases = []
    for index, boundary in enumerate(data['boundaries']):
        curve, pc, candidate_pc = [qa.curve(boundary[k]) for k in ('curve', 'originalPcurve', 'candidatePcurve')]
        original = qa.surface(boundary['originalSurface'])
        position = normal = tensor = 0.
        groups = [[0.,0.,0.],[0.,0.,0.]]
        worst_shape_t = 0.
        sites = sampling_sites(candidate_pc,data["surface"])
        for i,t in enumerate(sites):
            parameter = lambda c: c.FirstParameter() + t * (c.LastParameter() - c.FirstParameter())
            p = curve.Value(parameter(curve))
            uv = pc.Value(parameter(pc))
            q = candidate_pc.Value(parameter(candidate_pc))
            p_error = max(p.Distance(original.Value(uv.X(), uv.Y())), p.Distance(surface.Value(q.X(), q.Y())))
            position = max(position, p_error)
            n, a = qa.shape(original, uv.X(), uv.Y())
            m, b = qa.shape(surface, q.X(), q.Y())
            sign = 1 if sum(x*y for x,y in zip(n,m)) >= 0 else -1
            n_error = math.dist(n, [sign*x for x in m])
            h_error = math.sqrt(sum((a[i][j]-sign*b[i][j])**2 for i in range(3) for j in range(3)))
            normal = max(normal,n_error)
            if h_error > tensor:
                worst_shape_t = t
            tensor = max(tensor,h_error)
            groups[int(round(t*256)) % 2] = [max(x,y) for x,y in zip(groups[int(round(t*256)) % 2], [p_error,n_error,h_error])]
        cases.append(dict(boundary=index, samples=len(sites), positionErrorMm=position, normalError=normal, shapeTensorError=tensor, worstShapeParameter=worst_shape_t, gridEven129=groups[0], gridOdd128=groups[1]))
    junctions = []
    for i, first in enumerate(data['boundaries']):
        second = data['boundaries'][(i+1) % 6]
        p, q = qa.curve(first['originalPcurve']), qa.curve(second['originalPcurve'])
        u, v = p.Value(p.LastParameter()), q.Value(q.FirstParameter())
        n, a = qa.shape(qa.surface(first['originalSurface']), u.X(), u.Y())
        m, b = qa.shape(qa.surface(second['originalSurface']), v.X(), v.Y())
        sign = 1 if sum(x*y for x,y in zip(n,m)) >= 0 else -1
        junctions.append(dict(junction=i, normalError=math.dist(n, [sign*x for x in m]), shapeTensorError=math.sqrt(sum((a[x][y]-sign*b[x][y])**2 for x in range(3) for y in range(3)))))
    passed = all(c['positionErrorMm'] <= 1e-7 and c['normalError'] <= 1e-5 and c['shapeTensorError'] <= 1e-4 for c in cases)
    report = dict(sampledPassed=passed, wholeSeamQualified=False, sampling="OCCT curve-plane knot intersections plus interior stations", closedBodyQualified=False, occtVersion=OCP.__version__, inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(), cases=cases, originalSupportJunctions=junctions, interiorGrid=exploratory_grid(surface,data["boundaries"],interior_grid))
    suffix='-occt.json' if interior_grid==25 else '-occt-interior'+str(interior_grid)+'.json'
    path.with_name(path.stem+suffix).write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if passed else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', type=Path)
    parser.add_argument('--interior-grid',type=int,default=25)
    args=parser.parse_args()
    raise SystemExit(verify(args.path,args.interior_grid))
