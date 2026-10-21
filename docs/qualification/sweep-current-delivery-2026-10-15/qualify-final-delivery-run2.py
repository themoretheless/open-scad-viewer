import json,os,subprocess,time
from pathlib import Path

stage=Path(__file__).parent
output=Path('/private/tmp/osv-sweep-final-gates-run2-20261015')
output.mkdir(exist_ok=True)
receipt={'scope':'Final frozen source/artifact gates; full declared-mode STEP/UI still separate','passed':False,'phases':[]}
def save():
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
save()
commands=[
    ('raw',['python3','qualify-final-raw-resume.py']),
    ('packed',['python3','qualify-final-packed.py']),
    ('production',['npm','--ignore-scripts','run','build']),
    ('rush',['npx','vitest','run','tests/nurbsAutomaticCapsQualification.test.ts','tests/nurbsAnnularStationQualification.test.ts','--project=node','--maxWorkers=1','--no-file-parallelism']),
]
for name,args in commands:
    phase={'name':name,'state':'running','argv':args};receipt['phases'].append(phase);save()
    print(name+': started',flush=True);started=time.monotonic()
    with (output/(name+'.log')).open('w') as log:
        result=subprocess.run(args,cwd=stage,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'},stdout=log,stderr=subprocess.STDOUT)
    phase.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-started);save()
    print(name+': '+phase['state'],flush=True)
    if result.returncode:raise RuntimeError(name+' failed; retained log, no retry or gate changes')
receipt['passed']=True;save()
print('Final exact raw/packed ABI, independent STEP, strict production and Rush gates passed',flush=True)
