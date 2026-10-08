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
const outer=request('curve_circle',{center:[0,0,0],normal:[1,0,0],radius:.1})
const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],controlPoints:values.map(v=>[v,0,0]),weights:[1,1],periodic:false})
const path=request('curve_bezier',{points:[[0,0,0],[1,0,0],[2,1,0],[3,1,1]]})
const results=[]
const inspect=(body:any,name:string)=>{
 if(!process.argv.includes('--volume'))return undefined
 const model=body.model,capFaces=[model.faces.length-2,model.faces.length-1]
 const volume=request('brep_sweep_volume_audit',{model,capFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS})
 writeFileSync(output.replace(/\.json$/,'-last-volume.json'),JSON.stringify({artifact:{sha256,byteLength:bytes.length},name,modelSha256:hash(encodeBinary(model)),faces:model.faces.length,boundaryErrorUpper:body.boundaryErrorUpper,errorCells:body.approximation.report.errorCertificateCells,volume},null,2)+'\n')
 console.log(JSON.stringify({name,faces:model.faces.length,solidGeometryCertified:volume.solidGeometryCertified,individualPairs:volume.boundary.individualPairs,groupedPairs:volume.boundary.groupedPairs,obliqueGroups:volume.boundary.disjointGroups?.filter((group:any)=>group.projection!=null).length,nextPair:volume.boundary.nextPair,errorCells:body.approximation.report.errorCertificateCells}))
 assert.equal(volume.solidGeometryCertified,true)
 for(const key of ['allFacesInjective','allPairsClassified','boundaryEmbeddingCertified','exactBoundaryCertified','trimCertified'])assert.equal(volume.boundary[key],true,key)
 assert.deepEqual(volume.boundary.unresolvedFaces,[])
 assert.equal(volume.boundary.caps.length,2)
 for(const cap of volume.boundary.caps){assert.equal(cap.capCertified,true);assert.deepEqual(cap.unresolvedWalls,[])}
 const refused=request('brep_sweep_volume_audit',{model,capFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1})
 assert.equal(refused.solidGeometryCertified,false)
 return {certified:volume,limitedBudgetRefusal:refused}
}
const common={path,scale:scalar([1,1]),twist:scalar([0,0]),normal:[0,0,1],orientation:'corrected_frenet',spacing:'parameter',initial_sections:3,max_sections:129,max_deviation:.05,cap_correction_tolerance:1e-9,cap_correction_quantum:2**-40,cap_correction_max_work:1000000}
const hole=request('curve_reverse',{curve:request('curve_circle',{center:[0,0,0],normal:[1,0,0],radius:.05})})
const vector=(v:number[])=>({degree:1,knots:[0,0,1,1],controlPoints:[v,v],weights:[1,1],periodic:false})
for(const [name,loops,extra] of [
 ['nonplanar',[[outer]],{}],
 ['nonplanar-hollow',[[outer],[hole]],{}],
 ['nonplanar-hollow-affine',[[outer],[hole]],{axis_scale:vector([1.25,.75,1.1]),center_law:vector([.01,-.02,.03])}],
 ['arc-nonplanar-hollow-affine',[[outer],[hole]],{axis_scale:vector([1.25,.75,1.1]),center_law:vector([.01,-.02,.03]),spacing:'arc_length',length_tolerance:1e-6,length_max_cells:100000}],
] as const){
 const result=request('brep_nurbs_progressive_profile_body',{...common,loops,...extra})
 assert.equal(result.boundaryContinuousBound,true,name)
 assert.equal(result.boundaryErrorWithinBudget,true,name)
 assert.equal(result.filledCapErrorUpper.length,2,name)
 assert.ok(result.boundaryErrorUpper<=.05,name)
 assert.equal(result.globalEmbeddingCertified,false,name)
 results.push({name,upper:result.boundaryErrorUpper,faces:result.model.faces.length,volume:inspect(result,name)})
}
const ring=(points:number[][])=>points.slice(1).map((p,i)=>request('curve_line',{start:points[i],end:p}))
const straight=request('brep_nurbs_progressive_profile_body',{
 loops:[ring([[0,0,0],[2,0,0],[2,2,0],[0,2,0],[0,0,0]]),ring([[.5,.5,0],[.5,1.5,0],[1.5,1.5,0],[1.5,.5,0],[.5,.5,0]])],
 path:request('curve_line',{start:[0,0,0],end:[0,0,10]}),scale:scalar([1,1]),twist:scalar([0,0]),axis_scale:vector([2,3,1]),center_law:vector([.125,-.25,0]),normal:[1,0,0],orientation:'corrected_frenet',spacing:'parameter',initial_sections:65,max_sections:65,max_deviation:.01})
assert.equal(straight.model.faces.length,514)
assert.equal(straight.boundaryContinuousBound,true)
assert.equal(straight.boundaryErrorWithinBudget,true)
assert.ok(straight.approximation.report.errorCertificateCells<=10000)
results.push({name:'literal-straight-65',upper:straight.boundaryErrorUpper,cells:straight.approximation.report.errorCertificateCells,faces:514,volume:inspect(straight,'literal-straight-65')})
writeFileSync(output,JSON.stringify({artifact:{sha256,byteLength:bytes.length},scope:'Fresh staged ABI finite corrected body error checks; global embedding and full library completion are not asserted',results},null,2)+'\n')
console.log(`${results.length} corrected ABI error cases passed`)
if(process.argv.includes('--volume'))console.log(`${results.length} fresh Solid proofs and ${results.length} limited-budget refusals passed`)
