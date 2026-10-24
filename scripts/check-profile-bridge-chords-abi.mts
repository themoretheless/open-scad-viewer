import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
assert(process.argv[2]&&process.argv[3],'Provide WASM input path and output report path')
const bytes=readFileSync(process.argv[2])
const {instance}=await WebAssembly.instantiate(bytes)
const wasm=instance.exports as unknown as {memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
const call=(value:unknown)=>{
 const data=encodeBinary(value),ptr=writeLinear(wasm.memory,n=>wasm.abi_alloc(n),data)
 try{return decodePacked<any>(wasm.memory,(p,n)=>wasm.abi_free(p,n),wasm.abi_request(0,ptr,data.length))}
 finally{wasm.abi_free(ptr,data.length)}
}
const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/profile-bridge-native-transport-2026-10-10.json','utf8'))
const cases=[{name:'plane',...fixture},...['rationalProposed','normalDiameter'].map(name=>({name,...fixture[name]}))]
if(process.argv.includes('--whole-coverage'))for(const name of ['wholeCoverage','stoppedCoverage','curvedWholeCoverage'])cases.push({name,...fixture[name]})
for(const c of cases){const r=call(c.request);assert.equal(r.ok,true,JSON.stringify(r));assert.deepEqual(r.value,c.report,c.name)}
const zero=call({...fixture.request,maxSpans:0});assert.equal(zero.ok,true);assert.equal(zero.value.excluded,null);assert.equal(zero.value.spans,0)
const invalid=structuredClone(fixture.request);invalid.maxSpans=0;invalid.pair[0][0][1]=2
assert.equal(call(invalid).ok,false,'Zero work must not bypass native chart validation')
assert.deepEqual(call(fixture.request).value,fixture.report,'Recover after native validation error')
writeFileSync(process.argv[3],JSON.stringify({passed:true,scope:'direct binary ABI; no packaged-worker or UI claim',wasm:{source:process.argv[2],sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length},cases:cases.map(c=>c.name),zeroBudgetRefusal:true,invalidDomainRefusal:true,recovery:true},null,2)+'\n')
