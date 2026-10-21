"""Run independent STEP/UI gates after the authenticated serial public run."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

stage = Path('/private/tmp/osv-sweep-shared-generators-arc-partition-source-20261015')
base = Path(__file__).resolve().parent / 'current-source-build-arc-partition'
out = base / 'post-public-qa'
out.mkdir(exist_ok=False)
build_path = base / 'receipt.json'
build = json.loads(build_path.read_text())
assert build['passed']
assert json.loads((base / 'abi-qa/receipt.json').read_text())['passed']
public_path = base / 'public100-serial/receipt.json'
public = json.loads(public_path.read_text())
wait_pid = public['pid']
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger = json.loads((stage / 'frozen-current-main-source-sha256.json').read_text())
overrides = {r['file']: r['afterSha256'] for r in json.loads((stage / 'qa-source-overrides-current.json').read_text())['files']}
extra = json.loads((stage / 'qualification-external-inputs.json').read_text())
receipt = {'schema': 'fresh-current-independent-step-ui/1', 'state': 'waiting-serial-public-run',
    'passed': False, 'pid': os.getpid(), 'waitPid': wait_pid, 'driverSha256': sha(__file__),
    'geometryWasmSha256': build['artifacts']['packed']['sha256'], 'phases': [],
    'scope': 'Current-artifact multispan, supplemental STEP4/UI8 and canonical STEP207/UI478. Public failures remain explicit and prevent overall success.',
    'fullGoalComplete': False}

def save():
    p = out / 'receipt.json.writing'
    p.write_text(json.dumps(receipt, indent=2) + '\n')
    p.replace(out / 'receipt.json')

def identity():
    assert sha(__file__) == receipt['driverSha256']
    for name, h in ledger.items():
        assert sha(stage / name) == overrides.get(name, h), name
    for name, meta in extra['files'].items():
        assert sha(stage / name) == meta['sha256'], name
    for name, h in extra['browserDependencyFiles'].items():
        assert sha(Path(extra['browserDependenciesRealPath']) / name) == h, name
    for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm', 'dist/wasm/geometry-kernel.wasm']:
        assert sha(stage / name) == receipt['geometryWasmSha256'], name

def run(name, argv):
    identity()
    phase = {'name': name, 'state': 'running', 'argv': list(map(str, argv))}
    receipt['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    start = time.monotonic()
    with (out / (name + '.log')).open('w') as log:
        result = subprocess.run(phase['argv'], cwd=stage, env={**os.environ,
            'NODE_OPTIONS': '--max-old-space-size=8192', 'PYTHONDONTWRITEBYTECODE': '1',
            'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_NET_OFFLINE': 'true'},
            stdout=log, stderr=subprocess.STDOUT)
    identity()
    phase.update(state='passed' if result.returncode == 0 else 'failed', returncode=result.returncode,
        seconds=time.monotonic() - start)
    save()
    print(name + ': ' + phase['state'], flush=True)
    return result.returncode == 0

identity()
save()
while True:
    public = json.loads(public_path.read_text())
    if public['state'] in ['complete', 'failed']:
        break
    assert public['state'] == 'running' and public['pid'] == wait_pid
    os.kill(wait_pid, 0)
    time.sleep(10)
receipt.update(state='running', public100Outcome=public['state'], public100Passed=public['passed'],
    public100ReceiptSha256=sha(public_path))
save()
node = ['node', '--import', 'tsx']
oracle = '/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
run('constructed-multispan', node + ['scripts/check-staged-constructed-multispan-sweep.mts', stage, out / 'constructed-multispan.json'])
run('multispan-raw', node + ['scripts/check-staged-multispan-jets.mts', stage, out / 'multispan-raw.json', '--raw-built'])
run('multispan-packed', node + ['scripts/check-staged-multispan-jets.mts', stage, out / 'multispan-packed.json'])
run('closed-multispan', node + ['scripts/check-staged-closed-strip-jets.mts', stage, out / 'closed-multispan.json'])
c2step = out / 'c2-step4'
if run('c2-step4-export', node + ['--loader', './qualification-raw-imports.mjs', 'scripts/export-corrected-c2-step.mts', c2step]):
    manifest = json.loads((c2step / 'manifest.json').read_text())
    assert len(manifest['cases']) == 4
    assert manifest['artifactProvenance']['geometryWasmSha256'] == receipt['geometryWasmSha256']
    run('c2-step4-reference', [oracle, 'scripts/reference-rational-boundary-volume.py', c2step])
    run('c2-step4-opencascade', [oracle, 'scripts/verify-sweep-step-occt.py', c2step])
run('c2-ui8', ['node', 'scripts/check-sweep-miter-matrix-browser.mjs', out / 'c2-ui8',
    '--corrected-c2-body-matrix', '--native-running-cancel', '--gpu-backend=metal'])
producer = base / 'certify-sweep-geometry-migration'
assert sha(producer) == build['nativeProducerSha256']
canonical = out / 'canonical207-ui478'
canonical_ok = run('canonical207-ui478', ['python3', 'scripts/qualify-sweep-current-artifact.py',
    '--output', canonical, '--artifact-gates', base, '--wait-pid', str(build['pid']),
    '--expected-wasm', receipt['geometryWasmSha256'], '--native-producer', producer,
    '--source-ledger', 'frozen-current-main-source-sha256.json',
    '--qa-overrides', 'qa-source-overrides-current.json', '--extra-input-ledger', 'qualification-external-inputs.json'])
gate_path = canonical / 'receipt.json'
gate = json.loads(gate_path.read_text()) if gate_path.exists() else {}
after_ui_failure = not canonical_ok and gate.get('phases', [{}])[-1].get('name') == 'full-ui478'
if canonical_ok or after_ui_failure:
    if run('retained-interiors-producer', ['cargo', 'build', '--offline', '--locked', '--release',
        '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '--example', 'certify_retained_surface_interiors']):
        run('retained-interiors', ['python3', 'scripts/qualify-retained-surface-interiors.py',
            canonical / 'step207/manifest.json', gate_path,
            stage / 'crates/target/release/examples/certify_retained_surface_interiors',
            stage / 'crates/brep-core/examples/certify_retained_surface_interiors.rs', out / 'retained-interiors']
            + (['--after-ui-failure'] if after_ui_failure else []))
else:
    receipt['retainedInteriorsPending'] = 'Prerequisite complete geometry matrix not passed; no interior completion claim.'
receipt['passed'] = public['passed'] and all(p['state'] == 'passed' for p in receipt['phases'])
receipt['state'] = 'complete' if receipt['passed'] else 'failed'
save()
print('Fresh finite STEP/UI qualification outcome: ' + receipt['state'], flush=True)
raise SystemExit(0 if receipt['passed'] else 1)
