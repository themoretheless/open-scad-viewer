import {afterEach, expect, it, vi} from 'vitest'
import {compileGeometryKernelArtifact} from '../src/services/geometry/kernelCompilation'
import {assertVerifiedWasmModule} from '../src/services/wasmArtifact'
import identity from '../src/generated/geometry-kernels/identity'
import {installGeometryKernelFromTrustedHost} from '../src/services/geometry/kernel'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {decodePacked} from '../src/services/wasmHost'

let artifact: Promise<WebAssembly.Module> | undefined
const module = () => artifact ??= compileGeometryKernelArtifact()
afterEach(() => vi.unstubAllGlobals())

it('verifies the compiled native artifact before its immutable delivery', async () => {
  const compiled = await module()
  expect(() => assertVerifiedWasmModule(compiled, identity)).not.toThrow()
  const cloned = structuredClone(compiled)
  expect(cloned).toBeInstanceOf(WebAssembly.Module)
  // Verification belongs to the trusted parent before the clone crosses its
  // private Worker channel; WeakMap brands do not cross JavaScript realms.
  expect(() => assertVerifiedWasmModule(cloned, identity)).toThrow(/unverified/)
})

it('keeps native memory and CAD handle arenas separate between module instances', async () => {
  const compiled = await module()
  const a = await WebAssembly.instantiate(compiled)
  const b = await WebAssembly.instantiate(structuredClone(compiled))
  expect(a.exports.memory).not.toBe(b.exports.memory)
  type Exports = {memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;
    abi_free:(p:number,n:number)=>void;abi_request:(op:number,p:number,n:number)=>bigint}
  const request = (instance:WebAssembly.Instance, args:object):any => {
    const e = instance.exports as unknown as Exports
    const bytes = encodeBinary({op:'cad',...args}), p = e.abi_alloc(bytes.length)
    new Uint8Array(e.memory.buffer,p,bytes.length).set(bytes)
    try {return decodePacked(e.memory,(p,n)=>e.abi_free(p,n),e.abi_request(0,p,bytes.length))}
    finally {e.abi_free(p,bytes.length)}
  }
  const first = request(a,{action:'cube',size:[1,1,1],center:false})
  expect(first.ok).toBe(true)
  expect(request(b,{action:'inspect',id:first.value}).ok).toBe(false)
  const second = request(b,{action:'cube',size:[2,2,2],center:false})
  expect(second.ok).toBe(true)
  expect(request(a,{action:'inspect',id:first.value}).value.volume).toBe(1)
  expect(request(b,{action:'inspect',id:second.value}).value.volume).toBe(8)
  request(a,{action:'delete',ids:[first.value]})
  expect(request(a,{action:'inspect',id:first.value}).ok).toBe(false)
  expect(request(b,{action:'inspect',id:second.value}).value.volume).toBe(8)
  request(b,{action:'delete',ids:[second.value]})
})

it('rejects module installation through ordinary application or Node kernel APIs', async () => {
  await expect(installGeometryKernelFromTrustedHost(await module(),identity))
    .rejects.toThrow('qualification-worker-only')
})
