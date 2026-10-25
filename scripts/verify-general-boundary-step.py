"""Independent finite interchange check, never a production geometry owner.

Original binary64 bilinear span coefficients give an exact Fraction divergence
integral. OCCT independently imports STEP and checks topology and signed volume.
This fixture oracle explicitly excludes trimmed regions, holes and other degrees.
"""
import hashlib
import json
import sys
from fractions import Fraction as F
from pathlib import Path

from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp


def cross(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]


def cell_volume(poles):
    a, q, r, z = [[F(x) for x in p] for p in poles]
    b = [q[k]-a[k] for k in range(3)]
    c = [r[k]-a[k] for k in range(3)]
    d = [z[k]-q[k]-r[k]+a[k] for k in range(3)]
    position = {(0, 0): a, (1, 0): b, (0, 1): c, (1, 1): d}
    normal = {(0, 0): cross(b, c), (1, 0): cross(b, d), (0, 1): cross(d, c)}
    return sum((sum(x*y for x, y in zip(p, n))/F(3*(u+s+1)*(v+t+1))
                for (u, v), p in position.items() for (s, t), n in normal.items()), F(0))


def face_volume(face):
    assert not face['holes']
    s = face['surface']
    assert s['degreeU'] == s['degreeV'] == 1
    assert not s['periodicU'] and not s['periodicV']
    p = s['controlPoints']
    assert all(w == 1 for row in s['weights'] for w in row)
    for knots, size in [(s['knotsU'], len(p)), (s['knotsV'], len(p[0]))]:
        assert len(knots) == size+2 and knots[0] == knots[1] < knots[-2] == knots[-1]
        assert all(a < b for a, b in zip(knots[1:-2], knots[2:-1]))
    return sum((cell_volume([p[i][j], p[i+1][j], p[i][j+1], p[i+1][j+1]])
                for i in range(len(p)-1) for j in range(len(p[0])-1)), F(0))


fixture_path, step_path, output = map(Path, sys.argv[1:4])
model = json.loads(fixture_path.read_text())['common']['retained']
assert len(model['shells']) == len(model['bodies']) == 1
shell = model['shells'][0]
assert shell['closed'] and len(shell['faces']) == len(model['faces'])
expected = sum(((-1 if u['reversed'] else 1)*face_volume(model['faces'][u['face']])
                for u in shell['faces']), F(0))
assert expected > 0
reader = STEPControl_Reader()
assert reader.ReadFile(str(step_path)) == IFSelect_RetDone and reader.TransferRoots() > 0
shape = reader.OneShape()
counts = []
for typ in [TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID]:
    explorer = TopExp_Explorer(shape, typ)
    n = 0
    while explorer.More():
        n += 1
        explorer.Next()
    counts.append(n)
valid = BRepCheck_Analyzer(shape).IsValid()
properties = GProp_GProps()
estimate = BRepGProp.VolumeProperties_s(shape, properties, 1e-10, True)
volume = properties.Mass()
error = abs(volume-float(expected))
passed = valid and counts == [len(model['faces']), 1, 1] and error < 1e-9
report = {'scope': 'finite independent OCCT topology and exact retained bilinear-span volume; no ideal-motion proof',
          'passed': passed, 'valid': valid, 'faces': counts[0], 'shells': counts[1], 'solids': counts[2],
          'expectedVolume': float(expected), 'expectedExact': [str(expected.numerator), str(expected.denominator)],
          'volume': volume, 'volumeError': error, 'integrationEstimate': estimate,
          'fixtureSha256': hashlib.sha256(fixture_path.read_bytes()).hexdigest(),
          'stepSha256': hashlib.sha256(step_path.read_bytes()).hexdigest(),
          'oracleSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
output.write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
assert passed, report
