import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {TransparentBsp,type TransparentFragment} from '../src/services/transparentBsp'
import {ReferenceTransparentBsp} from './rush/transparentBsp-reference'
import {warmGeometryKernel,kernelRuntime,decodeNurbsResult} from '../src/services/geometry/kernel'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,packedSize} from '../src/services/wasmHost'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {compileWasmArtifact} from '../src/services/wasmArtifact'
const artifact=readFileSync('public/wasm/geometry-kernel.wasm')
setOptionalWasmCompiler(async(_,identity)=>compileWasmArtifact(artifact,identity!))
await warmGeometryKernel()
const runtime=kernelRuntime(),wasm=runtime.exports
const samples=15,warmups=3
const quantiles=(values:number[])=>{const sorted=[...values].sort((a,b)=>a-b);return {p50Ms:sorted[7],p95Ms:sorted[14]}}
function invoke(source:TransparentFragment[]){
 const start=performance.now(),bytes=encodeBinary({op:'transparent_bsp_build',triangles:source.map(f=>f.triangle),limit:100000,operationLimit:500000,tolerance:0}),encoded=performance.now()
 const pointer=writeLinear(runtime.memory,n=>wasm.abi_alloc(n),bytes),copied=performance.now();assert.ok(pointer)
 try {
  const packed=wasm.abi_request(0,pointer,bytes.length),invoked=performance.now()
  const result=decodeNurbsResult<{count:number}>(runtime.takeResponse(packed)),decoded=performance.now()
  return {count:result.count,requestBytes:bytes.length,responseBytes:packedSize(packed),encodeMs:encoded-start,copyMs:copied-encoded,rustAndSerializeMs:invoked-copied,decodeAndFreeMs:decoded-invoked,totalMs:decoded-start}
 } finally {wasm.abi_free(pointer,bytes.length)}
}
function invokeRaw(source:TransparentFragment[]){
 const start=performance.now(),width=source[0]?.triangle[0].length??0,input=new Float64Array(source.length*3*width)
 let offset=0;for(const f of source)for(const v of f.triangle){input.set(v,offset);offset+=width}
 const encoded=performance.now()
 const pointer=writeLinear(runtime.memory,n=>wasm.abi_alloc(n),new Uint8Array(input.buffer)),copied=performance.now()
 let handle=0
 try {
  const packed=wasm.abi_transparent_bsp(width,pointer,input.length,100000,500000,0),invoked=performance.now()
  handle=decodeNurbsResult<number>(runtime.takeResponse(packed))
  let responseBytes=0
  for(const [slot,bytes] of [[0,8],[2,4],[4,4],[6,8]]){
   const p=wasm.abi_array_field(handle,slot),n=wasm.abi_array_field(handle,slot+1)
   new Uint8Array(runtime.memory.buffer,p,n*bytes).slice();responseBytes+=n*bytes
  }
  const count=wasm.abi_array_field(handle,10)
  wasm.abi_array_free(handle);handle=0
  const decoded=performance.now()
  return {count,requestBytes:input.byteLength,responseBytes,encodeMs:encoded-start,copyMs:copied-encoded,rustAndSerializeMs:invoked-copied,decodeAndFreeMs:decoded-invoked,totalMs:decoded-start}
 }finally{if(handle)wasm.abi_array_free(handle);wasm.abi_free(pointer,input.byteLength)}
}
const results=[]
for(const [name,count] of [['layers',1000],['layers',10000],['coplanar',10000],['crossing',24]] as const){
 const source:TransparentFragment[]=Array.from({length:count},(_,i)=>({owner:String(i),triangle:[[-1,-1],[1,-1],[0,1]].map(([x,y])=>[name==='coplanar'?x+i:x,y,name==='layers'?i:name==='coplanar'?0:Math.sin(i*1.7)*x+Math.cos(i*2.3)*y+i*.017,...Array.from({length:10},(_,k)=>(k+1)*x+y)]) as unknown as TransparentFragment['triangle']}))
 const reference=new ReferenceTransparentBsp(source,100000,0,500000),native=new TransparentBsp(source,100000,0,500000)
 if(name!=='crossing'){assert.equal(native.fragmentCount,reference.fragmentCount);assert.equal(native.operationCount,reference.operationCount)}
 const storage=new Float32Array(native.fragmentCount*39)
 for(let i=0;i<warmups;i++){invoke(source);invokeRaw(source);new TransparentBsp(source,100000,0,500000);new ReferenceTransparentBsp(source,100000,0,500000);native.writeOrdered([.3,.4,1],storage)}
 const stages:ReturnType<typeof invoke>[]=[],rawStages:ReturnType<typeof invokeRaw>[]=[],nativeMs:number[]=[],referenceMs:number[]=[],traversalMs:number[]=[]
 for(let i=0;i<samples;i++){
  stages.push(invoke(source));rawStages.push(invokeRaw(source))
  for(const old of i%2?[true,false]:[false,true]){const start=performance.now();const tree=old?new ReferenceTransparentBsp(source,100000,0,500000):new TransparentBsp(source,100000,0,500000);const end=performance.now();if(name!=='crossing')assert.equal(tree.fragmentCount,reference.fragmentCount);(old?referenceMs:nativeMs).push(end-start)}
  const start=performance.now();native.writeOrdered([.3,.4,1],storage);traversalMs.push(performance.now()-start)
 }
 results.push({name,inputTriangles:count,outputFragments:native.fragmentCount,referenceFragments:reference.fragmentCount,operationLimit:500000,operations:native.operationCount,requestBytes:stages[0].requestBytes,responseBytes:stages[0].responseBytes,nativeConstructor:quantiles(nativeMs),referenceConstructor:quantiles(referenceMs),localTraversal:quantiles(traversalMs),genericStages:Object.fromEntries(['encodeMs','copyMs','rustAndSerializeMs','decodeAndFreeMs','totalMs'].map(key=>[key,quantiles(stages.map(s=>s[key as keyof typeof s]))])),rawRequestBytes:rawStages[0].requestBytes,rawResponseBytes:rawStages[0].responseBytes,rawStages:Object.fromEntries(['encodeMs','copyMs','rustAndSerializeMs','decodeAndFreeMs','totalMs'].map(key=>[key,quantiles(rawStages.map(s=>s[key as keyof typeof s]))]))})
}
console.log(JSON.stringify({artifactSha256:createHash('sha256').update(artifact).digest('hex'),node:process.version,samples,warmups,note:'Generic Rust stage includes request decoding and response encoding. Raw stages pack, upload, build and copy detached buffers; raw stage totals exclude TS node materialization. Measurements are local warmed Node WASM, excluding startup and GPU upload.',results},null,2))
