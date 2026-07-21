import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  type GeometryBuildRequest,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/services/geometryWorkerProtocol'

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
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('geometry Worker assertion integration', () => {
  it('carries a real parser assertion failure through one correlated terminal event', async () => {
    const scope = new IntegrationWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    const source = 'cube(1);\nassert(false, "invalid model") sphere(1);'
    const request: GeometryBuildRequest = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 13,
      jobId: 21,
      source,
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
