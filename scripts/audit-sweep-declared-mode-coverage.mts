/** Join actual qualified UI sources to effective Rust sweep dispatch modes.
 * This inspects declarations and receipt bindings; it computes no geometry and
 * does not turn mode examples into an all-input correctness theorem.
 */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve,join} from 'node:path'
import {compileRushFrontend} from '../src/services/rushFrontend'

const root=process.cwd()
const output=process.argv[2]
assert(output,'Usage: node --import tsx scripts/audit-sweep-declared-mode-coverage.mts OUTPUT.json')
const folder=resolve(root,'docs/qualification/sweep-current-delivery-2026-10-15/cap-projection-current-main/smoothness-gap/remaining-sweep-timeouts-2026-10-16')
const base=join(folder,'frozen-v11r1-common-source-20261018-independent-step-ui-final/canonical207-ui478/ui478')
const inputs:Record<string,string>={}
function bytes(path:string) {
 const data=readFileSync(path)
 inputs[path]=createHash('sha256').update(data).digest('hex')
 return data
}
function read(path:string):any {return JSON.parse(bytes(path).toString())}
const delivery=read(join(folder,'v11-primary-delivery-20261018.json'))
assert.equal(delivery.state,'complete');assert.equal(delivery.passed,true)
const artifact=delivery.geometryWasmSha256
for(const path of ['public/wasm/geometry-kernel.wasm','src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm']) {
 const file=resolve(root,path);bytes(file);assert.equal(inputs[file],artifact)
}
const plan=read(join(base,'inventory-plan.json'))
const qualified=read(join(base,'full-reconciliation.json'))
assert.equal(qualified.passed,true);assert.equal(qualified.cases,478)
assert.equal(qualified.geometryWasmSha256,artifact)
const metadata=read(join(folder,'v11-step-source-metadata-20261018.json'))
const nativeBoundary=read(join(base,'../step207/complete-boundary.json'))
const nativeGlobal=read(join(base,'../step207/global.json'))
const nativeSeams=read(join(base,'../step207/finite-modes.json'))
for(const report of [nativeBoundary,nativeGlobal,nativeSeams]) {
 assert.equal(report.manifestSha256,metadata.manifestSha256)
 assert.equal(report.artifactProvenance.geometryWasmSha256,artifact)
}
assert.equal(nativeBoundary.passed,true);assert.equal(nativeGlobal.passed,true)
const nativeBySource=new Map<string,any[]>()
for(const item of Object.values(metadata.records) as any[]) {
 if(!item.sourceSha256)continue
 const boundary=nativeBoundary.cases.find((c:any)=>c.file===item.file)
 const global=nativeGlobal.cases.find((c:any)=>c.file===item.file)
 const seams=nativeSeams.smoothness.find((c:any)=>c.file===item.file)
 assert(boundary&&global&&seams)
 const records=nativeBySource.get(item.sourceSha256)??[]
 records.push({file:item.file,boundary,global,seams})
 nativeBySource.set(item.sourceSha256,records)
}
const witnesses=new Map<string,any>()
for(const group of qualified.groups) {
 const matrix=read(group.reportPath);assert.equal(inputs[group.reportPath],group.sha256)
 assert.equal(matrix.passed,true);assert.equal(matrix.geometryWasmSha256,artifact)
 const job=plan.jobs.find((job:any)=>job.name===group.name);assert(job)
 for(const item of job.sources) {
  const path=resolve(job.sourceDirectory,item.file)
  const source=bytes(path).toString();assert.equal(inputs[path],item.sha256)
  const cases=matrix.cases.filter((c:any)=>c.file===item.file)
  assert.deepEqual(cases.map((c:any)=>c.viewport.width).sort((a:number,b:number)=>a-b),[600,1440])
  assert(cases.every((c:any)=>c.status==='passed'&&c.sourceSha256===item.sha256))
  const existing=witnesses.get(item.sha256)
  const witness={file:item.file,path,sha256:item.sha256,source,
   positiveSolid:cases.every((c:any)=>c.solid?.faces>0),groups:[group.name]}
  if(existing) {
   existing.positiveSolid ||= witness.positiveSolid;existing.groups.push(group.name)
  } else witnesses.set(item.sha256,witness)
 }
}
const rows:any[]=[]
for(const witness of witnesses.values()) {
 const doc=compileRushFrontend(witness.source).document
 const nodes=new Map(doc.nodes.map(node=>[node.id,node]))
 for(const node of doc.nodes as any[]) {
  if(!['progressive_sweep','brep_progressive_sweep','brep_progressive_miter_sweep','brep_miter_sweep'].includes(node.op))continue
  const guided=!!node.orientation_guide
  const contact=node.contact_profile!==undefined||node.contact_parameter!==undefined
  const authored=!!node.frame_axis||!!node.frame_normal
  const miter=node.op.includes('miter')
  const orientation=(node.orientation??'rmf').replaceAll('_','-')
  const owner=miter?'miter':authored?'authored':contact?'contact':guided?'guided':orientation
  // Declared input shape is not a geometric closure or regularity proof.
  const path:any=nodes.get(node.inputs?.[0])
  const closure=node.closed!==undefined?(node.closed?'closed':'open'):
   path?.periodic?'periodic':path?.control_points?
    JSON.stringify(path.control_points[0])===JSON.stringify(path.control_points.at(-1))?'closed':'open':'unclassified'
  const stepWitnesses=nativeBySource.get(witness.sha256)??[]
  const nativeClosures=Array.from(new Set(stepWitnesses.map(c=>c.boundary.closed?'closed':'open')))
  rows.push({file:witness.file,sourcePath:witness.path,sourceSha256:witness.sha256,groups:witness.groups,nodeId:node.id,
   op:node.op,owner,orientation,spacing:node.spacing??'parameter',declaredClosure:closure,
   nativeClosures,nativeStepWitnesses:stepWitnesses,
   affine:!!node.axis_scale||!!node.center_law,authored,guided,contact,
   loops:node.loops?.length??null,circleCorrection:node.circle_correction_tolerance!==undefined,
   stationReconstruction:doc.nodes.some((n:any)=>['brep_smooth_sweep_stations','brep_smooth_miter_stations'].includes(n.op)),
   positiveSolid:witness.positiveSolid})
 }
}
const owners=Array.from(new Set(rows.map(row=>row.owner))).sort()
const summary=owners.map(owner=>({owner,bodyRows:rows.filter(row=>row.owner===owner&&row.op.startsWith('brep')).length,
 positiveBodyRows:rows.filter(row=>row.owner===owner&&row.positiveSolid).length,
 spacings:Array.from(new Set(rows.filter(row=>row.owner===owner).map(row=>row.spacing))).sort(),
 nativeClosures:Array.from(new Set(rows.filter(row=>row.owner===owner).flatMap(row=>row.nativeClosures))).sort(),
 declaredClosures:Array.from(new Set(rows.filter(row=>row.owner===owner).map(row=>row.declaredClosure))).sort()}))
const expected=['rmf','fixed','fixed-normal','frenet','corrected-frenet','authored','guided','contact','miter']
const missingOwners=expected.filter(owner=>!rows.some(row=>row.owner===owner&&row.positiveSolid))
const result={schema:'sweep-declared-mode-source-coverage/1',geometryWasmSha256:artifact,
 scope:'Actual original478 UI source declarations and positive Solid witnesses. Shape labels describe declarations only. Supplemental sources and direct API cases are separate; no Cartesian or universal correctness claim.',
 inputsSha256:inputs,sourceCount:witnesses.size,rows,summary,missingPositiveOwners:missingOwners,
 allDispatchOwnersHavePositiveWitness:missingOwners.length===0,fullGoalComplete:false}
writeFileSync(output,JSON.stringify(result,null,2)+'\n')
console.log(JSON.stringify({sourceCount:witnesses.size,rows:rows.length,summary,missingPositiveOwners:missingOwners},null,2))
