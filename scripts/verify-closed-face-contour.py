#!/usr/bin/env python3
"""Independent saved-circle samples and STEP geometry after closed face partition."""
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer


def evaluate(curve, t):
    p, knots, points, weights = curve['degree'], curve['knots'], curve['controlPoints'], curve['weights']
    assert all(w > 0 for w in weights)
    span = len(points)-1 if t == knots[len(points)] else next(i for i in range(p, len(points)) if knots[i] <= t < knots[i+1])
    d = [[x*weights[i] for x in points[i]]+[weights[i]] for i in range(span-p, span+1)]
    for r in range(1, p+1):
        for j in range(p, r-1, -1):
            i = span-p+j
            denominator = knots[i+p-r+1]-knots[i]
            alpha = (t-knots[i])/denominator if denominator else 0.0
            d[j] = [(1-alpha)*a+alpha*b for a, b in zip(d[j-1], d[j])]
    return [x/d[p][-1] for x in d[p][:-1]]


def main(directory):
    receipt = json.loads((directory/'manifest.json').read_text())
    model = json.loads((directory/'model.json').read_text())
    for file, key in [('partition.step', 'stepSha256'), ('model.json', 'modelSha256')]:
        assert hashlib.sha256((directory/file).read_bytes()).hexdigest() == receipt[key]
    assert receipt['nativeClosed'] and receipt['savedReloadEqual'] and receipt['originalEdgeIdentitiesPreserved']
    assert receipt['cells'] <= 100000 and receipt['exactWork'] <= 1000000
    maximum, sampled, first, previous = 0., 0, None, None
    for edge_id in receipt['cutEdges']:
        edge = model['edges'][edge_id]
        curve = edge['curve']
        points, knots, p = curve['controlPoints'], curve['knots'], curve['degree']
        assert not curve['periodic']
        for end, vertex in enumerate(edge['vertices']):
            assert points[0 if end == 0 else -1] == model['vertices'][vertex]['point']
        if previous is not None:
            assert previous == points[0]
        else:
            first = points[0]
        previous = points[-1]
        uses = [c for loop in model['loops'] for c in loop['coedges'] if c['edge'] == edge_id]
        assert len(uses) == 2 and sorted(c['reversed'] for c in uses) == [False, True]
        for i in range(1025):
            t = knots[p]+(knots[len(points)]-knots[p])*i/1024
            x, y, z = evaluate(curve, t)
            maximum = max(maximum, abs(math.hypot(x-2., y-2.)-0.8)+abs(z))
            sampled += 1
    assert previous == first and maximum <= 1e-7
    spec = importlib.util.spec_from_file_location('cap_check', Path(__file__).with_name('verify-curved-cap-occt.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(directory/'partition.step')) == IFSelect_RetDone and reader.TransferRoots()
    shape = reader.OneShape()
    assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology = {name: module.count(shape, kind) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    assert topology == dict(solids=1, faces=7, edges=16, vertices=12)
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props)
    assert abs(props.Mass()-32.) <= 1e-7
    explorer, faces, areas = TopExp_Explorer(shape, TopAbs_FACE), [], []
    while explorer.More():
        face = explorer.Current()
        if not any(face.IsSame(old) for old in faces):
            faces.append(face)
            area = GProp_GProps()
            BRepGProp.SurfaceProperties_s(face, area)
            areas.append(area.Mass())
        explorer.Next()
    disk = math.pi*0.8**2
    expected = sorted([disk, 16.-disk, 16., 8., 8., 8., 8.])
    assert max(abs(a-b) for a, b in zip(sorted(areas), expected)) <= 1e-6
    report = dict(passed=True, topology=topology, volumeMm3=props.Mass(), faceAreasMm2=sorted(areas),
                  expectedDiskAreaMm2=disk, samples=sampled, maximumSampledCircleErrorMm=maximum,
                  exactlySharedClosedEndpoints=True, completeSurfaceIntersectionClaim=False, filletClaim=False)
    (directory/'independent.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main(Path(sys.argv[1]))
