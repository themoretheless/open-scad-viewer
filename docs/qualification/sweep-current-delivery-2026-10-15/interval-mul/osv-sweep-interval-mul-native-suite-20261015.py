"""Full native regressions after the already-running full finite qualification."""
from pathlib import Path
import json,hashlib,os,subprocess,time
stage=Path('/private/tmp/osv-sweep-interval-mul-wasm-20261015')
prior=Path('/private/tmp/osv-sweep-interval-mul-full-qualification-20261015/receipt.json')
out=Path('/private/tmp/osv-sweep-interval-mul-native-suite-20261015');out.mkdir(exist_ok=True)
assert not (out/'receipt.json').exists(),'Do not restart an existing native suite'
receipt={'state':'waiting-existing-full-qualification','passed':False,'phases':[],'scope':'Full default-feature native library regressions on exact frozen N source; existing ignored tests remain explicit, not claimed executed'}
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save():
 p=out/'receipt.json.writing';p.write_text(json.dumps(receipt,indent=2)+'\n');p.replace(out/'receipt.json')
def sources():
 ledger=json.loads((stage/'frozen-runtime-source-sha256.json').read_text());native={p:h for p,h in ledger.items() if p.startswith('crates/')}
 for p,h in native.items():assert sha(stage/p)==h,p
 return {'sourceLedgerSha256':sha(stage/'frozen-runtime-source-sha256.json'),'nativeFilesChecked':len(native)}
receipt['sourceBinding']=sources();receipt['driverSha256']=sha(__file__);save()
while True:
 d=json.loads(prior.read_text())
 if d['state']=='complete':assert d['passed'] is True;break
 if d['state'].startswith('stopped') or d['state']=='failed':receipt['state']='stopped-after-prior-failure';save();raise SystemExit(1)
 try:os.kill(96307,0)
 except ProcessLookupError:receipt['state']='prior-ended-without-complete-pass';save();raise SystemExit(1)
 time.sleep(5)
assert d['geometryWasmSha256']=='6cce95b887ded1d73c8ec1886828c2fe77bfd07b34c535a38c9203299a64aabe'
receipt.update(state='running',priorReceiptSha256=sha(prior));save()
commands=[('libraries',['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','nurbs-core','-p','brep-core','-p','geometry-bridge','--lib','--','--test-threads=2']),('native-migration-producer',['cargo','test','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_sweep_geometry_migration','--','--test-threads=2'])]
for name,argv in commands:
 assert sources()==receipt['sourceBinding'];assert sha(__file__)==receipt['driverSha256']
 p={'name':name,'state':'running','argv':argv};receipt['phases'].append(p);save();print(name+': started',flush=True);t=time.monotonic()
 with (out/(name+'.log')).open('w') as log:r=subprocess.run(argv,cwd=stage,env={**os.environ,'CARGO_BUILD_JOBS':'2','PYTHONDONTWRITEBYTECODE':'1'},stdout=log,stderr=subprocess.STDOUT)
 p.update(state='passed' if r.returncode==0 else 'failed',returncode=r.returncode,seconds=time.monotonic()-t);save();print(name+': '+p['state'],flush=True)
 if r.returncode:receipt['state']='failed';save();raise SystemExit(r.returncode)
assert sources()==receipt['sourceBinding'];receipt.update(state='complete',passed=True);save()
