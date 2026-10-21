/** Reconcile captured Rust proof scopes against actual retained topology. */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync} from 'node:fs'
import {resolve,join} from 'node:path'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {checkRetainedAxisAudit} from './retained-axis-audit-contract.mjs'
import {checkCompleteFacePairCover} from './sweep-face-pair-cover.mjs'

const [directory,output]=process.argv.slice(2)
assert(directory&&output&&!existsSync(output),'Supply qualification directory and a new result path')
const root=resolve(directory),inputs:Record<string,string>={}
const sha=(data:string|Buffer)=>createHash('sha256').update(data).digest('hex')
function read(path:string):any {const data=readFileSync(path);inputs[path]=sha(data);return JSON.parse(data.toString())}
const gate=read(join(root,'v11-original-source-step37-occt-20261018.json'))
assert.equal(gate.state,'complete');assert.equal(gate.passed,true)
const manifest=read(gate.manifest);assert.equal(inputs[gate.manifest],gate.manifestSha256)
const occt=read(gate.report);assert.equal(inputs[gate.report],gate.reportSha256);assert.equal(occt.passed,true)
const intrinsic=read(join(root,'v11-original-source-interiors37-20261018/result.json'))
const gaps=read(join(root,'v11-retained-proof-gaps9-20261018.json'))
for(const report of [intrinsic,gaps]) {
 assert.equal(report.passed,true);assert.equal(report.manifestSha256,gate.manifestSha256)
 assert.equal(report.geometryWasmSha256,gate.geometryWasmSha256)
}
const supplement=new Map(gaps.cases.map((c:any)=>[c.file,c]))
const independentCaps:any[]=[]
function ownerOf(item:any):any {
 if(item.construction?.profileSmoothness)return item.construction
 const extra:any=supplement.get(item.file);assert(extra,'Missing actual seam capture')
 const owners=Object.values(extra.construction).filter((c:any)=>c.profileSmoothness&&
  JSON.stringify(c.profileSmoothness)===JSON.stringify(extra.profileSmoothness))
 assert.equal(owners.length,1,'Ambiguous retained proof owner');return owners[0]
}
function bound(item:any,owner:any,caps:number[]) {
 const m=owner.boundaryCertificate,b=m??owner
 const complete=m?b.continuousBound:b.boundaryContinuousBound
 const upper=m?b.errorUpper:b.boundaryErrorUpper
 const budget=m?b.budget:owner.budget
 const within=m?b.withinBudget:b.boundaryErrorWithinBudget
 const closed=m?b.closed:owner.closedPath
 assert.equal(complete,true);assert.equal(typeof closed,'boolean')
 assert(Number.isFinite(upper)&&upper>=0)
 assert.equal(m?b.scope:b.boundaryErrorScope,m?'boundary-set-hausdorff':'constructor-owned-retained-wall-and-cap-union')
 if(m){assert.equal(b.method,'retained-sweep-boundary-union');assert(Number.isFinite(b.wallErrorUpper)&&b.wallErrorUpper>=0&&b.wallErrorUpper<=upper)}
 if(budget===null){assert(m);assert.equal(b.reason,'no-authored-tolerance');assert.equal(within,null)}
 else {assert(Number.isFinite(budget)&&budget>0&&upper<=budget);assert.equal(within,true)}
 if(closed){assert.equal(caps.length,0);assert.equal(b.filledCapErrorUpper,null)}
 else {
  assert(caps.length>=2);assert.equal(b.filledCapErrorUpper.length,2)
  assert(b.filledCapErrorUpper.every((v:any)=>Number.isFinite(v)&&v>=0&&v<=upper))
 }
 const shown=item.boundaryError;assert.equal(shown.continuousBound,true)
 assert.equal(shown.errorUpper??shown.boundaryErrorUpper,upper)
 assert.equal(shown.budget??shown.boundaryErrorBudget,budget)
 return {closed,upper,budget,withinBudget:within}
}
function owned(item:any,profile:any,caps:number[],groups:number[][],capReports:any[]) {
 checkRetainedAxisAudit(profile,profile.profile,profile.g1Audit,profile.profileG1Certified,profile.profile.exactG1G2Certified)
 const station=profile.station
 checkRetainedAxisAudit(station,station.g2,station.g1Audit,station.stationG1Certified,station.stationG2Certified)
 assert.equal(profile.totalExactWork,profile.exactWork+station.exactWork)
 assert.equal(profile.fullBoundarySmoothnessCertified,false)
 const uses=new Map<number,number[]>()
 for(const [face,wires]of item.faceLoops.entries())for(const wire of [wires.outer,...wires.holes])for(const edge of wire){
  assert(Number.isInteger(edge)&&edge>=0&&edge<item.edges)
  const list=uses.get(edge)??[];list.push(face);uses.set(edge,list)
 }
 assert.equal(uses.size,item.edges);assert([...uses.values()].every(faces=>faces.length===2))
 const internal=[...uses].filter(([,faces])=>faces.every(face=>caps.includes(face))).map(([edge])=>edge)
 if(internal.length){
  assert.equal(groups.length,2);assert.deepEqual(groups.flat(),caps)
  assert.equal(capReports.length,groups.filter(group=>group.length>1).length)
  let index=0
  for(const group of groups){
   if(group.length<2)continue
   const report=capReports[index++]
   const expected=internal.filter(edge=>uses.get(edge)!.every(face=>group.includes(face)))
   assert.equal(report.extractionComplete,true);assert.deepEqual(report.unclassifiedFaces,[]);assert.deepEqual(report.unpairedEdges,[])
   assert.deepEqual([...report.edgeIds].sort((a,b)=>a-b),expected.sort((a,b)=>a-b),'Cap interior seam omitted or rebound')
  }
 }
 const complete=[...profile.edgeIds,...station.edgeIds,...station.capEdges,...internal]
 assert.equal(new Set(complete).size,complete.length,'Retained edge scopes overlap')
 assert.deepEqual(complete.sort((a,b)=>a-b),Array.from({length:item.edges},(_,i)=>i),'Retained edge scope incomplete')
 return {edges:item.edges,profileEdges:profile.edgeIds.length,stationEdges:station.edgeIds.length,
  capWallEdges:station.capEdges.length,internalCapEdges:internal.length,
  profileG1:profile.profileG1Certified,profileG2:profile.profile.exactG1G2Certified,stationContinuity:profile.stationContinuity}
}
const cases:any[]=[]
for(const item of manifest.cases){
 const owner=ownerOf(item),profile=owner.profileSmoothness
 const groups=owner.capFaceGroups??(item.capFaces.length===2?item.capFaces.map((face:number)=>[face]):[[],[]])
 const caps=owner.capFaces??owner.capFaceGroups?.flat()??item.capFaces
 assert.equal(new Set(caps).size,caps.length);assert(caps.every((f:number)=>Number.isInteger(f)&&f>=0&&f<item.faces))
 const boundary=bound(item,owner,caps)
 const native=item.nativeVolume
 for(const flag of ['solidGeometryCertified','boundaryEmbeddingCertified','allFacesInjective','allPairsClassified'])assert.equal(native[flag],true)
 assert.equal(native.nesting.rolesConsistent,true);assert.equal(native.nesting.visitedPairs,native.nesting.totalPairs)
 assert(native.orientations.every((r:any)=>r.outward===r.expectedOutward&&typeof r.outward==='boolean'))
 const pairCover=checkCompleteFacePairCover(native.boundary,item.faces)
 const basis=intrinsic.cases.find((c:any)=>c.file===item.file);assert(basis?.complete&&basis.unvisitedFaces===0)
 assert.deepEqual(basis.faces.map((f:any)=>f.face),Array.from({length:item.faces},(_,i)=>i).filter(i=>!item.capFaces.includes(i)))
 assert(basis.faces.every((f:any)=>f.parametricC2Certified&&f.parametricC1Certified&&f.surfaceRegularityCertified===false&&f.geometricG1G2Certified===false))
 let capReports=owner.retainedSmoothness?.capInternalSeams??[]
 if(groups.some((g:number[])=>g.length>1)&&!capReports.length){
  // Reconstructed annular caps are checked on the ACTUAL saved model, rather
  // than inheriting the original constructor's cap report or work ledger.
  assert.equal(item.file,'oblique-rmf-annular-body-station-g2.step')
  const model=owner.model;assert(model);assert.equal(sha(JSON.stringify(model)),item.uiModelSha256)
  capReports=[];let work=0
  for(const group of groups){
   const excluded=Array.from({length:item.faces},(_,i)=>i).filter(i=>!group.includes(i))
   const report=callGeometryRust<any>('brep_miter_profile_smoothness_audit',{model,capFaces:excluded,maxWork:2000000-work})
   checkRetainedAxisAudit(report,report.profile,report.g1Audit,report.profileG1Certified,report.profile.exactG1G2Certified)
   work+=report.totalExactWork;assert(work<=2000000)
   capReports.push(report)
  }
  independentCaps.push({file:item.file,maxWork:2000000,exactWork:work,reports:capReports,
   scope:'Fresh native diagnostics on unchanged final cap geometry; independent of the original constructor work ledger.'})
 }
 const coverage=owned(item,profile,caps,groups,capReports)
 cases.push({file:item.file,sourceSha256:item.sourceSha256,uiModelSha256:item.uiModelSha256,boundary,
  ...coverage,interiorFaces:basis.faces.length,pairCover,
  geometricInteriorPrerequisitesPresent:true,
  premiseScope:'Native basis C2 plus complete actual-model strict Jacobian/injectivity admission. Primitive basis flags remain unchanged; owned seams and cap-wall C0 remain separate.'})
}
assert.equal(cases.length,37)
for(const [file,digest]of Object.entries(inputs))assert.equal(sha(readFileSync(file)),digest,file)
writeFileSync(output,JSON.stringify({schema:'sweep-source-proof-composition/1',passed:true,
 geometryWasmSha256:gate.geometryWasmSha256,manifestSha256:gate.manifestSha256,
 inputsSha256:inputs,scriptSha256:sha(readFileSync(new URL(import.meta.url))),cases,independentCaps,
 scope:'Finite original37 whole-boundary/report, actual-global, intrinsic-basis and complete owned-edge composition. No all-input theorem or new whole-boundary smoothness flag.',fullGoalComplete:false},null,2)+'\n')
console.log('All37source-bound bodies have complete error/global/interior/owned-edge coverage.')
