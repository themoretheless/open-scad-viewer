// QA timing/capture only. Geometry, binding and material decisions remain Rust-owned.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import {createHash} from 'node:crypto'
import * as module from 'node:module'
import {decodeBinary} from '../src/services/valueBinaryCodec'
import {load} from '../tests/fixtures/raw-import-loader.mjs'

const [sourcePath,outputPath]=process.argv.slice(2)
assert.ok(sourcePath&&outputPath)
const captureOnly=process.argv.includes('--capture-bound-replay')
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const output=resolve(outputPath),sha=(v:Uint8Array|string)=>createHash('sha256').update(v).digest('hex')
mkdirSync(dirname(output),{recursive:true})
const source=readFileSync(sourcePath,'utf8')
const placed=source.replace('.brep_tessellate(4)',
 '.transform(matrix: [[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]).transform(matrix: [[-1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]).brep_tessellate(4)')
assert.notEqual(placed,source)
const report:any={scope:'Instrumented unchanged affine fixture; diagnostic timings are not a latency gate.',
 sourcePath,sourceSha256:sha(source),placedSourceSha256:sha(placed),geometryWasmSha256:sha(readFileSync('public/wasm/geometry-kernel.wasm')),
 calls:[],phases:[],passed:false}
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
const instances=new WeakSet<WebAssembly.Instance>()
function instrument(instance:WebAssembly.Instance) {
 const actual=instance.exports as any
 if(instances.has(instance)||typeof actual.abi_request!=='function')return
 instances.add(instance)
 Object.defineProperty(instance,'exports',{value:{...actual,abi_request:(operation:number,pointer:number,length:number)=>{
  const bytes=new Uint8Array(actual.memory.buffer,pointer,length)
  const request=operation===0?decodeBinary(bytes) as any:null,op=request?.op??`ABI:${operation}`
  if(op==='brep_miter_owned_place'&&request.expectedResultModel){
   const path=output+'.'+report.calls.length+'.request.json'
   writeFileSync(path,JSON.stringify(request)+'\n')
   if(captureOnly){report.capturedRequest={path,requestSha256:sha(bytes)};throw Error('QA_BOUND_REPLAY_CAPTURE_COMPLETE')}
  }
  const start=performance.now()
  try{return actual.abi_request(operation,pointer,length)}
  finally{report.calls.push({op,milliseconds:performance.now()-start,requestBytes:length})}
 }}})
}
const Instance=WebAssembly.Instance
WebAssembly.Instance=class extends Instance {constructor(m:WebAssembly.Module,i?:WebAssembly.Imports){super(m,i);instrument(this)}}
const instantiate=WebAssembly.instantiate.bind(WebAssembly)
;(WebAssembly as any).instantiate=async(...args:any[])=>{const r=await(instantiate as any)(...args);instrument(r instanceof Instance?r:r.instance);return r}
async function phase<T>(name:string,run:()=>T):Promise<T>{const start=performance.now();try{return run()}finally{report.phases.push({name,milliseconds:performance.now()-start});save()}}
try{
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {readNativeGeometryData}=await import('../src/services/nativeGeometrySnapshot')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const document=compileRushFrontend(placed).document
 const built=await phase('placed-build',()=>buildOwnNurbs(document,{action:'build',display:{segments:4,subdivisionLevels:0}}))
 assert.ok(built.nativeGeometry)
 const model=readNativeGeometryData(built.nativeGeometry).geometry
 await phase('positive-binding',()=>assert.equal(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,model)?.solidGeometryCertified,true))
 const changed=structuredClone(model);changed.faces[0].surface.controlPoints[0][0][0]+=.125
 await phase('changed-model-binding',()=>assert.throws(()=>inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,changed),/final model differs/))
 const original=await phase('original-build',()=>buildOwnNurbs(compileRushFrontend(source).document,{action:'build',display:{segments:4,subdivisionLevels:0}}))
 await phase('original-admission',()=>assert.equal(inspectProgressiveSweepSolidAdmission(original.nativeGeometry!,readNativeGeometryData(original.nativeGeometry!).geometry)?.solidGeometryCertified,true))
 report.passed=true
}catch(error){report.error=String(error);if(captureOnly&&report.capturedRequest&&report.error.includes('QA_BOUND_REPLAY_CAPTURE_COMPLETE'))report.captureComplete=true;else process.exitCode=1}
finally{save();console.log(JSON.stringify({passed:report.passed,captureComplete:report.captureComplete,phases:report.phases,calls:report.calls.filter((c:any)=>c.op==='brep_miter_owned_place')}))}
