import {afterEach, expect, it, vi} from 'vitest'
import {compileOptionalWasm, setOptionalWasmCompiler} from '../src/services/wasmCompilation'

afterEach(() => {
  setOptionalWasmCompiler(undefined)
  vi.unstubAllGlobals()
})

it('uses embedded bytes by default even when browser globals exist', async () => {
  vi.stubGlobal('window', {})
  const fetch = vi.fn()
  vi.stubGlobal('fetch', fetch)
  await expect(compileOptionalWasm('/wasm/test.wasm')).resolves.toBeNull()
  expect(fetch).not.toHaveBeenCalled()
})

it('delegates only to an explicitly installed host compiler and can reset', async () => {
  const module = new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0]))
  const compiler = vi.fn().mockResolvedValue(module)
  setOptionalWasmCompiler(compiler)
  await expect(compileOptionalWasm('/wasm/test.wasm')).resolves.toBe(module)
  expect(compiler).toHaveBeenCalledExactlyOnceWith('/wasm/test.wasm')
  setOptionalWasmCompiler(undefined)
  await expect(compileOptionalWasm('/wasm/test.wasm')).resolves.toBeNull()
  expect(compiler).toHaveBeenCalledTimes(1)
})
