import { describe, expect, it } from 'vitest'
import { planGeometrySourceExecution } from '../src/core/geometryExecution'
import {
  BuildCoordinator,
  type CoordinatorTimers,
  type PublishedGeometryBuild,
  type WorkerLike,
} from '../src/services/buildCoordinator'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  type GeometrySweepPreview,
  type GeometryBuildFailure,
  type GeometryBuildRequest,
  type GeometryBuildSuccess,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/services/geometryWorkerProtocol'
import type { MeshData } from '../src/core/mesh'

class FakeWorker implements WorkerLike {
  readonly messages: GeometryWorkerRequest[] = []
  readonly listeners = new Map<string, Set<EventListenerOrEventListenerObject>>()
  terminated = false

  postMessage(message: GeometryWorkerRequest) {
    if (this.terminated) throw new Error('Worker is terminated')
    this.messages.push(message)
  }

  terminate() {
    this.terminated = true
  }

  addEventListener(type: string, listener: EventListenerOrEventListenerObject) {
    const listeners = this.listeners.get(type) ?? new Set<EventListenerOrEventListenerObject>()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(type: string, listener: EventListenerOrEventListenerObject) {
    this.listeners.get(type)?.delete(listener)
  }

  emitMessage(data: unknown) {
    this.emit('message', { data } as MessageEvent<unknown>)
  }

  emitError(message = 'worker crashed') {
    this.emit('error', { message, error: new Error(message) } as ErrorEvent)
  }

  emitMessageError() {
    this.emit('messageerror', {} as MessageEvent)
  }

  private emit(type: string, event: Event) {
    for (const listener of this.listeners.get(type) ?? []) {
      if (typeof listener === 'function') listener(event)
      else listener.handleEvent(event)
    }
  }
}

class FakeTimers implements CoordinatorTimers {
  private nextId = 1
  private callbacks = new Map<number, { callback: () => void; at: number }>()
  now = 0

  setTimeout(callback: () => void, delayMs: number): unknown {
    const id = this.nextId++
    this.callbacks.set(id, { callback, at: this.now + delayMs })
    return id
  }

  clearTimeout(handle: unknown) {
    this.callbacks.delete(handle as number)
  }

  get size() {
    return this.callbacks.size
  }

  get scheduledCallbacks() {
    return [...this.callbacks.values()].map(timer => timer.callback)
  }

  advance(delayMs: number) {
    const target = this.now + delayMs
    for (;;) {
      const next = [...this.callbacks].filter(([, timer]) => timer.at <= target)
        .sort((a, b) => a[1].at - b[1].at)[0]
      if (!next) break
      this.now = next[1].at
      this.callbacks.delete(next[0])
      next[1].callback()
    }
    this.now = target
  }

