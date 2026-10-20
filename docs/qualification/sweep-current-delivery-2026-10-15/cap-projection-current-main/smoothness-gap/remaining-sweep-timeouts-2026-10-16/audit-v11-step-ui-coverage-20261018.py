"""Join current STEP obligations to planned UI inputs without claiming UI success.

Uses the small source-bound native audit outputs. It neither rereads/rebuilds
geometry nor changes the immutable artifact that is executing the full UI run.
"""
from pathlib import Path
import hashlib
import itertools
import json
import subprocess

OUT = Path(__file__).resolve().parent
BASE = OUT / 'frozen-v11r1-common-source-20261018-independent-step-ui-final/canonical207-ui478'


def sha(path):
    with path.open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def source_metadata(path):
    # Read only top-level case metadata tokens, without materializing the
    # 905 MB native models. Refuse ambiguous/nested duplicate field matches.
    result = subprocess.run(['nice', '-n', '19', 'rg', '-o',
                             r'"(file|sourceSha256|sourceFile)":"[^"\\]*"', str(path)],
                            check=True, capture_output=True, text=True)
    records = []
    current = None
    for token in result.stdout.splitlines():
        key, value = next(iter(json.loads('{' + token + '}').items()))
        if key == 'file':
            if current is not None:
                records.append(current)
            current = {key: value}
        else:
            assert current is not None and key not in current
            current[key] = value
    if current is not None:
        records.append(current)
    assert len(records) == 207 and len({record['file'] for record in records}) == 207
    return {record['file']: record for record in records}


def main():
    names = ['step207/finite-modes.json', 'step207/current-inventory.json',
             'step207/global.json', 'step207/complete-boundary.json', 'ui478/inventory-plan.json']
    inputs = {name: sha(BASE / name) for name in names}
    finite, inventory, global_report, boundary, ui = [json.loads((BASE / name).read_text()) for name in names]
    expected = ui['geometryWasmSha256']
    assert ui['passedPreflight'] is True and ui['totalCases'] == 478
    assert inventory['passed'] is True and inventory['cases'] == 207
    assert global_report['passed'] is True and boundary['passed'] is True
    for report in (finite, global_report, boundary):
        assert report['artifactProvenance']['geometryWasmSha256'] == expected
        assert report['manifestSha256'] == inventory['manifestSha256']
    jobs = {job['name']: job for job in ui['jobs']}
    global_cases = {case['file']: case for case in global_report['cases']}
    manifest_path = BASE / 'step207/manifest.json'
    assert sha(manifest_path) == inventory['manifestSha256']
    metadata = source_metadata(manifest_path)
    assert set(metadata) == set(global_cases)
    boundary_cases = {case['file']: case for case in boundary['cases']}
    seam_cases = {case['file']: case for case in finite['smoothness']}
    laws = []
    modes = set()
    for case in finite['lawCases']:
        mode = case['mode']
        assert set(mode) == {'closed', 'affine', 'authored', 'guided'}
        assert all(type(value) is bool for value in mode.values())
        key = tuple(mode[name] for name in ('closed', 'affine', 'authored', 'guided'))
        assert key not in modes
        modes.add(key)
        bound = case['boundary']
        assert bound['continuousBound'] is True and bound['withinBudget'] is True
        assert bound['budget'] == (10 if mode['closed'] else 2)
        assert case['nativeSolid'] is True and case['stationContinuity'] == 'C0'
        assert global_cases[case['file']]['status'] == 'certified'
        witnesses = []
        for group, suffix in [('miter-laws', ''), ('miter-laws-profile-g2', '-profile-g2')]:
            filename = case['file'].removesuffix('.step') + suffix + '.r'
            source = next(source for source in jobs[group]['sources'] if source['file'] == filename)
            step_file = case['file'].removesuffix('.step') + suffix + '.step'
            assert metadata[step_file]['sourceSha256'] == source['sha256']
            widths = [width for width in (1440, 600) if f'{width}:{filename}' in jobs[group]['caseKeys']]
            assert widths == [1440, 600]
            witnesses.append({'group': group, 'source': source, 'widths': widths})
        laws.append({'step': case['file'], 'mode': mode, 'nativeBoundaryAndSolidCertified': True,
                     'stationContinuity': 'C0', 'plannedUiWitnesses': witnesses})
    assert modes == set(itertools.product((False, True), repeat=4))
    companions = []
    for case in inventory['companions']:
        assert global_cases[case['file']]['status'] == 'certified'
        assert boundary_cases[case['file']]['status'] == 'certified'
        seam = seam_cases[case['file']]
        assert seam['profileG2'] is True and seam['stationContinuity'] == case['station']
        assert seam['wholeBoundarySmoothness'] is False
        witnesses = []
        for job in ui['jobs']:
            for source in job['sources']:
                if source['sha256'] != case['sourceSha256']:
                    continue
                filename = source['file']
                widths = [width for width in (1440, 600) if f'{width}:{filename}' in job['caseKeys']]
                assert widths == [1440, 600]
                witnesses.append({'group': job['name'], 'file': filename, 'widths': widths})
        assert witnesses, f"No byte-identical UI source: {case['file']}"
        companions.append({**case, 'nativeBoundaryAndSolidCertified': True,
                           'wholeBoundarySmoothness': False, 'plannedUiWitnesses': witnesses})
    assert len(companions) == 50 and len({case['file'] for case in companions}) == 50
    ui_hashes = {source['sha256'] for job in ui['jobs'] for source in job['sources']}
    source_gaps = [record for record in metadata.values()
                   if record.get('sourceSha256') is not None and record['sourceSha256'] not in ui_hashes]
    without_source = [record['file'] for record in metadata.values() if record.get('sourceSha256') is None]
    assert len(source_gaps) == 4 and len(without_source) == 57
    assert sha(manifest_path) == inventory['manifestSha256']
    assert inputs == {name: sha(BASE / name) for name in names}
    report = {'schema': 'sweep-v11-step-ui-coverage/1', 'passed': True,
              'scope': 'Current native STEP proofs joined to planned UI sources and both widths. Planned witnesses are not completed UI executions or an all-input theorem.',
              'geometryWasmSha256': expected, 'manifestSha256': inventory['manifestSha256'],
              'inputsSha256': inputs, 'verifierSha256': sha(Path(__file__).resolve()),
              'lawModes': laws, 'correctedCompanions': companions,
              'sameSourceUiCases': 146, 'authoredSourceUiGaps': source_gaps,
              'casesWithoutAuthoredSourceHash': without_source,
              'fullUiQualified': False, 'fullGoalComplete': False}
    destination = OUT / 'v11-step-ui-coverage-20261018.json'
    temporary = destination.with_suffix('.writing')
    temporary.write_text(json.dumps(report, indent=2) + '\n')
    temporary.replace(destination)
    print('16 native law modes and50 native corrected companions have UI inputs at both widths; full UI execution remains separate.')


if __name__ == '__main__':
    main()
