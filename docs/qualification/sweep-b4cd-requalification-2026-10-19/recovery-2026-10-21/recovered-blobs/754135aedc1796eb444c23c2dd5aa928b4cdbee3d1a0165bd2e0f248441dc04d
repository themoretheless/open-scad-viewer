import { parseDirectDocumentAsync, type DirectBody } from '../directModeling'
import { warmGeometryKernel } from '../geometry/kernel'
import { EXACT_SOLID_TIMEOUT_MS, isExactSolidRequest, isExactSolidResponse, type ExactSolidRequest } from './exactSolidProtocol'

export interface ExactSolidWorkerPort {
  postMessage(request: ExactSolidRequest): void
  terminate(): void
  addEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener): void
  removeEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener): void
}

/** A disposable realm gives synchronous WASM a hard cancellation boundary. */
export function buildExactSolidsInWorker(
  source: string,
  signal?: AbortSignal,
  factory: () => ExactSolidWorkerPort = () => new Worker(new URL('../../workers/geometry.worker.ts', import.meta.url), { type: 'module' }),
): Promise<DirectBody[]> {
  const request: ExactSolidRequest = { kind: 'exact-solid', version: 1, source }
  if (!isExactSolidRequest(request)) return Promise.reject(new RangeError('Group source exceeds 100000 characters.'))
  if (signal?.aborted) return Promise.reject(new DOMException('Operation cancelled', 'AbortError'))
  return new Promise((resolve, reject) => {
    const worker = factory()
    const validationAbort = new AbortController()
    let settled = false
    let timer: ReturnType<typeof setTimeout> | undefined
    const finish = (error?: unknown, bodies?: DirectBody[]) => {
      if (settled) return
      settled = true
      validationAbort.abort()
      clearTimeout(timer)
      signal?.removeEventListener('abort', abort)
      worker.removeEventListener('message', message)
      worker.removeEventListener('error', crash)
      worker.removeEventListener('messageerror', crash)
      worker.terminate()
      if (error !== undefined) reject(error)
      else resolve(bodies!)
    }
    const abort = () => finish(new DOMException('Operation cancelled', 'AbortError'))
    const crash: EventListener = () => finish(new Error('Exact-solid worker stopped unexpectedly.'))
    const message: EventListener = async event => {
      if (settled) return
      worker.removeEventListener('message', message)
      try {
        const response: unknown = (event as MessageEvent).data
        if (!isExactSolidResponse(response)) throw new Error('Invalid exact-solid worker response.')
        if (!response.ok) {
          const error = new Error(response.error.message)
          error.name = response.error.name
          throw error
        }
        // Validation inspects the returned B-rep in the receiving realm too.
        await warmGeometryKernel()
        if (settled) return
        const document = await parseDirectDocumentAsync(response.document, { signal: validationAbort.signal })
        if (settled) return
        if (document.sketches.length || document.curves?.length || document.surfaces?.length
          || document.bodies.some(body => !body.brep)) throw new Error('Worker returned non-solid geometry.')
        finish(undefined, document.bodies)
      } catch (error) { finish(error) }
    }
    worker.addEventListener('message', message)
    worker.addEventListener('error', crash)
    worker.addEventListener('messageerror', crash)
    signal?.addEventListener('abort', abort, { once: true })
    timer = setTimeout(() => finish(new Error('Exact-solid build exceeded its 120 second limit.')), EXACT_SOLID_TIMEOUT_MS)
    if (signal?.aborted) { abort(); return }
    try { worker.postMessage(request) } catch (error) { finish(error) }
  })
}
