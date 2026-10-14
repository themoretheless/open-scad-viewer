"""Independent exact retained quintic-wall volume, then OCCT interchange.

Constant positive profile-row weights induce an exact positive reparameterization
of the polynomial wall image. No native proof receipt supplies the volume.
"""
import hashlib
import json
import math
import subprocess
import sys
from fractions import Fraction as F
from pathlib import Path


def bernstein(degree, index):
    return {index + k: F(math.comb(degree, index) * math.comb(degree-index, k)
                        * (-1)**k) for k in range(degree-index+1)}


def multiply(a, b):
    result = {}
    for (u, v), x in a.items():
        for (s, t), y in b.items():
            key = (u+s, v+t)
            result[key] = result.get(key, F(0)) + x*y
    return result


def subtract(a, b):
    result = dict(a)
    for key, value in b.items():
        result[key] = result.get(key, F(0))-value
    return result


def derivative(poly, axis):
    result = {}
    for key, value in poly.items():
        if key[axis]:
            lowered = list(key)
            lowered[axis] -= 1
            result[tuple(lowered)] = value*key[axis]
    return result


def patch_volume(surface):
    assert surface['degreeU'] == 1 and surface['degreeV'] == 5
    assert surface['knotsU'] == [0, 0, 1, 1]
    assert surface['knotsV'] == [0]*6+[1]*6
    assert not surface['periodicU'] and not surface['periodicV']
    poles, weights = surface['controlPoints'], surface['weights']
    assert len(poles) == len(weights) == 2
    assert all(len(row) == 6 for row in poles+weights)
    assert all(F(row[0]) > 0 and all(F(w) == F(row[0]) for w in row)
               for row in weights)
    position = [{}, {}, {}]
    for i in range(2):
        for j in range(6):
            assert len(poles[i][j]) == 3
            for u, a in bernstein(1, i).items():
                for v, b in bernstein(5, j).items():
                    for k in range(3):
                        key = (u, v)
                        position[k][key] = position[k].get(key, F(0))+F(poles[i][j][k])*a*b
    du = [derivative(p, 0) for p in position]
    dv = [derivative(p, 1) for p in position]
    normal = [subtract(multiply(du[1], dv[2]), multiply(du[2], dv[1])),
              subtract(multiply(du[2], dv[0]), multiply(du[0], dv[2])),
              subtract(multiply(du[0], dv[1]), multiply(du[1], dv[0]))]
    # Divergence theorem; exact binary64 inputs interpreted as rational numbers.
    return sum((value/F(3*(u+1)*(v+1))
                for p, n in zip(position, normal)
                for (u, v), value in multiply(p, n).items()), F(0))


def self_test():
    # Nonlinear quintic parameterization v^5 on a plane has the same oriented
    # area as a linear parameterization. Its flux is exactly height / 3.
    surface = {'degreeU': 1, 'degreeV': 5, 'knotsU': [0, 0, 1, 1],
               'knotsV': [0]*6+[1]*6, 'periodicU': False, 'periodicV': False,
               'controlPoints': [[[i, int(j == 5), 3] for j in range(6)] for i in range(2)],
               'weights': [[1]*6, [1]*6]}
    assert patch_volume(surface) == 1
    surface['weights'] = [[1]*6, [2]*6]
    assert patch_volume(surface) == 1
    surface['controlPoints'].reverse()
    assert patch_volume(surface) == -1
    surface['weights'][0][3] = 3
    try:
        patch_volume(surface)
    except AssertionError:
        pass
    else:
        raise AssertionError('V-dependent weights must refuse the polynomial oracle')
    print('Exact quintic volume oracle self-test passed')


def main(root):
    manifest_path = root/'manifest.json'
    manifest = json.loads(manifest_path.read_text())
    manifest['independentVolumeOracle'] = {
        'method': 'exact-binary64-rational-quintic-divergence-integral',
        'sourceSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    for case in manifest['cases']:
        assert len(case['sourceShells']) == 1 and case['sourceShells'][0]['closed']
        uses = case['sourceShells'][0]['faces']
        assert sorted(use['face'] for use in uses) == list(range(case['faces']))
        volume = sum(((-1 if use['reversed'] else 1)*patch_volume(case['wallSurfaces'][use['face']])
                      for use in uses), F(0))
        assert volume > 0
        case['expectedVolume'] = float(volume)
        case['expectedVolumeExact'] = {'numerator': str(volume.numerator),
                                       'denominator': str(volume.denominator)}
    manifest_path.write_text(json.dumps(manifest, indent=2)+'\n')
    subprocess.run([sys.executable, str(Path(__file__).with_name('verify-sweep-step-occt.py')),
                    str(root)], check=True)


def prepare_native(bundle_path, root):
    data = bundle_path.read_bytes()
    bundle = json.loads(data)
    assert bundle['schema'] == 'native-polygon-station-step/1'
    root.mkdir(parents=True, exist_ok=True)
    cases = []
    for entry in bundle['cases']:
        body = entry['body']
        assert body['accepted'] and body['stationContinuity'] == 'G2'
        model = body['model']
        mode = 'spatial' if entry['spatial'] else 'planar'
        weight = 'weighted' if entry['weighted'] else 'unit'
        file = f"quintic-{mode}-{weight}-{entry['stations']}.step"
        text = entry['export']['text'].encode()
        (root/file).write_bytes(text)
        faces = model['faces']
        wires = [model['loops'][face['outer']]['coedges'] for face in faces]
        assert all(not face['holes'] for face in faces)
        cases.append(dict(file=file, mode=mode, stations=entry['stations'],
                          weighted=entry['weighted'], faces=len(faces), solids=1, shells=1,
                          capHoleFaces=0, capFaces=[], requireNativeSolid=True,
                          nativeVolume=body['volume'], sourceShells=model['shells'],
                          nativeStation=body['station'], edges=len(model['edges']),
                          faceLoops=[dict(outer=[c['edge'] for c in wire], holes=[])
                                     for wire in wires], wallCoedges=wires,
                          wallSurfaces=[face['surface'] for face in faces],
                          wallSamples=entry['wallSamples'], edgeCurves=entry['edgeCurves'],
                          surfaceToleranceMm=1e-8, relativeVolumeTolerance=1e-7,
                          sha256=hashlib.sha256(text).hexdigest(), expectedVolume=None))
    manifest = dict(schema='sweep-external-step/2', cases=cases,
                    artifactProvenance=dict(kind='native-fixture-bundle',
                                            bundleSha256=hashlib.sha256(data).hexdigest(),
                                            wasmVerified=False))
    (root/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        self_test()
    elif len(sys.argv) == 4 and sys.argv[1] == '--native-bundle':
        prepare_native(Path(sys.argv[2]), Path(sys.argv[3]))
        main(Path(sys.argv[3]))
    else:
        main(Path(sys.argv[1]))
