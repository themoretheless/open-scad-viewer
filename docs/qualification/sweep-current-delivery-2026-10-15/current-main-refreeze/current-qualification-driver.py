from pathlib import Path
import hashlib,json,os,subprocess,time,shutil
stage=Path('/private/tmp/osv-sweep-current-main-source-20261015');out=Path('/private/tmp/osv-sweep-current-main-qualification-20261015');out.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
build=Path('/private/tmp/osv-sweep-current-main-build-20261015/receipt.json');b=json.loads(build.read_text());assert b['passed'] is True and b['state']=='complete'
ui=Path('/private/tmp/osv-sweep-current-main-focused-closed-frenet-arc-hollow-20261015/matrix.json');u=json.loads(ui.read_text());assert u.get('passed') is True and len(u['cases'])==2 and all(c['status']=='passed' for c in u['cases'])
ledger=json.loads((stage/'frozen-current-main-source-sha256.json').read_text());override=json.loads((stage/'qa-source-overrides-current.json').read_text());over={r['file']:r['afterSha256'] for r in override['files']};extra=json.loads((stage/'qualification-external-inputs.json').read_text())
r={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'buildReceiptSha256':sha(build),'focusedClosedHollowReportSha256':sha(ui),'sourceLedgerSha256':sha(stage/'frozen-current-main-source-sha256.json'),'sourceFilesChecked':len(ledger),'qaOverrideLedgerSha256':sha(stage/'qa-source-overrides-current.json'),'extraInputLedgerSha256':sha(stage/'qualification-external-inputs.json'),'geometryWasmSha256':b['artifacts']['packed']['sha256'],'phases':[],'scope':'Fresh current-main focused native sweep/cusp and ABI regressions, then full207 STEP and478 UI/1472 native observations. No finite-to-universal or current-main overwrite claim.','fullGoalComplete':False}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(r,indent=2)+'\n');p.replace(out/'receipt.json')
def identity():
 assert sha(__file__)==r['driverSha256'];assert sha(build)==r['buildReceiptSha256'];assert sha(ui)==r['focusedClosedHollowReportSha256']
 for p,h in ledger.items():assert sha(stage/p)==over.get(p,h),p
 assert sha(stage/'qa-source-overrides-current.json')==r['qaOverrideLedgerSha256'];assert sha(stage/'qualification-external-inputs.json')==r['extraInputLedgerSha256']
 for p,item in extra['files'].items():assert sha(stage/p)==item['sha256'],p
 for kind,p in [('raw','crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'),('packed','public/wasm/geometry-kernel.wasm')]:assert sha(stage/p)==b['artifacts'][kind]['sha256']
def run(name,argv):
 identity();phase={'name':name,'state':'running','argv':list(map(str,argv))};r['phases'].append(phase);save();print(name+': started',flush=True);t=time.monotonic()
 with (out/(name+'.log')).open('w') as log:status=subprocess.run(list(map(str,argv)),cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','NODE_OPTIONS':'--max-old-space-size=8192','PYTHONDONTWRITEBYTECODE':'1'},stdout=log,stderr=subprocess.STDOUT)
 identity();phase.update(state='passed' if status.returncode==0 else 'failed',returncode=status.returncode,seconds=time.monotonic()-t);save();print(name+': '+phase['state'],flush=True)
 if status.returncode:r['state']='failed';save();raise SystemExit(status.returncode)
identity();save();print('current qualification pid',os.getpid(),flush=True)
cargo=['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml']
run('native-current-cusp',cargo+['-p','geometry-bridge','--lib','brep::cusp_sampling::tests::','--','--test-threads=2'])
run('native-current-sweep-core',cargo+['-p','nurbs-core','--lib','sweeps::','--','--test-threads=2'])
run('native-current-brep-sweep',cargo+['-p','brep-core','--lib','sweep','--','--test-threads=2'])
run('native-current-migration-tests',cargo+['-p','brep-core','--example','certify_sweep_geometry_migration','--','--test-threads=2'])
run('native-current-migration-producer',['cargo','build','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_sweep_geometry_migration','--message-format=json'])
executables=[]
for line in (out/'native-current-migration-producer.log').read_text().splitlines():
 try:item=json.loads(line)
 except json.JSONDecodeError:continue
 if item.get('reason')=='compiler-artifact' and item.get('target',{}).get('name')=='certify_sweep_geometry_migration' and item.get('executable'):executables.append(Path(item['executable']))
assert len(executables)==1,executables
producer=out/'certify-sweep-geometry-migration';shutil.copyfile(executables[0],producer);producer.chmod(0o755);r['nativeProducerSha256']=sha(producer);save()
for kind,args in [('raw',['--raw-built']),('packed',[])]:run('multispan-'+kind,['node','--import','tsx','scripts/check-staged-multispan-jets.mts',stage,out/('multispan-'+kind+'.json'),*args])
run('closed-multispan',['node','--import','tsx','scripts/check-staged-closed-strip-jets.mts',stage,out/'closed-multispan.json'])
run('fresh-step207-ui478',['python3','scripts/qualify-sweep-current-artifact.py','--output',out/'fresh207-ui478','--artifact-gates',build.parent,'--wait-pid',str(os.getpid()),'--expected-wasm',r['geometryWasmSha256'],'--native-producer',producer,'--source-ledger','frozen-current-main-source-sha256.json','--extra-input-ledger','qualification-external-inputs.json'])
r.update(state='complete',passed=True);save();print('Fresh finite qualification passed; universal audit and delivery still separate',flush=True)
