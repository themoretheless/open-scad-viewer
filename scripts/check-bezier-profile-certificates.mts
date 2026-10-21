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
 try {
  const r=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,request.length))
  if(!r.ok)throw new Error(JSON.stringify(r.error))
  return r.value
 }finally{wasm.abi_free(ptr,request.length)}
}
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const cubic=(controlPoints:number[][],weights=[1,1,1,1]):NurbsCurve=>({degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints,weights,periodic:false})
const reverse=(c:NurbsCurve):NurbsCurve=>({...c,controlPoints:[...c.controlPoints].reverse(),weights:[...c.weights].reverse()})
const fixtures:{name:string;loops:NurbsCurve[][];expected:number}[]=[]
for(const transformed of [false,true]){
 const a=cubic([[0,0],[2,0],[4,2],[6,6]]),b=cubic([[0,0],[2,0],[6,2],[12,12]])
 const wire=[b,line([12,12],[6,6]),reverse(a)]
 if(transformed)for(const c of wire)for(const p of c.controlPoints){const[x,y]=p;p[0]=17+x-y;p[1]=-9+x+y}
 fixtures.push({name:transformed?'affine-cusp':'same-side-cusp',loops:[wire],expected:transformed?25.2:12.6})
}
const c=cubic([[0,0],[2.5,0],[6,1],[10,5]],[3,4,5,6])
fixtures.push({name:'nonprimitive-rational-cubic',loops:[[line([0,0],[10,0]),line([10,0],[10,5]),reverse(c)]],expected:50/3})
const circle=call('brep_profile_author',{kind:'circle',radius:2})
const ellipse=call('brep_profile_transform',{loops:circle.loops,matrix:[3,0,0,0,1,2,0,0,0,0,1,0,0,0,0,1],toleranceMm:1e-7})
fixtures.push({name:'retained-rational-ellipse',loops:ellipse.loops,expected:24*Math.PI})
const records=[]
for(const f of fixtures){
 const start=performance.now(),before=JSON.stringify(f.loops)
 const profile=call('brep_profile_validate',{loops:f.loops,toleranceMm:1e-7})
 assert(Math.abs(profile.areaMm2-f.expected)<1e-5)
 const certificate=call('brep_profile_certify',{loops:f.loops,toleranceMm:1e-7})
 const verified=call('brep_profile_verify_certificate',{loops:f.loops,toleranceMm:1e-7,certificate})
 assert.equal(verified.status,'verified');assert.equal(verified.scope,'represented-profile-topology')
 for(const bad of [{...certificate,source:certificate.source+'0'},{...certificate,version:2},
  {...certificate,separatingNormals:[]},{...certificate,nestingDepth:[1]},{...certificate,solidValid:true}])
  assert.throws(()=>call('brep_profile_verify_certificate',{loops:f.loops,toleranceMm:1e-7,certificate:bad}))
 const solid=call('brep_nurbs_extrude_curves',{loops:f.loops,zMin:0,zMax:3})
 assert.equal(call('brep_nurbs_inspect',{model:solid}).topologyValid,true)
 const mesh=call('brep_nurbs_tessellate',{model:solid,segments:32})
 assert(mesh.report.closed);assert.equal(mesh.report.degenerateTriangles,0)
 assert.equal(mesh.report.nonManifoldEdges,0);assert.equal(mesh.report.orientationConflicts,0)
 assert(Math.abs(mesh.report.signedVolumeMm3-3*f.expected)/(3*f.expected)<0.003)
 assert.equal(JSON.stringify(f.loops),before)
 if(process.argv[4]&&f.name==='retained-rational-ellipse'){
  const display=call('brep_profile_display',{loops:f.loops})
  writeFileSync(process.argv[4],JSON.stringify({version:1,
   sketches:[{id:'ellipse-profile',name:'Рациональный эллипс',points:display[0],closed:true,retainedProfile:profile}],
   bodies:[{id:'ellipse-body',name:'Рациональный эллипс',brep:solid,
    mesh:{positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)},
    material:{name:'Ellipse',color:'#36b4ae'}}],curves:[],surfaces:[]},null,2)+'\n')
 }
 records.push({name:f.name,loops:f.loops,toleranceMm:1e-7,certificate,verified,
  areaMm2:profile.areaMm2,mesh:mesh.report,elapsedMs:performance.now()-start})
}
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),records}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,fixtures:records.map(r=>({name:r.name,status:r.verified.status,elapsedMs:r.elapsedMs}))},null,2))
