"""Independent rational Cox--de Boor, endpoint, and projected curvature checks."""
from fractions import Fraction as Q
from math import comb, inf, nextafter
from pathlib import Path
import json
import sys


def mul(a, b):
    terms = [Q(0)] * (len(a) + len(b) - 1)
    for i, x in enumerate(a):
        for j, y in enumerate(b):
            terms[i + j] += x * y
    return terms


def derivative(poly):
    return [i * x for i, x in enumerate(poly) if i]


def span_polynomials(curve):
    degree = curve["degree"]
    knots = list(map(Q, curve["knots"]))
    poles = [list(map(Q, p)) for p in curve["controlPoints"]]
    assert all(Q(w) == 1 for w in curve["weights"])
    for span in range(degree, len(poles)):
        start, end = knots[span:span + 2]
        if start == end:
            continue
        width = end - start
        basis = {span: [Q(1)]}
        for order in range(1, degree + 1):
            nxt = {}
            for i in range(span - order, span + 1):
                coefficients = [Q(0)] * (order + 1)
                for j, lo, hi, sign in (
                    (i, knots[i], knots[i + order], 1),
                    (i + 1, knots[i + 1], knots[i + order + 1], -1),
                ):
                    if j not in basis or lo == hi:
                        continue
                    constant = (start - lo) if sign > 0 else (hi - start)
                    factor = [constant / (hi - lo), sign * width / (hi - lo)]
                    for k, value in enumerate(mul(basis[j], factor)):
                        coefficients[k] += value
                nxt[i] = coefficients
            basis = nxt
        yield width, [
            [sum(poles[i][axis] * v[k] for i, v in basis.items())
             for k in range(degree + 1)] for axis in range(3)
        ]


def endpoint(span, side, order):
    width, xyz = span
    values = []
    for poly in xyz:
        for _ in range(order):
            poly = derivative(poly)
        values.append((sum(poly) if side else poly[0]) / width ** order)
    return values


def bernstein(poly):
    degree = len(poly) - 1
    return [
        sum(poly[j] * Q(comb(i, j), comb(degree, j)) for j in range(i + 1))
        for i in range(degree + 1)
    ]


def verify(curve):
    spans = list(span_polynomials(curve))
    for left, right in zip(spans, spans[1:] + spans[:1]):
        for order in range(5):
            assert endpoint(left, 1, order) == endpoint(right, 0, order)
    assert endpoint(spans[0], 0, 0) == [0, 0, 0]
    minima = []
    for width, (x, y, _) in spans:
        dx, dy = derivative(x), derivative(y)
        cross = [u - v for u, v in zip(
            mul(dx, derivative(dy)), mul(dy, derivative(dx))
        )]
        minimum = min(bernstein(cross)) / width ** 3
        assert minimum > 0
        minima.append(minimum)
    damaged = dict(curve, knots=curve["knots"].copy())
    damaged["knots"][1] = nextafter(damaged["knots"][1], inf)
    bad_spans = list(span_polynomials(damaged))
    start = endpoint(bad_spans[0], 0, 0)
    end = endpoint(bad_spans[-1], 1, 0)
    gap = [a - b for a, b in zip(start, end)]
    assert any(gap)
    return {"spans": len(spans), "exactEndpointOrder": 4,
            "oneUlpExteriorKnotGap": list(map(str, gap)),
            "projectedCurvatureNumeratorLower": str(min(minima))}


if __name__ == "__main__":
    prefix = "periodic-frenet-material-source="
    sources = [
        json.loads(line.split(prefix, 1)[1])
        for line in Path(sys.argv[1]).read_text().splitlines() if prefix in line
    ]
    assert len(sources) == 1
    print(json.dumps(verify(sources[0]["path"]), indent=2))
