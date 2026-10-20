"""Authenticate real Surface native cancellation for all baseline110 UI cases.

Runs after the independent parent, companion27 and original Rush UI8 chain.
The frozen product/QA sources and all geometry/work/time limits stay unchanged.
Only the additive external Surface build-probe runner is selected.
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
BASE = OUT / 'frozen-v11r1-common-source-20261018-independent-step-ui-final/canonical207-ui478'
EXPECTED = '68dd0ed6d368a7f4e5beb8e1a65c26720b8e6316c54a4f953e1ee49161ca4bd1'
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
FOLDER = OUT / 'v11-surface-native-followup-20261018'
RECEIPT = OUT / 'v11-surface-native-followup-20261018.json'


def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def save(value):
    temporary = RECEIPT.with_suffix('.writing')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(RECEIPT)


def main():
    assert not RECEIPT.exists() and not FOLDER.exists(), 'Preserve every earlier run'
    assert os.getpriority(os.PRIO_PROCESS, 0) == 0, 'Runtime UI requires normal priority'
    FOLDER.mkdir()
    harness_path = OUT / 'v11-surface-native-harness-20261018.json'
    harness = read(harness_path)
    runner = Path(harness['runner'])
    assert sha(runner) == harness['runnerSha256']
    plan_path = BASE / 'ui478/inventory-plan.json'
    plan = read(plan_path)
    assert plan['geometryWasmSha256'] == EXPECTED and plan['totalCases'] == 478
    job = next(job for job in plan['jobs'] if job['name'] == 'surface')
    assert len(job['sources']) == 55 and len(job['caseKeys']) == 110
    ledger_path = STAGE / 'runtime-source-sha256.json'
    ledger = read(ledger_path)
    extra_path = STAGE / 'qualification-external-inputs.json'
    extra = read(extra_path)
    delivery_plan = OUT / 'v11-delivery-plan-20261018.json'
    distribution = next(entry['after']['files'] for entry in read(delivery_plan)['targets'] if entry['path'] == 'dist')
    input_paths = [Path(__file__), harness_path, runner, plan_path, ledger_path, extra_path, delivery_plan,
                   OUT / 'prepare-v11-surface-native-harness-20261018.py',
                   OUT / 'surface-native-original-browser-checker-20261018.mjs',
                   OUT / 'surface-native-build-only-harness-20261018.patch',
                   OUT / 'v11r1-common-source-runtime-20261018.json']
    for source in job['sources']:
        path = Path(job['sourceDirectory']) / source['file']
        assert sha(path) == source['sha256']
        input_paths.append(path)
    r = {'schema': 'sweep-v11-surface-native-followup/1', 'state': 'waiting-original-rush-ui8',
         'passed': False, 'pid': os.getpid(), 'geometryWasmSha256': EXPECTED,
         'inputsSha256': {str(path): sha(path) for path in input_paths}, 'fullGoalComplete': False,
         'scope': 'All55 original Surface sources at both widths; authenticated native build cancellation and source replacement, final-certificate clearing and restoration. Native Solid is inapplicable; existing Solid refusal and held-dispatch tests remain additional controls.'}
    save(r)
    try:
        prerequisite_path = OUT / 'v11-native-rush-ui8-followup-20261018.json'
        while True:
            prerequisite = read(prerequisite_path)
            if prerequisite['state'] == 'failed':
                raise RuntimeError('Upstream original Rush UI8 failed; preserve this run without launching')
            if prerequisite['state'] == 'complete':
                assert prerequisite['passed'] is True and prerequisite['cases'] == 8
                assert prerequisite['nativeObservations'] == 32 and prerequisite['geometryWasmSha256'] == EXPECTED
                break
            os.kill(prerequisite['pid'], 0)
            time.sleep(5)
        r['prerequisiteReceipt'] = str(prerequisite_path)
        r['prerequisiteSha256'] = sha(prerequisite_path)
        baseline_path = BASE / 'ui478/full-reconciliation.json'
        baseline = read(baseline_path)
        assert baseline['passed'] is True and baseline['cases'] == 478
        assert baseline['nativeObservations'] == 1472 and baseline['geometryWasmSha256'] == EXPECTED
        r['baselineUiReceiptSha256'] = sha(baseline_path)

        def identity():
            for name, digest in r['inputsSha256'].items():
                assert sha(name) == digest, name
            assert sha(prerequisite_path) == r['prerequisiteSha256']
            assert sha(baseline_path) == r['baselineUiReceiptSha256']
            for name, digest in ledger.items():
                assert sha(STAGE / name) == digest, name
            for name, metadata in extra['files'].items():
                assert sha(STAGE / name) == metadata['sha256'], name
            for name, digest in extra['browserDependencyFiles'].items():
                assert sha(Path(extra['browserDependenciesRealPath']) / name) == digest, name
            for name, metadata in distribution.items():
                assert sha(STAGE / 'dist' / name) == metadata['sha256'], name
            for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm', 'dist/wasm/geometry-kernel.wasm']:
                assert sha(STAGE / name) == EXPECTED

        identity()
        stable_since = None
        while stable_since is None or time.monotonic() - stable_since < 30:
            stable_since = (stable_since or time.monotonic()) if os.getloadavg()[0] <= 8 else None
            time.sleep(2)
        destination = FOLDER / 'ui110'
        argv = [NODE, str(runner), str(destination), '--surface', '--surface-native-build-only',
                '--native-running-cancel', '--gpu-backend=metal']
        r.update(state='running', argv=argv)
        save(r)
        env = {**os.environ, 'PATH': str(Path(NODE).parent) + os.pathsep + os.environ['PATH'],
               'NODE_OPTIONS': '--max-old-space-size=8192', 'PYTHONDONTWRITEBYTECODE': '1'}
        with (FOLDER / 'ui110.log').open('w') as log:
            result = subprocess.run(argv, cwd=STAGE, env=env, stdout=log, stderr=subprocess.STDOUT)
        assert result.returncode == 0, 'Surface native UI run failed; inspect preserved ui110 report/log'
        identity()
        matrix_path = destination / 'matrix.json'
        matrix = read(matrix_path)
        assert matrix['passed'] is True and matrix['headed'] is False and matrix['evidenceOnly'] is False
        assert matrix['gpuBackendRequested'] == 'metal' and matrix['geometryWasmSha256'] == EXPECTED
        assert matrix['nativeRunningCancelProbe'] is True and matrix['distIndexSha256'] == plan['distIndexSha256']
        assert matrix['supplementalHarness']['runnerSha256'] == harness['runnerSha256']
        assert matrix['supplementalHarness']['originalRunnerSha256'] == harness['originalRunnerSha256']
        assert matrix['selection']['fullInventory'] == job['caseKeys']
        assert matrix['selection']['plannedCaseKeys'] == job['caseKeys']
        assert [f"{case['viewport']['width']}:{case['file']}" for case in matrix['cases']] == job['caseKeys']
        for name, digest in matrix['sourceHashes'].items():
            assert sha(STAGE / name) == digest
        source_hashes = {source['file']: source['sha256'] for source in job['sources']}
        observations = []
        for case in matrix['cases']:
            assert case['status'] == 'passed' and case['pageErrors'] == []
            assert case['sourceSha256'] == source_hashes[case['file']]
            assert case['gpuAdapter']['vendor'] == 'apple' and case['gpuAdapter']['architecture'].startswith('metal')
            assert not case.get('solid') and all(a['status'] == 'passed' for a in case['assertions'])
            for name in ['build', 'viewport-renderer-ready', 'native-build-cancel-restoration',
                         'native-build-source-change-restoration', 'build-cancellation', 'solid-cancellation',
                         'source-change', 'invalid-path-build-refusal', 'invalid-path-solid-refusal',
                         'surface-solid-refusal', 'refused-empty-project-restoration']:
                assert len([a for a in case['assertions'] if a['case'] == name]) == 1
            assert not any(a['case'] in ('native-solid-cancel', 'native-solid-source-change') for a in case['assertions'])
            original = (Path(job['sourceDirectory']) / case['file']).read_text()
            busy_sha = hashlib.sha256((original + '\n// Observe original native surface work in a new revision.\n').encode()).hexdigest()
            for name in ['native-build-cancellation', 'native-build-source-change']:
                records = [a for a in case['assertions'] if a['case'] == name]
                assert len(records) == 1
                observation = records[0]
                assert observation['sourceSha256'] == busy_sha
                sample = observation['nativeStack']
                assert sample['geometryWasmSha256'] == EXPECTED and sample['nativeSampleCount'] >= 5
                assert sample['debuggerPaused'] is False and 0 <= sample['lastSampleToProfileEndMs'] <= 5
                assert math.isfinite(observation['workerDestroyedAfterMs']) and observation['workerDestroyedAfterMs'] >= 0
                observations.append({'file': case['file'], 'width': case['viewport']['width'], 'case': name,
                                     'sourceSha256': busy_sha, 'samples': sample['nativeSampleCount'],
                                     'tailMs': sample['lastSampleToProfileEndMs'],
                                     'workerDestroyedAfterMs': observation['workerDestroyedAfterMs']})
        assert len(observations) == 220
        r.update(state='complete', passed=True, cases=110, nativeObservations=220,
                 report=str(matrix_path), reportSha256=sha(matrix_path), observations=observations)
        save(r)
        print('All110Surface UI cases passed with220real-native build/source-change observations.', flush=True)
    except BaseException as error:
        r.update(state='failed', passed=False, reason=repr(error))
        save(r)
        raise


if __name__ == '__main__':
    main()
