import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'
import {smoothCapGapCompanions} from './sweep-automatic-cap-contracts.mjs'

const sha=b=>createHash('sha256').update(b).digest('hex')
const canonical=v=>v===null||typeof v!=='object'?v:Array.isArray(v)?v.map(canonical):Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]))
const fingerprint=v=>sha(Buffer.from(JSON.stringify(canonical(v))))

// Qualification correspondence only; no geometry decisions are made in JS.
// Each native input is bound to BOTH historical fingerprints and actual current
// coefficient arrays. An edited model cannot reuse a previous displacement.
export function loadGeometryMigration(reviewPath,{manifestSha256,baselineSha256,baseline,producerPath}){
 const review=JSON.parse(readFileSync(reviewPath))
 assert.equal(review.schema,'sweep-reviewed-rounding-migration/1')
 assert.equal(review.change,'shared-math-fused-multiply-add')
 assert.equal(review.currentManifestSha256,manifestSha256)
 assert.equal(review.originalBaselineSha256,baselineSha256)
 assert.equal(review.originalManifestSha256,baseline.sourceManifestSha256)
 const budgetBytes=readFileSync(resolve(dirname(reviewPath),review.originalBudgetLedger))
 assert.equal(sha(budgetBytes),review.originalBudgetLedgerSha256)
 const budgetLedger=JSON.parse(budgetBytes)
 assert.equal(budgetLedger.schema,'sweep-original-authored-budget-capture/1')
 assert.equal(budgetLedger.originalManifestSha256,baseline.sourceManifestSha256)
 assert.equal(budgetLedger.extractorSha256,sha(readFileSync(new URL('./capture-sweep-original-budgets.py',import.meta.url))))
 assert.deepEqual(budgetLedger.cases.map(c=>c.file).sort(),[...review.files].sort())
 const originalBudgets=new Map(budgetLedger.cases.map(c=>[c.file,c]))
 assert.equal(originalBudgets.size,budgetLedger.cases.length)
 const producerSha256=sha(readFileSync(producerPath)),entries=new Map()
 const previous=new Map(baseline.cases.map(c=>[c.file,c]))
 const bindings=[]
 for(const batch of review.batches){
  const reqBytes=readFileSync(resolve(dirname(reviewPath),batch.request)),outBytes=readFileSync(resolve(dirname(reviewPath),batch.output))
  assert.equal(sha(reqBytes),batch.requestSha256);assert.equal(sha(outBytes),batch.outputSha256)
  const request=JSON.parse(reqBytes),output=JSON.parse(outBytes)
  assert.equal(request.currentManifestSha256,manifestSha256)
  assert.equal(request.originalBaselineSha256,baselineSha256)
  assert.equal(request.originalManifestSha256,baseline.sourceManifestSha256)
  assert.ok(Number.isSafeInteger(request.maxWork)&&request.maxWork>=0&&request.maxWork<=1000000)
  assert.equal(output.schema,'sweep-retained-evaluation-migration/1')
  assert.equal(output.requestSha256,batch.requestSha256)
  assert.equal(output.producerSha256,producerSha256)
  assert.equal(output.passed,true);assert.equal(output.globalEmbeddingCertified,false)
  assert.deepEqual(output.cases.map(c=>c.file),request.cases.map(c=>c.file))
  for(const [i,req] of request.cases.entries()){
   assert.ok(previous.has(req.file));assert.ok(!entries.has(req.file),'Repeated migration case')
   const result=output.cases[i]
   assert.equal(result.certified,true);assert.equal(result.withinOriginalBudget,true);assert.equal(result.reason,null)
   assert.ok(Number.isSafeInteger(result.work)&&result.work>0&&result.work<=request.maxWork)
   assert.equal(result.maxWork,request.maxWork)
   assert.ok(Number.isFinite(result.upperMm)&&result.upperMm>=0)
   assert.ok(Number.isFinite(result.composedUpperMm)&&result.composedUpperMm>=req.currentBoundaryUpperMm)
   assert.ok(result.composedUpperMm<=req.originalBudgetMm)
   assert.deepEqual(result.subjects.map(s=>s.id),req.subjects.map(s=>s.id))
   assert.equal(new Set(req.subjects.map(s=>s.id)).size,req.subjects.length)
   assert.equal(Math.max(...result.subjects.map(s=>s.upperMm)),result.upperMm)
   assert.ok(result.subjects.every(s=>Number.isFinite(s.upperMm)&&s.upperMm>=0))
   entries.set(req.file,{request:req,result,previous:previous.get(req.file)})
  }
  bindings.push({requestSha256:batch.requestSha256,outputSha256:batch.outputSha256})
 }
 assert.deepEqual([...entries.keys()].sort(),[...review.files].sort())
 assert.equal(new Set(review.files).size,review.files.length)
 return {
  provenance:{reviewSha256:sha(readFileSync(reviewPath)),producerSha256,originalBudgetLedgerSha256:review.originalBudgetLedgerSha256,bindings},
  has:file=>entries.has(file),
  check(item){
   const entry=entries.get(item.file);assert.ok(entry,'Missing reviewed native migration')
   const {request:req,result,previous}=entry
   const walls=req.subjects.filter(s=>s.kind==='surface'),edges=req.subjects.filter(s=>s.kind==='curve')
   assert.equal(walls.length+edges.length,req.subjects.length)
   const radialUpgrade=smoothCapGapCompanions.has(item.file)&&item.capRepresentation==='radial-annular'
   assert.equal(item.wallSurfaces.length,walls.length+(radialUpgrade?8:0),'Uncovered current retained surface')
   assert.equal(item.edgeCurves.length,edges.length+(radialUpgrade?8:0),'Uncovered current retained edge')
   assert.deepEqual(req.subjects.map(s=>s.id),[...walls.map((_,i)=>`wall:${i}`),...edges.map((_,i)=>`edge:${i}`)])
   assert.equal(fingerprint(walls.map(s=>s.original)),previous.retainedGeometryFingerprints.wallSurfaces,'Historical wall fingerprint differs')
   assert.equal(fingerprint(edges.map(s=>s.original)),previous.retainedGeometryFingerprints.edgeCurves,'Historical edge fingerprint differs')
   assert.deepEqual(walls.map(s=>s.current),item.wallSurfaces.slice(0,walls.length),'Current walls differ from native proof input')
   assert.deepEqual(edges.map(s=>s.current),item.edgeCurves.slice(0,edges.length).map(e=>e.curve),'Current edges differ from native proof input')
   const bound=checkCompleteBoundaryContract(item)
   assert.equal(bound?.status,'certified')
   assert.equal(bound.upper,req.currentBoundaryUpperMm)
   assert.equal(bound.budget,req.originalBudgetMm)
   const originalBudget=originalBudgets.get(item.file)
   assert.ok(originalBudget.originalPaths.length>0)
   assert.equal(originalBudget.budgetMm,req.originalBudgetMm)
   if(previous.completeBoundaryBudget!==null)assert.equal(previous.completeBoundaryBudget,req.originalBudgetMm)
   return {file:item.file,reviewedRetainedEvaluationMigration:true,
    correspondenceUpperMm:result.upperMm,composedUpperMm:result.composedUpperMm,
    originalBudgetMm:req.originalBudgetMm,work:result.work,maxWork:result.maxWork,
    walls:walls.length,edges:edges.length,
    scope:'Reviewed coefficient rounding migration on the entire common natural domain; exact historical model equality is not claimed. Cap replacement, source identity, topology, global embedding and actual STEP remain separate gates.'}
  },
  files:[...entries.keys()],
 }
}
