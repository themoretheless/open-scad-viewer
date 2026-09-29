"""Independent exact oracle for separable rational graph charts.

Positive separable weights make each projected scalar coordinate the weighted
Bernstein mean of strictly increasing i/degree. Its derivative on (0,1) is a
positive covariance, hence it is strictly increasing. An invertible affine
map preserves injectivity. The third coordinate may have arbitrary curvature.
"""
import json
import sys
from fractions import Fraction as F
cases = json.load(open(sys.argv[1]))['cases']
assert len(cases) == 72
for c in cases:
    s, axes = c['surface'], c['axes']
    assert s['degreeU'] == s['degreeV'] == 2
    assert len(s['controlPoints']) == 3 and all(len(row) == 3 for row in s['controlPoints'])
    assert len(axes) == 2 and len(set(axes)) == 2 and all(a in (0,1,2) for a in axes)
    assert not s['periodicU'] and not s['periodicV']
    for knots in (s['knotsU'], s['knotsV']):
        assert len(knots) == 6 and knots[:3] == [knots[0]]*3 and knots[3:] == [knots[3]]*3 and F(knots[0]) < F(knots[3])
    a = [[F(x) for x in row] for row in c['matrix']]
    b = list(map(F,c['offset']))
    assert a[0][0]*a[1][1]-a[0][1]*a[1][0] != 0
    w = [[F(x) for x in row] for row in s['weights']]
    for i in range(3):
        for j in range(3):
            assert w[i][j] > 0 and w[i][j]*w[0][0] == w[i][0]*w[0][j]
            for k in range(2):
                assert F(s['controlPoints'][i][j][axes[k]]) == a[k][0]*F(i,2)+a[k][1]*F(j,2)+b[k]
    assert c['proven'] and c['reason'] == 'global-projection-contraction'
    assert 0 <= F(c['contractionUpper']) < 1 and c['spans'] == 1
print(json.dumps({'cases':len(cases),'independentlyInjective':len(cases),'maximumContractionUpper':max(c['contractionUpper'] for c in cases)}))
