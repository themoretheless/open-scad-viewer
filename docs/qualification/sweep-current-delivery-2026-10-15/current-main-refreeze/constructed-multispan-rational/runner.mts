import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'

// Authored curve inputs and QA only: Rust surface_sweep constructs every
// retained surface, then Rust audits the complete multi-span seam collection.
const [root,output]=process.argv.slice(2)
assert.ok(root&&output)
const sha=(x:Uint8Array|string)=>createHash('sha256').update(x).digest('hex')
const build=JSON.parse(readFileSync(join(root,'completed-build-receipt.json'),'utf8'))
assert.equal(build.passed,true)
const report:any={schema:'constructed-multispan-sweep-jets/1',passed:false,
 scope:'Actual native translation-sweep surfaces retaining cubic B-spline path spans; native whole-strip regularity and four-patch G1/G2 closure around the profile. The source path is open. Not moving-frame/periodic-path/source-error/caps/B-rep/Solid/STEP/UI qualification.',
 nativeSilenceLimitMs:30000,maxWorkPerCall:1000000,
 runnerSha256:sha(readFileSync(new URL(import.meta.url))),
 nativeSources:Object.fromEntries(['crates/nurbs-core/src/core/surface.rs','crates/nurbs-core/src/transport.rs',
  'crates/nurbs-core/src/join/continuity/exact_strip.rs','crates/nurbs-core/src/join/continuity/projective_collection.rs']
  .map(p=>[p,sha(readFileSync(join(root,p)))])),artifacts:[]}
mkdirSync(dirname(output),{recursive:true})
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
const profiles=[
 [[1,0],[1,.25],[.75,.5],[.5,.75],[.25,1],[0,1]],
 [[0,1],[-.25,1],[-.5,.75],[-.75,.5],[-1,.25],[-1,0]],
 [[-1,0],[-1,-.25],[-.75,-.5],[-.5,-.75],[-.25,-1],[0,-1]],
 [[0,-1],[.25,-1],[.5,-.75],[.75,-.5],[1,-.25],[1,0]],
].map(p=>({degree:5,knots:[0,0,0,0,0,0,1,1,1,1,1,1],
 controlPoints:p.map(([x,y])=>[x,y,0]),weights:[1,1,1,1,1,1],periodic:false}))
for(const kind of ['raw','packed']){
 const bytes=readFileSync(join(root,kind==='raw'?'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm':'public/wasm/geometry-kernel.wasm'))
 assert.deepEqual({sha256:sha(bytes),byteLength:bytes.length},build.artifacts[kind])
 const wasm=(await WebAssembly.instantiate(bytes)).instance.exports as unknown as {
  memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint
 }
 const item:any={kind,...build.artifacts[kind],passed:false,calls:[],cases:[]}
 report.artifacts.push(item);save()
 function call(name:string,request:any):any{
  const before=JSON.stringify(request),encoded=encodeBinary(request),ptr=writeLinear(wasm.memory,wasm.abi_alloc,encoded)
  assert.ok(ptr)
  let response:any,milliseconds:number
  try{const began=performance.now();response=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,encoded.length));milliseconds=performance.now()-began}
  finally{wasm.abi_free(ptr,encoded.length)}
  item.calls.push({name,request,requestSha256:sha(before),response,milliseconds:milliseconds!});save()
  assert.ok(milliseconds!<=30000);assert.equal(JSON.stringify(request),before)
  assert.equal(response.ok,true,JSON.stringify(response.error));return response.value
 }
 for(const [name,knots,weights] of [
  ['unit',[0,0,0,0,.5,1,1,1,1],[1,1,1,1,1]],
  ['nonuniform-domain',[2,2,2,2,3,6,6,6,6],[1,1,1,1,1]],
  ['rational-unit',[0,0,0,0,.5,1,1,1,1],[1,2,3,2,1]],
  ['rational-nonuniform-domain',[2,2,2,2,3,6,6,6,6],[1,2,3,2,1]],
 ] as const){
  const path={degree:3,knots:[...knots],controlPoints:[[0,0,0],[0,0,.125],[0,0,.5],[0,0,.875],[0,0,1]],weights:[...weights],periodic:false}
  const patches=profiles.map((profile,i)=>call(`${name}-native-sweep-${i}`,{op:'surface_sweep',profile,path}))
  for(const s of patches){
   assert.equal(s.degreeU,5);assert.equal(s.degreeV,3)
   assert.deepEqual(s.knotsV,path.knots)
   assert.equal(new Set(s.knotsV.slice(s.degreeV+1,s.controlPoints[0].length)).size,1,'Real retained interior knot required')
  }
  function audit(label:string,order=2,surfaces=patches,maxWork=1000000){
   const seams=surfaces.map((_:unknown,i:number)=>({patches:[i,(i+1)%4],boundaries:['uMax','uMin'],order,normalScale:1}))
   const proof=call(`${name}-${label}`,{op:'sweep_projective_seams_audit',patches:surfaces,seams,maxWork})
   assert.equal(proof.seams.length,4)
   assert.equal(proof.exactWork,proof.seams.reduce((n:number,s:any)=>n+s.work,0));assert.ok(proof.exactWork<=maxWork)
   assert.deepEqual(proof.unresolvedSeams,proof.seams.flatMap((s:any,i:number)=>s.certified?[]:[i]))
   for(const s of proof.seams)if(s.certified){assert.equal(s.exactIdentity,true);assert.equal(s.regularityCertified,true)}
   return proof
  }
  const g1=audit('whole-strip-g1',1),g2=audit('whole-strip-g2')
  assert.equal(g1.exactG1G2Certified,true);assert.equal(g2.exactG1G2Certified,true)
  assert.equal(audit('zero-work',2,patches,0).exactG1G2Certified,false)
  assert.equal(audit('one-short',2,patches,g2.exactWork-1).exactG1G2Certified,false)
  const changed=structuredClone(patches)
  for(const point of changed[0].controlPoints[2])point[0]=.875
  assert.equal(audit('changed-closing-curvature-g1',1,changed).exactG1G2Certified,true)
  const refused=audit('changed-closing-curvature-g2',2,changed)
  assert.equal(refused.exactG1G2Certified,false);assert.deepEqual(refused.unresolvedSeams,[3])
  assert.ok(refused.seams.slice(0,3).every((s:any)=>s.certified))
  item.cases.push({name,path,profiles,patches,g1,g2});save()
 }
 item.passed=true;save()
}
assert.deepEqual(report.artifacts[0].calls.map(({milliseconds,...call}:any)=>call),
 report.artifacts[1].calls.map(({milliseconds,...call}:any)=>call),'Entire raw/packed constructor and proof responses must match')
report.passed=true;save()
console.log('Four native-constructed polynomial/rational multi-span sweep families: raw/packed geometry, whole-strip profile-closure G1/G2, regularity and refusal parity passed')
