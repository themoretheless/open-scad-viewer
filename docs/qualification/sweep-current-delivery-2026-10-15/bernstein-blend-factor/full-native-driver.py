from pathlib import Path
import json,hashlib,os,subprocess,time
stage=Path('/private/tmp/osv-sweep-bernstein-blend-candidate-20261015');frozen=Path('/private/tmp/osv-sweep-interval-mul-wasm-20261015');prior_path=Path('/private/tmp/osv-sweep-bernstein-blend-resume-20261015/receipt.json');prior=json.loads(prior_path.read_text());assert prior['state']=='complete' and prior['passed'] is True
bs=[m['milliseconds'] for m in prior['measurements'] if m['kind']=='baseline'];cs=[m['milliseconds'] for m in prior['measurements'] if m['kind']=='candidate'];assert len(bs)==len(cs)==2 and max(cs)<min(bs)
assert len(set(m['reportSha256'] for m in prior['measurements']))==1
out=Path('/private/tmp/osv-sweep-bernstein-full-native-20261015');out.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();ledger=json.loads((frozen/'frozen-runtime-source-sha256.json').read_text());native={p:h for p,h in ledger.items() if p.startswith('crates/')};fixtures=json.loads(Path('/private/tmp/osv-sweep-bernstein-blend-qualification-20261015/test-fixture-repair.json').read_text())['files'];changed='crates/nurbs-core/src/distance/surface_distance/bezier_restriction.rs'
r={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'priorReceiptSha256':sha(prior_path),'nativeSourceLedgerSha256':sha(frozen/'frozen-runtime-source-sha256.json'),'nativeFilesChecked':len(native),'candidateSourceSha256':prior['candidateSourceSha256'],'phases':[],'scope':'Complete default-feature native library tests of frozen N plus one bit-equivalent Bernstein optimization, split by package to retain completed phases. Existing ignored tests remain explicit. Not current-main-other-changes/WASM/UI/goal qualification.','mainWasmDelivered':False,'fullUiPassed':False}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(r,indent=2)+'\n');p.replace(out/'receipt.json')
def identity():
 assert sha(__file__)==r['driverSha256']
 for p,h in native.items():assert sha(stage/p)==(prior['candidateSourceSha256'] if p==changed else h),p
 for f in fixtures:assert sha(stage/f['file'])==f['sha256'],f['file']
identity();save();print('full native pid',os.getpid(),flush=True)
commands=[(p,['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p',p,'--lib','--','--test-threads=2']) for p in ['nurbs-core','brep-core','geometry-bridge']]
commands.append(('migration-producer',['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_sweep_geometry_migration','--','--test-threads=2']))
for name,argv in commands:
 identity();p={'name':name,'argv':argv,'state':'running'};r['phases'].append(p);save();print(name+': started',flush=True);t=time.monotonic()
 with (out/(name+'.log')).open('w') as log:status=subprocess.run(argv,cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2'},stdout=log,stderr=subprocess.STDOUT)
 identity();p.update(state='passed' if status.returncode==0 else 'failed',returncode=status.returncode,seconds=time.monotonic()-t);save();print(name+': '+p['state'],flush=True)
 if status.returncode:r['state']='failed';save();raise SystemExit(status.returncode)
r.update(state='complete',passed=True);save()
