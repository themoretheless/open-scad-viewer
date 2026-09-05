import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerEvent,
  type DocumentRevision,
  type GeometryBuildError,
  type GeometryBuildFailure,
  type GeometryBuildPhase,
  type GeometryBuildProgress,
  type GeometryBuildRequest,
  type GeometryBuildStale,
  type GeometryBuildSuccess,
  type GeometryBuildCancelled,
  type GeometryCancelReason,
  type GeometryJobId,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from './geometryWorkerProtocol'
import { sha256Hex } from '../core/sha256'
import type { GeometryQuality } from '../core/build'
import {
  parseGeometrySourceRoutingHeader,
  planGeometrySourceExecution,
} from '../core/geometryExecution'

export interface WorkerLike {
  postMessage(message: GeometryWorkerRequest): void
  terminate(): void
  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void
  removeEventListener(type: string, listener: EventListenerOrEventListenerObject): void
}

export interface CoordinatorTimers {
  setTimeout(callback: () => void, delayMs: number): unknown
  clearTimeout(handle: unknown): void
}

export interface GeometryBuildInput {
  documentRevision: DocumentRevision
  source: string
  quality: GeometryQuality
}

export type BuildCoordinatorStatus = 'idle' | 'building' | 'ready' | 'failed' | 'cancelled' | 'stale' | 'disposed'
export type PublishedGeometryBuild = (GeometryBuildSuccess | GeometryBuildFailure) & {
  /** Main-thread request to validated publication; includes queue/startup/transport. */
  readonly hostElapsedMs?: number
}

export interface BuildCoordinatorState {
  readonly status: BuildCoordinatorStatus
  readonly documentRevision: DocumentRevision | null
  readonly jobId: GeometryJobId | null
  readonly requestedQuality: GeometryQuality | null
  readonly publishedQuality: GeometryQuality | null
  readonly phase: GeometryBuildPhase | null
  readonly progress: number | null
  readonly activeJobIds: readonly GeometryJobId[]
  readonly error: GeometryBuildError | null
}

export interface BuildCoordinatorOptions {
  workerFactory: () => WorkerLike
  /** Called for publishable successes and failures only; stale jobs never reach it. */
  onPublish?: (outcome: PublishedGeometryBuild) => void
  onProgress?: (progress: GeometryBuildProgress) => void
  onStateChange?: (state: BuildCoordinatorState) => void
  now?: () => number
  timers?: CoordinatorTimers
  /**
   * Allows an already-finishing worker to stay warm. Once the grace expires,
   * the worker is replaced because synchronous Manifold work cannot be
   * cooperatively interrupted. Defaults to immediate hard preemption.
   */
  supersedeGraceMs?: number
}

type JobLifecycle = 'queued' | 'posted' | 'accepted' | 'started' | 'superseded' | 'terminal'

interface JobRecord {
  request: GeometryBuildRequest
  lifecycle: JobLifecycle
  requestedAt: number
  workerGeneration: number | null
}

interface WorkerBinding {
  worker: WorkerLike
  generation: number
  messageListener: EventListener
  errorListener: EventListener
}

function sourceSpansFit(source: string, event: GeometryWorkerEvent): boolean {
  const isBoundary = (offset: number) => offset === 0
    || offset === source.length
    || !(source.charCodeAt(offset - 1) >= 0xd800
      && source.charCodeAt(offset - 1) <= 0xdbff
      && source.charCodeAt(offset) >= 0xdc00
      && source.charCodeAt(offset) <= 0xdfff)
  const spanFits = (start: number, end: number) => start <= end
    && end <= source.length
    && isBoundary(start)
    && isBoundary(end)
  if (event.status === 'failed') {
    const { start, end } = event.error
    return (start === undefined && end === undefined)
      || (start !== undefined && end !== undefined && spanFits(start, end))
  }
  if (event.status !== 'succeeded') return true
  return event.meshes.every(mesh => mesh.provenance.every(run => (
    run.source === null || spanFits(run.source.start, run.source.end)
  )))
}

function deepFreezeWorkerSnapshot(value: unknown, seen = new WeakSet<object>()): void {
  if (value === null || typeof value !== 'object' || seen.has(value)) return
  seen.add(value)
  if (ArrayBuffer.isView(value)) {
    Object.freeze(value.buffer)
    return
  }
  for (const key of Reflect.ownKeys(value)) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    if (descriptor && Object.hasOwn(descriptor, 'value')) deepFreezeWorkerSnapshot(descriptor.value, seen)
  }
  Object.freeze(value)
}