  runAll() {
    const callbacks = [...this.callbacks.values()]
    this.callbacks.clear()
    for (const { callback } of callbacks) callback()
  }
}

function buildRequests(worker: FakeWorker): GeometryBuildRequest[] {
  return worker.messages.filter((message): message is GeometryBuildRequest => message.type === 'build')
}

function success(request: GeometryBuildRequest): GeometryBuildSuccess {
  return {
    protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
    status: 'succeeded',
    phase: 'complete',
    documentRevision: request.documentRevision,
    jobId: request.jobId,
    quality: request.quality,
    sourceSha256: request.sourceSha256,
    execution: {
      ...planGeometrySourceExecution('cube(1);', { quality: request.quality, purpose: request.quality }),
      evidence: 'runtime',
    },
    meshes: [],
    warnings: [],
    volume: request.documentRevision,
    surfaceArea: 0,
    reduced: false,
    timings: { parseMs: 1, bindMs: 0, initializeMs: 1, evaluateMs: 2, analyzeMs: 6 },
    durationMs: 10,
  }
}

function accepted(request: GeometryBuildRequest): GeometryWorkerEvent {
  return {
    protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
    status: 'accepted',
    phase: 'queued',
    documentRevision: request.documentRevision,
    jobId: request.jobId,
    quality: request.quality,
    sourceSha256: request.sourceSha256,
  }
}

function emptyMeshWithSourceEnd(end: number): MeshData {
  return {
    entityId: 'entity:span-test',
    vertices: new Float32Array([
      0, 0, 0, 0, 0, 1,
      1, 0, 0, 0, 0, 1,
      0, 1, 0, 0, 0, 1,
    ]),
    indices: new Uint32Array([0, 1, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    faceIds: new Uint32Array([0]),
    color: [1, 1, 1, 1],
    transform: new Float32Array([
      1, 0, 0, 0,
      0, 1, 0, 0,
      0, 0, 1, 0,
      0, 0, 0, 1,
    ]),
    bvh: {
      version: 1,
      vertexStride: 6,
      leafSize: 8,
      nodeCount: 1,
      bounds: new Float32Array([0, 0, 0, 1, 1, 0]),
      nodes: new Uint32Array([0, 0x80000001]),
      triangles: new Uint32Array([0]),
    },
    provenance: [{
      triangleStart: 0,
      triangleEnd: 1,
      backside: false,
      source: { id: 1, originalId: 1, start: 0, end, label: 'span' },
    }],
    topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function harness(options: { grace?: number; timers?: FakeTimers; silence?: number } = {}) {
  const workers: FakeWorker[] = []
  const published: PublishedGeometryBuild[] = []
  const previews:GeometrySweepPreview[]=[]
  let restarts = 0
  const coordinator = new BuildCoordinator({
    workerFactory: () => {
      const worker = new FakeWorker()
      workers.push(worker)
      return worker
    },
    onPublish: outcome => published.push(outcome),
    onSweepPreview:preview=>previews.push(preview),
    supersedeGraceMs: options.grace,
    workerSilenceTimeoutMs: options.silence,
    onWorkerRestart: () => { restarts++ },
    timers: options.timers,
    now: () => options.timers?.now ?? 100,
  })
  return { coordinator, workers, published, previews, get restarts() { return restarts } }
}

describe('BuildCoordinator', () => {
  it('coalesces repeated exact build keys without growing the worker queue', () => {
    const { coordinator, workers, published } = harness()
    const jobIds = Array.from({ length: 100 }, () => (
      coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'preview' })
    ))

    expect(new Set(jobIds)).toEqual(new Set([jobIds[0]]))
    const [onlyRequest] = buildRequests(workers[0])
    expect(buildRequests(workers[0])).toHaveLength(1)

    workers[0].emitMessage(success(onlyRequest))
    expect(published.map(result => result.jobId)).toEqual([onlyRequest.jobId])
    expect(coordinator.state).toMatchObject({ status: 'ready', documentRevision: 1, publishedQuality: 'preview' })
  })

  it('keeps a warm worker for preview then full and publishes both in useful order', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 4, source: 'sphere(10);', quality: 'preview' })
    coordinator.requestBuild({ documentRevision: 4, source: 'sphere(10);', quality: 'full' })

    expect(workers).toHaveLength(1)
    expect(workers[0].terminated).toBe(false)
    const [preview, full] = buildRequests(workers[0])

    workers[0].emitMessage(accepted(preview))
    workers[0].emitMessage(success(preview))
    expect(published.map(result => result.quality)).toEqual(['preview'])
    expect(coordinator.state).toMatchObject({ status: 'building', requestedQuality: 'full', publishedQuality: 'preview' })

    workers[0].emitMessage(success(full))
    expect(published.map(result => result.quality)).toEqual(['preview', 'full'])
    expect(coordinator.state).toMatchObject({ status: 'ready', requestedQuality: 'full', publishedQuality: 'full' })
  })

  it('never lets a late preview downgrade an already published full result', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 2, source: 'sphere(2);', quality: 'preview' })
    coordinator.requestBuild({ documentRevision: 2, source: 'sphere(2);', quality: 'full' })
    const [preview, full] = buildRequests(workers[0])

    workers[0].emitMessage(success(full))
    workers[0].emitMessage(success(preview))

