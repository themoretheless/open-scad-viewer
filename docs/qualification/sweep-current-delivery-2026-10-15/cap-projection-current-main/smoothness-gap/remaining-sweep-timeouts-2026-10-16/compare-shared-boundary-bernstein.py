"""A-B-B-A on an unchanged captured original hollow request."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import os

out = Path(__file__).resolve().parent
request = out / 'fixed-plain-hollow-trace.json.27.request.json'
executables = {
    'A': Path('/private/tmp/osv-sweep-trim-probes-source-20261016/crates/target/release/examples/bench_sweep_volume_request'),
    'B': Path('/private/tmp/osv-sweep-shared-boundary-bernstein-source-20261016/crates/target/release/examples/bench_sweep_volume_request'),
}
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
receipt_path = out / 'shared-boundary-bernstein-paired.json'
assert not receipt_path.exists(), 'Preserve previous paired measurements'
receipt = {'schema': 'contact-cache-paired/1', 'state': 'running', 'pid': os.getpid(),
           'driverSha256': sha(__file__), 'requestSha256': sha(request),
           'executableSha256': {k: sha(v) for k, v in executables.items()}, 'runs': [],
           'scope': 'Native exact original request, unchanged proof budgets; original Bernstein restriction identity can tighten intervals and reduce work; full guarantee checks and response hashes retained. Not a browser or universal deadline gate.',
           'mainSourceModified': False, 'fullGoalComplete': False}

def save():
    tmp = receipt_path.with_suffix('.json.writing')
    tmp.write_text(json.dumps(receipt, indent=2) + '\n')
    tmp.replace(receipt_path)

save()
reference = None
for index, label in enumerate(['A', 'B', 'B', 'A']):
    assert sha(__file__) == receipt['driverSha256']
    assert sha(request) == receipt['requestSha256']
    assert sha(executables[label]) == receipt['executableSha256'][label]
    path = out / ('shared-boundary-bernstein-run-' + str(index) + '-' + label + '.json')
    entry = {'label': label, 'state': 'running', 'output': str(path)}
    receipt['runs'].append(entry)
    save()
    print(label + str(index) + ': started', flush=True)
    with path.with_suffix('.log').open('w') as log:
        result = subprocess.run([str(executables[label]), str(request), str(path)], stdout=log, stderr=subprocess.STDOUT)
    assert result.returncode == 0, (label, result.returncode)
    timing = json.loads(path.read_text())
    response = Path(str(path) + '.report.json')
    digest = sha(response)
    if reference is None:
        reference = digest
    checked=json.loads(response.read_text())
    assert checked['solidGeometryCertified'] is True
    assert checked['allFacesInjective'] is True and checked['allPairsClassified'] is True
    entry['sameResponseAsBaseline']=digest == reference
    entry['contactCells']=checked['contactCells']
    entry['contactDomainCells']=checked['contactDomainCells']
    assert timing['requestSha256'] == receipt['requestSha256']
    assert timing['solidGeometryCertified'] is True
    entry.update(state='passed', milliseconds=timing['milliseconds'], steps=timing['steps'],
                 responseSha256=digest)
    save()
    print(label + str(index) + ': ' + str(round(timing['milliseconds'])) + 'ms, original source request and all admission guarantees passed', flush=True)

a = [r['milliseconds'] for r in receipt['runs'] if r['label'] == 'A']
b = [r['milliseconds'] for r in receipt['runs'] if r['label'] == 'B']
ratio = statistics.median(b) / statistics.median(a)
stable = max(a) / min(a) < 1.15 and max(b) / min(b) < 1.15
benefit = ratio < 0.95 and all(new < old * 0.95 for old, new in zip(a, b))
receipt.update(state='complete', responsesIdentical=all(x['sameResponseAsBaseline'] for x in receipt['runs']), medianRatio=ratio,
               timingStable=stable, eligibleForAdoption=stable and benefit)
save()
print(json.dumps({'responsesIdentical': receipt['responsesIdentical'], 'medianRatio': ratio, 'timingStable': stable,
                  'eligibleForAdoption': stable and benefit}), flush=True)
