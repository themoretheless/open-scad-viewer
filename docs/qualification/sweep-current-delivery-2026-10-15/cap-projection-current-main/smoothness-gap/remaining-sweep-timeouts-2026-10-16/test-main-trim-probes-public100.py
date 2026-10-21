"""Original100-file inventory on the final current-main staged kernel."""
import json,hashlib,os,subprocess,time
from pathlib import Path
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-trim-probes-delivery-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
while True:
 p=out/'main-trim-probes31-receipt.json'
 if p.exists():
  prior=json.loads(p.read_text())
  if prior['state'] in ['complete','failed']:break
 time.sleep(5)
assert prior['passed'],'Current-main31failed; do not launch public100'
ledger_path=stage/'runtime-source-sha256.json';ledger=json.loads(ledger_path.read_text());f=out/'main-trim-probes-public100-receipt.json';assert not f.exists()
original=out.parent/'current-source-build-arc-partition/public100-serial/receipt.json';files=[x for x in json.loads(original.read_text())['argv'] if x.startswith('tests/') and x.endswith('.test.ts')];assert len(set(files))==len(files)==100
r={'schema':'current-main-original-public100/1','state':'waiting-before-launch','passed':False,'pid':os.getpid(),'packedSha256':prior['packedSha256'],'rawSha256':prior['rawSha256'],'driverSha256':sha(__file__),'runtimeSourceLedgerSha256':sha(ledger_path),'requiredFiles':100,'originalTestAndHookTimeoutMs':30000,'fullGoalComplete':False,'mainWasmModified':False}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
def identity():
 assert sha(stage/'public/wasm/geometry-kernel.wasm')==r['packedSha256']
 for name,digest in ledger.items():assert sha(stage/name)==digest,name
save();identity();start=time.monotonic()
while os.getloadavg()[0]>os.cpu_count():r['lastObservedLoad']=list(os.getloadavg());save();time.sleep(5)
r.update(state='running',preLaunchSchedulingSeconds=time.monotonic()-start,launchLoad=list(os.getloadavg()));save()
report=out/'main-trim-probes-public100.json';argv=[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(report)];r['argv']=argv;save();start=time.monotonic()
with (out/'main-trim-probes-public100.log').open('w') as log:result=subprocess.run(argv,cwd=stage,env={**os.environ,'VITEST_MAX_WORKERS':'1','NODE_OPTIONS':'--max-old-space-size=8192'},stdout=log,stderr=subprocess.STDOUT)
identity();j=json.loads(report.read_text());r.update(state='complete' if result.returncode==0 else 'failed',passed=result.returncode==0,numTotalTests=j['numTotalTests'],numPassedTests=j['numPassedTests'],numFailedTests=j['numFailedTests'],numPendingTests=j['numPendingTests'],seconds=time.monotonic()-start,reportSha256=sha(report));save();print(json.dumps(r),flush=True);raise SystemExit(result.returncode)
