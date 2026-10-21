"""A-B-B-A on an unchanged captured original hollow request."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import os
import time

out = Path(__file__).resolve().parent
request = out / 'closed-corrected-binary-sequence.json'
executables = {
    'A': Path('/private/tmp/osv-sweep-adaptive-hull-source-20261016/crates/target/release/examples/bench_sweep_source_sequence'),
    'B': Path('/private/tmp/osv-sweep-level-reuse-source-20261016/crates/target/release/examples/bench_sweep_source_sequence'),
}
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
receipt_path = out / 'level-reuse-paired.json'
assert not receipt_path.exists(), 'Preserve previous paired measurements'
receipt = {'schema': 'contact-cache-paired/1', 'state': 'running', 'pid': os.getpid(),
           'driverSha256': sha(__file__), 'requestSha256': sha(request),
           'executableSha256': {k: sha(v) for k, v in executables.items()}, 'runs': [],
           'scope': 'Native exact original request, unchanged proof budgets and complete response equality. Not a browser or universal deadline gate.',
           'mainSourceModified': False, 'fullGoalComplete': False}

def save():
    tmp = receipt_path.with_suffix('.json.writing')
    tmp.write_text(json.dumps(receipt, indent=2) + '\n')
    tmp.replace(receipt_path)

save()
reference = None
inputs=json.loads((out/'closed-corrected-binary-inputs.json').read_text())
receipt['originalBinaryInputs']=inputs
receipt['launchLoad']=list(os.getloadavg())
while os.getloadavg()[0]>os.cpu_count():time.sleep(5)
for index, label in enumerate(['A', 'B', 'B', 'A']):
    assert all(sha(p)==digest for p,digest in inputs.items())
    assert sha(__file__) == receipt['driverSha256']
    assert sha(request) == receipt['requestSha256']
    assert sha(executables[label]) == receipt['executableSha256'][label]
    path = out / ('level-reuse-run-' + str(index) + '-' + label + '.json')
    entry = {'label': label, 'state': 'running', 'output': str(path)}
    receipt['runs'].append(entry)
    save()
    print(label + str(index) + ': started', flush=True)
    with path.with_suffix('.log').open('w') as log:
        result = subprocess.run([str(executables[label]), str(request), str(path)], stdout=log, stderr=subprocess.STDOUT)
    assert result.returncode == 0, (label, result.returncode)
    timing = json.loads(path.read_text())
    response = Path(str(path) + '.reports.json')
    digest = sha(response)
    if reference is None:
        reference = digest
    assert digest == reference, 'Geometry/proof/work response changed'
    assert json.loads(response.read_text())[-1]['solidGeometryCertified'] is True
    entry.update(state='passed', milliseconds=timing['milliseconds'], responseSha256=digest)
    save()
    print(label + str(index) + ': ' + str(round(timing['milliseconds'])) + 'ms, response identical', flush=True)

a = [r['milliseconds'] for r in receipt['runs'] if r['label'] == 'A']
b = [r['milliseconds'] for r in receipt['runs'] if r['label'] == 'B']
ratio = statistics.median(b) / statistics.median(a)
stable = max(a) / min(a) < 1.15 and max(b) / min(b) < 1.15
benefit = ratio < 0.95 and all(new < old * 0.95 for old, new in zip(a, b))
receipt.update(state='complete', responsesIdentical=True, medianRatio=ratio,
               timingStable=stable, eligibleForAdoption=stable and benefit)
save()
print(json.dumps({'responsesIdentical': True, 'medianRatio': ratio, 'timingStable': stable,
                  'eligibleForAdoption': stable and benefit}), flush=True)
