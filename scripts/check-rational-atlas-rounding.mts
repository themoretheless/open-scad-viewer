import {readFileSync,writeFileSync} from 'node:fs'
import {join} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {box3d} from '../tests/fixtures/minkowski3d-cases'
const [wasmPath,fixtures,output]=process.argv.slice(2)
const bytes=readFileSync(wasmPath),w=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const report:any={wasmSha256:createHash('sha256').update(bytes).digest('hex'),node:process.version,sourceProofs:[],sumProofs:[],calls:[]}
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
function call(op:string,args:object):any {
 const start=performance.now(),request=encodeBinary({op,...args}),p=writeLinear(w.memory,w.abi_alloc,request);assert(p)
 try{const r=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,p,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}
 finally{w.abi_free(p,request.length);report.calls.push({op,milliseconds:performance.now()-start})}
}
// Canonicalize JSON zeros before either checker receives the source. This is
// part of the test input, not an alteration of a source after certification.
const load=(name:string)=>JSON.parse(JSON.stringify(JSON.parse(readFileSync(join(fixtures,name+'.json'),'utf8'))))
save()
for(const name of ['rational-freeform-box','rational-freeform-torus']) {
 const model=load(name);let result:any,depth=-1;const coarseRefusals=[]
 for(let d=0;d<=2;d++) {
  try{result=call('nurbs_source_atlas_certify',{model,depth:d});depth=d;break}
  catch(e){coarseRefusals.push({depth:d,error:String(e)})}
 }
 assert(result,JSON.stringify(coarseRefusals));assert.equal(result.status,'verified')
 const certificate=result.certificate
 assert.equal(certificate.report.kind,'rational-atlas-ambient-isotopy')
 assert.deepEqual(call('nurbs_source_atlas_verify',{model,certificate}),result)
 const forgeries=[]
 for(const mutation of ['source','coordinate','connectivity','report','hash','schema']) {
  const request=structuredClone({model,certificate})
  if(mutation==='source')request.model.faces[0].surface.controlPoints[1][1][2]+=.03125
  else if(mutation==='coordinate')request.certificate.positions[0][0][0]=(BigInt(request.certificate.positions[0][0][0])+1n).toString()
  else if(mutation==='connectivity')request.certificate.triangles[0][0]=request.certificate.triangles[0][1]
  else if(mutation==='report')request.certificate.report.vertexStars++
  else if(mutation==='hash')request.certificate.source='0'.repeat(64)
  else request.certificate.uncheckedClaim=true
  assert.throws(()=>call('nurbs_source_atlas_verify',request));forgeries.push({mutation,request})
 }
 const e=new Set<string>();for(const t of certificate.triangles)for(let k=0;k<3;k++)e.add([t[k],t[(k+1)%3]].sort((a,b)=>a-b).join(','))
 const euler=certificate.positions.length-e.size+certificate.triangles.length
 assert.equal(euler,name.includes('torus')?0:2)
 report.sourceProofs.push({name,depth,euler,mode:'verify-nurbs-source-atlas',request:{model,certificate},coarseRefusals,forgeries})
 save();console.error(name,certificate.report)
}
for(const [name,model,size,expectedAtlas] of [
 ['critical-coplanar-f32',load('curved-through-hole'),[1.9,.5],'signed-prismatic-ambient-isotopy'],
 ['rational-freeform-source-sum',load('rational-single-bump-box'),[.25,.25],'rational-atlas-ambient-isotopy'],
] as const) {
 const request={a:{kind:'brep',model},b:{kind:'mesh',mesh:box3d([0,0,0],[size[0],size[1],.25])},maximumSourceDeviationMm:100,meshPrecision:'f32'}
 const r=call('minkowski_nurbs_certify',request)
 assert.equal(r.certificate.sourceAtlasTopology[0]?.kind,expectedAtlas)
 assert.equal(r.certificate.version,name==='critical-coplanar-f32'?3:4)
 assert.deepEqual(r.sourceApproximationTopology,[true,true])
 assert.equal(r.roundingTopology.wholeLinearHomotopy,true)
 if(name==='critical-coplanar-f32'){assert.equal(r.volumeMm3,41.7375);assert.equal(r.approximationTopology,null);assert.equal(r.roundingTopology.identity,false)}
 const proof={...request,certificate:r.certificate}
 const verified=call('minkowski_nurbs_verify',proof);assert.deepEqual(verified.mesh,r.mesh)
 report.sumProofs.push({name,mode:'verify-minkowski-nurbs',request:proof,sourceApproximationTopology:r.sourceApproximationTopology,approximationTopology:r.approximationTopology,volumeMm3:r.volumeMm3,roundingTopology:r.roundingTopology})
 save();console.error(name,'verified')
}
save();console.log(JSON.stringify({wasmSha256:report.wasmSha256,sourceProofs:report.sourceProofs.length,forgedRejected:report.sourceProofs.reduce((n:number,p:any)=>n+p.forgeries.length,0),sumProofs:report.sumProofs.length,maxCallMs:Math.max(...report.calls.map((p:any)=>p.milliseconds))},null,2))
