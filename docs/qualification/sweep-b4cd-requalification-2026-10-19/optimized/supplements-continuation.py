"""Fresh original-source and native-running UI supplements after the main gate.

Historical files select unchanged fixtures and checker code, never proof results.
All geometric construction, admission and exact proof producers remain Rust.
"""
from pathlib import Path
import ast
import difflib
import hashlib
import json
import os
import re
import subprocess
import time

OUT = Path(__file__).resolve().parent
SNAPSHOT = OUT / 'snapshot.json'
snapshot = json.loads(SNAPSHOT.read_text())
ROOT = Path(snapshot.get('workspace', str(OUT.parents[2])))
OLD = ROOT / 'docs/qualification/sweep-current-delivery-2026-10-15/cap-projection-current-main/smoothness-gap/remaining-sweep-timeouts-2026-10-16'
STAGE = Path(snapshot['stage'])
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
ORACLE = '/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
WASM = snapshot['wasmSha256']
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
DEST = OUT / 'supplements'
assert not DEST.exists(), 'Preserve every attempt'
DEST.mkdir()
receipt_path = DEST / 'receipt.json'
record = {'state': 'waiting-primary', 'passed': False, 'geometryWasmSha256': WASM,
          'driverSha256': sha(__file__), 'snapshotSha256': sha(SNAPSHOT), 'phases': [],
          'inputsSha256': {}, 'scope': 'Fresh exact-source STEP37, basis and proof-composition; original Rush UI8, Surface native UI110 and companion27 source contracts.'}
env = {**os.environ, 'PATH': str(Path(NODE).parent) + os.pathsep + os.environ['PATH'],
       'NODE_OPTIONS': '--max-old-space-size=8192', 'PYTHONDONTWRITEBYTECODE': '1'}

def save():
    p = receipt_path.with_suffix('.writing')
    p.write_text(json.dumps(record, indent=2) + '\n')
    p.replace(receipt_path)

def bind(path):
    record['inputsSha256'][str(path)] = sha(path)
    return path

def read_bound(path):
    bind(path)
    return json.loads(path.read_text())

def identity():
    assert sha(__file__) == record['driverSha256']
    assert sha(SNAPSHOT) == record['snapshotSha256']
    for name, expected in record['inputsSha256'].items():
        assert sha(name) == expected, name
    for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm', 'dist/wasm/geometry-kernel.wasm']:
        assert sha(STAGE / name) == WASM

def run(name, argv):
    identity()
    phase = {'name': name, 'argv': list(map(str, argv)), 'state': 'running', 'startedAt': time.time()}
    record['phases'].append(phase)
    save()
    print(name + ': started', flush=True)
    start = time.monotonic()
    with (DEST / (name + '.log')).open('w') as log:
        p = subprocess.Popen(phase['argv'], cwd=STAGE, env=env, stdout=log, stderr=subprocess.STDOUT)
        phase['pid'] = p.pid
        save()
        code = p.wait()
    identity()
    phase.update(state='passed' if code == 0 else 'failed', returncode=code,
                 seconds=time.monotonic() - start, logSha256=sha(DEST / (name + '.log')))
    save()
    print(name + ': ' + phase['state'], flush=True)
    assert code == 0, name

def adapted(source, name, replacements=()):
    original = bind(source).read_text()
    text = original
    for before, after in replacements:
        assert text.count(before) == 1, before
        text = text.replace(before, after)
    def path(match):
        target = (source.parent / match.group(1)).resolve()
        if not target.is_file():
            candidates = [Path(str(target) + suffix) for suffix in ['.ts', '.mts', '.mjs']]
            target = next(p for p in candidates if p.is_file())
        bind(target)
        return "from '" + target.as_uri() + "'"
    text = re.sub(r"from '(\.[^']+)'", path, text)
    p = DEST / name
    p.write_text(text)
    (DEST / (name + '.patch')).write_text(''.join(difflib.unified_diff(original.splitlines(True), text.splitlines(True), fromfile=str(source), tofile=str(p))))
    bind(p)
    return p

