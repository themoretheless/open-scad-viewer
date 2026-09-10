import {unpackWasm} from '../wasmPacking'

let sharedModule: Promise<WebAssembly.Module> | null = null

/**
 * Compiles the kernel once per page; disposable Workers receive the module and only
 * instantiate it. The embedded bytes load lazily so the main bundle stays small.
 */
export function compilePhotogrammetryKernel(): Promise<WebAssembly.Module> {
  if (!sharedModule) {
    sharedModule = import('../../generated/photogrammetry/bytes').then(({default: wasmBase64}) => {
      const compressed = Uint8Array.from(atob(wasmBase64), character => character.charCodeAt(0))
      return WebAssembly.compile(unpackWasm(compressed))
    })
    sharedModule.catch(() => { sharedModule = null })
  }
  return sharedModule
}
