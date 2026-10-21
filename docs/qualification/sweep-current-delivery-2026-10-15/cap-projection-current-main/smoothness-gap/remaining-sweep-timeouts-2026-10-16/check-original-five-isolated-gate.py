"""Six original tests: five reported timeouts plus the unmodified periodic baseline."""
from pathlib import Path
import json,hashlib,os,time,subprocess
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-shared-boundary-integer-norm-delivery-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();f=out/'original-five-finalnorm-focus-receipt.json';assert not f.exists();r={'state':'waiting-for-stable-cpu','passed':False,'pid':os.getpid(),'packedSha256':sha(stage/'public/wasm/geometry-kernel.wasm'),'driverSha256':sha(__file__),'originalTestAndHookTimeoutMs':30000,'fullGoalComplete':False,'mainWasmModified':False}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
save();stable=None
while True:
 load=os.getloadavg()[0]
 if load<=10:
  if stable is None:stable=time.monotonic()
  if time.monotonic()-stable>=30:break
 else:stable=None
 r['lastObservedLoad']=list(os.getloadavg());save();time.sleep(5)
assert sha(stage/'public/wasm/geometry-kernel.wasm')==r['packedSha256'];assert sha(stage/'src/generated/geometry-kernels/kernel_bg.wasm')==r['packedSha256']
config=(stage/'vitest.config.ts').read_text();assert 'testTimeout: 30000' in config and 'hookTimeout: 30000' in config
pattern='arc-length-closed-corrected-frenet-nonplanar-body-boundary[.]r|open-spatial-rmf-varying-affine-hollow-corrected-body[.]r|periodic-fixed-normal-arc-'
report=out/'original-five-finalnorm-focus.json';argv=[node,'node_modules/vitest/vitest.mjs','run','tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts','--no-file-parallelism','--testNamePattern='+pattern,'--reporter=default','--reporter=json','--outputFile='+str(report)]
r.update(state='running',launchLoad=list(os.getloadavg()),argv=argv);save();print('Six unchanged source tests starting at stable CPU',flush=True)
with (out/'original-five-finalnorm-focus.log').open('w') as log:result=subprocess.run(argv,cwd=stage,env={**os.environ,'VITEST_MAX_WORKERS':'1'},stdout=log,stderr=subprocess.STDOUT)
j=json.loads(report.read_text());selected=[a for t in j['testResults'] for a in t['assertionResults'] if a['status']!='skipped'];assert len(selected)==6
r['selected']=[{'name':a['fullName'],'status':a['status'],'durationMs':a['duration']} for a in selected]
r.update(state='complete' if result.returncode==0 else 'failed',passed=result.returncode==0 and all(a['status']=='passed' and a['duration']<30000 for a in selected),returncode=result.returncode,reportSha256=sha(report),finalLoad=list(os.getloadavg()));save();raise SystemExit(0 if r['passed'] else 1)
