import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import * as codec from '../src/core/valueBinaryCodec'
const sha=(b:Uint8Array)=>createHash('sha256').update(b).digest('hex')
const ids:number[]=[], positions:number[]=[]
for(let box=0;box<144;box++){
  const base=positions.length/3
  for(const [x,y,z] of [[0,0,0],[1,0,0],[1,1,0],[0,1,0],[0,0,1],[1,0,1],[1,1,1],[0,1,1]])
    positions.push(x+2*(box%12),y+2*Math.floor(box/12),z)
  for(const i of [0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]) ids.push(i+base)
}
const mesh={positions,indices:ids}
const binaries=process.argv.slice(2).map(p=>readFileSync(p))
const engines=binaries.map(b=>new WebAssembly.Instance(new WebAssembly.Module(b),{}).exports as any)
for(const op of ['mesh_section','mesh_export_stl']){
 const request=codec.encodeBinary({op,mesh,...(op==='mesh_section'?{z:0.5}:{})})
 function run(w:any){
  const p=w.abi_alloc(request.length)
  new Uint8Array(w.memory.buffer,p,request.length).set(request)
  try{
   const res=w.abi_request(0,p,request.length),addr=Number(res&0xffffffffn),len=Number(res>>32n)
   try{return new Uint8Array(w.memory.buffer,addr,len).slice()}finally{w.abi_free(addr,len)}
  }finally{w.abi_free(p,request.length)}
 }
 const outputs=engines.map(run)
 const expected=codec.decodeBinary(outputs[0]) as {ok:boolean;value:any};assert.equal(expected.ok,true)
 if(op==='mesh_section')assert.equal(expected.value.contours.length,144)
 else assert.match(expected.value,/facet normal/)
 for(const out of outputs)assert.equal(sha(out),sha(outputs[0]))
 const results=engines.map(()=>[] as number[])
 for(let i=0;i<20;i++) for(const n of i%2?[1,0]:[0,1]){
   const start=performance.now()
   for(let j=0;j<7;j++)run(engines[n])
   if(i>=5)results[n].push((performance.now()-start)/7)
 }
 console.log(JSON.stringify({op,triangles:ids.length/3,resultSha:sha(outputs[0]),engines:results.map((samples,i)=>({sha256:sha(binaries[i]),bytes:binaries[i].length,medianMs:[...samples].sort((a,b)=>a-b)[7],samplesMs:samples}))}))
}
