"""Qualify exact patch reuse with unchanged native budgets and public deadlines."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

stage = Path('/private/tmp/osv-sweep-shared-boundary-integer-norm-source-20261016')
out = Path(__file__).resolve().parent
node = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
ledger_path = stage / 'shared-boundary-integer-norm-source-sha256.json'
ledger = json.loads(ledger_path.read_text())
receipt_path = out / 'shared-boundary-integer-norm-artifact-validation.json'
assert not receipt_path.exists(), 'Preserve previous validation; no blind restarts'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
packed = stage / 'public/wasm/geometry-kernel.wasm'
raw = stage / 'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
receipt = {'schema': 'original-level-reuse-artifact/1', 'state': 'running', 'passed': False,
           'pid': os.getpid(), 'driverSha256': sha(__file__), 'phases': [],
           'sourceLedgerSha256': sha(ledger_path), 'sourceFiles': len(ledger),
           'packedSha256': sha(packed), 'rawSha256': sha(raw),
           'nodeExecutable': node, 'nodeVersion': subprocess.check_output([node, '--version'], text=True).strip(),
           'mainWasmModified': False, 'buildingArtifact': True, 'fullGoalComplete': False,
           'scope': 'Rust existing original Bernstein restriction used by two shared-boundary callers; ordinary generic fallback preserved on compact bounded cache baseline. Same cells, trim predicates, proof budgets, public deadlines and strict asset limits. Final source-bound delivery and whole-library qualification separate.'}
assert receipt['packedSha256'] == '7635823be9c0b8b97a12adc3d1e8bb69e247cbeaf4e0601c714790809b9e1d6a', 'Unexpected seed artifact; inspect before rebuilding'

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
    if 'public' in name:
        scheduled=time.monotonic()
        while os.getloadavg()[0]>os.cpu_count():
            phase.update(state='waiting-for-cpu-before-launch',lastObservedLoad=list(os.getloadavg()))
            save();time.sleep(5)
        phase.update(state='running',preLaunchSchedulingSeconds=time.monotonic()-scheduled,launchLoad=list(os.getloadavg()))
        save()
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


save()
native=json.loads((out/'shared-boundary-integer-native-full.json').read_text())
assert native['state']=='complete' and native['passed']
receipt['schema']='combined-kernel-exact-norm-build-and-strict-sizing/1'
receipt['nativeQualificationReceiptSha256']=sha(out/'shared-boundary-integer-native-full.json')
run('exact-norm-native-focused',['cargo','test','--locked','--manifest-path','crates/Cargo.toml','-p','nurbs-core','surface_distance','--lib'])
run('shared-boundary-integer-norm-wasm', [node, 'scripts/build-geometry-kernels.mjs'])
receipt.update(packedSha256=sha(packed), rawSha256=sha(raw), buildingArtifact=False)
assert receipt['packedSha256'] != '7635823be9c0b8b97a12adc3d1e8bb69e247cbeaf4e0601c714790809b9e1d6a', 'Changed Rust source must be built'
save()
run('shared-boundary-integer-norm-vue-typecheck', [node, 'node_modules/vue-tsc/bin/vue-tsc.js', '--noEmit'])
run('shared-boundary-integer-norm-mcp-typecheck', [node, 'node_modules/typescript/bin/tsc', '--noEmit', '-p', 'tsconfig.mcp.json'])
run('shared-boundary-integer-norm-application', [node, 'node_modules/vite/bin/vite.js', 'build'])
run('shared-boundary-integer-norm-distribution', [node, 'scripts/verify-dist.mjs'])
assert sha(stage / 'dist/wasm/geometry-kernel.wasm') == receipt['packedSha256']
receipt.update(state='complete',passed=True,buildAndStrictSizingComplete=True,runtimeQualificationComplete=False,deadlineClosureConfirmed=False)
receipt['scope']='Build/type/application/original8080000strictsizing only. All public runtime checks are serialized on the actual current-main deliverable; no duplicate legacy runtime matrix.'
save()
print('Native build and unchanged original sizing passed; current-main runtime performance remains pending',flush=True)
