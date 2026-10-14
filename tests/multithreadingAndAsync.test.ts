import { afterEach, describe, expect, it, vi } from 'vitest'
import { MainSolidWorkerPool } from '../src/services/mainSolidWorkerPool'
import type { MainSolidPort } from '../src/services/mainSolidWorkerClient'
import type { MainSolidRequest } from '../src/services/mainSolidProtocol'
import type { TrussModel, TrussResponse } from '../src/services/trussAnalysis'
import { forEachChunkedAsync, mapChunkedAsync, yieldToEventLoop } from '../src/services/cooperativeAsync'
import { referenceMeshSlicer, referenceMeshSlicerAsync } from '../src/services/gpuSlicer'
import { referenceFilterGcodeSegments, referenceFilterGcodeSegmentsAsync } from '../src/services/gpuGcodePreview'
import { referenceImageContourTrace, referenceImageContourTraceAsync } from '../src/services/gpuImageTrace'
import { referenceSemanticEdges, referenceSemanticEdgesAsync } from '../src/services/gpuSemanticEdges'
import { buildFlatBvh, referenceBatchedBvhRaycast, referenceBatchedBvhRaycastAsync } from '../src/services/gpuBvhRaycast'
import { referenceNurbsTessellate, referenceNurbsTessellateAsync } from '../src/services/gpuNurbsTessellate'

class MockSolidPort implements MainSolidPort {
  onmessage: MainSolidPort['onmessage'] = null
  onerror: MainSolidPort['onerror'] = null
  onmessageerror: MainSolidPort['onmessageerror'] = null
  terminate = vi.fn()
  postMessage = vi.fn<(message: MainSolidRequest) => void>()

  reply(value: object) {
    const last = this.postMessage.mock.lastCall?.[0]
    if (!last) throw new Error('No message posted to port')
    const { version, id, job } = last
    this.onmessage?.({ data: { version, id, kind: job.kind, ...value } } as MessageEvent)
  }
}

const sampleModel = (): TrussModel => ({
  nodesMm: [
    [0, 0, 0],
    [10, 0, 0],
  ],
  members: [{ nodes: [0, 1], youngMpa: 2000, areaMm2: 2 }],
  restrained: [
    [true, true, true],
    [false, true, true],
  ],
  forcesN: [
    [0, 0, 0],
    [100, 0, 0],
  ],
})

const sampleResponse = (): TrussResponse => ({
  displacementsMm: [
    [0, 0, 0],
    [0.25, 0, 0],
  ],
  reactionsN: [
    [-100, 0, 0],
    [0, 0, 0],
  ],
  axialForcesN: [100],
  axialStressesMpa: [50],
  maxDeflectionMm: 0.25,
  maxRelativeResidual: 0,
  freeDofs: 1,
})

const pools: MainSolidWorkerPool[] = []
afterEach(() => {
  pools.splice(0).forEach(p => p.dispose())
})

