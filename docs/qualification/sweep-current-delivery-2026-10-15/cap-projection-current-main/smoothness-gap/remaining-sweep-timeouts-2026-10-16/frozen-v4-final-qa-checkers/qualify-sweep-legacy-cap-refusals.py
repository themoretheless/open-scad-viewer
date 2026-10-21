"""Bind the two exact-cap refusal contracts to a completed STEP qualification.

Receipt reconciliation only: Rust produced the geometry and exact witnesses.
Read the large compact manifest through mmap, decoding only the two cases.
"""
import argparse
import hashlib
import json
import mmap
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--canonical', required=True)
parser.add_argument('--expected-wasm', required=True)
parser.add_argument('--output', required=True)
parser.add_argument('--checker-directory', required=True)
parser.add_argument('--node', required=True)
args = parser.parse_args()
canonical = Path(args.canonical).resolve()
output = Path(args.output).resolve()
output.mkdir(exist_ok=False)
checker = Path(args.checker_directory).resolve()


def digest(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


qualification_path = canonical / 'receipt.json'
global_path = canonical / 'step207/global.json'
qualification = json.loads(qualification_path.read_text())
global_report = json.loads(global_path.read_text())
assert qualification['state'] == 'complete' and qualification['passed']
assert qualification['geometryWasmSha256'] == args.expected_wasm
assert global_report['passed']
assert global_report['artifactProvenance']['geometryWasmSha256'] == args.expected_wasm
inputs = {str(path): digest(path) for path in [qualification_path, global_path,
    checker / 'sweep-legacy-cap-refusal-contracts.mjs',
    checker / 'test-sweep-legacy-cap-refusal-contracts.mjs', Path(__file__)]}
manifest_path = canonical / 'step207/manifest.json'
names = ['open-spatial-hollow-miter.step', 'affine-oblique-hollow-miter.step']
cases = []
with manifest_path.open('rb') as source, mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
    manifest_sha = hashlib.sha256(data).hexdigest()
    assert manifest_sha == qualification['finalManifestSha256'] == global_report['manifestSha256']
    for name in names:
        marker = ('{"file":' + json.dumps(name)).encode()
        start = data.find(marker)
        assert start >= 0 and data.find(marker, start + 1) == -1, 'Missing or ambiguous captured case'
        # The exporter writes compact case objects with file as the first key.
        assert start > 0 and data[start - 1:start] in (b'[', b',')
        for size in (100_000, 1_000_000, 10_000_000):
            try:
                # Ignore only an incomplete trailing UTF-8 sequence outside the
                # decoded value; the decoder rejects malformed bytes within it.
                import codecs
                text = codecs.getincrementaldecoder('utf-8')().decode(data[start:start + size], final=False)
                case, _ = json.JSONDecoder().raw_decode(text)
                break
            except json.JSONDecodeError:
                continue
        else:
            raise ValueError('Captured legacy case exceeds extraction buffer')
        assert case['file'] == name
        reconciled = [item for item in global_report['cases'] if item['file'] == name]
        assert len(reconciled) == 1 and reconciled[0]['status'] == 'correctly-unproved'
        assert digest(canonical / 'step207' / name) == case['sha256'] == reconciled[0]['stepSha256']
        cases.append({key: case[key] for key in ['file', 'faces', 'shells', 'capFaces',
            'faceLoops', 'nativeExactUses', 'nativeBoundary', 'nativeVolume']})
    assert hashlib.sha256(data).hexdigest() == manifest_sha, 'Manifest changed during extraction'
captured = output / 'captured-evidence.json'
captured.write_text(json.dumps({'manifestSha256': manifest_sha,
    'artifactProvenance': global_report['artifactProvenance'], 'cases': cases}, indent=2) + '\n')
report = output / 'controls.json'
subprocess.run([args.node, str(checker / 'test-sweep-legacy-cap-refusal-contracts.mjs'),
    str(captured), str(report)], check=True)
for path, sha in inputs.items():
    assert digest(path) == sha, 'Qualification or checker changed during reconciliation'
controls = json.loads(report.read_text())
assert controls['passed'] and controls['manifestSha256'] == manifest_sha
(output / 'receipt.json').write_text(json.dumps({'state': 'complete', 'passed': True,
    'scope': 'Exact-cap refusal reconciliation on the named completed artifact; not a new native proof or full-goal completion',
    'geometryWasmSha256': args.expected_wasm, 'manifestSha256': manifest_sha,
    'sourceAndQualificationInputs': inputs, 'controlsSha256': digest(report),
    'capturedEvidenceSha256': digest(captured), 'cases': len(cases),
    'mutationControls': sum(len(item['refused']) for item in controls['cases']),
    'fullGoalComplete': False}, indent=2) + '\n')
