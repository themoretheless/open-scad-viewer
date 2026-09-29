"""Exact Fraction oracle for transformed rational planes and saddle contacts.
Validates original control nets and separability before deriving the contact.
No production evaluator, interval arithmetic, Newton solver or mesh is reused.
"""
import json
import sys
from fractions import Fraction as F


def contains(interval, x):
    return F(interval[0]) <= x <= F(interval[1])


def scalar(s, axis, parameter):
    knots = s['knotsU' if axis == 0 else 'knotsV']
    t = (parameter-F(knots[0]))/(F(knots[-1])-F(knots[0]))
    w = s['weights']
    ratio = F(w[1][0] if axis == 0 else w[0][1])/F(w[0][0])
    return ratio*t/(1-t+ratio*t)


def inverse(s, axis, coordinate):
    knots = s['knotsU' if axis == 0 else 'knotsV']
    w = s['weights']
    ratio = F(w[1][0] if axis == 0 else w[0][1])/F(w[0][0])
    t = coordinate/(ratio+(1-ratio)*coordinate)
    return F(knots[0])+(F(knots[-1])-F(knots[0]))*t


cases = json.load(open(sys.argv[1]))['cases']
assert len(cases) == 108
contacts = separated = 0
for c in cases:
    matrix = [[F(x) for x in row] for row in c['matrix']]
    det = sum(matrix[0][i]*(matrix[1][(i+1)%3]*matrix[2][(i+2)%3]-matrix[1][(i+2)%3]*matrix[2][(i+1)%3]) for i in range(3))
    assert det != 0
    for second, s in enumerate([c['a'],c['b']]):
        assert s['degreeU'] == s['degreeV'] == 1
        assert not s['periodicU'] and not s['periodicV']
        for knots in [s['knotsU'],s['knotsV']]:
            assert len(knots)==4 and knots[0]==knots[1]<knots[2]==knots[3]
        w = [[F(x) for x in row] for row in s['weights']]
        assert len(w)==2 and all(len(row)==2 for row in w)
        for i in range(2):
            for j in range(2):
                assert w[i][j]>0 and w[i][j]*w[0][0]==w[i][0]*w[0][j]
                p = [F(i),F(3,8),F(j)+(F(2) if c['separated'] else -F(5,8))] if second else [F(i),F(j),F(0)]
                if second and c["curved"]:
                    p = [F(i),F(j),F(i*j)-F(3,16)]
                for k in range(3):
                    expected = F(c['offset'][k])+sum(matrix[k][l]*p[l] for l in range(3))
                    assert F(s['controlPoints'][i][j][(k+c['rotation'])%3]) == expected
    if c['separated']:
        # Canonical z ranges are {0} and [2,3]; invertible affine maps
        # preserve disjointness, irrespective of their spatial AABB overlap.
        assert c['absenceProven'] and c['witness'] is None and c['unresolved']==0
        separated += 1
        continue
    w = c['witness']
    assert w is not None and not c['absenceProven'] and c['unresolved']>0
    assert 0 <= F(w['contractionUpper']) < F(1,2)
    fixed_axes = [k for k in range(2) if w['firstUv'][k][0]==w['firstUv'][k][1]]
    assert len(fixed_axes)==1
    axis=fixed_axes[0]
    coordinate=scalar(c['a'],axis,F(w['firstUv'][axis][0]))
    if c['curved']:
        # Separable weights reproduce (U,V,U*V-3/16) exactly.
        assert coordinate>0
        u,v=(coordinate,F(3,16)/coordinate) if axis==0 else (F(3,16)/coordinate,coordinate)
        uv_b=[inverse(c['b'],0,u),inverse(c['b'],1,v)]
    else:
        assert axis==0
        u,v=coordinate,F(3,8)
        uv_b=[inverse(c['b'],0,u),inverse(c['b'],1,F(5,8))]
    uv_a=[inverse(c['a'],0,u),inverse(c['a'],1,v)]
    for boxes, exact in [(w['firstUv'],uv_a),(w['secondUv'],uv_b)]:
        assert all(contains(box,x) for box,x in zip(boxes,exact))
    p = [u,v,F(0)]
    for k in range(3):
        x = F(c['offset'][k])+sum(matrix[k][l]*p[l] for l in range(3))
        assert contains(w['point'][(k+c['rotation'])%3],x)
    contacts += 1
print(json.dumps({'cases':len(cases),'exactContacts':contacts,'exactSeparated':separated,'maximumCells':max(c['cells'] for c in cases)}))
