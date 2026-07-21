import { describe, expect, it } from 'vitest'
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
    meshes: [],
    warnings: [],
    volume: request.documentRevision,
    surfaceArea: 0,
    reduced: false,
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
  it('publishes only the latest same-quality job', () => {
    const { coordinator, workers, published } = harness()
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'preview' })
    coordinator.requestBuild({ documentRevision: 1, source: 'cube(1);', quality: 'preview' })

    const [first, second] = buildRequests(workers[0])
    workers[0].emitMessage(success(first))
    expect(published).toEqual([])
    expect(coordinator.state.status).toBe('building')

    workers[0].emitMessage(success(second))
    expect(published.map(result => result.jobId)).toEqual([second.jobId])
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

    workers[0].emitMessage({ ...success(request), ...mismatch })

    expect(workers[0].terminated).toBe(true)
    expect(published).toHaveLength(1)
    expect(published[0]).toMatchObject({
      status: 'failed',
      documentRevision: 3,
      quality: 'preview',
      error: { name: 'ProtocolError', message: expect.stringContaining('correlation mismatch') },
    })
  })
})
