"""Independent exact-rational oracle for the plane lift corpus.

Identical xy control points, knots and positive weights prove identical xy
functions for the whole domain. Constant z coefficients prove distance |z|.
No native evaluator or floating point sampling is used.
"""
import json
import sys
from fractions import Fraction as F

cases = json.load(open(sys.argv[1]))['cases']
assert len(cases) in (30, 45, 90)
valid = mismatch = 0
for case in cases:
    c, p, s = (case[k] for k in ('curve', 'pcurve', 'surface'))
    assert s['degreeU'] == s['degreeV'] == 1
    assert s['knotsV'] == [0, 0, 1, 1]
    assert s['knotsU'] in ([0, 0, 1, 1], [0, 0, 0.3, 1, 1])
    for i, row in enumerate(s['controlPoints']):
        assert row == [[s['knotsU'][i+1], 0, 0], [s['knotsU'][i+1], 1, 0]]
    assert all(row == [1, 1] for row in s['weights'])
    assert c['knots'] == p['knots'] and c['weights'] == p['weights']
    assert c['degree'] == p['degree'] and c['degree'] in (1, 2)
    assert all(F(w) > 0 for w in c['weights'])
    assert [point[:2] for point in c['controlPoints']] == p['controlPoints']
    assert all(0 <= F(x) <= 1 for point in p['controlPoints'] for x in point)
    z = F(c['controlPoints'][0][2])
    assert all(F(point[2]) == z for point in c['controlPoints'])
    distance = abs(z)
    assert 0 < case['cells'] <= 4096
    if distance <= F(case['tolerance']):
        assert case['status'] == 'WithinTolerance'
        assert case['witness'] is None and case['distance'] is None
        valid += 1
    else:
        assert case['status'] == 'Mismatch'
        assert 0 <= F(case['witness']) <= 1
        lo, hi = map(F, case['distance'])
        assert F(case['tolerance']) < lo <= distance <= hi
        mismatch += 1
print(json.dumps({'cases': len(cases), 'withinTolerance': valid, 'mismatch': mismatch,
                  'maxCells': max(c['cells'] for c in cases)}))
