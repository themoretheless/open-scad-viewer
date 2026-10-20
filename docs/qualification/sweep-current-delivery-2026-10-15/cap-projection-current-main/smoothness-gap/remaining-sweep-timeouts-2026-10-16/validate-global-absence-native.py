"""Isolated lazy projection reuse; no runtime/source overwrite."""
import json,hashlib,os,subprocess,time
from pathlib import Path
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-global-absence-source-20261016');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
f=out/'global-absence-native-validation.json';assert not f.exists()
r={'state':'waiting-for-existing-deadline-run','pid':os.getpid(),'passed':False,'mainProductModified':False,'fullGoalComplete':False,'driverSha256':sha(__file__),'sourceSha256':sha(stage/'crates/nurbs-core/src/intersection/surface_contact_search.rs'),'phases':[]}
def save():
 q=f.with_suffix('.writing');q.write_text(json.dumps(r,indent=2)+'\n');q.replace(f)
save()
while json.loads((out/'endpoint-restriction-native-validation.json').read_text())['state'] in ['running','waiting-for-existing-deadline-run']:time.sleep(5)
env={**os.environ,'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','RUSTUP_TOOLCHAIN':'nightly-2026-09-16'}
commands=[('global-absence-native-focused',['cargo','test','--locked','-p','nurbs-core','surface_contact_search','--lib']),('global-absence-benchmark-build',['cargo','build','--locked','--release','-p','geometry-bridge','--example','bench_sweep_volume_request']),('global-absence-native-paired',['python3',str(out/'compare-global-absence.py')])]
for name,argv in commands:
 assert sha(stage/'crates/nurbs-core/src/intersection/surface_contact_search.rs')==r['sourceSha256'];r['state']='running';item={'name':name,'state':'running'};r['phases'].append(item);save();print(name+': started',flush=True)
 with (out/(name+'.log')).open('w') as log:result=subprocess.run(argv,cwd=stage/'crates',env=env,stdout=log,stderr=subprocess.STDOUT)
 item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode);save();print(name+': '+item['state'],flush=True)
 if result.returncode:r['state']='failed';save();raise SystemExit(result.returncode)
r.update(state='complete',passed=True);save()
