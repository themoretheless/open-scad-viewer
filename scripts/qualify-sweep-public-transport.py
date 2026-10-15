"""Run the frozen public regression inventory after an existing QA process ends."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--stage', required=True)
parser.add_argument('--inventory', required=True)
parser.add_argument('--wait-receipt', required=True)
parser.add_argument('--wait-driver', required=True)
parser.add_argument('--output', required=True)
args = parser.parse_args()
stage = Path(args.stage).resolve()
inventory_path = Path(args.inventory).resolve()
prior_path = Path(args.wait_receipt).resolve()
prior_driver = Path(args.wait_driver).resolve()
out = Path(args.output).resolve()
out.mkdir(exist_ok=False)
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
inventory = json.loads(inventory_path.read_text())
prior = json.loads(prior_path.read_text())
wait_pid = prior['pid']
ledger_path = stage / 'frozen-current-main-source-sha256.json'
ledger = json.loads(ledger_path.read_text())
override_path = stage / 'qa-source-overrides-current.json'
overrides = {r['file']: r['afterSha256'] for r in json.loads(override_path.read_text())['files']}
build_path = stage / 'completed-build-receipt.json'
build = json.loads(build_path.read_text())
assert build['passed'] is True
assert inventory['artifactSha256'] == build['artifacts']['packed']['sha256']
receipt = {'schema': 'sweep-public-transport-qualification/1', 'state': 'waiting-current-pipeline',
           'passed': False, 'pid': os.getpid(), 'waitPid': wait_pid,
           'driverSha256': digest(__file__), 'priorDriverSha256': digest(prior_driver),
           'inventorySha256': digest(inventory_path), 'sourceLedgerSha256': digest(ledger_path),
           'overrideLedgerSha256': digest(override_path), 'buildReceiptSha256': digest(build_path),
           'geometryWasmSha256': inventory['artifactSha256'], 'requiredFiles': len(inventory['files']),
           'scope': 'Frozen public Rust/WASM/Rush and lifecycle regression inventory; independent of prior STEP/UI outcome. No universal or whole-goal completion claim.',
           'fullGoalComplete': False}


def save():
    temporary = out / 'receipt.json.writing'
    temporary.write_text(json.dumps(receipt, indent=2) + '\n')
    temporary.replace(out / 'receipt.json')


def identity():
    for path, key in [(Path(__file__), 'driverSha256'), (prior_driver, 'priorDriverSha256'),
                      (inventory_path, 'inventorySha256'), (ledger_path, 'sourceLedgerSha256'),
                      (override_path, 'overrideLedgerSha256'), (build_path, 'buildReceiptSha256')]:
        assert digest(path) == receipt[key], str(path)
    assert inventory['sourceLedgerSha256'] == receipt['sourceLedgerSha256']
    for name, expected in ledger.items():
        assert digest(stage / name) == overrides.get(name, expected), name
    for name, expected in inventory['files'].items():
        assert digest(stage / name) == expected, name
    for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm',
                 'dist/wasm/geometry-kernel.wasm']:
        assert digest(stage / name) == receipt['geometryWasmSha256'], name
    assert receipt['geometryWasmSha256'] in (stage / 'src/generated/geometry-kernels/identity.ts').read_text()


save()
identity()
print(f'Public regression job {os.getpid()} waits for current pipeline {wait_pid}', flush=True)
while True:
    try:
        os.kill(wait_pid, 0)
    except ProcessLookupError:
        break
    process = subprocess.run(['/bin/ps', '-p', str(wait_pid), '-o', 'args='],
                             capture_output=True, text=True)
    if process.returncode == 0:
        assert str(prior_driver) in process.stdout, 'Prior PID now belongs to another process'
    elif process.returncode != 1:
        raise RuntimeError('Cannot authenticate prior process')
    time.sleep(10)

identity()
receipt.update(state='running', priorFinalReceiptSha256=digest(prior_path),
               priorOutcome=json.loads(prior_path.read_text())['state'])
save()
argv = inventory['command'] + ['--reporter=json', '--outputFile=' + str(out / 'vitest.json')]
assert inventory['originalTestAndHookTimeoutMs'] == 30000
receipt['argv'] = argv
save()
started = time.monotonic()
print('Current pipeline ended; public regression inventory started', flush=True)
with (out / 'vitest.log').open('w') as log:
    status = subprocess.run(argv, cwd=stage,
                            env={**os.environ, 'NODE_OPTIONS': '--max-old-space-size=8192'},
                            stdout=log, stderr=subprocess.STDOUT)
receipt.update(returncode=status.returncode, seconds=time.monotonic() - started)
save()
identity()
if (out / 'vitest.json').exists():
    report = json.loads((out / 'vitest.json').read_text())
    observed = [str(Path(item['name']).resolve().relative_to(stage)) for item in report['testResults']]
    assert sorted(observed) == sorted(inventory['files']), 'Public regression file inventory differs'
    receipt.update(reportSha256=digest(out / 'vitest.json'), observedFiles=len(observed),
                   totalTests=report['numTotalTests'], passedTests=report['numPassedTests'],
                   failedTests=report['numFailedTests'], pendingTests=report['numPendingTests'])
    receipt['passed'] = status.returncode == 0 and report['success'] is True
receipt['state'] = 'complete' if receipt['passed'] else 'failed'
save()
print('Public regressions: ' + receipt['state'], flush=True)
raise SystemExit(0 if receipt['passed'] else 1)
