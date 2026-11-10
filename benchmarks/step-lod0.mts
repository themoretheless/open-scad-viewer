import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {dirname,resolve} from 'node:path'
import os from 'node:os'
import {createStepLod0BenchmarkFixture} from './step-lod0-fixture'

const values=new Map<string,string>()
Object.defineProperty(globalThis,'localStorage',{configurable:true,value:{
  getItem:(key:string)=>values.get(key)??null,
  setItem:(key:string,value:string)=>{values.set(key,value)},
  removeItem:(key:string)=>{values.delete(key)},
  key:(index:number)=>[...values.keys()][index]??null,
  get length(){return values.size},
}})

const {importStepForWorkbench}=await import('../src/kernel/cadStepRouting')
const {estimateDirectStepLod0Hausdorff}=await import('../src/kernel/cadNurbsStep')
const {stepLod0Cache,MAX_LOD0_MEMORY_BYTES,MAX_LOD0_DURABLE_BYTES,MAX_LOD0_SOURCE_INDEX_BYTES,MAX_LOD0_ENTRIES}=await import('../src/services/stepLod0Cache')
const fixture=createStepLod0BenchmarkFixture()
const sha256=(value:string|Uint8Array)=>createHash('sha256').update(value).digest('hex')
const summarize=(samplesMs:number[])=>{
  const sorted=[...samplesMs].sort((a,b)=>a-b)
  const percentile=(fraction:number)=>sorted[Math.min(sorted.length-1,Math.ceil(sorted.length*fraction)-1)]
  return {samplesMs,p50Ms:percentile(0.5),p95Ms:percentile(0.95),minMs:sorted[0],maxMs:sorted.at(-1)}
}
const timed=<T>(fn:()=>T)=>{const start=performance.now();const value=fn();return {value,ms:performance.now()-start}}

stepLod0Cache.clear()
const cold=timed(()=>importStepForWorkbench(fixture.mixedText))
assert.equal(cold.value.report.route,'retained-ap242-brep')
assert.equal(cold.value.lod0?.requiresRetessellation,false)
assert.deepEqual(Array.from(cold.value.bodies[0].mesh.positions),fixture.lod0Mesh.positions)
assert.deepEqual(Array.from(cold.value.bodies[0].mesh.indices),fixture.lod0Mesh.indices)
const durable=timed(()=>{stepLod0Cache.reload();return importStepForWorkbench(fixture.mixedText)})
assert.equal(durable.value.lod0?.cacheKey,cold.value.lod0?.cacheKey)

const warmups=3,samples=15
for(let index=0;index<warmups;index++)importStepForWorkbench(fixture.mixedText)
const memoryShell=importStepForWorkbench(fixture.mixedText).lod0?.lod0Shells[0]
const directSamples:number[]=[]
for(let index=0;index<samples;index++){
  const run=timed(()=>importStepForWorkbench(fixture.mixedText))
  assert.equal(run.value.lod0?.lod0Shells[0],memoryShell,'repeat import must be a memory cache hit')
  directSamples.push(run.ms)
}
for(let index=0;index<warmups;index++)importStepForWorkbench(fixture.exactText)
const retessellationSamples:number[]=[]
for(let index=0;index<samples;index++){
  const run=timed(()=>importStepForWorkbench(fixture.exactText))
  assert.deepEqual(Array.from(run.value.bodies[0].mesh.positions),fixture.exactMesh.positions)
  assert.deepEqual(Array.from(run.value.bodies[0].mesh.indices),fixture.exactMesh.indices)
  retessellationSamples.push(run.ms)
}
const isolatedHit=timed(()=>stepLod0Cache.getDocument(fixture.mixedText))
assert.ok(isolatedHit.value)
const hausdorff=estimateDirectStepLod0Hausdorff(fixture.exactMesh,fixture.lod0Mesh)
assert.equal(hausdorff.certified,false)
assert.equal(hausdorff.method,'two-sided-bvh-sampled-hausdorff-estimate')
assert.ok(hausdorff.max<=1e-12)

const shell=cold.value.lod0!.lod0Shells[0]
const meshBytes=shell.mesh.positions.byteLength+shell.mesh.indices.byteLength
const cacheStats=stepLod0Cache.stats()
const durableBytes=[...values].filter(([key])=>key.startsWith('open-scad-viewer.step-lod0-v1.'))
  .reduce((sum,[,value])=>sum+new TextEncoder().encode(value).byteLength,0)
assert.ok(cacheStats.bytes<=MAX_LOD0_MEMORY_BYTES)
assert.ok(cacheStats.entries<=MAX_LOD0_ENTRIES)
assert.ok(durableBytes<=MAX_LOD0_DURABLE_BYTES)
assert.ok(new TextEncoder().encode(fixture.mixedText).byteLength<=MAX_LOD0_SOURCE_INDEX_BYTES)
assert.ok(shell.mesh.positions.length<=300_000&&shell.mesh.indices.length<=300_000)
const direct=summarize(directSamples),retessellation=summarize(retessellationSamples)
const report={
  schema:1,
  environment:{runtime:`Node ${process.version}`,platform:process.platform,arch:process.arch,cpu:os.cpus()[0]?.model??'unknown',logicalCores:os.cpus().length,browserMeasured:false},
  fixture:{kind:'mixed exact AP242 MANIFOLD_SOLID_BREP + linked TESSELLATED_SHELL',sourceBytes:new TextEncoder().encode(fixture.mixedText).byteLength,sourceSha256:sha256(fixture.mixedText),vertices:shell.vertexCount,triangles:shell.triangleCount,meshBytes},
  timing:{warmups,samplesPerPath:samples,coldDecodeMs:cold.ms,durableReloadHitMs:durable.ms,isolatedMemoryHitMs:isolatedHit.ms,directCachedLod0:direct,exactRetessellation:retessellation,p50RatioDirectOverRetessellation:direct.p50Ms/retessellation.p50Ms,
    scope:'Whole synchronous frontend route includes exact import and scene import. Direct path reads baked LOD-0; comparison exact-only path tessellates the same exact box at 8 segments. Durable reload clears only memory. No paint, GPU, Worker transport, or browser frame timing.'},
  budgets:{meshBytes,memoryBytes:cacheStats.bytes,memoryLimitBytes:MAX_LOD0_MEMORY_BYTES,durableBytes,durableLimitBytes:MAX_LOD0_DURABLE_BYTES,sourceIndexLimitBytes:MAX_LOD0_SOURCE_INDEX_BYTES,entries:cacheStats.entries,entryLimit:MAX_LOD0_ENTRIES,positionComponents:shell.mesh.positions.length,indexComponents:shell.mesh.indices.length,documentComponentLimit:300_000},
  hausdorff,
  threshold:{candidateMs:200,status:'open',reason:'Node/WASM on this host is not a controlled browser measurement; no pass/fail claim is made for <200 ms.'},
  artifacts:['src/generated/geometry-kernels/kernel_bg.wasm'].map(path=>({path,sha256:sha256(readFileSync(path))})),
}
const out=resolve(process.argv[2]??'tmp/performance/step-lod0.json')
mkdirSync(dirname(out),{recursive:true})
writeFileSync(out,JSON.stringify(report,null,2))
console.log(JSON.stringify(report,null,2))
