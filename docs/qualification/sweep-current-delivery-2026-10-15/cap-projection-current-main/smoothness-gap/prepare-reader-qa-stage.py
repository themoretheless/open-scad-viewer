"""Separate test-only QA overlay; preserve the live STEP/UI source stage."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess

root = Path(__file__).resolve().parents[5]
source = Path('/private/tmp/osv-sweep-shared-generators-arc-partition-source-20261015')
stage = Path('/private/tmp/osv-sweep-cffa-reader-qa-20261015')
base = Path(__file__).resolve().parent / 'current-source-build-arc-partition'
out = base / 'reader-qa'
stage.mkdir(exist_ok=False)
out.mkdir(exist_ok=False)
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger = json.loads((source / 'frozen-current-main-source-sha256.json').read_text())
original_overrides = json.loads((source / 'qa-source-overrides-current.json').read_text())
expected = {r['file']: r['afterSha256'] for r in original_overrides['files']}
inputs = {}
for name, h in ledger.items():
    p = source / name
    assert sha(p) == expected.get(name, h), name
    target = stage / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, target)
    inputs[name] = sha(target)
for name in ['src/generated', 'public/wasm', 'dist']:
    target = stage / name
    target.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(['/bin/cp', '-cR', str(source / name), str(target)], check=True)
(stage / 'node_modules').symlink_to((root / 'node_modules').resolve(), target_is_directory=True)
raw = 'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
(stage / raw).parent.mkdir(parents=True, exist_ok=True)
shutil.copy2(source / raw, stage / raw)
extra = json.loads((source / 'qualification-external-inputs.json').read_text())
for name, meta in extra['files'].items():
    if name in inputs:
        continue
    target = stage / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source / name, target)
    assert sha(target) == meta['sha256'], name
fixes = json.loads((base / 'pending-positive-snapshot-reader-fixes.json').read_text())
overrides = []
for record in fixes['files']:
    name = record['file']
    assert name.startswith('tests/')
    before = sha(stage / name)
    shutil.copy2(root / name, stage / name)
    after = sha(stage / name)
    assert before != after, name
    overrides.append({**record, 'beforeSha256': before, 'afterSha256': after})
    inputs[name] = after
for name, h in inputs.items():
    assert sha(stage / name) == h, name
receipt = {'schema': 'current-artifact-reader-test-overlay/1', 'passed': True,
    'stage': str(stage), 'sourceStage': str(source), 'geometryWasmSha256': sha(stage / 'public/wasm/geometry-kernel.wasm'),
    'rawWasmSha256': sha(stage / raw), 'files': overrides, 'sourceFiles': inputs,
    'scope': 'Same frozen product/native source and exact CFFA artifact; explicit positive-test decoder repairs only. Live STEP/UI source untouched.',
    'runtimeTestsPassed': False, 'fullGoalComplete': False}
assert receipt['geometryWasmSha256'] == 'cffa125ff73808c84735ccb26434e4dd6dfd0c2178258ed2719035b2114e1256'
for target in [out / 'source-receipt.json', stage / 'reader-qa-source-receipt.json']:
    target.write_text(json.dumps(receipt, indent=2) + '\n')
print(f'Separate reader QA stage ready: {len(overrides)} test overlays; compiled source unchanged', flush=True)
