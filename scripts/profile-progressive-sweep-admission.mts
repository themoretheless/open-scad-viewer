import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'

const sourceFile=process.argv[2]??'examples/rush/closed-periodic-spatial-rmf-multiple-holes-body.r'
const namedWasm=process.argv.find(arg=>arg.startsWith('--named-wasm='))?.slice('--named-wasm='.length)
const modelOutput=process.argv.find(arg=>arg.startsWith('--model-output='))?.slice('--model-output='.length)
const started=process.hrtime.bigint(),cpuStarted=process.cpuUsage()
const emit=(stage:string,extra:Record<string,unknown>={})=>{
 const cpu=process.cpuUsage(cpuStarted)
 console.log(JSON.stringify({stage,elapsedMs:Number(process.hrtime.bigint()-started)/1e6,
  processCpuMs:(cpu.user+cpu.system)/1000,rssBytes:process.memoryUsage().rss,...extra}))
}
emit('start',{sourceFile,artifact:sweepStepArtifactProvenance(),scope:'Observational phase/CPU profile; does not replace the unchanged120000ms test gate'})
const document=compileRushFrontend(readFileSync(sourceFile,'utf8')).document
emit('compiled')
const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}},
 {onSweepPreview:(_id,preview)=>{
  const r=preview.report as unknown as Record<string,unknown>
  emit('preview',{sectionCount:r.sectionCount,accepted:r.accepted,continuousBound:r.continuousBound})
 }})
assert.ok(built.nativeGeometry,'Missing actual retained body')
emit('built')
const evidence=readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)
assert.ok(evidence?.continuousBound&&evidence.withinBudget,'Missing complete original boundary bound')
const model=JSON.parse(built.nativeGeometry.geometryJson).geometry
if(modelOutput)writeFileSync(modelOutput,JSON.stringify(model)+'\n')
emit('parsed',{faces:model.faces.length,shells:model.shells.length})
let namedAudit:((budgets:typeof DEFAULT_SWEEP_VOLUME_BUDGETS)=>ReturnType<typeof inspectSweepVolume>)|undefined
if(namedWasm){
 const bytes=readFileSync(namedWasm)
 const instance=new WebAssembly.Instance(new WebAssembly.Module(bytes))
 const wasm=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
 emit('diagnostic-kernel',{path:namedWasm,sha256:createHash('sha256').update(bytes).digest('hex'),scope:'Named diagnostic kernel; not published-artifact qualification or index attribution'})
 namedAudit=budgets=>{
  const request=encodeBinary({op:'brep_sweep_volume_audit',model,capFaces:[],...budgets})
  const ptr=writeLinear(wasm.memory,wasm.abi_alloc,request)
  assert.ok(ptr,'Diagnostic request allocation failed')
  try{
   const reply=decodePacked<{ok:boolean;value:ReturnType<typeof inspectSweepVolume>;error?:unknown}>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,request.length))
   assert.ok(reply.ok,JSON.stringify(reply.error))
   return reply.value
  }finally{wasm.abi_free(ptr,request.length)}
 }
}
const admission=namedAudit?namedAudit(DEFAULT_SWEEP_VOLUME_BUDGETS):inspectProgressiveSweepSolidAdmission(built.nativeGeometry,model)
assert.ok(admission?.solidGeometryCertified&&admission.allFacesInjective&&admission.allPairsClassified)
emit('fresh-solid',{parents:admission.nesting?.parents,individualPairs:admission.individualPairs,groupedPairs:admission.groupedPairs})
const exhaustedBudgets={...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1}
const exhausted=namedAudit?namedAudit(exhaustedBudgets):inspectSweepVolume(model,[],exhaustedBudgets)
assert.equal(exhausted.solidGeometryCertified,false)
assert.equal(exhausted.boundaryEmbeddingCertified,false)
emit('exhausted-work-refused')