    expect(published.map(result => result.quality)).toEqual(['full'])
    expect(coordinator.state).toMatchObject({ status: 'ready', publishedQuality: 'full' })
  })

  it('discards an older revision and can reuse a worker that finishes during the grace period', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ grace: 50, timers })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'preview' })
    const oldRequest = buildRequests(workers[0])[0]
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'preview' })

    expect(timers.size).toBe(1)
    expect(buildRequests(workers[0])).toHaveLength(1)
    workers[0].emitMessage(success(oldRequest))

    expect(published).toEqual([])
    expect(timers.size).toBe(0)
    expect(workers).toHaveLength(1)
    expect(workers[0].terminated).toBe(false)
    const currentRequest = buildRequests(workers[0])[1]
    expect(currentRequest.documentRevision).toBe(2)

    workers[0].emitMessage(success(currentRequest))
    expect(published.map(result => result.documentRevision)).toEqual([2])
  })

  it('hard-restarts a busy synchronous worker when supersede grace expires', () => {
    const timers = new FakeTimers()
    const { coordinator, workers } = harness({ grace: 25, timers })
    coordinator.requestBuild({ documentRevision: 10, source: 'cube(10);', quality: 'full' })
    coordinator.requestBuild({ documentRevision: 11, source: 'cube(11);', quality: 'preview' })

    expect(workers[0].messages.some(message => message.type === 'cancel')).toBe(true)
    expect(buildRequests(workers[0])).toHaveLength(1)
    timers.runAll()

    expect(workers[0].terminated).toBe(true)
    expect(workers).toHaveLength(2)
    expect(buildRequests(workers[1])[0]).toMatchObject({ documentRevision: 11, quality: 'preview' })
    expect(coordinator.state).toMatchObject({ status: 'building', documentRevision: 11 })
  })

  it('turns a worker error into a current failure and recovers on the next revision', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'full' })
    workers[0].emitError('GPU process ended')

    expect(workers[0].terminated).toBe(true)
    expect(coordinator.state).toMatchObject({ status: 'failed', documentRevision: 1 })
    expect((published[0] as GeometryBuildFailure).error.message).toBe('GPU process ended')

    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'full' })
    expect(workers).toHaveLength(2)
    const request = buildRequests(workers[1])[0]
    workers[1].emitMessage(success(request))
    expect(published.map(result => result.status)).toEqual(['failed', 'succeeded'])
    expect(coordinator.state.status).toBe('ready')
  })

  it('represents a terminal stale response without publishing it', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 8, source: 'cube(8);', quality: 'preview' })
    const request = buildRequests(workers[0])[0]
    workers[0].emitMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      status: 'stale',
      phase: 'compiling',
      documentRevision: request.documentRevision,
      jobId: request.jobId,
      quality: request.quality,
      sourceSha256: request.sourceSha256,
      durationMs: 4,
    } satisfies GeometryWorkerEvent)

    expect(published).toEqual([])
    expect(coordinator.state).toMatchObject({ status: 'stale', activeJobIds: [] })
  })

  it('rejects revision aliasing and messages from a mismatched protocol', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 3, source: 'cube(3);', quality: 'preview' })
    expect(() => coordinator.requestBuild({ documentRevision: 3, source: 'cube(4);', quality: 'full' }))
      .toThrow(/exactly one source snapshot/)

    workers[0].emitMessage({ id: 1, ok: true })
    expect(workers[0].terminated).toBe(true)
    expect(published[0]).toMatchObject({ status: 'failed', error: { name: 'ProtocolError' } })
  })

  it.each([
    ['document revision', { documentRevision: 4 }],
    ['quality', { quality: 'full' as const }],
  ])('rejects a valid event whose %s does not correlate with its job', (_label, mismatch) => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 3, source: 'cube(3);', quality: 'preview' })
    const request = buildRequests(workers[0])[0]

    const event = success(request)
    if (mismatch.quality) {
      event.execution = {
        ...event.execution,
        purpose: mismatch.quality,
        quality: mismatch.quality,
      }
    }
    workers[0].emitMessage({ ...event, ...mismatch })

    expect(workers[0].terminated).toBe(true)
    expect(published).toHaveLength(1)
    expect(published[0]).toMatchObject({
      status: 'failed',
      documentRevision: 3,
      quality: 'preview',
      error: { name: 'ProtocolError', message: expect.stringContaining('correlation mismatch') },
    })
  })

  it('rejects worker provenance that does not match the requested source route', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({
      documentRevision: 9,
      source: '// @requires geometry.mesh\ncube(1);',
      quality: 'full',
    })
    const request = buildRequests(workers[0])[0]
    workers[0].emitMessage(success(request))

    expect(workers[0].terminated).toBe(true)
    expect(published).toHaveLength(1)
    expect(published[0]).toMatchObject({
      status: 'failed',
      error: { name: 'ProtocolError', message: expect.stringContaining('provenance mismatch') },
    })
  })

  it('rejects a same-route terminal replayed for different exact source bytes', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'full' })
    const first = buildRequests(workers[0])[0]
    const replay = success(first)
    workers[0].emitMessage(replay)
    expect(published.at(-1)?.status).toBe('succeeded')

    coordinator.requestBuild({ documentRevision: 2, source: 'sphere(1);', quality: 'full' })
    const second = buildRequests(workers[0]).at(-1)!
    workers[0].emitMessage({
      ...replay,
      documentRevision: second.documentRevision,
      jobId: second.jobId,
    })

    expect(workers[0].terminated).toBe(true)
    expect(published.at(-1)).toMatchObject({
      status: 'failed',
      error: { name: 'ProtocolError', message: expect.stringContaining('source attestation mismatch') },
    })
  })

  it('rejects provenance spans outside the exact attested source', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 5, source: 'cube(1);', quality: 'full' })
    const request = buildRequests(workers[0])[0]
    workers[0].emitMessage({
      ...success(request),
      meshes: [emptyMeshWithSourceEnd(request.source.length + 1)],
    })

    expect(workers[0].terminated).toBe(true)
    expect(published.at(-1)).toMatchObject({
      status: 'failed',
      error: { name: 'ProtocolError', message: expect.stringContaining('source span') },
    })
  })

  it('rejects failed diagnostic spans outside the exact attested source', () => {
    const { coordinator, workers, published } = harness()
    const source = 'cube(1);'
    coordinator.requestBuild({ documentRevision: 6, source, quality: 'full' })
    const request = buildRequests(workers[0])[0]
    const failed: GeometryBuildFailure = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      status: 'failed',
      phase: 'compiling',
      documentRevision: request.documentRevision,
      jobId: request.jobId,
      quality: request.quality,
      sourceSha256: request.sourceSha256,
      execution: success(request).execution,
      error: { name: 'ParseError', message: 'bad span', start: 0, end: source.length + 1 },
      durationMs: 1,
    }
    workers[0].emitMessage(failed)

    expect(workers[0].terminated).toBe(true)
    expect(published.at(-1)).toMatchObject({
      status: 'failed',
      error: { name: 'ProtocolError', message: expect.stringContaining('source span') },
    })
  })

  it('rejects source spans that split a UTF-16 surrogate pair', () => {
    const { coordinator, workers, published } = harness()
    const source = '🙂'
    coordinator.requestBuild({ documentRevision: 61, source, quality: 'full' })
    const request = buildRequests(workers[0])[0]
    workers[0].emitMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      status: 'failed',
      phase: 'compiling',
      documentRevision: request.documentRevision,
      jobId: request.jobId,
      quality: request.quality,
      sourceSha256: request.sourceSha256,
      execution: success(request).execution,
      error: { name: 'ParseError', message: 'split surrogate', start: 1, end: 1 },
      durationMs: 1,
    } satisfies GeometryBuildFailure)

    expect(workers[0].terminated).toBe(true)
    expect(published.at(-1)).toMatchObject({
      status: 'failed',
      error: { name: 'ProtocolError', message: expect.stringContaining('source span') },
    })
  })

  it('publishes a deeply frozen snapshot with exclusively owned mesh buffers', () => {
    const { coordinator, workers, published } = harness()
    const source = 'cube(1);'
    coordinator.requestBuild({ documentRevision: 62, source, quality: 'full' })
    const request = buildRequests(workers[0])[0]
    const originalMesh = emptyMeshWithSourceEnd(source.length)
    const terminal = { ...success(request), meshes: [originalMesh] }
    workers[0].emitMessage(terminal)

    const outcome = published.at(-1) as GeometryBuildSuccess
    expect(outcome.status).toBe('succeeded')
    expect(outcome.meshes[0].vertices.buffer).not.toBe(originalMesh.vertices.buffer)
    expect(Object.isFrozen(outcome)).toBe(true)
    expect(Object.isFrozen(outcome.meshes)).toBe(true)
    expect(Object.isFrozen(outcome.meshes[0])).toBe(true)
    expect(Object.isFrozen(outcome.meshes[0].bvh)).toBe(true)
    expect(Object.isFrozen(outcome.meshes[0].provenance)).toBe(true)
    expect(Object.isFrozen(outcome.meshes[0].provenance[0].source)).toBe(true)
    originalMesh.vertices[0] = 99
    originalMesh.provenance[0].source!.label = 'mutated'
    expect(outcome.meshes[0].vertices[0]).toBe(0)
    expect(outcome.meshes[0].provenance[0].source?.label).toBe('span')
  })

  it('rejects accessor-based terminal payloads without evaluating the accessor', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 63, source: 'cube(1);', quality: 'full' })
    const request = buildRequests(workers[0])[0]
    const terminal = success(request) as GeometryBuildSuccess & Record<string, unknown>
    let reads = 0
    Object.defineProperty(terminal, 'durationMs', {
      enumerable: true,
      get() {
        reads++
        return reads === 1 ? 1 : -1
      },
    })
    workers[0].emitMessage(terminal)

    expect(reads).toBe(0)
    expect(workers[0].terminated).toBe(true)
    expect(published.at(-1)).toMatchObject({ status: 'failed', error: { name: 'ProtocolError' } })
  })

  it('accepts an exact EOF diagnostic span and re-freezes execution provenance', () => {
    const { coordinator, workers, published } = harness()
    const source = 'cube(1);'
    coordinator.requestBuild({ documentRevision: 7, source, quality: 'full' })
    const request = buildRequests(workers[0])[0]
    const failed: GeometryBuildFailure = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      status: 'failed',
      phase: 'compiling',
      documentRevision: request.documentRevision,
      jobId: request.jobId,
      quality: request.quality,
      sourceSha256: request.sourceSha256,
      execution: success(request).execution,
      error: {
        name: 'OpenSCADParseError', message: 'at EOF', start: source.length, end: source.length,
      },
      durationMs: 1,
    }
    workers[0].emitMessage(failed)

    const outcome = published.at(-1)!
    expect(outcome).toMatchObject({ status: 'failed', error: { message: 'at EOF' } })
    expect(Object.isFrozen(outcome)).toBe(true)
    expect(Object.isFrozen((outcome as GeometryBuildFailure).error)).toBe(true)
    expect(Object.isFrozen(outcome.execution)).toBe(true)
    expect(Object.isFrozen(outcome.execution?.requiredCapabilities)).toBe(true)
    expect(Object.isFrozen(outcome.execution?.effectiveLimits)).toBe(true)
    expect(() => {
      ;(outcome as GeometryBuildFailure).execution = {
        ...success(request).execution,
        engineClass: 'brep',
      }
    }).toThrow(TypeError)
  })
})