describe('MainSolidWorkerPool & Cooperative Async Execution', () => {
  it('dispatches concurrent jobs in parallel across multiple warm workers without aborting in-flight work', async () => {
    const ports: MockSolidPort[] = []
    const pool = new MainSolidWorkerPool(() => {
      const p = new MockSolidPort()
      ports.push(p)
      return p
    }, 2)
    pools.push(pool)

    const jobA = pool.run({ kind: 'truss', model: sampleModel() })
    const jobB = pool.run({ kind: 'inspect', bodies: [] })
    const jobC = pool.run({ kind: 'truss', model: sampleModel() })

    // Wait a microtask for pump() to dispatch jobA and jobB to slots 0 and 1
    await Promise.resolve()
    expect(ports).toHaveLength(2)
    expect(ports[0]!.terminate).not.toHaveBeenCalled()
    expect(ports[1]!.terminate).not.toHaveBeenCalled()
    expect(pool.getStats()).toMatchObject({
      poolSize: 2,
      activeTasks: 2,
      queuedTasks: 1,
      completedTasks: 0,
    })

    // Complete jobB on worker 1; jobC should immediately be dispatched to worker 1
    ports[1]!.reply({ ok: true, result: [] })
    await expect(jobB).resolves.toEqual([])
    await Promise.resolve()

    expect(ports).toHaveLength(2) // Reused warm worker 1!
    expect(ports[1]!.postMessage).toHaveBeenCalledTimes(2)

    ports[0]!.reply({ ok: true, result: sampleResponse() })
    ports[1]!.reply({ ok: true, result: sampleResponse() })

    await expect(jobA).resolves.toEqual(sampleResponse())
    await expect(jobC).resolves.toEqual(sampleResponse())

    expect(pool.getStats()).toMatchObject({
      poolSize: 2,
      activeTasks: 0,
      queuedTasks: 0,
      completedTasks: 3,
      perWorkerCompleted: [1, 2],
    })
  })

  it('supports priority ordering and per-task AbortSignal cancellation in the queue without killing active workers', async () => {
    const ports: MockSolidPort[] = []
    const pool = new MainSolidWorkerPool(() => {
      const p = new MockSolidPort()
      ports.push(p)
      return p
    }, 1)
    pools.push(pool)

    const order: string[] = []
    const activeJob = pool.run({ kind: 'inspect', bodies: [] }).then(res => {
      order.push('first')
      return res
    })

    const abortCtrl = new AbortController()
    const cancelledQueued = pool.run({ kind: 'inspect', bodies: [] }, { signal: abortCtrl.signal, priority: 1 })
    const lowPriority = pool.run({ kind: 'inspect', bodies: [] }, { priority: 1 }).then(res => {
      order.push('low')
      return res
    })
    const highPriority = pool.run({ kind: 'inspect', bodies: [] }, { priority: 10 }).then(res => {
      order.push('high')
      return res
    })

    // Abort the queued task before the active job finishes
    abortCtrl.abort()
    await expect(cancelledQueued).rejects.toMatchObject({ name: 'AbortError' })
    expect(ports[0]!.terminate).not.toHaveBeenCalled()

    // Finish active job -> highPriority should run next, then lowPriority
    ports[0]!.reply({ ok: true, result: [] })
    await activeJob
    await Promise.resolve()

    ports[0]!.reply({ ok: true, result: [] })
    await highPriority
    await Promise.resolve()

    ports[0]!.reply({ ok: true, result: [] })
    await lowPriority

    expect(order).toEqual(['first', 'high', 'low'])
  })

  it('executes cooperative async chunked loops and matches synchronous CPU references bit-for-bit', async () => {
    await yieldToEventLoop()
    const mapped = await mapChunkedAsync([1, 2, 3, 4, 5], x => x * 10, { checkStride: 2, timeBudgetMs: 0 })
    expect(mapped).toEqual([10, 20, 30, 40, 50])

    const abortController = new AbortController()
    const promise = forEachChunkedAsync(
      100,
      i => {
        if (i === 5) abortController.abort()
      },
      { checkStride: 2, signal: abortController.signal }
    )
    await expect(promise).rejects.toMatchObject({ name: 'AbortError' })

    // Verify referenceMeshSlicerAsync matches referenceMeshSlicer
    const vertices = new Float32Array([0, 0, 0, 10, 0, 0, 0, 10, 10])
    const indices = new Uint32Array([0, 1, 2])
    const sliceOpts = { firstLayerDistance: 2, layerStep: 2, numLayers: 4, checkStride: 1, timeBudgetMs: 0 }
    const syncSlice = referenceMeshSlicer(vertices, indices, sliceOpts)
    const asyncSlice = await referenceMeshSlicerAsync(vertices, indices, sliceOpts)
    expect(asyncSlice).toEqual(syncSlice)

    // Verify referenceFilterGcodeSegmentsAsync matches referenceFilterGcodeSegments
    const moves = new Float32Array([
      0, 0, 0, 0, 50, 0, 0,
      10, 0, 0, 1, 60, 1, 1,
      10, 10, 0, 2, 60, 1, 1,
    ])
    const syncGcode = referenceFilterGcodeSegments(moves, { minLayer: 1, maxLayer: 2 })
    const asyncGcode = await referenceFilterGcodeSegmentsAsync(moves, {
      minLayer: 1,
      maxLayer: 2,
      checkStride: 1,
      timeBudgetMs: 0,
    })
    expect(asyncGcode).toEqual(syncGcode)

    // Verify referenceImageContourTraceAsync matches referenceImageContourTrace
    const pixels = new Uint8Array([
      0, 0, 0, 255, 255, 255, 255, 255,
      255, 255, 255, 255, 0, 0, 0, 255,
    ])
    const traceOpts = { widthPx: 2, heightPx: 2, widthMm: 10, checkStride: 1, timeBudgetMs: 0 }
    const syncTrace = referenceImageContourTrace(pixels, traceOpts)
    const asyncTrace = await referenceImageContourTraceAsync(pixels, traceOpts)
    expect(asyncTrace).toEqual(syncTrace)

    // Verify referenceSemanticEdgesAsync matches referenceSemanticEdges
    const boxVerts = new Float32Array([
      0, 0, 0,
      10, 0, 0,
      10, 10, 0,
      0, 0, 10,
    ])
    const boxIndices = new Uint32Array([
      0, 2, 1,
      0, 1, 3,
      1, 2, 3,
      2, 0, 3,
    ])
    const syncEdges = referenceSemanticEdges(boxVerts, boxIndices, { creaseAngleDeg: 30 })
    const asyncEdges = await referenceSemanticEdgesAsync(boxVerts, boxIndices, {
      creaseAngleDeg: 30,
      checkStride: 1,
      timeBudgetMs: 0,
    })
    expect(asyncEdges).toEqual(syncEdges)

    // Verify referenceBatchedBvhRaycastAsync matches referenceBatchedBvhRaycast
    const bvh = buildFlatBvh([
      { id: 'box-a', min: [0, 0, 0], max: [10, 10, 10] },
      { id: 'box-b', min: [20, 0, 0], max: [30, 10, 10] },
    ])
    const rays = [
      { origin: [5, 5, -5] as [number, number, number], direction: [0, 0, 1] as [number, number, number] },
      { origin: [50, 50, -5] as [number, number, number], direction: [0, 0, 1] as [number, number, number] },
    ]
    const syncHits = referenceBatchedBvhRaycast(bvh, rays)
    const asyncHits = await referenceBatchedBvhRaycastAsync(bvh, rays, { checkStride: 1, timeBudgetMs: 0 })
    expect(asyncHits).toEqual(syncHits)

    // Verify referenceNurbsTessellateAsync matches referenceNurbsTessellate
    const patch = {
      degreeU: 1,
      degreeV: 1,
      numCpU: 2,
      numCpV: 2,
      knotsU: new Float32Array([0, 0, 1, 1]),
      knotsV: new Float32Array([0, 0, 1, 1]),
      controlPoints: new Float32Array([
        0, 0, 0,
        10, 0, 0,
        0, 10, 0,
        10, 10, 5,
      ]),
    }
    const syncNurbs = referenceNurbsTessellate(patch, 4, 4)
    const asyncNurbs = await referenceNurbsTessellateAsync(patch, 4, 4, { checkStride: 1, timeBudgetMs: 0 })
    expect(asyncNurbs).toEqual(syncNurbs)
  })
})
