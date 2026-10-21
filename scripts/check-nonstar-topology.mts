import {readFileSync,writeFileSync} from 'node:fs'
import {join} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {box3d} from '../tests/fixtures/minkowski3d-cases'
const bytes=readFileSync(process.argv[2]),w=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const report:any={wasmSha256:createHash('sha256').update(bytes).digest('hex'),proofs:[],refusals:[],calls:[]}
function call(op:string,args:object):any{const start=performance.now(),request=encodeBinary({op,...args}),p=writeLinear(w.memory,w.abi_alloc,request);assert(p);try{const r=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,p,request.length));assert(r.ok,JSON.stringify(r.error));return r.value}finally{w.abi_free(p,request.length);report.calls.push({op,milliseconds:performance.now()-start})}}
const fixtures=process.argv[3],output=process.argv[4]
writeFileSync(output,JSON.stringify(report,null,2)+"\n")
for(const[name,file,size,kind,full,holes]of[
 ['curved-through-hole','curved-through-hole',[.25,.25],'signed-prismatic-ambient-isotopy',true,1],
 ['curved-hole-filled','curved-through-hole',[3,3],'signed-prismatic-ambient-isotopy',true,0],
 ['curved-hole-lost-in-coarse-sum','curved-through-hole',[1.9,.5],'signed-prismatic-ambient-isotopy',false,null],
 ['nonstar-c','nonstar-c',[.25,.25],'signed-prismatic-ambient-isotopy',true,null],
 ['two-holes-permuted','two-holes-permuted',[.25,.25],'signed-prismatic-ambient-isotopy',true,null],
 ['sealed-curved-cavity','sealed-curved-cavity',[.25,.25],'signed-prismatic-ambient-isotopy',false,null],
 ['sheared-through-hole','sheared-through-hole',[.25,.25],'polynomial-shear-ambient-isotopy',false,null],
 ['sheared-nonstar','sheared-nonstar',[.25,.25],'polynomial-shear-ambient-isotopy',false,null],
 ['sheared-sealed-cavity','sheared-sealed-cavity',[.25,.25],'polynomial-shear-ambient-isotopy',false,null],
] as const){
 const model=JSON.parse(readFileSync(join(fixtures,file+'.json'),'utf8'))
 const request={a:{kind:'brep',model},b:{kind:'mesh',mesh:box3d([0,0,0],[size[0],size[1],.25])},maximumSourceDeviationMm:100,meshPrecision:'f32'}
 if(name==='curved-hole-lost-in-coarse-sum'){
  assert.throws(()=>call('minkowski_nurbs_certify',request),/rounding separation/)
  report.refusals.push({name,mode:'certify-minkowski-nurbs',request,reason:'display rounding embedding unproved; the internal exact source/sum topology counterexample is covered by the native diagnostic'})
  writeFileSync(output,JSON.stringify(report,null,2)+'\n');console.error(name,'safe display-rounding refusal');continue
 }
 const r=call('minkowski_nurbs_certify',request)
 assert.equal(r.certificate.version,3);assert.equal(r.certificate.sourceAtlasTopology[0]?.kind,kind)
 if(kind==='polynomial-shear-ambient-isotopy'){assert.equal(r.certificate.inner.vertices.length,0);assert.equal(r.maximumSetDeviationExactMm,null)}
 assert.deepEqual(r.sourceApproximationTopology,[true,true]);assert.equal(r.approximationTopology!==null,full)
 if(holes!==null)assert.equal(r.approximationTopology.survivingHoles,holes)
 const proof={...request,certificate:r.certificate},verified=call('minkowski_nurbs_verify',proof)
 assert.deepEqual(verified.mesh,r.mesh);assert.deepEqual(verified.sourceApproximationTopology,r.sourceApproximationTopology)
 const forgeries=[]
 for(const mutation of ['source-atlas','source-geometry',...(r.certificate.sumTopology?['sum-topology']:[])] as const){
  const p=structuredClone(proof)
  if(mutation==='source-geometry')p.a.model.faces[0].surface.controlPoints[0][0][0]+=.125
  else if(mutation==='sum-topology')p.certificate.sumTopology.survivingHoles+=1
  else{const a=p.certificate.sourceAtlasTopology[0];if(a.kind==='signed-prismatic-ambient-isotopy')a.shells[0].coefficients[0]*=-1;else a.polynomial[0][0]=['999','1']}
  assert.throws(()=>call('minkowski_nurbs_verify',p));forgeries.push({mutation,request:p})
 }
 report.proofs.push({name,mode:'verify-minkowski-nurbs',request:proof,verified:{scope:r.scope,sourceApproximationTopology:r.sourceApproximationTopology,approximationTopology:r.approximationTopology,nominalVolumeMm3:r.volumeMm3},forgeries})
 writeFileSync(output,JSON.stringify(report,null,2)+'\n');console.error(name,'verified',r.approximationTopology?.kind??'source-only')
}
console.log(JSON.stringify({wasmSha256:report.wasmSha256,certificates:report.proofs.length,forgedRejected:report.proofs.reduce((n:number,p:any)=>n+p.forgeries.length,0),maxCallMs:Math.max(...report.calls.map((p:any)=>p.milliseconds))},null,2))
