"""Guarded reversible runtime installation, followed by actual main-tree gates."""
from pathlib import Path
from datetime import datetime, timezone
import hashlib, json, os, shutil, subprocess

REC = Path(__file__).resolve().parent
Q = REC.parent
ROOT = Q.parents[2]
OPT = Q / 'optimized'
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
WASM = '6744fc28c6c750b94e97d8a3790dcc321ba7b72fc022778c78ffaf3192684f1b'
OLD = 'b4cd5d582ad11bb4f11aebbc0567f98bbf33df7f8a2d57f3a28a2b1b00cc9e06'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
PAYLOAD = REC / 'runtime-v5'
OUT = REC / 'main-delivery'
assert not OUT.exists(), 'Preserve every delivery attempt'
OUT.mkdir()
record = {'state': 'preparing', 'passed': False, 'geometryWasmSha256': WASM,
          'driverSha256': sha(__file__), 'phases': [], 'files': {},
          'focusPolicy': 'Headless Chromium only; no navigation, foregrounding or server restart'}
env = {**os.environ, 'PATH': str(Path(NODE).parent) + ':' + os.environ['PATH'],
       'NODE_OPTIONS': '--max-old-space-size=8192', 'VITEST_MAX_WORKERS': '1'}

def save():
    p = OUT / 'receipt.writing'
    p.write_text(json.dumps(record, indent=2) + '\n'); p.replace(OUT / 'receipt.json')

