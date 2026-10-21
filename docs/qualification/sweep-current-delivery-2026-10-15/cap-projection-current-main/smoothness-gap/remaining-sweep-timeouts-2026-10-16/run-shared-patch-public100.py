"""The original100-file/736-case inventory, unchanged time limits and sources."""
import hashlib,json,os,subprocess,time
from pathlib import Path
out=Path(__file__).resolve().parent
stage=Path('/private/tmp/osv-sweep-adaptive-hull-source-20261016')
node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
original=out.parent/'current-source-build-arc-partition/public100-serial/receipt.json'
files=[x for x in json.loads(original.read_text())['argv'] if x.startswith('tests/') and x.endswith('.test.ts')]
assert len(files)==len(set(files))==100
receipt_path=out/'shared-patch-public100-receipt.json'
assert not receipt_path.exists(),'Preserve previous runs'
while True:
    gate=json.loads((out/'shared-patch-artifact-validation.json').read_text())
    if gate['state']=='complete':break
    if gate['state']=='failed':raise SystemExit('Focused qualification failed; do not launch public100')
    time.sleep(2)
assert gate['passed']
ledger=json.loads((stage/'shared-patch-source-sha256.json').read_text())
def identity():
    assert sha(stage/'public/wasm/geometry-kernel.wasm')==gate['packedSha256']
    for f in files:assert sha(stage/f)==ledger[f],f
    for f in ['crates/nurbs-core/src/distance/surface_distance.rs','crates/nurbs-core/src/intersection/surface_contact.rs','crates/nurbs-core/src/intersection/surface_contact_search.rs']:
        assert sha(stage/f)==ledger[f],f
identity()
r={'schema':'unchanged-public100-shared-patches/1','state':'waiting-for-cpu-before-launch','passed':False,'geometryWasmSha256':gate['packedSha256'],'rawSha256':gate['rawSha256'],'node':node,'nodeVersion':subprocess.check_output([node,'--version'],text=True).strip(),'originalInventoryReceiptSha256':sha(original),'validationReceiptSha256':sha(out/'shared-patch-artifact-validation.json'),'driverSha256':sha(__file__),'sourceLedgerSha256':sha(stage/'shared-patch-source-sha256.json'),'requiredFiles':100,'expectedTests':736,'originalTestAndHookTimeoutMs':30000,'mainWasmModified':False,'fullGoalComplete':False}
def save():
    q=receipt_path.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(receipt_path)
started=time.monotonic()
while os.getloadavg()[0]>os.cpu_count():
    r['lastObservedLoad']=list(os.getloadavg());save();time.sleep(5)
r['preLaunchSchedulingSeconds']=time.monotonic()-started
r['launchLoad']=list(os.getloadavg())
r['state']='running'
report=out/'shared-patch-public100.json'
argv=[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=json','--outputFile='+str(report)]
r['argv']=argv;save();identity();started=time.monotonic()
env={**os.environ,'VITEST_MAX_WORKERS':'1','NODE_OPTIONS':'--max-old-space-size=8192'}
with (out/'shared-patch-public100.log').open('w') as log:
    result=subprocess.run(argv,cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
identity()
j=json.loads(report.read_text())
r.update(state='complete' if result.returncode==0 else 'failed',passed=result.returncode==0,returncode=result.returncode,seconds=time.monotonic()-started,reportSha256=sha(report),numTotalTests=j['numTotalTests'],numPassedTests=j['numPassedTests'],numFailedTests=j['numFailedTests'],numPendingTests=j['numPendingTests'])
assert j['numTotalTests']==736
save();print(json.dumps({k:r[k] for k in ['state','numTotalTests','numPassedTests','numFailedTests','numPendingTests']}),flush=True)
raise SystemExit(result.returncode)
