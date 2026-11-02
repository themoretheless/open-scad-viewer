"""Independent Fraction recomposition of actual native negative witnesses.

Imports the previously qualified polynomial/point oracle, never Rust flags.
This validates source points, not positive global embedding or arbitrary roots.
"""
import hashlib
import json
import runpy
import sys
from fractions import Fraction as F
from pathlib import Path

base = Path('docs/qualification/closed-contact-material-2026-11-03/exact_witness.py')
lib = runpy.run_path(str(base))
at, tangent, surface_at, domain = (lib[k] for k in ('at', 'tangent', 'surface_at', 'domain'))
records = []
for line in Path(sys.argv[1]).read_text().splitlines():
    if 'exact-contact-witness=' in line:
        records.append(json.loads(line.split('exact-contact-witness=', 1)[1]))
assert len(records) == 7
covered = set(); bound_spans = 0; rows = []
for r in records:
    p, g = r['path'], r['guide']; axis = r['axis']; sign = F(r['normalSign'])
    assert not p['periodic'] and not g['periodic']
    assert p['degree'] == g['degree'] and p['knots'] == g['knots'] and p['weights'] == g['weights']
    assert len(p['controlPoints']) == len(g['controlPoints']) == len(p['weights'])
    offset = [F(b)-F(a) for a,b in zip(p['controlPoints'][0], g['controlPoints'][0])]
    assert [k for k in range(3) if offset[k] != 0] == [axis]
    assert sign == (1 if offset[axis] > 0 else -1)
    assert all(F(q[axis]) == F(p['controlPoints'][0][axis]) for q in p['controlPoints'])
    assert all([F(b)-F(a) for a,b in zip(x,y)] == offset for x,y in zip(p['controlPoints'],g['controlPoints']))
    assert all(F(q[0]) == 0 for q in r['twist']['controlPoints'])
    width = abs(offset[axis]); start = at(p, F(0))
    initial_tangent = tangent(p, F(0))
    assert initial_tangent[axis] == 0 and any(x != 0 for x in initial_tangent)
    ref = r['profiles'][0]; lo = F(ref['knots'][ref['degree']]); hi = F(ref['knots'][len(ref['controlPoints'])])
    parameter = F(r.get('anchorParameter', lo))
    anchor_point = at(ref, (parameter-lo)/(hi-lo))
    anchor = sign * (anchor_point[axis]-start[axis]); assert anchor > 0
    rational_anchor = 'anchorParameter' in r
    if rational_anchor:
        assert anchor == F(27,1280) and anchor != width
    else:
        assert anchor == width
        covered.add((axis,sign)); bound_spans += lib['verify_binding'](r)
        lib['canonical_ring'](r['trims'][0], [F(1),F(4)], F(2), False)
        lib['canonical_ring'](r['trims'][1], [F(1),F(4)], F(1,2), True)
    a, b = F(p['knots'][p['degree']]), F(p['knots'][len(p['controlPoints'])])
    raw = list(map(F,r['rawPathParameters']))
    assert a <= raw[0] < raw[1] <= b and raw != [a,b]
    points = []; uv_addresses = []
    for t0, proposed in zip(raw,r['capNormalizedUV']):
        t = (t0-a)/(b-a); pos = at(p,t); d = tangent(p,t)
        assert d[axis] == 0 and any(x != 0 for x in d)
        uv = list(map(F,proposed)); assert all(0 <= x <= 1 for x in uv)
        authored_uv = [lo+(hi-lo)*q for q,(lo,hi) in zip(uv,domain(r['surface']))]
        foot = surface_at(r['surface'],authored_uv)
        assert all(foot[k] == start[k] for k in range(3) if k != axis)
        normal = sign*(foot[axis]-start[axis])
        scale = at(r['scale'],t)[0]; axes = at(r['axes'],t); center = at(r['center'],t)
        assert scale > 0 and axes[0] > 0 and center[1:] == [0,0]
        den = scale*axes[0]*anchor+center[0]; assert den > 0
        world = pos[:]; world[axis] += sign*width/den*(scale*axes[0]*normal+center[0])
        points.append(world); uv_addresses.append(list(map(str,authored_uv)))
        if not rational_anchor:
            # Exact canonical quarter-ring Jordan regions from the base oracle.
            assert authored_uv[1] == 4 and F(1,2) < abs(authored_uv[0]-1) < 2
    assert points[0] == points[1] == list(map(F,r['world']))
    rows.append({'axis':axis,'sign':int(sign),'rationalInteriorAnchor':rational_anchor,
                 'rawPathParameters':list(map(str,raw)),'authoredUV':uv_addresses,
                 'world':list(map(str,points[0]))})
assert covered == {(a,s) for a in range(3) for s in [-1,1]}
assert sum(r['rationalInteriorAnchor'] for r in rows) == 1
print(json.dumps({'nativeWitnessesVerified':len(rows),'exactMaterialSpanIdentities':bound_spans,
    'allAxesAndSigns':True,'roundedAnchorRejectedAsAuthority':True,'witnesses':rows,
    'oracleDependencySha256':hashlib.sha256(base.read_bytes()).hexdigest(),
    'scope':'Negative original filled-map coincidences; six are also actual material interior collisions.'},indent=2))
