"""Compact read-only progress; never interpret a running case as a pass."""
from pathlib import Path
import json

OUT = Path(__file__).resolve().parent / 'optimized'


def read(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None


for relative in ['finish-retry2.json', 'supplements/receipt.json']:
    data = read(OUT / relative)
    if data:
        phase = next((p['name'] for p in reversed(data.get('phases', []))
                      if p['state'] in ['running', 'failed']), None)
        print(relative, data['state'], phase or '', data.get('error', ''))

ui = OUT / 'ui478-complete-retry2'
jobs = read(ui / 'fresh-run.json')
total = 0
if jobs:
    for job in jobs['jobs']:
        matrix = read(ui / job['name'] / 'matrix.json')
        if not matrix:
            continue
        cases = matrix.get('cases', [])
        count = sum(case.get('status') == 'passed' for case in cases)
        total += count
        if job['status'] in ['running', 'failed']:
            print(job['name'], job['status'], count,
                  [(c.get('file'), c.get('status'), c.get('viewport', {}).get('width'))
                   for c in cases[-1:]])
    print('UI passed', total, '/478')
