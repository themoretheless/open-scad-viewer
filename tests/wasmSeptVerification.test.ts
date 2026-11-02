import {expect,it} from 'vitest'
import {brotliCompressSync} from 'node:zlib'
import {encodeSeptBase118} from '../scripts/wasm-base91.mjs'
import {verifyPackedWasmChunk,verifyUniquePackedWasm} from '../scripts/verify-packed-wasm.mjs'
it('verifies sept literals by exact decoded WASM and rejects duplicate ownership',()=>{
 const wasm=Buffer.from([0,97,115,109,1,0,0,0]),header=Buffer.alloc(4);header.writeUInt32LE(wasm.length)
 const payload=encodeSeptBase118(Buffer.concat([header,brotliCompressSync(wasm)])),source=`export default '${payload}'`
 expect(verifyPackedWasmChunk(source,wasm,'sept')).toBe(wasm.length)
 expect(verifyUniquePackedWasm([{path:'one.js',source}])).toBe(1)
 expect(()=>verifyUniquePackedWasm([{path:'one.js',source},{path:'two.js',source}])).toThrow('Duplicate')
 expect(()=>verifyPackedWasmChunk(source+';'+source,wasm,'duplicate')).toThrow('exactly one')
 expect(()=>verifyPackedWasmChunk(source,Buffer.concat([wasm,Buffer.from([0])]),'wrong size')).toThrow('decoded size')
 expect(()=>verifyPackedWasmChunk(source.replace('bKx:','bKx:ffffffff'),wasm,'corrupt')).toThrow()
})
