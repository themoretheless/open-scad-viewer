// QA corruption only; every geometry and admission decision runs in Rust.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {join,dirname} from 'node:path'
import {createHash} from 'node:crypto'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

const [root,sourcePath,output]=process.argv.slice(2)
assert.ok(root&&sourcePath&&output)
const sha=(v:Uint8Array|string)=>createHash('sha256').update(v).digest('hex')
const bytes=readFileSync(sourcePath),source=JSON.parse(bytes.toString())
assert.equal(source.passed,true)
const report:any={passed:false,scope:'Exact-artifact C2 cached positive volume followed by a one-ULP retained-pole corruption. No source-error, universal geometry or UI completion claim.',
 runnerSha256:sha(readFileSync(new URL(import.meta.url))),sourceReportSha256:sha(bytes),artifacts:[]}
mkdirSync(dirname(output),{recursive:true})
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
function nextUp(value:number){
 assert.ok(Number.isFinite(value)&&value!==0)
 const view=new DataView(new ArrayBuffer(8));view.setFloat64(0,value)
 view.setBigUint64(0,view.getBigUint64(0)+(value>0?1n:-1n))
 return view.getFloat64(0)
}
for(const kind of ['raw','packed']){
 const artifact=source.artifacts.find((a:any)=>a.kind===kind);assert.equal(artifact.passed,true)
 const wasm=readFileSync(join(root,kind==='raw'?'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm':'public/wasm/geometry-kernel.wasm'))
 assert.equal(sha(wasm),artifact.sha256)
 const w=(await WebAssembly.instantiate(wasm)).instance.exports as any
 const item:any={kind,sha256:sha(wasm),passed:false,calls:[],cases:[]};report.artifacts.push(item);save()
 function call(label:string,model:any,capFaces:number[]){
  const request={op:'brep_sweep_volume_charts_audit',model,capFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS}
  const encoded=encodeBinary(request),ptr=writeLinear(w.memory,w.abi_alloc,encoded),before=JSON.stringify(model)
  let response:any,milliseconds:number
  try{const start=performance.now();response=decodePacked(w.memory,w.abi_free,w.abi_request(0,ptr,encoded.length));milliseconds=performance.now()-start}
  finally{w.abi_free(ptr,encoded.length)}
  item.calls.push({label,requestSha256:sha(encoded),response,milliseconds:milliseconds!});save()
  assert.equal(JSON.stringify(model),before);assert.ok(milliseconds!<=30000)
  assert.equal(response.ok,true,JSON.stringify(response.error));return response.value
 }
 for(const c of artifact.cases){
  const model=structuredClone(c.body.model),caps=c.body.planarCapFaces
  assert.equal(call(c.spacing+'-positive-cache',model,caps).solidGeometryCertified,true)
  const changed=structuredClone(model),pole=changed.faces[0].surface.controlPoints[0][0]
  const axis=pole.findIndex((v:number)=>v!==0);assert.ok(axis>=0)
  const original=pole[axis];pole[axis]=nextUp(original)
  assert.notEqual(pole[axis],original)
  assert.equal(call(c.spacing+'-mutated-after-positive',changed,caps).solidGeometryCertified,false)
  assert.deepEqual(model,c.body.model)
  item.cases.push({spacing:c.spacing,axis,before:original,after:pole[axis],passed:true});save()
 }
 item.passed=true;save()
}
assert.deepEqual(report.artifacts[0].calls.map(({milliseconds,...c}:any)=>c),report.artifacts[1].calls.map(({milliseconds,...c}:any)=>c))
report.passed=true;save();console.log('C2 original-model cache cannot admit one-ULP changed retained bodies; raw/packed full response parity passed')
