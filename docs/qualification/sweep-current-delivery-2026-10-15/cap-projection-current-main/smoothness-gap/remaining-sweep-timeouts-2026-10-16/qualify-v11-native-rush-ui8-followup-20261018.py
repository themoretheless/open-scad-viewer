"""Run four omitted original Rush STEP sources at both UI widths, serially.

Uses the unchanged browser checker and byte-identical source fixtures. Its
generic budgeted-miter file label is recorded explicitly; source text and
geometry, worker deadlines, observer criteria and proof budgets are unchanged.
Runs only after the complete current independent parent and companion27 gate.
"""
from pathlib import Path
import hashlib
import json
import math
import os
import subprocess
import time

OUT = Path(__file__).resolve().parent
STAGE = Path('/private/tmp/osv-sweep-common-source-v11-20261018')
BASE = OUT / 'frozen-v11r1-common-source-20261018-independent-step-ui-final'
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
EXPECTED = '68dd0ed6d368a7f4e5beb8e1a65c26720b8e6316c54a4f953e1ee49161ca4bd1'
RECEIPT = OUT / 'v11-native-rush-ui8-followup-20261018.json'
FOLDER = OUT / 'v11-native-rush-ui8-followup-20261018'
MAPPING = {
    'rush-unclamped-hollow.step': 'miter-unclamped-hollow.r',
    'rush-periodic-hollow.step': 'miter-periodic-hollow.r',
    'rush-periodic-frame-guide-affine-hollow.step': 'miter-periodic-frame-guide-affine-hollow.r',
    'unsegmented-frame-guide-affine-hollow-miter.step': 'miter-unsegmented-frame-guide-affine-hollow.r',
}
ALIAS = 'miter-affine-hollow.r'


def sha(path):
    with path.open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_text())


def save(value):
    temporary = RECEIPT.with_suffix('.writing')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(RECEIPT)


