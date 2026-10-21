// Diagnostic orchestration only; every geometric decision remains Rust-owned.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import {createHash} from 'node:crypto'
import * as module from 'node:module'
import {decodeBinary} from '../src/services/valueBinaryCodec'
import {load} from '../tests/fixtures/raw-import-loader.mjs'

const [sourcePath,outputPath,segmentText]=process.argv.slice(2)
const segments=Number(segmentText);assert.ok(Number.isInteger(segments)&&segments>0)
assert.ok(sourcePath&&outputPath)
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const output=resolve(outputPath),sha=(v:Uint8Array|string)=>createHash('sha256').update(v).digest('hex')
mkdirSync(dirname(output),{recursive:true})
const source=readFileSync(sourcePath,'utf8')
const report:any={scope:'Instrumented original sweep fixture; timings are diagnostic, not a deadline gate.',
 sourcePath,sourceSha256:sha(source),geometryWasmSha256:sha(readFileSync('public/wasm/geometry-kernel.wasm')),
 nodeExecutable:process.execPath,nodeVersion:process.version,calls:[],phases:[],passed:false}
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
const instances=new WeakSet<WebAssembly.Instance>()
function instrument(instance:WebAssembly.Instance){
 const actual=instance.exports as any
 if(instances.has(instance)||typeof actual.abi_request!=='function')return
 instances.add(instance)
 Object.defineProperty(instance,'exports',{value:{...actual,abi_request:(operation:number,pointer:number,length:number)=>{
  const input=new Uint8Array(actual.memory.buffer,pointer,length).slice()
  const request=operation===0?decodeBinary(input) as any:null,op=request?.op??`ABI:${operation}`
  const start=performance.now()
  try{return actual.abi_request(operation,pointer,length)}
  finally{
   const milliseconds=performance.now()-start,index=report.calls.length
   const item:any={op,milliseconds,requestBytes:length}
   if(request&&milliseconds>=100){
    item.requestPath=output+'.'+index+'.request.json';writeFileSync(item.requestPath,JSON.stringify(request)+'\n')
    item.requestBinarySha256=sha(input)
   }
   report.calls.push(item)
  }
 }}})
}
const Instance=WebAssembly.Instance
WebAssembly.Instance=class extends Instance{constructor(m:WebAssembly.Module,i?:WebAssembly.Imports){super(m,i);instrument(this)}}
const instantiate=WebAssembly.instantiate.bind(WebAssembly)
;(WebAssembly as any).instantiate=async(...args:any[])=>{const r=await(instantiate as any)(...args);instrument(r instanceof Instance?r:r.instance);return r}
async function phase<T>(name:string,run:()=>T|Promise<T>):Promise<T>{const start=performance.now();try{return await run()}finally{report.phases.push({name,milliseconds:performance.now()-start});save()}}
try{
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readNativeGeometryData}=await import('../src/services/nativeGeometrySnapshot')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await phase('build',()=>buildOwnNurbsAsync(document,{action:'build',display:{segments,subdivisionLevels:0}}))
 assert.ok(built.nativeGeometry)
 const data=readNativeGeometryData(built.nativeGeometry)
 report.faces=data.geometry.faces.length
 report.segments=segments
 report.poleStencils=Object.entries(data.geometry.faces.reduce((a:any,f:any)=>{const s=f.surface,k=JSON.stringify([s.controlPoints.length,s.controlPoints[0].length,s.degreeU,s.degreeV]);a[k]=(a[k]??0)+1;return a},{}))
 await phase('solid-admission',()=>assert.equal(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,data.geometry)?.solidGeometryCertified,true))
 assert.equal(JSON.stringify(document),before)
 report.passed=true
}catch(error){report.error=String(error);process.exitCode=1}
finally{save();console.log(JSON.stringify({passed:report.passed,phases:report.phases,slowCalls:report.calls.filter((c:any)=>c.milliseconds>=100)}))}
