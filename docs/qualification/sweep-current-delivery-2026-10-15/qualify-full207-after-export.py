"""Qualify the existing export once complete; never restart it or change limits."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

stage = Path(__file__).parent
output = Path('/private/tmp/osv-sweep-current-full207-run2-20261015')
manifest = output / 'manifest.json'
python = '/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
export_pid = 89784
receipt = {'scope': 'Exact current packed WASM, full finite 207-case independent checks; not an all-input theorem',
           'state': 'waiting-existing-export', 'passed': False, 'phases': []}

def save():
    temporary = output / 'qualification-receipt.json.writing'
    temporary.write_text(json.dumps(receipt, indent=2) + '\n')
    temporary.replace(output / 'qualification-receipt.json')

save()
while not manifest.exists():
    try:
        os.kill(export_pid, 0)
    except ProcessLookupError:
        receipt['state'] = 'export-ended-without-manifest'
        save()
        raise RuntimeError('Existing export ended without publishing a complete manifest; no retry')
    time.sleep(5)

data = json.loads(manifest.read_text())
assert len(data['cases']) == 207
assert len({c['file'] for c in data['cases']}) == 207
wasm_sha = hashlib.sha256((stage / 'public/wasm/geometry-kernel.wasm').read_bytes()).hexdigest()
assert wasm_sha == 'da073844c67f3271c28a12107076854234f393f01dd8bdb4f45391955ee23929'
assert data['artifactProvenance']['geometryWasmSha256'] == wasm_sha
shutil.copyfile(manifest, output / 'manifest-before-independent-references.json')
for case in data['cases']:
    source = case.get('sourceFile')
    if source and source.startswith('examples/'):
        destination = output / source
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(stage / source, destination)
receipt.update(state='qualifying', geometryWasmSha256=wasm_sha)
save()

commands = [
    ('polynomial-reference', [python, 'scripts/reference-sweep-generator-volume.py', str(output)]),
    ('rational-reference', [python, 'scripts/reference-rational-boundary-volume.py', str(output)]),
    ('opencascade', [python, 'scripts/verify-sweep-step-occt.py', str(output)]),
    ('complete-boundary', ['node', 'scripts/verify-sweep-complete-boundary-contracts.mjs', str(manifest), str(output/'complete-boundary.json')]),
    ('global', ['node', 'scripts/verify-sweep-global-matrix.mjs', str(manifest), str(output/'opencascade-sweep.json'), str(output/'global.json')]),
    ('finite-smoothness', ['node', 'scripts/verify-sweep-finite-modes.mjs', str(manifest), str(output/'opencascade-sweep.json'), str(output/'finite-modes.json')]),
]
for name, argv in commands:
    phase = {'name': name, 'argv': argv, 'state': 'running'}
    receipt['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    started = time.monotonic()
    with (output / (name+'.log')).open('w') as log:
        result = subprocess.run(argv, cwd=stage, env={**os.environ, 'PYTHONDONTWRITEBYTECODE':'1'}, stdout=log, stderr=subprocess.STDOUT)
    phase.update(state='passed' if result.returncode == 0 else 'failed', returncode=result.returncode, seconds=time.monotonic()-started)
    save()
    print(name + ': ' + phase['state'], flush=True)
    if result.returncode:
        receipt['state'] = 'failed'
        save()
        raise RuntimeError(name + ' failed; original failure retained, no retry or relaxed check')
receipt.update(state='complete', passed=True)
save()
print('Finite207 independent reference/OCP/boundary/global/smoothness checks passed; inventory and all-input audit remain separate', flush=True)
