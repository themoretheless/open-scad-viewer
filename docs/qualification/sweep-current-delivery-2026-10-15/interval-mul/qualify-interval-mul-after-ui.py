"""Wait for the existing focused UI, then qualify the exact frozen artifacts."""
from pathlib import Path
import hashlib,json,os,subprocess,time
stage=Path(__file__).parent
output=Path('/private/tmp/osv-sweep-interval-mul-artifact-qualification-20261015');output.mkdir(exist_ok=True)
ui=Path('/private/tmp/osv-sweep-interval-mul-closed-hollow-ui-run2-20261015/matrix.json')
receipt={'state':'waiting-existing-focused-ui','passed':False,'phases':[]}
def save():
 p=output/'receipt.json.writing';p.write_text(json.dumps(receipt,indent=2)+'\n');p.replace(output/'receipt.json')
save()
while True:
 report=json.loads(ui.read_text()) if ui.exists() else {}
 if report.get('passed') is True:break
 if any(c.get('status')=='failed' for c in report.get('cases',[])):
  receipt['state']='stopped-after-focused-ui-failure';save();raise SystemExit(1)
 try:os.kill(84300,0)
 except ProcessLookupError:
  receipt['state']='focused-ui-ended-without-complete-pass';save();raise SystemExit(1)
 time.sleep(5)
assert report['headed'] is False and len(report['cases'])==2
artifacts=json.loads((stage/'completed-build-receipt.json').read_text())['artifacts']
assert report['geometryWasmSha256']==artifacts['packed']['sha256']
receipt.update(state='running',artifacts=artifacts,scope='Exact raw and packed artifact native ABI/source/STEP/transport gates; finite coverage, full207 and478 remain separate');save()
python='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
native='/private/tmp/osv-sweep-auto-caps-native-20261014'
def run(name,argv):
 phase={'name':name,'state':'running','argv':list(map(str,argv))};receipt['phases'].append(phase);save();print(name+': started',flush=True);t=time.monotonic()
 with (output/(name+'.log')).open('w') as log:
  result=subprocess.run(list(map(str,argv)),cwd=stage,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'},stdout=log,stderr=subprocess.STDOUT)
 phase.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-t);save();print(name+': '+phase['state'],flush=True)
 if result.returncode:receipt['state']='failed';save();raise SystemExit(result.returncode)
for kind,flags in [('raw',['--raw-built']),('packed',[])]:
 target=output/kind
 run(kind+'-station-abi',['node','--import','tsx','scripts/check-staged-auto-station-caps.mts',stage,target/'station'/'receipt.json',native,*flags])
 for mode,extra in [('automatic',['--automatic-caps']),('explicit',[])]:
  folder=target/mode
  run(kind+'-'+mode+'-abi',['node','--import','tsx','scripts/check-staged-smooth-annular-kernel.mts',stage,folder/'abi.json',native,*flags,*extra,'--caps-first'])
  run(kind+'-'+mode+'-reference',[python,'scripts/reference-rational-boundary-volume.py',folder])
  run(kind+'-'+mode+'-occt',[python,'scripts/verify-sweep-step-occt.py',folder])
 folder=target/'legacy'
 run(kind+'-legacy-abi',['node','--import','tsx','scripts/check-staged-annular-miter-kernel.mts',stage,folder/'abi.json','--joint-material-step','--unbudgeted-source',*flags])
 run(kind+'-legacy-reference',[python,'scripts/reference-rational-boundary-volume.py',folder])
 run(kind+'-legacy-occt',[python,'scripts/verify-sweep-step-occt.py',folder])
run('rush',['node','node_modules/vitest/vitest.mjs','run','tests/nurbsAutomaticCapsQualification.test.ts','tests/nurbsAnnularStationQualification.test.ts','--project=node','--maxWorkers=1','--no-file-parallelism'])
receipt.update(state='complete',passed=True);save()
