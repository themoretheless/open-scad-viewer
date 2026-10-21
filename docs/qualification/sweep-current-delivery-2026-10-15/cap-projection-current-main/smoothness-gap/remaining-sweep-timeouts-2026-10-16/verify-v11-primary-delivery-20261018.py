"""Verify exact primary bytes and fresh native provider execution after delivery.

This cannot publish, rebuild, replace source or complete the full sweep goal.
The original qualified candidate is immutable; the primary must equal it.
"""
from pathlib import Path
import datetime
import hashlib
import importlib.util
import json
import subprocess
import sys

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[5]
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    temp = path.with_suffix('.writing')
    temp.write_text(json.dumps(value, indent=2) + '\n')
    temp.replace(path)


def main():
    assert not sys.argv[1:], 'This verifier takes no arguments'
    journal_path = OUT / 'v11-primary-delivery-20261018.json'
    if not journal_path.exists():
        print('Verification refused: no primary delivery journal; candidate is not published.')
        return 2
    journal = json.loads(journal_path.read_text())
    assert journal['state'] == 'complete-awaiting-primary-runtime-verification'
    assert journal['passed'] is False and len(journal['completed']) == 12
    plan_path = OUT / 'v11-delivery-plan-20261018.json'
    plan = json.loads(plan_path.read_text())
    assert sha(plan_path) == journal['planSha256']
    assert sha(OUT / 'publish-v11-guarded-20261018.py') == journal['publisherSha256']
    assert sha(OUT / 'test_v11_delivery_guards.py') == journal['guardTestsSha256']
    assert journal['geometryWasmSha256'] == plan['candidateWasmSha256']
    backup = Path(journal['backup'])
    assert sha(backup / 'plan.json') == journal['planSha256']
    assert sha(backup / 'preflight.json') == journal['preflightSha256']
    spec = importlib.util.spec_from_file_location('inventory', OUT / 'prepare-v11-delivery-20261018.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    inventory = module.inventory

    def exact_primary():
        for entry in plan['targets']:
            assert inventory(ROOT / entry['path']) == entry['after'], f"Primary drift: {entry['path']}"
            if entry['before'] is not None:
                assert inventory(backup / 'before' / entry['path']) == entry['before']

    exact_primary()
    provider_path = ROOT / plan['newProviderRecord']['path']
    assert sha(provider_path) == journal['providerRecordSha256']
    provider = json.loads(provider_path.read_text())
    assert provider['state'] == 'complete' and provider['qualified'] is True
    assert provider['artifacts']['wasmSha256'] == plan['candidateWasmSha256']
    for entry in provider['finalArtifactEvidence'].values():
        assert sha(Path(entry['path'])) == entry['sha256']
    phases = []
    commands = [
        ('primary-finite-preflight', [sys.executable, str(OUT / 'audit-v11-delivery-readiness-20261018.py')]),
        ('primary-distribution', [NODE, 'scripts/verify-dist.mjs']),
        ('primary-provider-identity', [NODE, '--import', 'tsx', 'scripts/audit-runtime-geometry-identity.mts']),
        ('primary-native-provider-execution', [NODE, '--import', 'tsx', 'scripts/audit-runtime-manifests.mts']),
    ]
    report_path = OUT / 'v11-primary-runtime-verification-20261018.json'
    report = {'schema': 'sweep-v11-primary-runtime-verification/1', 'state': 'running',
              'passed': False, 'geometryWasmSha256': plan['candidateWasmSha256'],
              'verifierSha256': sha(Path(__file__).resolve()), 'phases': phases,
              'scope': 'Exact primary delivery and fresh two-provider native execution. Full sweep requirement audit remains separate.',
              'fullGoalComplete': False}
    write(report_path, report)
    try:
        for name, argv in commands:
            result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
            log = OUT / f'v11-{name}-20261018.log'
            log.write_text(result.stdout + result.stderr)
            assert result.returncode == 0, f'{name} failed; inspect {log}'
            if name == 'primary-finite-preflight':
                preflight = json.loads((OUT / 'v11-delivery-readiness-current-20261018.json').read_text())
                assert preflight['readyForFiniteArtifactDelivery'] is True
            elif name == 'primary-provider-identity':
                identity = json.loads(result.stdout)
                assert identity['matches'] is True and len(identity['records']) == 2
                for record in identity['records']:
                    assert record['qualificationStatus'] == 'qualified'
                    assert record['observedFingerprint'] == 'sha256:' + plan['candidateWasmSha256']
            elif name == 'primary-native-provider-execution':
                runtime = json.loads(result.stdout)
                assert runtime['matches'] is True and runtime['publicArtifactMatches'] is True
                assert runtime['artifactSha256'] == plan['candidateWasmSha256']
                assert runtime['compilerLoads'] == 1
                assert {r['engineClass'] for r in runtime['records']} == {'mesh', 'brep'}
                assert all(r['matches'] is True and r['runtimeEvidence'] is True and r['volumeMm3'] > 0 for r in runtime['records'])
            phases.append({'name': name, 'passed': True, 'log': str(log), 'sha256': sha(log)})
            write(report_path, report)
        exact_primary()
        assert sha(provider_path) == journal['providerRecordSha256']
        report.update(state='complete', passed=True,
                      verifiedAtUtc=datetime.datetime.now(datetime.timezone.utc).isoformat())
        write(report_path, report)
        journal.update(state='complete', passed=True,
                       primaryRuntimeVerificationSha256=sha(report_path),
                       primaryRuntimeVerification=str(report_path))
        write(journal_path, journal)
    except BaseException as error:
        report.update(state='failed-needs-investigation', error=repr(error))
        write(report_path, report)
        raise
    print(json.dumps({'deliveredAndVerified': True, 'geometryWasmSha256': plan['candidateWasmSha256'],
                      'preservedOriginals': str(backup), 'fullGoalComplete': False}, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
