import { checkGcodePreviewJob, isGcodePreviewDocument, type GcodePreviewDocument, type GcodePreviewJob, type GcodePreviewRequest, type GcodePreviewResponse } from './gcodePreviewProtocol'

export interface GcodeWorkerPort {
  postMessage(message: GcodePreviewRequest): void
  terminate(): void
  addEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener): void
  removeEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener): void
}
const aborted = () => new DOMException('G-code preview cancelled.', 'AbortError')

/** Each panel owns its worker. Termination interrupts synchronous WASM work. */
export class GcodePreviewWorker {
  private worker: GcodeWorkerPort | null = null
  private active: { cancel: () => void } | null = null
  private nextId = 1
  private disposed = false
  private idleTimer: ReturnType<typeof setTimeout> | undefined
  constructor(private readonly factory: () => GcodeWorkerPort, private readonly timeoutMs = 120000) {
    if (!Number.isFinite(timeoutMs) || timeoutMs < 1 || timeoutMs > 120000) throw new RangeError('G-code timeout must be between 1 and 120000 ms.')
  }

  run(job: GcodePreviewJob): Promise<GcodePreviewDocument> {
    if (this.disposed) return Promise.reject(new Error('G-code preview is closed.'))
    let input: GcodePreviewJob
    try { input = checkGcodePreviewJob(job) } catch (error) { return Promise.reject(error) }
    this.cancel()
    clearTimeout(this.idleTimer)
    let worker: GcodeWorkerPort
    try { worker = this.worker ??= this.factory() }
    catch { return Promise.reject(new Error('Could not start G-code processing. Reload the page and try again.')) }
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      let settled = false
      let timer: ReturnType<typeof setTimeout> | undefined
      const finish = (error?: Error, result?: GcodePreviewDocument, discard = false) => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        worker.removeEventListener('message', message)
        worker.removeEventListener('error', crash)
        worker.removeEventListener('messageerror', crash)
        if (this.active?.cancel === cancel) this.active = null
        if (discard) this.drop(worker)
        else this.idleTimer = setTimeout(() => { if (!this.active) this.drop(worker) }, 15000)
        if (error) reject(error)
        else resolve(result!)
      }
      const cancel = () => finish(aborted(), undefined, true)
      const crash: EventListener = () => finish(new Error('G-code processing stopped unexpectedly. Try a simpler mesh or fewer layers.'), undefined, true)
      const message: EventListener = event => {
        const data = (event as MessageEvent<unknown>).data as Partial<GcodePreviewResponse> | null
        if (!data || data.version !== 1 || data.id !== id) {
          finish(new Error('Invalid response from G-code processing.'), undefined, true)
        } else if (data.ok === true && 'result' in data && isGcodePreviewDocument(data.result)) {
          finish(undefined, data.result)
        } else if (data.ok === false && typeof data.error === 'string') {
          finish(new Error(data.error))
        } else finish(new Error('Invalid response from G-code processing.'), undefined, true)
      }
      this.active = { cancel }
      worker.addEventListener('message', message)
      worker.addEventListener('error', crash)
      worker.addEventListener('messageerror', crash)
      timer = setTimeout(() => finish(new Error('G-code processing exceeded two minutes. Increase layer height, reduce the Z range, or simplify the mesh.'), undefined, true), this.timeoutMs)
      try { worker.postMessage({ version: 1, id, job: input }) }
      catch { finish(new Error('Could not send the G-code data for processing.'), undefined, true) }
    })
  }
  cancel() { this.active?.cancel() }
  dispose() {
    this.disposed = true
    this.cancel()
    clearTimeout(this.idleTimer)
    if (this.worker) this.drop(this.worker)
  }
  private drop(worker: GcodeWorkerPort) { if (this.worker === worker) this.worker = null; worker.terminate() }
}
export function createGcodePreviewWorker() {
  return new GcodePreviewWorker(() => new Worker(new URL('../workers/gcodePreview.worker.ts', import.meta.url), { type: 'module' }))
}
