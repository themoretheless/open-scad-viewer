import {afterEach, beforeEach, expect, it, vi} from 'vitest'
import {compileStreamingWasm} from '../src/services/wasmStreaming'

beforeEach(() => {
  vi.useFakeTimers()
  vi.stubGlobal('window', {})
})
afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

it('returns the streaming module and clears its deadline', async () => {
  const module = new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0]))
  const fetchMock = vi.fn().mockResolvedValue(new Response())
  vi.stubGlobal('fetch', fetchMock)
  vi.spyOn(WebAssembly, 'compileStreaming').mockResolvedValue(module)
  await expect(compileStreamingWasm('/wasm/test.wasm')).resolves.toBe(module)
  expect(fetchMock).toHaveBeenCalledWith('/wasm/test.wasm', {signal: expect.any(AbortSignal)})
  expect(vi.getTimerCount()).toBe(0)
})

it('bounds even an uncancellable compiler and ignores its late rejection', async () => {
  const fetchMock = vi.fn().mockResolvedValue(new Response())
  vi.stubGlobal('fetch', fetchMock)
  let reject!: (error: Error) => void
  vi.spyOn(WebAssembly, 'compileStreaming').mockImplementation(() => new Promise((_, fail) => {reject = fail}))
  const pending = compileStreamingWasm('/wasm/test.wasm')
  let settled = false
  void pending.then(() => {settled = true})
  await vi.advanceTimersByTimeAsync(1_999)
  expect(settled).toBe(false)
  await vi.advanceTimersByTimeAsync(1)
  await expect(pending).resolves.toBeNull()
  expect(fetchMock.mock.calls[0]![1].signal.aborted).toBe(true)
  reject(new Error('late compilation failure'))
  await vi.advanceTimersByTimeAsync(0)
  expect(vi.getTimerCount()).toBe(0)
})

it('aborts a stalled fetch and allows a subsequent call to succeed', async () => {
  vi.stubGlobal('fetch', vi.fn((_url, {signal}: RequestInit) => new Promise((_resolve, reject) => {
    signal!.addEventListener('abort', () => reject(new Error('aborted')), {once: true})
  })))
  vi.spyOn(WebAssembly, 'compileStreaming').mockImplementation(async response => {
    await response
    throw new Error('unexpected response')
  })
  const pending = compileStreamingWasm('/wasm/test.wasm')
  await vi.advanceTimersByTimeAsync(2_000)
  await expect(pending).resolves.toBeNull()
  const module = new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0]))
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response()))
  vi.mocked(WebAssembly.compileStreaming).mockResolvedValue(module)
  await expect(compileStreamingWasm('/wasm/test.wasm')).resolves.toBe(module)
  expect(vi.getTimerCount()).toBe(0)
})

it('falls back immediately on streaming failure without leaking a timer', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response()))
  vi.spyOn(WebAssembly, 'compileStreaming').mockRejectedValue(new Error('invalid MIME or WASM'))
  await expect(compileStreamingWasm('/wasm/test.wasm')).resolves.toBeNull()
  expect(vi.getTimerCount()).toBe(0)
})

it('does not start a network request outside the browser', async () => {
  vi.stubGlobal('window', undefined)
  const fetchMock = vi.fn()
  vi.stubGlobal('fetch', fetchMock)
  await expect(compileStreamingWasm('/wasm/test.wasm')).resolves.toBeNull()
  expect(fetchMock).not.toHaveBeenCalled()
  expect(vi.getTimerCount()).toBe(0)
})

it('warms the real embedded language kernel after the streaming deadline', async () => {
  vi.resetModules()
  const {setOptionalWasmCompiler} = await import('../src/services/wasmCompilation')
  setOptionalWasmCompiler(compileStreamingWasm)
  const {warmLanguageKernel, isLanguageKernelReady, scadCompileRust} = await import('../src/services/languages/kernel')
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response()))
  vi.spyOn(WebAssembly, 'compileStreaming').mockImplementation(() => new Promise(() => {}))
  expect(isLanguageKernelReady()).toBe(false)
  const warming = warmLanguageKernel()
  await vi.advanceTimersByTimeAsync(2_000)
  try {
    await warming
  } finally {
    setOptionalWasmCompiler(undefined)
  }
  expect(isLanguageKernelReady()).toBe(true)
  expect(scadCompileRust('cube([1,2,3]);').ok).toBe(true)
  expect(vi.getTimerCount()).toBe(0)
})
