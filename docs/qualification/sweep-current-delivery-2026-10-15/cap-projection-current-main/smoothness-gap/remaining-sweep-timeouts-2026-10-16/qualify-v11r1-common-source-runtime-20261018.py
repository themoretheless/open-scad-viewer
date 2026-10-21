"""Serial source-bound successor. No publication or relaxed product limits."""
from pathlib import Path
import hashlib,json,os,subprocess,time
out=Path(__file__).resolve().parent
stage=Path('/private/tmp/osv-sweep-common-source-v11-20261018')
node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
f=out/'v11r1-common-source-runtime-20261018.json';assert not f.exists()
r={'state':'waiting-native-tests','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'phases':[],'stage':str(stage),'published':False,'fullGoalComplete':False,'resourceGate':{'runtimeMaxOneMinuteLoad':8,'runtimeStableSeconds':30,'compilerNice':19,'compilerMaxCargoJobs':2,'compilerWaitForMeasurementGuard':False},'originalTestAndHookTimeoutMs':30000}
def save():
 t=f.with_suffix('.writing');t.write_text(json.dumps(r,indent=2)+'\n');t.replace(f)
def quiet():
 since=None
 while True:
  now=time.monotonic();load=os.getloadavg()
  if load[0]<=8:
   since=now if since is None else since
   if now-since>=30:return
  else:since=None
  r.update(lastObservedLoad=list(load));save();time.sleep(5)
save()
try:
 prepath=out/'v10r1-endpoint-restriction-validation-20261018.json';pre=json.loads(prepath.read_text())
 assert pre['passed'] and pre['pairedWinValidated'] and pre['geometricReportsIdentical']
 capturepath=out/'v11-common-source-capture-20261018.json';capture=json.loads(capturepath.read_text())
 assert capture['peerManifestSourcesMatch'] and capture['sharedCheckoutModified'] is False
 patch='crates/nurbs-core/src/distance/surface_distance/bezier_restriction.rs'
 assert sha(stage/patch)==capture['explicitPrivateOverrides'][patch]['stageSha256']
 assert sha(stage/patch)==sha(Path('/private/tmp/osv-sweep-endpoint-restriction-v10-source-20261018')/patch)
 r.update(nativeTestReceiptSha256=sha(prepath),sharedSourceCaptureSha256=sha(capturepath),performanceEvidenceScope='V10 paired evidence selects the exact unchanged Rust math patch only. Combined V11 runtime performance, whole geometry and delivery remain unqualified until current-source gates pass.')
 nativepath=stage/'main-native-source-sha256.json';runtimepath=stage/'runtime-source-sha256.json';extrapath=stage/'qualification-external-inputs.json'
 native=json.loads(nativepath.read_text());runtime=json.loads(runtimepath.read_text());extra=json.loads(extrapath.read_text())
 r.update(nativeLedgerSha256=sha(nativepath),sourceLedgerSha256=sha(runtimepath),extraLedgerSha256=sha(extrapath),mainVerifierSha256=sha(stage/'scripts/verify-dist.mjs'))
 assert r['nativeLedgerSha256']==capture['nativeLedgerSha256'] and r['sourceLedgerSha256']==capture['runtimeLedgerSha256']
 priorpath=out/'v11-common-source-runtime-20261018.json';prior=json.loads(priorpath.read_text())
 assert prior['state']=='superseded' and prior['nativeLedgerSha256']==r['nativeLedgerSha256'] and prior['sourceLedgerSha256']==r['sourceLedgerSha256']
 assert [p['name'] for p in prior['phases']]==['common-profile-certificate','common-minkowski3d','common-bezier-profile','native-brep-regression']
 assert all(p['state']=='passed' and p['returncode']==0 for p in prior['phases'])
 r['retainedNativeReceiptSha256']=sha(priorpath);priorphases={p['name']:p for p in prior['phases']}

 def identity():
  assert sha(__file__)==r['driverSha256']
  assert sha(prepath)==r['nativeTestReceiptSha256'] and sha(capturepath)==r['sharedSourceCaptureSha256']
  assert sha(priorpath)==r['retainedNativeReceiptSha256']
  assert sha(nativepath)==r['nativeLedgerSha256'] and sha(runtimepath)==r['sourceLedgerSha256'] and sha(extrapath)==r['extraLedgerSha256']
  assert all(sha(stage/n)==h for n,h in native.items())
  assert all(sha(stage/n)==h for n,h in runtime.items())
  assert all(sha(stage/n)==m['sha256'] for n,m in extra['files'].items())
  assert all(sha(Path(extra['browserDependenciesRealPath'])/n)==h for n,h in extra['browserDependencyFiles'].items())
  assert sha(stage/'scripts/verify-dist.mjs')==r['mainVerifierSha256']
  config=(stage/'vitest.config.ts').read_text();assert 'testTimeout: 30000' in config and 'hookTimeout: 30000' in config
  if 'packedSha256' in r:
   for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm']:assert sha(stage/n)==r['packedSha256']
 def run(name,argv):
  identity()
  if name in priorphases:
   previous=priorphases[name];assert previous['argv']==argv
   r['phases'].append({**previous,'retainedFromReceiptSha256':r['retainedNativeReceiptSha256']});save();print(name+': retained same-source pass',flush=True);return
  compilation=(argv[0]=='cargo' and argv[1]=='build') or any(x in ['scripts/build-geometry-kernels.mjs','node_modules/vite/bin/vite.js','node_modules/vue-tsc/bin/vue-tsc.js','node_modules/typescript/bin/tsc'] for x in argv)
  if not compilation:r['state']='waiting-for-quiet-resources';save();quiet();identity()

  item={'name':name,'argv':argv,'state':'running','launchLoad':list(os.getloadavg()),'resourcePolicy':'background-compiler-nice19-jobs2' if compilation else 'unchanged-runtime-quiet-guard'};r['phases'].append(item);r['state']='running';save();print(name+': started',flush=True);since=time.monotonic()
  env={**os.environ,'PATH':str(Path(node).parent)+os.pathsep+os.environ['PATH'],'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','VITEST_MAX_WORKERS':'1','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','CARGO_NET_OFFLINE':'true','BINARYEN_CORES':'2','OSV_WASM_OPT_NATIVE':'/Users/themoretheless/.cargo/bin/wasm-opt'}
  # No external raw loader or inherited worker environment workaround.
  env.pop('NODE_OPTIONS',None)
  with (out/('v11-'+name+'.log')).open('w') as log:result=subprocess.run((['nice','-n','19'] if argv[0]=='cargo' or any(x in ['scripts/build-geometry-kernels.mjs','node_modules/vite/bin/vite.js','node_modules/vue-tsc/bin/vue-tsc.js','node_modules/typescript/bin/tsc'] for x in argv) else [])+argv,cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
  identity();item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-since);save();assert result.returncode==0,name+' failed; inspect preserved log'
  print(name+': passed',flush=True)
 for phase,term in [('common-profile-certificate','profile_certificate'),('common-minkowski3d','minkowski3d'),('common-bezier-profile','bezier_profile')]:
  run(phase,['cargo','test','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--lib',term])
 run('native-brep-regression',['cargo','test','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--lib','brep::'])
 run('geometry-build',[node,'scripts/build-geometry-kernels.mjs'])
 packed=sha(stage/'public/wasm/geometry-kernel.wasm');assert packed==sha(stage/'src/generated/geometry-kernels/kernel_bg.wasm')
 r.update(packedSha256=packed,rawSha256=sha(stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'));save()
 metadata=stage/'src/core/currentRuntimeGeometryEvidence.ts';old=metadata.read_bytes();(out/'v11-provider-evidence-before-build.ts').write_bytes(old)
 ref='runtime-provider-'+packed[:12]+'-2026-10-18'
 evidence={'wasmSha256':packed,'noticesSha256':sha(stage/'THIRD_PARTY_NOTICES.md'),'lockfileSha256':sha(stage/'package-lock.json'),'rustLockfileSha256':sha(stage/'crates/Cargo.lock'),'recordId':'docs/qualification/'+ref+'.json','corpusVersion':ref}
 metadata.write_text('// Private candidate. Publish only with the complete referenced qualification record.\nexport const CURRENT_RUNTIME_GEOMETRY_EVIDENCE = Object.freeze('+json.dumps(evidence,indent=2)+')\n')
 runtime['src/core/currentRuntimeGeometryEvidence.ts']=sha(metadata);runtimepath.write_text(json.dumps(runtime,indent=2)+'\n');r['sourceLedgerSha256']=sha(runtimepath)
 r['metadataTransition']={'beforeSha256':hashlib.sha256(old).hexdigest(),'afterSha256':sha(metadata),'recordId':evidence['recordId'],'qualificationRecordComplete':False};save();identity()
 run('native-migration-producer',['cargo','build','--locked','--offline','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_sweep_geometry_migration'])
 producer=stage/'crates/target/release/examples/certify_sweep_geometry_migration';r['nativeProducerSha256']=sha(producer);save()
 build={'state':'complete','passed':True,'packedSha256':packed,'rawSha256':r['rawSha256'],'nativeLedgerSha256':r['nativeLedgerSha256'],'nativeProducerSha256':sha(producer),'sourceChangedDuringBuild':False,'runtimeReceipt':str(f),'fullGoalComplete':False,'published':False}
 buildpath=out/'frozen-current-v11-build.json';assert not buildpath.exists();buildpath.write_text(json.dumps(build,indent=2)+'\n')
 adapter={'schema':'frozen-current-build-adapter/1','state':'complete','passed':True,'scope':'Native repair/WASM build only; complete runtime/STEP/UI and deployment unqualified.','artifacts':{'raw':{'sha256':r['rawSha256'],'byteLength':(stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm').stat().st_size},'packed':{'sha256':packed,'byteLength':(stage/'public/wasm/geometry-kernel.wasm').stat().st_size}},'buildReceipt':str(buildpath),'buildReceiptSha256':sha(buildpath),'nativeLedgerSha256':r['nativeLedgerSha256'],'fullGoalComplete':False}
 (stage/'completed-build-receipt.json').write_text(json.dumps(adapter,indent=2)+'\n')
 run('vue-typecheck',[node,'node_modules/vue-tsc/bin/vue-tsc.js','--noEmit'])
 run('mcp-typecheck',[node,'node_modules/typescript/bin/tsc','--noEmit','-p','tsconfig.mcp.json'])
 run('application',[node,'node_modules/vite/bin/vite.js','build']);run('distribution',[node,'scripts/verify-dist.mjs'])
 assert sha(stage/'dist/wasm/geometry-kernel.wasm')==packed
 run('runtime-identity',[node,'--import','tsx','scripts/audit-runtime-geometry-identity.mts'])
 def vitest(name,files):
  report=out/('v11-'+name+'.json')
  run(name,[node,'node_modules/vitest/vitest.mjs','run',*files,'--no-file-parallelism','--bail=1','--reporter=default','--reporter=json','--outputFile='+str(report)])
  j=json.loads(report.read_text());assert j['numTotalTests']==j['numPassedTests'] and j['numFailedTests']==j['numPendingTests']==0
  return j
 j=vitest('original31',['tests/nurbsPeriodicFixedNormalArcRush.test.ts','tests/nurbsClosedCorrectedRegularBody.test.ts','tests/nurbsSpatialRmfError.test.ts']);assert j['numPassedTests']==31
 r['originalFive']={}
 for pattern in ['arc-length-closed-corrected-frenet-nonplanar-body-boundary.r','open-spatial-rmf-varying-affine-hollow-corrected-body.r','periodic-fixed-normal-arc-plain-hollo','periodic-fixed-normal-arc-affine-soli','periodic-fixed-normal-arc-affine-holl']:
  cases=[a for t in j['testResults'] for a in t['assertionResults'] if pattern in a['fullName']];assert len(cases)==1 and cases[0]['status']=='passed' and cases[0]['duration']<30000;r['originalFive'][pattern]={'status':'passed','durationMs':cases[0]['duration']}
 r['focusedDeadlinePass']=True;save();assert vitest('affine',['tests/sweepAffineLattice.test.ts'])['numPassedTests']==10
 files=sorted(json.loads((stage/'public-100-baseline-inventory.json').read_text())['files']);assert len(files)==100
 j=vitest('public100',files);assert j['numPassedTests']>=736;r['public100Tests']=j['numPassedTests'];r['deadlineClosureConfirmed']=True;save()
 files=sorted(set(json.loads((out/'v5-provider-baseline-inventory.json').read_text())['files']+['tests/runtimeGeometryActualBinding.test.ts']))
 j=vitest('provider-baseline',files);r['providerBaselineTests']=j['numPassedTests'];r['providerBaselineFiles']=len(files);save()
 j=vitest('common-peer-host',['tests/certifiedGeometryFollowup.test.ts', 'tests/minkowski3d.test.ts', 'tests/minkowskiRegionCertificates.test.ts', 'tests/path2dCurvex.test.ts', 'tests/bezierProfileBoolean.test.ts', 'tests/brepProfileMeasureCertificates.test.ts', 'tests/brepProfileExtrusionAudit.test.ts', 'tests/bezierFollowupCertificates.test.ts', 'tests/bezierRemainingCertificates.test.ts', 'tests/bezierProfileDegeneracies.test.ts', 'tests/brepProfileCertificates.test.ts', 'tests/directProfileTools.test.ts', 'tests/brepProfile.test.ts', 'tests/bezierRegionCertificates.test.ts', 'tests/mainSolidWorkerRuntime.test.ts', 'tests/mainSolidWorkerClient.test.ts']);assert j['numPassedTests']>=153;r['commonPeerHostTests']=j['numPassedTests'];save()
 run('common-peer-ui',[node,'node_modules/vitest/vitest.mjs','run','tests/directModelerUi.test.ts','-t','Minkowski|fully subtracted B-rep','--no-file-parallelism','--bail=1','--reporter=default','--reporter=json','--outputFile='+str(out/'v11-common-peer-ui.json')])
 j=json.loads((out/'v11-common-peer-ui.json').read_text());assert j['numPassedTests']==5 and j['numFailedTests']==0;r['commonPeerUiTests']=5;save()
 run('observer-unit',[node,'--test','scripts/test-sweep-native-cancel-probe.mjs'])
 r.update(state='complete',passed=True,qualificationRecordComplete=False);save();print('Source-bound runtime gates passed; fresh STEP/UI and final qualification record still required.',flush=True)
except Exception as error:
 r.update(state='failed',passed=False,reason=str(error));save();raise
