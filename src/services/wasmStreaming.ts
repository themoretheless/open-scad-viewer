export async function compileStreamingWasm(url: string): Promise<WebAssembly.Module | null> {
  if (typeof window === 'undefined') return null
  if (typeof WebAssembly.compileStreaming !== 'function' || typeof fetch !== 'function') return null
  try {
    return await WebAssembly.compileStreaming(fetch(url))
  } catch {
    return null
  }
}
