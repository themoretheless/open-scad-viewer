"""Independent floating quadrature of original annular source coefficients.
This checks one literal-endpoint, single-Bezier fixture. Convergence is a
numerical cross-check, not a replacement for native outward interval proof.
"""
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import sys
import numpy as np


def basis(degree, t):
    t = np.asarray(t)
    b = np.stack([math.comb(degree, k)*t**k*(1-t)**(degree-k)
                  for k in range(degree+1)], axis=-1)
    low = np.stack([math.comb(degree-1, k)*t**k*(1-t)**(degree-1-k)
                    for k in range(degree)], axis=-1)
    zero = np.zeros_like(t)[..., None]
    derivative = degree*(np.concatenate([zero, low], axis=-1)
                         - np.concatenate([low, zero], axis=-1))
    return b, derivative


def domain(knots, degree, count):
    assert count == degree+1 and len(knots) == 2*(degree+1)
    lo, hi = knots[degree], knots[count]
    assert hi > lo and all(k == lo for k in knots[:count])
    assert all(k == hi for k in knots[count:])
    return lo, hi


def surface(s, u, v):
    cp, weights = np.array(s['controlPoints']), np.array(s['weights'])
    assert not s['periodicU'] and not s['periodicV'] and np.all(weights > 0)
    du = domain(s['knotsU'], s['degreeU'], cp.shape[0])
    dv = domain(s['knotsV'], s['degreeV'], cp.shape[1])
    bu, ju = basis(s['degreeU'], (u-du[0])/(du[1]-du[0]))
    bv, jv = basis(s['degreeV'], (v-dv[0])/(dv[1]-dv[0]))
    h = np.concatenate([cp*weights[..., None], weights[..., None]], axis=-1)
    val = np.einsum('ni,nj,ijk->nk', bu, bv, h)
    a = np.einsum('ni,nj,ijk->nk', ju, bv, h)/(du[1]-du[0])
    b = np.einsum('ni,nj,ijk->nk', bu, jv, h)/(dv[1]-dv[0])
    point = val[:, :3]/val[:, 3:]
    return point, (a[:, :3]-point*a[:, 3:])/val[:, 3:], (b[:, :3]-point*b[:, 3:])/val[:, 3:]


def curve(c, t):
    cp, weights = np.array(c['controlPoints']), np.array(c['weights'])
    assert not c['periodic'] and np.all(weights > 0)
    lo, hi = domain(c['knots'], c['degree'], len(cp))
    b, j = basis(c['degree'], (t-lo)/(hi-lo))
    h = np.concatenate([cp*weights[:, None], weights[:, None]], axis=-1)
    val, derivative = b@h, (j@h)/(hi-lo)
    point = val[:, :-1]/val[:, -1:]
    return point, (derivative[:, :-1]-point*derivative[:, -1:])/val[:, -1:]


def integrate(faces, order):
    nodes, weights = np.polynomial.legendre.leggauss(order)
    t, w = (nodes+1)/2, weights/2
    total = 0.
    for f in faces:
        s = f['surface']
        cp = np.array(s['controlPoints'])
        if f['wholeChartMaterial']:
            u0, u1 = domain(s['knotsU'], s['degreeU'], len(cp))
            v0, v1 = domain(s['knotsV'], s['degreeV'], cp.shape[1])
            u, v = np.meshgrid(u0+(u1-u0)*t, v0+(v1-v0)*t, indexing='ij')
            p, a, b = surface(s, u.ravel(), v.ravel())
            flux = p[:, 2]*(a[:, 0]*b[:, 1]-a[:, 1]*b[:, 0])
            total += f['chartWinding']*np.sum(flux.reshape(order, order)*w[:, None]*w[None, :])*(u1-u0)*(v1-v0)
        else:
            # All trimmed fixture faces are affine planar charts. Green's
            # boundary integral avoids relying on the native material mask.
            assert s['degreeU'] == s['degreeV'] == 1
            assert np.all(np.array(s['weights']) == 1)
            assert np.allclose(cp[0, 0]+cp[1, 1], cp[0, 1]+cp[1, 0], rtol=0, atol=0)
            if not np.all(cp[:, :, 2] == cp[0, 0, 2]):
                # A vertical affine chart has zero XY Jacobian and zero Z flux.
                # Exact binary-rational arithmetic checks this rank condition.
                a = [Fraction(float(cp[1, 0, k]))-Fraction(float(cp[0, 0, k])) for k in [0, 1]]
                b = [Fraction(float(cp[0, 1, k]))-Fraction(float(cp[0, 0, k])) for k in [0, 1]]
                assert a[0]*b[1]-a[1]*b[0] == 0, 'Unsupported tilted trimmed plane'
                continue
            for wire in f['loops']:
                for edge in wire:
                    lo, hi = edge['parameters']
                    uv, jet = curve(edge['curve'], lo+(hi-lo)*t)
                    p, a, b = surface(s, uv[:, 0], uv[:, 1])
                    tangent = a*jet[:, 0:1]+b*jet[:, 1:2]
                    total += np.dot(w, p[:, 2]*(p[:, 0]*tangent[:, 1]-p[:, 1]*tangent[:, 0])/2)*(hi-lo)
    return float(total)


def main(source, output):
    raw = source.read_bytes()
    fixture = json.loads(raw)
    assert fixture['schema'] == 'cad-original-annular-flux/1'
    values = [integrate(fixture['faces'], n) for n in [16, 32, 64]]
    lo, hi = fixture['nativeVolumeBounds']
    drift = max(abs(values[2]-v) for v in values[:2])
    passed = lo <= values[2] <= hi and drift < 1e-7
    report = dict(schema='cad-original-annular-flux-quadrature/1',
                  sourceSha256=hashlib.sha256(raw).hexdigest(), faces=len(fixture['faces']),
                  orders=[16, 32, 64], signedVolumeMm3=values,
                  convergenceDifferenceMm3=drift, nativeVolumeBoundsMm3=[lo, hi],
                  scope='Independent floating quadrature of original single-Bezier coefficients and affine planar literal-endpoint boundaries; no STEP reader or rigorous quadrature error certificate.',
                  passed=passed)
    output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2])))
