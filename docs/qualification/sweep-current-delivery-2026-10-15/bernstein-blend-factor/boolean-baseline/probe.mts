import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {encodeBinary} from '/Users/themoretheless/Documents/Sources/open-scad-viewer/src/services/valueBinaryCodec.ts'
import {writeLinear,decodePacked} from '/Users/themoretheless/Documents/Sources/open-scad-viewer/src/services/wasmHost.ts'
const hash=(b:Uint8Array)=>createHash('sha256').update(b).digest('hex')
const out='/private/tmp/osv-sweep-boolean-baseline-diagnostic-20261015';mkdirSync(out,{recursive:true})
const paths=[['frozen-N-before-bernstein-change','/private/tmp/osv-sweep-interval-mul-wasm-20261015/crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm'],['current-main-peer-artifact','/Users/themoretheless/Documents/Sources/open-scad-viewer/public/wasm/geometry-kernel.wasm']]
const report:any={scope:'Reproduction of broad native Boolean failures on immutable pre-optimization WASM and actual main peer artifact; not a qualification pass or admission waiver.',artifacts:[],cases:[]}
let inputs:any[]=[]
for(const [kind,path] of paths){
 const bytes=readFileSync(path),artifact={kind,sha256:hash(bytes),byteLength:bytes.length};report.artifacts.push(artifact)
 if(kind==='frozen-N-before-bernstein-change')assert.equal(artifact.sha256,'180c3337e75b032bfa4ff5c2bccbda224ca02b2213342f8195900e7e1fd62990')
 const w=(await WebAssembly.instantiate(bytes)).instance.exports as any
 const call=(op:string,args:any)=>{const data=encodeBinary({op,...args}),ptr=writeLinear(w.memory,w.abi_alloc,data);assert.ok(ptr);try{const start=performance.now(),response=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,ptr,data.length));assert.ok(performance.now()-start<=30000);return response}finally{w.abi_free(ptr,data.length)}}
 if(!inputs.length){
  const box=call('brep_nurbs_box',{min:[.25,-1,-1],max:[.75,2,3]});assert.ok(box.ok)
  for(const [spansU,spansV] of [[2,1],[1,2],[2,2]]){const graph=call('brep_nurbs_canonical_multispan_graph_solid',{spansU,spansV});assert.ok(graph.ok);inputs.push({spansU,spansV,a:graph.value,b:box.value})}
 }
 for(const input of inputs)for(const operation of ['intersection','difference']){
  const request={a:input.a,b:input.b,operation},response=call('brep_nurbs_boolean_general',request)
  report.cases.push({artifact:kind,spansU:input.spansU,spansV:input.spansV,operation,request,response})
  writeFileSync(out+'/report.json',JSON.stringify(report,null,2)+'\n')
  console.log(kind,input.spansU,input.spansV,operation,response.ok?'accepted':response.error?.message)
 }
}
