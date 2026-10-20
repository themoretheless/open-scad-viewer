"""Resume only the missing unchanged photogrammetry build-product binding."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[2];DEST=OUT/'optimized'
STAGE=Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
NODE='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
prior=DEST/'preparation.json';old=json.loads(prior.read_text());assert old['state']=='failed'and old['error']=='dist-check'
assert all(p['state']=='passed'for p in old['phases'][:-1])and old['phases'][-1]['name']=='dist-check'
assert 'photogrammetry_wasm.wasm' in (DEST/'dist-check.log').read_text()and'ENOENT'in(DEST/'dist-check.log').read_text()
assert not (DEST/'preparation-resume.json').exists()
source=ROOT/'crates/target/wasm32-unknown-unknown/release/photogrammetry_wasm.wasm'
target=STAGE/'crates/target/wasm32-unknown-unknown/release/photogrammetry_wasm.wasm'
assert not target.exists()
assert sha(source)==sha(STAGE/'public/wasm/photogrammetry.wasm')
shutil.copy2(source,target)
record={'state':'running','passed':False,'driverSha256':sha(__file__),'previousReceiptSha256':sha(prior),
 'previousDriverSha256':sha(OUT/'prepare-optimized.py'),'externalUnchangedBuildProduct':{'source':str(source),'target':str(target),'sha256':sha(target)},
 'previousPhaseLogSha256':{p['name']:sha(DEST/(p['name']+'.log'))for p in old['phases']}}
try:
 (DEST/'preparation-resume.json').write_text(json.dumps(record,indent=2)+'\n')
 with (DEST/'dist-check-resumed.log').open('w')as log:r=subprocess.run([NODE,'scripts/verify-dist.mjs'],cwd=STAGE,stdout=log,stderr=subprocess.STDOUT)
 assert r.returncode==0,'Unchanged packaging verifier refused'
 kernel=old['geometryWasmSha256']
 for name in ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']:assert sha(STAGE/name)==kernel
 original=json.loads((ROOT/'docs/qualification/trimmed-branches-topology-2026-10-19/frozen-native-source-sha256.json').read_text())
 original['crates/nurbs-core/src/numerics/distance_bounds.rs']=sha(STAGE/'crates/nurbs-core/src/numerics/distance_bounds.rs')
 for name,h in original.items():assert sha(STAGE/name)==h,name
 ledger={}
 for parent in STAGE.iterdir():
  if parent.name in ['node_modules','tools','qualification-inputs.json','qa-source-overrides-current.json']:continue
  if parent.is_dir():
   paths=[]
   for folder,dirs,files in os.walk(parent):
    dirs[:]=[d for d in dirs if d not in ['target','__pycache__']]
    paths.extend(Path(folder)/f for f in files)
  else:paths=[parent]
  for p in paths:
   if p.is_file():ledger[str(p.relative_to(STAGE))]=sha(p)
 (STAGE/'qualification-inputs.json').write_text(json.dumps(ledger,indent=2)+'\n')
 snapshot=json.loads((OUT/'snapshot.json').read_text());snapshot.update(stage=str(STAGE),workspace=str(ROOT),wasmSha256=kernel,
  inputLedgerSha256=sha(STAGE/'qualification-inputs.json'),inputFiles=len(ledger),originalBaselineWasm=snapshot['wasmSha256'],
  purpose='Full qualification of measured Rust interval validation optimization; original source fixtures, proof quotas and 30-second deadlines preserved.')
 (DEST/'snapshot.json').write_text(json.dumps(snapshot,indent=2)+'\n')
 for name in ['run.py','supplements.py']:shutil.copy2(OUT/name,DEST/name)
 record.update(state='complete',passed=True,sourceLedgerSha256=sha(STAGE/'qualification-inputs.json'),geometryWasmSha256=kernel,
  distCheckSha256=sha(DEST/'dist-check-resumed.log'),nativeInputsChecked=len(original))
except BaseException as e:
 record.update(state='failed',passed=False,error=str(e));raise
finally:(DEST/'preparation-resume.json').write_text(json.dumps(record,indent=2)+'\n')
