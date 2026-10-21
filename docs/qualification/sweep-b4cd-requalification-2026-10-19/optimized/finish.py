"""Finish fresh UI and supplements after an authenticated pre-browser package failure."""
from pathlib import Path
import hashlib,json,os,subprocess,time
OUT=Path(__file__).resolve().parent
snapshot=json.loads((OUT/'snapshot.json').read_text());STAGE=Path(snapshot['stage'])
NODE='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';ORACLE='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
WASM=snapshot['wasmSha256'];sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
OLD=OUT/'canonical207-ui478';prior=json.loads((OLD/'receipt.json').read_text())
assert prior['state']=='failed'and prior['passed']is False
assert prior['phases'][-1]['name']=='full-ui478'and prior['phases'][-1]['state']=='failed'
assert all(p['state']=='passed'for p in prior['phases'][:-1])
assert 'E_QUALIFICATION_PLAYWRIGHT_PACKAGE_MISSING'in(OLD/'ui478/closed-frenet.log').read_text()
assert not (OLD/'ui478/closed-frenet/matrix.json').exists(),'The diagnosed failure must precede browser cases'
ledger=json.loads((STAGE/'qualification-inputs.json').read_text());path=OUT/'finish.json';assert not path.exists()
record={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),
 'geometryWasmSha256':WASM,'phases':[],'previousCanonicalReceiptSha256':sha(OLD/'receipt.json'),
 'sourceLedgerSha256':sha(STAGE/'qualification-inputs.json'),
 'browserPackagePreflightSha256':sha(OUT/'browser-package-preflight.json'),
 'restoredPackageManifestSha256':sha(STAGE/'docs/qualification/playwright-license-manifest-v1.json'),
 'scope':'Original unchanged completed STEP phases plus an entirely fresh UI478 execution after fixing the pre-browser package dependency; stopped attempts remain failed.'}
def save():
 p=path.with_suffix('.writing');p.write_text(json.dumps(record,indent=2)+'\n');p.replace(path)
def identity():
 assert sha(__file__)==record['driverSha256']
 assert sha(OLD/'receipt.json')==record['previousCanonicalReceiptSha256']
 assert sha(STAGE/'qualification-inputs.json')==record['sourceLedgerSha256']
 assert sha(OUT/'browser-package-preflight.json')==record['browserPackagePreflightSha256']
 assert sha(STAGE/'docs/qualification/playwright-license-manifest-v1.json')==record['restoredPackageManifestSha256']
 for name,h in ledger.items():assert sha(STAGE/name)==h,name
 for name in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(STAGE/name)==WASM
env={**os.environ,'PATH':str(Path(NODE).parent)+os.pathsep+os.environ['PATH'],'NODE_OPTIONS':'--max-old-space-size=8192','PYTHONDONTWRITEBYTECODE':'1'}
def run(name,argv):
 identity();p={'name':name,'argv':list(map(str,argv)),'state':'running','startedAt':time.time()};record['phases'].append(p);save();print(name+': started',flush=True);start=time.monotonic()
 with(OUT/(name+'.log')).open('w')as log:
  child=subprocess.Popen(p['argv'],cwd=STAGE,env=env,stdout=log,stderr=subprocess.STDOUT);p['pid']=child.pid;save();code=child.wait()
 identity();p.update(state='passed'if code==0 else'failed',returncode=code,seconds=time.monotonic()-start,logSha256=sha(OUT/(name+'.log')));save();print(name+': '+p['state'],flush=True);assert code==0,name
try:
 save()
 c2=OUT/'c2-step4'
 run('c2-step4-export',[NODE,'--import','tsx','--loader','./qualification-raw-imports.mjs','scripts/export-corrected-c2-step.mts',c2])
 run('c2-step4-reference',[ORACLE,'scripts/reference-rational-boundary-volume.py',c2])
 run('c2-step4-occt',[ORACLE,'scripts/verify-sweep-step-occt.py',c2])
 ui=OUT/'ui478-complete'
 run('full-ui478',[NODE,'scripts/run-fresh-full-ui-audit.mjs',ui])
 ui_report=json.loads((ui/'full-reconciliation.json').read_text())
 assert ui_report['passed']and ui_report['cases']==478 and ui_report['nativeObservations']==1472 and ui_report['geometryWasmSha256']==WASM
 complete=OUT/'canonical-complete';complete.mkdir()
 (complete/'step207').symlink_to(OLD/'step207',target_is_directory=True)
 (complete/'annular-station').symlink_to(OLD/'annular-station',target_is_directory=True)
 (complete/'ui478').symlink_to(ui,target_is_directory=True)
 manifest=OLD/'step207/manifest.json';global_report=json.loads((OLD/'step207/global.json').read_text())
 assert global_report['passed']and global_report['manifestSha256']==sha(manifest)
 composed={**prior,'state':'complete','passed':True,'finalManifestSha256':sha(manifest),
  'phases':[p for p in prior['phases']if p['state']=='passed']+[record['phases'][-1]],
  'failedOriginalAttempt':{'path':str(OLD/'receipt.json'),'sha256':sha(OLD/'receipt.json'),'state':'failed'},
  'freshUiReport':str(ui/'full-reconciliation.json'),'freshUiReportSha256':sha(ui/'full-reconciliation.json'),
  'scope':'Composition of unchanged completed STEP/native phases and fresh complete same-artifact UI478; original pre-browser failure retained without relabeling.',
  'composingDriverSha256':record['driverSha256'],'restoredPackageManifestSha256':record['restoredPackageManifestSha256']}
 (complete/'receipt.json').write_text(json.dumps(composed,indent=2)+'\n')
 record['canonicalCompleteReceiptSha256']=sha(complete/'receipt.json');save()
 run('legacy-cap-refusal-controls',['python3','scripts/qualify-sweep-legacy-cap-refusals.py','--canonical',complete,'--expected-wasm',WASM,'--output',OUT/'legacy-cap-refusals','--checker-directory',STAGE/'scripts','--node',NODE])
 producer=STAGE/'crates/target/release/examples/certify_retained_surface_interiors'
 run('retained-interiors',['python3','scripts/qualify-retained-surface-interiors.py',complete/'step207/manifest.json',complete/'receipt.json',producer,STAGE/'crates/brep-core/examples/certify_retained_surface_interiors.rs',OUT/'retained-interiors'])
 run('c2-ui8',[NODE,'scripts/check-sweep-miter-matrix-browser.mjs',OUT/'c2-ui8','--corrected-c2-body-matrix','--native-running-cancel','--gpu-backend=metal'])
 identity();record.update(state='complete',passed=True);save()
except BaseException as error:record.update(state='failed',passed=False,error=str(error));save();raise
