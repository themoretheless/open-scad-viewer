import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  type GeometryBuildRequest,
  type GeometryWorkerEvent,
  type GeometryWorkerRequest,
} from '../src/services/geometryWorkerProtocol'
import { OpenSCADParseError } from '../src/services/openscadParser'
import { sha256Hex } from '../src/core/sha256'

const parseOpenSCADMock = vi.hoisted(() => vi.fn())
// Both qualified providers share the WASM kernel loader, while only the mesh
// provider invokes parseOpenSCAD directly.
const warmGeometryKernelMock = vi.hoisted(() => vi.fn(() => Promise.resolve()))

vi.mock('../src/services/openscadParser', async importOriginal => {
  const original = await importOriginal<typeof import('../src/services/openscadParser')>()
  return { ...original, parseOpenSCAD: parseOpenSCADMock, warmGeometryKernel: warmGeometryKernelMock }
})

class FakeWorkerScope {
  readonly events: GeometryWorkerEvent[] = []
  readonly transfers: Transferable[][] = []
  private readonly listeners = new Map<string, Set<EventListenerOrEventListenerObject>>()

  postMessage(event: GeometryWorkerEvent, options: { transfer?: Transferable[] } = {}) {
    this.events.push(event)
    this.transfers.push([...(options.transfer ?? [])])
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

class CloneFailingWorkerScope extends FakeWorkerScope {
  override postMessage(event: GeometryWorkerEvent, options: { transfer?: Transferable[] } = {}) {
    if (event.status === 'succeeded') {
      throw new DOMException('The object could not be cloned', 'DataCloneError')
    }
    super.postMessage(event, options)
  }
}

function identity(prefix: string, length: number, suffix = 'x'): string {
  return `${prefix}${suffix.repeat(length - prefix.length)}`
}

function identityMesh(options: {
  entityId: string
  operationId: string
  instanceId: string
}) {
  return {
    entityId: options.entityId,
    vertices: new Float32Array([
      0, 0, 0, 0, 0, 1,
      1, 0, 0, 0, 0, 1,
      0, 1, 0, 0, 0, 1,
    ]),
    indices: new Uint32Array([0, 1, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    faceIds: new Uint32Array([0]),
    color: [1, 1, 1, 1] as [number, number, number, number],
    transform: new Float32Array([
      1, 0, 0, 0,
      0, 1, 0, 0,
      0, 0, 1, 0,
      0, 0, 0, 1,
    ]),
    bvh: {
      version: 1 as const,
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
      source: {
        id: 1,
        originalId: 1,
        start: 0,
        end: 0,
        label: 'cube()',
        operationId: options.operationId,
        instanceId: options.instanceId,
      },
    }],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function identityResult(options: {
  entityId: string
  operationId: string
  instanceId: string
}) {
  return {
    meshes: [identityMesh(options)],
    warnings: [],
    volume: 0,
    surfaceArea: 0.5,
    quality: 'full' as const,
    reduced: false as const,
    timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
  }
}

afterEach(() => {
  parseOpenSCADMock.mockReset()
  warmGeometryKernelMock.mockClear()
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('geometry Worker lifecycle', () => {
  it('publishes exact 256-code-unit entity, operation, and instance identities', async () => {
    const identities = {
      entityId: identity('entity:', 256),
      operationId: identity('op:', 256),
      instanceId: identity('entity:', 256, 'i'),
    }
    parseOpenSCADMock.mockResolvedValue(identityResult(identities))
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')
    const source = 'cube(1);'

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    })

    await vi.waitFor(() => expect(scope.events.at(-1)?.status).toBe('succeeded'))
    const success = scope.events.at(-1)
    expect(success).toMatchObject({
      status: 'succeeded',
      meshes: [{
        entityId: identities.entityId,
        provenance: [{
          source: {
            operationId: identities.operationId,
            instanceId: identities.instanceId,
          },
        }],
      }],
    })
    expect(scope.events.filter(event => ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status)))
      .toHaveLength(1)
    expect(scope.transfers.at(-1)?.length).toBeGreaterThan(0)
  })

  it.each([
    ['entityId', 'entity:'],
    ['operationId', 'op:'],
    ['instanceId', 'entity:'],
  ] as const)('fails a 257-code-unit %s before success transfer and recovers on the same Worker', async (field, prefix) => {
    const valid = {
      entityId: identity('entity:', 256),
      operationId: identity('op:', 256),
      instanceId: identity('entity:', 256, 'i'),
    }
    const oversized = identity(prefix, 257, field === 'instanceId' ? 'j' : 'z')
    const invalid = { ...valid, [field]: oversized }
    parseOpenSCADMock
      .mockResolvedValueOnce(identityResult(invalid))
      .mockResolvedValueOnce(identityResult(valid))
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')
    const firstSource = 'cube(1);'

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source: firstSource,
      sourceSha256: sha256Hex(firstSource),
      quality: 'full',
    })

    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({
      jobId: 1,
      status: 'failed',
      phase: 'serializing',
      error: {
        name: 'GeometryWorkerProtocolError',
        code: 'WORKER_RESULT_UNPUBLISHABLE',
        message: 'Geometry result cannot be published under protocol v6',
      },
    }))
    const firstJobEvents = scope.events.filter(event => event.jobId === 1)
    expect(firstJobEvents.filter(event => ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status)))
      .toHaveLength(1)
    expect(firstJobEvents.some(event => event.status === 'succeeded')).toBe(false)
    expect(JSON.stringify(firstJobEvents.at(-1))).not.toContain(oversized.slice(0, 256))
    expect(scope.transfers.filter((_transfer, index) => scope.events[index]?.jobId === 1).flat()).toHaveLength(0)

    const secondSource = 'cube(2);'
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 2,
      jobId: 2,
      source: secondSource,
      sourceSha256: sha256Hex(secondSource),
      quality: 'full',
    })

    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({ jobId: 2, status: 'succeeded' }))
    expect(scope.events.filter(event => event.jobId === 2
      && ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status))).toHaveLength(1)
    expect(parseOpenSCADMock).toHaveBeenCalledTimes(2)
  })

  it('ignores a replayed active job id without replacing or duplicating its terminal', async () => {
    let finish!: (result: {
      meshes: []
      warnings: []
      volume: number
      surfaceArea: number
      quality: 'full'
      reduced: false
      timings: { parseMs: number; bindMs: number; initializeMs: number; evaluateMs: number; analyzeMs: number }
    }) => void
    parseOpenSCADMock.mockReturnValue(new Promise(resolve => {
      finish = resolve
    }))
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')
    const source = 'cube(1);'
    const request: GeometryBuildRequest = {
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 42,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    }
    scope.dispatchMessage(request)
    scope.dispatchMessage(request)
    finish({
      meshes: [], warnings: [], volume: 1, surfaceArea: 6, quality: 'full', reduced: false,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    })

    await vi.waitFor(() => expect(scope.events.at(-1)?.status).toBe('succeeded'))
    expect(parseOpenSCADMock).toHaveBeenCalledTimes(1)
    expect(scope.events.filter(event => event.status === 'succeeded')).toHaveLength(1)
  })

  it('tombstones a terminal job id but continues with the next monotonic id', async () => {
    parseOpenSCADMock.mockImplementation(async source => ({
      meshes: [], warnings: [], volume: source.length, surfaceArea: 0, quality: 'full' as const,
      reduced: false,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    }))
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')
    const request = (jobId: number, source: string, documentRevision = jobId): GeometryBuildRequest => ({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision,
      jobId,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    })

    scope.dispatchMessage(request(7, 'cube(1);'))
    await vi.waitFor(() => expect(scope.events.at(-1)?.status).toBe('succeeded'))
    const eventCount = scope.events.length
    scope.dispatchMessage(request(7, 'sphere(1);', 99))
    await Promise.resolve()
    expect(scope.events).toHaveLength(eventCount)
    expect(parseOpenSCADMock).toHaveBeenCalledTimes(1)

    scope.dispatchMessage(request(8, 'cube(2);'))
    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({ jobId: 8, status: 'succeeded' }))
    expect(parseOpenSCADMock).toHaveBeenCalledTimes(2)
    expect(scope.events.filter(event => event.jobId === 7 && event.status === 'accepted')).toHaveLength(1)
    expect(scope.events.filter(event => event.jobId === 7
      && ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status))).toHaveLength(1)
  })

  it('executes a B-rep source without invoking the mesh parser', async () => {
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    const source = '// @language openscad-viewer/brep-1\ncube(1);'
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 1,
      jobId: 1,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    })

    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({
      status: 'succeeded',
      execution: {
        languageContract: 'openscad-viewer/brep-1',
        engineClass: 'brep',
        semanticProgramVersion: 'semantic-program-contract-v1',
        evidence: 'runtime',
        automaticFallback: false,
      },
    }))
    expect(parseOpenSCADMock).not.toHaveBeenCalled()
    expect(warmGeometryKernelMock).toHaveBeenCalledTimes(1)
  })

  it('reports a malformed header before stale queue state without rerouting providers', async () => {
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    const newer = '// @language openscad-viewer/brep-1\ncube(2);'
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build', documentRevision: 2, jobId: 2, source: newer,
      sourceSha256: sha256Hex(newer), quality: 'full',
    })
    await vi.waitFor(() => expect(scope.events.at(-1)?.status).toBe('succeeded'))

    const malformed = '// @engine manifold\ncube(1);'
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build', documentRevision: 1, jobId: 3, source: malformed,
      sourceSha256: sha256Hex(malformed), quality: 'full',
    })
    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({
      jobId: 3,
      status: 'failed',
      error: { code: 'LANGUAGE_CONTRACT_UNSUPPORTED' },
    }))
    expect(scope.events.some(event => event.jobId === 3 && event.status === 'stale')).toBe(false)
    expect(warmGeometryKernelMock).toHaveBeenCalledTimes(1)
    expect(parseOpenSCADMock).not.toHaveBeenCalled()
  })

  it('does not let a malformed newer revision supersede valid running work', async () => {
    let finish!: (result: {
      meshes: []
      warnings: []
      volume: number
      surfaceArea: number
      quality: 'full'
      reduced: false
      timings: { parseMs: number; bindMs: number; initializeMs: number; evaluateMs: number; analyzeMs: number }
    }) => void
    parseOpenSCADMock.mockReturnValue(new Promise(resolve => { finish = resolve }))
    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    const valid = 'cube(1);'
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build', documentRevision: 1, jobId: 1, source: valid,
      sourceSha256: sha256Hex(valid), quality: 'full',
    })
    await vi.waitFor(() => expect(parseOpenSCADMock).toHaveBeenCalledOnce())

    const malformed = '// @engine brep\ncube(2);'
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build', documentRevision: 2, jobId: 2, source: malformed,
      sourceSha256: sha256Hex(malformed), quality: 'full',
    })
    expect(scope.events.at(-1)).toMatchObject({
      jobId: 2,
      status: 'failed',
      error: { code: 'LANGUAGE_CONTRACT_UNSUPPORTED' },
    })

    finish({
      meshes: [], warnings: [], volume: 1, surfaceArea: 6, quality: 'full', reduced: false,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    })
    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({ jobId: 1, status: 'succeeded' }))
    expect(scope.events.some(event => event.jobId === 1 && event.status === 'stale')).toBe(false)
  })

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
      sourceSha256: sha256Hex('cube(1);'),
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
      sourceSha256: sha256Hex('cube(1);'),
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
        timings: { parseMs: 1, bindMs: 0, initializeMs: 1, evaluateMs: 1, analyzeMs: 1 },
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
      sourceSha256: sha256Hex('cube(1);'),
      quality: 'full',
    })
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 2,
      jobId: 2,
      source: 'cube(2);',
      sourceSha256: sha256Hex('cube(2);'),
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

  it('publishes one positioned failure when a model assertion rejects the build', async () => {
    const source = 'cube(1);\nassert(false, "invalid model");'
    parseOpenSCADMock.mockRejectedValue(new OpenSCADParseError(
      source,
      source.indexOf('assert'),
      "Assertion 'false' failed: invalid model",
    ))

    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 7,
      jobId: 11,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    })

    await vi.waitFor(() => {
      expect(scope.events.at(-1)?.status).toBe('failed')
    })
    expect(scope.events.map(event => event.status)).toEqual([
      'accepted', 'started', 'progress', 'failed',
    ])
    expect(scope.events.at(-1)).toMatchObject({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      documentRevision: 7,
      jobId: 11,
      quality: 'full',
      status: 'failed',
      phase: 'compiling',
      execution: {
        languageContract: 'legacy/current',
        engineClass: 'mesh',
        evidence: 'runtime',
      },
      error: {
        name: 'OpenSCADParseError',
        line: 2,
        column: 1,
      },
    })
    expect(scope.events.filter(event => ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status)))
      .toHaveLength(1)
  })

  it('keeps runtime provenance when success serialization fails after the build', async () => {
    parseOpenSCADMock.mockResolvedValue({
      meshes: [],
      warnings: [],
      volume: 1,
      surfaceArea: 6,
      quality: 'full',
      reduced: false,
      timings: { parseMs: 1, bindMs: 0, initializeMs: 1, evaluateMs: 1, analyzeMs: 1 },
    })
    const scope = new CloneFailingWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 8,
      jobId: 12,
      source: 'cube(1);',
      sourceSha256: sha256Hex('cube(1);'),
      quality: 'full',
    })

    await vi.waitFor(() => expect(scope.events.at(-1)).toMatchObject({
      status: 'failed',
      execution: {
        engineClass: 'mesh',
        evidence: 'runtime',
        quality: 'full',
      },
      error: { name: 'DataCloneError' },
    }))
  })

  it('keeps cancellation terminal when an assertion failure settles after cancel', async () => {
    const source = 'assert(false, "cancelled guard");'
    let rejectParse!: (reason?: unknown) => void
    parseOpenSCADMock.mockReturnValue(new Promise((_resolve, reject) => {
      rejectParse = reject
    }))

    const scope = new FakeWorkerScope()
    vi.stubGlobal('self', scope)
    await import('../src/workers/geometry.worker')

    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'build',
      documentRevision: 8,
      jobId: 12,
      source,
      sourceSha256: sha256Hex(source),
      quality: 'full',
    })
    scope.dispatchMessage({
      protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
      type: 'cancel',
      documentRevision: 8,
      jobId: 12,
      reason: 'user',
    })
    rejectParse(new OpenSCADParseError(
      source,
      0,
      "Assertion 'false' failed: cancelled guard",
    ))

    await vi.waitFor(() => {
      expect(scope.events.at(-1)).toMatchObject({
        status: 'cancelled',
        reason: 'user',
        jobId: 12,
      })
    })
    expect(scope.events.some(event => event.status === 'failed')).toBe(false)
    expect(scope.events.filter(event => ['failed', 'cancelled', 'stale', 'succeeded'].includes(event.status)))
      .toHaveLength(1)
  })
})
