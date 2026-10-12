"""Same original whole model, full budgets, source-bound frozen P/R binaries."""
from pathlib import Path
import hashlib
import json
import resource
import statistics
import subprocess
import time

base = Path('/Users/themoretheless/.codex/tmp')
directory = Path(__file__).resolve().parent
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
model = base / 'osv-blend-factor-original-model.json'
assert model.stat().st_size == 1715251
assert sha(model) == '1466930912255099b45e298ee0f02d56308c9c25bd41e6643177b3a344b225bb'
variants = {
    'P': base / 'osv-sweep-global-probe-first-jets-bin',
    'R': base / 'osv-sweep-global-probe-stable-span-cache-bin',
}
receipts = {
    'P': json.loads((directory.parent / 'closed-frenet-first-jets-current/probe-build-receipt.json').read_text()),
    'R': json.loads((directory / 'probe-build-receipt.json').read_text()),
}
for name, binary in variants.items():
    assert receipts[name]['sourceOwnerVerified'] is True
    assert sha(binary) == receipts[name]['binarySha256']
reference = json.loads((base / 'osv-first-jets-global-1-P.json').read_text())['report']
runs = []
for index, name in enumerate(['P', 'R', 'R', 'P']):
    output = base / f'osv-stable-span-cache-global-{index}-{name}.json'
    log = output.with_suffix('.log')
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.monotonic()
    with log.open('w') as stream:
        result = subprocess.run([str(variants[name]), str(model), str(output)], stdout=stream, stderr=subprocess.STDOUT)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    assert result.returncode == 0, (name, log)
    data = json.loads(output.read_text())
    assert data['report'] == reference, 'Complete report changed'
    canonical = hashlib.sha256(json.dumps(data['report'], sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    assert canonical == '9637d9fec5ee4fdcb4ad5532a1f1d3647228bd2f29920ba40f6a627a56e2d32f'
    run = {
        'variant': name, 'wallSeconds': time.monotonic() - start,
        'processCpuSeconds': after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
        'auditThreadCpuSeconds': data['cpuSeconds'], 'reportCanonicalSha256': canonical,
        'output': str(output), 'outputSha256': sha(output), 'binarySha256': sha(variants[name]),
    }
    runs.append(run)
    print(json.dumps(run), flush=True)
medians = {name: statistics.median(r['processCpuSeconds'] for r in runs if r['variant'] == name) for name in variants}
receipt = {
    'scope': 'Original256face/2shell full global ABBA, complete report equality. Concurrent native jobs; no universal or UI deadline proof.',
    'modelSha256': sha(model), 'runs': runs, 'processCpuMedians': medians,
    'candidateToBaselineRatio': medians['R'] / medians['P'], 'completeReportParity': True,
    'productDeadlineQualified': False, 'integrated': False,
}
(directory / 'full-global-paired-benchmark.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(medians), flush=True)
