import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {encodeBinary,decodeBinary} from '../src/services/valueBinaryCodec.ts'
const [wasmPath,fixturePath,directory]=process.argv.slice(2)
assert(wasmPath&&fixturePath&&directory,'Expected wasm path, native artifact JSON, output directory')
mkdirSync(directory,{recursive:true})
const bytes=readFileSync(wasmPath),native=JSON.parse(readFileSync(fixturePath,'utf8'))
const module=await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module),[])
const instance=await WebAssembly.instantiate(module,{})
const wasm=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc(n:number):number;abi_free(p:number,n:number):void;abi_request(op:number,p:number,n:number):bigint}
const cases:object[]=[]
function request(value:unknown):any{
 const data=encodeBinary(value),ptr=wasm.abi_alloc(data.length);assert(ptr)
 new Uint8Array(wasm.memory.buffer,ptr,data.length).set(data)
 try{
  const packed=wasm.abi_request(0,ptr,data.length),p=Number(packed&0xffffffffn),n=Number(packed>>32n)
  assert(p&&n>0)
  try{return decodeBinary(new Uint8Array(wasm.memory.buffer,p,n))}
  finally{wasm.abi_free(p,n)}
 }finally{wasm.abi_free(ptr,data.length)}
}
function run(name:string,value:unknown,ok:boolean){
 const start=performance.now(),reply=request(value)
 assert.equal(reply.ok,ok,`${name}: ${JSON.stringify(reply.error)}`)
 cases.push({name,ok,elapsedMs:performance.now()-start,error:reply.error})
 console.log(JSON.stringify(cases.at(-1)))
 return reply.value
}
const restored=run('native-frozen-recipe-to-wasm',{op:'cad_full_round_exact_restore',artifact:native},true)
assert.deepEqual(restored.geometry,native.geometry)
assert.deepEqual(restored.recipe,native.recipe)
assert(restored.geometry.topologyIds.faces.length===restored.geometry.faces.length)
assert(restored.geometry.faceOrigins.length===restored.geometry.faces.length)
const legacy=structuredClone(restored)
delete legacy.geometry.topologyIds;delete legacy.geometry.faceOrigins
const upgraded=run('legacy-naming-upgrade',{op:'cad_full_round_exact_restore',artifact:legacy},true)
assert.deepEqual(upgraded.geometry,restored.geometry)
const forgedLineage=structuredClone(restored)
forgedLineage.geometry.faceOrigins[0].origin.face='f:00000000000000000000000000000000'
run('forged-source-lineage',{op:'cad_full_round_exact_restore',artifact:forgedLineage},false)
const constructed=run('wasm-original-source-construction',{op:'cad_full_round_exact',source:native.source,faces:native.faces},true)
const roundtrip=run('wasm-json-frozen-roundtrip',{op:'cad_full_round_exact_restore',artifact:JSON.parse(JSON.stringify(constructed))},true)
assert.deepEqual(roundtrip.geometry,constructed.geometry)
const changed=structuredClone(restored)
changed.geometry.edges[0].curve.homogeneousControls[0][0][0]='999'
run('forged-rational-output',{op:'cad_full_round_exact_restore',artifact:changed},false)
run('caller-success-field',{op:'cad_full_round_exact',source:native.source,faces:native.faces,noAddedMaterial:true},false)
const badRecipe=structuredClone(restored);badRecipe.recipe.contactRanges[0][0]=['1','4']
run('changed-original-source-interval',{op:'cad_full_round_exact_restore',artifact:badRecipe},false)
const report={schema:'full-round-exact-transport-wasm/1',passed:true,
 wasmPath:resolve(wasmPath),wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,
 nativeFixtureSha256:createHash('sha256').update(readFileSync(fixturePath)).digest('hex'),cases,
 scope:'Fresh custom WASM via real binary ABI. Native frozen recipe geometry is identical after reconstruction. This does not update the packaged application artifact or qualify ModelGraph/UI integration.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify({passed:true,cases:cases.length,wasmSha256:report.wasmSha256}))
