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


def polynomial_surface_flux(surface):
    """Exact integral r dot (r_u cross r_v)/3 for a full polynomial patch."""
    p, q = surface['degreeU'], surface['degreeV']
    assert 1 <= p <= 6 and 1 <= q <= 6
    assert not surface.get('periodicU',False) and not surface.get('periodicV',False)
    assert surface['knotsU'] == [0]*(p+1)+[1]*(p+1)
    assert surface['knotsV'] == [0]*(q+1)+[1]*(q+1)
    poles = [[list(map(F, point)) for point in row] for row in surface['controlPoints']]
    assert len(poles) == p+1 and all(len(row) == q+1 for row in poles)
    weights = surface['weights']
    assert weights[0][0] > 0 and all(w == weights[0][0] for row in weights for w in row)
    du = [[[p*(poles[i+1][j][k]-poles[i][j][k]) for k in range(3)] for j in range(q+1)] for i in range(p)]
    dv = [[[q*(poles[i][j+1][k]-poles[i][j][k]) for k in range(3)] for j in range(q)] for i in range(p+1)]
    total = F(0)
    for i in range(p+1):
        for j in range(p):
            for k in range(p+1):
                iu = F(math.comb(p,i)*math.comb(p-1,j)*math.comb(p,k), 3*p*math.comb(3*p-1,i+j+k))
                for a in range(q+1):
                    for b in range(q+1):
                        for c in range(q):
                            iv = F(math.comb(q,a)*math.comb(q,b)*math.comb(q-1,c), 3*q*math.comb(3*q-1,a+b+c))
                            total += iu*iv*determinant(poles[i][a],du[j][b],dv[k][c])/3
    return total


def polynomial_cap_flux(curves):
    """Planar cap flux from its outward-oriented polynomial boundary loops."""
    area = [F(0)]*3
    points = []
    ring_start = previous_end = None
    for curve in curves:
        p = curve['degree']
        assert not curve.get('periodic',False)
        assert 1 <= p <= 6 and curve['knots'] == [0]*(p+1)+[1]*(p+1)
        poles = [list(map(F, point)) for point in curve['controlPoints']]
        assert len(poles) == p+1 and all(len(point) == 3 for point in poles)
        weights = curve['weights']
        assert weights[0] > 0 and all(w == weights[0] for w in weights)
        if ring_start is None:
            ring_start = poles[0]
        else:
            assert previous_end == poles[0], 'Open cap contour join'
        previous_end = poles[-1]
        if previous_end == ring_start:
            ring_start = previous_end = None
        points.extend(poles)
        for i in range(p+1):
            for j in range(p):
                delta = [p*(poles[j+1][k]-poles[j][k]) for k in range(3)]
                integral = F(math.comb(p,i)*math.comb(p-1,j), 2*p*math.comb(2*p-1,i+j))
                product = [poles[i][1]*delta[2]-poles[i][2]*delta[1],
                           poles[i][2]*delta[0]-poles[i][0]*delta[2],
                           poles[i][0]*delta[1]-poles[i][1]*delta[0]]
                area = [area[k]+integral*product[k]/2 for k in range(3)]
    assert ring_start is None, 'Open cap contour'
    assert points and any(area)
    origin = points[0]
    assert all(sum(area[k]*(point[k]-origin[k]) for k in range(3)) == 0 for point in points), 'Nonplanar cap boundary'
    return sum(origin[k]*area[k] for k in range(3))/3


def polynomial_boundary_volume(case):
    assert len(case['wallSurfaces']) == len(case['wallFaceReversed'])
    walls = sum(((-1 if reversed_face else 1)*polynomial_surface_flux(surface)
                 for surface, reversed_face in zip(case['wallSurfaces'],case['wallFaceReversed'])), F(0))
    caps = sum((polynomial_cap_flux(curves) for curves in case['outwardCapContours']), F(0))
    return walls+caps


def self_test_polynomial_volume():
    def case_for_rings(rings, z=0):
        walls = []
        caps = [[],[]]
        for ring in rings:
            for i,a in enumerate(ring):
                b = ring[(i+1)%len(ring)]
                walls.append(dict(degreeU=1,degreeV=1,knotsU=[0,0,1,1],knotsV=[0,0,1,1],
                    controlPoints=[[[*a,z],[*a,z+1]],[[*b,z],[*b,z+1]]],weights=[[1,1],[1,1]]))
                caps[1].append(dict(degree=1,knots=[0,0,1,1],controlPoints=[[*a,z+1],[*b,z+1]],weights=[1,1]))
            reverse = list(reversed(ring))
            caps[0].extend(dict(degree=1,knots=[0,0,1,1],
                controlPoints=[[*a,z],[*reverse[(i+1)%len(reverse)],z]],weights=[1,1]) for i,a in enumerate(reverse))
        return dict(wallSurfaces=walls,wallFaceReversed=[False]*len(walls),outwardCapContours=caps)
    outer = [(0,0),(1,0),(1,1),(0,1)]
    hole = [(F(1,4),F(1,4)),(F(1,4),F(3,4)),(F(3,4),F(3,4)),(F(3,4),F(1,4))]
    assert polynomial_boundary_volume(case_for_rings([outer])) == 1
    assert polynomial_boundary_volume(case_for_rings([outer,hole])) == F(3,4)
    assert polynomial_boundary_volume(case_for_rings([outer,hole],7)) == F(3,4)
    bad = case_for_rings([outer])['outwardCapContours'][1]
    bad[0]['controlPoints'][0][2] += 1
    try:
        polynomial_cap_flux(bad)
    except AssertionError:
        pass
    else:
        raise AssertionError('Open/nonplanar cap was accepted')
    print('Exact polynomial reference: cube=1, hollow=3/4, translated hollow=3/4; invalid cap refused')


def main():
    path = Path(sys.argv[1])/'manifest.json'
    manifest = json.loads(path.read_text())
    for case in manifest['cases']:
        if case.get('volumeReferenceMethod') == 'analytic-hollow-rectangular-prism':
            dimensions = case['analyticPrism']
            outer, hole, scale = (list(map(F, dimensions[key])) for key in ('outer', 'hole', 'axisScale'))
            height = F(dimensions['height'])
            assert len(outer) == len(hole) == len(scale) == 2
            assert height > 0 and all(scale[i] > 0 and 0 < hole[i] < outer[i] for i in range(2))
            volume = (outer[0]*outer[1]-hole[0]*hole[1])*scale[0]*scale[1]*height
            assert float(volume) == case['expectedVolume'], 'Authored prism volume mismatch'
            case['volumeReference'] = {'method': case['volumeReferenceMethod'],
                'numerator': str(volume.numerator), 'denominator': str(volume.denominator),
                'scope': 'exact independent authored rectangular area minus hole, affine area scale and path height; no kernel or STEP calls'}
            print(case['file'], float(volume))
            continue
        if case.get('volumeReferenceMethod') == 'polynomial-boundary-divergence-integral':
            volume = polynomial_boundary_volume(case)
            assert volume > 0
            case['expectedVolume'] = float(volume)
            case['volumeReference'] = {'method': case['volumeReferenceMethod'],
                'numerator': str(volume.numerator), 'denominator': str(volume.denominator),
                'scope': 'exact retained polynomial wall flux and exact planar cap boundary flux; no kernel or STEP calls'}
            print(case['file'], case['expectedVolume'])
            continue
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
    staged = path.with_suffix('.json.writing')
    with staged.open('w') as output:
        json.dump(manifest, output, separators=(',', ':'))
        output.write('\n')
    staged.replace(path)


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        self_test_polynomial_volume()
    else:
        main()
