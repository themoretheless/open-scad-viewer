import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'

const [wasmPath,output]=process.argv.slice(2)
assert.ok(wasmPath&&output)
// The supplied WASM audits the actual retained model of this Rust-built source.
// This is a seam regression, not qualification of a newly packaged full UI.
const source=readFileSync('examples/rush/closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r','utf8')
const built=buildOwnNurbs(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
assert.ok(built.nativeGeometry)
const model=readNativeGeometryData(built.nativeGeometry).geometry
const bytes=readFileSync(wasmPath),wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as unknown as {
 memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
const encoded=encodeBinary({op:'brep_miter_profile_smoothness_audit',model,capFaces:[],maxWork:2000000})
const ptr=writeLinear(wasm.memory,wasm.abi_alloc,encoded)
assert.ok(ptr)
let value:any
try{
 const reply=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,encoded.length))
 assert.equal(reply.ok,true,JSON.stringify(reply.error));value=reply.value
}finally{wasm.abi_free(ptr,encoded.length)}
assert.equal(value.profile.exactG1G2Certified,true)
assert.equal(value.stationContinuity,'C0')
assert.equal(value.station.edgeIds.length,128)
assert.equal(value.station.g2.seams.filter((s:any)=>s.certified).length,96)
assert.equal(value.station.g2.seams.filter((s:any)=>!s.certified).length,32)
assert.ok(value.totalExactWork<=2000000)
writeFileSync(output,JSON.stringify({scope:'Exact supplied WASM retained-station seam regression, separately built source model',
 sha256:createHash('sha256').update(bytes).digest('hex'),modelSha256:createHash('sha256').update(JSON.stringify(model)).digest('hex'),
 sourceSha256:createHash('sha256').update(source).digest('hex'),profileSmoothness:value},null,2)+'\n')
console.log('All 96 applicable retained miter G2 joins proved; 32 sharp joins retain C0')
