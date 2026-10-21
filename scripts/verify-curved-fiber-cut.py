#!/usr/bin/env python3
"""Independent saved-coefficient samples of a native qualified curved cut.
Samples supplement the native whole-piece proofs; no closed-body assertion.
"""
import json
import math
import sys
from pathlib import Path


def main(directory):
    receipt = json.loads((directory / 'manifest.json').read_text())
    model = json.loads((directory / 'model.json').read_text())
    maximum_error = 0.0
    previous = None
    sampled = 0
    for item in receipt['curves']:
        edge = model['edges'][item['edge']]
        assert edge['curve'] == item['curve'] and edge['vertices'] == item['vertices']
        curve = edge['curve']
        assert curve['degree'] == 3 and not curve['periodic']
        assert curve['weights'] == [1.0]*len(curve['controlPoints'])
        knots=curve['knots']
        assert knots[:4] == [knots[3]]*4 and knots[-4:] == [knots[-4]]*4
        assert all(knots.count(k)==3 for k in set(knots[4:-4]))
        points = curve['controlPoints']
        if previous is not None:
            assert previous == points[0]
        previous = points[-1]
        for index, vertex in enumerate(edge['vertices']):
            assert model['vertices'][vertex]['point'] == points[0 if index==0 else -1]
        uses = [use for wire in model['loops'] for use in wire['coedges'] if use['edge'] == item['edge']]
        assert len(uses) == item['uses'] == 2
        assert sorted(use['reversed'] for use in uses) == [False, True]
        for span in range(3,len(points)):
            if knots[span] == knots[span+1]:
                continue
            local=points[span-3:span+1]
            for i in range(1025):
                t = i / 1024
                basis = [(1-t)**3, 3*t*(1-t)**2, 3*t*t*(1-t), t**3]
                x, y, z = [sum(basis[j]*local[j][k] for j in range(4)) for k in range(3)]
                u = (x - 0.25) / 0.25
                expected_y = math.sqrt(0.25 + 0.125*x*x - 0.0625*u*u)
                expected_z = 0.25 + 0.125*x*x + 0.02*expected_y*expected_y
                error = abs(y-expected_y) + abs(z-expected_z)
                maximum_error = max(maximum_error, error)
                sampled += 1
    assert receipt['cells'] <= 100000 and receipt['exactWork'] <= 1000000
    assert receipt['allBoundaryRootsCovered'] and receipt['savedReloadEqual']
    assert maximum_error <= receipt['rootErrorUpperMm']
    report = dict(samples=sampled, segments=len(receipt['curves']), maximumSampledRootErrorMm=maximum_error,
                  nativeWholeRootErrorUpperMm=receipt['rootErrorUpperMm'], exactlySharedEndpoints=True,
                  twoOppositeUsesPerSavedEdge=True, closedSolidClaim=False,
                  completeSurfaceIntersectionClaim=False, passed=True)
    (directory / 'independent-samples.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main(Path(sys.argv[1]))
