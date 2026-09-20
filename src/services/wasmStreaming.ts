import {compileWasmArtifact, snapshotWasmIdentity, type WasmArtifactIdentity} from './wasmArtifact'

// Streaming is optional: a stalled response must not block the embedded kernel.
const STREAMING_TIMEOUT_MS = 2_000

async function compileBoundedResponse(response: Response, identity: WasmArtifactIdentity, signal: AbortSignal): Promise<WebAssembly.Module> {
  if (!response.ok || !response.body) throw new Error('Missing WASM response body')
  if (signal.aborted) {
    void response.body.cancel().catch(() => {})
    throw new Error('WASM fetch aborted')
  }
  const reader = response.body.getReader()
  const cancel = () => { void reader.cancel().catch(() => {}) }
  signal.addEventListener('abort', cancel, {once: true})
  const bytes = new Uint8Array(identity.byteLength)
  let offset = 0
  try {
    for (;;) {
      const {done, value} = await reader.read()
      if (done) break
      if (value.byteLength > bytes.length - offset) throw new Error('WASM response exceeds artifact size')
      bytes.set(value, offset)
      offset += value.byteLength
    }
    if (offset !== bytes.length) throw new Error('Truncated WASM response')
  } finally {
    signal.removeEventListener('abort', cancel)
    cancel()
    reader.releaseLock()
  }
  return compileWasmArtifact(bytes, identity)
}

export async function compileStreamingWasm(url: string, expected?: WasmArtifactIdentity): Promise<WebAssembly.Module | null> {
  if (typeof window === 'undefined') return null
  if (typeof fetch !== 'function' || (!expected && typeof WebAssembly.compileStreaming !== 'function')) return null
  const controller = new AbortController()
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    const deadline = new Promise<null>(resolve => {
      timer = setTimeout(() => resolve(null), STREAMING_TIMEOUT_MS)
    })
    const identity = expected ? snapshotWasmIdentity(expected) : undefined
    const response = fetch(url, {signal: controller.signal})
    // Bound artifacts are verified before compilation; no unverified module is published.
    const compilation = identity
      ? response.then(body => compileBoundedResponse(body, identity, controller.signal))
      : WebAssembly.compileStreaming(response)
    return await Promise.race([
      compilation,
      deadline,
    ])
  } catch {
    return null
  } finally {
    clearTimeout(timer)
    // Also releases a body still being fetched after early compilation failure.
    controller.abort()
  }
}
