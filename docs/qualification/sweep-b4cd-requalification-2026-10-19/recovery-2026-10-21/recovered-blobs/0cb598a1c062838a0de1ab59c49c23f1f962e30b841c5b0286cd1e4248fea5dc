/**
 * Cooperative Asynchronous Chunking & Non-Blocking Yield Primitives.
 *
 * Prevents long-running CPU fallbacks (such as Marching Squares, mesh slicing,
 * NURBS evaluation, BVH raycasting, and G-code move filtering) from freezing
 * the browser main UI thread when WebGPU is unavailable or when running outside
 * a dedicated Web Worker.
 */

export interface CooperativeAsyncOptions {
  /** Maximum time slice in milliseconds before yielding back to the event loop (default: 12ms for 60fps responsiveness) */
  timeBudgetMs?: number
  /** Minimum iterations between clock checks to avoid `performance.now()` overhead (default: 256) */
  checkStride?: number
  /** Optional AbortSignal to cancel the asynchronous loop early */
  signal?: AbortSignal
}

/**
 * Yields control back to the host event loop (`MessageChannel` macrotask when available,
 * falling back to `setTimeout(..., 0)`), allowing pending UI input, animations, and
 * `AbortController` events to process.
 */
export function yieldToEventLoop(): Promise<void> {
  if (typeof MessageChannel !== 'undefined') {
    return new Promise<void>(resolve => {
      const channel = new MessageChannel()
      channel.port1.onmessage = () => {
        channel.port1.onmessage = null
        resolve()
      }
      channel.port2.postMessage(null)
    })
  }
  return new Promise<void>(resolve => setTimeout(resolve, 0))
}

function createAbortError(): Error {
  const err = new Error('Operation cancelled')
  err.name = 'AbortError'
  return err
}

/**
 * Executes an indexed loop `[0..length)` cooperatively across macrotask time slices,
 * yielding whenever `timeBudgetMs` is exceeded.
 */
export async function forEachChunkedAsync(
  length: number,
  step: (index: number) => void,
  options: CooperativeAsyncOptions = {}
): Promise<void> {
  const timeBudgetMs = options.timeBudgetMs ?? 12
  const checkStride = Math.max(1, options.checkStride ?? 256)
  const signal = options.signal

  if (signal?.aborted) throw createAbortError()

  let sliceStart = performance.now()
  for (let i = 0; i < length; i++) {
    step(i)
    if ((i + 1) % checkStride === 0) {
      if (signal?.aborted) throw createAbortError()
      const now = performance.now()
      if (now - sliceStart >= timeBudgetMs) {
        await yieldToEventLoop()
        if (signal?.aborted) throw createAbortError()
        sliceStart = performance.now()
      }
    }
  }
}

/**
 * Maps an array `items` asynchronously across cooperative time slices, preserving exact order.
 */
export async function mapChunkedAsync<T, R>(
  items: readonly T[],
  mapper: (item: T, index: number) => R,
  options: CooperativeAsyncOptions = {}
): Promise<R[]> {
  const out = new Array<R>(items.length)
  await forEachChunkedAsync(
    items.length,
    i => {
      out[i] = mapper(items[i]!, i)
    },
    options
  )
  return out
}
