import {multispanFirst,multispanSecond} from '../tests/fixtures/bezier-region-cases'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {minkowskiL,minkowskiKernel,minkowskiFrame,minkowskiSquare,minkowskiNeck,zeroJetRegion,retracedLine,monotoneStationary,spanFirst,spanSecond,rationalKnotFirst,rationalKnotSecond,polygonWire,retracingImageFirst,retracingImageSecond,cuspImageFirst,cuspImageSecond} from '../tests/fixtures/minkowski-cases'
const bytes=readFileSync(process.argv[2]??'public/wasm/geometry-kernel.wasm'),wasm=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const calls:any[]=[],proofs:any[]=[],models:any[]=[]
function call(op:string,args:object):any{
 const t=performance.now(),request=encodeBinary({op,...args}),p=writeLinear(wasm.memory,wasm.abi_alloc,request);assert(p)
 try{const r=decodePacked<any>(wasm.memory,wasm.abi_free,wasm.abi_request(0,p,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}finally{wasm.abi_free(p,request.length);calls.push({op,milliseconds:performance.now()-t})}
}
function mesh(p:any,name:string){
 const model=call('brep_nurbs_extrude_curves',{loops:p.loops,zMin:0,zMax:2}),mesh=call('brep_nurbs_tessellate',{model,segments:32})
 assert(mesh.report.closed);assert.equal(mesh.report.nonManifoldEdges,0);assert.equal(mesh.report.orientationConflicts,0);assert.equal(mesh.report.degenerateTriangles,0);assert(Math.abs(mesh.report.signedVolumeMm3-2*p.areaMm2)/(2*p.areaMm2)<.003)
 models.push({name,areaMm2:p.areaMm2,mesh:mesh.report});return {model,mesh}
}
function minkowski(name:string,a:any,b:any,distanceMm:number|undefined,eps:number,area?:number){
 const r=distanceMm===undefined?call('brep_profile_minkowski_sum',{a,b,maximumSourceDeviationMm:eps}):call('brep_profile_minkowski_offset',{loops:a,distanceMm,maximumSourceDeviationMm:eps})
 const request={a,b},certificate=r.certificate,verified=call('brep_profile_verify_minkowski_certificate',{...request,certificate})
 assert.equal(r.modelingStatus,'validated',r.modelingRefusal)
 if(area!==undefined)assert(Math.abs(r.components.reduce((s:number,p:any)=>s+p.areaMm2,0)-area)<1e-8)
 const forged=structuredClone(certificate)
 if(name==='collapse'){forged.diskInner[0][0][0]=['100','1']}else if(name==='zero-jet-sum'){forged.firstPartition[0].pop()}else if(forged.outer.length){forged.outer[0].pop()}else{forged.outer=[[[['0','1'],['0','1']],[['1','1'],['0','1']],[['0','1'],['1','1']]]]}
 assert.throws(()=>call('brep_profile_verify_minkowski_certificate',{...request,certificate:forged}))
 proofs.push({name,mode:'verify-minkowski',...request,certificate,verified,forged})
 console.error(name,'checked',r.exactSetResult?'exact set':'inclusion bounds')
 return r
}
for(const [name,a,area] of [['nonconvex-sum',[minkowskiL],27],['frame-sum',minkowskiFrame,140]] as const){
 const r=minkowski(name,a,[minkowskiKernel],undefined,0,area);assert(r.exactSetResult)
 for(const [i,p]of r.components.entries()){
  const out=mesh(p,name+'-'+i)
  if(name==='frame-sum'&&process.argv[4]){const display=call('brep_profile_display',{loops:p.loops});writeFileSync(process.argv[4],JSON.stringify({version:1,sketches:display.map((points:number[][],i:number)=>({id:'minkowski-'+i,name:i?'Отверстие после суммы':'Сумма Минковского',points,closed:true,retainedProfile:p})),bodies:[{id:'minkowski-body',name:'Минковский · область с отверстием',brep:out.model,mesh:{positions:Array.from(out.mesh.positions),indices:Array.from(out.mesh.indices)},material:{name:'Minkowski',color:'#9bbf62'}}],curves:[],surfaces:[]},null,2)+'\n')}
 }
}
const collapse=minkowski('collapse',[minkowskiSquare],[],-3,.1,0);assert(collapse.exactSetResult);assert.deepEqual(collapse.outerExactRings,[])
const neck=minkowski('neck-split',[minkowskiNeck],[],-.4,.1);assert.equal(neck.components.length,2);neck.components.forEach((p:any,i:number)=>mesh(p,'neck-'+i))
minkowski('zero-jet-sum',[zeroJetRegion],[minkowskiKernel],undefined,.2)
const curved=minkowski('zero-jet-offset',[zeroJetRegion],[],.5,.2);curved.components.forEach((p:any,i:number)=>mesh(p,'zero-jet-'+i))
const ui=call('brep_profile_offset',{loops:[zeroJetRegion],distance:.5,maximumSourceDeviationMm:.2});assert.equal(ui.regionStatus,'inclusion_verified');assert.equal(ui.geometryStatus,'bounded_approximation')
const retraced=minkowski('retraced-empty',[[retracedLine]],[minkowskiKernel],undefined,0,0);assert(retraced.exactSetResult)
const bow=polygonWire([[0,0],[4,4],[0,4],[4,0]]);minkowski('self-crossing-fill',[bow],[minkowskiSquare],undefined,0,44)
{
 const request={first:monotoneStationary,second:monotoneStationary,selfPair:true,maximumParameterWidth:1e-12},certificate=call('brep_curve_certify_algebraic_intersections',request),verified=call('brep_curve_verify_algebraic_intersections_certificate',{...request,certificate});assert(verified.stationaryCensusComplete);assert.equal(verified.singularParameters.length,1)
 const forged={...certificate,factors:[]};assert.throws(()=>call('brep_curve_verify_algebraic_intersections_certificate',{...request,certificate:forged}));proofs.push({name:'stationary-third',mode:'verify-algebraic-intersections',...request,certificate,verified,forged})
}
for(const[name,first,second]of [['multi-span-common',spanFirst,spanSecond],['multi-span-contacts',rationalKnotFirst,rationalKnotSecond]] as const){
 const request={first,second,selfPair:false,maximumParameterWidth:1e-12},certificate=call('brep_curve_certify_span_intersections',request),verified=call('brep_curve_verify_span_intersections_certificate',{...request,certificate});if(name==='multi-span-common')assert(verified.overlaps.length>=3);else assert.equal(verified.events.length,2)
 const forged=structuredClone(certificate);forged.pairs.pop();assert.throws(()=>call('brep_curve_verify_span_intersections_certificate',{...request,certificate:forged}));proofs.push({name,mode:'verify-span-intersections',...request,certificate,verified,forged})
}
{
 const request={first:retracedLine,second:retracedLine,firstDomain:[0,1],secondDomain:[0,1]},certificate=call('brep_curve_certify_multispan_agreement',request),verified=call('brep_curve_verify_multispan_agreement_certificate',{...request,certificate});assert.equal(certificate.version,2)
 const forged={...certificate,source:certificate.source+'x'};assert.throws(()=>call('brep_curve_verify_multispan_agreement_certificate',{...request,certificate:forged}));proofs.push({name:'retracing-affine-identity',mode:'verify-multispan-agreement',...request,certificate,verified,forged})
}

for(const [name,first,second]of [['nonmonotone-nonlinear-images',retracingImageFirst,retracingImageSecond],['irrational-cusp-preimage',cuspImageFirst,cuspImageSecond],['target-cell-quantifier-union',multispanFirst,multispanSecond]] as const){
 const request={first,second,firstDomain:[0,1],secondDomain:[0,1]},certificate=call('brep_curve_certify_image_agreement',request),verified=call('brep_curve_verify_image_agreement_certificate',{...request,certificate})
 const forged=structuredClone(certificate);forged.forward[0].sections.pop();assert.throws(()=>call('brep_curve_verify_image_agreement_certificate',{...request,certificate:forged}))
 proofs.push({name,mode:'verify-image-agreement',...request,certificate,verified,forged})
}
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),proofs,models,calls}
if(process.argv[3])writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({wasmSha256:report.wasmSha256,proofs:proofs.map(p=>({name:p.name,status:p.verified.status,exactSetResult:p.verified.exactSetResult})),models,calls},null,2))
