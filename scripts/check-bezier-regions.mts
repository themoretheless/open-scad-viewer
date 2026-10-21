import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {bezierLine} from '../tests/fixtures/bezier-algebraic-cases'
import {irrationalQuartic,irrationalContactLine,nondyadicTangency,nondyadicContactLine,irrationalLoop,irrationalCusp,multipleFiberFirst,multipleFiberSecond,multispanFirst,multispanSecond,regionBox,curvedRegion} from '../tests/fixtures/bezier-region-cases'
const bytes=readFileSync(process.argv[2]??'public/wasm/geometry-kernel.wasm')
const wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const calls:{op:string;milliseconds:number}[]=[]
function call(op:string,args:object):any{
 const start=performance.now(),request=encodeBinary({op,...args}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,request);assert(ptr)
 try{const r=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}
 finally{wasm.abi_free(ptr,request.length);calls.push({op,milliseconds:performance.now()-start})}
}
const proofs:any[]=[],models:any[]=[]
for(const [name,first,second,selfPair,count,singular] of [['irrational-quartic',irrationalQuartic,irrationalContactLine,false,2,0],['third-parameter-tangency',nondyadicTangency,nondyadicContactLine,false,1,0],['separated-projection',multipleFiberFirst,multipleFiberSecond,false,2,0],['irrational-self-loop',irrationalLoop,irrationalLoop,true,1,0],['irrational-stationary',irrationalCusp,irrationalCusp,true,undefined,2]] as const){
 const request={first,second,selfPair,maximumParameterWidth:1e-12},certificate=call('brep_curve_certify_algebraic_intersections',request)
 const verified=call('brep_curve_verify_algebraic_intersections_certificate',{...request,certificate})
 if(count!==undefined)assert.equal(verified.events.length,count)
 assert.equal(verified.singularParameters.length,singular)
 const bad=structuredClone(certificate);bad.factors.find((f:any)=>f.roots.length)!.roots.pop();assert.throws(()=>call('brep_curve_verify_algebraic_intersections_certificate',{...request,certificate:bad}))
 proofs.push({name,mode:'verify-algebraic-intersections',...request,certificate,verified,forged:bad})
}
const request={first:multispanFirst,second:multispanSecond,firstDomain:[0,1],secondDomain:[0,1]}
const certificate=call('brep_curve_certify_multispan_agreement',request),verified=call('brep_curve_verify_multispan_agreement_certificate',{...request,certificate})
assert.equal(verified.commonSpans,2)
const forged=structuredClone(certificate);forged.steps.pop();assert.throws(()=>call('brep_curve_verify_multispan_agreement_certificate',{...request,certificate:forged}))
proofs.push({name:'multispan-irrational-knot-preimage',mode:'verify-multispan-agreement',...request,certificate,verified,forged})
function mesh(loops:any,name:string){
 const profile=call('brep_profile_validate',{loops}),brep=call('brep_nurbs_extrude_curves',{loops,zMin:0,zMax:2}),mesh=call('brep_nurbs_tessellate',{model:brep,segments:32})
 assert(mesh.report.closed);assert.equal(mesh.report.degenerateTriangles,0);assert.equal(mesh.report.nonManifoldEdges,0);assert.equal(mesh.report.orientationConflicts,0)
 assert(Math.abs(mesh.report.signedVolumeMm3-2*profile.areaMm2)/(2*profile.areaMm2)<.003)
 models.push({name,areaMm2:profile.areaMm2,mesh:mesh.report})
 return {profile,brep,mesh}
}
const normalized=call('brep_profile_normalize',{loops:[[irrationalLoop,bezierLine([3,1.5],[3,-1.5])]]})
assert(normalized.loops.length>=2);normalized.loops.forEach((l:any,i:number)=>mesh([l],'irrational-loop-'+i))
for(const [name,loops,distanceMm] of [['merged-overlapping-offsets',[regionBox(),regionBox(5)],1],['inward-artifact-cleanup',[regionBox()],-1],['curved-source-offset',[curvedRegion],.5]] as const){
 const request={loops,distanceMm,maximumDeviationMm:.05,joinStyle:'round',miterLimitMm:4}
 const r=call('brep_profile_offset_regions_certified',request),certificate=r.certificate,verified=call('brep_profile_verify_offset_regions_certificate',{...request,certificate})
 assert.equal(r.components.length,1);if(distanceMm===-1)assert(Math.abs(r.components[0].areaMm2-4)<1e-10)
 const forged=structuredClone(certificate);forged.exactRings[0].pop();assert.throws(()=>call('brep_profile_verify_offset_regions_certificate',{...request,certificate:forged}))
 proofs.push({name,mode:'verify-offset-regions',...request,certificate,verified,forged})
 for(const [i,p] of r.components.entries()){
  const output=mesh(p.loops,name+'-'+i)
  if(name==='merged-overlapping-offsets'&&process.argv[4]){
   const display=call('brep_profile_display',{loops:p.loops})
   writeFileSync(process.argv[4],JSON.stringify({version:1,sketches:display.map((points:number[][],i:number)=>({id:'region-offset-'+i,name:'Объединённый offset',points,closed:true,retainedProfile:p})),bodies:[{id:'region-body',name:'Объединённые offset-области',brep:output.brep,mesh:{positions:Array.from(output.mesh.positions),indices:Array.from(output.mesh.indices)},material:{name:'Offset region',color:'#36b4ae'}}],curves:[],surfaces:[]},null,2)+'\n')
  }
 }
}
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),proofs,models,calls}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,proofs:proofs.map(p=>({name:p.name,...p.verified})),models,calls},null,2))
