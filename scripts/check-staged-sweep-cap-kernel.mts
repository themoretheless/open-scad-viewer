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
const cases=[]
for(const mode of ['oblique-rmf','curved-rmf','curved-frenet']){
 if(process.argv.includes('--oblique-only')&&mode!=='oblique-rmf')continue
 const oblique=mode==='oblique-rmf',axis=oblique?[3,4,0]:[1,0,0]
 const outer=request('curve_circle',{center:[0,0,0],normal:axis,radius:.1})
 const hole=request('curve_reverse',{curve:request('curve_circle',{center:[0,0,0],normal:axis,radius:.05})})
 const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],controlPoints:values.map(v=>[v,0,0]),weights:[1,1],periodic:false})
 const args={loops:[[outer],[hole]],path:request('curve_bezier',{points:oblique?[[0,0,0],[3,4,0]]:[[0,0,0],[.5,0,0],[1,1,0]]}),
  scale:scalar(oblique?[1,2]:[1,1]),twist:scalar([0,0]),normal:[0,0,1],orientation:mode==='curved-frenet'?'frenet':'rmf',spacing:'parameter',
  initial_sections:process.argv.includes('--sections=65')?65:oblique?2:3,max_sections:process.argv.includes('--sections=65')?65:oblique?2:129,max_deviation:.01,
  cap_correction_tolerance:1e-9,cap_correction_quantum:2**-40,cap_correction_max_work:1000000}
 const body=request('brep_nurbs_progressive_profile_body',args)
 if(process.argv.includes('--details'))writeFileSync(output.replace(/\.json$/,`-${mode}-body.json`),JSON.stringify(body,null,2)+'\n')
 assert.equal(body.boundaryContinuousBound,true);assert.equal(body.boundaryErrorWithinBudget,true)
 assert.equal(body.retainedCaps.exact,true);assert.equal(body.retainedCaps.continuousBound,false);assert.equal(body.globalEmbeddingCertified,false)
 assert.equal(body.filledCapErrorUpper.length,2)
 assert.ok(body.filledCapErrorUpper.every((e:number)=>e>=0&&e<=.01))
 const model=body.model,capFaces=[model.faces.length-2,model.faces.length-1]
 const budgets=process.argv.includes('--original-native-limits')
  ?{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:10000,maxCells:100000,maxLinearCells:20000}:DEFAULT_SWEEP_VOLUME_BUDGETS
 if(process.argv.includes('--sections=65'))assert.equal(model.faces.length,514)
 const volume=request('brep_sweep_volume_audit',{model,capFaces,...budgets})
 assert.equal(volume.solidGeometryCertified,true);assert.equal(volume.allFacesInjective,true);assert.equal(volume.allPairsClassified,true)
 const refused=request('brep_sweep_volume_audit',{model,capFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1})
 assert.equal(refused.solidGeometryCertified,false)
 cases.push({mode,faces:model.faces.length,boundaryErrorUpper:body.boundaryErrorUpper,filledCapErrorUpper:body.filledCapErrorUpper,budgets,volume})
 console.log(mode,'staged ABI boundary and fresh Solid passed')
}
writeFileSync(output,JSON.stringify({schema:'staged-sweep-cap-kernel/1',artifact:{sha256,byteLength:bytes.length,stagedPublicAndGeneratedVerified:true},
 scope:'Direct staged WASM ABI checks; no published Rush, viewport, rendered UI or independent STEP claim.',cases},null,2)+'\n')
