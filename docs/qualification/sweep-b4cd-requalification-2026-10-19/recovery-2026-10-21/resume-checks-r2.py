"""Resume captured UI cases only after exact artifact and application recovery."""
from pathlib import Path
import ast, hashlib, json, os, shutil, subprocess, time

REC = Path(__file__).resolve().parent
Q = REC.parent
O = Q / 'optimized'
ROOT = Q.parents[2]
S = Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
BASE = Path('/private/tmp/osv-sweep-b4cd-requalification-20261019')
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
WASM = '6744fc28c6c750b94e97d8a3790dcc321ba7b72fc022778c78ffaf3192684f1b'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
path = O / 'finish-recovered-r2-20261021.json'
assert not path.exists(), 'Preserve every attempt'
record = {'state': 'preparing', 'passed': False, 'pid': os.getpid(),
          'geometryWasmSha256': WASM, 'driverSha256': sha(__file__), 'phases': [],
          'interruptedParent': str(O / 'finish-retry2.json'),
          'interruptedParentSha256': sha(O / 'finish-retry2.json'),
          'sourceArchiveSha256': sha(REC / 'recovered-source.tar.gz')}
env = {**os.environ, 'PATH': str(Path(NODE).parent) + ':' + os.environ['PATH'],
       'NODE_OPTIONS': '--max-old-space-size=8192', 'RUSTUP_TOOLCHAIN': 'nightly-2026-09-16',
       'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_NET_OFFLINE': 'true',
       'PYTHONDONTWRITEBYTECODE': '1'}

def save():
    temporary = path.with_suffix('.writing')
    temporary.write_text(json.dumps(record, indent=2) + '\n')
    temporary.replace(path)

def run(name, argv):
    phase = {'name': name, 'state': 'running', 'argv': list(map(str, argv))}
    record['phases'].append(phase); save(); print(name + ': started', flush=True)
    with (REC / (name + '.log')).open('w') as stream:
        child = subprocess.Popen(phase['argv'], cwd=S, env=env, stdout=stream, stderr=subprocess.STDOUT)
        phase['pid'] = child.pid; save(); code = child.wait()
    phase.update(state='passed' if code == 0 else 'failed', returncode=code,
                 logSha256=sha(REC / (name + '.log')))
    save(); print(name + ': ' + phase['state'], flush=True)
    assert code == 0, name

