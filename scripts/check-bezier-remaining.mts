import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {nonlinearFirst,nonlinearSecond,nonlinearWire,fourthOrderContact,contactLine,degreeEightLoop,nonprimitiveLoop,quarticContactFirst,quarticContactSecond,exactCusp,squareWire,bezierLine} from '../tests/fixtures/bezier-algebraic-cases'
const bytes=readFileSync(process.argv[2]??'public/wasm/geometry-kernel.wasm')
const wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
function call(op:string,args:object):any{
 const request=encodeBinary({op,...args}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,request);assert(ptr)
 try{const r=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}
 finally{wasm.abi_free(ptr,request.length)}
}
const request={first:nonlinearFirst,second:nonlinearSecond,firstDomain:[0,1],secondDomain:[0,1]}
const c=call('brep_curve_certify_nonlinear_agreement',request),nonlinear={...request,certificate:c,verified:call('brep_curve_verify_nonlinear_agreement_certificate',{...request,certificate:c})}
const bad=structuredClone(c);bad.relation[0][0]=['1','1'];assert.throws(()=>call('brep_curve_verify_nonlinear_agreement_certificate',{...request,certificate:bad}))
const a=[nonlinearWire(nonlinearFirst)],b=[nonlinearWire(nonlinearSecond)]
assert.deepEqual(call('brep_profile_boolean',{a,b,operation:'difference'}).loops,[])
assert.deepEqual(call('brep_profile_normalize',{loops:[...a,...b]}).loops,[])
const intersections=[]
for(const [name,first,second,selfPair] of [['fourth-order',fourthOrderContact,contactLine,false],['degree-eight-loop',degreeEightLoop,degreeEightLoop,true],['nonprimitive-loop',nonprimitiveLoop,nonprimitiveLoop,true],['cusp',exactCusp,exactCusp,true]] as const){
 const request={first,second,selfPair},certificate=call('brep_curve_certify_intersections',request),verified=call('brep_curve_verify_intersections_certificate',{...request,certificate})
 if(name==='fourth-order')assert.deepEqual(verified.events,[[.5,.5]])
 if(name==='degree-eight-loop'||name==='nonprimitive-loop')assert.deepEqual(verified.events,[[.25,.75]])
 if(name==='cusp')assert.deepEqual(verified.singularParameters,[.5])
 const bad={...certificate,events:[],singularParameters:[]};assert.throws(()=>call('brep_curve_verify_intersections_certificate',{...request,certificate:bad}))
 intersections.push({name,...request,certificate,verified})
}
const contactRequest={first:quarticContactFirst,second:quarticContactSecond}
const contactCertificate=call('brep_curve_certify_endpoint_contacts',contactRequest)
assert.equal(contactCertificate.separator,65534)
const endpointContact={...contactRequest,certificate:contactCertificate,verified:call('brep_curve_verify_endpoint_contacts_certificate',{...contactRequest,certificate:contactCertificate})}
const offsets=[],models=[]
function model(loops:any,name:string){
 const profile=call('brep_profile_validate',{loops}),brep=call('brep_nurbs_extrude_curves',{loops,zMin:0,zMax:3}),mesh=call('brep_nurbs_tessellate',{model:brep,segments:32})
 assert(mesh.report.closed);assert.equal(mesh.report.degenerateTriangles,0);assert.equal(mesh.report.nonManifoldEdges,0);assert.equal(mesh.report.orientationConflicts,0)
 assert(Math.abs(mesh.report.signedVolumeMm3-3*profile.areaMm2)/(3*profile.areaMm2)<.003)
 models.push({name,areaMm2:profile.areaMm2,mesh:mesh.report})
 return {profile,brep,mesh}
}
for(const joinStyle of ['bevel','miter','round']){
 const request={curves:squareWire,distanceMm:-1,maximumDeviationMm:.01,closed:true,joinStyle,miterLimitMm:3},r=call('brep_profile_offset_wire_certified',request)
 const verified=call('brep_profile_verify_offset_wire_certificate',{...request,certificate:r.certificate})
 const bad=structuredClone(r.certificate);bad.joins[0][0][0]+=1;assert.throws(()=>call('brep_profile_verify_offset_wire_certificate',{...request,certificate:bad}))
 offsets.push({...request,certificate:r.certificate,verified})
 const output=model([r.curves],joinStyle+'-offset')
 if(joinStyle==='round'&&process.argv[4]){
  const display=call('brep_profile_display',{loops:output.profile.loops})
  writeFileSync(process.argv[4],JSON.stringify({version:1,sketches:display.map((points:number[][],i:number)=>({id:'joined-offset-'+i,name:'Offset со скруглёнными стыками',points,closed:true,retainedProfile:output.profile})),bodies:[{id:'joined-offset-body',name:'Offset · round · 0,01 мм',brep:output.brep,mesh:{positions:Array.from(output.mesh.positions),indices:Array.from(output.mesh.indices)},material:{name:'Joined offset',color:'#36b4ae'}}],curves:[],surfaces:[]},null,2)+'\n')
 }
}
const normalized=call('brep_profile_normalize',{loops:[[degreeEightLoop,bezierLine([0,84],[0,0])]]})
assert(normalized.loops.length>=2)
normalized.loops.forEach((loop:any,i:number)=>model([loop],'loop-component-'+i))
const zero=call('brep_profile_offset_wire_certified',{curves:[bezierLine([2,3],[2,3])],distanceMm:0,maximumDeviationMm:.001,closed:false,joinStyle:'bevel',miterLimitMm:1})
assert.deepEqual(zero.curves[0].controlPoints,[[2,3],[2,3]])
const report={zeroOffsetPointPreserved:true,endpointContact,wasmSha256:createHash('sha256').update(bytes).digest('hex'),nonlinear,intersections,offsets,models}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,nonlinear:nonlinear.verified,intersections:intersections.map(r=>({name:r.name,...r.verified})),offsets:offsets.map(r=>({style:r.joinStyle,...r.verified})),models},null,2))
