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
 try{const result=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,request.length));assert(result.ok,JSON.stringify(result.error));return result.value}
 finally{wasm.abi_free(ptr,request.length)}
}
const first:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[5,0],[10,5]],weights:[1,1,1],periodic:false}
const second={...first,weights:[1,.5,.25]},projectiveRequest={first,second,firstDomain:[0,1],secondDomain:[0,1],parameterScale:2}
const projectiveCertificate=call('brep_curve_certify_projective_agreement',projectiveRequest)
const projectiveVerified=call('brep_curve_verify_projective_agreement_certificate',{...projectiveRequest,certificate:projectiveCertificate})
assert.equal(projectiveVerified.status,'verified')
assert.throws(()=>call('brep_curve_verify_projective_agreement_certificate',{...projectiveRequest,parameterScale:3,certificate:projectiveCertificate}))
const quadratic=(y:number):NurbsCurve=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,y],[2,0]],weights:[1,1,1],periodic:false})
const contactRequest={first:quadratic(-1),second:quadratic(1)}
const contactCertificate=call('brep_curve_certify_endpoint_contacts',contactRequest)
const contactVerified=call('brep_curve_verify_endpoint_contacts_certificate',{...contactRequest,certificate:contactCertificate})
assert.equal(contactVerified.commonEndpoints,2)
assert.throws(()=>call('brep_curve_verify_endpoint_contacts_certificate',{...contactRequest,certificate:{...contactCertificate,commonEndpoints:0}}))
const curve:NurbsCurve={degree:4,knots:[0,0,0,0,0,1,1,1,1,1],controlPoints:[[0,0],[2,0],[4,1],[6,3],[8,4]],weights:[1,2,3,2,1],periodic:false}
const offsetRequest={curve,distanceMm:.5,maximumDeviationMm:.05},before=JSON.stringify(curve)
const offset=call('brep_curve_offset_certified',offsetRequest)
const offsetVerified=call('brep_curve_verify_offset_certificate',{...offsetRequest,certificate:offset.certificate})
assert.equal(offsetVerified.segments,offset.curves.length);assert.equal(JSON.stringify(curve),before)
const bad=structuredClone(offset.certificate);bad.points[1][0]+=1
assert.throws(()=>call('brep_curve_verify_offset_certificate',{...offsetRequest,certificate:bad}))
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const models=[]
for(const degree of [4,8,2]){
 let loops:NurbsCurve[][]
 if(degree===2){const top=quadratic(1);top.controlPoints.reverse();loops=[[quadratic(-1),top]]}
 else {
  const floor:NurbsCurve={degree,knots:[...Array(degree+1).fill(0),...Array(degree+1).fill(1)],controlPoints:Array.from({length:degree+1},(_,i)=>[8*i/degree,-Math.min(i,degree-i)]),weights:Array(degree+1).fill(1),periodic:false}
  const a=[[floor,line([8,0],[8,8]),line([8,8],[0,8]),line([0,8],[0,0])]]
  const points=[[4,-4],[12,-4],[12,4],[4,4]],b=[points.map((p,i)=>line(p,points[(i+1)%4]))]
  loops=call('brep_profile_boolean',{a,b,operation:'intersection'}).loops
  assert(loops.flat().some(c=>c.degree===degree))
 }
 const profile=call('brep_profile_validate',{loops}),model=call('brep_nurbs_extrude_curves',{loops,zMin:0,zMax:3})
 const mesh=call('brep_nurbs_tessellate',{model,segments:32})
 assert(mesh.report.closed);assert.equal(mesh.report.degenerateTriangles,0);assert.equal(mesh.report.nonManifoldEdges,0);assert.equal(mesh.report.orientationConflicts,0)
 assert(Math.abs(mesh.report.signedVolumeMm3-3*profile.areaMm2)/(3*profile.areaMm2)<.003)
 models.push({degree,loops,areaMm2:profile.areaMm2,mesh:mesh.report})
 if(degree===8&&process.argv[4]){
  const display=call('brep_profile_display',{loops})
  writeFileSync(process.argv[4],JSON.stringify({version:1,sketches:display.map((points:number[][],i:number)=>({id:'degree-eight-'+i,name:'Профиль степени 8',points,closed:true,retainedProfile:profile})),bodies:[{id:'degree-eight-body',name:'Bézier степени 8',brep:model,mesh:{positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)},material:{name:'Degree eight',color:'#36b4ae'}}],curves:[],surfaces:[]},null,2)+'\n')
 }
}
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),projective:{...projectiveRequest,certificate:projectiveCertificate,verified:projectiveVerified},contacts:{...contactRequest,certificate:contactCertificate,verified:contactVerified},offset:{...offsetRequest,certificate:offset.certificate,verified:offsetVerified},models}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,projective:projectiveVerified,contacts:contactVerified,offset:offsetVerified,models:models.map(m=>({degree:m.degree,areaMm2:m.areaMm2,closed:m.mesh.closed}))},null,2))