describe('coordinator measurement boundaries', () => {
  it('measures main-clock latency independently of Worker time and counts coalesced requests once', () => {
    const worker = new FakeWorker()
    let now = 100
    let result: PublishedGeometryBuild | undefined
    const coordinator = new BuildCoordinator({ workerFactory: () => worker, now: () => now, onPublish: event => { result = event } })
    const input = { documentRevision: 1, source: 'cube(1);', quality: 'preview' as const }
    const job = coordinator.requestBuild(input)
    expect(coordinator.requestBuild(input)).toBe(job)
    now = 145
    worker.emitMessage(success(buildRequests(worker)[0]))
    expect(result).toMatchObject({ hostElapsedMs: 45, durationMs: 10 })
    expect(coordinator.diagnostics).toEqual({ builds: 1, superseded: 0, workerStarts: 1, hardRestarts: 0 })
    coordinator.dispose()
  })

  it('counts supersession and hard restarts without counting disposal as a restart', () => {
    const { coordinator } = harness()
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'preview' })
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'preview' })
    expect(coordinator.diagnostics).toEqual({ builds: 2, superseded: 1, workerStarts: 2, hardRestarts: 1 })
    coordinator.dispose()
    expect(coordinator.diagnostics.hardRestarts).toBe(1)
  })
})

