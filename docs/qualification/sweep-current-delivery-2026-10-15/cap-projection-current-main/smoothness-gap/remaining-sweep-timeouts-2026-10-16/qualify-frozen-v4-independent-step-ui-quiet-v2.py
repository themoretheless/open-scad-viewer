"""Fresh complete STEP/UI checks for the one frozen current-source artifact."""
from pathlib import Path
import os,time,json,hashlib,subprocess
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-frozen-current-v4-source-20261016');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();folder=out/'frozen-v4-independent-step-ui-quiet-v2';folder.mkdir(exist_ok=False);f=folder/'receipt.json';r={'state':'waiting-public100','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'phases':[],'fullGoalComplete':False}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
save()
while True:
 q=json.loads((out/'frozen-current-v4-qualification-quiet-v2.json').read_text())
 if q['state'] in ['complete','failed']:break
 time.sleep(5)
if not q['passed']:r.update(state='stopped-after-public-gate-failure',publicReceiptSha256=sha(out/'frozen-current-v4-qualification-quiet-v2.json'));save();raise SystemExit(1)
build=json.loads((out/'frozen-current-v4-build.json').read_text());assert build['passed'];packed=build['packedSha256'];ledger=json.loads((stage/'runtime-source-sha256.json').read_text());extra=json.loads((stage/'qualification-external-inputs.json').read_text());producer=stage/'crates/target/release/examples/certify_sweep_geometry_migration';assert producer.exists()
r.update(state='running',geometryWasmSha256=packed,publicReceiptSha256=sha(out/'frozen-current-v4-qualification-quiet-v2.json'),nativeProducerSha256=sha(producer));save()
def identity():
 for name,digest in ledger.items():assert sha(stage/name)==digest,name
 for name,meta in extra['files'].items():assert sha(stage/name)==meta['sha256'],name
 for name,digest in extra['browserDependencyFiles'].items():assert sha(Path(extra['browserDependenciesRealPath'])/name)==digest,name
 for name in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(stage/name)==packed,name
 assert sha(producer)==r['nativeProducerSha256']
def run(name,argv):
 identity();item={'name':name,'argv':list(map(str,argv)),'state':'running'};r['phases'].append(item);save();print(name+': started',flush=True);start=time.monotonic()
 env={**os.environ,'PATH':str(Path(node).parent)+os.pathsep+os.environ['PATH'],'CARGO_BUILD_JOBS':'2','RUST_TEST_THREADS':'2','CARGO_NET_OFFLINE':'true','RUSTUP_TOOLCHAIN':'nightly-2026-09-16','PYTHONDONTWRITEBYTECODE':'1','NODE_OPTIONS':'--max-old-space-size=8192'}
 with (folder/(name+'.log')).open('w') as log:result=subprocess.run(item['argv'],cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT)
 identity();item.update(state='passed' if result.returncode==0 else 'failed',returncode=result.returncode,seconds=time.monotonic()-start);save();print(name+': '+item['state'],flush=True)
 if result.returncode:r['state']='failed';save();raise SystemExit(result.returncode)
gates=folder/'artifact-gates';gates.mkdir();(gates/'receipt.json').write_text(json.dumps({'state':'complete','passed':True,'artifacts':{'packed':{'sha256':packed}},'qualificationReceiptSha256':r['publicReceiptSha256'],'buildReceiptSha256':sha(out/'frozen-current-v4-build.json'),'scope':'Authenticated current frozen build plus original100file runtime gate','fullGoalComplete':False},indent=2)+'\n')
run('constructed-multispan',[node,'--import','tsx','scripts/check-staged-constructed-multispan-sweep.mts',stage,folder/'constructed-multispan.json'])
run('multispan-packed',[node,'--import','tsx','scripts/check-staged-multispan-jets.mts',stage,folder/'multispan-packed.json'])
run('closed-multispan',[node,'--import','tsx','scripts/check-staged-closed-strip-jets.mts',stage,folder/'closed-multispan.json'])
canonical=folder/'canonical207-ui478'
run('canonical207-ui478',['python3','scripts/qualify-sweep-current-artifact.py','--output',canonical,'--artifact-gates',gates,'--wait-pid',str(os.getpid()),'--expected-wasm',packed,'--native-producer',producer,'--source-ledger','runtime-source-sha256.json','--qa-overrides','qa-source-overrides-current.json','--extra-input-ledger','qualification-external-inputs.json'])
run('retained-interiors-producer',['cargo','build','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_retained_surface_interiors'])
run('retained-interiors',['python3','scripts/qualify-retained-surface-interiors.py',canonical/'step207/manifest.json',canonical/'receipt.json',stage/'crates/target/release/examples/certify_retained_surface_interiors',stage/'crates/brep-core/examples/certify_retained_surface_interiors.rs',folder/'retained-interiors'])
c2=folder/'c2-step4';run('c2-step4-export',[node,'--import','tsx','--loader','./qualification-raw-imports.mjs','scripts/export-corrected-c2-step.mts',c2]);oracle='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
run('c2-step4-reference',[oracle,'scripts/reference-rational-boundary-volume.py',c2]);run('c2-step4-opencascade',[oracle,'scripts/verify-sweep-step-occt.py',c2])
run('c2-ui8',[node,'scripts/check-sweep-miter-matrix-browser.mjs',folder/'c2-ui8','--corrected-c2-body-matrix','--native-running-cancel','--gpu-backend=metal'])
r.update(state='complete',passed=True);save();print('Fresh finite STEP/UI and retained-interior checks passed; complete objective audit remains separate.',flush=True)
