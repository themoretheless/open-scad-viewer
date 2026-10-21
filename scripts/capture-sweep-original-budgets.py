"""Capture original authored budgets missing from the older compact baseline.

This extracts original constructor data, never a current bound or tolerance.
"""
import hashlib
import json
from pathlib import Path
import sys

manifest_path, request_path, output_path = map(Path, sys.argv[1:])
raw = manifest_path.read_bytes()
request = json.loads(request_path.read_text())
assert hashlib.sha256(raw).hexdigest() == request['originalManifestSha256']
manifest = json.loads(raw)
original = {case['file']: case for case in manifest['cases']}
cases = []
for req in request['cases']:
    item = original[req['file']]
    paths = [
        ('construction', 'bodyBoundaryEvidence', 'budget'),
        ('construction', 'boundaryCertificate', 'budget'),
        ('boundaryCertificate', 'budget'),
        ('boundaryError', 'budget'),
        ('construction', 'budget'),
    ]
    found = []
    for keys in paths:
        value = item
        for key in keys:
            value = value.get(key) if isinstance(value, dict) else None
        if value is not None:
            found.append(('.'.join(keys), value))
    assert found, f'Missing original authored budget: {req["file"]}'
    assert all(value == req['originalBudgetMm'] for _, value in found), req['file']
    cases.append({'file': req['file'], 'budgetMm': found[0][1],
                  'originalPaths': [path for path, _ in found]})
output = {'schema': 'sweep-original-authored-budget-capture/1',
          'originalManifestSha256': request['originalManifestSha256'],
          'extractorSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          'cases': cases,
          'scope': 'Exact original constructor budget extraction; not old proof outcome reuse.'}
output_path.write_text(json.dumps(output, indent=2) + '\n')
print(f'{len(cases)} original authored budgets independently extracted and unchanged')
