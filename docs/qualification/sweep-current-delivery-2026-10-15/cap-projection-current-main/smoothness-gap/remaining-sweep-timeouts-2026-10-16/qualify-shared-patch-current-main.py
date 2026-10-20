"""Build and test current-main inputs without overwriting live runtime assets."""
import hashlib,json,os,subprocess,time
from pathlib import Path
out=Path(__file__).resolve().parent
stage=Path('/private/tmp/osv-sweep-shared-patch-current-main-source-20261016')
node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger_path=stage/'current-main-qualified-source-sha256.json';ledger=json.loads(ledger_path.read_text())
f=out/'shared-patch-current-main-validation.json';assert not f.exists(),'Preserve prior attempts'
packed=stage/'public/wasm/geometry-kernel.wasm';raw=stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'
r={'schema':'current-main-shared-patch-runtime/1','state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'sourceLedgerSha256':sha(ledger_path),'qaOverridesSha256':sha(stage/'current-main-qa-overrides.json'),'sourceFiles':len(ledger),'phases':[],'originalTestAndHookTimeoutMs':30000,'originalStrictAssetLimit':8080000,'nodeVersion':subprocess.check_output([node,'--version'],text=True).strip(),'buildingArtifact':True,'packedSha256':sha(packed),'rawSha256':sha(raw),'mainWasmModified':False,'fullGoalComplete':False,'deadlineClosureConfirmed':False}
env={**os.environ,'VITEST_MAX_WORKERS':'1','CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','BINARYEN_CORES':'2','OSV_WASM_OPT_NATIVE':'/Users/themoretheless/.cargo/bin/wasm-opt','NODE_OPTIONS':'--max-old-space-size=8192'}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
def identity():
 assert sha(__file__)==r['driverSha256']
 assert sha(ledger_path)==r['sourceLedgerSha256']
 for name,digest in ledger.items():
  if name.startswith(('src/generated/geometry-kernels/','src/generated/wasm-brotli/')) or name=='public/wasm/geometry-kernel.wasm':continue
  assert sha(stage/name)==digest,name
 if not r['buildingArtifact']:
  assert sha(packed)==r['packedSha256'] and sha(raw)==r['rawSha256']
  assert sha(stage/'src/generated/geometry-kernels/kernel_bg.wasm')==r['packedSha256']
 assert 'const totalBudget = 8_080_000' in (stage/'scripts/verify-dist.mjs').read_text()
 assert 'const geometryChunkBudget = 3_970_000' in (stage/'scripts/verify-dist.mjs').read_text()
def run(name,argv,required=True,timing=False):
 identity();phase={'name':name,'state':'running','argv':argv};r['phases'].append(phase);save();print(name+': started',flush=True)
 if timing:
  started=time.monotonic()
  while True:
   prior=json.loads((out/'shared-patch-five-ready-receipt.json').read_text())
   if prior['state'] in ['complete','failed'] and os.getloadavg()[0]<=os.cpu_count():break
   phase.update(state='waiting-before-launch',lastObservedLoad=list(os.getloadavg()),priorFiveState=prior['state']);save();time.sleep(5)
  phase.update(state='running',preLaunchSchedulingSeconds=time.monotonic()-started,launchLoad=list(os.getloadavg()));save()
 started=time.monotonic()
 with (out/(name+'.log')).open('w') as log:result=subprocess.run(argv,cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
 identity();phase.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-started,finalLoad=list(os.getloadavg()));save();print(name+': '+phase['state'],flush=True)
 if result.returncode and required:
  r['state']='failed';save();raise SystemExit(result.returncode)
 return result.returncode==0
save()
run('current-main-wasm',[node,'scripts/build-geometry-kernels.mjs'])
r.update(packedSha256=sha(packed),rawSha256=sha(raw),buildingArtifact=False);save()
run('current-main-vue-typecheck',[node,'node_modules/vue-tsc/bin/vue-tsc.js','--noEmit'])
run('current-main-mcp-typecheck',[node,'node_modules/typescript/bin/tsc','--noEmit','-p','tsconfig.mcp.json'])
run('current-main-application',[node,'node_modules/vite/bin/vite.js','build'])
size=run('current-main-strict-distribution',[node,'scripts/verify-dist.mjs'],required=False)
r['strictDistributionPassed']=size;save()
pattern='arc-length-closed-corrected-frenet-nonplanar-body-boundary[.]r|periodic-fixed-normal-arc-(plain-hollow|affine-(solid|hollow))-station-g2[.]r|open-spatial-rmf-varying-affine-hollow-corrected-body[.]r'
files=['tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts']
run('current-main-five',[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--testNamePattern='+pattern,'--reporter=default','--reporter=json','--outputFile='+str(out/'current-main-five.json')],timing=True)
five=json.loads((out/'current-main-five.json').read_text());assert five['numPassedTests']==5 and five['numFailedTests']==0
r['fiveCases']=[{'name':a['fullName'],'durationMs':a['duration']} for t in five['testResults'] for a in t['assertionResults'] if a['status']=='passed'];save()
original=out.parent/'current-source-build-arc-partition/public100-serial/receipt.json';old=json.loads(original.read_text());files=[x for x in old['argv'] if x.startswith('tests/') and x.endswith('.test.ts')];assert len(set(files))==len(files)==100
run('current-main-public100',[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--reporter=default','--reporter=json','--outputFile='+str(out/'current-main-public100.json')],timing=True)
j=json.loads((out/'current-main-public100.json').read_text());assert j['numFailedTests']==0 and j['numPendingTests']==0
r.update(state='complete',passed=size,deadlineClosureConfirmed=True,public100={'total':j['numTotalTests'],'passed':j['numPassedTests'],'failed':j['numFailedTests'],'pending':j['numPendingTests'],'reportSha256':sha(out/'current-main-public100.json')},fiveReportSha256=sha(out/'current-main-five.json'));save();print(json.dumps({'deadlineClosureConfirmed':True,'strictDistributionPassed':size,'public100':r['public100']}),flush=True)
