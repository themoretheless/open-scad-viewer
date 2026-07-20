import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  type GeometryBuildRequest,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/services/geometryWorkerProtocol'

const parseOpenSCADMock = vi.hoisted(() => vi.fn())

vi.mock('../src/services/openscadParser', async importOriginal => {
  const original = await importOriginal<typeof import('../src/services/openscadParser')>()
  return { ...original, parseOpenSCAD: parseOpenSCADMock }
})

class FakeWorkerScope {
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
  parseOpenSCADMock.mockReset()
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('geometry Worker cancellation', () => {
  it('does not report a cold-initialization cancellation until runBuild reaches a checkpoint', async () => {
    let finishInitialization!: (result: {
      meshes: []
      warnings: []
      volume: number
      surfaceArea: number
      quality: 'preview'
    }) => void
    const pendingInitialization = new Promise<Parameters<typeof finishInitialization>[0]>(resolve => {
      finishInitialization = resolve
    })
    parseOpenSCADMock.mockReturnValue(pendingInitialization)

    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    const build: GeometryBuildRequest = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source: 'cube(1);',
      quality: 'preview',
    }
    scope.dispatchMessage(build)
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'cancel',
      documentRevision: 1,
      jobId: 1,
      reason: 'superseded',
    })

    expect(scope.events.map(event => event.status)).toEqual(['accepted', 'started', 'progress'])

    finishInitialization({
      meshes: [],
      warnings: [],
      volume: 0,
      surfaceArea: 0,
      quality: 'preview',
    })
    await vi.waitFor(() => {
      expect(scope.events.map(event => event.status)).toEqual(['accepted', 'started', 'progress', 'cancelled'])
    })
    expect(scope.events.at(-1)).toMatchObject({
      status: 'cancelled',
      phase: 'compiling',
      reason: 'superseded',
    })
  })
})
