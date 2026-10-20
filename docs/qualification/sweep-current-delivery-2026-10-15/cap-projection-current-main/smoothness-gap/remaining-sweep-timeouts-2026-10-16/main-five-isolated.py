"""Five original scenarios, separate processes and unchanged30s limits."""
import json,hashlib,os,time,subprocess
from pathlib import Path
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-trim-probes-delivery-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
cases=[('closed-corrected','tests/nurbsClosedCorrectedRegularBody.test.ts','arc-length-closed-corrected-frenet-nonplanar-body-boundary[.]r'),('open-rmf','tests/nurbsSpatialRmfError.test.ts','open-spatial-rmf-varying-affine-hollow-corrected-body[.]r'),('periodic-plain-hollow','tests/nurbsPeriodicFixedNormalArcRush.test.ts','periodic-fixed-normal-arc-plain-hollo'),('periodic-affine-solid','tests/nurbsPeriodicFixedNormalArcRush.test.ts','periodic-fixed-normal-arc-affine-soli'),('periodic-affine-hollow','tests/nurbsPeriodicFixedNormalArcRush.test.ts','periodic-fixed-normal-arc-affine-holl')]
f=out/'main-five-isolated-receipt.json';assert not f.exists();packed=sha(stage/'public/wasm/geometry-kernel.wasm');r={'state':'running','passed':False,'pid':os.getpid(),'packedSha256':packed,'originalTestAndHookTimeoutMs':30000,'driverSha256':sha(__file__),'mainWasmModified':False,'fullGoalComplete':False,'cases':[]}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
save()
for name,file,pattern in cases:
 item={'case':name,'state':'waiting-for-stable-cpu'};r['cases'].append(item);save();stable=None
 while True:
  load=os.getloadavg()[0]
  if load<=8:
   if stable is None:stable=time.monotonic()
   if time.monotonic()-stable>=30:break
  else:stable=None
  item['lastObservedLoad']=list(os.getloadavg());save();time.sleep(5)
 assert sha(stage/'public/wasm/geometry-kernel.wasm')==packed
 item.update(state='running',launchLoad=list(os.getloadavg()));save()
 report=out/('main-five-'+name+'.json');argv=[node,'node_modules/vitest/vitest.mjs','run',file,'--no-file-parallelism','--testNamePattern='+pattern,'--reporter=default','--reporter=json','--outputFile='+str(report)]
 with (out/('main-five-'+name+'.log')).open('w') as log:result=subprocess.run(argv,cwd=stage,env={**os.environ,'VITEST_MAX_WORKERS':'1'},stdout=log,stderr=subprocess.STDOUT)
 j=json.loads(report.read_text());selected=[a for t in j['testResults'] for a in t['assertionResults'] if a['status']!='skipped'];assert len(selected)==1
 item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,status=selected[0]['status'],durationMs=selected[0]['duration'],reportSha256=sha(report),finalLoad=list(os.getloadavg()));save();print(name,item['state'],item['durationMs'],flush=True)
r.update(state='complete',passed=all(c['state']=='passed' for c in r['cases']));save();raise SystemExit(0 if r['passed'] else 1)
