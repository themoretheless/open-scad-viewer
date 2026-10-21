import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {box3d} from '../tests/fixtures/minkowski3d-cases'
import {rationalRoof,triangularRoof,constantHalf,squareParameter,retracedParameter,planarCurve,constantMultispan} from '../tests/fixtures/certified-followup-cases'
const bytes=readFileSync(process.argv[2]),w=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const proofs:any[]=[],refusals:any[]=[],calls:any[]=[]
function call(op:string,args:object):any{const start=performance.now(),request=encodeBinary({op,...args}),p=writeLinear(w.memory,w.abi_alloc,request);assert(p);try{const r=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,p,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}finally{w.abi_free(p,request.length);calls.push({op,milliseconds:performance.now()-start})}}
const base=call('brep_nurbs_box',{min:[0,0,0],max:[2,2,2]}),cylinder=call('brep_nurbs_cylinder',{radius:1,height:2}),tube=call('brep_nurbs_tube',{outerRadius:2,innerRadius:1,height:2}),l=call('brep_nurbs_extrude_polygon',{profile:[[0,0],[4,0],[4,1],[1,1],[1,4],[0,4]],zMin:0,zMax:2})
const cube={kind:'mesh',mesh:box3d([0,0,0],[1,1,1])},small={kind:'mesh',mesh:box3d([0,0,0],[.25,.25,.25])}
for(const[name,model,tolerance,kernel,topology]of[
 ['authored-box',base,1,cube,true],['rational-roof',rationalRoof(base),1,cube,true],['triangular-curved-roof',triangularRoof(base),1,cube,true],['curved-trim-cylinder',cylinder,1,cube,true],['holed-trim-tube',tube,4,small,false],['nonconvex-trim-prism',l,1,cube,true],
]as const){
 const request={a:{kind:'brep',model},b:kernel,maximumSourceDeviationMm:tolerance,meshPrecision:'f32'},r=call('minkowski_nurbs_certify',request),verified=call('minkowski_nurbs_verify',{...request,certificate:r.certificate})
 assert.equal(r.certificate.version,2);assert.equal(r.roundingTopology.status,'proved');assert.deepEqual(verified.mesh,r.mesh);assert.equal(r.approximationTopology!==null,topology);assert(r.sourceSurfaceDeviationMm[0]<=tolerance)
 if(name==='nonconvex-trim-prism'){assert.equal(r.volumeMm3,48);assert.equal(r.maximumSetDeviationMm,0)}
 if(name==='holed-trim-tube'){assert.equal(r.approximationTopology,null);assert(r.volumeMm3<22);}
 const forgeries=[];for(const mutation of ['source-trim','continuous-bound','outer-region','topology']as const){const p=structuredClone({...request,certificate:r.certificate});if(mutation==='source-trim')p.a.model.loops[p.a.model.faces.at(-1)!.outer].coedges[0].pcurve.controlPoints[0][0]+=.125;else if(mutation==='continuous-bound')p.certificate.sourceHomotopyBounds[0]=['999','1'];else if(mutation==='outer-region')p.certificate.outer.triangles.pop();else{if(p.certificate.sourceTopology[0])p.certificate.sourceTopology[0].center[0]=['999','1'];else p.certificate.sourceTopology[0]=p.certificate.sourceTopology[1]};assert.throws(()=>call('minkowski_nurbs_verify',p));forgeries.push({mutation,request:p})}
 proofs.push({name,mode:'verify-minkowski-nurbs',request:{...request,certificate:r.certificate},verified:{status:r.status,scope:r.scope,nominalVolumeMm3:r.volumeMm3,maximumSetDeviationExactMm:r.maximumSetDeviationExactMm,approximationTopology:r.approximationTopology},forgeries});console.error(name,'verified',r.volumeMm3,r.maximumSetDeviationMm,topology)
}
const circleA=planarCurve([[0,0],[-.5,0],[0,0]]),circleB=planarCurve([[-.375,0],[.125,0],[-.375,0]]),cuspA=planarCurve([[-.125,0],[.125,0],[-.125,0],[.125,0]])
for(const[name,first,second,selfPair]of[
 ['irrational-circle-branches',circleA,circleB,false],['cusp-branches',cuspA,squareParameter(),false],['crossing-branches',circleA,circleA,false],['deleted-diagonal',retracedParameter(),retracedParameter(),true],['vertical-irrational-fiber',squareParameter(),constantHalf(),false],['constant-square',constantHalf(),constantHalf(),true],['constant-multispan',constantMultispan(),constantMultispan(),true],
]as const){
 const request={first,second,selfPair,maximumParameterWidth:1e-10},certificate=call('brep_curve_certify_span_intersections',request),verified=call('brep_curve_verify_span_intersections_certificate',{...request,certificate});assert.equal(certificate.version,3);assert(verified.branchDecompositions.length>0)
 const graph=verified.branchDecompositions[0].decomposition
 if(name==='irrational-circle-branches'){assert.equal(graph.cuts.length,4);assert.deepEqual(graph.slabs.map((s:any)=>s.roots.length),[0,2,0]);assert.deepEqual(graph.cuts[1].right,[0,0]);assert.deepEqual(graph.cuts[2].left,[0,0])}
 if(name==='deleted-diagonal'){assert.deepEqual(graph.cuts[1].excludedNodes,[true]);assert.deepEqual(graph.slabs.map((s:any)=>s.diagonalSide),[[-1],[1]])}
 if(name==='constant-square')assert.equal(graph.dimension,2)
 if(name==='vertical-irrational-fiber'){const c=graph.cuts.find((c:any)=>c.vertical);assert(c);assert.deepEqual(c.verticalArcs,[[0,1]])}
 const forgeries=[];for(const mutation of ['pair-census','branch-census','branch-connectivity']as const){const p=structuredClone({...request,certificate});if(mutation==='pair-census')p.certificate.pairs=[];else{const g=p.certificate.pairs.find((p:any)=>p.branches)!.branches;if(mutation==='branch-census')g.dimension=g.dimension===2?1:2;else if(g.cuts.length)g.cuts[0].right.push(999);else g.projection=[['999','1']]};assert.throws(()=>call('brep_curve_verify_span_intersections_certificate',p));forgeries.push({mutation,request:p})}
 proofs.push({name,mode:'verify-span-intersections',request:{...request,certificate},verified,forgeries});console.error(name,'verified')
}
const invalid=structuredClone(cylinder);invalid.loops[invalid.faces[4].outer].coedges[0].pcurve.controlPoints[1][0]=2
const bad={a:{kind:'brep',model:invalid},b:cube,maximumSourceDeviationMm:1,meshPrecision:'f32'};assert.throws(()=>call('minkowski_nurbs_certify',bad));refusals.push({name:'invalid-original-trim',mode:'certify-minkowski-nurbs',request:bad})
const sphere=call('brep_nurbs_sphere',{radius:1}),unsewn={a:{kind:'brep',model:sphere},b:cube,maximumSourceDeviationMm:1,meshPrecision:'f32'};assert.throws(()=>call('minkowski_nurbs_certify',unsewn),/MINKOWSKI_3D_SOURCE_SEAM/);refusals.push({name:'nonexact-spherical-equator',mode:'certify-minkowski-nurbs',request:unsewn})
const thin=box3d([1,0,0],[1+2**-30,1,1]),thinRequest={a:thin,b:thin,meshPrecision:'f32'};assert.throws(()=>call('minkowski_3d_certify',thinRequest));refusals.push({name:'f32-thin-collapse',mode:'certify-minkowski-3d',request:thinRequest})
if(process.argv[4]){const mesh=call('brep_nurbs_tessellate',{model:cylinder,segments:4});writeFileSync(process.argv[4],JSON.stringify({version:1,sketches:[],bodies:[{id:'a',name:'Обрезанная NURBS-поверхность',brep:cylinder,mesh:{positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)}},{id:'b',name:'Ядро',mesh:cube.mesh}],curves:[],surfaces:[]},null,2)+'\n')}
const report={wasmSha256:createHash('sha256').update(bytes).digest('hex'),proofs,refusals,calls};writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({wasmSha256:report.wasmSha256,certificates:proofs.length,forgedRejected:proofs.reduce((n,p)=>n+p.forgeries.length,0),refusals:refusals.length,maxCallMs:Math.max(...calls.map(p=>p.milliseconds))},null,2))
