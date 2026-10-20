"""Fresh complete STEP/UI checks for the one frozen current-source artifact."""
from pathlib import Path
import os,time,json,hashlib,subprocess
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-common-source-v11-20261018');node='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();folder=out/'frozen-v11-common-source-20261018-independent-step-ui-final';folder.mkdir(exist_ok=False);f=folder/'receipt.json';r={'state':'waiting-public100','passed':False,'pid':os.getpid(),'driverSha256':sha(__file__),'phases':[],'fullGoalComplete':False}
def save():
 p=f.with_suffix('.writing');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(f)
try:
 save()
 while not (out/'v11-common-source-runtime-20261018.json').exists():time.sleep(2)
 while True:
  q=json.loads((out/'v11-common-source-runtime-20261018.json').read_text())
  if q['state'] in ['complete','failed','stopped-after-native-tests']:break
  time.sleep(5)
 if not q['passed']:r.update(state='stopped-after-public-gate-failure',publicReceiptSha256=sha(out/'v11-common-source-runtime-20261018.json'));save();raise SystemExit(1)
 build=json.loads((out/'frozen-current-v11-build.json').read_text());assert build['passed'];packed=build['packedSha256'];ledger=json.loads((stage/'runtime-source-sha256.json').read_text());extra=json.loads((stage/'qualification-external-inputs.json').read_text());producer=stage/'crates/target/release/examples/certify_sweep_geometry_migration';assert producer.exists()
 r.update(state='running',geometryWasmSha256=packed,publicReceiptSha256=sha(out/'v11-common-source-runtime-20261018.json'),nativeProducerSha256=sha(producer))
 checker_root=stage/'scripts'
 diagnostic_root=folder/'multispan';diagnostic_root.mkdir()
 qa_files=[Path(__file__),stage/'completed-build-receipt.json',stage/'runtime-source-sha256.json',stage/'main-native-source-sha256.json',stage/'qualification-external-inputs.json',out/'v11-common-source-runtime-20261018.json',out/'frozen-current-v11-build.json',*[checker_root/name for name in ['sweep-legacy-cap-refusal-contracts.mjs','test-sweep-legacy-cap-refusal-contracts.mjs','qualify-sweep-legacy-cap-refusals.py']]]
 r['additionalQaInputs']={str(path):sha(path) for path in qa_files};save()
 def identity():
  assert sha(__file__)==r['driverSha256']
  for name,digest in r['additionalQaInputs'].items():assert sha(name)==digest,name
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
 diagnostic_phases=[]
 for name,script in [('constructed','check-staged-constructed-multispan-sweep.mts'),('packed-jets','check-staged-multispan-jets.mts'),('closed-jets','check-staged-closed-strip-jets.mts')]:
  run('multispan-'+name,[node,'--import','tsx','scripts/'+script,stage,diagnostic_root/(name+'.json')])
  report=diagnostic_root/(name+'.json');assert json.loads(report.read_text())['passed']
  r['additionalQaInputs'][str(report)]=sha(report);diagnostic_phases.append(r['phases'][-1]);save()
 diagnostic_receipt=diagnostic_root/'receipt.json'
 diagnostic_receipt.write_text(json.dumps({'state':'complete','passed':True,'artifactSha256':packed,'sourceLedgerSha256':sha(stage/'runtime-source-sha256.json'),'completedBuildAdapterSha256':sha(stage/'completed-build-receipt.json'),'phases':diagnostic_phases,'scope':'Fresh constructed and synthetic multispan/closed-strip G1/G2 diagnostics only; no source-error, cap, BRep, Solid, STEP, UI or full-goal completion claim.','fullGoalComplete':False},indent=2)+'\n')
 r['additionalQaInputs'][str(diagnostic_receipt)]=sha(diagnostic_receipt);save()
 gates=folder/'artifact-gates';gates.mkdir();(gates/'receipt.json').write_text(json.dumps({'state':'complete','passed':True,'artifacts':{'packed':{'sha256':packed}},'qualificationReceiptSha256':r['publicReceiptSha256'],'buildReceiptSha256':sha(out/'frozen-current-v11-build.json'),'scope':'Authenticated current frozen build plus original100file runtime gate','fullGoalComplete':False},indent=2)+'\n')
 canonical=folder/'canonical207-ui478'
 run('canonical207-ui478',['python3','scripts/qualify-sweep-current-artifact.py','--output',canonical,'--artifact-gates',gates,'--wait-pid',str(os.getpid()),'--expected-wasm',packed,'--native-producer',producer,'--source-ledger','runtime-source-sha256.json','--qa-overrides','qa-source-overrides-current.json','--extra-input-ledger','qualification-external-inputs.json'])
 run('legacy-cap-refusal-reconciliation',['python3',checker_root/'qualify-sweep-legacy-cap-refusals.py','--canonical',canonical,'--expected-wasm',packed,'--output',folder/'legacy-cap-refusals','--checker-directory',checker_root,'--node',node])
 run('retained-interiors-producer',['cargo','build','--offline','--locked','--release','--manifest-path','crates/Cargo.toml','-p','brep-core','--example','certify_retained_surface_interiors'])
 run('retained-interiors',['python3','scripts/qualify-retained-surface-interiors.py',canonical/'step207/manifest.json',canonical/'receipt.json',stage/'crates/target/release/examples/certify_retained_surface_interiors',stage/'crates/brep-core/examples/certify_retained_surface_interiors.rs',folder/'retained-interiors'])
 c2=folder/'c2-step4';run('c2-step4-export',[node,'--import','tsx','--loader','./qualification-raw-imports.mjs','scripts/export-corrected-c2-step.mts',c2]);oracle='/Users/themoretheless/.codex/tmp/osv-step-oracle-20261013/bin/python'
 run('c2-step4-reference',[oracle,'scripts/reference-rational-boundary-volume.py',c2]);run('c2-step4-opencascade',[oracle,'scripts/verify-sweep-step-occt.py',c2])
 run('c2-ui8',[node,'scripts/check-sweep-miter-matrix-browser.mjs',folder/'c2-ui8','--corrected-c2-body-matrix','--native-running-cancel','--gpu-backend=metal'])
 r.update(state='complete',passed=True);save();print('Fresh finite STEP/UI and retained-interior checks passed; complete objective audit remains separate.',flush=True)
except Exception as error:
 r.update(state="failed",passed=False,reason=str(error));save();raise
