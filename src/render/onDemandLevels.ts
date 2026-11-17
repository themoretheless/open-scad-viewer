/**
 * Display levels produced on demand instead of shipped with the published
 * mesh. A B-rep mesh published without `lods` first gets a level summary
 * (counts and certified errors); `mesh.lod` is then created with deferred
 * placeholder levels shared by every instance of the asset. The progressive
 * level store asks for one level's arrays only when the scheduler targets it,
 * and a newer target for the same geometry cancels the stale request.
 */
import type { MeshLodLevel } from '../core/mesh'
import type { DisplayLevelPayload, DisplayLevelSummaryEntry } from '../kernel/displayLevelService'
import type { DisplayLevelRequest, DisplayLevelResponse } from '../kernel/displayLevelWorkerRuntime'
import { placeholderLevel, isDeferredLevel } from './deferredLevels'
import { createMeshLodState } from './meshLod'
import type { GMesh } from './types'

export interface DisplayLevelProvider {
  summary(geometry: string, geometryJson: string, signal: AbortSignal): Promise<DisplayLevelSummaryEntry[] | null>
  level(geometry: string, geometryJson: string, index: number, signal: AbortSignal): Promise<DisplayLevelPayload>
  forget?(geometry: string): void
  dispose?(): void
}

const abortError = () => new DOMException('Display level request superseded', 'AbortError')

interface GeometryLevels {
  readonly json: string
  summary: DisplayLevelSummaryEntry[] | null | undefined
  summaryAbort: AbortController | null
  /** Shared, mutable coarse-to-fine list handed to every instance's LOD state. */
  levels: MeshLodLevel[] | null
  inflight: { index: number; abort: AbortController; done: Promise<void> } | null
}

/** Geometry key → summary state. `geometryKey` comes from the display scheduler. */
export class OnDemandLevelSource {
  private readonly geometries = new Map<string, GeometryLevels>()

  constructor(
    private readonly provider: DisplayLevelProvider,
    private readonly keyOf: (mesh: GMesh) => string,
    private readonly onSummary: () => void = () => {},
  ) {}

  /** Meshes this source may give levels to: B-rep snapshots published without levels. */
  static eligible(mesh: GMesh): boolean {
    return !mesh.lod && mesh.nativeGeometry?.kind === 'brep' && !!mesh.assetId
  }

  /** Give known level lists to eligible meshes and request missing summaries. Returns meshes that gained LOD state. */
  attach(meshes: readonly GMesh[]): GMesh[] {
    const attached: GMesh[] = []
    for (const mesh of meshes) {
      if (!OnDemandLevelSource.eligible(mesh)) continue
      const key = this.keyOf(mesh)
      const json = mesh.nativeGeometry!.geometryJson
      let entry = this.geometries.get(key)
      if (entry && entry.json !== json) {
        this.drop(key)
        entry = undefined
      }
      if (!entry) {
        entry = { json, summary: undefined, summaryAbort: null, levels: null, inflight: null }
        this.geometries.set(key, entry)
        this.requestSummary(key, entry)
        continue
      }
      if (!entry.summary) continue
      entry.levels ??= entry.summary.map(placeholderLevel)
      const lod = createMeshLodState(mesh, entry.levels)
      if (!lod) continue
      mesh.lod = lod
      attached.push(mesh)
    }
    return attached
  }

  /**
   * Produce the arrays of `index` for the mesh's geometry. Only one level per
   * geometry is in flight: asking for another level aborts the stale request.
   */
  load(mesh: GMesh, index: number): Promise<void> {
    const key = this.keyOf(mesh)
    const entry = this.geometries.get(key)
    const levels = entry?.levels
    if (!entry || !levels || !levels[index]) return Promise.reject(new RangeError('No deferred level to load'))
    if (!isDeferredLevel(levels[index])) return Promise.resolve()
    if (entry.inflight?.index === index) return entry.inflight.done
    entry.inflight?.abort.abort()
    const abort = new AbortController()
    const base = mesh.lod?.base.provenance[0]
    const done = this.provider.level(key, entry.json, index, abort.signal).then(payload => {
      if (abort.signal.aborted) throw abortError()
      const previous = levels[index]!
      levels[index] = {
        ...payload,
        provenance: [
          {
            triangleStart: 0,
            triangleEnd: previous.triangleCount,
            backside: base?.backside ?? false,
            source: base?.source ?? null,
          },
        ],
        triangleCount: previous.triangleCount,
        errorUpper: previous.errorUpper,
      }
    })
    const inflight = { index, abort, done }
    entry.inflight = inflight
    const clear = () => {
      if (entry.inflight === inflight) entry.inflight = null
    }
    done.then(clear, clear)
    return done
  }

  /** Abort the in-flight level of a geometry unless it is `keep`. */
  cancelStale(mesh: GMesh, keep: number) {
    const entry = this.geometries.get(this.keyOf(mesh))
    if (entry?.inflight && entry.inflight.index !== keep) {
      entry.inflight.abort.abort()
      entry.inflight = null
    }
  }

