import assert from 'node:assert/strict'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'
import { encodeBinary, decodeBinary } from '../src/app/valueBinaryCodec.ts'
const [wasmPath, directory, sourceCommit] = process.argv.slice(2)
assert(wasmPath && directory && /^[a-f0-9]{40}$/.test(sourceCommit ?? ''))
mkdirSync(directory, { recursive: true })
const bytes = readFileSync(wasmPath)
const module = await WebAssembly.compile(bytes)
assert.deepEqual(WebAssembly.Module.imports(module), [])
const instance = await WebAssembly.instantiate(module, {})
const w = instance.exports as unknown as { memory: WebAssembly.Memory; abi_alloc(n:number):number; abi_free(p:number,n:number):void; abi_request(op:number,p:number,n:number):bigint }
const cases: object[] = []
function request(v:unknown):any {
  const data = encodeBinary(v), original = data.slice(), ptr = w.abi_alloc(data.length)
  assert(ptr)
  new Uint8Array(w.memory.buffer, ptr, data.length).set(data)
  try {
    const packed = w.abi_request(0, ptr, data.length), p = Number(packed & 0xffffffffn), n = Number(packed >> 32n)
    try {
      const result = decodeBinary(new Uint8Array(w.memory.buffer, p, n))
      assert.deepEqual(new Uint8Array(w.memory.buffer, ptr, data.length), original)
      return result
    } finally { w.abi_free(p, n) }
  } finally { w.abi_free(ptr, data.length) }
}
const nativeLog = readFileSync(process.argv[5], 'utf8');
const records = [...nativeLog.matchAll(/exact-transport-oracle=(\{[^\n]+\})/g)].map(m=>({raw:m[1],...JSON.parse(m[1])}));
assert.equal(records.length,11);
for (const record of records) {
 const start=performance.now(), reply=request(record.request);
 assert.equal(reply.ok,true);
 assert.deepEqual(reply.value,record.response);
 const refused=request({...record.request,maxWork:1});
 assert.equal(refused.ok,true); assert.equal(refused.value.status,'refused');
 assert.equal(refused.value.refusal,'resource_limit');
 assert.equal(refused.value.geometry,null); assert.equal(refused.value.metadata,null);
 assert.equal(refused.value.topologyAuthority,false);
 const forged=request({...record.request,topologyAuthority:true}); assert.equal(forged.ok,false);
 cases.push({op:record.request.op,elapsedMs:performance.now()-start,nativeEqual:true,atomicRefusal:true,forgedAuthorityRejected:true});
 // Preserve original integer token spelling: JSON.stringify rounds 2^60's decimal text.
 const rawRequest=record.raw.match(/"request":(\{.*\}),"response":/); assert(rawRequest);
 console.log('exact-transport-oracle={"request":'+rawRequest[1]+',"response":'+JSON.stringify(reply.value)+'}');
}
const result={sourceCommit,nativeLogSha256:createHash('sha256').update(nativeLog).digest('hex'),checks:cases.length*3,wasmSha256:createHash('sha256').update(bytes).digest('hex'),wasmBytes:bytes.length,cases,installedPackageUpdated:false,scope:'Actual raw WASM ABI/native parity for exact primitive records; not installed package, Worker, editor or broad performance qualification'};
writeFileSync(resolve(directory,'results.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result,null,2));