describe('coordinator Worker silence recovery', () => {
  it.each([0, -1, NaN, Infinity])('rejects invalid silence timeout %s', silence => {
    expect(() => harness({ silence })).toThrow(/workerSilenceTimeoutMs must be positive/)
  })

  it('retries a silent startup once with exact request identity and original elapsed time', () => {
    const timers = new FakeTimers()
    const h = harness({ timers, silence: 100 })
    const jobId = h.coordinator.requestBuild({ documentRevision: 5, source: 'cube(5);', quality: 'full' })
    const original = buildRequests(h.workers[0])[0]
    const oldMessage = [...h.workers[0].listeners.get('message')!][0] as EventListener
    timers.advance(100)
    expect(h.workers[0].terminated).toBe(true)
    expect(h.workers[0].listeners.get('messageerror')?.size).toBe(0)
    expect(buildRequests(h.workers[1])).toEqual([original])
    expect(h.coordinator.diagnostics).toEqual({ builds: 1, superseded: 0, workerStarts: 2, hardRestarts: 1 })
    expect(h.restarts).toBe(1)
    oldMessage({ data: success(original) } as MessageEvent)
    expect(h.published).toEqual([])
    timers.advance(30)
    h.workers[1].emitMessage(success(original))
    expect(h.published).toHaveLength(1)
    expect(h.published[0]).toMatchObject({ jobId, hostElapsedMs: 130 })
    expect(h.coordinator.state.status).toBe('ready')
    expect(timers.size).toBe(0)
  })

  it('fails after the retry stays silent and gives a new manual request its own allowance', () => {
    const timers = new FakeTimers()
    const h = harness({ timers, silence: 100 })
    const input = { documentRevision: 1, source: 'cube(1);', quality: 'full' as const }
    const first = h.coordinator.requestBuild(input)
    timers.advance(200)
    expect(h.workers).toHaveLength(2)
    expect(h.workers.every(worker => worker.terminated)).toBe(true)
    expect(h.published).toHaveLength(1)
    expect(h.published[0]).toMatchObject({ jobId: first, status: 'failed', hostElapsedMs: 200,
      error: { name: 'TimeoutError', code: 'WORKER_TIMEOUT' } })
    expect(h.coordinator.state).toMatchObject({ status: 'failed', activeJobIds: [] })
    expect(timers.size).toBe(0)
    const manual = h.coordinator.requestBuild(input)
    expect(manual).not.toBe(first)
    timers.advance(100)
    expect(h.workers).toHaveLength(4)
    h.workers[3].emitMessage(success(buildRequests(h.workers[3])[0]))
    expect(h.coordinator.state.status).toBe('ready')
    expect(h.restarts).toBe(2)
  })

  it('refreshes silence only for attested live messages and fences already queued timeout callbacks', () => {
    const timers = new FakeTimers()
    const { coordinator, workers } = harness({ timers, silence: 100 })
    const input = { documentRevision: 1, source: 'cube(1);', quality: 'preview' as const }
    coordinator.requestBuild(input)
    const request = buildRequests(workers[0])[0]
    const staleTimeout = timers.scheduledCallbacks[0]
    timers.advance(60)
    workers[0].emitMessage({ ...accepted(request), status: 'progress', phase: 'compiling', progress: null })
    staleTimeout()
    expect(workers).toHaveLength(1)
    timers.advance(60)
    coordinator.requestBuild(input)
    workers[0].emitMessage(accepted({ ...request, jobId: 999 }))
    timers.advance(39)
    expect(workers).toHaveLength(1)
    timers.advance(1)
    expect(workers).toHaveLength(2)
    coordinator.dispose()
  })

  it('retries concurrent preview/full together and cannot publish a late preview downgrade', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100 })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'preview' })
    timers.advance(60)
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'full' })
    const requests = buildRequests(workers[0])
    timers.advance(40)
    expect(buildRequests(workers[1])).toEqual(requests)
    expect(coordinator.state.activeJobIds).toEqual(requests.map(request => request.jobId))
    workers[1].emitMessage(success(requests[1]))
    workers[1].emitMessage(success(requests[0]))
    expect(published.map(event => event.quality)).toEqual(['full'])
    expect(coordinator.state).toMatchObject({ status: 'ready', publishedQuality: 'full' })
    expect(timers.size).toBe(0)
  })

  it('gives a later full build its own retry without retrying an exhausted preview again', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100 })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'preview' })
    timers.advance(100)
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'full' })
    const full = buildRequests(workers[1])[1]
    timers.advance(100)
    expect(buildRequests(workers[2])).toEqual([full])
    workers[2].emitMessage(success(full))
    expect(published.map(event => event.quality)).toEqual(['full'])
    expect(coordinator.state.status).toBe('ready')
  })

  it('preserves a published full result when a leftover preview goes silent', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100 })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'preview' })
    const fullId = coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'full' })
    const full = buildRequests(workers[0])[1]
    workers[0].emitMessage(success(full))
    timers.advance(200)
    expect(workers).toHaveLength(1)
    expect(workers[0].terminated).toBe(true)
    expect(published).toHaveLength(1)
    expect(published[0]).toMatchObject({ status: 'succeeded', quality: 'full' })
    expect(coordinator.state).toMatchObject({ status: 'ready', requestedQuality: 'full',
      publishedQuality: 'full', jobId: fullId, activeJobIds: [], error: null })
    expect(timers.size).toBe(0)
  })

  it.each(['error', 'messageerror', 'protocol'] as const)('preserves full geometry when the leftover preview Worker has a %s failure', failure => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100 })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'preview' })
    const fullId = coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'full' })
    workers[0].emitMessage(success(buildRequests(workers[0])[1]))
    if (failure === 'error') workers[0].emitError()
    else if (failure === 'messageerror') workers[0].emitMessageError()
    else workers[0].emitMessage({ invalid: 'protocol' })
    expect(workers[0].terminated).toBe(true)
    expect(published).toHaveLength(1)
    expect(published[0]).toMatchObject({ status: 'succeeded', quality: 'full' })
    expect(coordinator.state).toMatchObject({ status: 'ready', requestedQuality: 'full',
      publishedQuality: 'full', jobId: fullId, activeJobIds: [], error: null })
    expect(timers.size).toBe(0)
  })

  it('does not replenish the full retry allowance when preview succeeds', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100 })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'preview' })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'full' })
    timers.advance(100)
    const [preview, full] = buildRequests(workers[1])
    timers.advance(30)
    workers[1].emitMessage(success(preview))
    expect(coordinator.state).toMatchObject({ status: 'building', publishedQuality: 'preview' })
    timers.advance(100)
    expect(workers).toHaveLength(2)
    expect(published).toHaveLength(2)
    expect(published[1]).toMatchObject({ jobId: full.jobId, quality: 'full', status: 'failed',
      hostElapsedMs: 230, error: { code: 'WORKER_TIMEOUT' } })
    expect(timers.size).toBe(0)
  })

  it('leaves supersession to its grace timer and resets the retry budget on a new revision', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100, grace: 50 })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'full' })
    timers.advance(100)
    const staleSilence = timers.scheduledCallbacks[0]
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'full' })
    staleSilence()
    expect(workers).toHaveLength(2)
    expect(timers.size).toBe(1)
    timers.advance(50)
    expect(workers).toHaveLength(3)
    timers.advance(100)
    expect(workers).toHaveLength(4)
    expect(published).toEqual([])
    workers[3].emitMessage(success(buildRequests(workers[3])[0]))
    expect(coordinator.state).toMatchObject({ status: 'ready', documentRevision: 2 })
  })

  it('ignores a stale grace callback after a warm worker accepts newer work', () => {
    const timers = new FakeTimers()
    const { coordinator, workers } = harness({ timers, silence: 100, grace: 50 })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'preview' })
    const old = buildRequests(workers[0])[0]
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'preview' })
    const staleGrace = timers.scheduledCallbacks[0]
    workers[0].emitMessage(success(old))
    staleGrace()
    expect(workers).toHaveLength(1)
    expect(workers[0].terminated).toBe(false)
    workers[0].emitMessage(success(buildRequests(workers[0])[1]))
    expect(coordinator.state.status).toBe('ready')
    expect(timers.size).toBe(0)
  })

  it.each(['cancel', 'dispose'] as const)('clears and fences every timeout on %s', operation => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100, grace: 50 })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'full' })
    const staleSilence = timers.scheduledCallbacks[0]
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'full' })
    const staleGrace = timers.scheduledCallbacks[0]
    coordinator[operation]()
    staleSilence()
    staleGrace()
    expect(timers.size).toBe(0)
    expect(workers).toHaveLength(1)
    expect(workers[0].terminated).toBe(true)
    expect(published).toEqual([])
    expect(coordinator.state.status).toBe(operation === 'dispose' ? 'disposed' : 'cancelled')
  })

  it('settles message deserialization failure immediately and recovers in a new Worker', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100 })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'full' })
    workers[0].emitMessageError()
    expect(workers[0].terminated).toBe(true)
    expect(workers[0].listeners.get('messageerror')?.size).toBe(0)
    expect(published[0]).toMatchObject({ status: 'failed', error: { message: 'Could not deserialize a geometry Worker message' } })
    expect(timers.size).toBe(0)
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'full' })
    workers[1].emitMessage(success(buildRequests(workers[1])[0]))
    expect(coordinator.state.status).toBe('ready')
  })

  it('recovers queued superseding work when the old Worker has a messageerror', () => {
    const timers = new FakeTimers()
    const { coordinator, workers, published } = harness({ timers, silence: 100, grace: 50 })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'full' })
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'preview' })
    workers[0].emitMessageError()
    expect(workers[0].terminated).toBe(true)
    expect(buildRequests(workers[1])[0]).toMatchObject({ documentRevision: 2, quality: 'preview' })
    expect(published).toEqual([])
    workers[1].emitMessage(success(buildRequests(workers[1])[0]))
    expect(coordinator.state).toMatchObject({ status: 'ready', documentRevision: 2 })
    expect(timers.size).toBe(0)
  })

  it.each(['factory', 'post'] as const)('settles a retry %s failure without resurrecting a sibling job', mode => {
    const timers = new FakeTimers()
    const workers: FakeWorker[] = []
    const published: PublishedGeometryBuild[] = []
    let factories = 0
    const coordinator = new BuildCoordinator({
      workerSilenceTimeoutMs: 100,
      timers,
      workerFactory: () => {
        factories++
        if (factories === 2 && mode === 'factory') throw new Error('retry factory failed')
        const worker = new FakeWorker()
        if (factories === 2) worker.postMessage = () => { throw new Error('retry post failed') }
        workers.push(worker)
        return worker
      },
      onPublish: event => published.push(event),
    })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'preview' })
    coordinator.requestBuild({ documentRevision: 1, source: 'sphere(1);', quality: 'full' })
    timers.advance(100)
    expect(factories).toBe(2)
    expect(workers.every(worker => worker.terminated)).toBe(true)
    expect(published).toHaveLength(1)
    expect(published[0]).toMatchObject({ status: 'failed', quality: 'full',
      error: { message: `retry ${mode} failed` } })
    expect(coordinator.state).toMatchObject({ status: 'failed', activeJobIds: [] })
    expect(timers.size).toBe(0)
    coordinator.requestBuild({ documentRevision: 2, source: 'cube(2);', quality: 'full' })
    const recovery = workers.at(-1)!
    recovery.emitMessage(success(buildRequests(recovery)[0]))
    expect(coordinator.state.status).toBe('ready')
  })
})


