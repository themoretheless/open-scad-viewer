export type OptionalWasmCompiler = (url: string) => Promise<WebAssembly.Module | null>

// Each JS realm defaults to embedded bytes. Only the browser composition root
// installs network compilation; shared kernels and MCP workers own no host I/O.
let optionalCompiler: OptionalWasmCompiler | undefined

export function setOptionalWasmCompiler(compiler: OptionalWasmCompiler | undefined): void {
  optionalCompiler = compiler
}

export function compileOptionalWasm(url: string): Promise<WebAssembly.Module | null> {
  return optionalCompiler ? optionalCompiler(url) : Promise.resolve(null)
}