  /** Forget geometry no longer present. */
  retain(meshes: readonly GMesh[]) {
    const live = new Set(meshes.filter(mesh => mesh.nativeGeometry?.kind === 'brep').map(this.keyOf))
    for (const key of [...this.geometries.keys()]) if (!live.has(key)) this.drop(key)
  }

  get pendingRequests() {
    let count = 0
    for (const entry of this.geometries.values()) count += (entry.summaryAbort ? 1 : 0) + (entry.inflight ? 1 : 0)
    return count
  }

  dispose() {
    for (const key of [...this.geometries.keys()]) this.drop(key)
    this.provider.dispose?.()
  }

  private requestSummary(key: string, entry: GeometryLevels) {
    const abort = new AbortController()
    entry.summaryAbort = abort
    this.provider.summary(key, entry.json, abort.signal).then(
      summary => {
        if (abort.signal.aborted || this.geometries.get(key) !== entry) return
        entry.summaryAbort = null
        entry.summary = summary && summary.length >= 2 && summary.length <= 3 ? summary : null
        if (entry.summary) this.onSummary()
      },
      () => {
        if (this.geometries.get(key) !== entry) return
        entry.summaryAbort = null
        entry.summary = null
      },
    )
  }

  private drop(key: string) {
    const entry = this.geometries.get(key)
    if (!entry) return
    entry.summaryAbort?.abort()
    entry.inflight?.abort.abort()
    this.geometries.delete(key)
    this.provider.forget?.(key)
  }
}

/** Provider backed by the dedicated display level worker. */
export class WorkerDisplayLevelProvider implements DisplayLevelProvider {
  private worker: Worker | null = null
  private nextId = 1
  private readonly waiting = new Map<number, { resolve: (value: unknown) => void; reject: (error: unknown) => void }>()

  constructor(private readonly createWorker: () => Worker) {}

  summary(geometry: string, geometryJson: string, signal: AbortSignal) {
    return this.send<DisplayLevelSummaryEntry[] | null>(
      { type: 'summary', geometry, geometryJson },
      signal,
      response => ('summary' in response ? response.summary : null),
    )
  }

  level(geometry: string, geometryJson: string, index: number, signal: AbortSignal) {
    return this.send<DisplayLevelPayload>({ type: 'level', geometry, geometryJson, index }, signal, response => {
      if (!('level' in response)) throw new Error('Display level worker returned no level')
      return response.level
    })
  }

  forget(geometry: string) {
    this.worker?.postMessage({ type: 'forget', geometry } satisfies DisplayLevelRequest)
  }

  dispose() {
    this.worker?.terminate()
    this.worker = null
    for (const { reject } of this.waiting.values()) reject(abortError())
    this.waiting.clear()
  }

  private send<T>(
    request:
      | { type: 'summary'; geometry: string; geometryJson: string }
      | { type: 'level'; geometry: string; geometryJson: string; index: number },
    signal: AbortSignal,
    read: (response: Extract<DisplayLevelResponse, { ok: true }>) => T,
  ): Promise<T> {
    if (signal.aborted) return Promise.reject(abortError())
    const worker = this.ensureWorker()
    const id = this.nextId++
    return new Promise<T>((resolve, reject) => {
      const onAbort = () => {
        if (!this.waiting.delete(id)) return
        worker.postMessage({ type: 'cancel', id } satisfies DisplayLevelRequest)
        reject(abortError())
      }
      signal.addEventListener('abort', onAbort, { once: true })
      this.waiting.set(id, {
        resolve: value => {
          signal.removeEventListener('abort', onAbort)
          try {
            resolve(read(value as Extract<DisplayLevelResponse, { ok: true }>))
          } catch (error) {
            reject(error)
          }
        },
        reject: error => {
          signal.removeEventListener('abort', onAbort)
          reject(error)
        },
      })
      worker.postMessage({ ...request, id } as DisplayLevelRequest)
    })
  }

  private ensureWorker(): Worker {
    if (this.worker) return this.worker
    const worker = this.createWorker()
    worker.addEventListener('message', (event: MessageEvent<DisplayLevelResponse>) => {
      const response = event.data
      const waiter = this.waiting.get(response.id)
      if (!waiter) return
      this.waiting.delete(response.id)
      if (response.ok) waiter.resolve(response)
      else waiter.reject(new Error(response.error))
    })
    worker.addEventListener('error', () => this.dispose())
    this.worker = worker
    return worker
  }
}

let providerFactory: (() => DisplayLevelProvider | null) | null = null

/** The app registers how display levels are produced (the worker lives outside the render layer). */
export function registerDisplayLevelProvider(factory: (() => DisplayLevelProvider | null) | null) {
  providerFactory = factory
}

export const createDisplayLevelProvider = (): DisplayLevelProvider | null => providerFactory?.() ?? null
