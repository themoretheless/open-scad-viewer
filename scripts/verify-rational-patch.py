"""Independent binary64 B-spline evaluation of the Solid quarter-cylinder fixture."""
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
assert all(math.isfinite(w) and w > 0 for row in weights for w in row)
max_radius_error = max_height_error = 0.
for i in range(65):
    u = i / 64
    bu = basis(surface['knotsU'], surface['degreeU'], len(points), u)
    for j in range(33):
        v = j / 32
        bv = basis(surface['knotsV'], surface['degreeV'], len(points[0]), v)
        h = [0., 0., 0., 0.]
        for a, row in enumerate(points):
            for b, point in enumerate(row):
                w = bu[a] * bv[b] * weights[a][b]
                for k in range(3):
                    h[k] += w * point[k]
                h[3] += w
        p = [x / h[3] for x in h[:3]]
        assert p[0] >= -1e-12 and p[1] >= -1e-12
        max_radius_error = max(max_radius_error, abs(math.hypot(p[0], p[1]) - 1.))
        max_height_error = max(max_height_error, abs(p[2] - 2 * v))
assert max_radius_error < 1e-12 and max_height_error < 1e-12
report = {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'samples': 65 * 33,
          'maxRadiusErrorMm': max_radius_error, 'maxHeightErrorMm': max_height_error,
          'oracle': 'Independent Python Cox-de Boor evaluation against unit quarter-cylinder'}
path.with_name('independent-patch-report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
