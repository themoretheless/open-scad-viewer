"""Sequential fresh qualification, preserving the original geometry and deadlines."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

OUT = Path(__file__).resolve().parent
SNAPSHOT = OUT / 'snapshot.json'
snapshot = json.loads(SNAPSHOT.read_text())
STAGE = Path(snapshot['stage'])
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
ORACLE = '/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
WASM = snapshot['wasmSha256']
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger = json.loads((STAGE / 'qualification-inputs.json').read_text())
receipt_path = OUT / 'receipt.json'
assert not receipt_path.exists(), 'Keep failed and previous runs separate'
record = {'schema': 'sweep-b4cd-requalification/1', 'state': 'running', 'passed': False,
          'pid': os.getpid(), 'driverSha256': sha(__file__), 'snapshotSha256': sha(SNAPSHOT),
          'geometryWasmSha256': WASM, 'phases': [], 'testAndHookTimeoutMs': 30000,
          'maxWorkers': 1, 'fullRepositorySuite': False,
          'scope': 'Fresh public100, STEP207 plus supplements, full UI and native certificate evidence for the frozen installed artifact.'}

def save():
    temporary = receipt_path.with_suffix('.writing')
    temporary.write_text(json.dumps(record, indent=2) + '\n')
    temporary.replace(receipt_path)

def identity():
    assert sha(__file__) == record['driverSha256']
    assert sha(SNAPSHOT) == record['snapshotSha256']
    for name, expected in ledger.items():
        assert sha(STAGE / name) == expected, name
    for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm',
                 'dist/wasm/geometry-kernel.wasm']:
        assert sha(STAGE / name) == WASM, name

ENV = {**os.environ, 'PATH': str(Path(NODE).parent) + os.pathsep + os.environ['PATH'],
       'NODE_OPTIONS': '--max-old-space-size=8192', 'VITEST_MAX_WORKERS': '1',
       'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_NET_OFFLINE': 'true',
       'RUSTUP_TOOLCHAIN': 'nightly-2026-09-16', 'PYTHONDONTWRITEBYTECODE': '1'}

def run(name, argv):
    identity()
    phase = {'name': name, 'argv': list(map(str, argv)), 'state': 'running',
             'startedAt': time.time()}
    record['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    started = time.monotonic()
    with (OUT / (name + '.log')).open('w') as log:
        process = subprocess.Popen(phase['argv'], cwd=STAGE, env=ENV, stdout=log, stderr=subprocess.STDOUT)
        phase['pid'] = process.pid
        save()
        code = process.wait()
    identity()
    phase.update(state='passed' if code == 0 else 'failed', returncode=code,
                 seconds=time.monotonic() - started, logSha256=sha(OUT / (name + '.log')))
    save()
    print(name + ': ' + phase['state'], flush=True)
    assert code == 0, name + ': failed; original evidence retained'

def tests(name, files):
    report = OUT / (name + '.json')
    run(name, [NODE, 'node_modules/vitest/vitest.mjs', 'run', '--maxWorkers=1',
               '--no-file-parallelism', *files, '--reporter=default', '--reporter=json',
               '--outputFile=' + str(report)])
    data = json.loads(report.read_text())
    assert data['success'] and data['numFailedTests'] == 0
    observed = [str(Path(item['name']).relative_to(STAGE)) for item in data['testResults']]
    assert sorted(observed) == sorted(files)
    record[name] = {'passed': True, 'files': len(observed), 'tests': data['numTotalTests'],
                    'passedTests': data['numPassedTests'], 'reportSha256': sha(report)}
    save()

try:
    save()
    tests('original-two-files', ['tests/nurbsPeriodicFixedNormalArcRush.test.ts', 'tests/nurbsProgressiveMiter.test.ts'])
    tests('public100', list(snapshot['publicFiles']))
    assert record['public100']['passedTests'] == record['public100']['tests'] == 736
    run('native-producers', ['cargo', 'build', '--locked', '--offline', '--release',
        '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core',
        '--example', 'certify_sweep_geometry_migration', '--example', 'certify_retained_surface_interiors'])
    producer = STAGE / 'crates/target/release/examples/certify_sweep_geometry_migration'
    interior = STAGE / 'crates/target/release/examples/certify_retained_surface_interiors'
    record['nativeProducers'] = {str(p): sha(p) for p in [producer, interior]}
    save()
    diagnostic = OUT / 'multispan'
    diagnostic.mkdir()
    for name, script in [('constructed', 'check-staged-constructed-multispan-sweep.mts'),
                         ('packed-jets', 'check-staged-multispan-jets.mts'),
                         ('closed-jets', 'check-staged-closed-strip-jets.mts')]:
        run('multispan-' + name, [NODE, '--import', 'tsx', 'scripts/' + script, STAGE, diagnostic / (name + '.json')])
        assert json.loads((diagnostic / (name + '.json')).read_text())['passed']
    gates = OUT / 'artifact-gates'
    gates.mkdir()
    (gates / 'receipt.json').write_text(json.dumps({'state': 'complete', 'passed': True,
        'artifacts': {'packed': {'sha256': WASM}}, 'publicReportSha256': sha(OUT / 'public100.json'),
        'scope': 'Frozen b4cd identity and fresh complete 736-test execution; not historical evidence.'}, indent=2) + '\n')
    canonical = OUT / 'canonical207-ui478'
    run('canonical207-ui478', ['python3', 'scripts/qualify-sweep-current-artifact.py',
        '--output', canonical, '--artifact-gates', gates, '--wait-pid', str(os.getpid()),
        '--expected-wasm', WASM, '--native-producer', producer,
        '--source-ledger', 'qualification-inputs.json', '--qa-overrides', 'qa-source-overrides-current.json'])
    run('legacy-cap-refusal-controls', ['python3', 'scripts/qualify-sweep-legacy-cap-refusals.py',
        '--canonical', canonical, '--expected-wasm', WASM, '--output', OUT / 'legacy-cap-refusals',
        '--checker-directory', STAGE / 'scripts', '--node', NODE])
    run('retained-interiors', ['python3', 'scripts/qualify-retained-surface-interiors.py',
        canonical / 'step207/manifest.json', canonical / 'receipt.json', interior,
        STAGE / 'crates/brep-core/examples/certify_retained_surface_interiors.rs', OUT / 'retained-interiors'])
    c2 = OUT / 'c2-step4'
    run('c2-step4-export', [NODE, '--import', 'tsx', '--loader', './qualification-raw-imports.mjs', 'scripts/export-corrected-c2-step.mts', c2])
    run('c2-step4-reference', [ORACLE, 'scripts/reference-rational-boundary-volume.py', c2])
    run('c2-step4-occt', [ORACLE, 'scripts/verify-sweep-step-occt.py', c2])
    run('c2-ui8', [NODE, 'scripts/check-sweep-miter-matrix-browser.mjs', OUT / 'c2-ui8',
        '--corrected-c2-body-matrix', '--native-running-cancel', '--gpu-backend=metal'])
    identity()
    record.update(state='complete', passed=True)
    save()
except BaseException as error:
    record.update(state='failed', passed=False, error=str(error))
    save()
    raise
