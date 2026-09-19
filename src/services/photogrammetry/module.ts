import {compileStreamingWasm} from '../wasmStreaming'

let sharedModule: Promise<WebAssembly.Module> | null = null

/**
 * Compiles the kernel once per page; disposable Workers receive the module and only
 * instantiate it. The embedded bytes load lazily so the main bundle stays small.
 */
export function compilePhotogrammetryKernel(): Promise<WebAssembly.Module> {
  if (!sharedModule) {
    sharedModule = compileStreamingWasm('/wasm/photogrammetry.wasm').then(async module => {
      if (module) return module
      const [{default: wasmBase64}, {unpackBrotliWasmBase64}] = await Promise.all([
        import('../../generated/photogrammetry/bytes'),
        import('../wasmBrotliPacking'),
      ])
      return WebAssembly.compile(unpackBrotliWasmBase64(wasmBase64))
    })
    sharedModule.catch(() => { sharedModule = null })
  }
  return sharedModule
}
