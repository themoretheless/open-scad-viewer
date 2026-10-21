"""One actual-main runtime pipeline; no simultaneous legacy deadline checks."""
import os,time,json,hashlib,subprocess
from pathlib import Path
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-main-shared-boundary-integer-delivery-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
f=out/'current-main-combined-qualification.json';assert not f.exists();r={'state':'waiting-for-builds','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'phases':[],'originalTestAndHookTimeoutMs':30000,'mainWasmModified':False,'fullGoalComplete':False}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
save()
while True:
 build=json.loads((out/'main-shared-boundary-integer-staged-build.json').read_text())
 owned=json.loads((out/'shared-boundary-integer-artifact-validation.json').read_text())
 if build['state'] in ['complete','failed'] and owned['state'] in ['complete','failed']:break
 time.sleep(5)
assert build['state']=='complete' and not build['sourceChangedDuringBuild'];assert owned['state']=='complete' and owned['passed']
ledger_path=stage/'runtime-source-sha256.json'
while not ledger_path.exists():time.sleep(2)
ledger=json.loads(ledger_path.read_text());native=json.loads((stage/'main-native-source-sha256.json').read_text())
for name,digest in native.items():assert sha(stage/name)==digest,name
r.update(state='running',packedSha256=build['packedSha256'],rawSha256=build['rawSha256'],sourceFiles=len(ledger),sourceLedgerSha256=sha(ledger_path),mainVerifierSha256=sha(stage/'scripts/verify-dist.mjs'),mainVerifierUnchangedFromSnapshot=True)
def identity():
 assert sha(stage/'public/wasm/geometry-kernel.wasm')==r['packedSha256']
 assert sha(stage/'src/generated/geometry-kernels/kernel_bg.wasm')==r['packedSha256']
 for name,digest in ledger.items():assert sha(stage/name)==digest,name
 text=(stage/'vitest.config.ts').read_text();assert 'testTimeout: 30000' in text and 'hookTimeout: 30000' in text
 assert sha(stage/'scripts/verify-dist.mjs')==r['mainVerifierSha256']
def run(name,argv):
 identity();item={'name':name,'argv':argv,'state':'running'};r['phases'].append(item);save();print(name+': started',flush=True)
 if 'public' in name:
  while os.getloadavg()[0]>os.cpu_count():item.update(state='waiting-before-launch',lastObservedLoad=list(os.getloadavg()));save();time.sleep(5)
  item.update(state='running',launchLoad=list(os.getloadavg()));save()
 start=time.monotonic()
 with (out/(name+'.log')).open('w') as log:result=subprocess.run(argv,cwd=stage,env={**os.environ,'VITEST_MAX_WORKERS':'1'},stdout=log,stderr=subprocess.STDOUT)
 identity();item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-start);save();print(name+': '+item['state'],flush=True)
 if result.returncode:r.update(state='failed',passed=False);save();raise SystemExit(result.returncode)
run('combined-main-vue-typecheck',[node,'node_modules/vue-tsc/bin/vue-tsc.js','--noEmit'])
run('combined-main-mcp-typecheck',[node,'node_modules/typescript/bin/tsc','--noEmit','-p','tsconfig.mcp.json'])
run('combined-main-application',[node,'node_modules/vite/bin/vite.js','build'])
run('combined-main-distribution',[node,'scripts/verify-dist.mjs'])
assert sha(stage/'dist/wasm/geometry-kernel.wasm')==r['packedSha256']
report=out/'combined-main-original31.json';run('combined-main-original31-public',[node,'node_modules/vitest/vitest.mjs','run','tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts','--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(report)])
j=json.loads(report.read_text());assert j['numTotalTests']==j['numPassedTests']==31 and j['numFailedTests']==j['numPendingTests']==0
patterns=['arc-length-closed-corrected-frenet-nonplanar-body-boundary.r','open-spatial-rmf-varying-affine-hollow-corrected-body.r','periodic-fixed-normal-arc-plain-hollo','periodic-fixed-normal-arc-affine-soli','periodic-fixed-normal-arc-affine-holl'];r['originalFive']={}
for pattern in patterns:
 cases=[a for t in j['testResults'] for a in t['assertionResults'] if pattern in a['fullName']];assert len(cases)==1;case=cases[0];assert case['status']=='passed' and case['duration']<30000;r['originalFive'][pattern]={'status':case['status'],'durationMs':case['duration']}
r['deadlineClosureConfirmed']=True;save()
run('combined-main-affine-public',[node,'node_modules/vitest/vitest.mjs','run','tests/sweepAffineLattice.test.ts','--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(out/'combined-main-affine.json')])
old=json.loads((out.parent/'current-source-build-arc-partition/public100-serial/receipt.json').read_text());files=[x for x in old['argv'] if x.startswith('tests/') and x.endswith('.test.ts')];assert len(files)==len(set(files))==100
run('combined-main-public100',[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(out/'combined-main-public100.json')])
j=json.loads((out/'combined-main-public100.json').read_text());assert j['numTotalTests']>=736 and j['numTotalTests']==j['numPassedTests'] and j['numFailedTests']==j['numPendingTests']==0
r.update(state='complete',passed=True,originalInventoryFiles=100,baselineInventoryTests=736,actualCurrentPublicTests=j['numTotalTests'],deliveredToMain=False);save();print('Actual-main checks passed; guarded publication still required.',flush=True)
