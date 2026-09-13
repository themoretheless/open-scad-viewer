import { SVG_MAX_BYTES, SVG_MAX_FONTS, SVG_MAX_FONT_BYTES, SVG_MAX_TOTAL_FONT_BYTES } from './svgLimits'
import { svgWorkerResult, type SvgGeometryResult, type SvgJob, type SvgWorkerRequest, type SvgWorkerResponse } from './svgWorkerProtocol'

export interface SvgWorkerPort {
  postMessage(message: SvgWorkerRequest): void
  terminate(): void
  addEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener): void
  removeEventListener(type: 'message' | 'error' | 'messageerror', listener: EventListener): void
}
export class SvgWorkerError extends Error {
  constructor(readonly code: string, message: string, name = 'SvgWorkerError') { super(message); this.name = name }
}
const aborted = () => new SvgWorkerError('SVG_CANCELLED', 'SVG operation cancelled.', 'AbortError')
function checkJob(job: SvgJob): SvgJob {
  const fonts = job.options.fonts ?? []
  if (fonts.length > SVG_MAX_FONTS || fonts.some(font => !(font instanceof Uint8Array) || !font.length || font.byteLength > SVG_MAX_FONT_BYTES)
    || fonts.reduce((sum, font) => sum + font.byteLength, 0) > SVG_MAX_TOTAL_FONT_BYTES) throw new RangeError('SVG fonts exceed the supported size or count.')
  const options = { ...job.options, fonts: [...fonts] }
  if (job.kind !== 'project') {
    if (typeof job.svg !== 'string' || job.svg.length > SVG_MAX_BYTES || new TextEncoder().encode(job.svg).length > SVG_MAX_BYTES) throw new RangeError('SVG exceeds 4 MiB.')
    return { ...job, options }
  }
  if (job.face) {
    const mesh = job.meshes[job.face.meshIndex], triangle = job.face.triangleIndex
    if (!mesh || !Number.isInteger(triangle) || triangle < 0 || triangle * 3 + 2 >= mesh.indices.length) throw new RangeError('Select a valid planar face.')
    const faceId = mesh.faceIds[triangle], indices: number[] = [], vertices: number[] = [], remap = new Map<number, number>()
    for (let offset = 0; offset < mesh.indices.length; offset += 3) {
      if (faceId === undefined ? offset !== triangle * 3 : mesh.faceIds[offset / 3] !== faceId) continue
      if (indices.length >= 60000) throw new RangeError('SVG projection is limited to 20000 triangles.')
      for (let corner = 0; corner < 3; corner++) {
        const source = mesh.indices[offset + corner]
        let target = remap.get(source)
        if (target === undefined) {
          target = vertices.length / 6; remap.set(source, target)
          for (let field = 0; field < 6; field++) vertices.push(mesh.vertices[source * 6 + field])
        }
        indices.push(target)
      }
    }
    return { ...job, options, face: { meshIndex: 0, triangleIndex: 0 }, meshes: [{ vertices: new Float32Array(vertices), indices: new Uint32Array(indices), faceIds: new Uint32Array(indices.length / 3), transform: new Float32Array(mesh.transform) }] }
  }
  if (job.meshes.reduce((sum, mesh) => sum + mesh.indices.length / 3, 0) > 20000) throw new RangeError('SVG projection is limited to 20000 triangles.')
  if (job.meshes.reduce((sum, mesh) => sum + mesh.vertices.byteLength + mesh.indices.byteLength + mesh.faceIds.byteLength + mesh.transform.byteLength, 0) > 16 * 1024 * 1024) throw new RangeError('SVG projection data exceeds 16 MiB.')
  // Do not clone renderer metadata or send Vue proxies; never transfer buffers owned by the live scene.
  return { ...job, options, face: undefined,
    meshes: job.meshes.map(mesh => ({ vertices: mesh.vertices, indices: mesh.indices, faceIds: mesh.faceIds, transform: mesh.transform })) }
}

/** One warm realm per owner; noncooperative WASM is stopped by terminating that realm. */
export class SvgWorkerClient {
  private worker: SvgWorkerPort | null = null
  private active: { fail: (error: Error, discard: boolean) => void } | null = null
  private nextId = 1
  private closed = false
  private idleTimer: ReturnType<typeof setTimeout> | undefined
  constructor(private readonly factory: () => SvgWorkerPort, private readonly config: { timeoutMs?: number; idleMs?: number } = {}) {}

