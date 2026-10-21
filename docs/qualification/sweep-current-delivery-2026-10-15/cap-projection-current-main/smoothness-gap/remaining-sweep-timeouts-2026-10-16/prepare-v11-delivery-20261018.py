"""Prepare a reviewable delivery inventory; never changes product files.

The provider metadata references a record that must accompany publication.
This draft binds the already completed provider gates and lists every file
that would be replaced, including original bytes that must be backed up.
It deliberately cannot authorize publication while STEP/UI is incomplete.
"""
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[6]
OUT = Path(__file__).resolve().parent
STAGE = Path('/private/tmp/osv-sweep-common-source-v11-20261018')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(path):
    if path.is_symlink():
        raise ValueError(f'Refusing symlink delivery target: {path}')
    if not path.exists():
        return None
    if path.is_file():
        return {'kind': 'file', 'sha256': sha(path), 'bytes': path.stat().st_size}
    entries = {}
    for entry in sorted(path.rglob('*')):
        if entry.is_symlink():
            raise ValueError(f'Refusing symlink in artifact tree: {entry}')
        if entry.is_file():
            entries[entry.relative_to(path).as_posix()] = {
                'sha256': sha(entry), 'bytes': entry.stat().st_size,
            }
    return {'kind': 'directory', 'files': entries}


def main():
    assert ROOT.name == 'open-scad-viewer' and (ROOT / 'package.json').is_file()
    capture = json.loads((OUT / 'v11-common-source-capture-20261018.json').read_text())
    runtime_path = OUT / 'v11r1-common-source-runtime-20261018.json'
    runtime = json.loads(runtime_path.read_text())
    assert runtime['state'] == 'complete' and runtime['passed'] is True
    metadata = (STAGE / 'src/core/currentRuntimeGeometryEvidence.ts').read_text()
    metadata = json.loads(metadata[metadata.index('Object.freeze(') + 14:metadata.rindex(')')])
    expected = runtime['packedSha256']
    assert metadata['wasmSha256'] == expected
    for name, key in [('THIRD_PARTY_NOTICES.md', 'noticesSha256'),
                      ('package-lock.json', 'lockfileSha256'),
                      ('crates/Cargo.lock', 'rustLockfileSha256')]:
        assert sha(STAGE / name) == metadata[key]
        assert sha(ROOT / name) == metadata[key]
    record_name = metadata['recordId']
    assert re.fullmatch(r'docs/qualification/runtime-provider-[0-9a-f]{12}-2026-10-18.json', record_name)
    assert not (ROOT / record_name).exists(), 'Never overwrite an existing provider record'
    targets = list(capture['explicitPrivateOverrides']) + [
        'src/generated/geometry-kernels', 'src/generated/wasm-brotli',
        'public/wasm/geometry-kernel.wasm', 'dist',
    ]
    entries = []
    for name in targets:
        before = inventory(ROOT / name)
        after = inventory(STAGE / name)
        assert after is not None
        if name in capture['explicitPrivateOverrides']:
            prior = capture['explicitPrivateOverrides'][name]['mainSha256']
            assert (before['sha256'] if before else None) == prior, f'Primary source drift: {name}'
        entries.append({'path': name, 'before': before, 'after': after})
    # Hash twice: a concurrent build cannot silently produce a mixed inventory.
    assert all(inventory(ROOT / e['path']) == e['before'] for e in entries)
    assert all(inventory(STAGE / e['path']) == e['after'] for e in entries)
    receipt_names = [
        'v11r1-common-source-runtime-20261018.json',
        'frozen-current-v11-build.json',
        'v11-original-five-broad-confirmed-20261018.json',
        'v11-provider-baseline.json',
        'v11-common-peer-host.json',
    ]
    draft_record = {
        'schema': 'runtime-provider-qualification/1',
        'state': 'draft-awaiting-final-artifact-gates',
        'qualified': False,
        'recordId': record_name,
        'corpusVersion': metadata['corpusVersion'],
        'target': 'browser-worker/node-mcp',
        'artifacts': metadata,
        'completedRuntimeEvidence': {
            name: {'path': str(OUT / name), 'sha256': sha(OUT / name)}
            for name in receipt_names
        },
        'providerBaseline': {'files': runtime['providerBaselineFiles'],
                             'tests': runtime['providerBaselineTests']},
        'publicRegressionTests': runtime['public100Tests'],
        'fullGoalComplete': False,
        'scope': 'Exact-artifact provider/runtime qualification. Full sweep proof and UI obligations remain separate.',
    }
    plan = {
        'schema': 'sweep-v11-delivery-plan/1',
        'scriptSha256': sha(Path(__file__).resolve()),
        'candidateRoot': str(STAGE),
        'primaryRoot': str(ROOT),
        'candidateWasmSha256': expected,
        'sourceCaptureSha256': sha(OUT / 'v11-common-source-capture-20261018.json'),
        'runtimeReceiptSha256': sha(runtime_path),
        'targets': entries,
        'newProviderRecord': {'path': record_name, 'draft': draft_record},
        'canPublish': False,
        'requiredBeforePublication': [
            'Rerun audit-v11-delivery-readiness-20261018.py and require every check passed.',
            'Require exact before/after inventories and preserve all original bytes in a recoverable backup.',
            'Finalize the referenced provider record with complete final STEP/UI and supplemental receipts.',
            'Copy the eight source overrides, generated trees, public WASM and distribution as one guarded delivery.',
            'Verify complete delivered identities and actual runtime provider binding; preserve concurrent work.',
        ],
        'fullGoalComplete': False,
    }
    path = OUT / 'v11-delivery-plan-20261018.json'
    temp = path.with_suffix('.writing')
    temp.write_text(json.dumps(plan, indent=2) + '\n')
    temp.replace(path)
    print(json.dumps({'targets': len(entries), 'canPublish': False,
                      'providerRecordExists': False, 'plan': str(path)}, indent=2))


if __name__ == '__main__':
    main()
