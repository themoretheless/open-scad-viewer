import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
const [wasmPath,input,hostPath,output]=process.argv.slice(2),bytes=readFileSync(wasmPath)
const w=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports as any
const base=JSON.parse(readFileSync(input,'utf8')),host=JSON.parse(readFileSync(hostPath,'utf8'))
assert.equal(createHash('sha256').update(bytes).digest('hex'),base.wasmSha256)
const results:any={wasmSha256:base.wasmSha256,forgeries:[]}
function rejects(request:any){const data=encodeBinary({op:'minkowski_nurbs_verify',...request}),p=writeLinear(w.memory,w.abi_alloc,data);try{const r=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,p,data.length));assert(!r.ok,'forgery accepted');return r.error}finally{w.abi_free(p,data.length)}}
const source=base.sumProofs.find((p:any)=>p.name==='rational-freeform-source-sum')
for(const mutation of ['legacy-version','atlas-receipt','source-bound']){
 const request=structuredClone(source.request)
 if(mutation==='legacy-version')request.certificate.version=3
 else if(mutation==='atlas-receipt')request.certificate.sourceAtlasTopology[0].chartTriangles++
 else request.certificate.sourceHomotopyBounds[0]=['0','1']
 results.forgeries.push({name:source.name,mode:'verify-minkowski-nurbs',mutation,request,rejection:rejects(request)})
}
const critical=base.sumProofs.find((p:any)=>p.name==='critical-coplanar-f32'),request=structuredClone(critical.request)
request.certificate.sumTopology=host.results.find((p:any)=>p.name==='curved-through-hole').proof.certificate.sumTopology
assert(request.certificate.sumTopology)
results.forgeries.push({name:critical.name,mode:'verify-minkowski-nurbs',mutation:'invented-sum-topology',request,rejection:rejects(request)})
writeFileSync(output,JSON.stringify(results,null,2)+'\n');console.log(JSON.stringify({wasmSha256:results.wasmSha256,rejected:results.forgeries.length}))
