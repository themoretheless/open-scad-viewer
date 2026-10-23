import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import {performance} from 'node:perf_hooks'
import path from 'node:path'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {validateBrepProfile} from '../src/services/geometry/brepProfile'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import identity from '../src/generated/geometry-kernels/identity'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {decodePacked,writeLinear} from '../src/services/wasmHost'
import type {BrepProfile} from '../src/services/geometry/brepProfile'
function loop(x:number,y:number,size:number):NurbsCurve[]{
 const p=[[x,y],[x+size,y],[x+size,y+size],[x,y+size]]
 const wire:NurbsCurve[]=p.map((a,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[a,p[(i+1)%4]],weights:[1,1],periodic:false}))
 wire[0]={degree:2,knots:[0,0,0,.5,1,1,1],controlPoints:[[x,y],[x+size*.25,y-size*.1],[x+size*.75,y-size*.1],[x+size,y]],weights:[1,.8,1.2,1],periodic:false}
 return wire
}
const directory=path.resolve(process.argv[2]??'/tmp/general-profile-benchmark')
const fixture=[loop(3,3,1),loop(20,0,3),loop(0,0,10),loop(2,2,4)]
const source=JSON.stringify(fixture),hash=(s:string|Buffer)=>createHash('sha256').update(s).digest('hex')
const before=process.argv.find(a=>a.startsWith('--before='))?.slice(9)
const kernels:{identity:{sha256:string;byteLength:number};run:()=>BrepProfile}[]=[]
if(before){
 for(const file of [before,'public/wasm/geometry-kernel.wasm']){
  const bytes=await readFile(file),instance=await WebAssembly.instantiate(bytes),wasm=instance.instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
  kernels.push({identity:{sha256:hash(bytes),byteLength:bytes.length},run:()=>{
   const request=encodeBinary({op:'brep_profile_validate',loops:fixture,fillRule:'even-odd',toleranceMm:1e-7})
   const pointer=writeLinear(wasm.memory,n=>wasm.abi_alloc(n),request);assert.ok(pointer)
   try{
    const response=decodePacked<{ok:boolean;value:BrepProfile;error?:unknown}>(wasm.memory,(p,n)=>wasm.abi_free(p,n),wasm.abi_request(0,pointer,request.length))
    assert.ok(response.ok,JSON.stringify(response.error));return response.value
   }finally{wasm.abi_free(pointer,request.length)}
  }})
 }
 assert.notEqual(kernels[0].identity.sha256,kernels[1].identity.sha256,'Comparison requires different artifacts')
}else{
 assert.equal(hash(await readFile('public/wasm/geometry-kernel.wasm')),identity.sha256)
 await warmGeometryKernel();kernels.push({identity,run:()=>validateBrepProfile(fixture,'even-odd')})
}
await mkdir(directory,{recursive:true})
const expected=kernels[0].run(),samples=kernels.map(()=>[] as number[])
for(const k of kernels)assert.deepEqual(k.run(),expected)
for(let i=0;i<7;i++){
 const order=kernels.map((_,j)=>j);if(i%2)order.reverse()
 for(const j of order){const start=performance.now(),result=kernels[j].run();samples[j].push(performance.now()-start);assert.deepEqual(result,expected);assert.equal(JSON.stringify(fixture),source)}
}
const report={scope:'Node/WASM retained region validation and binary ABI only; excludes UI, worker messaging and rendering',alternatingArtifacts:!!before,fixtureSha256:hash(source),resultSha256:hash(JSON.stringify(expected)),sourcePreserved:true,kernels:kernels.map((k,j)=>{const times=[...samples[j]].sort((a,b)=>a-b);return {identity:k.identity,iterations:times.length,samplesMs:samples[j],medianMs:times[3],maxMs:times.at(-1)}})}
await writeFile(path.join(directory,'benchmark.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report,null,2))
