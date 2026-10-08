/** Immutable compiled artifact delivery; mutable native state belongs to its instance. */
import {unpackBrotliWasmBase64} from '../wasmBrotliPacking'
import {compileOptionalWasm} from '../wasmCompilation'
import {compileWasmArtifact, compileWasmArtifactSync} from '../wasmArtifact'
import artifactIdentity from '../../generated/geometry-kernels/identity'
import wasmBase64 from '../../generated/geometry-kernels/bytes'

export async function compileGeometryKernelArtifact(): Promise<WebAssembly.Module> {
  // Keep the payload constant outside a logical expression to avoid duplicated
  // fallback literal copies when the bundler inlines this function.
  let module = await compileOptionalWasm('/wasm/geometry-kernel.wasm', artifactIdentity)
  if (!module) module = await compileWasmArtifact(unpackBrotliWasmBase64(wasmBase64), artifactIdentity)
  return module
}
export function compileGeometryKernelArtifactSync(): WebAssembly.Module {
  return compileWasmArtifactSync(unpackBrotliWasmBase64(wasmBase64), artifactIdentity)
}
