"""Reproduce each previously timed-out public case with original limits."""
from pathlib import Path
import hashlib
import json
import os
import re
import subprocess
import time

root = Path(__file__).resolve().parents[5]
out = Path(__file__).resolve().parent / 'isolated-timeouts'
out.mkdir(exist_ok=False)
prior = json.loads((out.parent / 'updated-snapshot-and-bound-contracts.json').read_text())
cases = [(str(Path(s['name']).relative_to(root)), t['fullName'])
         for s in prior['testResults'] for t in s['assertionResults']
         if t['status'] == 'failed' and any('STACK_TRACE_ERROR' in m for m in t.get('failureMessages', []))
         and t['fullName'] != 'builds retained hollow body through Rush and the real editor entrypoint']
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
inputs = sorted({p for p, _ in cases} | {'vitest.config.ts', 'package-lock.json',
    'public/wasm/geometry-kernel.wasm', 'src/generated/geometry-kernels/kernel_bg.wasm',
    'src/generated/geometry-kernels/identity.ts'})
identity = {p: digest(root / p) for p in inputs}
receipt = {'schema': 'sweep-isolated-timeouts/1', 'state': 'running', 'pid': os.getpid(),
           'inputs': identity, 'originalTimeoutMs': 30000, 'maxWorkers': 1,
           'cases': [], 'fullGoalComplete': False}

def save():
    p = out / 'receipt.json.writing'
    p.write_text(json.dumps(receipt, indent=2) + '\n')
    p.replace(out / 'receipt.json')

def unchanged():
    for p, h in identity.items():
        assert digest(root / p) == h, p

save()
for index, (file, name) in enumerate(cases):
    unchanged()
    argv = ['node', 'node_modules/vitest/vitest.mjs', 'run', '--maxWorkers=1', file,
            '-t', '^' + re.escape(name) + '$', '--reporter=json',
            '--outputFile=' + str(out / f'{index:02}.json')]
    case = {'file': file, 'name': name, 'state': 'running', 'argv': argv}
    receipt['cases'].append(case)
    save()
    print(f'{index + 1}/{len(cases)} started: {name}', flush=True)
    start = time.monotonic()
    with (out / f'{index:02}.log').open('w') as log:
        status = subprocess.run(argv, cwd=root, env={**os.environ, 'NODE_OPTIONS': '--max-old-space-size=8192'},
                                stdout=log, stderr=subprocess.STDOUT)
    unchanged()
    report = json.loads((out / f'{index:02}.json').read_text())
    selected = [t for s in report['testResults'] for t in s['assertionResults'] if t['status'] in ['passed', 'failed']]
    assert len(selected) == 1 and selected[0]['fullName'] == name
    case.update(state='passed' if status.returncode == 0 and report['success'] else 'failed',
                returncode=status.returncode, seconds=time.monotonic() - start,
                testMilliseconds=selected[0].get('duration'), reportSha256=digest(out / f'{index:02}.json'))
    save()
    print(f'{index + 1}/{len(cases)} {case["state"]}: {case["testMilliseconds"]} ms', flush=True)
receipt.update(state='complete', passed=sum(c['state'] == 'passed' for c in receipt['cases']),
               failed=sum(c['state'] == 'failed' for c in receipt['cases']))
save()
print(json.dumps({k: receipt[k] for k in ['state', 'passed', 'failed']}), flush=True)