def main():
    assert not RECEIPT.exists() and not FOLDER.exists(), 'Preserve every earlier run'
    assert os.getpriority(os.PRIO_PROCESS, 0) == 0, 'Runtime UI must retain normal priority'
    FOLDER.mkdir()
    coverage_path = OUT / 'v11-step-ui-coverage-20261018.json'
    coverage = read(coverage_path)
    gaps = {case['file']: case for case in coverage['authoredSourceUiGaps']}
    assert coverage['passed'] is True and coverage['geometryWasmSha256'] == EXPECTED
    assert set(gaps) == set(MAPPING)
    inputs = [Path(__file__), coverage_path, OUT / 'audit-v11-step-ui-coverage-20261018.py',
              STAGE / 'scripts/check-sweep-miter-matrix-browser.mjs',
              STAGE / 'scripts/sweep-native-cancel-probe.mjs',
              STAGE / 'scripts/worker-wasm-fingerprint.mjs',
              STAGE / 'runtime-source-sha256.json', OUT / 'v11r1-common-source-runtime-20261018.json']
    fixtures = []
    for step, name in MAPPING.items():
        source = STAGE / 'examples/rush' / name
        assert sha(source) == gaps[step]['sourceSha256']
        directory = FOLDER / 'sources' / Path(step).stem
        directory.mkdir(parents=True)
        fixture = directory / ALIAS
        fixture.write_bytes(source.read_bytes())
        assert sha(fixture) == sha(source)
        inputs.extend([source, fixture])
        fixtures.append({'step': step, 'originalSource': str(source), 'sourceSha256': sha(source),
                         'uiFileLabel': ALIAS, 'sourceDirectory': str(directory)})
    r = {'schema': 'sweep-v11-native-rush-ui8-followup/1', 'state': 'waiting-full-artifact-and-companion27',
         'passed': False, 'pid': os.getpid(), 'geometryWasmSha256': EXPECTED,
         'inputsSha256': {str(path): sha(path) for path in inputs}, 'fixtures': fixtures,
         'phases': [], 'fullGoalComplete': False,
         'scope': 'Four original Rush STEP inputs absent from baseline478 UI: exact source bytes with an explicit generic budgeted-miter file label; additional8cases/32real-native observations. No geometry/source changes or relaxation of the unchanged checker.'}
    save(r)
    try:
        prerequisite_paths = [BASE / 'receipt.json', OUT / 'v11-companion27-followup-20261018.json']
        while True:
            reports = [read(path) for path in prerequisite_paths]
            for report in reports:
                if report['state'] in ('failed', 'stopped-after-public-gate-failure'):
                    raise RuntimeError('A mandatory upstream gate failed; do not run against an unqualified candidate')
                if report['state'] != 'complete':
                    os.kill(report['pid'], 0)
            if all(report['state'] == 'complete' and report['passed'] is True for report in reports):
                break
            time.sleep(5)
        assert all(report['geometryWasmSha256'] == EXPECTED for report in reports)
        r['prerequisitesSha256'] = {str(path): sha(path) for path in prerequisite_paths}
        canonical = BASE / 'canonical207-ui478'
        ui = read(canonical / 'ui478/full-reconciliation.json')
        assert ui['passed'] is True and ui['cases'] == 478 and ui['nativeObservations'] == 1472
        assert ui['geometryWasmSha256'] == EXPECTED
        global_report = read(canonical / 'step207/global.json')
        assert global_report['manifestSha256'] == coverage['manifestSha256']
        assert sha(canonical / 'step207/manifest.json') == coverage['manifestSha256']
        global_cases = {case['file']: case for case in global_report['cases']}

        def identity():
            for name, expected in {**r['inputsSha256'], **r['prerequisitesSha256']}.items():
                assert sha(Path(name)) == expected, name
            for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm', 'dist/wasm/geometry-kernel.wasm']:
                assert sha(STAGE / name) == EXPECTED

        r['state'] = 'running'
        save(r)
        observations = 0
        for fixture in fixtures:
            identity()
            stable_since = None
            while stable_since is None or time.monotonic() - stable_since < 30:
                stable_since = (stable_since or time.monotonic()) if os.getloadavg()[0] <= 8 else None
                time.sleep(2)
            name = Path(fixture['step']).stem
            destination = FOLDER / 'ui' / name
            phase = {'name': name, 'state': 'running'}
            r['phases'].append(phase)
            save(r)
            argv = [NODE, 'scripts/check-sweep-miter-matrix-browser.mjs', str(destination),
                    '--mode=' + ALIAS, '--source-directory=' + fixture['sourceDirectory'],
                    '--native-running-cancel', '--gpu-backend=metal']
            with (FOLDER / (name + '.log')).open('w') as log:
                result = subprocess.run(argv, cwd=STAGE, stdout=log, stderr=subprocess.STDOUT)
            assert result.returncode == 0, f'UI fixture failed: {name}'
            identity()
            matrix_path = destination / 'matrix.json'
            matrix = read(matrix_path)
            assert matrix['passed'] is True and matrix['headed'] is False and matrix['evidenceOnly'] is False
            assert matrix['gpuBackendRequested'] == 'metal' and matrix['geometryWasmSha256'] == EXPECTED
            assert matrix['nativeRunningCancelProbe'] is True
            assert matrix['distIndexSha256'] == ui['distIndexSha256']
            assert matrix['selection']['fullInventory'] == [f'{width}:{ALIAS}' for width in (1440, 600)]
            assert matrix['selection']['plannedCaseKeys'] == matrix['selection']['fullInventory']
            assert len(matrix['cases']) == 2
            for case, width in zip(matrix['cases'], (1440, 600)):
                assert case['status'] == 'passed' and case['pageErrors'] == []
                assert case['viewport']['width'] == width and case['sourceSha256'] == fixture['sourceSha256']
                assert case['gpuAdapter']['vendor'] == 'apple' and case['gpuAdapter']['architecture'].startswith('metal')
                assert case['solid']['renderer'] == 'webgpu' and case['solid']['faces'] > 0 and case['solid']['shells'] > 0
                assert all(a['status'] == 'passed' for a in case['assertions'])
                for label in ['continuous-boundary-presentation', 'solid-publication', 'restore',
                              'build-cancellation', 'solid-cancellation', 'source-change',
                              'invalid-path-build-refusal', 'invalid-path-solid-refusal']:
                    assert len([a for a in case['assertions'] if a['case'] == label]) == 1
                for label in ['native-build-cancellation', 'native-build-source-change', 'native-solid-cancel', 'native-solid-source-change']:
                    matches = [a for a in case['assertions'] if a['case'] == label]
                    assert len(matches) == 1
                    sample = matches[0]['nativeStack']
                    assert sample['geometryWasmSha256'] == EXPECTED and sample['nativeSampleCount'] >= 5
                    assert sample['debuggerPaused'] is False and 0 <= sample['lastSampleToProfileEndMs'] <= 5
                    elapsed = matches[0]['workerDestroyedAfterMs']
                    assert math.isfinite(elapsed) and elapsed >= 0
                    observations += 1
            native = global_cases[fixture['step']]
            assert native['status'] == 'certified'
            assert sha(canonical / 'step207' / fixture['step']) == native['stepSha256']
            phase.update(state='passed', report=str(matrix_path), sha256=sha(matrix_path), cases=2,
                         sourceSha256=fixture['sourceSha256'], stepSha256=native['stepSha256'])
            save(r)
            print(name + ':2cases passed', flush=True)
        assert observations == 32
        identity()
        r.update(state='complete', passed=True, cases=8, nativeObservations=observations)
        save(r)
        print('Original4Rush STEP sources qualified at both widths with32actual native observations.', flush=True)
    except BaseException as error:
        r.update(state='failed', passed=False, reason=repr(error))
        save(r)
        raise


if __name__ == '__main__':
    main()
