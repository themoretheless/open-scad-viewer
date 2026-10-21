from pathlib import Path
import hashlib,json,os,subprocess,time
stage=Path('/private/tmp/osv-sweep-bernstein-blend-candidate-20261015')
frozen=Path('/private/tmp/osv-sweep-interval-mul-wasm-20261015')
prior=Path('/private/tmp/osv-sweep-bernstein-blend-qualification-20261015/receipt.json')
out=Path('/private/tmp/osv-sweep-bernstein-native-suite-20261015');out.mkdir(exist_ok=False)
source='crates/nurbs-core/src/distance/surface_distance/bezier_restriction.rs'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
ledger=json.loads((frozen/'frozen-runtime-source-sha256.json').read_text());native={p:h for p,h in ledger.items() if p.startswith('crates/')}
original=(frozen/source).read_text();candidate=original.replace('a[k].mul(beta)?.add(b[k].mul(alpha)?)?','super::mul_blend_factor(a[k],beta)?.add(super::mul_blend_factor(b[k],alpha)?)?')
assert candidate!=original
r={'state':'waiting-existing-ABBA','passed':False,'driverSha256':sha(__file__),'waitPid':75226,'phases':[],'scope':'Full frozen N plus exactly one Bernstein interval optimization: native library regressions and multispan exact-strip coverage in the existing library suite. No UI/STEP/artifact/main delivery promotion.','fullUiPassed':False,'mainCandidateRetained':False}
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(r,indent=2)+'\n');p.replace(out/'receipt.json')
def identity():
 assert sha(__file__)==r['driverSha256']
 for p,h in native.items():
  if p==source:assert (stage/p).read_text()==candidate
  else:assert sha(stage/p)==h,p
save();print('native suite pid',os.getpid(),flush=True)
while True:
 d=json.loads(prior.read_text())
 if d['state']=='complete':assert d['passed'] is True;break
 if d['state']=='failed':r['state']='stopped-after-prior-failure';save();raise SystemExit(1)
 try:os.kill(75226,0)
 except ProcessLookupError:r['state']='prior-ended-without-complete-pass';save();raise SystemExit(1)
 time.sleep(5)
measurements=d['measurements'];baseline=[m['milliseconds'] for m in measurements if m['kind']=='baseline'];changed=[m['milliseconds'] for m in measurements if m['kind']=='candidate']
assert len(baseline)==len(changed)==2
if max(changed)>=min(baseline):r.update(state='skipped-mixed-or-slower-candidate',reason='No full suite for a candidate without a consistent ABBA improvement; keep actual timing reports and do not adopt.');save();raise SystemExit(2)
identity();r.update(state='running',priorReceiptSha256=sha(prior),nativeFilesChecked=len(native));save()
commands=[('libraries',['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','nurbs-core','-p','brep-core','-p','geometry-bridge','--lib','--','--test-threads=2']),('migration-producer',['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_sweep_geometry_migration','--','--test-threads=2'])]
for name,argv in commands:
 identity();p={'name':name,'argv':argv,'state':'running'};r['phases'].append(p);save();t=time.monotonic();print(name+': started',flush=True)
 with (out/(name+'.log')).open('w') as log:status=subprocess.run(argv,cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2'},stdout=log,stderr=subprocess.STDOUT)
 identity();p.update(state='passed' if status.returncode==0 else 'failed',returncode=status.returncode,seconds=time.monotonic()-t);save();print(name+': '+p['state'],flush=True)
 if status.returncode:r['state']='failed';save();raise SystemExit(status.returncode)
r.update(state='complete',passed=True);save()
