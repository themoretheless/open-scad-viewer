// ABI transport, native-result assertions and timings; no host geometry.
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {cpus} from 'node:os'
import assert from 'node:assert/strict'
import {encodeBinary,decodeBinary} from '../../../src/services/valueBinaryCodec'
const [output]=process.argv.slice(2)
const base='/private/tmp/osv-fibered-ci-size-profiles-20261017/'
const variants=['baseline','nurbs-z','nurbs-predicates-z','nurbs-predicates-planar-z'].map(name=>{
 const bytes=readFileSync(base+name+'/kernel_bg.wasm')
 return {name,wasmBytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex'),module:new WebAssembly.Module(bytes)}
})
const files=['fibered-step/three-pieces/source.json','rational-cap-step/nonplanar-caps.source.json','rational-cap-step/moving-affine-caps.source.json']
const cases=files.map(name=>{
 const raw=readFileSync('docs/qualification/fibered-source-caps-2026-10-16/'+name)
 const request=JSON.parse(raw.toString());if(name.startsWith('rational-cap-step/'))request.limits.constructionRows=1000000
 const input=encodeBinary(request)
 return {name,input,sha256:createHash('sha256').update(raw).digest('hex'),effectiveInputSha256:createHash('sha256').update(input).digest('hex')}
})
function sample(module:WebAssembly.Module,input:Uint8Array) {
 const e=new WebAssembly.Instance(module).exports as any
 const p=e.abi_alloc(input.length);assert(p)
 new Uint8Array(e.memory.buffer,p,input.length).set(input)
 const start=performance.now(),packed=e.abi_request(0,p,input.length) as bigint,elapsedMs=performance.now()-start
 const ptr=Number(packed&0xffffffffn),len=Number(packed>>32n)
 const envelope=decodeBinary(new Uint8Array(e.memory.buffer,ptr,len).slice()) as any
 e.abi_free(ptr,len);e.abi_free(p,input.length)
 assert.equal(envelope.ok,true,JSON.stringify(envelope.error))
 assert.equal(envelope.value.certified,true,envelope.value.reason)
 assert.equal(envelope.value.idealSourceBoundaryG2Certified,true)
 return elapsedMs
}
const rows:any[]=[]
for(const c of cases) {
 // Compile outside timings; warm each module, then alternate paired order.
 for(const v of variants)sample(v.module,c.input)
 const timings=variants.map(()=>[] as number[])
 for(let i=0;i<8;i++)for(const j of i%2?[3,2,1,0]:[0,1,2,3])timings[j].push(sample(variants[j].module,c.input))
 const med=(x:number[])=>{const y=[...x].sort((a,b)=>a-b);return (y[3]+y[4])/2}
 const medians=timings.map(med),ratios=medians.map(x=>x/medians[0])
 const row={case:c.name,inputSha256:c.sha256,effectiveInputSha256:c.effectiveInputSha256,timingsMs:timings,mediansMs:medians,ratiosToBaseline:ratios}
 rows.push(row);console.log(JSON.stringify(row))
}
writeFileSync(output,JSON.stringify({schema:'paired-native-wasm-profile-benchmark/1',node:process.version,cpu:cpus()[0].model,scope:'Three specified Rust certificate requests, fresh instance per sample, ABI execution only, eight alternating pairs; geometry and proof assertions stay native.',variants:variants.map(({module,...r})=>r),rows,fullGoalComplete:false},null,2)+'\n')
