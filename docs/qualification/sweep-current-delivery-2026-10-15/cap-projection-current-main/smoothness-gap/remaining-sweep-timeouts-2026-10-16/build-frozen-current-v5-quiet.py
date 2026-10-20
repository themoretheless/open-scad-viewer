"""Build the agreed immutable candidate when resources permit; never publish."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

out = Path(__file__).resolve().parent
stage = Path('/private/tmp/osv-sweep-frozen-current-v5-source-20261016')
main = Path.cwd()
node = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
f = out / 'frozen-current-v5-build.json'
assert not f.exists(), 'Preserve the existing build'
capture = json.loads((out / 'frozen-current-v5-capture.json').read_text())
assert capture['state'] == 'complete' and capture['passed']
native_path = stage / 'main-native-source-sha256.json'
runtime_path = stage / 'runtime-source-sha256.json'
external_path = stage / 'qualification-external-inputs.json'
native = json.loads(native_path.read_text())
runtime = json.loads(runtime_path.read_text())
extra = json.loads(external_path.read_text())
r = {'state': 'waiting-for-quiet-resources', 'passed': False, 'pid': os.getpid(),
    'stage': str(stage), 'driverSha256': sha(__file__), 'phases': [],
    'captureReceiptSha256': sha(out / 'frozen-current-v5-capture.json'),
    'inputBindingReceiptSha256': sha(out / 'v5-snapshot-input-binding.json'),
    'nativeLedgerSha256': sha(native_path), 'runtimeLedgerSha256': sha(runtime_path),
    'externalLedgerSha256': sha(external_path), 'mainWasmBefore': sha(main / 'public/wasm/geometry-kernel.wasm'),
    'resourceGate': {'maxOneMinuteLoad': 8, 'stableSeconds': 30, 'compilerNice': 19},
    'fullGoalComplete': False, 'published': False}

def save():
    temporary = f.with_suffix('.writing')
    temporary.write_text(json.dumps(r, indent=2) + '\n')
    temporary.replace(f)

def identity():
    assert sha(__file__) == r['driverSha256']
    assert sha(native_path) == r['nativeLedgerSha256']
    assert sha(runtime_path) == r['runtimeLedgerSha256']
    assert sha(external_path) == r['externalLedgerSha256']
    assert all(sha(stage / name) == h for name, h in native.items())
    assert all(sha(stage / name) == h for name, h in runtime.items())
    assert all(sha(stage / name) == m['sha256'] for name, m in extra['files'].items())
    assert all(sha(Path(extra['browserDependenciesRealPath']) / name) == h
        for name, h in extra['browserDependencyFiles'].items())

def quiet():
    since = None
    while True:
        now = time.monotonic()
        load = os.getloadavg()
        if load[0] <= 8:
            since = now if since is None else since
            if now - since >= 30:
                return
        else:
            since = None
        r.update(state='waiting-for-quiet-resources', lastObservedLoad=list(load),
            quietSeconds=0 if since is None else now-since)
        save()
        time.sleep(5)

def run(name, argv):
    quiet()
    identity()
    phase = {'name': name, 'argv': argv, 'state': 'running', 'launchLoad': list(os.getloadavg())}
    r['phases'].append(phase)
    r['state'] = 'running'
    save()
    print(name + ': started', flush=True)
    started = time.monotonic()
    env = {**os.environ, 'PATH': str(Path(node).parent) + os.pathsep + os.environ['PATH'],
        'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_NET_OFFLINE': 'true',
        'RUSTUP_TOOLCHAIN': 'nightly-2026-09-16', 'BINARYEN_CORES': '2',
        'OSV_WASM_OPT_NATIVE': '/Users/themoretheless/.cargo/bin/wasm-opt'}
    with (out / ('v5-' + name + '.log')).open('w') as log:
        result = subprocess.run(['nice', '-n', '19', *argv], cwd=stage, env=env,
            stdout=log, stderr=subprocess.STDOUT)
    identity()
    phase.update(state='passed' if result.returncode == 0 else 'failed',
        returncode=result.returncode, seconds=time.monotonic()-started)
    save()
    if result.returncode:
        raise RuntimeError(name + ' failed; inspect preserved log')
    print(name + ': passed', flush=True)

save()
try:
    run('geometry-build', [node, 'scripts/build-geometry-kernels.mjs'])
    packed = stage / 'public/wasm/geometry-kernel.wasm'
    raw = stage / 'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
    assert sha(packed) == sha(stage / 'src/generated/geometry-kernels/kernel_bg.wasm')
    r.update(packedSha256=sha(packed), rawSha256=sha(raw))
    save()
    run('native-migration-producer', ['cargo', 'build', '--locked', '--offline', '--release',
        '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core', '--example', 'certify_sweep_geometry_migration'])
    producer = stage / 'crates/target/release/examples/certify_sweep_geometry_migration'
    assert producer.is_file()
    r.update(state='complete', passed=True, nativeProducerSha256=sha(producer),
        sourceChangedDuringBuild=False, mainWasmAfter=sha(main / 'public/wasm/geometry-kernel.wasm'))
    save()
    adapter = {'schema': 'frozen-current-build-adapter/1', 'state': 'complete', 'passed': True,
        'scope': 'Source-coherent WASM/native build only; runtime/STEP/UI and full goal remain unqualified.',
        'artifacts': {kind: {'sha256': sha(p), 'byteLength': p.stat().st_size} for kind, p in [('raw', raw), ('packed', packed)]},
        'buildReceipt': str(f), 'buildReceiptSha256': sha(f),
        'nativeLedgerSha256': sha(native_path), 'fullGoalComplete': False}
    (stage / 'completed-build-receipt.json').write_text(json.dumps(adapter, indent=2) + '\n')
    print('V5 build complete; no application publication or runtime qualification claimed.', flush=True)
except Exception as error:
    r.update(state='failed', passed=False, reason=str(error))
    save()
    raise
