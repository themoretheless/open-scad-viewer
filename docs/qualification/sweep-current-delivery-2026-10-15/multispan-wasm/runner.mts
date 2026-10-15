import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'

// Qualification transport only. Original tensor surfaces, exact strip jets,
// regularity and every work-limit decision are evaluated by the Rust ABI.
const [root,output]=process.argv.slice(2)
assert.ok(root&&output)
const raw=process.argv.includes('--raw-built'),kind=raw?'raw':'packed'
const hash=(value:Uint8Array|string)=>createHash('sha256').update(value).digest('hex')
const bytes=readFileSync(join(root,raw?'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm':'public/wasm/geometry-kernel.wasm'))
const completed=JSON.parse(readFileSync(join(root,'completed-build-receipt.json'),'utf8'))
assert.deepEqual({sha256:hash(bytes),byteLength:bytes.length},completed.artifacts[kind])
const wasm=(await WebAssembly.instantiate(bytes)).instance.exports as unknown as {
 memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint
}
const result:any={schema:'sweep-multispan-strip-jets/1',passed:false,
 artifact:{kind,...completed.artifacts[kind]},nativeSilenceLimitMs:30000,maxWorkPerCall:1000000,
 scope:'Native whole-strip multispan/projective G1/G2 and independent regularity through the actual WASM ABI. Synthetic declared tensors; no STEP/UI/material/whole-boundary claim.',
 nativeSourceSha256:hash(readFileSync(join(root,'crates/nurbs-core/src/join/continuity/exact_strip.rs'))),
 cases:[],calls:[]}
