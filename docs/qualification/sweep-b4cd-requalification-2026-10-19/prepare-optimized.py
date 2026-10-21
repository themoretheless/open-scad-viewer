"""Prepare a private v5 runtime without rebinding published v4 manifests."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,time
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[2]
STAGE=Path('/private/tmp/osv-sweep-b4cd-interval-candidate-20261019')
BASE=Path('/private/tmp/osv-sweep-b4cd-requalification-20261019')
NODE='/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
paired=json.loads((OUT/'paired-interval/receipt.json').read_text());assert paired['state']=='complete'and paired['passed']
DEST=OUT/'optimized';DEST.mkdir(exist_ok=False)
state={'state':'preparing-private-runtime','passed':False,'driverSha256':sha(__file__),
       'pairedReceiptSha256':sha(OUT/'paired-interval/receipt.json'),'phases':[]}
env={**os.environ,'PATH':str(Path(NODE).parent)+os.pathsep+os.environ['PATH'],
     'NODE_OPTIONS':'--max-old-space-size=8192','VITEST_MAX_WORKERS':'1','PYTHONDONTWRITEBYTECODE':'1'}
def save():(DEST/'preparation.json').write_text(json.dumps(state,indent=2)+'\n')
def run(name,argv,cwd=STAGE):
 p={'name':name,'argv':list(map(str,argv)),'state':'running'};state['phases'].append(p);save();print(name+': started',flush=True);start=time.monotonic()
 with (DEST/(name+'.log')).open('w')as log:result=subprocess.run(p['argv'],cwd=cwd,env=env,stdout=log,stderr=subprocess.STDOUT)
 p.update(state='passed'if result.returncode==0 else'failed',returncode=result.returncode,seconds=time.monotonic()-start);save();assert result.returncode==0,name
def replace(path,before,after):
 text=path.read_text();assert text.count(before)==1,(str(path),before);path.write_text(text.replace(before,after))
try:
 save()
 dump="import{writeFileSync}from'node:fs';import{GEOMETRY_MANIFEST_ARCHIVE as a}from'./src/core/geometryExecution.ts';writeFileSync(process.argv[1],JSON.stringify({mesh:a['own-rust-node-v4'],brep:a['brep-closed-v4']},null,2)+'\\n')"
 run('baseline-v4-capture',[NODE,'--import','tsx','--loader',BASE/'qualification-raw-imports.mjs','--input-type=module','-e',dump,DEST/'baseline-v4.json'],BASE)
 old=(BASE/'src/core/currentRuntimeGeometryEvidence.ts').read_text()
 (STAGE/'src/core/runtimeGeometryEvidenceV4.ts').write_text(old.replace('CURRENT_RUNTIME_GEOMETRY_EVIDENCE','RUNTIME_GEOMETRY_EVIDENCE_V4'))
 kernel=sha(STAGE/'public/wasm/geometry-kernel.wasm');assert kernel!=paired['inputsSha256'][str(BASE/'public/wasm/geometry-kernel.wasm')]
 identity=json.loads(old[old.index('{'):old.rindex('}')+1]);identity.update(wasmSha256=kernel,
  recordId='docs/qualification/runtime-provider-'+kernel[:12]+'-2026-10-19.json',corpusVersion='runtime-provider-'+kernel[:12]+'-2026-10-19')
 for path,key in [('THIRD_PARTY_NOTICES.md','noticesSha256'),('package-lock.json','lockfileSha256'),('crates/Cargo.lock','rustLockfileSha256')]:
  target=STAGE/path
  if not target.exists():shutil.copy2(ROOT/path,target)
  assert sha(target)==identity[key],path
 (STAGE/'src/core/currentRuntimeGeometryEvidence.ts').write_text('// Private candidate: publish only with its completed exact-artifact qualification.\nexport const CURRENT_RUNTIME_GEOMETRY_EVIDENCE = Object.freeze('+json.dumps(identity,indent=2)+' as const)\n')
 geometry=STAGE/'src/core/geometryExecution.ts'
 replace(geometry,"import { RUNTIME_GEOMETRY_EVIDENCE_V3 } from './runtimeGeometryEvidenceV3'",
  "import { RUNTIME_GEOMETRY_EVIDENCE_V3 } from './runtimeGeometryEvidenceV3'\nimport { RUNTIME_GEOMETRY_EVIDENCE_V4 } from './runtimeGeometryEvidenceV4'")
 for stem,key,engine in [('ownRustCad','own-rust-node','own-rust-cad'),('brepClosed','brep-closed','rust-brep-closed')]:
  original=f"const {stem}RuntimeV4 = bindCurrentRuntime({stem}ManifestV1, '{key}-v4', '{engine}-v4', CURRENT_RUNTIME_GEOMETRY_EVIDENCE)"
  revised=original.replace('CURRENT_RUNTIME_GEOMETRY_EVIDENCE','RUNTIME_GEOMETRY_EVIDENCE_V4')+'\n'+original.replace('RuntimeV4','RuntimeV5').replace("-v4'","-v5'")
  replace(geometry,original,revised)
 replace(geometry,'export const GEOMETRY_MANIFEST_ARCHIVE = deepFreeze({',
  "export const GEOMETRY_MANIFEST_ARCHIVE = deepFreeze({\n  'own-rust-node-v5': ownRustCadRuntimeV5,\n  'brep-closed-v5': brepClosedRuntimeV5,")
 replace(geometry,"  mesh: 'own-rust-node-v4',","  mesh: 'own-rust-node-v5',")
 replace(geometry,"  brep: 'brep-closed-v4',","  brep: 'brep-closed-v5',")
 test=STAGE/'tests/engineManifest.test.ts'
 replace(test,"expect(manifest.capabilityManifestVersion).toBe('own-rust-node-v4')","expect(manifest.capabilityManifestVersion).toBe('own-rust-node-v5')")
 replace(test,"    expect(archivedGeometryManifest('missing-v1')).toBeNull()",
  "    expect(archivedGeometryManifest('own-rust-node-v4')?.kernelFingerprint).toBe('sha256:"+paired['inputsSha256'][str(BASE/'public/wasm/geometry-kernel.wasm')]+"')\n    expect(archivedGeometryManifest('brep-closed-v4')?.kernelFingerprint).toBe('sha256:"+paired['inputsSha256'][str(BASE/'public/wasm/geometry-kernel.wasm')]+"')\n    expect(archivedGeometryManifest('missing-v1')).toBeNull()")
 replace(STAGE/'tests/manifoldPlanBackend.test.ts',"      'core/runtimeGeometryEvidenceV3.ts',","      'core/runtimeGeometryEvidenceV3.ts',\n      'core/runtimeGeometryEvidenceV4.ts',")
 run('preserved-v4-capture',[NODE,'--import','tsx','--loader',STAGE/'qualification-raw-imports.mjs','--input-type=module','-e',dump,DEST/'preserved-v4.json'])
 assert sha(DEST/'baseline-v4.json')==sha(DEST/'preserved-v4.json')
 state.update(geometryWasmSha256=kernel,preservedV4Sha256=sha(DEST/'preserved-v4.json'),privateEvidence=identity);save()
 provider=json.loads((ROOT/'docs/qualification/trimmed-branches-topology-2026-10-19/provider-baseline.json').read_text())
 files=[str(Path(t['name']).relative_to(ROOT))for t in provider['testResults']];assert len(files)==19
 run('provider303',[NODE,'node_modules/vitest/vitest.mjs','run','--maxWorkers=1','--no-file-parallelism',*files,'--reporter=default','--reporter=json','--outputFile='+str(DEST/'provider303.json')])
 result=json.loads((DEST/'provider303.json').read_text());assert result['success']and result['numPassedTests']==result['numTotalTests']==303
 run('runtime-providers',[NODE,'--import','tsx','--loader',STAGE/'qualification-raw-imports.mjs','scripts/audit-runtime-manifests.mts'])
 run('types',[NODE,'node_modules/vue-tsc/bin/vue-tsc.js','--noEmit'])
 run('mcp-types',[NODE,'node_modules/typescript/bin/tsc','--noEmit','-p','tsconfig.mcp.json'])
 run('dist',[NODE,'node_modules/vite/bin/vite.js','build'])
 run('dist-check',[NODE,'scripts/verify-dist.mjs'])
 assert sha(STAGE/'dist/wasm/geometry-kernel.wasm')==kernel
 ledger={}
 for parent in STAGE.iterdir():
  if parent.name in ['node_modules','tools','qualification-inputs.json','qa-source-overrides-current.json']:continue
  for p in parent.rglob('*')if parent.is_dir()else[parent]:
   if p.is_file()and'__pycache__'not in p.parts and'target'not in p.relative_to(STAGE).parts:ledger[str(p.relative_to(STAGE))]=sha(p)
 (STAGE/'qualification-inputs.json').write_text(json.dumps(ledger,indent=2)+'\n')
 snapshot=json.loads((OUT/'snapshot.json').read_text());snapshot.update(stage=str(STAGE),workspace=str(ROOT),wasmSha256=kernel,
  inputLedgerSha256=sha(STAGE/'qualification-inputs.json'),inputFiles=len(ledger),originalBaselineWasm=paired['inputsSha256'][str(BASE/'public/wasm/geometry-kernel.wasm')],
  purpose='Full qualification of the measured Rust interval validation optimization; unchanged original test geometry and 30-second deadlines.')
 (DEST/'snapshot.json').write_text(json.dumps(snapshot,indent=2)+'\n')
 for name in ['run.py','supplements.py']:shutil.copy2(OUT/name,DEST/name)
 state.update(state='complete',passed=True,sourceLedgerSha256=sha(STAGE/'qualification-inputs.json'));save()
except BaseException as error:
 state.update(state='failed',passed=False,error=str(error));save();raise
