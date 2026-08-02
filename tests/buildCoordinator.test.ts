import { describe, expect, it } from 'vitest'
import { LEGACY_MANIFOLD_EXECUTION } from '../src/core/geometryExecution'
import {
  BuildCoordinator,
  type CoordinatorTimers,
  type PublishedGeometryBuild,
  type WorkerLike,
} from '../src/services/buildCoordinator'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
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

  private emit(type: string, event: Event) {
    for (const listener of this.listeners.get(type) ?? []) {
      if (typeof listener === 'function') listener(event)
      else listener.handleEvent(event)
    }
  }
}

class FakeTimers implements CoordinatorTimers {
  private nextId = 1
  private callbacks = new Map<number, () => void>()

  setTimeout(callback: () => void, _delayMs: number): unknown {
    const id = this.nextId++
    this.callbacks.set(id, callback)
    return id
  }

  clearTimeout(handle: unknown) {
    this.callbacks.delete(handle as number)
  }

  get size() {
    return this.callbacks.size
  }

  runAll() {
    const callbacks = [...this.callbacks.values()]
    this.callbacks.clear()
    for (const callback of callbacks) callback()
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
      ...LEGACY_MANIFOLD_EXECUTION,
      purpose: request.quality,
      quality: request.quality,
      evidence: 'runtime',
      effectiveLimits: { sourceCharacters: 250_000, triangles: 750_000 },
    },
    meshes: [],
    warnings: [],
    volume: request.documentRevision,
    surfaceArea: 0,
    reduced: false,
    timings: { parseMs: 1, initializeMs: 1, evaluateMs: 2, analyzeMs: 6 },
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

function harness(options: { grace?: number; timers?: FakeTimers } = {}) {
  const workers: FakeWorker[] = []
  const published: PublishedGeometryBuild[] = []
  const coordinator = new BuildCoordinator({
    workerFactory: () => {
      const worker = new FakeWorker()
      workers.push(worker)
      return worker
    },
    onPublish: outcome => published.push(outcome),
    supersedeGraceMs: options.grace,
    timers: options.timers,
    now: () => 100,
  })
  return { coordinator, workers, published }
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
