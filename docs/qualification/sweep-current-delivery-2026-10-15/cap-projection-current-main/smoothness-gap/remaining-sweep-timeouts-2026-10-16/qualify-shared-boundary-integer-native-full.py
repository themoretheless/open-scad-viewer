"""Fresh full primitive coverage and affected BRep callers; existing full980BRep baseline retained."""
from pathlib import Path
import subprocess,json,hashlib,os,time
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-shared-boundary-integer-source-20261016');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();f=out/'shared-boundary-integer-native-full.json';assert not f.exists()
pair=json.loads((out/'shared-boundary-integer-paired.json').read_text());assert pair['state']=='complete' and pair['responsesIdentical']
a=[x['milliseconds'] for x in pair['runs'] if x['label']=='A'];b=[x['milliseconds'] for x in pair['runs'] if x['label']=='B']
assert all(new<old for old,new in zip(a,b)), 'No measured pair may regress'
# Correctness gate only: host drift prevented native timing acceptance.
# Unchanged WASM deadlines remain the authoritative performance gate.
r={'state':'running','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'ledgerSha256':sha(stage/'shared-boundary-integer-source-sha256.json'),'fullBrepBase':'980passed/0failed/3existingignored on shared-boundary Bernstein baseline; not claimed as a fresh combined run','phases':[],'mainWasmModified':False,'fullGoalComplete':False,'nativeTimingStable':pair['timingStable'],'nativeStrictTimingAccepted':pair['eligibleForAdoption'],'performanceQualificationPendingUnchangedWasmDeadlines':True}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
def identity():
 j=json.loads((stage/'shared-boundary-integer-source-sha256.json').read_text())
 for name,digest in j.items():
  if name.startswith(('src/generated/geometry-kernels/','src/generated/wasm-brotli/')) or name=='public/wasm/geometry-kernel.wasm':continue
  assert sha(stage/name)==digest,name
save();env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','RUSTUP_TOOLCHAIN':'nightly-2026-09-16'}
for name,package,pattern in [('combined-native-nurbs','nurbs-core',None),('combined-native-face-contacts','brep-core','face_contacts'),('combined-native-volume-validity','brep-core','volume_validity')]:
 identity();argv=['cargo','test','--locked','--manifest-path','crates/Cargo.toml','-p',package,'--lib'];argv+=[] if pattern is None else [pattern];item={'name':name,'state':'running'};r['phases'].append(item);save();print(name+': started',flush=True)
 with (out/(name+'.log')).open('w') as log:result=subprocess.run(argv,cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
 identity();item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode);save();print(name+': '+item['state'],flush=True)
 if result.returncode:r['state']='failed';save();raise SystemExit(result.returncode)
r.update(state='complete',passed=True);save()
