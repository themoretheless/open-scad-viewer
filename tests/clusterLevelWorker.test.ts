import { describe, expect, it } from 'vitest'
import type { MeshLodLevel } from '../src/core/mesh'
import { buildMeshBvh } from '../src/kernel/meshBvh'
import { createDisplayLevelWorkerHandler, type DisplayLevelRequest } from '../src/kernel/displayLevelWorkerRuntime'
import { ClusterLevelSource, clusterLevel, clusterResolution } from '../src/render/clusterLevels'
import { isDeferredLevel } from '../src/render/deferredLevels'
import type { IdleScheduler } from '../src/render/displayScheduler'
import { WorkerDisplayLevelProvider } from '../src/render/onDemandLevels'
import type { GMesh } from '../src/render/types'

function sphere(segments: number) {
  const vertices: number[] = []
  const indices: number[] = []
  for (let ring = 0; ring <= segments; ring++) {
    const theta = (ring / segments) * Math.PI
    for (let step = 0; step <= segments; step++) {
      const phi = (step / segments) * 2 * Math.PI
      const p = [Math.sin(theta) * Math.cos(phi), Math.cos(theta), Math.sin(theta) * Math.sin(phi)]
      vertices.push(...p, ...p)
    }
  }
  const row = segments + 1
  for (let ring = 0; ring < segments; ring++) {
    for (let step = 0; step < segments; step++) {
      const a = ring * row + step
      indices.push(a, a + row, a + 1, a + 1, a + row, a + row + 1)
    }
  }
  return { vertices: new Float32Array(vertices), indices: new Uint32Array(indices) }
}

function mesh(): GMesh {
  const { vertices, indices } = sphere(48)
  const triangles = indices.length / 3
  return {
    vertices,
    indices,
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 50]),
    faceIds: new Uint32Array(triangles).map((_, i) => i % 7),
    bvh: buildMeshBvh(vertices, indices),
    provenance: [{ triangleStart: 0, triangleEnd: triangles, backside: true, source: null }],
  } as unknown as GMesh
}

/** In-process stand-in for the display level worker: structured clone both ways, as postMessage does. */
class FakeWorker {
  private readonly listeners: ((event: MessageEvent) => void)[] = []
  readonly received: DisplayLevelRequest[] = []
  private readonly handle = createDisplayLevelWorkerHandler(
    (message, transfer) => {
      const data = structuredClone(message, { transfer })
      queueMicrotask(() => this.listeners.forEach(listener => listener({ data } as MessageEvent)))
    },
    {} as never,
    task => queueMicrotask(task),
  )
  addEventListener(type: string, listener: (event: MessageEvent) => void) {
    if (type === 'message') this.listeners.push(listener)
  }
  postMessage(message: DisplayLevelRequest) {
    const data = structuredClone(message)
    this.received.push(data)
    this.handle(data)
  }
  terminate() {}
}

const views = (level: MeshLodLevel) => [
  level.vertices,
  level.indices,
  level.edgeIndices,
  level.faceIds,
  level.bvh.bounds,
  level.bvh.nodes,
  level.bvh.triangles,
]
const asBytes = (view: ArrayBufferView) => Array.from(new Uint8Array(view.buffer, view.byteOffset, view.byteLength))
const reference = (source: GMesh) => clusterLevel(mesh(), clusterResolution(Math.floor(source.indices.length / 3 / 8)))!

function idle() {
  const tasks: (() => void)[] = []
  const schedule: IdleScheduler = task => tasks.push(task)
  return { schedule, tasks, flush: () => tasks.splice(0).forEach(task => task()) }
}

describe('clustered levels in the display level worker', () => {
  it('builds a level byte-identical to the main-thread build', async () => {
    const worker = new FakeWorker()
    const provider = new WorkerDisplayLevelProvider(() => worker as unknown as Worker)
    const scheduler = idle()
    const source = new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, scheduler.schedule)
    source.setBuilder((base, resolution) => provider.cluster(base, resolution))
    const large = mesh()
    source.attach([large])
    await source.load(large, 0)
    expect(scheduler.tasks).toHaveLength(0)
    expect(worker.received.map(request => request.type)).toEqual(['cluster'])
    const built = large.lod!.levels[0]!
    expect(isDeferredLevel(built)).toBe(false)
    const expected = reference(large)
    expect(built.triangleCount).toBe(expected.triangleCount)
    expect(built.errorUpper).toBe(expected.errorUpper)
    expect(built.provenance).toEqual(expected.provenance)
    expect(built.bvh.leafSize).toBe(expected.bvh.leafSize)
    views(built).forEach((view, i) => expect(asBytes(view)).toEqual(asBytes(views(expected)[i]!)))
    // The base arrays are copied to the worker, not transferred.
    expect(large.lod!.levels[1]!.vertices.byteLength).toBeGreaterThan(0)
  })

  it('falls back to the main thread when the worker cannot start', async () => {
    const provider = new WorkerDisplayLevelProvider(() => {
      throw new Error('workers unavailable')
    })
    const scheduler = idle()
    const source = new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, scheduler.schedule)
    source.setBuilder((base, resolution) => provider.cluster(base, resolution))
    const large = mesh()
    source.attach([large])
    const done = source.load(large, 0)
    expect(scheduler.tasks).toHaveLength(1)
    scheduler.flush()
    await done
    views(large.lod!.levels[0]!).forEach((view, i) =>
      expect(asBytes(view)).toEqual(asBytes(views(reference(large))[i]!)),
    )
  })

  it('falls back to the main thread when the worker request fails', async () => {
    const scheduler = idle()
    const source = new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, scheduler.schedule)
    source.setBuilder(() => Promise.reject(new Error('worker crashed')))
    const large = mesh()
    source.attach([large])
    const done = source.load(large, 0)
    await new Promise(resolve => setTimeout(resolve, 0))
    expect(scheduler.tasks).toHaveLength(1)
    scheduler.flush()
    await done
    expect(isDeferredLevel(large.lod!.levels[0])).toBe(false)
  })
})
