#!/usr/bin/env python3
"""Read paired samples from prebuilt executables; do not benchmark Cargo."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys

before, after, output = map(Path, sys.argv[1:])
bins = {'before': before, 'after': after}
runs = []
for cycle in range(2):
    for version in ['before', 'after', 'after', 'before']:
        result = subprocess.run([str(bins[version].resolve())], text=True, capture_output=True,
                                check=True, timeout=60)
        runs.append({'cycle': cycle, 'version': version, 'data': json.loads(result.stdout)})
rows = []
for i, case in enumerate(runs[0]['data']):
    a, b = [statistics.median(r['data'][i]['native']['medianUs'] for r in runs if r['version'] == v)
            for v in ['before', 'after']]
    rows.append({'case': case['case'], 'before_us': a, 'after_us': b, 'ratio_after_before': b/a})
output.write_text(json.dumps({'method': 'Two A/B/B/A cycles; 21 samples after 3 warmups per case. No compiler/test/rasterizer from this task ran during measurement.',
    'binaries': {v: {'path': str(p.resolve()), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()} for v, p in bins.items()},
    'rows': rows, 'runs': runs}, indent=2)+'\n')
print(json.dumps(rows, indent=2))
