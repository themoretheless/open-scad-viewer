// Diagnostic transport timing only. Every request and geometric decision is unchanged Rust/WASM.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import {createHash} from 'node:crypto'
import * as module from 'node:module'
import {decodeBinary} from '../src/services/valueBinaryCodec'
import {load} from '../tests/fixtures/raw-import-loader.mjs'

const [sourcePath,outputPath]=process.argv.slice(2)
const captureOnly=process.argv.includes('--capture-first-level')
assert.ok(sourcePath&&outputPath,'Supply Rush source and diagnostic output JSON')
if(typeof (module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const output=resolve(outputPath),source=readFileSync(sourcePath,'utf8')
mkdirSync(dirname(output),{recursive:true})
const sha=(bytes:Uint8Array|string)=>createHash('sha256').update(bytes).digest('hex')
const packedPath=resolve('public/wasm/geometry-kernel.wasm')
const report:any={scope:'Instrumented kernel and phase timing; not an uninstrumented latency gate or new geometry certificate.',
 sourcePath,sourceSha256:sha(source),packedWasmSha256:sha(readFileSync(packedPath)),calls:[],phases:[],passed:false}
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
const instrumented=new WeakSet<WebAssembly.Instance>()
function instrument(instance:WebAssembly.Instance) {
 const actual=instance.exports as any
 if(instrumented.has(instance)||typeof actual.abi_request!=='function')return instance
 instrumented.add(instance)
 const exports={...actual,abi_request:(operation:number,pointer:number,length:number)=>{
  const bytes=new Uint8Array(actual.memory.buffer,pointer,length)
  const request=operation===0?decodeBinary(bytes) as any:null
  const op=request?.op??`ABI:${operation}`
  const requestSha256=sha(bytes)
  if(captureOnly&&op==='surface_progressive_sweep_level') {
   const path=output+'.request.json'
   writeFileSync(path,JSON.stringify(request)+'\n')
   report.capturedRequest={path,requestSha256,requestBytes:length}
   throw Error('QA_FIRST_LEVEL_CAPTURE_COMPLETE')
  }
  if(op==='brep_sweep_volume_begin'||op==='brep_sweep_volume_charts_audit') {
   const path=output+'.'+report.calls.length+'.request.json'
   writeFileSync(path,JSON.stringify(request)+'\n')
  }
  const start=performance.now()
  try{return actual.abi_request(operation,pointer,length)}
  finally{report.calls.push({op,requestSha256,requestBytes:length,milliseconds:performance.now()-start})}
 }}
 Object.defineProperty(instance,'exports',{value:exports})
 return instance
}
const Instance=WebAssembly.Instance
WebAssembly.Instance=class extends Instance {
 constructor(module:WebAssembly.Module,imports?:WebAssembly.Imports){super(module,imports);instrument(this)}
}
const instantiate=WebAssembly.instantiate.bind(WebAssembly)
;(WebAssembly as any).instantiate=async(...args:any[])=>{
 const result=await (instantiate as any)(...args)
 instrument(result instanceof Instance?result:result.instance)
 return result
}
async function phase<T>(name:string,run:()=>T|Promise<T>):Promise<T> {
 const start=performance.now()
 try{return await run()}
 finally{report.phases.push({name,milliseconds:performance.now()-start});save()}
}
try {
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readNativeGeometryData}=await import('../src/services/nativeGeometrySnapshot')
 const {tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS}=await import('../src/services/nurbsSweepEmbedding')
 const document=await phase('compile',()=>compileRushFrontend(source).document)
 const built=await phase('build',()=>buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}}))
 assert.ok(built.nativeGeometry)
 const data=await phase('snapshot',()=>readNativeGeometryData(built.nativeGeometry!))
 const model=data.geometry as any
 const mesh=await phase('tessellate',()=>tessellateNurbsBrep(model,4))
 assert.equal(mesh.report.closed,true)
 const volume=await phase('fresh-solid',()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model))
 assert.equal(volume?.solidGeometryCertified,true)
 await phase('short-budget-refusal',()=>assert.equal(inspectSweepVolume(model,[],{
  ...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1}).solidGeometryCertified,false))
 assert.equal(sha(readFileSync(packedPath)),report.packedWasmSha256)
 report.passed=true
} catch(error) {
 report.error=String(error)
 if(captureOnly&&report.capturedRequest&&report.error.includes('QA_FIRST_LEVEL_CAPTURE_COMPLETE'))report.captureComplete=true
 else process.exitCode=1
} finally {
 const totals=new Map<string,{op:string;count:number;milliseconds:number;maximumMilliseconds:number}>()
 for(const call of report.calls){
  const total=totals.get(call.op)??{op:call.op,count:0,milliseconds:0,maximumMilliseconds:0}
  total.count++;total.milliseconds+=call.milliseconds
  total.maximumMilliseconds=Math.max(total.maximumMilliseconds,call.milliseconds)
  totals.set(call.op,total)
 }
 report.totals=[...totals.values()].sort((a,b)=>b.milliseconds-a.milliseconds)
 save()
 console.log(JSON.stringify({passed:report.passed,phases:report.phases,totals:report.totals.slice(0,10)}))
}
