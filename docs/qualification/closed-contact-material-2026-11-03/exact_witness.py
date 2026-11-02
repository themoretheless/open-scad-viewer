"""Independent Fraction composition identities and actual material collision.

Consumes original curve/surface definitions emitted by the public native test.
No Rust report flags, numerical evaluator or sampled fit supplies an identity.
"""
import copy
import hashlib
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

def clean(p):
    while len(p) > 1 and p[-1] == 0:
        p.pop()
    return p

def add(a, b):
    return clean([(a[i] if i < len(a) else F(0)) +
                  (b[i] if i < len(b) else F(0)) for i in range(max(len(a), len(b)))])

def scale(a, s):
    return clean([x * s for x in a])

def mul(a, b):
    p = [F(0)] * (len(a) + len(b) - 1)
    for i, x in enumerate(a):
        for j, y in enumerate(b):
            p[i + j] += x * y
    return clean(p)

def bernstein(a):
    n = len(a) - 1
    p = [F(0)] * (n + 1)
    for i, x in enumerate(a):
        for j in range(n - i + 1):
            p[i + j] += x * math.comb(n, i) * math.comb(n - i, j) * (-1) ** j
    return clean(p)

def channels(c):
    p = c['degree']; cp = c['controlPoints']; k = list(map(F, c['knots']))
    assert not c['periodic'] and len(cp) == p + 1
    assert k == [k[0]] * (p + 1) + [k[-1]] * (p + 1) and k[0] < k[-1]
    w = list(map(F, c['weights'])); assert len(w) == len(cp) and all(x > 0 for x in w)
    return [bernstein([F(q[a]) * v for q, v in zip(cp, w)])
            for a in range(len(cp[0]))] + [bernstein(w)]

def domain(s):
    return [(F(s['knotsU'][s['degreeU']]), F(s['knotsU'][len(s['controlPoints'])])),
            (F(s['knotsV'][s['degreeV']]), F(s['knotsV'][len(s['controlPoints'][0])]))]

def carrier(s):
    assert s['degreeU'] == s['degreeV'] == 1
    assert len(s['controlPoints']) == 2 and all(len(row) == 2 for row in s['controlPoints'])
    assert all(len(p) == 3 for row in s['controlPoints'] for p in row)
    assert len(s['weights']) == 2 and all(len(row) == 2 for row in s['weights'])
    weights = [F(x) for row in s['weights'] for x in row]
    assert weights[0] > 0 and all(x == weights[0] for x in weights)
    return [[[F(x) for x in p] for p in row] for row in s['controlPoints']]

def lift_equal(world, uv, s):
    xyz = channels(world); a, b, d = channels(uv); net = carrier(s)
    (u0, u1), (v0, v1) = domain(s)
    u = scale(add(a, scale(d, -u0)), 1 / (u1 - u0))
    v = scale(add(b, scale(d, -v0)), 1 / (v1 - v0))
    us = [add(d, scale(u, -1)), u]; vs = [add(d, scale(v, -1)), v]
    for axis in range(3):
        image = [F(0)]
        for i in range(2):
            for j in range(2):
                image = add(image, scale(mul(us[i], vs[j]), net[i][j][axis]))
        if mul(xyz[axis], mul(d, d)) != mul(image, xyz[-1]):
            return False
    return True

def verify_binding(r):
    uv = [c for ring in r['trims'] for c in ring]
    assert len(uv) == len(r['profiles']) == 8
    assert all(lift_equal(c, p, r['surface']) for c, p in zip(r['profiles'], uv))
    return len(uv)

def deboor(p, k, rows, t):
    n = len(rows) - 1
    i = n if t == k[n + 1] else max(j for j in range(p, n + 1) if k[j] <= t < k[j + 1])
    d = [list(row) for row in rows[i - p:i + 1]]
    for r in range(1, p + 1):
        for j in range(p, r - 1, -1):
            den = k[i + j - r + 1] - k[i - p + j]; assert den > 0
            a = (t - k[i - p + j]) / den
            d[j] = [(1 - a) * x + a * y for x, y in zip(d[j - 1], d[j])]
    return d[p]

def at(c, t):
    p = c['degree']; k = list(map(F, c['knots'])); n = len(c['controlPoints'])
    raw = k[p] + t * (k[n] - k[p])
    rows = [[F(x) * F(w) for x in cp] + [F(w)] for cp, w in zip(c['controlPoints'], c['weights'])]
    h = deboor(p, k, rows, raw); assert h[-1] > 0
    return [x / h[-1] for x in h[:-1]]

def tangent(c, t):
    p = c['degree']; k = list(map(F, c['knots'])); cp = c['controlPoints']
    assert all(F(w) == F(c['weights'][0]) for w in c['weights'])
    rows = [[p * (F(b) - F(a)) / (k[i + p + 1] - k[i + 1]) for a, b in zip(cp[i], cp[i + 1])]
            for i in range(len(cp) - 1)]
    return deboor(p - 1, k[1:-1], rows, k[p] + t * (k[len(cp)] - k[p]))

def surface_at(s, uv):
    net = carrier(s); ranges = domain(s)
    a, b = [(x - lo) / (hi - lo) for x, (lo, hi) in zip(uv, ranges)]
    return [sum(net[i][j][axis] * [1 - a, a][i] * [1 - b, b][j]
                for i in range(2) for j in range(2)) for axis in range(3)]

