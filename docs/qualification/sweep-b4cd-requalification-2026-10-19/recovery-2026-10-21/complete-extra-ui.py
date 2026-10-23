"""Compose only actual successful cases from the unchanged Surface probe."""
from pathlib import Path
import hashlib, json

REC = Path(__file__).resolve().parent
O = REC.parent / 'optimized'
D = O / 'supplements-recovered-20261021'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
target = D / 'receipt-complete-20261021.json'
assert not target.exists()
parent_path = D / 'receipt-resumed-20261021.json'
parent = json.loads(parent_path.read_text())
assert parent['state'] == 'failed' and parent['error'] == 'surface-native-ui110'
assert all(p['state'] == 'passed' for p in parent['phases'][:-1])
old_path = D / 'surface-native-ui110/matrix.json'
tail_path = D / 'surface-native-tail75/matrix.json'
old, tail = [json.loads(p.read_text()) for p in [old_path, tail_path]]
assert tail['passed'] and not tail['headed']
assert json.loads((D / 'surface-native-tail75-exit.json').read_text())['returncode'] == 0
assert old['sourceHashes'] == tail['sourceHashes']
assert old['geometryWasmSha256'] == tail['geometryWasmSha256'] == parent['geometryWasmSha256']
assert old['distIndexSha256'] == tail['distIndexSha256']
prefix = [c for c in old['cases'] if c['status'] == 'passed']
assert len(prefix) == 35 and len(tail['cases']) == 75
cases = prefix + tail['cases']
keys = [str(c['viewport']['width']) + ':' + c['file'] for c in cases]
assert keys == old['selection']['fullInventory']
assert tail['selection']['plannedCaseKeys'] == keys[35:]
combined = {**tail, 'cases': cases,
    'selection': {**tail['selection'], 'startAt': None, 'plannedCaseKeys': keys},
    'continuationInputs': [{'path': str(p), 'sha256': sha(p), 'successfulCases': n}
                          for p, n in [(old_path, 35), (tail_path, 75)]]}
surface_path = D / 'surface-native-complete.json'
assert not surface_path.exists()
observations = 0
reports = [(combined, ['native-build-cancellation', 'native-build-source-change'])]
for p in sorted((D / 'rush-ui8').glob('*/matrix.json')):
    report = json.loads(p.read_text()); assert report['passed'] and len(report['cases']) == 2
    reports.append((report, ['native-build-cancellation', 'native-build-source-change',
                            'native-solid-cancel', 'native-solid-source-change']))
assert len(reports) == 5
for report, labels in reports:
    assert report['geometryWasmSha256'] == parent['geometryWasmSha256']
    for case in report['cases']:
        assert case['status'] == 'passed' and not case['pageErrors']
        for label in labels:
            matches = [a for a in case['assertions'] if a['case'] == label]
            assert len(matches) == 1 and matches[0]['status'] == 'passed'
            native = matches[0]['nativeStack']
            assert native['geometryWasmSha256'] == parent['geometryWasmSha256']
            assert native['nativeSampleCount'] >= 5 and native['debuggerPaused'] is False
            assert 0 <= native['lastSampleToProfileEndMs'] <= 5
            observations += 1
assert observations == 252
surface_path.write_text(json.dumps(combined, indent=2) + '\n')
interiors = json.loads((D / 'source-interiors37/result.json').read_text())
result = {**parent, 'state': 'complete', 'passed': True, 'error': None,
    'phases': parent['phases'][:-1] + [{'name': 'surface-native-ui110-composed',
        'state': 'passed', 'returncode': 0, 'reportSha256': sha(surface_path)}],
    'previousFailedReceiptSha256': sha(parent_path),
    'extraUiExecutions': 118, 'actualNativeObservations': observations,
    'extraStepSources': 37, 'sourceInteriorSurfaces': interiors['surfaces'],
    'surfaceComposite': str(surface_path), 'surfaceCompositeSha256': sha(surface_path),
    'compositionDriverSha256': sha(__file__),
    'scope': 'Original unchanged requests and limits. Unqualified observer attempt remains failed; only complete successful cases from authenticated executions are composed.'}
target.write_text(json.dumps(result, indent=2) + '\n')
print('118 extra UI executions and 252 authenticated native observations verified')
