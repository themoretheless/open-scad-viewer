// Streaming is optional: a stalled response must not block the embedded kernel.
const STREAMING_TIMEOUT_MS = 2_000

export async function compileStreamingWasm(url: string): Promise<WebAssembly.Module | null> {
  if (typeof window === 'undefined') return null
  if (typeof WebAssembly.compileStreaming !== 'function' || typeof fetch !== 'function') return null
  const controller = new AbortController()
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    const deadline = new Promise<null>(resolve => {
      timer = setTimeout(() => resolve(null), STREAMING_TIMEOUT_MS)
    })
    return await Promise.race([
      WebAssembly.compileStreaming(fetch(url, {signal: controller.signal})),
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
