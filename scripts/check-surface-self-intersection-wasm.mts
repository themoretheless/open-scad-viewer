import {quadricGraphCases,graphQuery,boundaryQuery} from '../tools/browser-qualification/quadric-graph-fixtures.ts'
import {isOriginalQuadricGraphReport,type OriginalQuadricGraphReport} from '../src/services/originalQuadricGraphProtocol.ts'
import {quadricCarrierCases} from '../tools/browser-qualification/quadric-carrier-fixtures.ts'
import {isOriginalQuadricCarrierReport} from '../src/services/originalQuadricCarrierProtocol.ts'
import {commonRootPatchCases} from '../tools/browser-qualification/common-root-patch-fixtures.ts'
import {advancedPatchCases} from '../tools/browser-qualification/advanced-patch-fixtures.ts'
import {isOriginalPatchContactsReport} from '../src/services/originalPatchContactsProtocol.ts'
import {originalPatchFixture,originalPatchKnotFixture,originalPatchAlgebraicBoundaryFixture} from '../tools/browser-qualification/original-patch-fixtures.ts'
import assert from 'node:assert/strict'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'
import {isSurfaceIsoTargetReport} from '../src/services/surfaceIsoTargetProtocol.ts'
import {isSurfaceIsoAuditReport} from '../src/services/surfaceIsoSelfIntersectionProtocol.ts'
import {isSurfaceAuditReport} from '../src/services/surfaceSelfIntersectionProtocol.ts'
import {nonlinearSurfaceCases,spatialSurfaceCases,isoTargetFixture,offCenterSpatialSurface} from '../tools/browser-qualification/surface-self-fixtures.ts'
import { encodeBinary, decodeBinary } from '../src/services/valueBinaryCodec.ts'
const [wasmPath, directory, sourceCommit] = process.argv.slice(2)
assert(wasmPath && directory && /^[a-f0-9]{40}$/.test(sourceCommit ?? ''))
mkdirSync(directory, { recursive: true })
const bytes = readFileSync(wasmPath)
const module = await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module), [])
const instance = await WebAssembly.instantiate(module, {})
const w = instance.exports as unknown as { memory: WebAssembly.Memory; abi_alloc(n:number):number; abi_free(p:number,n:number):void; abi_request(op:number,p:number,n:number):bigint }
const cases: object[] = []
function request(v:unknown):any {
  const data = encodeBinary(v), original = data.slice(), ptr = w.abi_alloc(data.length)
  assert(ptr)
  new Uint8Array(w.memory.buffer, ptr, data.length).set(data)
  try {
    const packed = w.abi_request(0, ptr, data.length), p = Number(packed & 0xffffffffn), n = Number(packed >> 32n)
    try {
      const result = decodeBinary(new Uint8Array(w.memory.buffer, p, n))
      assert.deepEqual(new Uint8Array(w.memory.buffer, ptr, data.length), original)
      return result
    } finally { w.abi_free(p, n) }
  } finally { w.abi_free(ptr, data.length) }
}
const op = 'nurbs_surface_self_intersection_audit'
function run(name:string, surface:object, status:string, extra:object = {}):any {
  const v = { op, surface, maxSpans:100, ...extra }, before = JSON.stringify(v), start = performance.now()
  const reply = request(v)
  assert.equal(JSON.stringify(v), before)
  assert.equal(reply.ok, true, `${name}: ${JSON.stringify(reply.error)}`)
  const r = reply.value
  assert.equal(r.schema, 'nurbs-surface-self-intersection/1')
  assert.equal(r.status, status)
  assert.equal(r.presenceProven, status === 'present')
  assert.equal(r.absenceProven, status === 'absent')
  assert.equal(r.allContactBranchesEnumerated, false)
  assert(isSurfaceAuditReport(r))
  if (status === 'present') assert(r.exactWitness||r.certifiedCrossing)
  else { assert.equal(r.exactWitness, null); assert.equal(r.witness, null) }
  cases.push({name, status, elapsedMs:performance.now()-start,certificate:r.certifiedCrossing,inputIntact:true})
  return r
}
function reject(name:string, v:object):void {
  const start=performance.now(),reply=request(v)
  assert.equal(reply.ok,false,`${name}: ${JSON.stringify(reply)}`)
  assert.equal(typeof reply.error.code,'string')
  cases.push({name,error:reply.error.code,elapsedMs:performance.now()-start})
}
function graph():any { return { degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],controlPoints:[0,.5,1].map(x=>[[x,0,x*x],[x,1,x*x]]),weights:[[1,1],[1,1],[1,1]],periodicU:false,periodicV:false } }
// Surface codec uses the canonical camelCase wire fields.
run('global-absence',graph(),'absent')
const fold=graph();fold.controlPoints[2]=structuredClone(fold.controlPoints[0])
const f=run('exact-global-fold',fold,'present');assert.deepEqual(f.witness,[[.25,.5],[.75,.5]])
const local=graph();local.knotsU=[0,0,0,1,1,3,3,3]
local.controlPoints=[local.controlPoints[0],local.controlPoints[1],local.controlPoints[0],local.controlPoints[2],local.controlPoints[2]];local.weights=Array.from({length:5},()=>[1,1])
const r=run('non-dyadic-witness',local,'present');assert.equal(r.witness,null)
assert.deepEqual(r.exactWitness[0][0],{numerator:'1',denominator:'12'})
const large=structuredClone(local);large.knotsU=[0,0,0,.1,.1,3,3,3]
assert.deepEqual(run('large-exact-integer-strings',large,'present').exactWitness[0][0],{numerator:'3602879701896397',denominator:'432345564227567616'})
const projective=graph();projective.degreeU=3;projective.knotsU=[0,0,0,0,1,1,1,1]
projective.controlPoints=[0,1.5,4,0].map(x=>[[x,0,2*x],[x,1,2*x]]);projective.weights=[1,2,3,4].map(x=>[x,x])
run('asymmetric-projective-gauge',projective,'present')
const crossing=graph();crossing.degreeU=1;crossing.knotsU=[0,0,1,2,3,3]
crossing.controlPoints=[[0,0],[1,1],[0,1],[1,0]].map(([x,y])=>[[x,y,0],[x,y,2]]);crossing.weights=[1,3,2,1].map(x=>[x,x])
const c=run('rational-spatial-crossing',crossing,'present');assert.deepEqual(c.exactWitness[1][0],{numerator:'8',denominator:'9'})
const overlap=structuredClone(crossing);overlap.knotsU=[0,0,1,2,2];overlap.controlPoints=[[0,0],[3,0],[1,0]].map(([x,y])=>[[x,y,0],[x,y,2]]);overlap.weights=Array.from({length:3},()=>[1,1])
run('adjacent-backtracking-overlap',overlap,'present',{maxSpans:1})
const short=graph();short.degreeU=1;short.knotsU=[0,0,.5,1,1]
run('budget-unresolved',short,'unresolved',{maxSpans:1})
reject('forged-verdict',{op,surface:graph(),maxSpans:100,presenceProven:true})
reject('zero-budget',{op,surface:graph(),maxSpans:0})
const invalid=graph();invalid.weights[0][0]=-1
reject('invalid-source',{op,surface:invalid,maxSpans:100})
run('recovery-after-refusal',graph(),'absent')
for(const fixture of nonlinearSurfaceCases){
 const r=run(fixture.name,fixture.surface(),'present',{maxSpans:10000})
 assert.equal(r.witness,null);assert.equal(r.exactWitness,null)
 const c=r.certifiedCrossing;assert.equal(c?.method,'original-planar-krawczyk');assert(c.cells>0&&c.cells<=10000)
 const axis=fixture.axis??0
 for(let i=0;i<2;i++){assert(c.parameterBoxes[i][axis][0]<=fixture.parameters[i]!&&fixture.parameters[i]!<=c.parameterBoxes[i][axis][1]);assert.deepEqual(c.parameterBoxes[i][1-axis],[.5,.5])}
 for(let i=0;i<3;i++)assert(c.positionBounds[i][0]<=fixture.position[i]!&&fixture.position[i]!<=c.positionBounds[i][1])
}
const skew=nonlinearSurfaceCases[2]!.surface()
const bits=new DataView(new ArrayBuffer(8));for(const p of skew.controlPoints[1]!){bits.setFloat64(0,p[2]!,true);bits.setBigUint64(0,bits.getBigUint64(0,true)+1n,true);p[2]=bits.getFloat64(0,true)}
const shifted=run('one-ulp-nonplanar-source-spatial-contact',skew,'present',{maxSpans:10000})
assert.equal(shifted.certifiedCrossing.method,'original-curve-surface-krawczyk')
// Original rational spatial contacts with different transverse preimages.
for(const fixture of spatialSurfaceCases){
 const axis=fixture.axis,surface=fixture.surface()
 const r=run(`original-rational-spatial-axis-${axis}`,surface,'present',{maxSpans:10000}),c=r.certifiedCrossing
 assert.equal(c.method,'original-curve-surface-krawczyk');assert.equal(r.exactWitness,null);assert.equal(r.witness,null)
 for(const [i,t] of [.25,.75].entries())assert(c.parameterBoxes[i][axis][0]<=t&&t<=c.parameterBoxes[i][axis][1])
 const v=c.parameterBoxes[1][1-axis];assert(v[0]<=473/1063&&473/1063<=v[1]);assert(v[1]<.5)
 assert(r.probeSearch.chargedCells<=10000)
}
const separated=spatialSurfaceCases[0]!.surface()
separated.weights=separated.weights.map(()=>[1,1])
for(const row of separated.controlPoints)row[1]![2]=row[0]![2]!+1/64
const separatedReply=request({op,surface:separated,maxSpans:10000})
assert.equal(separatedReply.ok,true);assert(isSurfaceAuditReport(separatedReply.value))
assert.notEqual(separatedReply.value.status,'present');assert.equal(separatedReply.value.certifiedCrossing,null)
cases.push({name:'xy-projection-with-separated-height-bands',status:separatedReply.value.status})
for(const parameter of [.25,.75]){
 const surface=nonlinearSurfaceCases[1]!.surface()
 const reply=request({op:'nurbs_surface_iso_self_intersection_audit',surface,axis:0,parameter,tolerance:1e-8,maxSpans:10000})
 assert.equal(reply.ok,true);const r=reply.value
 assert(isSurfaceIsoAuditReport(r));assert.equal(r.status,'present')
 assert.equal(r.surfaceAbsenceProven,false);assert.equal(r.allContactBranchesEnumerated,false)
 assert(r.crossings.length>0)
 for(const c of r.crossings)for(const p of c.parameterBoxes)assert.deepEqual(p[1],[parameter,parameter])
 cases.push({name:`explicit-iso-${parameter}`,status:r.status,scope:'selected fiber only'})
}
for(const axis of [0,1] as const){
 const q=isoTargetFixture(axis),before=JSON.stringify(q),reply=request({op:'nurbs_surface_iso_target_window',...q})
 assert.equal(JSON.stringify(q),before);assert.equal(reply.ok,true);const r=reply.value;assert(isSurfaceIsoTargetReport(r));assert.equal(r.roots.length,2);assert.equal(r.windowCoverageComplete,true)
 for(const t of [.25,.75]){const root=r.roots.find((x:any)=>x.parameterBox[0][0]<=t&&t<=x.parameterBox[0][1]);assert(root);const z=4/3+t*t*t/4;assert(root.parameterBox[2][0]<=10+2*z&&10+2*z<=root.parameterBox[2][1])}
 cases.push({name:`iso-target-two-roots-axis-${axis}`,status:r.status,roots:r.roots.length,complete:r.windowCoverageComplete})
}
const q=isoTargetFixture(0)
const shortReply=request({op:'nurbs_surface_iso_target_window',...q,maxCells:1})
assert.equal(shortReply.ok,true);assert(isSurfaceIsoTargetReport(shortReply.value));assert.equal(shortReply.value.status,'unresolved');assert.equal(shortReply.value.windowCoverageComplete,false)
cases.push({name:'iso-target-one-cell-incomplete',status:shortReply.value.status})
for(const row of q.target.controlPoints)for(const p of row)p[0]=10
const absentReply=request({op:'nurbs_surface_iso_target_window',...q})
assert.equal(absentReply.ok,true);assert(isSurfaceIsoTargetReport(absentReply.value));assert.equal(absentReply.value.status,'absent-in-window');assert.equal(absentReply.value.allSurfaceBranchesEnumerated,false)
cases.push({name:'iso-target-separated-window-absence',status:absentReply.value.status})
reject('iso-target-forged-evidence',{op:'nurbs_surface_iso_target_window',...q,bodyAdmission:true})
const offCenter=run('off-center-spatial-contact',offCenterSpatialSurface(),'present',{maxSpans:10000})
assert.equal(offCenter.certifiedCrossing.method,'original-curve-surface-krawczyk')
assert.notEqual(offCenter.certifiedCrossing.parameterBoxes[0][1][0],.5)
assert(offCenter.probeSearch.attempts.some((p:any)=>p.kind?.startsWith('curve-surface')&&p.parameter!==.5))
for(const order of [2,4] as const){
 const query=originalPatchFixture(order),before=JSON.stringify(query),reply=request({op:'nurbs_original_affine_patch_contacts',...query})
 assert.equal(JSON.stringify(query),before);assert.equal(reply.ok,true);const r=reply.value;assert(isOriginalPatchContactsReport(r));assert.equal(r.status,'present');assert.equal(r.curvePatchCoverageComplete,true);assert.equal(r.contacts.length,1);assert.deepEqual(r.contacts[0].multiplicities,[order]);assert(!Object.hasOwn(r.contacts[0],'crossingSign'))
 cases.push({name:`original-patch-contact-order-${order}`,status:r.status,multiplicities:r.contacts[0].multiplicities,complete:r.curvePatchCoverageComplete})
}
function patchReply(query:any){const reply=request({op:'nurbs_original_affine_patch_contacts',...query});assert.equal(reply.ok,true);assert(isOriginalPatchContactsReport(reply.value));return reply.value}
const knot=patchReply(originalPatchKnotFixture());assert.equal(knot.contacts.length,1);assert.deepEqual(knot.contacts[0].multiplicities,[2,1]);assert.deepEqual(knot.contacts[0].parameterBox[0],[1,1]);cases.push({name:'original-patch-one-sided-orders',status:knot.status,multiplicities:knot.contacts[0].multiplicities})
const zero=patchReply({...originalPatchFixture(2),maxExactWork:0});assert.equal(zero.status,'unresolved');assert.equal(zero.exactWork,0);assert.equal(zero.curvePatchCoverageComplete,false);cases.push({name:'original-patch-zero-exact-work',status:zero.status})
const finite=originalPatchFixture(2);for(const p of finite.curve.controlPoints)p[1]=2
const exterior=patchReply(finite);assert.equal(exterior.status,'absent-on-patch');assert.equal(exterior.patchAbsenceProven,true);cases.push({name:'original-patch-finite-exclusion',status:exterior.status})
for(const p of finite.curve.controlPoints)p[1]=0
const boundary=patchReply(finite);assert.equal(boundary.status,'present');assert.equal(boundary.contacts[0].uvMethod,'exact-boundary-projective');cases.push({name:'original-patch-exact-boundary',status:boundary.status})
const intervalBoundary=structuredClone(finite);intervalBoundary.curve.weights=[1,2,4]
const intervalEdge=patchReply(intervalBoundary);assert.equal(intervalEdge.status,'present');assert.equal(intervalEdge.curvePatchCoverageComplete,true);assert.equal(intervalEdge.contacts[0].uvMethod,'original-affine-coordinate-bounds');cases.push({name:'original-patch-interval-edge-identity',status:intervalEdge.status})
const irrationalEdge=patchReply(originalPatchAlgebraicBoundaryFixture());assert.equal(irrationalEdge.status,'present');assert.equal(irrationalEdge.curvePatchCoverageComplete,true);assert.equal(irrationalEdge.contacts[0].uvMethod,'original-affine-coordinate-bounds');assert.deepEqual(irrationalEdge.contacts[0].positionBounds,[[0,0],[0,0],[.5,.5]]);cases.push({name:'original-patch-irrational-edge-identity',status:irrationalEdge.status})
const commonRoot=patchReply(originalPatchAlgebraicBoundaryFixture(true));assert.equal(commonRoot.status,'present');assert.equal(commonRoot.curvePatchCoverageComplete,true);assert.equal(commonRoot.contacts[0].uvMethod,'original-polynomial-common-root');cases.push({name:'original-patch-nonproportional-common-root-proven',status:commonRoot.status})
reject('original-patch-forged-body-admission',{op:'nurbs_original_affine_patch_contacts',...originalPatchFixture(2),bodyAdmission:true})
reject('original-patch-over-limit-exact-budget',{op:'nurbs_original_affine_patch_contacts',...originalPatchFixture(2),maxExactWork:1000001})
for(const fixture of [...advancedPatchCases,...commonRootPatchCases]){const before=JSON.stringify(fixture.query),r=patchReply(fixture.query);assert.equal(JSON.stringify(fixture.query),before);fixture.check(r);cases.push({name:fixture.name,status:r.status,contact:r.contacts[0]??null,complete:r.curvePatchCoverageComplete})}
const scalarOnlyQuery=originalPatchAlgebraicBoundaryFixture(true);for(const p of scalarOnlyQuery.curve.controlPoints)p[1]=2
const scalarOnly=patchReply(scalarOnlyQuery);assert.equal(scalarOnly.status,'absent-on-patch');cases.push({name:'common-root-scalar-budget-baseline',status:scalarOnly.status,exactWork:scalarOnly.exactWork})
const auxiliaryBudget=patchReply({...originalPatchAlgebraicBoundaryFixture(true),maxExactWork:scalarOnly.exactWork+10});assert.equal(auxiliaryBudget.status,'unresolved');assert.equal(auxiliaryBudget.contacts.length,0);assert(auxiliaryBudget.ambiguousParameters.length>0);assert(auxiliaryBudget.exactWork<=auxiliaryBudget.maxExactWork);assert.equal(auxiliaryBudget.exactWork,auxiliaryBudget.scalarProofs.reduce((sum,p)=>sum+p.workUsed,0));cases.push({name:'common-root-auxiliary-work-exhaustion',status:auxiliaryBudget.status,exactWork:auxiliaryBudget.exactWork,maxExactWork:auxiliaryBudget.maxExactWork})
for(const fixture of quadricCarrierCases){const before=JSON.stringify(fixture.query),reply=request({op:'nurbs_original_quadric_carrier_roots',...fixture.query});assert.equal(reply.ok,true);assert(isOriginalQuadricCarrierReport(reply.value));fixture.check(reply.value);assert.equal(JSON.stringify(fixture.query),before);cases.push({name:fixture.name,status:reply.value.status,complete:reply.value.curveCarrierCoverageComplete,roots:reply.value.roots})}
reject('quadric-forged-finite-admission',{op:'nurbs_original_quadric_carrier_roots',...quadricCarrierCases[0]!.query,finiteSurfaceUvAdmission:true})
reject('quadric-over-limit-work',{op:'nurbs_original_quadric_carrier_roots',...quadricCarrierCases[0]!.query,maxExactWork:1000001})
let graphBaseline:OriginalQuadricGraphReport|undefined
for(const fixture of quadricGraphCases){const before=JSON.stringify(fixture.query),reply=request({op:'nurbs_original_quadric_graph_contacts',...fixture.query});assert.equal(reply.ok,true);assert(isOriginalQuadricGraphReport(reply.value));fixture.check(reply.value);assert.equal(JSON.stringify(fixture.query),before);if(fixture.name==='graph-edge-0')graphBaseline=reply.value;cases.push({name:fixture.name,status:reply.value.status,complete:reply.value.curvePatchCoverageComplete,contacts:reply.value.contacts,exactWork:reply.value.exactWork})}
assert(graphBaseline?.carrier)
const graphBudgetQuery={...boundaryQuery(),maxExactWork:graphBaseline.targetMembership.workUsed+graphBaseline.carrier.exactWork}
const graphBudgetReply=request({op:'nurbs_original_quadric_graph_contacts',...graphBudgetQuery});assert.equal(graphBudgetReply.ok,true);assert(isOriginalQuadricGraphReport(graphBudgetReply.value));assert.equal(graphBudgetReply.value.status,'unresolved');assert.equal(graphBudgetReply.value.finiteSurfaceUvAdmission,false);assert.equal(graphBudgetReply.value.carrier?.curveCarrierCoverageComplete,true);cases.push({name:'graph-boundary-budget-exhaustion',status:graphBudgetReply.value.status,exactWork:graphBudgetReply.value.exactWork})
reject('graph-forged-body',{op:'nurbs_original_quadric_graph_contacts',...graphQuery(),bodyAdmission:true})
reject('graph-forged-target-proof',{op:'nurbs_original_quadric_graph_contacts',...graphQuery(),targetGraphProven:true})
reject('graph-over-limit-work',{op:'nurbs_original_quadric_graph_contacts',...graphQuery(),maxExactWork:1000001})
const graphBadWeights=graphQuery();graphBadWeights.patch.weights[1]![1]=2
reject('graph-unsupported-target-weights',{op:'nurbs_original_quadric_graph_contacts',...graphBadWeights})
const graphBadSheet=graphQuery();graphBadSheet.quadric=[[0,0,0,0],[0,0,0,0],[0,0,1,0],[0,0,0,-1]]
reject('graph-ambiguous-sheet',{op:'nurbs_original_quadric_graph_contacts',...graphBadSheet})
const graphPerturbedRank=graphQuery();graphPerturbedRank.patch.weights=[[1,3],[2,6+Number.EPSILON*4]]
reject('graph-perturbed-separable-target',{op:'nurbs_original_quadric_graph_contacts',...graphPerturbedRank})
const multiBase=quadricGraphCases.find(c=>c.name==='graph-multispan-internal-knot')!
const multiDrift=structuredClone(multiBase.query);multiDrift.patch.controlPoints[1]![0]![0]=.25+Number.EPSILON/4
reject('graph-multispan-ulp-projection-drift',{op:'nurbs_original_quadric_graph_contacts',...multiDrift})
const multiRational=structuredClone(multiBase.query);multiRational.patch.weights[1]=[14,14,14,14]
reject('graph-multispan-unproved-rational-clock',{op:'nurbs_original_quadric_graph_contacts',...multiRational})
const result={status:'passed' ,nodeVersion:process.version,sourceCommit,wasmPath:resolve(wasmPath),wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,imports:[],cases,installedPackageUpdated:false,scope:'Actual import-free raw WASM binary ABI; not packaged-WASM, Worker, editor or full parity qualification'}
writeFileSync(resolve(directory,'results.json'),JSON.stringify(result,null,2)+'\n')
console.log(JSON.stringify(result,null,2))
