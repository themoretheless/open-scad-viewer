"""Fresh provider baseline on V5; never treats stale bindings as qualified."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

out = Path(__file__).resolve().parent
stage = Path('/private/tmp/osv-sweep-runtime-bound-v6-source-20261016')
node = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
inventory = out / 'v5-provider-baseline-inventory.json'
files = json.loads(inventory.read_text())['files']
f = out / 'v6-provider-baseline-receipt.json'
assert not f.exists()
r = {'state': 'waiting-public100', 'passed': False, 'pid': os.getpid(),
    'driverSha256': sha(__file__), 'inventorySha256': sha(inventory),
    'runtimeBindingFixed': True, 'fullGoalComplete': False}

def save():
    p = f.with_suffix('.writing')
    p.write_text(json.dumps(r, indent=2) + '\n')
    p.replace(f)

save()
try:
    while True:
        q = json.loads((out / 'frozen-current-v5-qualification.json').read_text())
        if q['state'] in ['complete', 'failed', 'stopped-after-build-failure']:
            break
        time.sleep(5)
    if not q['passed']:
        r['state'] = 'stopped-after-runtime-failure'
        save()
        raise SystemExit(1)
    r.update(geometryWasmSha256=q['packedSha256'],
        prerequisiteSha256=sha(out / 'frozen-current-v5-qualification.json'))
    ledger = json.loads((stage / 'runtime-source-sha256.json').read_text())
    r['sourceLedgerSha256'] = sha(stage / 'runtime-source-sha256.json')
    r['testSourceHashes'] = {name: sha(stage / name) for name in files}
    def identity():
        assert sha(__file__) == r['driverSha256']
        assert sha(inventory) == r['inventorySha256']
        assert sha(stage / 'runtime-source-sha256.json') == r['sourceLedgerSha256']
        assert all(sha(stage / name) == digest for name, digest in ledger.items())
        for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm', 'dist/wasm/geometry-kernel.wasm']:
            assert sha(stage / name) == r['geometryWasmSha256']
        config = (stage / 'vitest.config.ts').read_text()
        assert 'testTimeout: 30000' in config and 'hookTimeout: 30000' in config
    since = None
    while True:
        now = time.monotonic()
        load = os.getloadavg()
        if load[0] <= 8:
            since = now if since is None else since
            if now - since >= 30:
                break
        else:
            since = None
        r.update(state='waiting-for-quiet-resources', lastObservedLoad=list(load))
        save()
        time.sleep(5)
    identity()
    report = out / 'v6-provider-baseline-tests.json'
    argv = [node, 'node_modules/vitest/vitest.mjs', 'run', *files,
        '--no-file-parallelism', '--reporter=default', '--reporter=json', '--outputFile=' + str(report)]
    r.update(state='running', argv=argv, launchLoad=list(os.getloadavg()))
    save()
    with (out / 'v6-provider-baseline.log').open('w') as log:
        result = subprocess.run(argv, cwd=stage, stdout=log, stderr=subprocess.STDOUT,
            env={**os.environ, 'PATH': str(Path(node).parent) + os.pathsep + os.environ['PATH'],
                'NODE_OPTIONS': '--loader=' + str(stage / 'qualification-raw-imports.mjs'), 'VITEST_MAX_WORKERS': '1', 'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2',
                'RUSTUP_TOOLCHAIN': 'nightly-2026-09-16', 'CARGO_NET_OFFLINE': 'true'})
    identity()
    assert result.returncode == 0, 'Inspect preserved provider baseline log'
    j = json.loads(report.read_text())
    assert j['numTotalTests'] == j['numPassedTests'] and j['numFailedTests'] == j['numPendingTests'] == 0
    assert j['numTotalTestSuites'] == j['numPassedTestSuites']
    r.update(state='complete', passed=True, tests=j['numPassedTests'], files=len(files),
        reportSha256=sha(report), scope='Native geometry/protocol baseline only. Metadata binding now matches actual artifact; this baseline alone does not qualify deployment, browser parity or the full goal.')
    save()
except Exception as error:
    r.update(state='failed', passed=False, reason=str(error))
    save()
    raise