  run(job: SvgJob, options: { signal?: AbortSignal; timeoutMs?: number } = {}): Promise<SvgGeometryResult> {
    if (this.closed) return Promise.reject(new SvgWorkerError('SVG_DISPOSED', 'SVG operations are closed.'))
    if (options.signal?.aborted) return Promise.reject(aborted())
    const timeoutMs = options.timeoutMs ?? this.config.timeoutMs ?? 30000
    if (!Number.isFinite(timeoutMs) || timeoutMs < 1 || timeoutMs > 120000) return Promise.reject(new RangeError('SVG timeout must be between 1 and 120000 ms.'))
    let input: SvgJob
    try { input = checkJob(job) } catch (error) { return Promise.reject(error) }
    this.cancel()
    clearTimeout(this.idleTimer)
    let worker: SvgWorkerPort
    try { worker = this.worker ??= this.factory() }
    catch { return Promise.reject(new SvgWorkerError('SVG_STARTUP', 'Could not start SVG processing. Reload the page and try again.')) }
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      let settled = false
      let timer: ReturnType<typeof setTimeout> | undefined
      const cleanup = () => {
        clearTimeout(timer)
        options.signal?.removeEventListener('abort', abort)
        worker.removeEventListener('message', message)
        worker.removeEventListener('error', crash)
        worker.removeEventListener('messageerror', crash)
        if (this.active?.fail === fail) this.active = null
      }
      const finish = (error?: Error, result?: SvgGeometryResult, discard = false) => {
        if (settled) return
        settled = true; cleanup()
        if (discard) this.drop(worker)
        else this.idleTimer = setTimeout(() => { if (!this.active) this.drop(worker) }, this.config.idleMs ?? 15000)
        error ? reject(error) : resolve(result!)
      }
      const fail = (error: Error, discard: boolean) => finish(error, undefined, discard)
      const abort = () => fail(aborted(), true)
      const crash: EventListener = () => fail(new SvgWorkerError('SVG_CRASH', 'SVG processing stopped unexpectedly. Try again with a simpler SVG.'), true)
      const message: EventListener = event => {
        const data = (event as MessageEvent<unknown>).data as Partial<SvgWorkerResponse> | null
        if (data && data.version === 1 && typeof data.id === 'number' && data.id < id) return
        if (!data || typeof data !== 'object' || data.version !== 1 || data.id !== id) {
          fail(new SvgWorkerError('SVG_PROTOCOL', 'Invalid response from SVG processing.'), true); return
        }
        if (data.ok === true && 'result' in data && svgWorkerResult(data.result) && (input.kind !== 'extrude' || typeof data.result.source === 'string')) finish(undefined, data.result)
        else if (data.ok === false && 'error' in data && data.error && typeof data.error.message === 'string' && typeof data.error.name === 'string') {
          finish(new SvgWorkerError(data.error.code ?? 'SVG_INVALID', data.error.message, data.error.name))
        } else fail(new SvgWorkerError('SVG_PROTOCOL', 'Invalid response from SVG processing.'), true)
      }
      this.active = { fail }
      worker.addEventListener('message', message); worker.addEventListener('error', crash); worker.addEventListener('messageerror', crash)
      options.signal?.addEventListener('abort', abort, { once: true })
      timer = setTimeout(() => fail(new SvgWorkerError('SVG_TIMEOUT', 'SVG processing exceeded its time limit. Reduce complexity or silhouette resolution and try again.'), true), timeoutMs)
      try { worker.postMessage({ version: 1, id, job: input }) }
      catch { fail(new SvgWorkerError('SVG_TRANSPORT', 'SVG data could not be sent for processing.'), true) }
    })
  }
  cancel() { this.active?.fail(aborted(), true) }
  dispose() { this.closed = true; this.cancel(); clearTimeout(this.idleTimer); if (this.worker) this.drop(this.worker) }
  private drop(worker: SvgWorkerPort) { if (this.worker === worker) this.worker = null; worker.terminate() }
}
export function createSvgWorkerClient() {
  return new SvgWorkerClient(() => new Worker(new URL('../workers/svg.worker.ts', import.meta.url), { type: 'module' }))
}
