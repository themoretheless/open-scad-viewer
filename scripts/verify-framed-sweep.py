"""Independent binary64 B-spline evaluation of the Solid circular framed-sweep fixture."""
import hashlib
import json
import math
from pathlib import Path
import sys

path = Path(sys.argv[1])
document = json.loads(path.read_text())
assert len(document['surfaces']) == 1
surface = document['surfaces'][0]['surface']

def basis(knots, degree, count, t):
    if t == knots[count]:
        return [0.] * (count - 1) + [1.]
    values = [float(knots[i] <= t < knots[i + 1]) for i in range(len(knots) - 1)]
    for p in range(1, degree + 1):
        result = []
        for i in range(len(values) - 1):
            a, b = knots[i + p] - knots[i], knots[i + p + 1] - knots[i + 1]
            result.append(((t - knots[i]) / a * values[i] if a else 0.)
                          + ((knots[i + p + 1] - t) / b * values[i + 1] if b else 0.))
        values = result
    return values[:count]

points, weights = surface['controlPoints'], surface['weights']
assert surface['degreeU'] == surface['degreeV'] == 1
closed=bool(surface.get('periodicV'));count=len(points[0])
assert len(points) == 2 and count == (17 if closed else 32)
if closed:
    assert all(row[0] == row[-1] for row in points)
    assert all(row[0] == row[-1] for row in weights)
assert all(w == 1 for row in weights for w in row)
for i, row in enumerate(points):
    for p in row:
        assert abs(math.hypot(p[0], p[1]) - (1 + .2*i)) < 1e-12
        assert abs(p[2]) < 1e-12
    assert math.dist(row[0], [1 + .2*i, 0, 0]) < 1e-12
    assert math.dist(row[-1], [1 + .2*i, 0, 0] if closed else [0, 1 + .2*i, 0]) < 1e-12
angles=[]
for p in points[0]:
    angle=math.atan2(p[1],p[0])
    if closed and angles:
        while angle <= angles[-1]: angle += math.tau
    angles.append(angle)
assert all(b > a for a, b in zip(angles, angles[1:]))
# For this circular fixture each span is a chord: the largest radial error is
# its outer-edge sagitta. This does not certify arbitrary sweep paths.
maximum_sagitta = max(1.2 * (1 - math.cos((b-a)/2)) for a,b in zip(angles, angles[1:]))
assert maximum_sagitta < (.1 if closed else .001)
maximum_error = 0.
for i in range(17):
    u = i / 16
    bu = basis(surface['knotsU'], surface['degreeU'], len(points), u)
    for j in range(497):
        v = j / 496
        bv = basis(surface['knotsV'], surface['degreeV'], len(points[0]), v)
        p = [sum(bu[a]*bv[b]*points[a][b][k] for a in range(2) for b in range(count)) for k in range(3)]
        error = abs(math.hypot(p[0], p[1]) - (1 + .2*u))
        maximum_error = max(maximum_error, error)
assert maximum_error <= maximum_sagitta + 1e-12
seam_angle=None
if closed:
    left=[points[0][-1][k]-points[0][-2][k] for k in range(3)]
    right=[points[0][1][k]-points[0][0][k] for k in range(3)]
    cosine=sum(a*b for a,b in zip(left,right))/(math.hypot(*left)*math.hypot(*right))
    seam_angle=math.degrees(math.acos(max(-1,min(1,cosine))))
report = {'closedPath':closed,'seamContinuity':'C0' if closed else 'open','seamTangentAngleDegrees':seam_angle,'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'samples': 17*497,
          'maxSampledRadialErrorMm': maximum_error, 'circularFixtureSagittaBoundMm': maximum_sagitta,
          'generalSweepCertificate': False,
          'oracle': 'Independent Python Cox-de Boor evaluation and circular chord sagitta'}
path.with_name('independent-sweep-report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
