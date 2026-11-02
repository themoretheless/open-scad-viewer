"""Verify the frozen first-version source set. Run from any directory."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent
manifest = json.loads((root / 'clean-baseline.json').read_text())
expected = manifest['files']
current = {'SPEC.md'} | {
    str(path.relative_to(root)) for path in (root / 'clean').rglob('*') if path.is_file()
}
assert current == set(expected), 'clean source set changed'
for name, digest in expected.items():
    assert hashlib.sha256((root / name).read_bytes()).hexdigest() == digest, name
print(f'Clean baseline verified: {len(expected)} files unchanged.')
