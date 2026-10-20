from pathlib import Path
import hashlib,json,os,subprocess,time,shutil
stage=Path('/private/tmp/osv-sweep-corrected-c2-current-main-source-20261015');out=Path('/private/tmp/osv-sweep-corrected-c2-current-main-build-resume1-20261015');out.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();ledger=json.loads((stage/'frozen-current-main-source-repair1-sha256.json').read_text());capture=json.loads((stage/'source-capture-repair1-receipt.json').read_text());assert capture['passed'] is True
seed=json.loads((stage/'dependency-seed-receipt.json').read_text());r={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'sourceLedgerSha256':sha(stage/'frozen-current-main-source-repair1-sha256.json'),'sourceFilesChecked':len(ledger),'phases':[],'scope':'Fresh isolated current-main source, including all parallel changes and retained Rust optimization. Existing full207/478 scope remains required. No old artifact qualification or main overwrite.','fullGoalComplete':False,'mainModified':False}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(r,indent=2)+'\n');p.replace(out/'receipt.json')
def identity():
 assert sha(__file__)==r['driverSha256']
 for p,h in ledger.items():assert sha(stage/p)==h,p
 for p,h in capture['testOnlyExternalInputs'].items():assert sha(stage/p)==h,p
 assert sha(stage/'package-lock.json')==seed['packageLockSha256']
 assert sha(Path(seed['nodeModulesRealPath'])/'.package-lock.json')==seed['installedNodeLockSha256']
 assert sha(seed['optimizerPath'])==seed['optimizerSha256']
prior_path=Path('/private/tmp/osv-sweep-corrected-c2-current-main-build-20261015/receipt.json'); prior=json.loads(prior_path.read_text()); assert prior['state']=='failed' and all(p['state']=='passed' for p in prior['phases'][:2]); r['priorReceiptSha256']=sha(prior_path); r['reusedUnchangedNativeGates']=prior['phases'][:2]; r['scope']='Original5870-source snapshot plus explicit same-type public gear import repair; unchanged native focused/full1426 gates reused, only failed WASM phase and remaining application gates resumed. No main overwrite or latest-source qualification claim.'
identity();save();print('current main build pid',os.getpid(),flush=True)
commands=[('native-and-aux-wasm',['npm','run','build:geometry']),('vue-typecheck',['node','node_modules/vue-tsc/bin/vue-tsc.js','--noEmit']),('mcp-typecheck',['node','node_modules/typescript/bin/tsc','--noEmit','-p','tsconfig.mcp.json']),('application',['node','node_modules/vite/bin/vite.js','build']),('distribution',['node','scripts/verify-dist.mjs'])]
for name,argv in commands:
 identity();p={'name':name,'state':'running','argv':argv};r['phases'].append(p);save();print(name+': started',flush=True);t=time.monotonic()
 with (out/(name+'.log')).open('w') as log:status=subprocess.run(argv,cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','OSV_WASM_OPT_NATIVE':seed['optimizerPath'],'BINARYEN_CORES':'2','PYTHONDONTWRITEBYTECODE':'1'},stdout=log,stderr=subprocess.STDOUT)
 identity();p.update(state='passed' if status.returncode==0 else 'failed',returncode=status.returncode,seconds=time.monotonic()-t);save();print(name+': '+p['state'],flush=True)
 if status.returncode:r['state']='failed';save();raise SystemExit(status.returncode)
raw=stage/'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm';packed=stage/'public/wasm/geometry-kernel.wasm';assert packed.read_bytes()==(stage/'src/generated/geometry-kernels/kernel_bg.wasm').read_bytes();assert sha(packed) in (stage/'src/generated/geometry-kernels/identity.ts').read_text();assert (stage/'dist/wasm/geometry-kernel.wasm').read_bytes()==packed.read_bytes()
r.update(state='complete',passed=True,artifacts={kind:{'sha256':sha(p),'byteLength':p.stat().st_size} for kind,p in [('raw',raw),('packed',packed)]});identity();save();print('Current-main isolated build and unchanged distribution budgets passed; full fresh geometry/UI qualification remains',flush=True)