function freezeAttestedWorkerEvent(event: GeometryWorkerEvent): GeometryWorkerEvent {
  const snapshot = structuredClone(event) as GeometryWorkerEvent
  deepFreezeWorkerSnapshot(snapshot)
  return snapshot
}

const DEFAULT_TIMERS: CoordinatorTimers = {
  setTimeout: (callback, delayMs) => globalThis.setTimeout(callback, delayMs),
  clearTimeout: handle => globalThis.clearTimeout(handle as ReturnType<typeof globalThis.setTimeout>),
}

const INITIAL_STATE: BuildCoordinatorState = Object.freeze({
  status: 'idle',
  documentRevision: null,
  jobId: null,
  requestedQuality: null,
  publishedQuality: null,
  phase: null,
  progress: null,
  activeJobIds: Object.freeze([]),
  error: null,
})

function qualityRank(quality: GeometryQuality | null): number {
  if (quality === 'full') return 2
  if (quality === 'preview') return 1
  return 0
}

function assertInput(input: GeometryBuildInput) {
  if (!Number.isSafeInteger(input.documentRevision) || input.documentRevision < 0) {
    throw new RangeError('documentRevision must be a non-negative safe integer')
  }
  if (input.quality !== 'preview' && input.quality !== 'full') {
    throw new TypeError('quality must be preview or full')
  }
  if (typeof input.source !== 'string') throw new TypeError('source must be a string')
}

function workerError(message: string, cause?: unknown): GeometryBuildError {
  if (cause instanceof Error) return { name: cause.name || 'WorkerError', message: cause.message || message }
  return { name: 'WorkerError', message }
}

/**
 * Owns geometry-build ordering and the Worker lifecycle without depending on
 * Vue. Document revisions are monotonic, and only results belonging to the
 * current revision and latest job of a quality tier are publishable.
 */
export class BuildCoordinator {
  private readonly options: BuildCoordinatorOptions
  private readonly now: () => number
  private readonly timers: CoordinatorTimers
  private readonly supersedeGraceMs: number
  private readonly jobs = new Map<GeometryJobId, JobRecord>()
  private readonly latestJobByQuality = new Map<GeometryQuality, GeometryJobId>()
  private readonly pendingJobIds: GeometryJobId[] = []
  private readonly supersededWorkerJobs = new Set<GeometryJobId>()

  private binding: WorkerBinding | null = null
  private hardRestartTimer: unknown | null = null
  private nextJobId = 1
  private nextWorkerGeneration = 1
  private latestRevision: DocumentRevision | null = null
  private latestSource: string | null = null
  private snapshot: BuildCoordinatorState = INITIAL_STATE

  constructor(options: BuildCoordinatorOptions) {
    if (typeof options.workerFactory !== 'function') throw new TypeError('workerFactory is required')
    const grace = options.supersedeGraceMs ?? 0
    if (!Number.isFinite(grace) || grace < 0) throw new RangeError('supersedeGraceMs must be non-negative')
    this.options = options
    this.now = options.now ?? (() => performance.now())
    this.timers = options.timers ?? DEFAULT_TIMERS
    this.supersedeGraceMs = grace
  }

  private counters = { builds: 0, superseded: 0, workerStarts: 0, hardRestarts: 0 }

  get diagnostics() { return { ...this.counters } }

  get state(): BuildCoordinatorState {
    return this.snapshot
  }

  requestBuild(input: GeometryBuildInput): GeometryJobId {
    const requestedAt = this.now()
    this.assertUsable()
    assertInput(input)
    if (this.latestRevision !== null && input.documentRevision < this.latestRevision) {
      throw new RangeError(`documentRevision ${input.documentRevision} is older than ${this.latestRevision}`)
    }
    if (input.documentRevision === this.latestRevision && this.latestSource !== input.source) {
      throw new Error('A document revision must identify exactly one source snapshot')
    }

    const existingJobId = this.latestJobByQuality.get(input.quality)
    const existing = existingJobId === undefined ? undefined : this.jobs.get(existingJobId)
    if (existing
      && existing.request.documentRevision === input.documentRevision
      && existing.request.source === input.source
      && existing.lifecycle !== 'terminal'
      && existing.lifecycle !== 'superseded') {
      return existing.request.jobId
    }

    const request: GeometryBuildRequest = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: input.documentRevision,
      jobId: this.nextJobId++,
      source: input.source,
      sourceSha256: sha256Hex(input.source),
      quality: input.quality,
    }
    const record: JobRecord = {
      request,
      lifecycle: 'queued',
      requestedAt,
      workerGeneration: null,
    }
    this.counters.builds++
    this.jobs.set(request.jobId, record)

