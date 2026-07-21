import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  type GeometryBuildRequest,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/services/geometryWorkerProtocol'

const parseOpenSCADMock = vi.hoisted(() => vi.fn())
// The worker eagerly warms the WASM at startup; stub the loader so this unit
// test does not download/compile Manifold for every vi.resetModules() cycle.
const getWasmMock = vi.hoisted(() => vi.fn(() => Promise.resolve({})))

vi.mock('../src/services/openscadParser', async importOriginal => {
  const original = await importOriginal<typeof import('../src/services/openscadParser')>()
  return { ...original, parseOpenSCAD: parseOpenSCADMock, getWasm: getWasmMock }
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

  it('stops a running parse cooperatively when a cancel message arrives mid-build', async () => {
    const { AbortedError } = await import('../src/services/openscadParser')
    parseOpenSCADMock.mockImplementation(async (
      _source: string,
      options?: { shouldAbort?: () => boolean },
    ) => {
      // Simulate the parser's cooperative yield loop: spin macrotasks so the
      // queued cancel message is delivered, then honor shouldAbort.
      for (let spins = 0; spins < 200; spins++) {
        await new Promise(resolve => setTimeout(resolve, 0))
        if (options?.shouldAbort?.()) throw new AbortedError()
      }
      throw new Error('the cancel request never became visible to shouldAbort')
    })

    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source: 'cube(1);',
      quality: 'full',
    })
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'cancel',
      documentRevision: 1,
      jobId: 1,
      reason: 'user',
    })

    await vi.waitFor(() => {
      expect(scope.events.at(-1)).toMatchObject({
        status: 'cancelled',
        phase: 'compiling',
        reason: 'user',
        jobId: 1,
      })
    })
  })

  it('supersedes a running parse cooperatively when a newer build arrives', async () => {
    const { AbortedError } = await import('../src/services/openscadParser')
    parseOpenSCADMock.mockImplementation(async (
      source: string,
      options?: { shouldAbort?: () => boolean },
    ) => {
      for (let spins = 0; spins < 200; spins++) {
        await new Promise(resolve => setTimeout(resolve, 0))
        if (options?.shouldAbort?.()) throw new AbortedError()
      }
      return {
        meshes: [],
        warnings: [],
        volume: source.length,
        surfaceArea: 0,
        quality: 'full' as const,
        reduced: false,
      }
    })

    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source: 'cube(1);',
      quality: 'full',
    })
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 2,
      jobId: 2,
      source: 'cube(2);',
      quality: 'full',
    })

    await vi.waitFor(() => {
      const job1Terminal = scope.events.find(event => event.jobId === 1
        && (event.status === 'stale' || event.status === 'cancelled'))
      expect(job1Terminal).toMatchObject({ status: 'stale' })
      const job2Terminal = scope.events.find(event => event.jobId === 2 && event.status === 'succeeded')
      expect(job2Terminal).toBeDefined()
    })
  })
})
