import {assertVerifiedWasmModule, snapshotWasmIdentity, type WasmArtifactIdentity} from './wasmArtifact'

export type OptionalWasmCompiler = (url: string, identity?: WasmArtifactIdentity) => Promise<WebAssembly.Module | null>

// Each JS realm defaults to embedded bytes. Only the browser composition root
// installs network compilation; shared kernels and MCP workers own no host I/O.
let optionalCompiler: OptionalWasmCompiler | undefined

export function setOptionalWasmCompiler(compiler: OptionalWasmCompiler | undefined): void {
  optionalCompiler = compiler
}

export async function compileOptionalWasm(url: string, expected?: WasmArtifactIdentity): Promise<WebAssembly.Module | null> {
  if (!optionalCompiler) return null
  const identity = expected ? snapshotWasmIdentity(expected) : undefined
  const module = await (identity ? optionalCompiler(url, identity) : optionalCompiler(url))
  if (module && identity) assertVerifiedWasmModule(module, identity)
  return module
}
