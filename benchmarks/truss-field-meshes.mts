import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {trussFieldMeshes} from '../src/services/trussFieldMeshes'
import type {TrussModel,TrussResponse} from '../src/services/trussAnalysis'
import type {MeshData} from '../src/core/mesh'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {setOptionalWasmCompiler} from '../src/services/wasmCompilation'
import {compileWasmArtifact} from '../src/services/wasmArtifact'

const artifact=readFileSync('public/wasm/geometry-kernel.wasm')
setOptionalWasmCompiler(async(_,identity)=>{assert.ok(identity);return compileWasmArtifact(artifact,identity)})
await warmGeometryKernel()
const response=(axialForcesN:number[]):TrussResponse=>({axialForcesN,axialStressesMpa:[],displacementsMm:[],reactionsN:[],maxDeflectionMm:0,maxRelativeResidual:0,freeDofs:0})
const stats=(values:number[])=>{const sorted=[...values].sort((a,b)=>a-b);return {p50Ms:sorted[15],p95Ms:sorted[29],samplesMs:values}}
function canonical(meshes:MeshData[]){
  return meshes.flatMap(mesh=>Array.from({length:mesh.vertices.length/6},(_,i)=>`${mesh.color}:${Array.from(mesh.vertices.subarray(i*6,i*6+6))}`)).sort()
}
const results=[]
for(const count of [36,400]){
  const nodesMm:TrussModel['nodesMm']=Array.from({length:125},(_,i)=>[i%5*10,Math.floor(i/5)%5*10,Math.floor(i/25)*10])
  const members:TrussModel['members']=[]
  for(let a=0;a<nodesMm.length&&members.length<count;a++)for(let b=a+1;b<nodesMm.length&&members.length<count;b++)members.push({nodes:[a,b],youngMpa:2000,areaMm2:2})
  const model:TrussModel={nodesMm,members,restrained:[],forcesN:[]},values=response(members.map((_,i)=>i%3-1))
  const grouped=()=>trussFieldMeshes(model,values,1)
  const separate=()=>members.flatMap((member,i)=>trussFieldMeshes({...model,members:[member]},response([values.axialForcesN[i]]),1))
  const expected=grouped()
  assert.deepEqual(canonical(separate()),canonical(expected))
  for(let i=0;i<20;i++){grouped();separate()}
  const batchMs:number[]=[],separateMs:number[]=[]
  for(let i=0;i<31;i++)for(const batch of i%2?[true,false]:[false,true]){
    const start=performance.now(),meshes=batch?grouped():separate(),elapsed=performance.now()-start
    ;(batch?batchMs:separateMs).push(elapsed)
    assert.equal(meshes.reduce((sum,mesh)=>sum+mesh.indices.length/3,0),count*8)
  }
  results.push({members:count,triangles:count*8,batchMeshes:expected.length,separateMeshes:count,batch:stats(batchMs),separate:stats(separateMs)})
}
console.log(JSON.stringify({node:process.version,arch:process.arch,warmups:20,samples:31,
  scope:'Warm native-WASM-assisted display mesh preparation in Node; same marker geometry grouped by sign vs one mesh per member, not browser frame latency',results},null,2))