it('routes current sweep previews separately and drops stale or superseded previews',()=>{
 const {coordinator,workers,published,previews}=harness({grace:300})
 coordinator.requestBuild({documentRevision:1,source:'cube(1);',quality:'full'})
 const first=workers[0]!.messages[0] as GeometryBuildRequest
 const preview=(request:GeometryBuildRequest)=>({...accepted(request),status:'sweep-preview',phase:'compiling',
  nodeId:'sweep',sections:5,accepted:false,sampledControlDeviation:.1,budget:.01,meshes:[]})
 workers[0]!.emitMessage(preview(first))
 expect(first.acknowledgeSweepPreviews).toBe(true)
 expect(workers[0]!.messages.at(-1)).toMatchObject({type:'sweep-preview-ack',jobId:first.jobId,sourceSha256:first.sourceSha256,nodeId:'sweep',sections:5})
 expect(previews).toHaveLength(1)
 expect(published).toHaveLength(0)
 expect(coordinator.state.publishedQuality).toBeNull()
 coordinator.requestBuild({documentRevision:2,source:'cube(2);',quality:'full'})
 workers[0]!.emitMessage(preview(first))
 expect(previews).toHaveLength(1)
 coordinator.cancel('user')
 workers[0]!.emitMessage(preview(first))
 expect(previews).toHaveLength(1)
})
