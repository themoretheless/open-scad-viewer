from pathlib import Path
import hashlib,json,os,shutil,subprocess,time
stage=Path('/private/tmp/osv-sweep-cap-projection-current-main-source-final-20261015')
root=Path('/private/tmp/osv-sweep-cap-projection-artifact-qa-20261015');root.mkdir(exist_ok=False)
build_path=Path('/private/tmp/osv-sweep-cap-projection-current-main-build-20261015/receipt.json')
post_path=Path('/private/tmp/osv-sweep-cap-projection-post-cache-native-20261015/receipt.json')
build_driver=Path('/private/tmp/osv-sweep-build-cap-projection-current-main-20261015.py')
ledger_path=stage/'frozen-current-main-source-sha256.json';extra_path=stage/'qualification-external-inputs.json';override_path=stage/'qa-source-overrides-current.json'
ledger=json.loads(ledger_path.read_text());extra=json.loads(extra_path.read_text());sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
receipt={'schema':'sweep-cap-projection-artifact-qa/1','state':'waiting-native-app-gates','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'buildDriverSha256':sha(build_driver),'sourceLedgerSha256':sha(ledger_path),'extraInputLedgerSha256':sha(extra_path),'qaOverridesSha256':sha(override_path),'sourceFilesChecked':len(ledger),'phases':[],'scope':'New exact-artifact C2 policies/mutations/UI8/STEP4, actual retained multispan ABI, original100-file public tests and full canonical207 STEP/478 UI. Native current-main delivery and all-mode source proof completion remain separate.','fullGoalComplete':False}
def save():
 p=root/'receipt.json.writing';p.write_text(json.dumps(receipt,indent=2)+'\n');p.replace(root/'receipt.json')
def identity():
 for p,key in [(Path(__file__),'driverSha256'),(build_driver,'buildDriverSha256'),(ledger_path,'sourceLedgerSha256'),(extra_path,'extraInputLedgerSha256'),(override_path,'qaOverridesSha256')]:assert sha(p)==receipt[key],str(p)
 for name,h in ledger.items():assert sha(stage/name)==h,name
 for name,item in extra['files'].items():assert sha(stage/name)==item['sha256'],name
 browser=Path(extra['browserDependenciesRealPath'])
 for name,h in extra['browserDependencyFiles'].items():assert sha(browser/name)==h,name
 if 'geometryWasmSha256' in receipt:
  for name in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(stage/name)==receipt['geometryWasmSha256'],name
  assert receipt['geometryWasmSha256'] in (stage/'src/generated/geometry-kernels/identity.ts').read_text()
def run(name,argv):
 identity();phase={'name':name,'state':'running','argv':list(map(str,argv))};receipt['phases'].append(phase);save();print(name+': started',flush=True);start=time.monotonic()
 with (root/(name+'.log')).open('w') as log:status=subprocess.run(list(map(str,argv)),cwd=stage,env={**os.environ,'NODE_OPTIONS':'--max-old-space-size=8192','PYTHONDONTWRITEBYTECODE':'1','CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true'},stdout=log,stderr=subprocess.STDOUT)
 identity();phase.update(state='passed' if status.returncode==0 else 'failed',returncode=status.returncode,seconds=time.monotonic()-start);save();print(name+': '+phase['state'],flush=True)
 if status.returncode:receipt['state']='failed';save();raise SystemExit(status.returncode)
identity();save()
while True:
 build=json.loads(build_path.read_text());post=json.loads(post_path.read_text())
 if build['state']=='failed' or post['state']=='failed':receipt.update(state='stopped-after-native-app-gate-failure',buildOutcome=build['state'],postCacheOutcome=post['state']);save();raise SystemExit(1)
 if build['state']=='complete' and post['state']=='complete':
  assert build['passed'] and post['passed'];break
 try:os.kill(build['pid'],0)
 except ProcessLookupError:receipt['state']='build-ended-without-complete-receipt';save();raise SystemExit(1)
 process=subprocess.run(['/bin/ps','-p',str(build['pid']),'-o','args='],capture_output=True,text=True)
 if process.returncode==0:assert str(build_driver) in process.stdout,'Build PID reused'
 elif process.returncode!=1:raise RuntimeError('Cannot authenticate current build')
 time.sleep(10)
receipt.update(state='running',geometryWasmSha256=build['artifacts']['packed']['sha256'],buildReceiptSha256=sha(build_path),postCacheReceiptSha256=sha(post_path));identity();save()
shutil.copy2(build_path,stage/'completed-build-receipt.json')
node=['node','--import','tsx'];oracle='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
for label,flag in [('automatic-caps',[]),('exact-planar-caps',['--exact-planar-caps'])]:
 output=root/('c2-'+label+'.json')
 run('c2-'+label,node+['scripts/check-staged-corrected-c2.mts',stage,output,build_path]+flag)
 run('c2-'+label+'-model-mutation',node+['scripts/check-staged-c2-volume-mutation.mts',stage,output,root/('c2-'+label+'-mutation.json')])
run('constructed-multispan',node+['scripts/check-staged-constructed-multispan-sweep.mts',stage,root/'constructed-multispan.json'])
run('multispan-raw',node+['scripts/check-staged-multispan-jets.mts',stage,root/'multispan-raw.json','--raw-built'])
run('multispan-packed',node+['scripts/check-staged-multispan-jets.mts',stage,root/'multispan-packed.json'])
run('closed-multispan',node+['scripts/check-staged-closed-strip-jets.mts',stage,root/'closed-multispan.json'])
inventory=json.loads((stage/'public-100-baseline-inventory.json').read_text());assert len(inventory['files'])==100;assert inventory['originalTestAndHookTimeoutMs']==30000
for name,h in inventory['files'].items():assert sha(stage/name)==h,name
inventory.update(artifactSha256=receipt['geometryWasmSha256'],sourceLedgerSha256=receipt['sourceLedgerSha256'],status='bound-current-artifact-pending-test')
(root/'public100-inventory.json').write_text(json.dumps(inventory,indent=2)+'\n')
run('public100',inventory['command']+['--reporter=json','--outputFile='+str(root/'public100.json')])
public=json.loads((root/'public100.json').read_text());assert public['success'] and public['numFailedTests']==0
observed=[str(Path(r['name']).resolve().relative_to(stage)) for r in public['testResults']];assert sorted(observed)==sorted(inventory['files'])
receipt['public100']={k:public[k] for k in ['numTotalTests','numPassedTests','numFailedTests','numPendingTests']};save()
c2step=root/'c2-step4'
run('c2-step4-export',node+['--loader','./qualification-raw-imports.mjs','scripts/export-corrected-c2-step.mts',c2step])
manifest=json.loads((c2step/'manifest.json').read_text());assert len(manifest['cases'])==4 and manifest['artifactProvenance']['geometryWasmSha256']==receipt['geometryWasmSha256']
run('c2-step4-reference',[oracle,'scripts/reference-rational-boundary-volume.py',c2step])
run('c2-step4-opencascade',[oracle,'scripts/verify-sweep-step-occt.py',c2step])
run('c2-ui8',['node','scripts/check-sweep-miter-matrix-browser.mjs',root/'c2-ui8','--corrected-c2-body-matrix','--native-running-cancel','--gpu-backend=metal'])
ui=json.loads((root/'c2-ui8/matrix.json').read_text());assert ui['passed'] and len(ui['cases'])==8 and ui['geometryWasmSha256']==receipt['geometryWasmSha256']
producer=Path('/private/tmp/osv-sweep-cap-projection-current-main-build-20261015/certify-sweep-geometry-migration');assert sha(producer)==build['nativeProducerSha256']
run('canonical-step207-ui478',['python3','scripts/qualify-sweep-current-artifact.py','--output',root/'canonical207-ui478','--artifact-gates',build_path.parent,'--wait-pid',str(build['pid']),'--expected-wasm',receipt['geometryWasmSha256'],'--native-producer',producer,'--source-ledger',ledger_path.name,'--qa-overrides',override_path.name,'--extra-input-ledger',extra_path.name])
run('retained-interiors-producer',['cargo','build','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_retained_surface_interiors'])
run('retained-interiors',['python3','scripts/qualify-retained-surface-interiors.py',root/'canonical207-ui478/step207/manifest.json',root/'canonical207-ui478/receipt.json',stage/'crates/target/release/examples/certify_retained_surface_interiors',stage/'crates/brep-core/examples/certify_retained_surface_interiors.rs',root/'retained-interiors'])
identity();receipt.update(state='complete',passed=True);save();print('Exact-artifact finite qualification complete; all-mode audit and main delivery remain separate',flush=True)