mkdirSync(dirname(output),{recursive:true})
const save=()=>writeFileSync(output,JSON.stringify(result,null,2)+'\n')
save()
const clone=<T,>(x:T):T=>structuredClone(x)
const nextUp=(x:number)=>{const b=new DataView(new ArrayBuffer(8));b.setFloat64(0,x);b.setBigUint64(0,b.getBigUint64(0)+1n);return b.getFloat64(0)}
function inspect(name:string,a:any,b:any,order:number,maxWork=1000000,projective=true){
 const request={reference:a,edited:b,referenceBoundary:'vMax',editedBoundary:'vMin',order,normalScale:1,maxWork}
 const original=JSON.stringify(request),data=encodeBinary({op:projective?'surface_projective_strip_jets_audit':'surface_exact_strip_jets_audit',...request})
 const ptr=writeLinear(wasm.memory,wasm.abi_alloc,data);assert.ok(ptr)
 let response:any,milliseconds:number
 try{const began=performance.now();response=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,data.length));milliseconds=performance.now()-began}
 finally{wasm.abi_free(ptr,data.length)}
 assert.ok(response.ok,JSON.stringify(response.error));assert.ok(milliseconds!<=30000)
 assert.equal(JSON.stringify(request),original,'Native audit must preserve the authored surface request')
 const proof=response.value;assert.ok(Number.isSafeInteger(proof.work)&&proof.work>=0&&proof.work<=maxWork)
 if(proof.certified){assert.equal(proof.exactIdentity,true);assert.equal(proof.regularityCertified,true);assert.ok(proof.work>0)}
 result.calls.push({name,request,requestSha256:hash(original),proof,milliseconds:milliseconds!});save()
 return proof
}
function positive(name:string,a:any,b:any,projective=true){
 const g1=inspect(name+'-g1',a,b,1,1000000,projective),g2=inspect(name+'-g2',a,b,2,1000000,projective)
 assert.equal(g1.certified,true);assert.equal(g2.certified,true)
 assert.equal(inspect(name+'-zero-work',a,b,2,0,projective).certified,false)
 assert.equal(inspect(name+'-short-work',a,b,2,g2.work-1,projective).certified,false)
 result.cases.push({name,positive:true,g1,g2,internalKnotsU:a.knotsU.filter((k:number)=>k>a.knotsU[a.degreeU]&&k<a.knotsU[a.controlPoints.length])});save()
}
const cubic=(y:number)=>({degreeU:3,degreeV:1,knotsU:[0,0,0,0,.5,1,1,1,1],knotsV:[0,0,1,1],
 controlPoints:[0,.125,.5,.875,1].map(x=>[[x,y,0],[x,y+1,0]]),weights:[1,2,3,2,1].map(w=>[w,w]),periodicU:false,periodicV:false})
{
 const a=cubic(0),b=cubic(1);positive('common-rational-cubic-bspline',a,b,false)
 const changed=clone(b);changed.controlPoints[4][1][1]=nextUp(2)
 assert.equal(inspect('cubic-last-span-one-ulp-cross-jet',a,changed,2,1000000,false).exactIdentity,false)
 const knots=clone(b);knots.knotsU[4]=.25
 assert.equal(inspect('cubic-different-basis',a,knots,2,1000000,false).certified,false)
 const singularA=clone(a),singularB=clone(b)
 for(const s of [singularA,singularB])for(const row of s.controlPoints)for(const p of row)p[0]=0
 const singular=inspect('cubic-singular-identity',singularA,singularB,2,1000000,false)
 assert.equal(singular.exactIdentity,true);assert.equal(singular.regularityCertified,false);assert.equal(singular.certified,false)
}
const quarter=(second:boolean)=>{
 const points=second?[[0,1],[-1,1],[-1,0]]:[[1,0],[1,1],[0,1]]
 return {degreeU:1,degreeV:2,knotsU:[0,0,.5,1,1],knotsV:[0,0,0,1,1,1],
  controlPoints:[0,.5,1].map(z=>points.map(([x,y])=>[x+z,y,z])),weights:[0,1,2].map(()=>[1,Math.SQRT1_2,1]),periodicU:false,periodicV:false}
}
{
 const a=quarter(false),b=quarter(true);positive('rational-quarter-linear-multispan',a,b)
 const shiftedA=clone(a),shiftedB=clone(b)
 shiftedA.knotsU[2]=shiftedB.knotsU[2]=.25;positive('rational-quarter-nonuniform-speed',shiftedA,shiftedB)
 const kinkA=clone(a),kinkB=clone(b)
 for(const s of [kinkA,kinkB])for(const p of s.controlPoints[1])p[2]=nextUp(.5)
 assert.equal(inspect('linear-internal-one-ulp-kink',kinkA,kinkB,2).certified,false)
 const curvature=clone(b);for(const row of curvature.controlPoints)row[2][1]=.125
 assert.equal(inspect('quarter-curvature-changed-g1',a,curvature,1).certified,true)
 assert.equal(inspect('quarter-curvature-changed-g2',a,curvature,2).exactIdentity,false)
 const singularA=clone(a),singularB=clone(b)
 for(const s of [singularA,singularB])s.controlPoints=[0,1,2].map(()=>clone(s.controlPoints[0]))
 const singular=inspect('quarter-singular-identity',singularA,singularB,2)
 assert.equal(singular.exactIdentity,true);assert.equal(singular.regularityCertified,false);assert.equal(singular.certified,false)
}
const chain=(y:number)=>({degreeU:2,degreeV:1,knotsU:[0,0,0,.25,.25,.5,.5,1,1,1],knotsV:[0,0,1,1],
 controlPoints:[0,1.5,3,6.5,10,15.5,21].map(x=>[[x,y,0],[x,y+1,0]]),weights:Array.from({length:7},()=>[1,1]),periodicU:false,periodicV:false})
{
 const a=chain(0),b=chain(1);positive('nonuniform-full-multiplicity-chain',a,b)
 const changedA=clone(a),changedB=clone(b)
 for(const s of [changedA,changedB])for(const p of s.controlPoints[3])p[2]=.125
 assert.equal(inspect('chain-later-span-kink-g1',changedA,changedB,1).certified,false)
 assert.equal(inspect('chain-later-span-kink-g2',changedA,changedB,2).certified,false)
}
assert.equal(result.cases.length,4)
result.runnerSha256=hash(readFileSync(new URL(import.meta.url)))
result.passed=true;save()
console.log(`${kind}: four genuine multispan native G1/G2 families and ${result.calls.length} whole-strip/mutation/budget observations passed`)
