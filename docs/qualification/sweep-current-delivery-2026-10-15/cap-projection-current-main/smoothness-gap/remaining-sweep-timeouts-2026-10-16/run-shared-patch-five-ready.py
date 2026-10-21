"""Fresh original five-case deadline gate; preserve the previous failed run."""
import hashlib,json,os,subprocess,time
from pathlib import Path
out=Path(__file__).resolve().parent
stage=Path('/private/tmp/osv-sweep-adaptive-hull-source-20261016')
node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
gate=json.loads((out/'shared-patch-artifact-validation.json').read_text())
assert all(p['state']=='passed' for p in gate['phases'][:-1])
assert gate['phases'][-1]['name']=='shared-patch-affine-public'
assert gate['buildingArtifact']==False
ledger=json.loads((stage/'shared-patch-source-sha256.json').read_text())
files=['tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts']
pattern='arc-length-closed-corrected-frenet-nonplanar-body-boundary[.]r|periodic-fixed-normal-arc-(plain-hollow|affine-(solid|hollow))-station-g2[.]r|open-spatial-rmf-varying-affine-hollow-corrected-body[.]r'
f=out/'shared-patch-five-ready-receipt.json';assert not f.exists(),'Preserve previous measurements'
r={'schema':'original-five-deadlines/1','state':'waiting-for-cpu-before-launch','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'originalTestAndHookTimeoutMs':30000,'packedSha256':gate['packedSha256'],'rawSha256':gate['rawSha256'],'sourceLedgerSha256':sha(stage/'shared-patch-source-sha256.json'),'priorFailedValidationSha256':sha(out/'shared-patch-artifact-validation.json'),'nodeVersion':subprocess.check_output([node,'--version'],text=True).strip(),'originalCases':5,'fullGoalComplete':False,'mainWasmModified':False}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
def identity():
 assert sha(stage/'public/wasm/geometry-kernel.wasm')==gate['packedSha256']
 for name in files:assert sha(stage/name)==ledger[name],name
 for name in ['crates/nurbs-core/src/distance/surface_distance.rs','crates/nurbs-core/src/intersection/surface_contact.rs','crates/nurbs-core/src/intersection/surface_contact_search.rs']:assert sha(stage/name)==ledger[name],name
identity();started=time.monotonic()
while os.getloadavg()[0]>os.cpu_count():
 r['lastObservedLoad']=list(os.getloadavg());save();time.sleep(5)
r.update(state='running',schedulingWaitSeconds=time.monotonic()-started,launchLoad=list(os.getloadavg()))
report=out/'shared-patch-five-ready.json';argv=[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--testNamePattern='+pattern,'--reporter=default','--reporter=json','--outputFile='+str(report)];r['argv']=argv;save();identity();started=time.monotonic()
with (out/'shared-patch-five-ready.log').open('w') as log:
 result=subprocess.run(argv,cwd=stage,env={**os.environ,'VITEST_MAX_WORKERS':'1'},stdout=log,stderr=subprocess.STDOUT)
identity();j=json.loads(report.read_text());passed=[{'name':a['fullName'],'durationMs':a['duration']} for t in j['testResults'] for a in t['assertionResults'] if a['status']=='passed'];r.update(state='complete' if result.returncode==0 else 'failed',passed=result.returncode==0 and len(passed)==5,numPassedTests=j['numPassedTests'],numFailedTests=j['numFailedTests'],cases=passed,returncode=result.returncode,seconds=time.monotonic()-started,finalLoad=list(os.getloadavg()),reportSha256=sha(report));save();print(json.dumps({k:r[k] for k in ['state','passed','cases']}),flush=True);raise SystemExit(0 if r['passed'] else 1)
