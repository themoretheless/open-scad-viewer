#!/usr/bin/env python3
"""Independent STEP topology/volume and sampled world G2 checks on saved seams."""
import hashlib
import json
import math
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX, TopAbs_SOLID, TopAbs_REVERSED
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface, BRepAdaptor_Curve2d
from OCP.GeomAbs import GeomAbs_BSplineSurface
from OCP.gp import gp_Pnt, gp_Vec


def entities(shape, kind):
    explorer = TopExp_Explorer(shape, kind)
    result = []
    while explorer.More():
        current = explorer.Current()
        if not any(current.IsSame(old) for old in result):
            result.append(current)
        explorer.Next()
    return result


def dot(a, b):
    return sum(x*y for x, y in zip(a, b))


def properties(face, uv):
    surface = BRepAdaptor_Surface(face)
    point, du, dv, duu, dvv, duv = gp_Pnt(), gp_Vec(), gp_Vec(), gp_Vec(), gp_Vec(), gp_Vec()
    surface.D2(uv.X(), uv.Y(), point, du, dv, duu, dvv, duv)
    vector = lambda v: [v.X(), v.Y(), v.Z()]
    u, v, uu, vv, uv = map(vector, [du, dv, duu, dvv, duv])
    normal = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]]
    length = math.sqrt(dot(normal, normal))
    assert length > 0
    sense = -1 if face.Orientation() == TopAbs_REVERSED else 1
    normal = [sense*x/length for x in normal]
    e, f, g = dot(u, u), dot(u, v), dot(v, v)
    determinant = e*g-f*f
    assert determinant > 0
    inverse = [[g/determinant, -f/determinant], [-f/determinant, e/determinant]]
    second = [[dot(normal, uu), dot(normal, uv)], [dot(normal, uv), dot(normal, vv)]]
    basis = [u, v]
    # World second-fundamental-form tensor J G^-1 b G^-1 J^T.
    tensor = [[sum(basis[a][i]*inverse[a][b]*second[b][c]*inverse[c][d]*basis[d][j]
                   for a in range(2) for b in range(2) for c in range(2) for d in range(2))
               for j in range(3)] for i in range(3)]
    return vector(point), normal, tensor


def main(directory):
    receipt = json.loads((directory/'manifest.json').read_text())
    for file, key in [('model.json', 'modelSha256'), ('corner.step', 'stepSha256')]:
        assert hashlib.sha256((directory/file).read_bytes()).hexdigest() == receipt[key]
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(directory/'corner.step')) == IFSelect_RetDone and reader.TransferRoots()
    shape = reader.OneShape()
    assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology = {name: len(entities(shape, kind)) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    assert topology == dict(solids=1, faces=receipt['faces'], edges=receipt['edges'], vertices=receipt['vertices'])
    mass = GProp_GProps()
    integration_error = BRepGProp.VolumeProperties_s(shape, mass, 1e-10)
    # Integral of 3 + x^2/4 + y^2/8 over [-1,2]^2.
    expected = 27 + 9/4 + 9/8
    assert receipt['expectedVolumeMm3'] == expected and abs(mass.Mass()-expected) <= 1e-7
    faces = [TopoDS.Face_s(f) for f in entities(shape, TopAbs_FACE)]
    patches = [face for face in faces if BRepAdaptor_Surface(face).GetType() == GeomAbs_BSplineSurface
               and BRepAdaptor_Surface(face).UDegree() == receipt['patchDegree'][0]
               and BRepAdaptor_Surface(face).VDegree() == receipt['patchDegree'][1]]
    assert len(patches) == 1
    patch = patches[0]
    upper = [0., 0., 0.]
    sample_count = 0
    edges = entities(patch, TopAbs_EDGE)
    assert len(edges) == receipt.get("rimEdges", 4)
    for edge_shape in edges:
        edge = TopoDS.Edge_s(edge_shape)
        supports = [face for face in faces if not face.IsSame(patch)
                    and any(edge.IsSame(other) for other in entities(face, TopAbs_EDGE))]
        assert len(supports) == 1
        pcurve = BRepAdaptor_Curve2d(edge, patch)
        source_curve = BRepAdaptor_Curve2d(edge, supports[0])
        lo, hi = pcurve.FirstParameter(), pcurve.LastParameter()
        assert abs(lo-source_curve.FirstParameter()) < 1e-12 and abs(hi-source_curve.LastParameter()) < 1e-12
        for i in range(65):
            t = lo+(hi-lo)*i/64
            a = properties(patch, pcurve.Value(t))
            b = properties(supports[0], source_curve.Value(t))
            errors = [math.sqrt(sum((x-y)**2 for x, y in zip(a[0], b[0]))),
                      math.sqrt(sum((x-y)**2 for x, y in zip(a[1], b[1]))),
                      math.sqrt(sum((a[2][j][k]-b[2][j][k])**2 for j in range(3) for k in range(3)))]
            upper = [max(x, y) for x, y in zip(upper, errors)]
            sample_count += 1
    assert all(x <= tolerance for x, tolerance in zip(upper, receipt['tolerances']))
    report = dict(passed=True, topology=topology, volumeMm3=mass.Mass(), expectedVolumeMm3=expected,
                  integrationRelativeError=integration_error, sampledSeamUpper=upper, sampledSites=sample_count,
                  continuousProofSource='native original-support reports', prescribedRadiusClaim=False, globalEmbeddingClaim=False)
    (directory/'independent.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main(Path(sys.argv[1]))
