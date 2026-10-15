"""Bind native intrinsic basis proofs to every captured retained STEP surface.

Python transports data and checks receipt identity. C1/C2 decisions are Rust.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

manifest_path, full_gate_path, producer, producer_source, output = map(Path, sys.argv[1:])
sha = lambda data: hashlib.sha256(data).hexdigest()
raw = manifest_path.read_bytes()
manifest = json.loads(raw, parse_int=lambda text: -0.0 if text == '-0' else int(text))
gate = json.loads(full_gate_path.read_text())
assert gate['state'] == 'complete' and gate['passed'] is True
assert gate['finalManifestSha256'] == sha(raw)
assert manifest['artifactProvenance']['geometryWasmSha256'] == gate['geometryWasmSha256']
assert len(manifest['cases']) == 207
assert len({case['file'] for case in manifest['cases']}) == 207
output.mkdir(parents=True, exist_ok=True)
assert not (output / 'result.json').exists(), 'Preserve earlier audits'
producer_hash = sha(producer.read_bytes())
source_hash = sha(producer_source.read_bytes())
requests = []
metadata = {}
for case in manifest['cases']:
    faces = case['faces']
    caps = case['capFaces']
    assert type(faces) is int and 0 < faces <= 1024
    assert len(set(caps)) == len(caps) and all(type(face) is int and 0 <= face < faces for face in caps)
    owners = [face for face in range(faces) if face not in caps]
    assert len(owners) == len(case['wallSurfaces']) and owners
    surfaces = [{'face': face, 'surface': surface} for face, surface in zip(owners, case['wallSurfaces'])]
    requests.append({'file': case['file'], 'surfaces': surfaces})
    profile = case['nativeProfileSmoothness']
    metadata[case['file']] = {
        'omittedExactPlanarCaps': caps,
        'nativeWallProfileG1': profile['profileG1Certified'],
        'nativeWallProfileG2': profile['profile']['exactG1G2Certified'],
        'nativeWallStationG2': profile['station']['stationG2Certified'],
    }
common = {'maxInputItemsPerCase': 1000000, 'manifestSha256': sha(raw),
          'geometryWasmSha256': gate['geometryWasmSha256']}
batches, group = [], []
encode = lambda group: json.dumps({**common, 'cases': group}, separators=(',', ':'), allow_nan=False).encode()
for case in requests:
    if len(encode([*group, case])) > 31 * 1024 * 1024:
        assert group
        batches.append(group)
        group = [case]
    else:
        group.append(case)
    assert len(encode(group)) <= 31 * 1024 * 1024
if group:
    batches.append(group)
result = {'schema': 'sweep-retained-surface-interior-audit/1', 'passed': False,
          'manifestSha256': sha(raw), 'geometryWasmSha256': gate['geometryWasmSha256'],
          'producerSha256': producer_hash, 'producerSourceSha256': source_hash,
          'preparerSha256': sha(Path(__file__).read_bytes()), 'batches': [], 'cases': [],
          'scope': 'All captured nonplanar/retained rectangular surface interiors, including radial caps. Exact planar caps omitted by the existing manifest mask. Native basis sufficient parametric C1/C2 only; geometric regularity, sewn/periodic seams, source error and material remain separate. Low basis order is unproved higher smoothness, never a discontinuity counterexample.'}
for index, group in enumerate(batches):
    request_path = output / f'request-{index}.json'
    native_path = output / f'native-{index}.json'
    assert not request_path.exists() and not native_path.exists()
    request_path.write_bytes(encode(group))
    assert sha(producer.read_bytes()) == producer_hash
    with (output / f'native-{index}.log').open('w') as log:
        status = subprocess.run([str(producer.resolve()), str(request_path), str(native_path)], stdout=log, stderr=subprocess.STDOUT)
    assert status.returncode == 0, 'Native audit refused; retain its output, no relaxed budget'
    native = json.loads(native_path.read_text())
    assert native['schema'] == 'retained-surface-interior-continuity/1'
    assert native['passed'] is True
    assert native['requestSha256'] == sha(request_path.read_bytes())
    assert native['producerSha256'] == source_hash
    assert native['continuousBound'] is False and native['solidCertified'] is False
    assert [case['file'] for case in native['cases']] == [case['file'] for case in group]
    for expected, proof in zip(group, native['cases']):
        assert proof['complete'] is True and proof['reason'] is None and proof['unvisitedFaces'] == 0
        assert 0 < proof['work'] <= proof['maxInputItems'] == 1000000
        assert proof['inputItemUnit'] == 'weights-and-knot-entries'
        assert [face['face'] for face in proof['faces']] == [face['face'] for face in expected['surfaces']]
        for face in proof['faces']:
            assert face['parametricC0Certified'] is True
            assert type(face['parametricC1Certified']) is bool and type(face['parametricC2Certified']) is bool
            assert not face['parametricC2Certified'] or face['parametricC1Certified']
            assert face['periodicSeamsCertified'] is False and face['surfaceRegularityCertified'] is False
            assert face['geometricG1G2Certified'] is False
        result['cases'].append({**proof, **metadata[proof['file']]})
    result['batches'].append({'requestSha256': sha(request_path.read_bytes()),
                              'outputSha256': sha(native_path.read_bytes())})
assert len(result['cases']) == 207
assert [case['file'] for case in result['cases']] == [case['file'] for case in requests]
result['summary'] = {
    'cases': 207, 'surfaces': sum(len(case['faces']) for case in result['cases']),
    'unprovedBasisC1': [{'file': case['file'], 'face': face['face'], 'lines': face['interiorKnotLines']}
                       for case in result['cases'] for face in case['faces'] if not face['parametricC1Certified']],
    'unprovedBasisC2': [{'file': case['file'], 'face': face['face'], 'lines': face['interiorKnotLines']}
                       for case in result['cases'] for face in case['faces'] if not face['parametricC2Certified']],
}
result['passed'] = True
(output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print(f"207 exact-scope native interior audits; {result['summary']['surfaces']} surfaces; "
      f"{len(result['summary']['unprovedBasisC1'])} unproved basis-C1, {len(result['summary']['unprovedBasisC2'])} unproved basis-C2; no whole-boundary promotion")
