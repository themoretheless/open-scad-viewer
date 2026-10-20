"""Qualify the isolated Rust contact cache128 candidate under original limits."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

stage = Path('/private/tmp/osv-sweep-contact-cache128-source-20261016')
out = Path(__file__).resolve().parent
node = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
ledger_path = stage / 'contact-cache128-source-sha256.json'
ledger = json.loads(ledger_path.read_text())
receipt_path = out / 'cache128-artifact-validation.json'
assert not receipt_path.exists(), 'Preserve previous validation; no blind restarts'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
packed = stage / 'public/wasm/geometry-kernel.wasm'
raw = stage / 'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
receipt = {'schema': 'contact-cache128-artifact/1', 'state': 'running', 'passed': False,
           'pid': os.getpid(), 'driverSha256': sha(__file__), 'phases': [],
           'sourceLedgerSha256': sha(ledger_path), 'sourceFiles': len(ledger),
           'packedSha256': sha(packed), 'rawSha256': sha(raw),
           'nodeExecutable': node, 'nodeVersion': subprocess.check_output([node, '--version'], text=True).strip(),
           'mainWasmModified': False, 'buildingArtifact': True, 'fullGoalComplete': False,
           'scope': 'Isolated6c6-derived Rust per-search interval cache128; native BRep, unchanged strict distribution and focused original public runtime checks. Does not replace full public100/STEP/UI/all-mode proof gates.'}
assert receipt['packedSha256'] == '6c6beba96f7254906fa1c8f4b27bdf35b42265ba1a67dd18460e9b3f01e81f6b', 'Unexpected seed artifact; inspect before rebuilding'

def save():
    tmp = receipt_path.with_suffix('.json.writing')
    tmp.write_text(json.dumps(receipt, indent=2) + '\n')
    tmp.replace(receipt_path)

def identity():
    assert sha(__file__) == receipt['driverSha256']
    assert sha(ledger_path) == receipt['sourceLedgerSha256']
    for name, digest in ledger.items():
        if name.startswith(('src/generated/geometry-kernels/', 'src/generated/wasm-brotli/')) or name == 'public/wasm/geometry-kernel.wasm':
            continue
        assert sha(stage / name) == digest, name
    if not receipt.get('buildingArtifact', False):
        assert sha(packed) == receipt['packedSha256']
        assert sha(raw) == receipt['rawSha256']
        assert sha(stage / 'src/generated/geometry-kernels/kernel_bg.wasm') == receipt['packedSha256']
        assert receipt['packedSha256'] in (stage / 'src/generated/geometry-kernels/identity.ts').read_text()
    verifier = (stage / 'scripts/verify-dist.mjs').read_text()
    assert 'const geometryChunkBudget = 3_970_000' in verifier
    assert 'const totalBudget = 8_080_000' in verifier

def run(name, argv):
    identity()
    phase = {'name': name, 'argv': argv, 'state': 'running'}
    receipt['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    started = time.monotonic()
    env = {**os.environ, 'VITEST_MAX_WORKERS': '1', 'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_NET_OFFLINE': 'true', 'RUSTUP_TOOLCHAIN': 'nightly-2026-09-16', 'BINARYEN_CORES': '2', 'OSV_WASM_OPT_NATIVE': '/Users/themoretheless/.cargo/bin/wasm-opt'}
    with (out / (name + '.log')).open('w') as log:
        status = subprocess.run(argv, cwd=stage, env=env, stdout=log, stderr=subprocess.STDOUT)
    identity()
    phase.update(state='passed' if status.returncode == 0 else 'failed', returncode=status.returncode,
                 seconds=time.monotonic() - started)
    save()
    print(name + ': ' + phase['state'], flush=True)
    if status.returncode:
        receipt['state'] = 'failed'
        save()
        raise SystemExit(status.returncode)

assert 'test result: ok. 1429 passed; 0 failed;' in (out / 'cache128-full-nurbs.log').read_text(), 'Wait for successful native NURBS qualification'
save()
run('native-brep', ['cargo', 'test', '--offline', '--locked', '--release', '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '--lib', '--', '--test-threads=2'])
run('wasm', [node, 'scripts/build-geometry-kernels.mjs'])
receipt.update(packedSha256=sha(packed), rawSha256=sha(raw), buildingArtifact=False)
assert receipt['packedSha256'] != '6c6beba96f7254906fa1c8f4b27bdf35b42265ba1a67dd18460e9b3f01e81f6b', 'Changed Rust source must be built'
save()
run('cache128-vue-typecheck', [node, 'node_modules/vue-tsc/bin/vue-tsc.js', '--noEmit'])
run('cache128-mcp-typecheck', [node, 'node_modules/typescript/bin/tsc', '--noEmit', '-p', 'tsconfig.mcp.json'])
run('cache128-application', [node, 'node_modules/vite/bin/vite.js', 'build'])
run('cache128-distribution', [node, 'scripts/verify-dist.mjs'])
assert sha(stage / 'dist/wasm/geometry-kernel.wasm') == receipt['packedSha256']
run('cache128-affine-public', [node, 'node_modules/vitest/vitest.mjs', 'run', 'tests/sweepAffineLattice.test.ts',
                     '--no-file-parallelism', '--reporter=json', '--outputFile=' + str(out / 'cache128-affine-public.json')])
run('remaining-public', [node, 'node_modules/vitest/vitest.mjs', 'run', 'tests/nurbsPeriodicFixedNormalArcRush.test.ts', 'tests/nurbsClosedCorrectedRegularBody.test.ts', 'tests/nurbsSpatialRmfError.test.ts', '--no-file-parallelism', '--reporter=json', '--outputFile=' + str(out / 'remaining-public.json')])
receipt.update(state='complete', passed=True)
save()
print('Focused artifact gates passed; complete library qualification remains open', flush=True)
