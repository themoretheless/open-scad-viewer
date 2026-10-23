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
OUT = REC / 'main-delivery-r3'
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
    previous_path = REC / 'main-delivery-r2/receipt.json'
    previous = json.loads(previous_path.read_text())
    assert previous['state'] == 'failed' and previous['error'] == 'candidate-dist-budgets'
    public_result = json.loads((previous_path.parent / 'public736.json').read_text())
    assert public_result['success'] and public_result['numPassedTests'] == public_result['numTotalTests'] == 736
    record.update(state='installed-verifying', files=previous['files'],
        qualificationSha256=previous['qualificationSha256'],
        previousFailedReceiptSha256=sha(previous_path),
        reusedPublicReportSha256=sha(previous_path.parent / 'public736.json'),
        phases=[p for p in previous['phases'] if p['state'] == 'passed' and p['name'] not in ['main-dist','types','mcp-types','actual-runtime']],
        continuationReason='Unchanged 736-test result retained; only content-hashed chunk URL naming shortened to meet unchanged 8410000-byte aggregate budget. No Vue feature flags or runtime minifier settings changed.',
        buildConfigSha256=sha(ROOT / 'vite.config.ts'))
    for name, entry in record['files'].items(): assert sha(ROOT / name) == entry['after'], name
    assert sha(ROOT / 'public/wasm/geometry-kernel.wasm') == WASM
    runtime_record = ROOT / 'docs/qualification/runtime-provider-6744fc28c6c7-2026-10-19.json'
    runtime = json.loads(runtime_record.read_text())
    (OUT / 'backup').mkdir()
    save()
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