def canonical_ring(ring, center, radius, reverse):
    signs = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1), (1, 0)]
    expected = [[[center[a] + radius * p[a] for a in range(2)] for p in signs[2*i:2*i+3]] for i in range(4)]
    if reverse:
        expected = [list(reversed(p)) for p in reversed(expected)]
    assert len(ring) == 4
    for c, poles in zip(ring, expected):
        channels(c)
        assert c['degree'] == 2 and list(map(F, c['weights'])) == [1, F(1, 2), 1]
        assert [[F(x) for x in p] for p in c['controlPoints']] == poles
    # On each quadrant x=(1-t)/W, y=t/W, W=1-t+t²>0.
    # x'<0 and y'>0 in (0,1), giving simple disjoint quadrant arcs;
    # the horizontal center-line interior is exactly between the axis endpoints.

def collision(r):
    assert r['selectedClock'] == 'parameter' and r['normal'] == [0., 0., 1.]
    assert all(F(q[0]) == 0 for q in r['twist']['controlPoints'])
    p, g = r['path'], r['guide']; assert p['degree'] == g['degree']
    assert p['knots'] == g['knots'] and p['weights'] == g['weights']
    assert len(p['controlPoints']) == len(g['controlPoints']) == len(p['weights'])
    radius = F(g['controlPoints'][0][2]) - F(p['controlPoints'][0][2]); assert radius > 0
    assert all(F(q[2]) == 0 for q in p['controlPoints'])
    assert all([F(b) - F(a) for a, b in zip(x, y)] == [0, 0, radius]
               for x, y in zip(p['controlPoints'], g['controlPoints']))
    assert [F(x) for x in r['profiles'][0]['controlPoints'][0]] == [0, 0, radius]
    canonical_ring(r['trims'][0], [F(1), F(4)], F(2), False)
    canonical_ring(r['trims'][1], [F(1), F(4)], F(1, 2), True)
    target = radius / 2; points = []; uvs = []
    for t in [F(0), F(1, 2)]:
        pos = at(p, t); d = tangent(p, t)
        assert pos == [0, 0, 0] and d[2] == 0 and any(x != 0 for x in d)
        scale_value = at(r['scale'], t)[0]; axes = at(r['axes'], t); center = at(r['center'], t)
        assert scale_value > 0 and axes[0] > 0 and center[1:] == [0, 0]
        den = scale_value * axes[0] * radius + center[0]; assert den > 0
        normal = (target * den / radius - center[0]) / (scale_value * axes[0])
        uv = [1 + 2 * normal / radius, F(4)]
        assert F(3, 2) < uv[0] < 3  # Strictly inside outer ring, outside hole.
        assert surface_at(r['surface'], uv) == [0, 0, normal]
        points.append([pos[0], pos[1], pos[2] + radius / den * (scale_value * axes[0] * normal + center[0])])
        uvs.append(list(map(str, uv)))
    assert uvs[0] != uvs[1] and points[0] == points[1]
    return {'stations': ['0', '1/2'], 'uv': uvs, 'world': list(map(str, points[0]))}

if __name__ == '__main__':
    records = {}
    for line in Path(sys.argv[1]).read_text().splitlines():
        if 'contact-material-source=' in line:
            r = json.loads(line.split('contact-material-source=', 1)[1]); records[r['label']] = r
    assert set(records) == {'positive', 'collision'}
    counts = {key: verify_binding(r) for key, r in records.items()}
    # The native rank-loss control collapses this same Cartesian coordinate.
    rank = copy.deepcopy(records['positive'])
    for row in rank['surface']['controlPoints']:
        for p in row:
            p[1] = 0.
    for c in rank['profiles']:
        for p in c['controlPoints']:
            p[1] = 0.
    rank_spans = verify_binding(rank)
    # At u=1 the degree-one v restriction has two identical exact endpoints:
    # the entire restriction is constant, so its v derivative is identically 0.
    assert surface_at(rank['surface'], [F(1), F(2)]) == [0, 0, 0]
    assert surface_at(rank['surface'], [F(1), F(6)]) == [0, 0, 0]
    bad = copy.deepcopy(records['positive']); bad['profiles'][0]['controlPoints'][1][0] = math.nextafter(bad['profiles'][0]['controlPoints'][1][0], math.inf)
    assert not lift_equal(bad['profiles'][0], bad['trims'][0][0], bad['surface'])
    bad = copy.deepcopy(records['positive']); bad['trims'][1][0]['controlPoints'][1][0] = math.nextafter(bad['trims'][1][0]['controlPoints'][1][0], math.inf)
    assert not lift_equal(bad['profiles'][4], bad['trims'][1][0], bad['surface'])
    print(json.dumps({'exactBoundSpans': counts, 'oneUlpNegativeControls': 2, 'derivedRankControlExactBoundSpans': rank_spans, 'carrierDerivativeVZeroOnU1': True, 'actualMaterialCollision': collision(records['collision']),
        'sourcePayloadSha256': hashlib.sha256(json.dumps(records, sort_keys=True).encode()).hexdigest(),
        'scope': 'Exact source composition, derived rank loss and one actual material collision; no independent positive global embedding claim.'}, indent=2))