try:
    save()
    while True:
        primary = json.loads((OUT / 'continuation.json').read_text())
        if primary['state'] == 'failed':
            raise RuntimeError('Primary gate failed; do not launch supplements')
        if primary['state'] == 'complete':
            assert primary['passed'] and primary['geometryWasmSha256'] == WASM
            break
        os.kill(primary['pid'], 0)
        time.sleep(10)
    bind(OUT / 'continuation.json')
    canonical = OUT / 'canonical207-ui478'
    ui = canonical / 'ui478'
    complete = read_bound(ui / 'full-reconciliation.json')
    assert complete['passed'] and complete['cases'] == 478 and complete['nativeObservations'] == 1472
    assert complete['geometryWasmSha256'] == WASM
    record['state'] = 'running'
    checker = bind(OLD / 'check_companion27_contracts.py')
    run('companion27', ['python3', checker, canonical, STAGE, WASM, DEST / 'companion27.json'])
    selection = read_bound(OLD / 'v11-declared-mode-coverage-20261018.json')
    rows = [r for r in selection['rows'] if r['positiveSolid'] and not r['nativeStepWitnesses']]
    assert len(rows) == 37
    inputs = {}
    for row in rows:
        source = STAGE / 'examples/rush' / row['file']
        assert sha(source) == row['sourceSha256']
        row['sourcePath'] = str(source)
        inputs[str(bind(source))] = sha(source)
        for group in row['groups']:
            matrix_path = ui / group / 'matrix.json'
            matrix = read_bound(matrix_path)
            assert matrix['passed'] and matrix['geometryWasmSha256'] == WASM
            current = [c for c in matrix['cases'] if c['file'] == row['file']]
            assert sorted(c['viewport']['width'] for c in current) == [600, 1440]
            assert all(c['status'] == 'passed' and c['sourceSha256'] == row['sourceSha256'] and c['solid']['faces'] > 0 for c in current)
            inputs[str(matrix_path)] = sha(matrix_path)
    inventory = DEST / 'source37-inventory.json'
    inventory.write_text(json.dumps({'geometryWasmSha256': WASM, 'rows': rows, 'inputsSha256': inputs,
        'scope': 'Historical source selection only; positive Solid models and proof identity are fresh b4cd UI results.'}, indent=2) + '\n')
    bind(inventory)
    exporter = adapted(STAGE / 'scripts/export-sweep-source-coverage-step.mts', 'export-source37.mts', [
        ('construction:data.sweepBodyBoundaryEvidence??data.sweepStationReconstruction??null,',
         'construction:data.sweepBodyBoundaryEvidence??data.sweepStationReconstruction??{},')])
    source_step = DEST / 'source-step37'
    run('source37-export', [NODE, '--import', 'tsx', '--loader', STAGE / 'qualification-raw-imports.mjs', exporter, inventory, source_step])
    run('source37-reference', [ORACLE, 'scripts/reference-rational-boundary-volume.py', source_step])
    run('source37-occt', [ORACLE, 'scripts/verify-sweep-step-occt.py', source_step])
    manifest_path = source_step / 'manifest.json'
    manifest = read_bound(manifest_path)
    external = read_bound(source_step / 'opencascade-sweep.json')
    assert external['passed'] and len(external['cases']) == len(manifest['cases']) == 37
    gate_path = DEST / 'source-step37.json'
    gate_path.write_text(json.dumps({'state': 'complete', 'passed': True, 'cases': 37,
        'geometryWasmSha256': WASM, 'manifest': str(manifest_path), 'manifestSha256': sha(manifest_path),
        'report': str(source_step / 'opencascade-sweep.json'), 'reportSha256': sha(source_step / 'opencascade-sweep.json')}, indent=2) + '\n')
    bind(gate_path)
    run('source37-retained-seams', [NODE, '--import', 'tsx', 'scripts/capture-sweep-retained-proof-gaps.mts', gate_path, DEST / 'retained-proof-gaps9.json'])
    helper = bind(STAGE / 'scripts/qualify-retained-surface-interiors.py')
    text = helper.read_text()
    function = next(n for n in ast.parse(text).body if isinstance(n, ast.FunctionDef) and n.name == 'bounded_batches')
    namespace = {'json': json}
    exec(compile(ast.get_source_segment(text, function), str(helper), 'exec'), namespace)
    requests = []
    for c in manifest['cases']:
        owners = [face for face in range(c['faces']) if face not in c['capFaces']]
        assert len(owners) == len(c['wallSurfaces'])
        requests.append({'file': c['file'], 'surfaces': [{'face': face, 'surface': s} for face, s in zip(owners, c['wallSurfaces'])]})
    producer = bind(STAGE / 'crates/target/release/examples/certify_retained_surface_interiors')
    producer_source = bind(STAGE / 'crates/brep-core/examples/certify_retained_surface_interiors.rs')
    interior = DEST / 'source-interiors37'
    interior.mkdir()
    proofs = []
    for index, (group, encoded) in enumerate(namespace['bounded_batches']({'maxInputItemsPerCase': 1000000,
            'manifestSha256': sha(manifest_path), 'geometryWasmSha256': WASM}, requests, 31 * 1024 * 1024)):
        request = interior / f'request-{index}.json'
        output = interior / f'native-{index}.json'
        request.write_bytes(encoded)
        bind(request)
        run(f'source37-interior-{index}', [producer, request, output])
        result = read_bound(output)
        assert result['passed'] and result['requestSha256'] == sha(request) and result['producerSha256'] == sha(producer_source)
        assert [c['file'] for c in result['cases']] == [c['file'] for c in group]
        for expected, proof in zip(group, result['cases']):
            assert proof['complete'] and proof['reason'] is None and proof['unvisitedFaces'] == 0
            assert 0 < proof['work'] <= proof['maxInputItems'] == 1000000
            assert [f['face'] for f in proof['faces']] == [f['face'] for f in expected['surfaces']]
            assert all(f['parametricC0Certified'] and f['parametricC1Certified'] and f['parametricC2Certified'] and not f['geometricG1G2Certified'] for f in proof['faces'])
            proofs.append(proof)
    assert len(proofs) == 37
    (interior / 'result.json').write_text(json.dumps({'passed': True, 'geometryWasmSha256': WASM,
        'manifestSha256': sha(manifest_path), 'cases': proofs, 'surfaces': sum(len(c['faces']) for c in proofs)}, indent=2) + '\n')
    composition = adapted(STAGE / 'scripts/verify-sweep-source-composition.mts', 'source-composition.mts', [
        ('v11-original-source-step37-occt-20261018.json', 'source-step37.json'),
        ('v11-original-source-interiors37-20261018/result.json', 'source-interiors37/result.json'),
        ('v11-retained-proof-gaps9-20261018.json', 'retained-proof-gaps9.json')])
    run('source37-proof-composition', [NODE, '--import', 'tsx', composition, DEST, DEST / 'source-composition37.json'])
    alias = 'miter-affine-hollow.r'
    rush_reports = []
    for step, original in {
        'rush-unclamped-hollow.step': 'miter-unclamped-hollow.r',
        'rush-periodic-hollow.step': 'miter-periodic-hollow.r',
        'rush-periodic-frame-guide-affine-hollow.step': 'miter-periodic-frame-guide-affine-hollow.r',
        'unsegmented-frame-guide-affine-hollow-miter.step': 'miter-unsegmented-frame-guide-affine-hollow.r',
    }.items():
        fixture = DEST / 'rush-sources' / Path(step).stem
        fixture.mkdir(parents=True)
        source = bind(STAGE / 'examples/rush' / original)
        (fixture / alias).write_bytes(source.read_bytes())
        bind(fixture / alias)
        destination = DEST / 'rush-ui8' / Path(step).stem
        run('rush-ui-' + Path(step).stem, [NODE, 'scripts/check-sweep-miter-matrix-browser.mjs', destination,
            '--mode=' + alias, '--source-directory=' + str(fixture), '--native-running-cancel', '--gpu-backend=metal'])
        report = read_bound(destination / 'matrix.json')
        assert report['passed'] and not report['headed'] and report['geometryWasmSha256'] == WASM
        assert report['distIndexSha256'] == sha(STAGE / 'dist/index.html') and len(report['cases']) == 2
        assert all(c['status'] == 'passed' and c['sourceSha256'] == sha(source) and not c['pageErrors'] for c in report['cases'])
        rush_reports.append(report)
    old_harness = bind(OLD / 'check-v11-surface-native-build-matrix-20261018.mjs')
    baseline_runner = bind(OLD / 'surface-native-original-browser-checker-20261018.mjs')
    assert sha(baseline_runner) == sha(STAGE / 'scripts/check-sweep-miter-matrix-browser.mjs')
    old_stage = '/private/tmp/osv-sweep-common-source-v11-20261018'
    text = old_harness.read_text()
    assert text.count(old_stage) == 8
    harness = DEST / 'surface-native.mjs'
    harness.write_text(text.replace(old_stage, str(STAGE)))
    bind(harness)
    run('surface-native-ui110', [NODE, harness, DEST / 'surface-native-ui110', '--surface',
        '--surface-native-build-only', '--native-running-cancel', '--gpu-backend=metal'])
    surface = read_bound(DEST / 'surface-native-ui110/matrix.json')
    assert surface['passed'] and not surface['headed'] and surface['geometryWasmSha256'] == WASM
    assert len(surface['cases']) == 110
    assert surface['distIndexSha256'] == sha(STAGE / 'dist/index.html')
    observations = 0
    for report, labels in [(r, ['native-build-cancellation', 'native-build-source-change', 'native-solid-cancel', 'native-solid-source-change']) for r in rush_reports] + [(surface, ['native-build-cancellation', 'native-build-source-change'])]:
        for case in report['cases']:
            assert case['status'] == 'passed' and not case['pageErrors']
            for label in labels:
                matches = [a for a in case['assertions'] if a['case'] == label]
                assert len(matches) == 1 and matches[0]['status'] == 'passed'
                native = matches[0]['nativeStack']
                assert native['geometryWasmSha256'] == WASM and native['nativeSampleCount'] >= 5
                assert native['debuggerPaused'] is False and 0 <= native['lastSampleToProfileEndMs'] <= 5
                observations += 1
    assert observations == 252
    identity()
    record.update(state='complete', passed=True, extraUiExecutions=118, actualNativeObservations=252,
                  extraStepSources=37, sourceInteriorSurfaces=sum(len(c['faces']) for c in proofs))
    save()
except BaseException as error:
    record.update(state='failed', passed=False, error=str(error))
    save()
    raise
