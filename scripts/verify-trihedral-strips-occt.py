#!/usr/bin/env python3
"""Independent checks of three open quintic strip fixtures, not closed solids.

Native full-interval certificates remain primary. OCP seam samples supplement
those certificates; sampling does not prove whole-seam continuity.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

import OCP
from OCP.BRep import BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Curve, BRepAdaptor_Curve2d
from OCP.BRepTools import BRepTools, BRepTools_WireExplorer
from OCP.gp import gp_Pnt, gp_Vec
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
from OCP.GeomLProp import GeomLProp_SLProps
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SOLID, TopAbs_VERTEX, TopAbs_REVERSED
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS


def entities(shape, kind):
    result = []
    explorer = TopExp_Explorer(shape, kind)
    while explorer.More():
        current = explorer.Current()
        if not any(current.IsSame(previous) for previous in result):
            result.append(current)
        explorer.Next()
    return result


def xyz(point):
    return [point.X(), point.Y(), point.Z()]


def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def unit(a):
    length = math.sqrt(dot(a, a))
    assert length > 0
    return [x / length for x in a]


def section_length(n):
    """Independent integration of the declared unit-radius quintic section."""
    h = 0.35 * math.sqrt(2)
    poles = [[1, 0], [1 - h, 0], [1 - 2 * h, 0], [0, 1 - 2 * h], [0, 1 - h], [0, 1]]
    velocity = [[5 * (b[k] - a[k]) for k in range(2)] for a, b in zip(poles, poles[1:])]

    def speed(t):
        q = [p[:] for p in velocity]
        for size in range(4, 0, -1):
            for i in range(size):
                q[i] = [(1 - t) * q[i][k] + t * q[i + 1][k] for k in range(2)]
        return math.hypot(*q[0])

    return (speed(0) + speed(1) + sum((4 if i % 2 else 2) * speed(i / n) for i in range(1, n))) / (3 * n)


def gauss_nodes(n):
    """Gauss-Legendre nodes and weights on [0,1], without a mesh dependency."""
    nodes = []
    for i in range(n):
        x = math.cos(math.pi * (i + .75) / (n + .5))
        for _ in range(64):
            a, b = 1., x
            for k in range(2, n + 1):
                a, b = b, ((2*k-1)*x*b-(k-1)*a)/k
            derivative = n*(x*b-a)/(x*x-1)
            delta = b/derivative
            x -= delta
            if abs(delta) < 1e-15:
                break
        else:
            raise AssertionError('Quadrature root did not converge')
        nodes.append(((x+1)/2, 1/((1-x*x)*derivative*derivative)))
    return nodes


def imported_quad_area(face, surface, n):
    """Integrate the actual imported surface over its verified straight UV rim.

    Restricted to this four-sided fixture. Curved rims must use another verifier.
    BRepGProp's reported error is retained separately, not used as a certificate.
    """
    explorer = BRepTools_WireExplorer(BRepTools.OuterWire_s(face), face)
    corners, ends = [], []
    while explorer.More():
        edge = explorer.Current()
        curve = BRepAdaptor_Curve2d(edge, face)
        first, last = curve.FirstParameter(), curve.LastParameter()
        if edge.Orientation() == TopAbs_REVERSED:
            first, last = last, first
        a, b = curve.Value(first), curve.Value(last)
        # Exact linear representation, rather than sampled rim straightness.
        spline = curve.BSpline()
        assert spline.Degree() == 1 and spline.NbPoles() == 2
        corners.append((a.X(), a.Y()))
        ends.append((b.X(), b.Y()))
        explorer.Next()
    assert len(corners) == 4
    assert all(math.dist(ends[i], corners[(i+1)%4]) <= 1e-10 for i in range(4))
    total = 0.
    signs = []
    for s, ws in gauss_nodes(n):
        for t, wt in gauss_nodes(n):
            q0,q1,q2,q3 = corners
            uv = [(1-s)*(1-t)*q0[k]+s*(1-t)*q1[k]+s*t*q2[k]+(1-s)*t*q3[k] for k in range(2)]
            ds = [(1-t)*(q1[k]-q0[k])+t*(q2[k]-q3[k]) for k in range(2)]
            dt = [(1-s)*(q3[k]-q0[k])+s*(q2[k]-q1[k]) for k in range(2)]
            jac = ds[0]*dt[1]-ds[1]*dt[0]
            signs.append(jac)
            point, du, dv = gp_Pnt(), gp_Vec(), gp_Vec()
            surface.D1(*uv, point, du, dv)
            total += ws*wt*abs(jac)*du.Crossed(dv).Magnitude()
    assert min(signs) > 0 or max(signs) < 0
    return total


def verify(directory):
    l1, l2 = section_length(4096), section_length(8192)
    assert abs(l1 - l2) < 1e-10
    records = [json.loads((directory / f'skin-{i}.json').read_text()) for i in range(3)]
    radii = {data['sourceEdge']: data['radiusMm'] for data in records}
    rows = []
    for i, data in enumerate(records):
        path = directory / f'skin-{i}.step'
        reader = STEPControl_Reader()
        assert reader.ReadFile(str(path)) == IFSelect_RetDone and reader.TransferRoots() > 0
        shape = reader.OneShape()
        assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
        faces, edges, vertices = entities(shape, TopAbs_FACE), entities(shape, TopAbs_EDGE), entities(shape, TopAbs_VERTEX)
        assert len(faces) == 1 and len(edges) == 4 and len(vertices) == 4
        assert not entities(shape, TopAbs_SOLID), 'Open strip must not be advertised as a solid'
        mesher = BRepMesh_IncrementalMesh(shape, 0.01, False, 0.1, True)
        assert mesher.IsDone()
        surface = BRep_Tool.Surface_s(TopoDS.Face_s(faces[0]))
        source, model = data['source'], data['model']
        tolerance = source['toleranceMm']
        normals, plane_points = [], []
        for index in data['supports']:
            points = source['faces'][index]['surface']['controlPoints']
            plane_points.append(points[0][0])
            normals.append(unit(cross(sub(points[-1][0], points[0][0]), sub(points[0][-1], points[0][0]))))
        actual = []
        max_plane, max_projection, max_normal, max_curvature = 0., 0., 0., 0.
        for side, index in enumerate(data['contactEdges']):
            native = model['edges'][index]
            wanted = [model['vertices'][v]['point'] for v in native['vertices']]
            candidates = []
            for edge in edges:
                curve = BRepAdaptor_Curve(TopoDS.Edge_s(edge))
                endpoints = [xyz(curve.Value(curve.FirstParameter())), xyz(curve.Value(curve.LastParameter()))]
                errors = [max(math.dist(endpoints[k], wanted[k ^ reverse]) for k in range(2)) for reverse in [0, 1]]
                if min(errors) <= tolerance:
                    candidates.append((curve, errors[1] < errors[0]))
            assert len(candidates) == 1
            curve, reverse = candidates[0]
            samples = []
            for station in range(17):
                t = station / 16
                if reverse:
                    t = 1 - t
                parameter = curve.FirstParameter() + t * (curve.LastParameter() - curve.FirstParameter())
                point = curve.Value(parameter)
                p = xyz(point)
                max_plane = max(max_plane, abs(dot(sub(p, plane_points[side]), normals[side])))
                projection = GeomAPI_ProjectPointOnSurf(point, surface, 1e-12)
                assert projection.NbPoints() > 0
                u, v = projection.LowerDistanceParameters()
                max_projection = max(max_projection, projection.LowerDistance())
                props = GeomLProp_SLProps(surface, u, v, 2, 1e-12)
                assert props.IsNormalDefined() and props.IsCurvatureDefined()
                normal = xyz(props.Normal())
                max_normal = max(max_normal, min(math.dist(normal, normals[side]), math.dist(normal, [-x for x in normals[side]])))
                max_curvature = max(max_curvature, abs(props.MaxCurvature()), abs(props.MinCurvature()))
                samples.append(p)
            actual.append(samples)
        max_center_gap = 0.
        for a, b in zip(*actual):
            centers = [[p[k] + data['radiusMm'] * data['sides'][side] * normals[side][k] for k in range(3)] for side, p in enumerate([a, b])]
            max_center_gap = max(max_center_gap, math.dist(*centers))
        edge = source['edges'][data['sourceEdge']]
        assert 0 in edge['vertices']
        origin = source['vertices'][0]['point']
        other = next(v for v in edge['vertices'] if v != 0)
        ray = unit(sub(source['vertices'][other]['point'], origin))
        # These fixtures are orthogonal corner strips, including rigid poses.
        expected_start = max(r for e, r in radii.items() if e != data['sourceEdge']) + 0.1
        assert abs(expected_start - data['targetStartMm']) < 1e-10
        stations = [dot(sub(p, origin), ray) for samples in actual for p in [samples[0], samples[-1]]]
        end_error = max(abs(min(stations) - expected_start), abs(max(stations) - expected_start - data['spanMm']))
        props = GProp_GProps()
        gprop_error = BRepGProp.SurfaceProperties_s(shape, props, Eps=1e-10, SkipShared=False)
        area32 = imported_quad_area(TopoDS.Face_s(faces[0]), surface, 32)
        area64 = imported_quad_area(TopoDS.Face_s(faces[0]), surface, 64)
        assert abs(area32 - area64) <= 1e-10
        expected_area = data['radiusMm'] * data['spanMm'] * l2
        area_error = abs(area64 - expected_area)
        assert max_plane <= tolerance and max_projection <= tolerance and max_center_gap <= tolerance
        assert max_normal <= 1e-5 and max_curvature <= 1e-4
        assert end_error <= tolerance and area_error <= 1e-7, {
            'file':path.name,'endErrorMm':end_error,'areaMm2':props.Mass(),
            'expectedAreaMm2':expected_area,'areaErrorMm2':area_error,'positions':stations,
            'planeErrorMm':max_plane,'centerGapMm':max_center_gap,
        }
        rows.append({'file': path.name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'valid': True,
                     'faces': 1, 'edges': 4, 'vertices': 4, 'solids': 0, 'sampledSeamStations': 34,
                     'sampledPlaneErrorMm': max_plane, 'sampledSkinProjectionErrorMm': max_projection,
                     'sampledNormalError': max_normal, 'sampledCurvatureMagnitude': max_curvature,
                     'sampledRollingCenterGapMm': max_center_gap, 'endPositionErrorMm': end_error,
                     'areaMm2': area64, 'quadratureConvergenceMm2': abs(area32-area64),
                     'gpropAreaMm2': props.Mass(), 'gpropReportedRelativeError': gprop_error,
                     'independentQuinticAreaMm2': expected_area,
                     'areaErrorMm2': area_error, 'passed': True})
    report = {'passed': True, 'occtVersion': OCP.__version__, 'scope': 'three open orthogonal-corner quintic strips; sampled seam checks supplement native whole-interval proofs',
              'closedCornerQualified': False, 'circularSectionRadiusQualified': False, 'cases': rows}
    (directory / 'occt-report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    verify(parser.parse_args().directory)
