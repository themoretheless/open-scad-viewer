import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync,existsSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve,join,basename} from 'node:path'
import {spawnSync} from 'node:child_process'
import {readSweepManifest} from './sweep-manifest-reader.mjs'
import {checkOriginalRequestPreservation} from './sweep-original-request-preservation.mjs'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
import {smoothCapGapCompanions} from './sweep-automatic-cap-contracts.mjs'
import {loadGeometryMigration} from './sweep-geometry-migration.mjs'

// Prepare a FRESH native comparison for this exact manifest. An earlier native
// result must never be rebound to a new artifact or changed coefficient input.
const [manifestPath,baselinePath,projectionPath,budgetLedgerPath,nativeBinary,outputPath]=process.argv.slice(2)
assert.ok(outputPath,'Supply manifest, original baseline/projection/budgets, frozen native producer and output directory')
const sha=b=>createHash('sha256').update(b).digest('hex')
const canonical=v=>v===null||typeof v!=='object'?v:Array.isArray(v)?v.map(canonical):Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]))
const fingerprint=v=>sha(Buffer.from(JSON.stringify(canonical(v))))
const bytes=readFileSync(manifestPath),manifest=readSweepManifest(manifestPath,bytes)
const baselineBytes=readFileSync(baselinePath),baseline=JSON.parse(baselineBytes)
const projectionBytes=readFileSync(projectionPath),projection=JSON.parse(projectionBytes)
const budgets=JSON.parse(readFileSync(budgetLedgerPath))
assert.equal(baseline.cases.length,157);assert.equal(manifest.cases.length,207)
assert.equal(new Set(manifest.cases.map(c=>c.file)).size,207)
assert.equal(budgets.originalManifestSha256,baseline.sourceManifestSha256)
const originals=new Map(projection.map(c=>[c.file,c])),originalBudgets=new Map(budgets.cases.map(c=>[c.file,c]))
assert.equal(originals.size,157);assert.equal(originalBudgets.size,budgets.cases.length)
const cases=[]
for(const previous of baseline.cases){
 const old=originals.get(previous.file),item=manifest.cases.find(c=>c.file===previous.file)
 assert.ok(old&&item,'Missing original case')
 for(const [field,hash] of Object.entries(previous.retainedGeometryFingerprints))assert.equal(fingerprint(old[field]??null),hash,`${previous.file}: original projection differs`)
 const radialUpgrade=smoothCapGapCompanions.has(item.file)&&item.capRepresentation==='radial-annular'
 if(!radialUpgrade){
  const preserved=checkOriginalRequestPreservation(item,previous)
  if(preserved.capturedRetainedGeometryPreserved)continue
  assert.ok(preserved.geometryChanges.every(c=>c.field==='wallSurfaces'||c.field==='edgeCurves'),'Rounding migration cannot cover topology, pcurve or orientation changes')
 }
 const current=checkCompleteBoundaryContract(item),budget=originalBudgets.get(item.file)
 assert.equal(current?.status,'certified');assert.ok(budget,'Original budget extraction missing')
 assert.equal(current.budget,budget.budgetMm,'Original authored budget changed')
 assert.equal(previous.requestAndTopology.nativeAuditBudgets.maxExactWork,1000000)
 assert.equal(item.wallSurfaces.length,old.wallSurfaces.length+(radialUpgrade?8:0))
 assert.equal(item.edgeCurves.length,old.edgeCurves.length+(radialUpgrade?8:0))
 const subjects=[
  ...old.wallSurfaces.map((original,i)=>({id:`wall:${i}`,kind:'surface',original,current:item.wallSurfaces[i]})),
  ...old.edgeCurves.map((original,i)=>({id:`edge:${i}`,kind:'curve',original,current:item.edgeCurves[i].curve})),
 ]
 assert.ok(subjects.length>0)
 cases.push({file:item.file,currentBoundaryUpperMm:current.upper,originalBudgetMm:budget.budgetMm,subjects})
}
assert.ok(cases.length>0,'No migration review is needed for an unchanged matrix')
const root=resolve(outputPath);mkdirSync(root,{recursive:true})
assert.equal(existsSync(join(root,'review.json')),false,'Do not overwrite an existing review')
const common={maxWork:1000000,currentManifestSha256:sha(bytes),originalManifestSha256:baseline.sourceManifestSha256,originalBaselineSha256:sha(baselineBytes)}
const selectedBudgets={...budgets,cases:cases.map(c=>originalBudgets.get(c.file))}
writeFileSync(join(root,'original-budgets.json'),JSON.stringify(selectedBudgets,null,2)+'\n')
const review={schema:'sweep-reviewed-rounding-migration/1',change:'shared-math-fused-multiply-add',
 ...Object.fromEntries(Object.entries(common).filter(([key])=>key!=='maxWork')),
 originalBudgetLedger:'original-budgets.json',originalBudgetLedgerSha256:sha(readFileSync(join(root,'original-budgets.json'))),
 files:cases.map(c=>c.file),batches:[],
 scope:'Fresh exact-input native full-domain correspondence, separately composed within original authored budgets; not exact model equality or global admission.'}
const pending=[];let batch=[]
// Preserve the native codec's 32 MiB document limit and every per-case quota.
for(const c of cases){
 const next=[...batch,c]
 if(Buffer.byteLength(JSON.stringify({...common,cases:next}))>31*1024*1024){assert.ok(batch.length);pending.push(batch);batch=[c]}
 else batch=next
 assert.ok(Buffer.byteLength(JSON.stringify({...common,cases:batch}))<=31*1024*1024,'Individual native comparison exceeds original document limit')
}
if(batch.length)pending.push(batch)
for(const [i,group] of pending.entries()){
 const request=join(root,`request-${i}.json`),output=join(root,`native-${i}.json`)
 assert.equal(existsSync(request)||existsSync(output),false,'Preserve earlier native batch attempts')
 writeFileSync(request,JSON.stringify({...common,cases:group}))
 const result=spawnSync(resolve(nativeBinary),[request,output],{encoding:'utf8'})
 writeFileSync(join(root,`native-${i}.log`),(result.stdout??'')+(result.stderr??''))
 if(result.error)throw result.error
 assert.equal(result.status,0,'Fresh native migration proof failed; output retained, no relaxed acceptance')
 review.batches.push({request:basename(request),requestSha256:sha(readFileSync(request)),output:basename(output),outputSha256:sha(readFileSync(output))})
}
writeFileSync(join(root,'review.json'),JSON.stringify(review,null,2)+'\n')
const bound=loadGeometryMigration(join(root,'review.json'),{manifestSha256:sha(bytes),baselineSha256:sha(baselineBytes),baseline,producerPath:new URL('../crates/brep-core/examples/certify_sweep_geometry_migration.rs',import.meta.url)})
for(const file of bound.files)bound.check(manifest.cases.find(c=>c.file===file))
writeFileSync(join(root,'preparation.json'),JSON.stringify({passed:true,scope:review.scope,
 nativeBinarySha256:sha(readFileSync(resolve(nativeBinary))),originalProjectionSha256:sha(projectionBytes),
 currentManifestSha256:sha(bytes),cases:cases.length,batches:pending.length,provenance:bound.provenance,
 preparerSha256:sha(readFileSync(new URL(import.meta.url)))},null,2)+'\n')
console.log(`${cases.length} fresh native whole-domain migration comparisons bound to exact current and original geometry`)