try:
    save()
    assert json.loads((REC / 'recovered-build.json').read_text())['passed']
    assert sha(S / 'public/wasm/geometry-kernel.wasm') == WASM
    laws = S / 'qualification-law-sources'; laws.mkdir(exist_ok=True)
    for source in (O / 'canonical207-ui478/step207').glob('*.r'):
        shutil.copy2(source, laws / source.name)
    # Replay the original metadata-only v4 archive/v5 binding transformations.
    original = (BASE / 'src/core/currentRuntimeGeometryEvidence.ts').read_bytes()
    target = BASE / 'src/core/currentRuntimeGeometryEvidence.ts'
    target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(original)
    parsed = ast.parse((Q / 'prepare-optimized.py').read_text())
    replace = next(n for n in parsed.body if isinstance(n, ast.FunctionDef) and n.name == 'replace')
    body = next(n for n in parsed.body if isinstance(n, ast.Try)).body
    start = next(i for i, n in enumerate(body) if isinstance(n, ast.Assign)
                 and any(isinstance(t, ast.Name) and t.id == 'old' for t in n.targets))
    end = next(i for i, n in enumerate(body) if i > start and isinstance(n, ast.Expr)
               and isinstance(n.value, ast.Call) and isinstance(n.value.func, ast.Name)
               and n.value.func.id == 'run')
    ns = {'STAGE': S, 'BASE': BASE, 'ROOT': ROOT, 'sha': sha, 'json': json,
          'shutil': shutil, 'paired': json.loads((Q / 'paired-interval/receipt.json').read_text())}
    assert (S / 'src/core/runtimeGeometryEvidenceV4.ts').exists()
    assert WASM in (S / 'src/core/currentRuntimeGeometryEvidence.ts').read_text()
    old_ui = O / 'ui478-complete-retry2'
    old_plan = json.loads((old_ui / 'inventory-plan.json').read_text())
    for job in old_plan['jobs']:
        for source in job['sources']:
            assert sha(Path(job['sourceDirectory']) / source['file']) == source['sha256']
        matrix = old_ui / job['name'] / 'matrix.json'
        if matrix.exists():
            for name, expected in json.loads(matrix.read_text())['sourceHashes'].items():
                assert sha(S / name) == expected, name
    run('restored-dist', [NODE, 'node_modules/vite/bin/vite.js', 'build'])
    assert sha(S / 'dist/index.html') == old_plan['distIndexSha256'], 'Recovered application differs'
    assert sha(S / 'dist/wasm/geometry-kernel.wasm') == WASM
    ledger = {}
    for parent in S.iterdir():
        if parent.name in ['node_modules', 'tools', 'qualification-inputs.json']: continue
        for p in parent.rglob('*') if parent.is_dir() else [parent]:
            if p.is_file() and 'target' not in p.relative_to(S).parts and '__pycache__' not in p.parts:
                ledger[str(p.relative_to(S))] = sha(p)
    (S / 'qualification-inputs.json').write_text(json.dumps(ledger, indent=2) + '\n')
    record['recoveredInputLedgerSha256'] = sha(S / 'qualification-inputs.json'); save()
    recovered_snapshot = json.loads((O / 'snapshot.json').read_text())
    recovered_snapshot.update(inputLedgerSha256=record['recoveredInputLedgerSha256'],
        inputFiles=len(ledger), recoverySourceArchiveSha256=record['sourceArchiveSha256'],
        scope='Recovered original native/runtime bytes and identical WASM/dist; new recovered input ledger is explicitly distinct from the unavailable original full ledger.')
    (O / 'snapshot-recovered-20261021.json').write_text(json.dumps(recovered_snapshot, indent=2) + '\n')
    tail = REC / 'surface-tail'
    run('surface-tail9', [NODE, 'scripts/check-sweep-miter-matrix-browser.mjs', tail,
        '--surface', '--start-at=600:closed-guided-progressive-sweep.r', '--gpu-backend=metal'])
    old_path = old_ui / 'surface/matrix.json'
    new_path = tail / 'matrix.json'
    old, new = [json.loads(p.read_text()) for p in [old_path, new_path]]
    assert new['passed'] and old['sourceHashes'] == new['sourceHashes']
    good = [c for c in old['cases'] if c['status'] == 'passed']
    assert len(good) == 101 and len(new['cases']) == 9
    composite = {**new, 'cases': good + new['cases'],
        'selection': {**new['selection'], 'startAt': None,
                      'plannedCaseKeys': old['selection']['fullInventory']},
        'continuationInputs': [{'path': str(p), 'sha256': sha(p), 'successfulCases': n}
                               for p, n in [(old_path, 101), (new_path, 9)]]}
    composite_path = REC / 'surface-complete.json'
    composite_path.write_text(json.dumps(composite, indent=2) + '\n')
    ui = REC / 'ui478-complete'
    reuse = []
    for job in old_plan['jobs'][:-1]:
        p = composite_path if job['name'] == 'surface' else old_ui / job['name'] / 'matrix.json'
        reuse.append('--reuse-report=' + job['name'] + ':' + str(p))
    run('full-ui478-reconciliation', [NODE, 'scripts/run-fresh-full-ui-audit.mjs', ui, *reuse])
    result = json.loads((ui / 'full-reconciliation.json').read_text())
    assert result['passed'] and result['cases'] == 478 and result['nativeObservations'] == 1472
    for job in old_plan['jobs'][:-1]:
        directory = ui / job['name']; directory.mkdir()
        target = composite_path if job['name'] == 'surface' else old_ui / job['name'] / 'matrix.json'
        (directory / 'matrix.json').symlink_to(target)
    canonical = O / 'canonical-complete'; canonical.mkdir()
    for name in ['step207', 'annular-station']:
        (canonical / name).symlink_to(O / 'canonical207-ui478' / name, target_is_directory=True)
    (canonical / 'ui478').symlink_to(ui, target_is_directory=True)
    prior = json.loads((O / 'canonical207-ui478/receipt.json').read_text())
    composed = {**prior, 'state': 'complete', 'passed': True,
        'finalManifestSha256': sha(canonical / 'step207/manifest.json'),
        'phases': [p for p in prior['phases'] if p['state'] == 'passed'] + [record['phases'][-1]],
        'freshUiReportSha256': sha(ui / 'full-reconciliation.json'),
        'recoveryReceipt': str(path), 'scope': 'Exact recovered native/runtime artifact and hash-verified unchanged application; completed original STEP phases plus authenticated continued UI.'}
    (canonical / 'receipt.json').write_text(json.dumps(composed, indent=2) + '\n')
    run('native-producers', ['cargo', 'build', '--locked', '--offline', '--release',
        '--manifest-path', 'crates/Cargo.toml', '-p', 'brep-core',
        '--example', 'certify_sweep_geometry_migration', '--example', 'certify_retained_surface_interiors'])
    run('legacy-cap-refusal-controls', ['python3', 'scripts/qualify-sweep-legacy-cap-refusals.py',
        '--canonical', canonical, '--expected-wasm', WASM, '--output', O / 'legacy-cap-refusals',
        '--checker-directory', S / 'scripts', '--node', NODE])
    run('retained-interiors', ['python3', 'scripts/qualify-retained-surface-interiors.py',
        canonical / 'step207/manifest.json', canonical / 'receipt.json',
        S / 'crates/target/release/examples/certify_retained_surface_interiors',
        S / 'crates/brep-core/examples/certify_retained_surface_interiors.rs', O / 'retained-interiors'])
    run('c2-ui8', [NODE, 'scripts/check-sweep-miter-matrix-browser.mjs', O / 'c2-ui8',
        '--corrected-c2-body-matrix', '--native-running-cancel', '--gpu-backend=metal'])
    record.update(state='complete', passed=True); save()
except BaseException as error:
    record.update(state='failed', passed=False, error=str(error)); save(); raise
