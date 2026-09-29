"""Exact piecewise-affine oracle, including the parameter seam.

Checking coefficients at both ends of every affine piece proves agreement
throughout that piece; this is not a finite sample oracle for general curves.
"""
import json
import sys
from fractions import Fraction as F
cases = json.load(open(sys.argv[1]))['cases']
assert len(cases) == 42
counts = {'WithinTolerance': 0, 'Mismatch': 0}
for case in cases:
    c, p, s = (case[k] for k in ('curve', 'pcurve', 'surface'))
    assert s['periodicU'] and not s['periodicV']
    assert s['degreeU'] == s['degreeV'] == 1
    assert s['knotsU'] == [-1, 0, 1, 2, 3, 4] and s['knotsV'] == [0, 0, 1, 1]
    assert s['weights'] == [[1, 1]] * 4
    assert s['controlPoints'] == [[[0,0,0],[0,0,1]],[[1,0,0],[1,0,1]],[[0,1,0],[0,1,1]],[[0,0,0],[0,0,1]]]
    assert c['degree'] == p['degree'] == 1
    assert c['knots'] == [0, 0, 0.5, 1, 1] and p['knots'] == [0, 0, 1, 1]
    assert c['weights'] == [1, 1, 1] and p['weights'] == [1, 1]
    assert not c['periodic'] and not p['periodic']
    uv = [[F(x) for x in row] for row in p['controlPoints']]
    assert uv[0][1] == uv[1][1] == F(1,2)
    assert uv[1][0] - uv[0][0] == 1 and uv[0][0] % 3 == F(5,2)
    points = [[F(x) for x in row] for row in c['controlPoints']]
    if case['reversed']:
        points.reverse()
    # The only chart/knot crossing is exactly t=1/2. Both lifted pieces
    # are affine, so the following three endpoint residuals determine them.
    expected = [[F(0), F(1,2), F(1,2)], [F(0),F(0),F(1,2)], [F(1,2),F(0),F(1,2)]]
    residuals = [[x-y for x,y in zip(row, ref)] for row,ref in zip(points,expected)]
    assert all(row[:2] == [0,0] for row in residuals)
    assert all(row[2] == residuals[0][2] for row in residuals)
    distance = abs(residuals[0][2])
    if distance <= F(case['tolerance']):
        assert case['status'] == 'WithinTolerance'
        assert case['witness'] is None and case['distance'] is None
    else:
        assert case['status'] == 'Mismatch'
        assert 0 <= F(case['witness']) <= 1
        lo, hi = map(F, case['distance'])
        assert F(case['tolerance']) < lo <= distance <= hi
    assert 0 < case['cells'] <= 4096
    counts[case['status']] += 1
print(json.dumps({'cases':len(cases),**counts,'maxCells':max(c['cells'] for c in cases)}))