    const isNewRevision = this.latestRevision === null || input.documentRevision > this.latestRevision
    if (isNewRevision) this.beginRevision(input.documentRevision, input.source)
    this.latestJobByQuality.set(input.quality, request.jobId)

    if (isNewRevision && this.hasPostedWork()) {
      this.supersedePostedWork(request.jobId)
    } else if (this.hardRestartTimer !== null) {
      this.queuePending(request.jobId)
    } else {
      this.postJob(record)
    }

    if (record.lifecycle !== 'terminal') this.updateBuildingState()
    return request.jobId
  }

  cancel(reason: GeometryCancelReason = 'user') {
    if (this.snapshot.status === 'disposed') return
    this.clearHardRestartTimer()
    for (const record of this.jobs.values()) {
      if (record.lifecycle === 'terminal' || record.lifecycle === 'superseded') continue
      if (record.workerGeneration !== null) this.postCancel(record, reason)
      record.lifecycle = 'terminal'
    }
    this.pendingJobIds.length = 0
    this.supersededWorkerJobs.clear()
    this.destroyWorker()
    this.jobs.clear()
    this.latestJobByQuality.clear()
    this.setState({
      status: 'cancelled',
      phase: null,
      progress: null,
      activeJobIds: [],
      error: null,
    })
  }

  dispose() {
    if (this.snapshot.status === 'disposed') return
    this.cancel('disposed')
    this.jobs.clear()
    this.latestJobByQuality.clear()
    this.setState({
      status: 'disposed',
      documentRevision: null,
      jobId: null,
      requestedQuality: null,
      publishedQuality: null,
      phase: null,
      progress: null,
      activeJobIds: [],
      error: null,
    })
  }

  private assertUsable() {
    if (this.snapshot.status === 'disposed') throw new Error('BuildCoordinator has been disposed')
  }

  private beginRevision(revision: DocumentRevision, source: string) {
    this.latestRevision = revision
    this.latestSource = source
    this.latestJobByQuality.clear()
    for (const [jobId, record] of this.jobs) {
      if (record.request.documentRevision === revision || record.lifecycle === 'terminal') continue
      if (record.lifecycle !== 'superseded') this.counters.superseded++
      record.lifecycle = 'superseded'
      if (record.workerGeneration === this.binding?.generation) this.supersededWorkerJobs.add(record.request.jobId)
      else this.jobs.delete(jobId)
    }
    for (let index = this.pendingJobIds.length - 1; index >= 0; index--) {
      const record = this.jobs.get(this.pendingJobIds[index])
      if (!record || record.request.documentRevision !== revision) this.pendingJobIds.splice(index, 1)
    }
    this.setState({
      status: 'building',
      documentRevision: revision,
      jobId: null,
      requestedQuality: null,
      publishedQuality: null,
      phase: 'queued',
      progress: null,
      activeJobIds: [],
      error: null,
    })
  }

  private hasPostedWork() {
    return this.supersededWorkerJobs.size > 0
  }

  private supersedePostedWork(newJobId: GeometryJobId) {
    this.queuePending(newJobId)
    for (const jobId of this.supersededWorkerJobs) {
      const record = this.jobs.get(jobId)
      if (record) this.postCancel(record, 'superseded')
    }
    if (this.hardRestartTimer !== null) return
    if (this.supersedeGraceMs === 0) {
      this.restartAndPostPending()
      return
    }
    this.hardRestartTimer = this.timers.setTimeout(() => {
      this.hardRestartTimer = null
      this.restartAndPostPending()
    }, this.supersedeGraceMs)
  }

  private postCancel(record: JobRecord, reason: GeometryCancelReason) {
    if (!this.binding || record.workerGeneration !== this.binding.generation) return
    try {
      this.binding.worker.postMessage({
        protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
        type: 'cancel',
        documentRevision: record.request.documentRevision,
        jobId: record.request.jobId,
        reason,
      })
    } catch {
      // Replacement below is the actual cancellation boundary. The cancel
      // message is only a best-effort fast path for work not inside Manifold.
    }
  }

  private postJob(record: JobRecord) {
    if (record.lifecycle !== 'queued') return
    const binding = this.ensureWorker(record)
    if (!binding) return
    record.workerGeneration = binding.generation
    record.lifecycle = 'posted'
    try {
      binding.worker.postMessage(record.request)
    } catch (error) {
      this.failForWorker(record, workerError('Could not post a geometry build to the Worker', error))
    }
  }

  private queuePending(jobId: GeometryJobId) {
    const record = this.jobs.get(jobId)
    if (!record || record.lifecycle !== 'queued') return
    for (let index = this.pendingJobIds.length - 1; index >= 0; index--) {
      const pendingId = this.pendingJobIds[index]
      const pending = this.jobs.get(pendingId)
      if (!pending
        || pending.request.documentRevision !== record.request.documentRevision
        || pending.request.quality !== record.request.quality) continue
      this.pendingJobIds.splice(index, 1)
      pending.lifecycle = 'terminal'
      this.jobs.delete(pendingId)
    }
    this.pendingJobIds.push(jobId)
  }

  private isLatestPending(record: JobRecord) {
    return record.request.documentRevision === this.latestRevision
      && this.latestJobByQuality.get(record.request.quality) === record.request.jobId
  }

  private ensureWorker(record: JobRecord): WorkerBinding | null {
    if (this.binding) return this.binding
    try {
      const worker = this.options.workerFactory()
      this.counters.workerStarts++
      const generation = this.nextWorkerGeneration++
      const messageListener: EventListener = event => {
        this.handleWorkerMessage((event as MessageEvent<unknown>).data, generation)
      }
      const errorListener: EventListener = event => this.handleWorkerError(event, generation)
      worker.addEventListener('message', messageListener)
      worker.addEventListener('error', errorListener)
      this.binding = { worker, generation, messageListener, errorListener }
      return this.binding
    } catch (error) {
      this.failForWorker(record, workerError('Could not create the geometry Worker', error))
      return null
    }
  }

  private destroyWorker() {
    if (!this.binding) return
    const { worker, messageListener, errorListener } = this.binding
    worker.removeEventListener('message', messageListener)
    worker.removeEventListener('error', errorListener)
    worker.terminate()
    this.binding = null
  }

  private restartAndPostPending() {
    if (this.binding) this.counters.hardRestarts++
    this.clearHardRestartTimer()
    this.destroyWorker()
    for (const jobId of this.supersededWorkerJobs) this.jobs.delete(jobId)
    this.supersededWorkerJobs.clear()
    const pending = this.pendingJobIds.splice(0)
    for (const jobId of pending) {
      const record = this.jobs.get(jobId)
      if (record && this.isLatestPending(record)) this.postJob(record)
    }
    if (this.activeJobIds().length > 0) this.updateBuildingState()
  }

  private postPendingOnWarmWorker() {
    this.clearHardRestartTimer()
    const pending = this.pendingJobIds.splice(0)
    for (const jobId of pending) {
      const record = this.jobs.get(jobId)
      if (record && this.isLatestPending(record)) this.postJob(record)
    }
    if (this.activeJobIds().length > 0) this.updateBuildingState()
  }

  private handleWorkerMessage(data: unknown, generation: number) {
    if (!this.binding || generation !== this.binding.generation) return
    if (!isGeometryWorkerEvent(data)) {
      this.handleProtocolFailure(generation)
      return
    }
    let event: GeometryWorkerEvent
    try {
      // Snapshot immediately after admission. The clone owns every transferable
      // ArrayBuffer and severs aliases held by same-realm worker test doubles.
      event = freezeAttestedWorkerEvent(data)
    } catch {
      this.handleProtocolFailure(generation)
      return
    }
    const record = this.jobs.get(event.jobId)
    if (!record || record.workerGeneration !== generation) return
    if (event.documentRevision !== record.request.documentRevision || event.quality !== record.request.quality) {
      this.failForWorker(record, {
        name: 'ProtocolError',
        message: `Geometry Worker event correlation mismatch for job ${event.jobId}`,
      })
      return
    }
    if (event.sourceSha256 !== record.request.sourceSha256
      || event.sourceSha256 !== sha256Hex(record.request.source)) {
      this.failForWorker(record, {
        name: 'ProtocolError',
        message: `Geometry Worker source attestation mismatch for job ${event.jobId}`,
      })
      return
    }
    if (!sourceSpansFit(record.request.source, event)) {
      this.failForWorker(record, {
        name: 'ProtocolError',
        message: `Geometry Worker source span exceeds the attested source for job ${event.jobId}`,
      })
      return
    }
    if ('execution' in event && event.execution !== undefined) {
      const execution = event.execution
      let routeMatches = false
      try {
        const route = parseGeometrySourceRoutingHeader(record.request.source)
        routeMatches = route.languageContract === execution.languageContract
          && route.requiredCapabilities.length === execution.requiredCapabilities.length
          && route.requiredCapabilities.every((capability, index) => (
            capability === execution.requiredCapabilities[index]
          ))
      } catch {
        routeMatches = false
      }
      if (!routeMatches) {
        this.failForWorker(record, {
          name: 'ProtocolError',
          message: `Geometry Worker execution provenance mismatch for job ${event.jobId}`,
        })
        return
      }
    }
    if (event.status === 'failed' && event.execution === undefined) {
      try {
        planGeometrySourceExecution(record.request.source, {
          quality: record.request.quality,
          purpose: record.request.quality,
        })
        this.failForWorker(record, {
          name: 'ProtocolError',
          message: `Geometry Worker omitted execution provenance for job ${event.jobId}`,
        })
        return
      } catch {
        // A malformed routing header cannot produce a selected-engine descriptor.
      }
    }

    if (event.status === 'succeeded' || event.status === 'failed' || event.status === 'cancelled' || event.status === 'stale') {
      if (record.lifecycle === 'superseded') {
        this.finishSupersededJob(event.jobId)
        return
      }
      record.lifecycle = 'terminal'
      try {
        if (event.status === 'succeeded') this.handleSuccess(record, event)
        else if (event.status === 'failed') this.handleFailure(record, event)
        else this.handleNonPublishedTerminal(record, event)
      } finally {
        this.jobs.delete(event.jobId)
      }
      return
    }

    if (!this.isCurrentLatest(record)) return
    if (event.status === 'accepted') record.lifecycle = 'accepted'
    else if (event.status === 'started') record.lifecycle = 'started'
    else this.options.onProgress?.(event)

    if (record.request.quality === this.requestedQuality()) {
      this.setState({
        status: 'building',
        jobId: record.request.jobId,
        requestedQuality: record.request.quality,
        phase: event.phase,
        progress: event.status === 'progress' ? event.progress : null,
        activeJobIds: this.activeJobIds(),
        error: null,
      })
    }
  }

  private handleSuccess(record: JobRecord, event: GeometryBuildSuccess) {
    if (!this.isCurrentLatest(record)) return
    const published = this.snapshot.publishedQuality
    if (qualityRank(event.quality) < qualityRank(published)) {
      this.updateAfterTerminal()
      return
    }
    const nextPublished = qualityRank(event.quality) >= qualityRank(published) ? event.quality : published
    const active = this.activeJobIds(record.request.jobId)
    if (active.length > 0) {
      this.setState({
        status: 'building',
        publishedQuality: nextPublished,
        requestedQuality: this.requestedQuality(),
        jobId: this.requestedJobId(),
        phase: 'queued',
        progress: null,
        activeJobIds: active,
        error: null,
      })
    } else {
      this.setState({
        status: 'ready',
        publishedQuality: nextPublished,
        requestedQuality: event.quality,
        jobId: event.jobId,
        phase: 'complete',
        progress: 1,
        activeJobIds: [],
        error: null,
      })
    }
    this.options.onPublish?.(Object.freeze({ ...event, hostElapsedMs: Math.max(0, this.now() - record.requestedAt) }))
  }

  private handleFailure(record: JobRecord, event: GeometryBuildFailure) {
    if (!this.isCurrentLatest(record)) return
    if (qualityRank(event.quality) < qualityRank(this.snapshot.publishedQuality)) {
      this.updateAfterTerminal(record.request.jobId)
      return
    }
    const hasHigherQualityWork = [...this.jobs.values()].some(candidate => (
      candidate.request.documentRevision === this.latestRevision
      && candidate.request.jobId !== record.request.jobId
      && candidate.lifecycle !== 'terminal'
      && candidate.lifecycle !== 'superseded'
      && qualityRank(candidate.request.quality) > qualityRank(record.request.quality)
    ))
    if (hasHigherQualityWork) {
      this.updateAfterTerminal(record.request.jobId)
      return
    }
    this.setState({
      status: 'failed',
      requestedQuality: event.quality,
      jobId: event.jobId,
      phase: event.phase,
      progress: null,
      activeJobIds: this.activeJobIds(record.request.jobId),
      error: event.error,
    })
    this.options.onPublish?.(Object.freeze({ ...event, hostElapsedMs: Math.max(0, this.now() - record.requestedAt) }))
  }

  private handleNonPublishedTerminal(record: JobRecord, event: GeometryBuildCancelled | GeometryBuildStale) {
    if (!this.isCurrentLatest(record)) return
    const active = this.activeJobIds(record.request.jobId)
    if (active.length > 0) {
      this.updateAfterTerminal(record.request.jobId)
      return
    }
    this.setState({
      status: event.status,
      jobId: record.request.jobId,
      requestedQuality: record.request.quality,
      phase: null,
      progress: null,
      activeJobIds: [],
      error: null,
    })
  }

  private finishSupersededJob(jobId: GeometryJobId) {
    this.jobs.delete(jobId)
    this.supersededWorkerJobs.delete(jobId)
    if (this.supersededWorkerJobs.size === 0 && this.pendingJobIds.length > 0) {
      this.postPendingOnWarmWorker()
    }
  }

  private handleWorkerError(event: Event, generation: number) {
    if (!this.binding || generation !== this.binding.generation) return
    if (this.pendingJobIds.length > 0 && this.supersededWorkerJobs.size > 0) {
      this.restartAndPostPending()
      return
    }
    const record = this.primaryActiveJob(generation)
    if (!record) {
      this.destroyWorker()
      return
    }
    const candidate = event as ErrorEvent
    this.failForWorker(record, workerError(candidate.message || 'Geometry Worker failed', candidate.error))
  }

  private handleProtocolFailure(generation: number) {
    const record = this.primaryActiveJob(generation)
    if (!record) return
    this.failForWorker(record, {
      name: 'ProtocolError',
      message: `Geometry Worker protocol mismatch (expected v${GEOMETRY_WORKER_PROTOCOL_VERSION})`,
    })
  }

  private failForWorker(record: JobRecord, error: GeometryBuildError) {
    const effectiveRecord = this.primaryCurrentJob() ?? record
    const generation = record.workerGeneration
    let execution: GeometryBuildFailure['execution']
    try {
      execution = planGeometrySourceExecution(effectiveRecord.request.source, {
        quality: effectiveRecord.request.quality,
        purpose: effectiveRecord.request.quality,
      })
    } catch {
      execution = undefined
    }
    const event = freezeAttestedWorkerEvent({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      status: 'failed',
      phase: effectiveRecord.lifecycle === 'queued' ? 'queued' : 'compiling',
      documentRevision: effectiveRecord.request.documentRevision,
      jobId: effectiveRecord.request.jobId,
      quality: effectiveRecord.request.quality,
      sourceSha256: effectiveRecord.request.sourceSha256,
      ...(execution ? { execution } : {}),
      error,
      durationMs: Math.max(0, this.now() - effectiveRecord.requestedAt),
    } satisfies GeometryBuildFailure) as GeometryBuildFailure
    for (const candidate of this.jobs.values()) {
      if (generation === null || candidate.workerGeneration === generation) candidate.lifecycle = 'terminal'
    }
    this.clearHardRestartTimer()
    this.pendingJobIds.length = 0
    this.supersededWorkerJobs.clear()
    this.destroyWorker()
    this.jobs.clear()
    if (effectiveRecord.request.documentRevision !== this.latestRevision) return
    this.setState({
      status: 'failed',
      documentRevision: effectiveRecord.request.documentRevision,
      jobId: effectiveRecord.request.jobId,
      requestedQuality: effectiveRecord.request.quality,
      phase: event.phase,
      progress: null,
      activeJobIds: [],
      error,
    })
    this.options.onPublish?.(Object.freeze({ ...event, hostElapsedMs: Math.max(0, this.now() - effectiveRecord.requestedAt) }))
  }

  private primaryCurrentJob(): JobRecord | null {
    const candidates = [...this.jobs.values()].filter(record => (
      record.request.documentRevision === this.latestRevision
      && record.lifecycle !== 'terminal'
      && record.lifecycle !== 'superseded'
    ))
    candidates.sort((a, b) => qualityRank(b.request.quality) - qualityRank(a.request.quality) || b.request.jobId - a.request.jobId)
    return candidates[0] ?? null
  }

  private primaryActiveJob(generation: number): JobRecord | null {
    const candidates = [...this.jobs.values()].filter(record => (
      record.workerGeneration === generation
      && record.request.documentRevision === this.latestRevision
      && record.lifecycle !== 'terminal'
      && record.lifecycle !== 'superseded'
    ))
    candidates.sort((a, b) => qualityRank(b.request.quality) - qualityRank(a.request.quality) || b.request.jobId - a.request.jobId)
    return candidates[0] ?? null
  }

  private isCurrentLatest(record: JobRecord) {
    return record.request.documentRevision === this.latestRevision
      && this.latestJobByQuality.get(record.request.quality) === record.request.jobId
  }

  private requestedQuality(): GeometryQuality | null {
    const active = this.currentActiveRecords()
    if (active.some(record => record.request.quality === 'full')) return 'full'
    if (active.some(record => record.request.quality === 'preview')) return 'preview'
    return this.snapshot.requestedQuality
  }

  private requestedJobId(): GeometryJobId | null {
    const quality = this.requestedQuality()
    if (!quality) return null
    const active = this.currentActiveRecords().filter(record => record.request.quality === quality)
    return active.reduce<number | null>((latest, record) => (
      latest === null || record.request.jobId > latest ? record.request.jobId : latest
    ), null) ?? this.snapshot.jobId
  }

  private activeJobIds(exclude?: GeometryJobId): GeometryJobId[] {
    return this.currentActiveRecords(exclude)
      .map(record => record.request.jobId)
      .sort((a, b) => a - b)
  }

  private currentActiveRecords(exclude?: GeometryJobId): JobRecord[] {
    return [...this.jobs.values()].filter(record => (
      record.request.documentRevision === this.latestRevision
      && record.request.jobId !== exclude
      && record.lifecycle !== 'terminal'
      && record.lifecycle !== 'superseded'
      && this.latestJobByQuality.get(record.request.quality) === record.request.jobId
    ))
  }

  private updateAfterTerminal(exclude?: GeometryJobId) {
    const active = this.activeJobIds(exclude)
    const hasPublishedResult = this.snapshot.publishedQuality !== null
    this.setState({
      status: active.length > 0 ? 'building' : hasPublishedResult ? 'ready' : this.snapshot.status,
      jobId: this.requestedJobId(),
      requestedQuality: this.requestedQuality(),
      phase: active.length > 0 ? 'queued' : hasPublishedResult ? 'complete' : this.snapshot.phase,
      progress: hasPublishedResult && active.length === 0 ? 1 : null,
      activeJobIds: active,
      error: hasPublishedResult ? null : this.snapshot.error,
    })
  }

  private updateBuildingState() {
    const quality = this.requestedQuality()
    this.setState({
      status: 'building',
      documentRevision: this.latestRevision,
      jobId: this.requestedJobId(),
      requestedQuality: quality,
      phase: 'queued',
      progress: null,
      activeJobIds: this.activeJobIds(),
      error: null,
    })
  }

  private setState(patch: Partial<BuildCoordinatorState>) {
    const activeJobIds = Object.freeze([...(patch.activeJobIds ?? this.snapshot.activeJobIds)])
    const error = patch.error === undefined ? this.snapshot.error : patch.error
    this.snapshot = Object.freeze({
      ...this.snapshot,
      ...patch,
      activeJobIds,
      error: error ? Object.freeze({ ...error }) : null,
    })
    this.options.onStateChange?.(this.snapshot)
  }

  private clearHardRestartTimer() {
    if (this.hardRestartTimer === null) return
    this.timers.clearTimeout(this.hardRestartTimer)
    this.hardRestartTimer = null
  }
}
