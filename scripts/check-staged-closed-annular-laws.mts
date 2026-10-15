import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname} from 'node:path'
import {createHash} from 'node:crypto'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {progressiveMiterOwnedRequest,type AnnularMiterOwnedBoundaryReport} from '../src/services/nurbsMiterOwnedBoundary'
import {DEFAULT_SWEEP_VOLUME_BUDGETS,type SweepVolumeAudit} from '../src/services/nurbsSweepEmbedding'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {miterLawMatrixModes,miterLawMatrixName,miterLawMatrixSource,miterLawMatrixSmoothSource} from './miter-law-matrix-sources'
import type {NurbsCurve} from '../src/services/nurbsCurve'

// Exact-source transport and independent receiving-realm material checks.
// All geometry, saved jets, error composition and Solid decisions remain Rust.
const [output,expectedWasm]=process.argv.slice(2)
assert.ok(output&&expectedWasm)
const hash=(bytes:Uint8Array|string)=>createHash('sha256').update(bytes).digest('hex')
const provenance=sweepStepArtifactProvenance()
assert.equal(provenance.geometryWasmSha256,expectedWasm)
const report:any={schema:'sweep-closed-annular-laws/1',passed:false,artifactProvenance:provenance,
 scope:'Supplemental public annular-miter closed entry: all8joint laws, raw/profileG2companions. Exact Rush-model equality and fresh native material. No new UI/STEP inventory or whole-boundary G2 claim.',
 nativeSilenceLimitMs:30000,solidLimitMs:120000,cases:[]}
