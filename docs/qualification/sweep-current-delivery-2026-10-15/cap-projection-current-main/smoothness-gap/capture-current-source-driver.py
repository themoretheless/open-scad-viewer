"""Capture current source without replacing files used by the running viewer."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[5]
assert (root / 'crates/Cargo.toml').is_file() and (root / 'package.json').is_file()
stage = Path(sys.argv[1]) if len(sys.argv)>1 else Path('/private/tmp/osv-sweep-shared-generators-current-source-20261015')
out = Path(sys.argv[2]) if len(sys.argv)>2 else Path(__file__).resolve().parent / 'current-source-build'
stage.mkdir(exist_ok=False)
out.mkdir(exist_ok=False)
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()

def inventory():
    paths = subprocess.check_output(['git', '-c', 'core.fsmonitor=false', 'ls-files',
        '--cached', '--others', '--exclude-standard', '-z'], cwd=root).decode().split('\0')
    return sorted(set(p for p in paths if p and not p.startswith(
        ('docs/qualification/', 'public/wasm/', 'src/generated/'))))

paths = inventory()
before = {}
for name in paths:
    p = root / name
    assert p.is_file() and not p.is_symlink(), name
    before[name] = sha(p)
    target = stage / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
after = {p: sha(root / p) for p in inventory()}
assert before == after, 'Source changed during capture; preserve this attempt without building it'
for name, h in before.items():
    assert sha(stage / name) == h, name
external = ['docs/qualification/cad-roadmap-2026-09-28/parts-history/imported-flange.json',
            'docs/qualification/cad-roadmap-2026-09-28/parts-history/cap-api-fixtures.json']
for name in external:
    target = stage / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(root / name, target)
ledger = out / 'frozen-current-main-source-sha256.json'
ledger.write_text(json.dumps(before, indent=2) + '\n')
shutil.copy2(ledger, stage / ledger.name)
capture = {'passed': True, 'stage': str(stage), 'runtimeInputCount': len(before),
           'sourceLedgerSha256': sha(ledger), 'testOnlyExternalInputs': {p: sha(root / p) for p in external},
           'changedDuringCapture': [], 'addedDuringCapture': [], 'removedDuringCapture': [],
           'nativeCapSourceSha256': before['crates/nurbs-core/src/construct/surfaces/sections/section_projection.rs'],
           'nativeArcPartitionSourceSha256': before.get('crates/nurbs-core/src/analysis/curve_measure/division_partition.rs'),
           'mainModified': False, 'artifactQualified': False, 'fullGoalComplete': False}
assert capture['nativeCapSourceSha256'] == '53ee88ea4be1d6d046994a03d55f45a83f550250530fb6df592cbda661a2f292'
for dest in [out, stage]:
    (dest / 'source-capture-receipt.json').write_text(json.dumps(capture, indent=2) + '\n')
print(f'Source captured: {len(before)} files, {capture["sourceLedgerSha256"]}', flush=True)
for name in ['src/generated', 'public/wasm', 'crates/target']:
    source = root / name
    destination = stage / name
    if source.exists():
        assert not destination.exists()
        destination.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(['/bin/cp', '-cR', str(source), str(destination)], check=True)
        print(f'Unqualified cache seed copied: {name}', flush=True)
(stage / 'node_modules').symlink_to(root / 'node_modules', target_is_directory=True)
optimizer = Path('/Users/themoretheless/.cargo/bin/wasm-opt')
seed = {'scope': 'Unqualified generated and COW build cache only; all workspace source inputs touched to rebuild.',
        'nodeModulesRealPath': str((root / 'node_modules').resolve()),
        'packageLockSha256': sha(stage / 'package-lock.json'),
        'installedNodeLockSha256': sha(root / 'node_modules/.package-lock.json'),
        'optimizerPath': str(optimizer), 'optimizerSha256': sha(optimizer),
        'optimizerVersion': subprocess.check_output([str(optimizer), '--version']).decode().strip(),
        'generatedSeedQualified': False}
for dest in [out, stage]:
    (dest / 'dependency-seed-receipt.json').write_text(json.dumps(seed, indent=2) + '\n')
for name, h in before.items():
    p = stage / name
    assert sha(p) == h, name
    if name.startswith('crates/'):
        os.utime(p, None)
print('Seed complete; source identity rechecked; no main generated/public artifact overwritten', flush=True)
