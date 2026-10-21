from pathlib import Path
import hashlib,json,os,subprocess,sys,time
out=Path(__file__).resolve().parent;stage=Path('/private/tmp/osv-sweep-common-source-v11-20261018');parent=out/'frozen-v11r1-common-source-20261018-independent-step-ui-final';receipt=out/'v11-companion27-followup-20261018.json';assert not receipt.exists()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();checker=out/'check_companion27_contracts.py';tests=out/'companion27-checker-tests-20261018.json';test_data=json.loads(tests.read_text());require=lambda value,message: None if value else (_ for _ in ()).throw(AssertionError(message))
require(test_data['passed'] is True and test_data['testGroups']==8,'Checker unit controls are incomplete');require(test_data['checkerSha256']==sha(checker),'Checker changed after tests')
buildpath=out/'frozen-current-v11-build.json';build=json.loads(buildpath.read_text());require(build['passed'] is True,'Current common source is unbuilt');expected=build['packedSha256'];files=[Path(__file__),checker,tests,out/'test_companion27_contracts.py',out/'companion27-historical-checker-fixture-20261018.json',buildpath,stage/'scripts/spatial-rmf-profile-g2-sources.mjs'];inputs={str(p):sha(p) for p in files}
r={'schema':'sweep-v11-companion27-followup/1','state':'waiting-independent-parent','passed':False,'pid':os.getpid(),'geometryWasmSha256':expected,'inputsSha256':inputs,'fullGoalComplete':False}
def save():
 tmp=receipt.with_suffix('.writing');tmp.write_text(json.dumps(r,indent=2)+'\n');tmp.replace(receipt)
def identity():
 require(all(sha(Path(p))==h for p,h in inputs.items()),'Supplemental audit inputs changed')
 require(all(sha(stage/n)==expected for n in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']),'Current candidate WASM changed')
save()
try:
 while True:
  q=json.loads((parent/'receipt.json').read_text())
  if q['state']=='complete' and q['passed'] is True:break
  if q['state'] in ['failed','superseded','stopped-after-public-gate-failure']:
   r.update(state='stopped-after-parent-failure',reason=q['state']);save();raise SystemExit(1)
  time.sleep(5)
 identity();r.update(state='checking-current27',independentParentSha256=sha(parent/'receipt.json'));save();resultpath=out/'v11-companion27-current-contracts-20261018.json';require(not resultpath.exists(),'Current companion output already exists')
 with (out/'v11-companion27-current-contracts-20261018.log').open('w') as log:
  result=subprocess.run(['nice','-n','19',sys.executable,str(checker),str(parent/'canonical207-ui478'),str(stage),expected,str(resultpath)],stdout=log,stderr=subprocess.STDOUT)
 identity();require(sha(parent/'receipt.json')==r['independentParentSha256'],'Independent parent changed');require(result.returncode==0,'Current27 companion contracts failed; inspect preserved log')
 checked=json.loads(resultpath.read_text());require(checked['passed'] is True and checked['geometryWasmSha256']==expected and len(checked['cases'])==27,'Incomplete current companion result')
 r.update(state='complete',passed=True,reportSha256=sha(resultpath),cases=27);save()
except Exception as error:r.update(state='failed',passed=False,reason=str(error));save();raise
