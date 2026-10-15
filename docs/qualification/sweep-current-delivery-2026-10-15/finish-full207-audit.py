"""Finish independent inventory/control audits after the existing six phases."""
import json
import os
from pathlib import Path
import subprocess
import time

stage=Path(__file__).parent
output=Path('/private/tmp/osv-sweep-current-full207-run2-20261015')
previous=output/'qualification-receipt.json'
receipt={'state':'waiting-existing-independent-checks','passed':False,'phases':[]}
def save():
    p=output/'supplemental-audit.json.writing'
    p.write_text(json.dumps(receipt,indent=2)+'\n')
    p.replace(output/'supplemental-audit.json')
save()
while True:
    data=json.loads(previous.read_text())
    if data['state']=='complete':
        assert data['passed'] is True
        break
    if data['state'] in ['failed','export-ended-without-manifest']:
        receipt['state']='blocked-by-prior-failure';save()
        raise RuntimeError('Existing export/independent phase failed; no retry')
    try:os.kill(94515,0)
    except ProcessLookupError:
        receipt['state']='prior-process-ended-without-complete-receipt';save()
        raise RuntimeError('Existing driver ended without a complete receipt')
    time.sleep(5)
manifest=str(output/'manifest.json')
commands=[
    ('inventory',['node','scripts/verify-sweep-current-inventory-207.mjs',manifest,'original-step-inventory.json',str(stage),str(output/'current-inventory.json')]),
    ('pair-cover',['node','scripts/verify-sweep-face-pair-cover.mjs',manifest,str(output/'face-pair-cover.json')]),
    ('smoothness-controls',['node','scripts/verify-sweep-smoothness-refusal-controls.mjs',manifest,str(output/'smoothness-refusal-controls.json')]),
    ('original-source-preservation',['node','scripts/verify-sweep-original-request-preservation.mjs',manifest,'original-request-baseline.json',str(output/'original157-preservation.json'),'original-annular-walls.json']),
]
receipt['state']='running';save()
for name,argv in commands:
    phase={'name':name,'argv':argv,'state':'running'};receipt['phases'].append(phase);save()
    print(name+': started',flush=True)
    with (output/(name+'.log')).open('w') as log:
        result=subprocess.run(argv,cwd=stage,env={**os.environ,'NODE_OPTIONS':'--max-old-space-size=8192'},stdout=log,stderr=subprocess.STDOUT)
    phase.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode);save()
    print(name+': '+phase['state'],flush=True)
    if result.returncode:
        receipt['state']='failed';save();raise RuntimeError(name+' failed; no relaxed acceptance')
receipt.update(state='complete',passed=True);save()
print('Full finite207 audit and explicit original-source preservation passed',flush=True)
