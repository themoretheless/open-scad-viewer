import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import type {NurbsCurve} from '../src/services/nurbsCurve'
const bytes=readFileSync(process.argv[2]??'public/wasm/geometry-kernel.wasm')
const wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
function call(op:string,args:object):any {
 const request=encodeBinary({op,...args}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,request)
 assert(ptr)
 try {const r=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}
 finally {wasm.abi_free(ptr,request.length)}
}
const periodic:NurbsCurve={degree:3,knots:Array.from({length:11},(_,i)=>i),controlPoints:[[0,0],[8,0],[8,8],[0,8],[0,0],[8,0],[8,8]],weights:Array(7).fill(1),periodic:true}
const vertices=[[0,0],[8,8],[0,16],[-8,8]]
const elevated=vertices.map((a,i):NurbsCurve=>{const b=vertices[(i+1)%4];return {
 degree:8,knots:[...Array(9).fill(0),...Array(9).fill(1)],
 controlPoints:Array.from({length:9},(_,j)=>[a[0]+(b[0]-a[0])*j/8,a[1]+(b[1]-a[1])*j/8]),weights:Array(9).fill(1),periodic:false,
}})
const circle=call('brep_profile_author',{kind:'circle',radius:2})
const ellipse=call('brep_profile_transform',{loops:circle.loops,matrix:[3,0,0,0,1,2,0,0,0,0,1,0,0,0,0,1]})
const records=[]
for(const [name,loops] of [['periodic-cubic',[[periodic]]],['degree-eight',[elevated]],['rational-ellipse',ellipse.loops]] as [string,NurbsCurve[][]][]){
 const start=performance.now(),before=JSON.stringify(loops),toleranceMm=1e-7,maximumAreaWidthMm2=1e-6
 const certificate=call('brep_profile_certify_measure',{loops,toleranceMm,maximumAreaWidthMm2})
 const verified=call('brep_profile_verify_measure_certificate',{loops,toleranceMm,certificate})
 assert.equal(verified.scope,'represented-profile-topology-and-area')
 const ratio=(s:string[])=>s.map(BigInt),[ln,ld]=ratio(certificate.areaMm2.lower),[un,ud]=ratio(certificate.areaMm2.upper)
 assert((un*ld-ln*ud)*1000000n<=ld*ud)
 for(const bad of [{...certificate,solidValid:true},{...certificate,areaDepth:255},
  {...certificate,areaMm2:{lower:['1000000','1'],upper:['1000001','1']}},
  {...certificate,topology:{...certificate.topology,source:certificate.topology.source+'0'}}])
  assert.throws(()=>call('brep_profile_verify_measure_certificate',{loops,toleranceMm,certificate:bad}))
 assert.equal(JSON.stringify(loops),before)
 records.push({name,loops,toleranceMm,maximumAreaWidthMm2,certificate,verified,elapsedMs:performance.now()-start})
}
const first:NurbsCurve={degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:[[0,0],[2.5,0],[6,1],[10,5]],weights:[3,4,5,6],periodic:false}
const second:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[5,0],[10,5]],weights:[1,1,1],periodic:false}
const request={first,second,firstDomain:[.25,.75],secondDomain:[.25,.75]}
const certificate=call('brep_curve_certify_agreement',request)
const verified=call('brep_curve_verify_agreement_certificate',{...request,certificate})
assert.equal(verified.status,'verified')
assert.throws(()=>call('brep_curve_verify_agreement_certificate',{...request,firstDomain:[0,1],certificate}))
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),records,agreement:{...request,certificate,verified}}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,profiles:records.map(r=>({name:r.name,status:r.verified.status,elapsedMs:r.elapsedMs})),agreement:verified},null,2))
