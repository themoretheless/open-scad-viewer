"""Build the immutable current-source snapshot and retain every gate result."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time

stage = Path(sys.argv[1]) if len(sys.argv)>1 else Path('/private/tmp/osv-sweep-shared-generators-current-source-20261015')
out = Path(sys.argv[2]) if len(sys.argv)>2 else Path(__file__).resolve().parent / 'current-source-build'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger = json.loads((stage / 'frozen-current-main-source-sha256.json').read_text())
capture = json.loads((stage / 'source-capture-receipt.json').read_text())
seed = json.loads((stage / 'dependency-seed-receipt.json').read_text())
assert capture['passed'] and seed['generatedSeedQualified'] is False
receipt = {'schema': 'current-shared-generator-source-build/1', 'state': 'running', 'passed': False,
           'pid': os.getpid(), 'stage': str(stage), 'driverSha256': sha(__file__),
           'sourceLedgerSha256': sha(stage / 'frozen-current-main-source-sha256.json'),
           'sourceFilesChecked': len(ledger), 'phases': [], 'mainModified': False,
           'fullGoalComplete': False, 'scope': 'Fresh immutable current-main snapshot, including shared-generator cap fix, arc partition reuse and concurrent native additions. Build gates do not establish all-mode proof or STEP/UI qualification.'}
assert not (out / 'receipt.json').exists(), 'Do not restart an existing build'

def save():
    p = out / 'receipt.json.writing'
    p.write_text(json.dumps(receipt, indent=2) + '\n')
    p.replace(out / 'receipt.json')

def identity():
    assert sha(__file__) == receipt['driverSha256']
    for name, h in ledger.items():
        assert sha(stage / name) == h, name
    for name, h in capture['testOnlyExternalInputs'].items():
        assert sha(stage / name) == h, name
    assert sha(stage / 'package-lock.json') == seed['packageLockSha256']
    assert sha(Path(seed['nodeModulesRealPath']) / '.package-lock.json') == seed['installedNodeLockSha256']
    assert sha(seed['optimizerPath']) == seed['optimizerSha256']

base = ['cargo', 'test', '--offline', '--locked', '--release', '--manifest-path', 'crates/Cargo.toml']
commands = [
 ('focused-arc-partition', base + ['-p', 'nurbs-core', '--lib', 'curve_measure::', '--', '--test-threads=2']),
 ('focused-cap-projection', base + ['-p', 'nurbs-core', '--lib', 'section_projection::tests::', '--', '--test-threads=2']),
 ('whole-body-shared-generators', base + ['-p', 'brep-core', '--test', 'progressive_c2_planar_caps', '--', '--test-threads=2', '--nocapture']),
 ('full-nurbs', base + ['-p', 'nurbs-core', '--lib', '--', '--test-threads=2']),
 ('full-brep', base + ['-p', 'brep-core', '--lib', '--', '--test-threads=2']),
 ('full-geometry-bridge', base + ['-p', 'geometry-bridge', '--lib', '--', '--test-threads=2']),
 ('native-migration-producer', ['cargo', 'build', '--offline', '--locked', '--release', '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '--example', 'certify_sweep_geometry_migration']),
 ('native-and-aux-wasm', ['npm', 'run', 'build:geometry']),
 ('vue-typecheck', ['node', 'node_modules/vue-tsc/bin/vue-tsc.js', '--noEmit']),
 ('mcp-typecheck', ['node', 'node_modules/typescript/bin/tsc', '--noEmit', '-p', 'tsconfig.mcp.json']),
 ('application', ['node', 'node_modules/vite/bin/vite.js', 'build']),
 ('distribution', ['node', 'scripts/verify-dist.mjs']),
]
identity()
save()
for name, argv in commands:
    identity()
    phase = {'name': name, 'state': 'running', 'argv': argv}
    receipt['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    start = time.monotonic()
    with (out / (name + '.log')).open('w') as log:
        status = subprocess.run(argv, cwd=stage, env={**os.environ,
            'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_NET_OFFLINE': 'true',
            'OSV_WASM_OPT_NATIVE': seed['optimizerPath'], 'BINARYEN_CORES': '2',
            'PYTHONDONTWRITEBYTECODE': '1'}, stdout=log, stderr=subprocess.STDOUT)
    identity()
    phase.update(state='passed' if status.returncode == 0 else 'failed',
                 returncode=status.returncode, seconds=time.monotonic() - start)
    save()
    print(name + ': ' + phase['state'], flush=True)
    if status.returncode:
        receipt['state'] = 'failed'
        save()
        raise SystemExit(status.returncode)
raw = stage / 'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
packed = stage / 'public/wasm/geometry-kernel.wasm'
assert packed.read_bytes() == (stage / 'src/generated/geometry-kernels/kernel_bg.wasm').read_bytes()
assert sha(packed) in (stage / 'src/generated/geometry-kernels/identity.ts').read_text()
assert packed.read_bytes() == (stage / 'dist/wasm/geometry-kernel.wasm').read_bytes()
producer = stage / 'crates/target/release/examples/certify_sweep_geometry_migration'
shutil.copy2(producer, out / 'certify-sweep-geometry-migration')
receipt.update(state='complete', passed=True, nativeProducerSha256=sha(producer),
    artifacts={kind: {'sha256': sha(p), 'byteLength': p.stat().st_size}
               for kind, p in [('raw', raw), ('packed', packed)]})
identity()
save()
shutil.copy2(out / 'receipt.json', stage / 'completed-build-receipt.json')
print('Fresh source build passed; actual artifact QA and full objective audit remain', flush=True)
