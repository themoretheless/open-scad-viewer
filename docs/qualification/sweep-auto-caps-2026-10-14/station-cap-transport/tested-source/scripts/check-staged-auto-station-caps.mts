import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
const [root,output,nativeRoot]=process.argv.slice(2)
assert.ok(root&&output&&nativeRoot)
const raw=process.argv.includes('--raw-built')
const bytes=readFileSync(raw?join(root,'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'):join(root,'public/wasm/geometry-kernel.wasm'))
const sha=(data:Uint8Array)=>createHash('sha256').update(data).digest('hex')
const artifact={sha256:sha(bytes),byteLength:bytes.length,mode:raw?'raw-built-before-optimization':'packed-staged'}
if(!raw)assert.ok(readFileSync(join(root,'src/generated/geometry-kernels/identity.ts'),'utf8').includes(artifact.sha256))
const wasm=(await WebAssembly.instantiate(bytes)).instance.exports as unknown as {
 memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
const calls:{op:string;milliseconds:number}[]=[]
const invoke=(op:string,args:object):any=>{
 const encoded=encodeBinary({op,...args}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,encoded)
 assert.ok(ptr)
 try{
  const start=performance.now()
  const response=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,encoded.length))
  const milliseconds=performance.now()-start
  calls.push({op,milliseconds})
  assert.ok(response.ok,JSON.stringify(response.error))
  assert.ok(milliseconds<=30000,'Original native-silence limit exceeded: '+op)
  return response.value
 }finally{wasm.abi_free(ptr,encoded.length)}
}
const capture=JSON.parse(readFileSync(join(nativeRoot,'manifest.json'),'utf8'))
const entry=capture.cases.find((item:any)=>item.sourceRequestFile==='oblique-rmf-linear-scale-source.json')
assert.ok(entry)
const sourceBytes=readFileSync(join(nativeRoot,entry.sourceRequestFile))
assert.equal(sha(sourceBytes),entry.sourceRequestSha256)
const {op:_,...original}=JSON.parse(sourceBytes.toString())
assert.equal(original.max_deviation,.01)
assert.ok(!('cap_correction_tolerance' in original)&&!('circle_correction_tolerance' in original))
// Authored station-count variant of the captured source; all curves/laws stay unchanged.
const request={...original,initial_sections:3,max_sections:3,smoothness_max_work:2000000}
const owned=invoke('brep_nurbs_progressive_profile_auto_body',request)
const repaired=invoke('brep_nurbs_progressive_profile_auto_body_reconstruct_stations',{
 ...request,expected_source_model:owned.model,station_quantum:2**-40,station_tolerance:1e-9,station_max_work:1000000})
mkdirSync(dirname(output),{recursive:true})
writeFileSync(output,JSON.stringify({artifact,request,owned,repaired,calls,passed:false},null,2)+'\n')
assert.equal(owned.capRepresentation,'radial-annular')
assert.equal(owned.capFaces.length,8)
assert.deepEqual(repaired.capFaceGroups,owned.capFaceGroups)
assert.deepEqual(repaired.capFaces,owned.capFaces)
assert.deepEqual(repaired.planarCapFaces,[])
const candidate=repaired.stationReconstruction
assert.equal(candidate.boundaryCertificate.continuousBound,true)
assert.equal(candidate.boundaryCertificate.withinBudget,true)
assert.equal(candidate.boundaryCertificate.budget,.01)
assert.equal(candidate.profileSmoothness.station.stationG2Certified,true)
for(const face of owned.capFaces)assert.deepEqual(candidate.reconstruction.model.faces[face],owned.model.faces[face])
const start=performance.now()
const session=invoke('brep_sweep_volume_begin',{model:candidate.reconstruction.model,capFaces:repaired.planarCapFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS})
let volume:any
try{
 for(let i=0;i<10000;i++){
  const step=invoke('brep_sweep_volume_step',{session:session.session,maxUnits:32})
  assert.ok(performance.now()-start<=120000,'Original Solid limit exceeded')
  if(step.done){volume=step.report;break}
 }
}finally{invoke('brep_sweep_volume_drop',{session:session.session})}
assert.equal(volume?.solidGeometryCertified,true)
assert.equal(volume.boundary.allPairsClassified,true)
assert.equal(volume.boundary.individualPairs+volume.boundary.groupedPairs,volume.boundary.totalPairs)
writeFileSync(output,JSON.stringify({artifact,request,owned,repaired,volume,calls,passed:true,scope:'One original-source oblique three-station variant; native Auto radial caps, actual retained station G2 and fresh Solid; not Rush/UI or all-mode proof'},null,2)+'\n')
console.log('Native Auto oblique station G2, all eight caps preserved and fresh Solid passed')
