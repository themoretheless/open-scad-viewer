let sharedModule: Promise<WebAssembly.Module> | null = null

/**
 * Compiles the kernel once per page; disposable Workers receive the module and only
 * instantiate it. The embedded bytes load lazily so the main bundle stays small.
 */
export function compilePhotogrammetryKernel(): Promise<WebAssembly.Module> {
  if (!sharedModule) {
    sharedModule = Promise.all([
      import('../../generated/photogrammetry/bytes'),
      import('../wasmBrotliPacking'),
    ]).then(([{default: wasmBase64}, {unpackBrotliWasmBase64}]) => WebAssembly.compile(unpackBrotliWasmBase64(wasmBase64)))
    sharedModule.catch(() => { sharedModule = null })
  }
  return sharedModule
}
