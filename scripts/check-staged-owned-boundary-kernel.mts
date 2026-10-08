import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
const root=process.argv[2]!,output=process.argv[3]!
assert.ok(root&&output,'Staged root and report path are required')
const bytes=readFileSync(`${root}/public/wasm/geometry-kernel.wasm`)
const hash=(b:Uint8Array)=>createHash('sha256').update(b).digest('hex')
const sha256=hash(bytes)
assert.equal(hash(readFileSync(`${root}/src/generated/geometry-kernels/kernel_bg.wasm`)),sha256)
assert.ok(readFileSync(`${root}/src/generated/geometry-kernels/identity.ts`,'utf8').includes(sha256))
const wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as unknown as {
 memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
const request=(op:string,args:object):any=>{
 const encoded=encodeBinary({op,...args}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,encoded)
 assert.ok(ptr)
 try{
  const reply=decodePacked<{ok:boolean;value:unknown;error:unknown}>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,encoded.length))
  assert.ok(reply.ok,JSON.stringify(reply.error));return reply.value
 }finally{wasm.abi_free(ptr,encoded.length)}
}
// Same immutable source-owned replay used by the native regression.
const closed=process.argv.includes('--closed')
const source=JSON.parse(readFileSync(`crates/geometry-bridge/src/fixtures/${closed?'closed-periodic-profile-shear-request':'periodic-automatic-cap-shear-request'}.json`,'utf8'))
const snapshot=JSON.stringify(source)
const result=request(source.op,source)
assert.equal(JSON.stringify(source),snapshot)
assert.equal(result.solidGeometryCertified,true)
const boundary=result.volume.boundary
for(const key of ['boundaryEmbeddingCertified','exactBoundaryCertified','trimCertified','allFacesInjective','allPairsClassified'])assert.equal(boundary[key],true,key)
assert.deepEqual(boundary.unresolvedFaces,[])
assert.equal(boundary.caps.length,closed?0:2)
assert.ok(boundary.caps.every((cap:any)=>cap.capCertified&&cap.unresolvedWalls.length===0))
const bound=request(source.op,{...source,expectedResultModel:result.placement.model})
assert.equal(bound.resultModelBound,true)
assert.deepEqual(bound.volume.boundary,boundary)
const denied=request(source.op,{...source,wallCells:0})
assert.equal(denied.solidGeometryCertified,false)
assert.equal(denied.placement.model,null)
writeFileSync(output,JSON.stringify({schema:'staged-owned-boundary/1',artifact:{sha256,byteLength:bytes.length,stagedPublicAndGeneratedVerified:true},scope:'Direct staged ABI replay; no published Rush/UI/STEP claim',closed,boundary,volume:result.volume,finalModelBound:bound.resultModelBound,zeroWallBudgetRefused:!denied.solidGeometryCertified},null,2)+'\n')
console.log('Source-owned periodic affine cap boundary, final-model binding and zero-budget refusal passed')
