import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

const [path,output]=process.argv.slice(2)
assert.ok(path&&output,'Provide exact WASM path and output report')
const bytes=readFileSync(path)
const wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as unknown as {
 memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
const reply=(op:string,args:object):any=>{
 const encoded=encodeBinary({op,...args}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,encoded)
 assert.ok(ptr)
 try{return decodePacked(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,encoded.length))}
 finally{wasm.abi_free(ptr,encoded.length)}
}
const request=(op:string,args:object):any=>{
 const result=reply(op,args);assert.equal(result.ok,true,JSON.stringify(result.error));return result.value
}
const law=(value:number)=>({degree:1,knots:[0,0,1,1],controlPoints:[[value,0,0],[value,0,0]],weights:[1,1],periodic:false})
const source={loops:[[request('curve_circle',{center:[0,0,0],normal:[0,0,1],radius:0.5})]],
 path:request('curve_line',{start:[0,0,0],end:[0,0,10]}),scale:law(1),twist:law(0),normal:[1,0,0],orientation:'rmf',spacing:'parameter',
 initial_sections:3,max_sections:3,max_deviation:0.01,circle_correction_quantum:0.125,circle_correction_tolerance:1e-9,circle_correction_max_work:1000000,
 station_quantum:0.125,station_tolerance:1e-9,station_max_work:1000000,smoothness_max_work:2000000}
const op='brep_nurbs_progressive_profile_body_reconstruct_stations',body=request(op,source),candidate=body.stationReconstruction
assert.equal(candidate.boundaryCertificate.continuousBound,true)
assert.equal(candidate.boundaryCertificate.withinBudget,true)
assert.equal(candidate.boundaryCertificate.budget,source.max_deviation)
assert.equal(candidate.profileSmoothness.profile.exactG1G2Certified,true)
assert.equal(candidate.profileSmoothness.station.stationG2Certified,true)
assert.deepEqual(request(op,{...source,expected_source_model:body.model}),body)
const tainted=structuredClone(body.model);tainted.vertices[0].point[0]+=1e-12
assert.equal(reply(op,{...source,expected_source_model:tainted}).ok,false)
assert.equal(request(op,{...source,station_max_work:0}).stationReconstruction.reconstruction,null)
const model=candidate.reconstruction.model
const volume=request('brep_sweep_volume_audit',{model,capFaces:[model.faces.length-2,model.faces.length-1],...DEFAULT_SWEEP_VOLUME_BUDGETS})
assert.equal(volume.solidGeometryCertified,true)
assert.equal(volume.allPairsClassified,true)
let snapshot:object|undefined
const snapshotPath=process.argv.find(arg=>arg.startsWith('--snapshot-json='))?.slice('--snapshot-json='.length)
if(snapshotPath){
 const text=readFileSync(snapshotPath,'utf8'),packet=request('native_geometry_snapshot_pack',{json:text})
 assert.equal(request('native_geometry_snapshot_unpack',{snapshot:packet}),text)
 assert.ok(JSON.stringify(packet).length<=4*1024*1024)
 assert.equal(reply('native_geometry_snapshot_unpack',{snapshot:{...packet,decodedBytes:1}}).ok,false)
 snapshot={originalBytes:Buffer.byteLength(text),encodedCharacters:JSON.stringify(packet).length,exactJsonRestored:true,mutatedHeaderRefused:true}
}
writeFileSync(output,JSON.stringify({schema:'progressive-station-abi/1',scope:'Exact supplied WASM ABI; no packaged Rush/UI or all-mode claim',wasmPath:path,
 sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length,boundaryCertificate:candidate.boundaryCertificate,
 profileSmoothness:candidate.profileSmoothness,volume,sourceModelBound:true,mutatedSourceRefused:true,zeroStationWorkRefused:true,snapshot},null,2)+'\n')
console.log('Native WASM station G2, complete bound, fresh Solid and refusal controls passed')
