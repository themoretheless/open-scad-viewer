"""Interim STEP-only controls; the final post-UI cap gate remains mandatory."""
import codecs
import hashlib
import json
import mmap
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent
canonical = root / 'frozen-v4-independent-step-ui-final/canonical207-ui478'
checker = root / 'frozen-v4-final-qa-checkers'
output = root / 'current-step-only-cap-controls'
output.mkdir(exist_ok=False)
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
expected = 'b376cd35f06ca2b8d366ec8b96fabd7155c43306fdfb91fccc0e01a77ab4683f'
qualification_bytes = (canonical / 'receipt.json').read_bytes()
qualification = json.loads(qualification_bytes)
assert qualification['geometryWasmSha256'] == expected
required = ['step207-export', 'polynomial-reference', 'rational-reference',
    'opencascade207', 'complete-boundary', 'global', 'finite-smoothness',
    'inventory207', 'pair-cover', 'smoothness-controls', 'fresh-migration',
    'original157', 'new-station-step', 'new-station-reference', 'new-station-occt']
assert [p['name'] for p in qualification['phases'][:15]] == required
assert all(p['state'] == 'passed' and p['returncode'] == 0
    for p in qualification['phases'][:15])
global_path = canonical / 'step207/global.json'
global_report = json.loads(global_path.read_text())
assert global_report['passed'] and global_report['certified'] == 205 and global_report['refused'] == 2
assert global_report['artifactProvenance']['geometryWasmSha256'] == expected
inputs = {str(p): digest(p) for p in [global_path, Path(__file__),
    checker / 'sweep-legacy-cap-refusal-contracts.mjs',
    checker / 'test-sweep-legacy-cap-refusal-contracts.mjs']}
cases = []
with (canonical / 'step207/manifest.json').open('rb') as source, mmap.mmap(
    source.fileno(), 0, access=mmap.ACCESS_READ) as data:
    manifest_sha = hashlib.sha256(data).hexdigest()
    assert manifest_sha == global_report['manifestSha256']
    for name in ['open-spatial-hollow-miter.step', 'affine-oblique-hollow-miter.step']:
        marker = ('{"file":' + json.dumps(name)).encode()
        start = data.find(marker)
        assert start > 0 and data.find(marker, start + 1) == -1
        assert data[start - 1:start] in (b'[', b',')
        for size in [100_000, 1_000_000, 10_000_000]:
            try:
                text = codecs.getincrementaldecoder('utf-8')().decode(data[start:start + size], final=False)
                case, _ = json.JSONDecoder().raw_decode(text)
                break
            except json.JSONDecodeError:
                continue
        else:
            raise ValueError('Captured case exceeds extraction limit')
        assert case['file'] == name
        entries = [c for c in global_report['cases'] if c['file'] == name]
        assert len(entries) == 1 and entries[0]['status'] == 'correctly-unproved'
        step = canonical / 'step207' / name
        assert digest(step) == case['sha256'] == entries[0]['stepSha256']
        inputs[str(step)] = digest(step)
        cases.append({key: case[key] for key in ['file', 'faces', 'shells',
            'capFaces', 'faceLoops', 'nativeExactUses', 'nativeBoundary', 'nativeVolume']})
    assert hashlib.sha256(data).hexdigest() == manifest_sha
captured = output / 'captured-evidence.json'
captured.write_text(json.dumps({'manifestSha256': manifest_sha,
    'artifactProvenance': global_report['artifactProvenance'], 'cases': cases}, indent=2) + '\n')
controls = output / 'controls.json'
subprocess.run(['/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node',
    str(checker / 'test-sweep-legacy-cap-refusal-contracts.mjs'),
    str(captured), str(controls)], check=True)
assert all(digest(p) == sha for p, sha in inputs.items())
result = json.loads(controls.read_text())
assert result['passed'] and result['manifestSha256'] == manifest_sha
(output / 'receipt.json').write_text(json.dumps({'state': 'complete', 'passed': True,
    'scope': 'Interim controls on the completed current STEP phases only; UI is still running. Does not satisfy or replace the mandatory final post-UI cap gate, new geometric proof, or full goal.',
    'geometryWasmSha256': expected, 'manifestSha256': manifest_sha,
    'canonicalStateAtObservation': qualification['state'],
    'canonicalReceiptSha256AtObservation': hashlib.sha256(qualification_bytes).hexdigest(),
    'sourceInputs': inputs, 'capturedEvidenceSha256': digest(captured),
    'controlsSha256': digest(controls), 'cases': len(cases),
    'mutationControls': sum(len(c['refused']) for c in result['cases']),
    'finalPostUiGateSatisfied': False, 'fullGoalComplete': False}, indent=2) + '\n')