mkdirSync(dirname(output),{recursive:true})
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
save()
const calls:{op:string;milliseconds:number}[]=[]
const invoke=<T,>(op:string,args:object):T=>{
 const started=performance.now()
 try{return callGeometryRust<T>(op,args)}finally{
  const milliseconds=performance.now()-started;calls.push({op,milliseconds})
  assert.ok(milliseconds<=30000,`${op}: original native silence ceiling exceeded`)
 }
}
const canonical=(v:any):any=>v===null||typeof v!=='object'?v:Array.isArray(v)?v.map(canonical):Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]))
function volume(model:unknown){
 const began=performance.now(),session=invoke<{session:number}>('brep_sweep_volume_begin',{model,capFaces:[],...DEFAULT_SWEEP_VOLUME_BUDGETS})
 try{
  for(let i=0;i<10000;i++){
   const step=invoke<{done:boolean;report?:SweepVolumeAudit}>('brep_sweep_volume_step',{session:session.session,maxUnits:32})
   assert.ok(performance.now()-began<=120000,'Original Solid ceiling exceeded')
   if(step.done){assert.ok(step.report);return step.report}
  }
  throw Error('Closed native material did not finish')
 }finally{invoke('brep_sweep_volume_drop',{session:session.session})}
}
for(const mode of miterLawMatrixModes.filter(mode=>mode.closed))for(const profileG2 of [false,true]){
 const source=profileG2?miterLawMatrixSmoothSource(mode):miterLawMatrixSource(mode)
 const graph=compileRushFrontend(source).document,node:any=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')
 assert.ok(node&&node.closed===true)
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 assert.ok(built.nativeGeometry)
 const capturedModel=readNativeGeometryData(built.nativeGeometry).geometry
 const {kind:originalKind,...originalModel}=built.report.definitions[node.id] as any
 assert.equal(originalKind,'brep')
 const needCurve=(id:string):NurbsCurve=>{
  const value:any=built.report.definitions[id];assert.equal(value.kind,'curve')
  const {kind:_,...curve}=value;return curve as NurbsCurve
 }
 // Mirror only the existing wire normalization, using the actual compiled
 // graph fields and native-produced source curves; no authored values change.
 const corrections:any={}
 for(const kind of ['circle','cap'])if(node[`${kind}_correction_tolerance`]!==undefined)
  corrections[`${kind}Correction`]={tolerance:node[`${kind}_correction_tolerance`],
   quantum:node[`${kind}_correction_quantum`]??2**-40,maxWork:node[`${kind}_correction_max_work`]??1000000,
   ...(kind==='cap'?{authoredFrame:node.cap_correction_authored_frame}:{})}
 const request=progressiveMiterOwnedRequest(node.loops.map((wire:string[])=>wire.map(needCurve)),node.points,node.scale,node.twist,{
  ...corrections,retainedWallMaxInjectivityCells:node.retained_wall_max_injectivity_cells,
  ...(node.orientation_guide?{orientationGuide:needCurve(node.orientation_guide)}:{}),
  frameAxis:node.frame_axis,frameNormal:node.frame_normal,axisScale:node.axis_scale,centerLaw:node.center_law,
  normal:node.normal,closed:node.closed,miterLimit:node.miter_limit,initialSteps:node.initial_steps,maxSteps:node.max_steps,maxDeviation:node.max_deviation,
 })
 const before=JSON.stringify(request),name=miterLawMatrixName(mode)+(profileG2?'-profile-g2':'')
 report.activeCase={name,sourceSha256:hash(source),request};save()
 const body=invoke<AnnularMiterOwnedBoundaryReport>('brep_miter_annular_owned_construct',{...request,smoothnessMaxWork:2000000})
 report.activeCase.nativeBody=body;save()
 assert.equal(JSON.stringify(request),before)
 assert.deepEqual(body.model,originalModel,`${name}: actual closed model differs from Rush source`)
 // JSON snapshots normalize signed zero. Preserve exact captured coefficient
 // values using the same canonical fingerprints as original-model QA, with
 // no epsilon or coefficient displacement exception.
 assert.equal(hash(JSON.stringify(canonical(body.model))),hash(JSON.stringify(canonical(capturedModel))))
 assert.equal(body.closed,true);assert.equal(body.boundaryCertificate.closed,true)
 assert.equal(body.boundaryCertificate.continuousBound,true);assert.equal(body.boundaryCertificate.withinBudget,true)
 assert.equal(body.boundaryCertificate.method,'retained-sweep-boundary-union')
 assert.equal(body.boundaryCertificate.scope,'boundary-set-hausdorff')
 assert.equal(body.boundaryCertificate.budget,request.maxDeviation)
 assert.equal(body.boundaryCertificate.filledCapErrorUpper,null)
 assert.deepEqual(body.capFaceGroups,[[],[]]);assert.deepEqual(body.capFaces,[]);assert.deepEqual(body.planarCapFaces,[])
 assert.equal(body.retainedAnnularFilling,null)
 // "legacy" is this entry's existing closed-wall retention label. Physical
 // absence is established by the native masks/groups, not by inventing caps.
 assert.equal(body.capRepresentation,'legacy')
 const smooth=body.retainedSmoothness;assert.ok(smooth)
 assert.equal(smooth.auditOrder,'caps-first');assert.deepEqual(smooth.capInternalSeams,[])
 assert.equal(smooth.walls.maxWork,2000000);assert.equal(smooth.totalExactWork,smooth.walls.totalExactWork)
 assert.ok(smooth.totalExactWork<=2000000)
 if(profileG2)assert.equal(smooth.walls.profile.exactG1G2Certified,true)
 assert.equal(smooth.walls.stationContinuity,'C0','Original sharp miter joins must retain C0')
 assert.equal(smooth.fullBoundarySmoothnessCertified,false)
 const level=body.levels.at(-1);assert.ok(level)
 assert.deepEqual([level.affineLawsApplied,level.authoredFramesApplied,level.orientationGuideApplied],[mode.affine,mode.authored,mode.guided])
 const material=volume(body.model)
 report.activeCase.volume=material;save()
 assert.equal(material.solidGeometryCertified,true)
 assert.equal(material.allFacesInjective,true);assert.equal(material.allPairsClassified,true)
 assert.equal(material.boundary.totalPairs,body.model.faces.length*(body.model.faces.length-1)/2)
 assert.equal(material.boundary.individualPairs+material.boundary.groupedPairs,material.boundary.totalPairs)
 const zero=invoke<AnnularMiterOwnedBoundaryReport>('brep_miter_annular_owned_construct',{...request,smoothnessMaxWork:0})
 assert.deepEqual(zero.model,body.model)
 assert.equal(zero.retainedSmoothness?.totalExactWork,0)
 assert.equal(zero.retainedSmoothness?.walls.profile.exactG1G2Certified,false)
 assert.deepEqual(zero.capFaces,[])
 assert.throws(()=>invoke('brep_miter_annular_owned_construct',{...request,model:body.model}),/original source/)
 report.cases.push({name,mode,profileG2,sourceSha256:hash(source),requestSha256:hash(before),modelSha256:hash(JSON.stringify(body.model)),
  request,boundaryCertificate:body.boundaryCertificate,capFaceGroups:body.capFaceGroups,
  retainedSmoothness:smooth,volume:material,rushModelExactlyEqual:true,zeroSmoothnessWorkRefused:true,injectedModelRefused:true})
 delete report.activeCase
 save();console.log(name,'closed annular receiving-realm material passed')
}
assert.equal(report.cases.length,16)
report.passed=true;report.calls=calls;report.runnerSha256=hash(readFileSync(new URL(import.meta.url)));save()
console.log('Closed annular16 exact source/retained-model/material and refusal cases passed')
