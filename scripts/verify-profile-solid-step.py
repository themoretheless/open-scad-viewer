"""Independent exact polynomial volume, followed by the OCCT interchange oracle.

Only untrimmed, unit-weight bilinear faces are supported by this volume oracle.
This checks the retained body; it does not claim ideal sweep error or smooth seams.
"""
import hashlib
import json
import subprocess
import sys
from fractions import Fraction as F
from pathlib import Path


def cross(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]


def patch_volume(surface):
    assert surface['degreeU'] == surface['degreeV'] == 1
    assert surface['knotsU'] == surface['knotsV'] == [0, 0, 1, 1]
    assert not surface['periodicU'] and not surface['periodicV']
    assert surface['weights'] == [[1, 1], [1, 1]]
    poles = surface['controlPoints']
    assert len(poles) == 2 and all(len(row) == 2 for row in poles)
    a, q, r, z = [[F(x) for x in p] for p in [poles[0][0], poles[1][0], poles[0][1], poles[1][1]]]
    b = [q[k]-a[k] for k in range(3)]
    c = [r[k]-a[k] for k in range(3)]
    d = [z[k]-q[k]-r[k]+a[k] for k in range(3)]
    position = {(0, 0): a, (1, 0): b, (0, 1): c, (1, 1): d}
    normal = {(0, 0): cross(b, c), (1, 0): cross(b, d), (0, 1): cross(d, c)}
    # Divergence theorem: integral P dot (Pu cross Pv) / 3.
    return sum((sum(x*y for x, y in zip(p, n))/F(3*(u+s+1)*(v+t+1))
                for (u, v), p in position.items() for (s, t), n in normal.items()), F(0))


root = Path(sys.argv[1])
manifest_path = root/'manifest.json'
manifest = json.loads(manifest_path.read_text())
manifest['independentVolumeOracle'] = {'method': 'exact-binary64-rational-bilinear-divergence-integral',
                                     'sourceSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
for case in manifest['cases']:
    assert len(case['sourceShells']) == 1 and case['sourceShells'][0]['closed']
    uses = case['sourceShells'][0]['faces']
    assert sorted(use['face'] for use in uses) == list(range(case['faces']))
    volume = sum(((-1 if use['reversed'] else 1)*patch_volume(case['wallSurfaces'][use['face']])
                  for use in uses), F(0))
    assert volume > 0
    case['expectedVolume'] = float(volume)
    case['expectedVolumeExact'] = {'numerator': str(volume.numerator), 'denominator': str(volume.denominator)}
manifest_path.write_text(json.dumps(manifest, indent=2)+'\n')
subprocess.run([sys.executable, str(Path(__file__).with_name('verify-sweep-step-occt.py')), str(root)], check=True)
