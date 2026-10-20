"""Capture the agreed shared source; preserve the running V4 qualification."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

root = Path.cwd()
out = Path(__file__).resolve().parent
seed = Path('/private/tmp/osv-sweep-frozen-current-v4-source-20261016')
stable = Path('/private/tmp/osv-cad-stable-20261016T171722Z')
stage = Path('/private/tmp/osv-sweep-frozen-current-v5-source-20261016')
receipt = out / 'frozen-current-v5-capture.json'
assert not stage.exists() and not receipt.exists(), 'Preserve existing candidates'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
agreed = json.loads((stable / 'manifest.json').read_text())
extra = json.loads((seed / 'qualification-external-inputs.json').read_text())
old_runtime = json.loads((seed / 'runtime-source-sha256.json').read_text())
selected = ['src', 'tests', 'examples', 'integrations', 'tools', 'scripts', 'public', 'crates', 'docs']
generated = ('src/generated/geometry-kernels/', 'src/generated/wasm-brotli/')
metadata = {'runtime-source-sha256.json', 'main-native-source-sha256.json',
    'qualification-external-inputs.json', 'qa-source-overrides-current.json',
    'seed-v3-runtime-ledger.json', 'seed-v3-native-ledger.json',
    'completed-build-receipt.json'}
qa = set(extra['files']) | {n for n in old_runtime if n.startswith('docs/qualification/')}

def qa_source(name):
    if (root / name).is_file():
        return root / name
    expected = extra['files'][name]['sha256'] if name in extra['files'] else old_runtime[name]
    assert sha(seed / name) == expected, ('Sealed seed QA input changed', name)
    return seed / name

def inputs():
    files = {}
    for directory in selected:
        for folder, dirs, names in os.walk(root / directory):
            dirs[:] = [d for d in dirs if d not in {'target', '.git', 'node_modules', '__pycache__'}
                and not (directory == 'docs' and d == 'qualification')]
            for name in names:
                p = Path(folder) / name
                relative = str(p.relative_to(root))
                if p.is_file() and not relative.startswith(generated) and relative != 'public/wasm/geometry-kernel.wasm':
                    files[relative] = sha(p)
    for p in root.iterdir():
        if p.is_file() and p.name not in metadata and (p.suffix in {'.json', '.toml', '.ts', '.js', '.mjs', '.html', '.css'}
                or p.name in {'.npmrc', '.node-version', '.nvmrc'}):
            files[p.name] = sha(p)
    for name in qa:
        files[name] = sha(qa_source(name))
    return files

def native_inputs():
    files = []
    for package in (root / 'crates').iterdir():
        if not package.is_dir() or package.name == 'target':
            continue
        if (package / 'src').is_dir():
            files.extend(p for p in (package / 'src').rglob('*') if p.is_file())
        files.extend(p for p in [package / 'Cargo.toml', package / 'build.rs'] if p.is_file())
    files.extend(root / n for n in ['crates/Cargo.toml', 'crates/Cargo.lock', 'rust-toolchain.toml', 'package-lock.json',
        'scripts/build-geometry-kernels.mjs', 'scripts/build-wasm-brotli.mjs', 'scripts/reproducible-cargo.mjs',
        'scripts/wasm-optimize.mjs', 'scripts/wasm-opt-cache.mjs', 'scripts/pack-geometry-kernel.mjs'])
    return {str(p.relative_to(root)): sha(p) for p in files}

for name, meta in agreed['files'].items():
    assert sha(stable / name) == meta['sha256'] == sha(root / name), name
before = inputs()
native = native_inputs()
main_before = sha(root / 'public/wasm/geometry-kernel.wasm')
started = time.monotonic()
r = {'state': 'capturing', 'passed': False, 'fullGoalComplete': False,
    'stage': str(stage), 'pid': os.getpid(), 'driverSha256': sha(__file__),
    'agreedSourceManifestSha256': sha(stable / 'manifest.json'), 'mainWasmBefore': main_before}
receipt.write_text(json.dumps(r, indent=2) + '\n')
subprocess.run(['cp', '-cR', str(seed), str(stage)], check=True)
for directory in selected:
    source = stable / directory if directory == 'crates' else root / directory
    excludes = ['--exclude=node_modules/', '--exclude=__pycache__/']
    if directory == 'src':
        excludes += ['--exclude=generated/geometry-kernels/', '--exclude=generated/wasm-brotli/']
    if directory == 'public':
        excludes += ['--exclude=wasm/geometry-kernel.wasm']
    if directory == 'crates':
        excludes += ['--exclude=target/']
    if directory == 'docs':
        excludes += ['--exclude=qualification/']
    subprocess.run(['rsync', '-a', '--delete', *excludes, str(source) + '/', str(stage / directory) + '/'], check=True)
for name in before:
    if '/' not in name or name in qa:
        destination = stage / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        source = qa_source(name) if name in qa else root / name
        subprocess.run(['cp', '-c', str(source), str(destination)], check=True)
after = inputs()
assert before == after and native == native_inputs(), 'Shared source changed during capture; do not build'
assert all(sha(stage / n) == h for n, h in before.items()), 'Copied runtime source differs'
assert all(sha(stage / n) == h for n, h in native.items()), 'Copied native source differs'
for name in metadata:
    p = stage / name
    if p.exists():
        p.unlink()
new_extra = {'files': {n: {**m, 'sha256': sha(stage / n)} for n, m in extra['files'].items()},
    'browserDependenciesRealPath': extra['browserDependenciesRealPath'],
    'browserDependencyFiles': extra['browserDependencyFiles']}
for name, digest in new_extra['browserDependencyFiles'].items():
    assert sha(Path(new_extra['browserDependenciesRealPath']) / name) == digest, name
(stage / 'runtime-source-sha256.json').write_text(json.dumps(before, indent=2) + '\n')
(stage / 'main-native-source-sha256.json').write_text(json.dumps(native, indent=2) + '\n')
(stage / 'qualification-external-inputs.json').write_text(json.dumps(new_extra, indent=2) + '\n')
(stage / 'qa-source-overrides-current.json').write_text('{"files":[]}\n')
assert sha(root / 'public/wasm/geometry-kernel.wasm') == main_before
r.update(state='complete', passed=True, seconds=time.monotonic() - started,
    nativeInputs=len(native), runtimeInputs=len(before), agreedNativeFiles=agreed['nativeFiles'],
    sourceChangedDuringCapture=False, mainWasmModified=False,
    nativeLedgerSha256=sha(stage / 'main-native-source-sha256.json'),
    runtimeLedgerSha256=sha(stage / 'runtime-source-sha256.json'),
    scope='Coherent unbuilt shared-source candidate. Seed generated binaries and caches are not qualification of this changed source; all build/runtime/STEP/UI gates remain required.')
receipt.write_text(json.dumps(r, indent=2) + '\n')
print(json.dumps(r), flush=True)
