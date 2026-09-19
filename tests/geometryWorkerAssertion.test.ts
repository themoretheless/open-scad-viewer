import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  type GeometryBuildRequest,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/services/geometryWorkerProtocol'
import { sha256Hex } from '../src/core/sha256'

class IntegrationWorkerScope {
  readonly events: GeometryWorkerEvent[] = []
  private readonly listeners = new Map<string, Set<EventListenerOrEventListenerObject>>()

  postMessage(event: GeometryWorkerEvent) {
    this.events.push(event)
  }

  addEventListener(type: string, listener: EventListenerOrEventListenerObject) {
    const listeners = this.listeners.get(type) ?? new Set<EventListenerOrEventListenerObject>()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  dispatchMessage(data: GeometryWorkerRequest) {
    const event = { data } as MessageEvent<unknown>
    for (const listener of this.listeners.get('message') ?? []) {
      if (typeof listener === 'function') listener(event)
      else listener.handleEvent(event)
    }
  }
}

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('geometry Worker assertion integration', () => {
  it('bounds hung initialization and ignores its late rejection', async () => {
    const scope = new IntegrationWorkerScope()
    vi.stubGlobal('self', scope)
    const {defaultGeometryKernel} = await import('../src/services/cadGeometryKernel')
    let rejectWarm!: (error: Error) => void
    vi.spyOn(defaultGeometryKernel, 'warm').mockImplementation(() => new Promise((_resolve, reject) => { rejectWarm = reject }))
    await import('../src/workers/geometry.worker')
    vi.useFakeTimers()
    const source = 'cube(1);'
    scope.dispatchMessage({protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION, type: 'build',
      documentRevision: 1, jobId: 1, source, sourceSha256: sha256Hex(source), quality: 'full'})
    await vi.advanceTimersByTimeAsync(4999)
    expect(scope.events.map(event => event.status)).toEqual(['accepted', 'started'])
    await vi.advanceTimersByTimeAsync(1)
    expect(scope.events.at(-1)).toMatchObject({status: 'failed', phase: 'initializing',
      error: {name: 'GeometryEngineUnavailableError', code: 'ENGINE_UNAVAILABLE',
        message: expect.stringContaining('initialization check exceeded 5000 ms')}})
    rejectWarm(new Error('late initialization failure'))
    await vi.advanceTimersByTimeAsync(0)
    expect(scope.events.map(event => event.status)).toEqual(['accepted', 'started', 'failed'])
    expect(vi.getTimerCount()).toBe(0)
  })

  it.each([0, 350])('carries a real parser assertion through one terminal event after %i ms cold startup', async delayMs => {
    const scope = new IntegrationWorkerScope()
    vi.stubGlobal('self', scope)
    const {defaultGeometryKernel} = await import('../src/services/cadGeometryKernel')
    const warm = defaultGeometryKernel.warm.bind(defaultGeometryKernel)
    let pending: ReturnType<typeof warm> | undefined
    vi.spyOn(defaultGeometryKernel, 'warm').mockImplementation(() => pending ??= new Promise<void>(resolve => setTimeout(resolve, delayMs)).then(warm))
    await import('../src/workers/geometry.worker')

    const source = 'cube(1);\nassert(false, "invalid model") sphere(1);'
    const request: GeometryBuildRequest = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 13,
      jobId: 21,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    }
    scope.dispatchMessage(request)

    await vi.waitFor(() => {
      expect(scope.events.at(-1)?.status).toBe('failed')
    }, { timeout: 10_000 })

    expect(scope.events.map(event => event.status)).toEqual([
      'accepted', 'started', 'progress', 'failed',
    ])
    expect(scope.events.at(-1)).toMatchObject({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      documentRevision: 13,
      jobId: 21,
      quality: 'full',
      status: 'failed',
      phase: 'compiling',
      error: {
        name: 'OpenSCADParseError',
        message: expect.stringContaining("Assertion 'false' failed: invalid model"),
        line: 2,
        column: 1,
      },
    })
    expect(scope.events.filter(event => ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status)))
      .toHaveLength(1)
  })
})
