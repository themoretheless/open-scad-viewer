"""Deliver the exact qualified candidate, preserving every replaced byte.

Default invocation only checks readiness. --apply is deliberately rejected
until every independent artifact gate and the immutable delivery plan match.
No Git commands, rebuilds, timeout changes or peer-process control are used.
"""
from pathlib import Path
import datetime
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[5]
STAGE = Path('/private/tmp/osv-sweep-common-source-v11-20261018')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    temporary = path.with_suffix('.writing')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def load_inventory():
    spec = importlib.util.spec_from_file_location('delivery_inventory', OUT / 'prepare-v11-delivery-20261018.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.inventory


def apply_target(destination, incoming, displaced, before, after, inventory):
    """One atomic file/tree replacement, refusing concurrent target changes."""
    assert inventory(destination) == before, f'Concurrent write: {destination}'
    assert inventory(incoming) == after, f'Incoming delivery changed: {incoming}'
    if destination.is_dir():
        displaced.parent.mkdir(parents=True, exist_ok=True)
        os.rename(destination, displaced)
        try:
            os.rename(incoming, destination)
        except BaseException:
            os.rename(displaced, destination)
            raise
    else:
        destination.parent.mkdir(parents=True, exist_ok=True)
        os.replace(incoming, destination)
    assert inventory(destination) == after


def main():
    assert sys.argv[1:] in ([], ['--apply']), 'Use no arguments for a check, or --apply'
    assert ROOT.name == 'open-scad-viewer' and (ROOT / 'package.json').is_file()
    inventory = load_inventory()
    plan_path = OUT / 'v11-delivery-plan-20261018.json'
    plan = json.loads(plan_path.read_text())
    assert plan['primaryRoot'] == str(ROOT) and plan['candidateRoot'] == str(STAGE)
    assert plan['scriptSha256'] == sha(OUT / 'prepare-v11-delivery-20261018.py')
    assert plan['sourceCaptureSha256'] == sha(OUT / 'v11-common-source-capture-20261018.json')
    assert plan['runtimeReceiptSha256'] == sha(OUT / 'v11r1-common-source-runtime-20261018.json')
    guard_test = OUT / 'test_v11_delivery_guards.py'
    subprocess.run([sys.executable, str(guard_test)], cwd=ROOT,
                   check=True, capture_output=True, text=True)
    result = subprocess.run([sys.executable, str(OUT / 'audit-v11-delivery-readiness-20261018.py')], cwd=ROOT, check=True, capture_output=True, text=True)
    preflight_path = OUT / 'v11-delivery-readiness-current-20261018.json'
    preflight = json.loads(preflight_path.read_text())
    if preflight.get('readyForFiniteArtifactDelivery') is not True:
        print(result.stdout.strip())
        print('Delivery refused; no product files changed.')
        return 2
    assert preflight['candidateWasmSha256'] == plan['candidateWasmSha256']
    expected_paths = set(json.loads((OUT / 'v11-common-source-capture-20261018.json').read_text())['explicitPrivateOverrides']) | {
        'src/generated/geometry-kernels', 'src/generated/wasm-brotli',
        'public/wasm/geometry-kernel.wasm', 'dist',
    }
    assert {e['path'] for e in plan['targets']} == expected_paths and len(plan['targets']) == 12
    for entry in plan['targets']:
        assert inventory(ROOT / entry['path']) == entry['before'], f"Primary target changed: {entry['path']}"
        assert inventory(STAGE / entry['path']) == entry['after'], f"Candidate changed: {entry['path']}"
    record_path = ROOT / plan['newProviderRecord']['path']
    assert not record_path.exists(), 'Existing provider record must be preserved'
    if not sys.argv[1:]:
        print('All finite artifact gates passed; exact delivery inputs match. Check only; no product files changed.')
        return 0
    timestamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    backup_parent = Path.home() / '.codex' / 'backups' / 'open-scad-viewer'
    backup_parent.mkdir(parents=True, exist_ok=True)
    backup = Path(tempfile.mkdtemp(prefix=f'sweep-v11-primary-{timestamp}-', dir=backup_parent))
    assert backup.stat().st_dev == ROOT.stat().st_dev, 'Atomic delivery requires the backup on the primary filesystem'
    journal_path = OUT / 'v11-primary-delivery-20261018.json'
    assert not journal_path.exists(), 'Never replace an earlier delivery journal'
    journal = {
        'schema': 'sweep-v11-guarded-primary-delivery/1', 'state': 'preparing',
        'passed': False, 'backup': str(backup), 'planSha256': sha(plan_path),
        'publisherSha256': sha(Path(__file__).resolve()),
        'guardTestsSha256': sha(guard_test),
        'preflightSha256': sha(preflight_path),
        'geometryWasmSha256': plan['candidateWasmSha256'],
        'completed': [], 'fullGoalComplete': False,
    }
    shutil.copyfile(plan_path, backup / 'plan.json')
    shutil.copyfile(preflight_path, backup / 'preflight.json')
    assert sha(backup / 'plan.json') == journal['planSha256']
    assert sha(backup / 'preflight.json') == journal['preflightSha256']
    write_json(journal_path, journal)
    # Copy originals and incoming bytes before changing any product path.
    for entry in plan['targets']:
        relative = entry['path']
        for parent, source, expected in [('before', ROOT / relative, entry['before']),
                                         ('incoming', STAGE / relative, entry['after'])]:
            if expected is None:
                continue
            destination = backup / parent / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            if source.is_dir():
                shutil.copytree(source, destination)
            else:
                shutil.copy2(source, destination)
            assert inventory(destination) == expected
    evidence_names = [
        'v11r1-common-source-runtime-20261018.json', 'frozen-current-v11-build.json',
        'v11-original-five-broad-confirmed-20261018.json',
        'frozen-v11r1-common-source-20261018-independent-step-ui-final/receipt.json',
        'frozen-v11r1-common-source-20261018-independent-step-ui-final/canonical207-ui478/receipt.json',
        'frozen-v11r1-common-source-20261018-independent-step-ui-final/canonical207-ui478/ui478/full-reconciliation.json',
        'v11-companion27-followup-20261018.json', 'v11-companion27-current-contracts-20261018.json',
        'v11-step-ui-coverage-20261018.json', 'v11-native-rush-ui8-followup-20261018.json',
        'v11-surface-native-followup-20261018.json', 'v11-surface-native-harness-20261018.json',
    ]
    record = plan['newProviderRecord']['draft']
    record.update(state='complete', qualified=True,
                  finalArtifactEvidence={name: {'path': str(OUT / name), 'sha256': sha(OUT / name)} for name in evidence_names},
                  deliveryPlanSha256=sha(plan_path), deliveredAtUtc=timestamp)
    # Recheck every target after backup, before the first primary mutation.
    assert all(inventory(ROOT / e['path']) == e['before'] for e in plan['targets'])
    journal['state'] = 'applying'
    write_json(journal_path, journal)
    try:
        # O_EXCL prevents overwriting another writer's provider record.
        with record_path.open('x') as handle:
            handle.write(json.dumps(record, indent=2) + '\n')
        journal['providerRecordSha256'] = sha(record_path)
        write_json(journal_path, journal)
        for index, entry in enumerate(plan['targets']):
            destination = ROOT / entry['path']
            incoming = backup / 'incoming' / entry['path']
            # Preserve a second copy of displaced directory entries, including metadata.
            apply_target(destination, incoming, backup / 'displaced' / str(index),
                         entry['before'], entry['after'], inventory)
            journal['completed'].append(entry['path'])
            write_json(journal_path, journal)
        assert all(inventory(ROOT / e['path']) == e['after'] for e in plan['targets'])
        assert sha(record_path) == journal['providerRecordSha256']
        journal.update(state='complete-awaiting-primary-runtime-verification', passed=False)
        write_json(journal_path, journal)
    except BaseException as error:
        journal.update(state='interrupted-preserved-for-recovery', error=repr(error))
        write_json(journal_path, journal)
        # Do not roll back over a concurrent writer. Originals and displaced
        # trees are intact in the recorded backup, and the journal is explicit.
        raise
    print(json.dumps({'state': journal['state'], 'backup': str(backup),
                      'publishedPaths': len(journal['completed']),
                      'fullGoalComplete': False}, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
