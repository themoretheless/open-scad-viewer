"""Independent volume reference for canonical polynomial section generators.

For P(v,x,y)=C(v)+A(v)x+B(v)y, integrating its Jacobian over
the unit disk removes odd terms, leaving pi * integral det(C',A,B).
Reversed inner loops contribute negative determinants. All polynomial
coefficients and integrals below use exact fractions of authored binary64.
This reference never imports STEP, invokes the kernel, or samples surfaces.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path


def determinant(a, b, c):
    return (a[0]*(b[1]*c[2]-b[2]*c[1])
            - a[1]*(b[0]*c[2]-b[2]*c[0])
            + a[2]*(b[0]*c[1]-b[1]*c[0]))


def section_integral(patches, middle_weight=math.sqrt(.5)):
    n = patches[0]['degreeV']
    assert n >= 1 and len(patches) == 4
    for s in patches:
        assert s['degreeU'] == 2 and s['degreeV'] == n
        assert s['knotsU'] == [0, 0, 0, 1, 1, 1]
        assert s['knotsV'] == [0]*(n+1)+[1]*(n+1)
        assert len(s['controlPoints']) == 3
        assert all(len(row) == n+1 for row in s['controlPoints'])
        assert s['weights'] == [[w]*(n+1) for w in [1, middle_weight, 1]]
    points = [[[list(map(F, p)) for p in row] for row in s['controlPoints']]
              for s in patches]
    centres, axes_a, axes_b = [], [], []
    signs = [[(1, 0), (1, 1), (0, 1)],
             [(0, 1), (-1, 1), (-1, 0)],
             [(-1, 0), (-1, -1), (0, -1)],
             [(0, -1), (1, -1), (1, 0)]]
    for v in range(n+1):
        c = [(points[0][0][v][k]+points[2][0][v][k])/2 for k in range(3)]
        a = [points[0][0][v][k]-c[k] for k in range(3)]
        b = [points[1][0][v][k]-c[k] for k in range(3)]
        for q in range(4):
            for u, (x, y) in enumerate(signs[q]):
                assert points[q][u][v] == [c[k]+x*a[k]+y*b[k] for k in range(3)], 'Noncanonical retained poles'
        centres.append(c)
        axes_a.append(a)
        axes_b.append(b)
    dc = [[n*(centres[i+1][k]-centres[i][k]) for k in range(3)] for i in range(n)]
    total = F(0)
    for i in range(n):
        for j in range(n+1):
            for k in range(n+1):
                integral = F(math.comb(n-1, i)*math.comb(n, j)*math.comb(n, k),
                             3*n*math.comb(3*n-1, i+j+k))
                total += integral*determinant(dc[i], axes_a[j], axes_b[k])
    return total



def conic_area(weight):
    """Area of four symmetric quadratic arcs with weights [1,w,1].

    Green's theorem gives twice integral_0^1
    (2w-2(2w-1)t(1-t))/(1+2(w-1)t(1-t))**2 dt.
    Near w=1 use its convergent beta-integral series with exact fractions;
    elsewhere evaluate the analytic acos/acosh primitive.
    """
    assert math.isfinite(weight) and weight > 0
    if abs(weight-1) <= .01:
        w = F(weight)
        total = F(0)
        for k in range(24):
            beta = F(math.factorial(k)**2, math.factorial(2*k+1))
            next_beta = F(math.factorial(k+1)**2, math.factorial(2*k+3))
            total += (-1)**k*(k+1)*(2*(w-1))**k*(2*w*beta-2*(2*w-1)*next_beta)
        # |2(w-1)t(1-t)| <= .005: the omitted series is below 1e-50.
        return float(2*total)
    if weight > 1:
        inv2 = (1/weight)**2
        integral = (2-inv2-inv2*math.acosh(weight)/math.sqrt(1-inv2))/(1-inv2)
    else:
        square = weight*weight
        integral = (2*square-1-weight*math.acos(weight)/math.sqrt(1-square))/(square-1)
    area = 2*integral
    assert math.isfinite(area) and 2 <= area <= 4
    return area


def main():
    path = Path(sys.argv[1])/'manifest.json'
    manifest = json.loads(path.read_text())
    for case in manifest['cases']:
        if case.get('volumeReferenceMethod') not in ('canonical-generator-polynomial-integral', 'conic-generator-polynomial-integral'):
            continue
        surfaces = case['wallSurfaces']
        assert len(surfaces) % 4 == 0
        if case['volumeReferenceMethod'] == 'conic-generator-polynomial-integral':
            terms = []
            for i in range(0, len(surfaces), 4):
                weight = surfaces[i]['weights'][1][0]
                coefficient = section_integral(surfaces[i:i+4], weight)
                terms.append((coefficient, weight, conic_area(weight)))
            case['expectedVolume'] = math.fsum(float(c)*area for c, _, area in terms)
            assert case['expectedVolume'] > 0
            case['volumeReference'] = {'method': case['volumeReferenceMethod'],
                'scope': 'exact polynomial generators; independent Green-theorem conic area primitive',
                'terms': [{'numerator': str(c.numerator), 'denominator': str(c.denominator),
                           'middleWeight': w, 'sectionArea': area} for c, w, area in terms]}
            print(case['file'], case['expectedVolume'])
            continue
        coefficient = sum((section_integral(surfaces[i:i+4]) for i in range(0, len(surfaces), 4)), F(0))
        assert coefficient > 0
        case['expectedVolume'] = math.pi*float(coefficient)
        case['volumeReference'] = {'piCoefficientNumerator': str(coefficient.numerator),
                                  'piCoefficientDenominator': str(coefficient.denominator),
                                  'method': case['volumeReferenceMethod'],
                                  'scope': 'canonical circular weights rounded to binary64; exact polynomial generators'}
        print(case['file'], case['expectedVolume'])
    path.write_text(json.dumps(manifest, indent=2)+'\n')


if __name__ == '__main__':
    main()
