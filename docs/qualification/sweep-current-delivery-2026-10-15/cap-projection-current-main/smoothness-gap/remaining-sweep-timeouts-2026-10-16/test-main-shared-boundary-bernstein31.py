"""Actual current-main staged runtime, all31 cases and unchanged30s timers."""
import json,hashlib,os,subprocess,time
from pathlib import Path
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-shared-boundary-bernstein-delivery-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
while True:
 build=json.loads((out/'main-shared-boundary-bernstein-staged-build.json').read_text())
 if build['state'] in ['complete','failed']:break
 time.sleep(5)
assert build['state']=='complete' and not build['sourceChangedDuringBuild']
native=json.loads((stage/'main-native-source-sha256.json').read_text());
for name,digest in native.items():assert sha(stage/name)==digest,name
ledger_path=stage/'runtime-source-sha256.json';ledger=json.loads(ledger_path.read_text());f=out/'main-shared-boundary-bernstein31-receipt.json';assert not f.exists()
r={'schema':'current-main-original31-deadlines/1','state':'waiting-before-launch','passed':False,'pid':os.getpid(),'packedSha256':build['packedSha256'],'rawSha256':build['rawSha256'],'driverSha256':sha(__file__),'runtimeSourceLedgerSha256':sha(ledger_path),'buildReceiptSha256':sha(out/'main-shared-boundary-bernstein-staged-build.json'),'originalTestAndHookTimeoutMs':30000,'fullGoalComplete':False,'mainWasmModified':False}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
def identity():
 assert sha(stage/'public/wasm/geometry-kernel.wasm')==r['packedSha256']
 assert sha(stage/'src/generated/geometry-kernels/kernel_bg.wasm')==r['packedSha256']
 for name,digest in ledger.items():assert sha(stage/name)==digest,name
save();identity();start=time.monotonic()
while os.getloadavg()[0]>os.cpu_count():r['lastObservedLoad']=list(os.getloadavg());save();time.sleep(5)
r.update(state='running',schedulingWaitSeconds=time.monotonic()-start,launchLoad=list(os.getloadavg()));save()
report=out/'main-shared-boundary-bernstein31.json';argv=[node,'node_modules/vitest/vitest.mjs','run','tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts','--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(report)];r['argv']=argv;save();start=time.monotonic()
with (out/'main-shared-boundary-bernstein31.log').open('w') as log:result=subprocess.run(argv,cwd=stage,env={**os.environ,'VITEST_MAX_WORKERS':'1'},stdout=log,stderr=subprocess.STDOUT)
identity();j=json.loads(report.read_text());r.update(state='complete' if result.returncode==0 else 'failed',passed=result.returncode==0,numTotalTests=j['numTotalTests'],numPassedTests=j['numPassedTests'],numFailedTests=j['numFailedTests'],numPendingTests=j['numPendingTests'],seconds=time.monotonic()-start,reportSha256=sha(report));save();print(json.dumps(r),flush=True);raise SystemExit(result.returncode)
