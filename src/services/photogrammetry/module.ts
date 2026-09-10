import {unpackWasmBase64} from '../wasmPacking'

let sharedModule: Promise<WebAssembly.Module> | null = null

/**
 * Compiles the kernel once per page; disposable Workers receive the module and only
 * instantiate it. The embedded bytes load lazily so the main bundle stays small.
 */
export function compilePhotogrammetryKernel(): Promise<WebAssembly.Module> {
  if (!sharedModule) {
    sharedModule = import('../../generated/photogrammetry/bytes').then(({default: wasmBase64}) => WebAssembly.compile(unpackWasmBase64(wasmBase64)))
    sharedModule.catch(() => { sharedModule = null })
  }
  return sharedModule
}
