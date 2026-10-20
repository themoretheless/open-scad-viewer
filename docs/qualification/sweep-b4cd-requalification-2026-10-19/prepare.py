"""Preserve a private, byte-bound QA checkout of the installed b4cd kernel."""
from pathlib import Path
import hashlib
import json
import shutil

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
STAGE = Path('/private/tmp/osv-sweep-b4cd-requalification-20261019')
NATIVE = Path('/private/tmp/osv-trimmed-branches-topology-source-v2-20261019')
PREVIOUS = Path('/private/tmp/osv-sweep-common-source-v11-20261018')
PEER = ROOT / 'docs/qualification/trimmed-branches-topology-2026-10-19'
WASM = 'b4cd5d582ad11bb4f11aebbc0567f98bbf33df7f8a2d57f3a28a2b1b00cc9e06'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert not STAGE.exists(), 'Never overwrite an earlier snapshot'
STAGE.mkdir()
native = json.loads((PEER / 'frozen-native-source-sha256.json').read_text())
for name, expected in native.items():
    assert sha(NATIVE / name) == expected, name
shutil.copytree(NATIVE / 'crates', STAGE / 'crates',
                ignore=shutil.ignore_patterns('target', '__pycache__'))
for name in ['src', 'public', 'dist', 'scripts', 'tests', 'examples', 'integrations']:
    shutil.copytree(ROOT / name, STAGE / name)
shutil.copytree(NATIVE / 'docs', STAGE / 'docs')
for name in ['index.html', 'env.d.ts', 'package.json', 'package-lock.json',
             'tsconfig.json', 'tsconfig.mcp.json', 'vite.config.ts',
             'vite.qualification.config.ts', 'vitest.config.ts', 'rust-toolchain.toml']:
    source = ROOT / name
    if not source.exists():
        source = NATIVE / name
    shutil.copy2(source, STAGE / name)
(STAGE / 'node_modules').symlink_to(ROOT / 'node_modules', target_is_directory=True)
(STAGE / 'tools').mkdir()
(STAGE / 'tools/browser-qualification').symlink_to(ROOT / 'tools/browser-qualification', target_is_directory=True)
for name in ['qualification-raw-imports.mjs', 'original-annular-walls.json',
             'original-budget-ledger.json', 'original-geometry-projection.json',
             'original-request-baseline.json', 'original-step-inventory.json']:
    shutil.copy2(PREVIOUS / name, STAGE / name)
shutil.copytree(PREVIOUS / 'qualification-law-sources', STAGE / 'qualification-law-sources')
runtime = json.loads((PEER / 'runtime-source-sha256.json').read_text())
receipt = json.loads((PEER / 'source-receipt.json').read_text())
for name, expected in {**runtime, **receipt['runtimeSha256']}.items():
    assert sha(STAGE / name) == expected, name
for name, expected in native.items():
    assert sha(STAGE / name) == expected, name
for name in ['public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm',
             'dist/wasm/geometry-kernel.wasm']:
    assert sha(ROOT / name) == sha(STAGE / name) == WASM
ledger = {}
for parent in STAGE.iterdir():
    if parent.name in ['node_modules', 'tools']:
        continue
    paths = parent.rglob('*') if parent.is_dir() else [parent]
    for path in paths:
        if path.is_file() and '__pycache__' not in path.parts:
            ledger[str(path.relative_to(STAGE))] = sha(path)
(STAGE / 'qualification-inputs.json').write_text(json.dumps(ledger, indent=2) + '\n')
(STAGE / 'qa-source-overrides-current.json').write_text('{"files":[]}\n')
public = json.loads((PEER / 'public-test-inventory.json').read_text())
for name, expected in public['files'].items():
    assert sha(STAGE / name) == expected, name
record = {'stage': str(STAGE), 'wasmSha256': WASM, 'nativeSource': str(NATIVE),
          'nativeSourceFiles': len(native), 'runtimeFilesBoundToPeerDelivery': len(runtime),
          'inputLedgerSha256': sha(STAGE / 'qualification-inputs.json'),
          'inputFiles': len(ledger), 'publicFiles': public['files'],
          'originalTestAndHookTimeoutMs': 30000, 'maxWorkers': 1,
          'scope': 'Fresh execution; no historical success is relabeled as a b4cd result.'}
(OUT / 'snapshot.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps({k: v for k, v in record.items() if k != 'publicFiles'}))
