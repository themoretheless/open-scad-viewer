#!/usr/bin/env python3
"""Compare prebuilt stress executables; build both first, stop compilers, then run.
Usage: python3 compare.py /path/to/before /path/to/after results/benchmark.json
"""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys

before, after, output = map(Path, sys.argv[1:])
runs = []
for cycle in range(2):
    for version, executable in [('before', before), ('after', after), ('after', after), ('before', before)]:
        result = subprocess.run([str(executable.resolve()), '--benchmark-only'], text=True,
                                capture_output=True, timeout=60, check=True)
        data = json.loads(result.stdout)
        assert not data['oracle']['failures']
        assert all('error' not in row for row in data['benchmarks']), data
        runs.append({'cycle': cycle, 'version': version, 'data': data})
summary = []
for index, case in enumerate(runs[0]['data']['benchmarks']):
    medians = {v: [r['data']['benchmarks'][index]['median_us'] for r in runs if r['version'] == v]
               for v in ['before', 'after']}
    areas = [r['data']['benchmarks'][index]['area'] for r in runs]
    assert max(areas)-min(areas) < max(areas)*1e-8, (case['case'], areas)
    a,b = (statistics.median(medians[v]) for v in ['before','after'])
    summary.append({'case': case['case'], 'before_median_us': a, 'after_median_us': b,
                    'speedup': a/b, 'run_medians_us': medians,
                    'area_range': [min(areas),max(areas)]})
output.write_text(json.dumps({'executables': {v: {'path': str(p.resolve()),
                    'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
                    for v,p in [('before',before),('after',after)]},
                    'method': 'Two A/B/B/A cycles; each execution has three warmups and fifteen timed samples per case. Other machine activity is uncontrolled.',
                    'summary': summary, 'runs': runs}, indent=2)+'\n')
for row in summary:
    print(f"{row['case']}: {row['before_median_us']:.1f} -> {row['after_median_us']:.1f} us ({row['speedup']:.2f}x)")
