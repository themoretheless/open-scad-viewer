"""Compose completed exact-artifact evidence; never promote a running attempt."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import json

OUT = Path(__file__).resolve().parents[1]
OPT = OUT / 'optimized'
SNAPSHOT = json.loads((OPT / 'snapshot-recovered-20261021.json').read_text())
STAGE = Path(SNAPSHOT['stage'])
WASM = SNAPSHOT['wasmSha256']
TARGET = OUT / 'final-qualification.json'
assert not TARGET.exists(), 'Preserve earlier composed evidence'


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


evidence = {}


def read(relative, *, passed=False, complete=False):
    path = OUT / relative
    data = json.loads(path.read_text())
    if passed:
        assert data['passed'] is True, relative
    if complete:
        assert data['state'] == 'complete', relative
    evidence[relative] = {'path': str(path), 'sha256': sha(path)}
    return data


def tests(relative, count):
    data = read(relative)
    assert data['success'] and data['numTotalTests'] == count
    assert data['numPassedTests'] == count and data['numFailedTests'] == 0
    assert data['numPendingTests'] == 0 and data['numTodoTests'] == 0
    return data


assert SNAPSHOT['originalTestAndHookTimeoutMs'] == 30000
assert SNAPSHOT['maxWorkers'] == 1
tests('optimized/public100.json', 736)
tests('optimized/original-two-files.json', 36)
tests('optimized/provider303.json', 303)
paired = read('paired-interval/receipt.json', passed=True, complete=True)
assert paired['allPairsFaster'] and paired['limitsChanged'] is False
primary = read('optimized/finish-recovered-r2-20261021.json', passed=True, complete=True)
supplement = read('optimized/supplements-recovered-20261021/receipt-complete-20261021.json', passed=True, complete=True)
adjacent = read('optimized/adjacent-preservation-resumed.json', passed=True, complete=True)
for receipt in [primary, supplement]:
    assert receipt['geometryWasmSha256'] == WASM
    assert all(p['state'] == 'passed' and p['returncode'] == 0 for p in receipt['phases'])
canonical = read('optimized/canonical-complete/receipt.json', passed=True, complete=True)
assert canonical['geometryWasmSha256'] == WASM
main_ui = read('optimized/canonical-complete/ui478/full-reconciliation.json', passed=True)
assert main_ui['geometryWasmSha256'] == WASM
assert main_ui['cases'] == 478 and main_ui['nativeObservations'] == 1472
for relative, count in [
    ('optimized/canonical-complete/step207/opencascade-sweep.json', 207),
    ('optimized/canonical-complete/annular-station/opencascade-sweep.json', 1),
    ('optimized/c2-step4/opencascade-sweep.json', 4),
    ('optimized/supplements-recovered-20261021/source-step37/opencascade-sweep.json', 37),
]:
    oracle = read(relative, passed=True)
    assert len(oracle['cases']) == count
    assert oracle['artifactProvenance']['geometryWasmSha256'] == WASM
boundary = read('optimized/canonical-complete/step207/complete-boundary.json', passed=True)
global_proof = read('optimized/canonical-complete/step207/global.json', passed=True)
assert (boundary['certified'], boundary['unproved']) == (206, 1)
assert (global_proof['certified'], global_proof['refused']) == (205, 2)
for relative in [
    'optimized/canonical-complete/step207/current-inventory.json',
    'optimized/canonical-complete/step207/face-pair-cover.json',
    'optimized/canonical-complete/step207/smoothness-refusal-controls.json',
    'optimized/legacy-cap-refusals/receipt.json',
    'optimized/retained-interiors/result.json',
    'optimized/supplements-recovered-20261021/companion27.json',
    'optimized/supplements-recovered-20261021/source-interiors37/result.json',
    'optimized/supplements-recovered-20261021/source-composition37.json',
    'optimized/multispan-resumed/constructed.json',
    'optimized/multispan-resumed/packed-jets.json',
    'optimized/multispan-resumed/closed-jets.json',
]:
    read(relative, passed=True)
read('optimized/canonical-complete/step207/finite-modes.json')
assert supplement['extraUiExecutions'] == 118
assert supplement['actualNativeObservations'] == 252
c2_ui = read('optimized/c2-ui8/matrix.json', passed=True)
assert len(c2_ui['cases']) == 8 and not c2_ui['headed']
assert c2_ui['geometryWasmSha256'] == WASM
c2_observations = [a['nativeStack'] for c in c2_ui['cases'] for a in c['assertions']
                   if a['case'] in ['native-build-cancellation', 'native-build-source-change',
                                    'native-solid-cancel', 'native-solid-source-change']]
assert len(c2_observations) == 32
assert all(n['geometryWasmSha256'] == WASM and n['nativeSampleCount'] >= 5
           and n['debuggerPaused'] is False and 0 <= n['lastSampleToProfileEndMs'] <= 5
           for n in c2_observations)
ui_root = OPT / 'canonical-complete/ui478'
ui_jobs = json.loads((ui_root / 'fresh-run.json').read_text())['jobs']
ui_matrices = [ui_root / job['name'] / 'matrix.json' for job in ui_jobs]
ui_matrices += [OPT / 'c2-ui8/matrix.json']
ui_matrices += sorted((OPT / 'supplements-recovered-20261021/rush-ui8').glob('*/matrix.json'))
ui_matrices += [OPT / 'supplements-recovered-20261021/surface-native-complete.json']
ui_cases = []
for matrix_path in ui_matrices:
    matrix = read(str(matrix_path.relative_to(OUT)), passed=True)
    assert not matrix['headed'] and matrix['geometryWasmSha256'] == WASM
    assert matrix['distIndexSha256'] == sha(STAGE / 'dist/index.html')
    assert all(case['status'] == 'passed' and not case['pageErrors']
               for case in matrix['cases'])
    ui_cases.extend(matrix['cases'])
assert len(ui_cases) == 604
unique_ui_sources = {(case['sourceSha256'], case['viewport']['width']) for case in ui_cases}
for relative in ['public/wasm/geometry-kernel.wasm',
                 'src/generated/geometry-kernels/kernel_bg.wasm',
                 'dist/wasm/geometry-kernel.wasm']:
    assert sha(STAGE / relative) == WASM
ledger = json.loads((STAGE / 'qualification-inputs.json').read_text())
for relative, expected in ledger.items():
    assert sha(STAGE / relative) == expected, relative
for relative in ['optimized/snapshot.json', 'optimized/continuation-preparation.json',
                 'native-patch-applied.json']:
    read(relative)
assert sha(OPT / 'baseline-v4.json') == sha(OPT / 'preserved-v4.json')
archive = read('recovery-2026-10-21/archive-receipt.json', passed=True)
assert archive['inputLedgerSha256'] == sha(STAGE / 'qualification-inputs.json')
read('recovery-2026-10-21/recovered-build.json', passed=True)
read('optimized/snapshot-recovered-20261021.json')

record = {
    'schema': 'sweep-exact-artifact-requalification/1',
    'state': 'complete', 'passed': True,
    'qualifiedAtUtc': datetime.now(timezone.utc).isoformat(),
    'geometryWasmSha256': WASM,
    'previousGeometryWasmSha256': SNAPSHOT['originalBaselineWasm'],
    'sourceStage': str(STAGE),
    'sourceLedgerSha256': sha(STAGE / 'qualification-inputs.json'),
    'driverSha256': sha(__file__),
    'preservedSourceArchive': {'path': str(OUT / 'recovery-2026-10-21/qualified-inputs.tar.gz'),
                               'sha256': archive['archiveSha256']},
    'publicRegression': {'passed': 736, 'total': 736, 'singleRun': True,
                         'testAndHookTimeoutMs': 30000, 'maxWorkers': 1,
                         'limitsChanged': False},
    'providerRegression': {'passed': 303, 'total': 303},
    'pairedMedianImprovementPercent': paired['medianImprovementPercent'],
    'stepOracleExecutions': 249,
    'uiExecutions': 604,
    'uniqueUiSourceViewportCases': len(unique_ui_sources),
    'actualNativeObservations': 1756,
    'canonicalCompleteBoundary': {'certified': 206, 'unproved': 1},
    'canonicalGlobal': {'certified': 205, 'expectedRefusals': 2},
    'nativeGeometryOwnership': 'Rust; JavaScript and Python transport, QA and independent reference only',
    'scope': 'Fresh finite same-artifact qualification. Fixture success is not an all-input geometry theorem. Original undecided caps remain refused; sharp miter remains C0 where applicable. Earlier failed environment/setup attempts retain their failed receipts.',
    'deliveryState': 'Qualified frozen build; consult separate guarded delivery receipt for main-workspace installation',
    'evidence': evidence,
}
TARGET.write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'passed': True, 'path': str(TARGET), 'sha256': sha(TARGET)}))
