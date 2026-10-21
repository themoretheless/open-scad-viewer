import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
const [wasmPath,sourcePath,bindingPath,legacyPath,output]=process.argv.slice(2)
const input=JSON.parse(readFileSync(sourcePath,'utf8')),bindings=JSON.parse(readFileSync(bindingPath,'utf8')),legacy=JSON.parse(readFileSync(legacyPath,'utf8'))
const bytes=readFileSync(wasmPath),w=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const report:any={wasmSha256:createHash('sha256').update(bytes).digest('hex'),certificateProducerWasmSha256:input.wasmSha256,node:process.version,sourceProofs:[],sumProofs:[],bindingForgeries:[],legacyProofs:[],reproduced:[],calls:[]}
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
function call(op:string,args:object):any{const start=performance.now(),data=encodeBinary({op,...args}),p=writeLinear(w.memory,w.abi_alloc,data);try{const r=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,p,data.length));assert(r.ok,JSON.stringify(r.error));return r.value}finally{w.abi_free(p,data.length);report.calls.push({op,milliseconds:performance.now()-start})}}
save()
for(const [field,cases,op] of [['sourceProofs',input.sourceProofs,'nurbs_source_atlas_verify'],['sumProofs',input.sumProofs,'minkowski_nurbs_verify'],['legacyProofs',legacy.sumProofs,'minkowski_nurbs_verify']] as const){
 for(const c of cases){const r=call(op,c.request);assert.deepEqual(r.certificate,c.request.certificate)
  if(c.sourceApproximationTopology)assert.deepEqual(r.sourceApproximationTopology,c.sourceApproximationTopology)
  const forg=[];for(const f of c.forgeries??[]){assert.throws(()=>call(op,f.request));forg.push(f)}
  report[field].push({...c,forgeries:forg});save();console.error(field,c.name,'verified')
 }
}
for(const f of bindings.forgeries){assert.throws(()=>call('minkowski_nurbs_verify',f.request));report.bindingForgeries.push(f);save()}
// Verify is a recomputation, not a signature check. Also exercise each public
// producer entry point and compare its canonical certificate to the replay.
const box=input.sourceProofs[0]
assert.deepEqual(call('nurbs_source_atlas_certify',{model:box.request.model,depth:box.request.certificate.depth}).certificate,box.request.certificate)
report.reproduced.push(box.name)
for(const c of input.sumProofs){const {certificate,...request}=c.request,r=call('minkowski_nurbs_certify',request);assert.deepEqual(r.certificate,certificate);assert.deepEqual(r.sourceApproximationTopology,c.sourceApproximationTopology);assert.deepEqual(r.approximationTopology,c.approximationTopology);report.reproduced.push(c.name);save()}
report.complete=true;save();console.log(JSON.stringify({wasmSha256:report.wasmSha256,sourceProofs:report.sourceProofs.length,sumProofs:report.sumProofs.length,legacyProofs:report.legacyProofs.length,forgeriesRejected:[...report.sourceProofs,...report.sumProofs,...report.legacyProofs].reduce((n,p)=>n+p.forgeries.length,0)+report.bindingForgeries.length,reproduced:report.reproduced.length},null,2))