def run(name, argv):
    p = {'name': name, 'state': 'running', 'argv': list(map(str, argv))}
    record['phases'].append(p); save(); print(name + ': started', flush=True)
    with (OUT / (name + '.log')).open('w') as stream:
        child = subprocess.Popen(p['argv'], cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
        p['pid'] = child.pid; save(); code = child.wait()
    p.update(state='passed' if code == 0 else 'failed', returncode=code,
             logSha256=sha(OUT / (name + '.log'))); save()
    print(name + ': ' + p['state'], flush=True); assert code == 0, name

def atomic(relative, data):
    target = ROOT / relative; target.parent.mkdir(parents=True, exist_ok=True)
    temporary = OUT / 'atomic-payload'
    temporary.write_bytes(data); temporary.replace(target)

def capture(name):
    code = "import{GEOMETRY_MANIFEST_ARCHIVE as a}from'./src/core/geometryExecution.ts';import{writeFileSync}from'node:fs';writeFileSync(process.argv[1],JSON.stringify(Object.fromEntries(['own-rust-node-v3','brep-closed-v3','own-rust-node-v4','brep-closed-v4'].map(k=>[k,a[k]]))))"
    run(name, [NODE, '--import', 'tsx', '--loader', ROOT / 'docs/qualification/sweep-current-delivery-2026-10-15/qualification-raw-imports.mjs',
               '--input-type=module', '-e', code, OUT / (name + '.json')])

try:
    qualification = Q / 'final-qualification.json'
    assert json.loads(qualification.read_text())['passed']
    record['qualificationSha256'] = sha(qualification)
    assert sha(ROOT / 'public/wasm/geometry-kernel.wasm') == OLD
    parent = ROOT / 'docs/qualification/trimmed-branches-topology-2026-10-19'
    baseline = json.loads((parent / 'runtime-source-sha256.json').read_text())
    baseline.update(json.loads((parent / 'source-receipt.json').read_text())['runtimeSha256'])
    runtime_files = ['src/core/geometryExecution.ts', 'src/core/currentRuntimeGeometryEvidence.ts',
                     'tests/engineManifest.test.ts', 'tests/manifoldPlanBackend.test.ts']
    for name in runtime_files:
        assert sha(ROOT / name) == baseline[name], 'Concurrent runtime change: ' + name
    capture('archive-before')
    files = {name: (PAYLOAD / name).read_bytes() for name in runtime_files}
    files['src/core/runtimeGeometryEvidenceV4.ts'] = (PAYLOAD / 'src/core/runtimeGeometryEvidenceV4.ts').read_bytes()
    archived_v4 = ROOT / 'src/core/runtimeGeometryEvidenceV4.ts'
    if archived_v4.exists():
        assert archived_v4.read_bytes() == files['src/core/runtimeGeometryEvidenceV4.ts'], 'Conflicting v4 archive'
    for source in (PAYLOAD / 'src/generated/geometry-kernels').iterdir():
        if source.is_file():
            name = 'src/generated/geometry-kernels/' + source.name
            if not (ROOT / name).exists() or sha(ROOT / name) != sha(source):
                if name in baseline: assert sha(ROOT / name) == baseline[name], name
                files[name] = source.read_bytes()
    files['public/wasm/geometry-kernel.wasm'] = (REC / 'recovered-geometry-kernel.wasm').read_bytes()
    for name, data in files.items():
        old = ROOT / name
        backup = OUT / 'backup' / name
        if old.exists():
            backup.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(old, backup)
        record['files'][name] = {'before': sha(old) if old.exists() else None,
                               'after': hashlib.sha256(data).hexdigest()}
    identity_text = (PAYLOAD / 'src/core/currentRuntimeGeometryEvidence.ts').read_text()
    identity = json.loads(identity_text[identity_text.index('{'):identity_text.rindex('}') + 1])
    runtime_record = ROOT / identity['recordId']; assert not runtime_record.exists()
    runtime = {'schema': 'runtime-provider-qualification/1', 'state': 'complete', 'qualified': True,
               'recordId': identity['recordId'], 'corpusVersion': identity['corpusVersion'],
               'target': 'browser-worker/node-mcp', 'artifacts': identity,
               'fullGoalComplete': False,
               'scope': 'Completed finite sweep requalification of exact recovered native artifact; not universal CAD geometry coverage.',
               'qualificationDirectory': str(Q.relative_to(ROOT)),
               'qualificationSha256': record['qualificationSha256'],
               'deliveryState': 'pending-main-verification'}
    runtime_record.write_text(json.dumps(runtime, indent=2) + '\n')
    assert sha(ROOT / 'public/wasm/geometry-kernel.wasm') == OLD
    save()
    ordered = sorted(files, key=lambda name: (0 if name.endswith('runtimeGeometryEvidenceV4.ts') else
        98 if name.endswith('currentRuntimeGeometryEvidence.ts') else
        99 if name.endswith('geometryExecution.ts') else 50, name))
    for name in ordered:
        data = files[name]
        current = ROOT / name; expected = record['files'][name]['before']
        assert (sha(current) if current.exists() else None) == expected, name
        atomic(name, data)
    record['state'] = 'installed-verifying'; save()
    capture('archive-after')
    assert sha(OUT / 'archive-before.json') == sha(OUT / 'archive-after.json')
    provider = json.loads((OPT / 'provider303.json').read_text())
    provider_files = [str(Path(t['name']).relative_to(Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')))
                      for t in provider['testResults']]
    run('provider303', [NODE, 'node_modules/vitest/vitest.mjs', 'run', '--maxWorkers=1', '--no-file-parallelism',
                       *provider_files, '--reporter=default', '--reporter=json', '--outputFile=' + str(OUT / 'provider303.json')])
    public = json.loads((OPT / 'snapshot.json').read_text())['publicFiles']
    for name, expected in public.items(): assert sha(ROOT / name) == expected, name
    run('public736', [NODE, 'node_modules/vitest/vitest.mjs', 'run', '--maxWorkers=1', '--no-file-parallelism',
                     *public, '--reporter=default', '--reporter=json', '--outputFile=' + str(OUT / 'public736.json')])
    result = json.loads((OUT / 'public736.json').read_text())
    assert result['success'] and result['numPassedTests'] == result['numTotalTests'] == 736
    run('types', [NODE, 'node_modules/vue-tsc/bin/vue-tsc.js', '--noEmit'])
    run('mcp-types', [NODE, 'node_modules/typescript/bin/tsc', '--noEmit', '-p', 'tsconfig.mcp.json'])
    run('actual-runtime', [NODE, '--import', 'tsx', '--loader', ROOT / 'docs/qualification/sweep-current-delivery-2026-10-15/qualification-raw-imports.mjs', 'scripts/audit-runtime-manifests.mts'])
    candidate = OUT / 'dist-candidate'
    run('main-dist', [NODE, 'node_modules/vite/bin/vite.js', 'build', '--outDir', candidate, '--emptyOutDir'])
    original = (ROOT / 'scripts/verify-dist.mjs').read_text()
    adapted = original.replace("new URL('../dist/', import.meta.url)", 'new URL(' + json.dumps(candidate.as_uri() + '/') + ')')
    adapted = adapted.replace("'./verify-packed-wasm.mjs'", json.dumps((ROOT / 'scripts/verify-packed-wasm.mjs').as_uri()))
    adapted = adapted.replace("new URL('../src/", "new URL('" + (ROOT / 'src').as_uri() + '/')
    adapted = adapted.replace("new URL(`../crates/", "new URL(`" + (ROOT / 'crates').as_uri() + '/')
    # The variable 'original' in the raw-WASM loop also uses script-relative paths.
    adapted = adapted.replace('new URL(original, import.meta.url)',
                              'new URL(original, ' + json.dumps((ROOT / 'scripts/verify-dist.mjs').as_uri()) + ')')
    verifier = OUT / 'verify-candidate.mjs'; verifier.write_text(adapted)
    record['distBudgetVerifierOriginalSha256'] = sha(ROOT / 'scripts/verify-dist.mjs')
    record['distBudgetVerifierAdaptedSha256'] = sha(verifier); save()
    run('candidate-dist-budgets', [NODE, verifier])
    run('main-ui8', [NODE, 'scripts/check-sweep-miter-matrix-browser.mjs', OUT / 'ui8',
        '--periodic-fixed-normal-station-matrix', '--native-running-cancel', '--gpu-backend=metal',
        '--dist-directory=' + str(candidate)])
    ui = json.loads((OUT / 'ui8/matrix.json').read_text())
    assert ui['passed'] and not ui['headed'] and len(ui['cases']) == 8
    assert ui['geometryWasmSha256'] == WASM
    for name, entry in record['files'].items(): assert sha(ROOT / name) == entry['after'], name
    if (ROOT / 'dist').exists(): (ROOT / 'dist').replace(OUT / 'backup/dist')
    candidate.replace(ROOT / 'dist')
    run('installed-dist-budgets', [NODE, 'scripts/verify-dist.mjs'])
    assert sha(ROOT / 'dist/wasm/geometry-kernel.wasm') == WASM
    assert sha(ROOT / 'dist/index.html') == ui['distIndexSha256']
    runtime.update(deliveryState='installed-verified', deliveredAtUtc=datetime.now(timezone.utc).isoformat(),
                   mainVerificationReceipt=str((OUT / 'receipt.json').relative_to(ROOT)))
    runtime_record.write_text(json.dumps(runtime, indent=2) + '\n')
    record.update(state='complete', passed=True, runtimeQualificationSha256=sha(runtime_record)); save()
except BaseException as error:
    record.update(state='failed', passed=False, error=str(error)); save(); raise
