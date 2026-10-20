"""Fresh full STEP/UI qualification after an existing exact-artifact gate.

Runs each phase once, stops on failure, and preserves all outputs. This is a
finite qualification driver, never a declaration that the library goal is done.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--output', required=True)
parser.add_argument('--artifact-gates', required=True)
parser.add_argument('--wait-pid', required=True, type=int)
parser.add_argument('--expected-wasm', required=True)
parser.add_argument('--native-producer', required=True)
parser.add_argument('--source-ledger', default='frozen-runtime-source-sha256.json')
parser.add_argument('--qa-overrides', default='qa-source-overrides-current.json')
parser.add_argument('--extra-input-ledger')
args = parser.parse_args()
stage = Path(__file__).resolve().parents[1]
root = Path(args.output).resolve()
assert not (root / 'receipt.json').exists(), 'Preserve previous runs; do not restart blindly'
root.mkdir(parents=True, exist_ok=True)
step = root / 'step207'
ui = root / 'ui478'
producer = Path(args.native_producer).resolve()
oracle = '/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
receipt = {'schema': 'sweep-fresh-current-artifact/1', 'state': 'waiting-existing-artifact-gates',
           'passed': False, 'geometryWasmSha256': args.expected_wasm,
           'waitPid': args.wait_pid, 'phases': [],
           'scope': 'Fresh finite207 STEP/current proofs and478 UI cases; not universal geometric admission or goal completion.'}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save():
    temporary = root / 'receipt.json.writing'
    temporary.write_text(json.dumps(receipt, indent=2) + '\n')
    temporary.replace(root / 'receipt.json')


def identity():
    if 'driverSha256' in receipt:
        assert digest(__file__) == receipt['driverSha256'], 'Qualification driver changed'
    if 'nativeProducerSha256' in receipt:
        assert digest(producer) == receipt['nativeProducerSha256'], 'Frozen native producer changed'
    for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm',
                 'dist/wasm/geometry-kernel.wasm']:
        assert digest(stage / name) == args.expected_wasm, 'Exact artifact changed'
    assert args.expected_wasm in (stage / 'src/generated/geometry-kernels/identity.ts').read_text()


def sources():
    ledger_path = stage / args.source_ledger
    override_path = stage / args.qa_overrides
    ledger = json.loads(ledger_path.read_text())
    overrides = json.loads(override_path.read_text())['files']
    expected = {r['file']: r['afterSha256'] for r in overrides}
    outputs = ('src/generated/geometry-kernels/', 'src/generated/wasm-brotli/',
               'public/wasm/geometry-kernel.wasm')
    for name, sha in ledger.items():
        if name.startswith(outputs):
            continue
        assert digest(stage / name) == expected.get(name, sha), f'Source changed: {name}'
    for name, sha in expected.items():
        # QA-only contract repairs are hash-bound separately from the native
        # and application source used to build this exact artifact.
        assert name.startswith(('scripts/', 'tests/')), 'Only explicit qualification/test overrides allowed'
        assert digest(stage / name) == sha
    binding = {'frozenSourceLedgerSha256': digest(ledger_path),
               'qaOverrideLedgerSha256': digest(override_path),
               'sourceFilesChecked': len(ledger)}
    if args.extra_input_ledger:
        extra_path = stage / args.extra_input_ledger
        extra = json.loads(extra_path.read_text())
        for name, item in extra['files'].items():
            assert digest(stage / name) == item['sha256'], f'QA input changed: {name}'
        browser_root = Path(extra['browserDependenciesRealPath'])
        for name, sha in extra['browserDependencyFiles'].items():
            assert digest(browser_root / name) == sha, f'Browser dependency changed: {name}'
        binding.update(extraInputLedgerSha256=digest(extra_path),
                       extraInputsChecked=len(extra['files']) + len(extra['browserDependencyFiles']))
    return binding


def run(name, argv):
    identity()
    assert sources() == receipt['sourceBinding'], 'Source or QA input ledger changed'
    phase = {'name': name, 'argv': list(map(str, argv)), 'state': 'running'}
    receipt['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    started = time.monotonic()
    with (root / (name + '.log')).open('w') as log:
        result = subprocess.run(list(map(str, argv)), cwd=stage,
                                env={**os.environ, 'NODE_OPTIONS': '--max-old-space-size=8192',
                                     'PYTHONDONTWRITEBYTECODE': '1'}, stdout=log, stderr=subprocess.STDOUT)
    phase.update(state='passed' if result.returncode == 0 else 'failed',
                 returncode=result.returncode, seconds=time.monotonic() - started)
    identity()
    assert sources() == receipt['sourceBinding'], 'Source or QA input ledger changed'
    save()
    print(name + ': ' + phase['state'], flush=True)
    if result.returncode:
        receipt['state'] = 'failed'
        save()
        raise SystemExit(result.returncode)


def capture_export():
    # Release the large parsed manifest before starting another verifier.
    data = json.loads((step / 'manifest.json').read_text())
    assert len(data['cases']) == 207
    assert len({c['file'] for c in data['cases']}) == 207
    assert data['artifactProvenance']['geometryWasmSha256'] == args.expected_wasm
    for case in data['cases']:
        source = case.get('sourceFile')
        if source and source.startswith('examples/'):
            destination = step / source
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(stage / source, destination)


save()
receipt['driverSha256'] = digest(__file__)
identity()
receipt['sourceBinding'] = sources()
receipt['nativeProducerSha256'] = digest(producer)
save()
gate_path = Path(args.artifact_gates) / 'receipt.json'
while True:
    gate = json.loads(gate_path.read_text())
    if gate.get('state') == 'complete':
        assert gate['passed'] is True
        assert gate['artifacts']['packed']['sha256'] == args.expected_wasm
        break
    if gate.get('state') in ['failed', 'stopped-after-focused-ui-failure',
                             'focused-ui-ended-without-complete-pass']:
        receipt['state'] = 'stopped-after-artifact-gate-failure'
        save()
        raise SystemExit(1)
    try:
        os.kill(args.wait_pid, 0)
    except ProcessLookupError:
        receipt['state'] = 'prior-gate-ended-without-complete-pass'
        save()
        raise SystemExit(1)
    time.sleep(5)
receipt.update(state='running', artifactGateReceiptSha256=digest(gate_path))
save()
run('step207-export', ['node', '--import', 'tsx', '--loader', './qualification-raw-imports.mjs',
                       'scripts/export-sweep-step-oracle.mts', step,
                       '--automatic-smooth-caps', '--full-207'])
capture_export()
run('polynomial-reference', [oracle, 'scripts/reference-sweep-generator-volume.py', step])
run('rational-reference', [oracle, 'scripts/reference-rational-boundary-volume.py', step])
run('opencascade207', [oracle, 'scripts/verify-sweep-step-occt.py', step])
manifest = step / 'manifest.json'
occt = step / 'opencascade-sweep.json'
for name, argv in [
    ('complete-boundary', ['node', 'scripts/verify-sweep-complete-boundary-contracts.mjs', manifest, step / 'complete-boundary.json']),
    ('global', ['node', 'scripts/verify-sweep-global-matrix.mjs', manifest, occt, step / 'global.json']),
    ('finite-smoothness', ['node', 'scripts/verify-sweep-finite-modes.mjs', manifest, occt, step / 'finite-modes.json']),
    ('inventory207', ['node', 'scripts/verify-sweep-current-inventory-207.mjs', manifest, 'original-step-inventory.json', stage, step / 'current-inventory.json']),
    ('pair-cover', ['node', 'scripts/verify-sweep-face-pair-cover.mjs', manifest, step / 'face-pair-cover.json']),
    ('smoothness-controls', ['node', 'scripts/verify-sweep-smoothness-refusal-controls.mjs', manifest, step / 'smoothness-refusal-controls.json']),
    ('fresh-migration', ['node', 'scripts/prepare-sweep-geometry-migration.mjs', manifest,
                         'original-request-baseline.json', 'original-geometry-projection.json',
                         'original-budget-ledger.json', producer, step / 'geometry-migration']),
    ('original157', ['node', 'scripts/verify-sweep-original-request-preservation.mjs', manifest,
                     'original-request-baseline.json', step / 'original157-preservation.json',
                     'original-annular-walls.json', step / 'geometry-migration/review.json']),
]:
    run(name, argv)
station = root / 'annular-station'
run('new-station-step', ['node', '--import', 'tsx', '--loader', './qualification-raw-imports.mjs',
                        'scripts/export-sweep-step-oracle.mts', station, '--automatic-smooth-caps',
                        '--annular-station-g2-only'])
run('new-station-reference', [oracle, 'scripts/reference-rational-boundary-volume.py', station])
run('new-station-occt', [oracle, 'scripts/verify-sweep-step-occt.py', station])
run('full-ui478', ['node', 'scripts/run-fresh-full-ui-audit.mjs', ui])
receipt.update(state='complete', passed=True, sourceBinding=sources(),
               finalManifestSha256=digest(manifest), driverSha256=digest(__file__))
save()
print('Fresh finite STEP207, separate new-station STEP and UI478 qualification completed; all-mode completion audit remains separate', flush=True)
