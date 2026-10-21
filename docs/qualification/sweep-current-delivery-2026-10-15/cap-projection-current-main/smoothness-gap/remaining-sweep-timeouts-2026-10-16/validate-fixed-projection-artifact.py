"""Qualify exact patch reuse with unchanged native budgets and public deadlines."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

stage = Path('/private/tmp/osv-sweep-fixed-projection-source-20261016')
out = Path(__file__).resolve().parent
node = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
ledger_path = stage / 'fixed-projection-source-sha256.json'
ledger = json.loads(ledger_path.read_text())
receipt_path = out / 'fixed-projection-artifact-validation.json'
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
           'scope': 'Rust exact fixed-direction interval projection reuse on compact bounded cache baseline. Same cells, trim predicates, proof budgets, public deadlines and strict asset limits. Final source-bound delivery and whole-library qualification separate.'}
assert receipt['packedSha256'] == 'f49fdce82e4f89cd36568cbb56c8b52926a117c9aa936db687308fb8f670c388', 'Unexpected seed artifact; inspect before rebuilding'

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
paired=json.loads((out/'fixed-projection-paired.json').read_text())
assert paired['state']=='complete' and paired['eligibleForAdoption'], 'Do not build an unmeasured optimization'
assert '26 passed; 0 failed;' in (out/'fixed-projection-native-focused.log').read_text()
run('fixed-projection-native-full',['cargo','test','--locked','--manifest-path','crates/Cargo.toml','-p','nurbs-core','--lib'])
run('fixed-projection-native-face-contacts',['cargo','test','--locked','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib','face_contacts'])
run('fixed-projection-wasm', [node, 'scripts/build-geometry-kernels.mjs'])
receipt.update(packedSha256=sha(packed), rawSha256=sha(raw), buildingArtifact=False)
assert receipt['packedSha256'] != 'f49fdce82e4f89cd36568cbb56c8b52926a117c9aa936db687308fb8f670c388', 'Changed Rust source must be built'
save()
run('fixed-projection-vue-typecheck', [node, 'node_modules/vue-tsc/bin/vue-tsc.js', '--noEmit'])
run('fixed-projection-mcp-typecheck', [node, 'node_modules/typescript/bin/tsc', '--noEmit', '-p', 'tsconfig.mcp.json'])
run('fixed-projection-application', [node, 'node_modules/vite/bin/vite.js', 'build'])
run('fixed-projection-distribution', [node, 'scripts/verify-dist.mjs'])
assert sha(stage / 'dist/wasm/geometry-kernel.wasm') == receipt['packedSha256']
run('fixed-projection-remaining-public', [node,'node_modules/vitest/vitest.mjs','run','tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts','--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(out/'fixed-projection-remaining-public.json')])
run('fixed-projection-affine-public', [node,'node_modules/vitest/vitest.mjs','run','tests/sweepAffineLattice.test.ts','--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(out/'fixed-projection-affine-public.json')])
original=json.loads((out.parent/'current-source-build-arc-partition/public100-serial/receipt.json').read_text())
files=[x for x in original['argv'] if x.startswith('tests/') and x.endswith('.test.ts')]
assert len(files)==len(set(files))==100
run('fixed-projection-public100', [node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(out/'fixed-projection-public100.json')])
public=json.loads((out/'fixed-projection-public100.json').read_text())
assert public['numTotalTests']==public['numPassedTests']==736 and public['numFailedTests']==public['numPendingTests']==0
receipt['public100']={k:public[k] for k in ['numTotalTests','numPassedTests','numFailedTests','numPendingTests']}
receipt['deadlineClosureConfirmed']=True
receipt.update(state='complete', passed=True)
save()
print('Focused artifact gates passed; complete library qualification remains open', flush=True)
