#!/usr/bin/env python3
"""Independent saved-curve samples and STEP check of a shared open network."""
import hashlib
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


def count(shape, kind):
    explorer, found = TopExp_Explorer(shape, kind), []
    while explorer.More():
        item = explorer.Current()
        if not any(item.IsSame(old) for old in found):
            found.append(item)
        explorer.Next()
    return len(found)


def main(directory):
    receipt = json.loads((directory/'manifest.json').read_text())
    model = json.loads((directory/'model.json').read_text())
    for filename, key in [('network.step', 'stepSha256'), ('model.json', 'modelSha256')]:
        assert hashlib.sha256((directory/filename).read_bytes()).hexdigest() == receipt[key]
    assert receipt['allBoundaryRootsCovered'] and receipt['savedReloadEqual']
    assert receipt['originalEdgesQueried'] == 7 and len(receipt['roots']) == 6
    assert len(receipt['branches']) == 4 and receipt['faces'] == 6
    assert receipt['chargedCells'] <= 100000 and receipt['chargedExactWork'] <= 1000000
    assert receipt['chargedEvaluations'] <= 4096
    shared = [r for r in receipt['roots'] if r['sourceEdge'] == receipt['originalSharedEdge']]
    assert len(shared) == 2
    for root in shared:
        vertex = root['vertex']
        assert model['vertices'][vertex]['point'] == root['point']
        assert sum(vertex in b['endpoints'] for b in receipt['branches']) == 2
        assert sum(vertex in e['vertices'] for e in model['edges']) == 4
    maximum, samples = 0.0, 0
    for branch in receipt['branches']:
        previous = None
        for edge_id in branch['edges']:
            edge = model['edges'][edge_id]
            curve = edge['curve']
            points, knots = curve['controlPoints'], curve['knots']
            assert curve['degree'] == 3 and curve['weights'] == [1.0]*len(points)
            assert not curve['periodic']
            if previous is not None:
                assert previous == points[0]
            previous = points[-1]
            for i, vertex in enumerate(edge['vertices']):
                assert model['vertices'][vertex]['point'] == points[0 if i == 0 else -1]
            uses = [c for loop in model['loops'] for c in loop['coedges'] if c['edge'] == edge_id]
            assert len(uses) == 2 and sorted(c['reversed'] for c in uses) == [False, True]
            lower = points[0][1] < 0.5
            assert knots[:4] == [knots[3]]*4 and knots[-4:] == [knots[-4]]*4
            assert all(knots.count(k) == 3 for k in set(knots[4:-4]))
            for span in range(3, len(points)):
                if knots[span] == knots[span+1]:
                    continue
                local = points[span-3:span+1]
                for i in range(1025):
                    t = i/1024
                    basis = [(1-t)**3, 3*t*(1-t)**2, 3*t*t*(1-t), t**3]
                    x, y, z = [sum(basis[j]*local[j][k] for j in range(4)) for k in range(3)]
                    expected = 0.5 + (-1 if lower else 1)*math.sqrt(0.0625-0.005*x*x)
                    maximum = max(maximum, abs(y-expected)+abs(z))
                    samples += 1
    assert maximum <= receipt['toleranceMm']
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(directory/'network.step')) == IFSelect_RetDone and reader.TransferRoots()
    shape = reader.OneShape()
    assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology = {name: count(shape, kind) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    assert topology == dict(solids=0, faces=receipt['faces'], edges=receipt['edges'], vertices=receipt['vertices'])
    area = GProp_GProps()
    BRepGProp.SurfaceProperties_s(shape, area)
    assert abs(area.Mass()-2.0) <= 1e-7
    result = dict(passed=True, topology=topology, areaMm2=area.Mass(), samples=samples,
                  maximumSampledRootErrorMm=maximum, exactlySharedRoots=2,
                  closedSolidClaim=False, completeSurfaceIntersectionClaim=False)
    (directory/'independent.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main(Path(sys.argv[1]))
