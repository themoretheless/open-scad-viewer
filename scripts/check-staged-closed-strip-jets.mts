import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'

// Authored tensor fixtures and ABI transport only. Rust proves every full
// seam, regularity and the shared exact-work ledger, including the closing seam.
const [root,output]=process.argv.slice(2)
assert.ok(root&&output)
const hash=(x:Uint8Array|string)=>createHash('sha256').update(x).digest('hex')
const completed=JSON.parse(readFileSync(join(root,'completed-build-receipt.json'),'utf8'))
const report:any={schema:'sweep-closed-multispan-strip-jets/1',passed:false,artifacts:[],
 scope:'Synthetic four-patch closed tensor strip with genuine cubic B-spline along-seam spans. Native whole-interval sewn G1/G2, regularity and refusal coverage; not a B-rep topology/source-error/Solid/UI claim.',
 nativeSilenceLimitMs:30000,sharedMaxWork:1000000,
 nativeSourceSha256:hash(readFileSync(join(root,'crates/nurbs-core/src/join/continuity/projective_collection.rs')))}
mkdirSync(dirname(output),{recursive:true})
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
save()
const profiles=[
 [[1,0],[1,.25],[.75,.5],[.5,.75],[.25,1],[0,1]],
 [[0,1],[-.25,1],[-.5,.75],[-.75,.5],[-1,.25],[-1,0]],
 [[-1,0],[-1,-.25],[-.75,-.5],[-.5,-.75],[-.25,-1],[0,-1]],
 [[0,-1],[.25,-1],[.5,-.75],[.75,-.5],[1,-.25],[1,0]],
]
const original=profiles.map(profile=>({degreeU:3,degreeV:5,
 knotsU:[0,0,0,0,.5,1,1,1,1],knotsV:[0,0,0,0,0,0,1,1,1,1,1,1],
 controlPoints:[0,.125,.5,.875,1].map(z=>profile.map(([x,y])=>[x,y,z])),
 weights:[1,2,3,2,1].map(w=>Array(6).fill(w)),periodicU:false,periodicV:false}))
for(const kind of ['raw','packed']){
 const bytes=readFileSync(join(root,kind==='raw'?'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm':'public/wasm/geometry-kernel.wasm'))
 assert.deepEqual({sha256:hash(bytes),byteLength:bytes.length},completed.artifacts[kind])
 const wasm=(await WebAssembly.instantiate(bytes)).instance.exports as unknown as {
  memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint
 }
 const item:any={kind,...completed.artifacts[kind],passed:false,calls:[]};report.artifacts.push(item);save()
 function inspect(name:string,patches=original,order=2,maxWork=1000000,invalid=false){
  const seams=patches.map((_,i)=>({patches:[i,(i+1)%4],boundaries:['vMax','vMin'],order,normalScale:1}))
  if(invalid)seams[3].patches[1]=99
  const request={op:'sweep_projective_seams_audit',patches,seams,maxWork},before=JSON.stringify(request)
  const data=encodeBinary(request),ptr=writeLinear(wasm.memory,wasm.abi_alloc,data);assert.ok(ptr)
  let response:any,milliseconds:number
  try{const start=performance.now();response=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,data.length));milliseconds=performance.now()-start}
  finally{wasm.abi_free(ptr,data.length)}
  assert.ok(milliseconds!<=30000);assert.equal(JSON.stringify(request),before)
  item.calls.push({name,request,requestSha256:hash(before),response,milliseconds:milliseconds!});save()
  if(invalid){assert.equal(response.ok,false);assert.match(response.error.message,/patch index/);return}
  assert.equal(response.ok,true,JSON.stringify(response.error))
  const proof=response.value
  assert.equal(proof.seams.length,4);assert.equal(proof.exactWork,proof.seams.reduce((work:number,s:any)=>work+s.work,0))
  assert.ok(proof.exactWork<=maxWork)
  assert.deepEqual(proof.unresolvedSeams,proof.seams.flatMap((s:any,i:number)=>s.certified?[]:[i]))
  assert.ok(proof.seams.every((s:any)=>s.reason!=='seam-audit-skipped-after-refusal'))
  for(const seam of proof.seams)if(seam.certified){assert.equal(seam.exactIdentity,true);assert.equal(seam.regularityCertified,true)}
  return proof
 }
 const g1=inspect('closed-g1',original,1),g2=inspect('closed-g2')
 assert.equal(g1.exactG1G2Certified,true);assert.equal(g1.certifiedOrder,1)
 assert.equal(g2.exactG1G2Certified,true);assert.equal(g2.certifiedOrder,2)
 assert.equal(inspect('zero-shared-work',original,2,0).exactG1G2Certified,false)
 assert.equal(inspect('short-shared-work',original,2,g2.exactWork-1).exactG1G2Certified,false)
 const last=structuredClone(original)
 for(const row of last[0].controlPoints)row[2][0]=.875
 assert.equal(inspect('changed-closing-curvature-still-g1',last,1).exactG1G2Certified,true)
 const closing=inspect('changed-closing-curvature-g2-refused',last)
 assert.equal(closing.exactG1G2Certified,false);assert.deepEqual(closing.unresolvedSeams,[3])
 assert.ok(closing.seams.slice(0,3).every((s:any)=>s.certified));assert.equal(closing.seams[3].exactIdentity,false)
 const first=structuredClone(original)
 for(const row of first[1].controlPoints)row[2][1]=.875
 const early=inspect('first-refusal-must-not-hide-later-seams',first)
 assert.deepEqual(early.unresolvedSeams,[0]);assert.ok(early.seams.slice(1).every((s:any)=>s.certified))
 inspect('malformed-closing-declaration',original,2,1000000,true)
 item.passed=true;save();console.log(kind,'four closed multispan seams /',item.calls.length,'native observations passed')
}
assert.deepEqual(report.artifacts[0].calls.map((c:any)=>[c.name,c.requestSha256,c.response]),report.artifacts[1].calls.map((c:any)=>[c.name,c.requestSha256,c.response]))
report.runnerSha256=hash(readFileSync(new URL(import.meta.url)))
report.passed=true;save()
