"""Resume unchanged V7 artifact; compiler nice=19, original test priority restored."""
from pathlib import Path
import json,hashlib,subprocess,os,time
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-boolean-repair-v7-source-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();f=out/'v7-runtime-normal-priority-20261018.json';assert not f.exists();r={'state':'waiting-provider-baseline','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'phases':[],'published':False,'fullGoalComplete':False,'originalTestAndHookTimeoutMs':30000,'testPriority':'original-normal','compilerNice':19,'reason':'Prior V7 driver mistakenly niced all runtime tests; V5 qualification used normal priority. No source/kernel/tolerance/work/product deadline changes.'}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
save()
try:
 prepath=out/'v7-current-native-profile-20261018-receipt.json'
 while True:
  pre=json.loads(prepath.read_text())
  if pre['state'] in ['complete','failed']:break
  time.sleep(5)
 if not pre['passed']:r.update(state='stopped-after-provider-baseline',prerequisiteSha256=sha(prepath));save();raise SystemExit(1)
 previous=out/'v7-repair-delivery-runtime-retry3.json';prior=json.loads(previous.read_text());buildpath=out/'frozen-current-v7-build.json';build=json.loads(buildpath.read_text());assert build['passed'];assert prior['state']=='failed' and prior['reason']=='original31 failed; inspect preserved log'
 r.update(priorRuntimeReceiptSha256=sha(previous),prerequisiteSha256=sha(prepath),providerBaselinePassed=pre['providerPassed'],providerBaselineFiles=len(pre['testFiles']),buildReceiptSha256=sha(buildpath),packedSha256=build['packedSha256'],rawSha256=build['rawSha256'],nativeLedgerSha256=build['nativeLedgerSha256'],sourceLedgerSha256=sha(stage/'runtime-source-sha256.json'),externalLedgerSha256=sha(stage/'qualification-external-inputs.json'),nativeProducerSha256=build['nativeProducerSha256'])
 native=json.loads((stage/'main-native-source-sha256.json').read_text());runtime=json.loads((stage/'runtime-source-sha256.json').read_text());extra=json.loads((stage/'qualification-external-inputs.json').read_text())
 def identity():
  assert sha(__file__)==r['driverSha256'];assert sha(previous)==r['priorRuntimeReceiptSha256'];assert sha(prepath)==r['prerequisiteSha256'];assert sha(buildpath)==r['buildReceiptSha256']
  assert sha(stage/'main-native-source-sha256.json')==r['nativeLedgerSha256'];assert sha(stage/'runtime-source-sha256.json')==r['sourceLedgerSha256'];assert sha(stage/'qualification-external-inputs.json')==r['externalLedgerSha256']
  assert all(sha(stage/n)==v for n,v in native.items());assert all(sha(stage/n)==v for n,v in runtime.items());assert all(sha(stage/n)==v['sha256'] for n,v in extra['files'].items())
  assert all(sha(Path(extra['browserDependenciesRealPath'])/n)==h for n,h in extra['browserDependencyFiles'].items())
  assert sha(stage/'crates/target/release/examples/certify_sweep_geometry_migration')==r['nativeProducerSha256']
  for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(stage/n)==r['packedSha256']
  config=(stage/'vitest.config.ts').read_text();assert 'testTimeout: 30000' in config and 'hookTimeout: 30000' in config
 def run(name,argv):
  identity();since=None;item={'name':name,'argv':argv,'state':'waiting-for-quiet-resources'};r['phases'].append(item);save()
  while True:
   now=time.monotonic();load=os.getloadavg()
   if load[0]<=8:
    since=now if since is None else since
    if now-since>=30:break
   else:since=None
   item['lastObservedLoad']=list(load);save();time.sleep(5)
  identity();item.update(state='running',launchLoad=list(os.getloadavg()));r['state']='running';save();print(name+': started',flush=True);start=time.monotonic()
  env={**os.environ,'PATH':str(Path(node).parent)+os.pathsep+os.environ['PATH'],'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','VITEST_MAX_WORKERS':'1','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','CARGO_NET_OFFLINE':'true'};env.pop('NODE_OPTIONS',None)
  with (out/('v7-normal-'+name+'-20261018.log')).open('w') as log:result=subprocess.run(argv,cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
  identity();item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-start);save();assert result.returncode==0,name+' failed; preserve log';print(name+': passed',flush=True)
 def vitest(name,files):
  report=out/('v7-normal-'+name+'-20261018.json');run(name,[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(report)]);j=json.loads(report.read_text());assert j['numTotalTests']==j['numPassedTests'] and j['numFailedTests']==j['numPendingTests']==0;return j
 j=vitest('original31',['tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts']);assert j['numPassedTests']==31;r['originalFive']={}
 for pattern in ['arc-length-closed-corrected-frenet-nonplanar-body-boundary.r','open-spatial-rmf-varying-affine-hollow-corrected-body.r','periodic-fixed-normal-arc-plain-hollo','periodic-fixed-normal-arc-affine-soli','periodic-fixed-normal-arc-affine-holl']:
  cases=[a for t in j['testResults'] for a in t['assertionResults'] if pattern in a['fullName']];assert len(cases)==1 and cases[0]['status']=='passed' and cases[0]['duration']<30000;r['originalFive'][pattern]={'status':'passed','durationMs':cases[0]['duration']}
 r['deadlineClosureConfirmed']=True;save();assert vitest('affine',['tests/sweepAffineLattice.test.ts'])['numPassedTests']==10
 files=sorted(json.loads((stage/'public-100-baseline-inventory.json').read_text())['files']);assert len(files)==100;j=vitest('public100',files);assert j['numPassedTests']>=736;r['public100Tests']=j['numPassedTests'];save()
 run('observer-unit',[node,'--test','scripts/test-sweep-native-cancel-probe.mjs']);r.update(state='complete',passed=True,qualificationRecordComplete=False);save()
except Exception as e:r.update(state='failed',passed=False,reason=str(e));save();raise
