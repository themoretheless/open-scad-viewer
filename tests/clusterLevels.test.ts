import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { buildMeshBvh } from '../src/kernel/meshBvh'
import { ClusterLevelSource, clusterLevel } from '../src/render/clusterLevels'
import { isDeferredLevel } from '../src/render/deferredLevels'
import { DEFAULT_DISPLAY_BUDGET, boxProxiesOn, type IdleScheduler } from '../src/render/displayScheduler'
import { getDisplayBudget, resetDisplayBudgetSetting } from '../src/render/displayBudgetSetting'
import { GpuResidencyCache } from '../src/render/gpuResidencyCache'
import { MeshDisplayController } from '../src/render/meshDisplayController'
import { releaseMeshLod } from '../src/render/meshLod'
import type { NativePickingCache } from '../src/render/nativePickingCache'
import type { GMesh } from '../src/render/types'
import type { ViewerLodCamera } from '../src/render/viewerLod'
import * as viewerLod from '../src/render/viewerLod'

/** UV sphere with interleaved position/normal vertices. */
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

/** Separate triangles one unit apart; fine grids collapse nothing. */
function scattered(count: number) {
  const vertices: number[] = []
  for (let t = 0; t < count; t++) vertices.push(t, 0, 0, 0, 0, 1, t + 0.5, 0, 0, 0, 0, 1, t, 0.5, 0, 0, 0, 1)
  return { vertices: new Float32Array(vertices), indices: new Uint32Array(count * 3).map((_, i) => i) }
}

function mesh(geometry = sphere(48)): GMesh {
  const triangles = geometry.indices.length / 3
  return {
    vb: { destroy: vi.fn() },
    ib: { destroy: vi.fn() },
    ic: geometry.indices.length,
    edgeIB: null,
    edgeIC: 0,
    vertices: geometry.vertices,
    indices: geometry.indices,
    edgeIndices: new Uint32Array([0, 1, 1, 2]),
    faceIds: new Uint32Array(triangles),
    bvh: buildMeshBvh(geometry.vertices, geometry.indices),
    provenance: [{ triangleStart: 0, triangleEnd: triangles, backside: false, source: null }],
    color: [1, 0, 0, 1],
    visible: true,
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    inverseTransform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    worldBounds: { center: [0, 0, 0], radius: Math.sqrt(3), min: [-1, -1, -1], max: [1, 1, 1] },
  } as unknown as GMesh
}

function device() {
  const created: { size: number }[] = []
  return {
    created,
    dev: {
      limits: { maxBufferSize: 1 << 30 },
      createBuffer: ({ size }: { size: number }) => {
        const buffer = { size, destroy: vi.fn() }
        created.push(buffer)
        return buffer
      },
      queue: { writeBuffer: vi.fn() },
    } as unknown as GPUDevice,
  }
}

function manualScheduler() {
  const tasks: (() => void)[] = []
  const schedule: IdleScheduler = task => tasks.push(task)
  return {
    schedule,
    get size() {
      return tasks.length
    },
    flush() {
      while (tasks.length) tasks.shift()!()
    },
  }
}

const camera: ViewerLodCamera = {
  eye: [0, 0, 10],
  distance: 10,
  eyeDistance: 10,
  fovY: Math.PI / 2,
  projection: 'perspective',
  viewportHeightPx: 1000,
}
const picking = () => ({ release: vi.fn(), query: vi.fn(() => null) }) as unknown as NativePickingCache
const flushTimers = () => new Promise(resolve => setTimeout(resolve, 0))
const selectLevel = (pick: (count: number) => number) =>
  vi.spyOn(viewerLod, 'selectViewerLod').mockImplementation(levels => ({
    level: pick(levels.length),
    projectedError: 0,
    withinPixelBudget: true,
    reason: 'initial',
  }))

beforeEach(() => vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, COPY_DST: 4 }))
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('clustered coarse levels', () => {
  it('collapses a dense mesh and keeps every source vertex within the reported error', () => {
    const base = mesh()
    const level = clusterLevel(base, 6)!
    expect(level.triangleCount).toBeLessThan(base.indices.length / 3 / 4)
    expect(level.faceIds).toHaveLength(level.triangleCount)
    expect(level.provenance[0]!.triangleEnd).toBe(level.triangleCount)
    expect(Math.max(...level.indices)).toBeLessThan(level.vertices.length / 6)
    for (let v = 0; v < base.vertices.length; v += 6) {
      let nearest = Infinity
      for (let c = 0; c < level.vertices.length; c += 6) {
        nearest = Math.min(
          nearest,
          Math.hypot(
            base.vertices[v]! - level.vertices[c]!,
            base.vertices[v + 1]! - level.vertices[c + 1]!,
            base.vertices[v + 2]! - level.vertices[c + 2]!,
          ),
        )
      }
      expect(nearest).toBeLessThanOrEqual(level.errorUpper)
    }
  })

  it('returns null when nothing collapses', () => {
    expect(clusterLevel(mesh(scattered(20)), 1000)).toBeNull()
  })

  it('attaches a placeholder and builds it only when level 0 is asked for', async () => {
    const idle = manualScheduler()
    const source = new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, idle.schedule)
    const small = mesh(sphere(8))
    const large = mesh()
    expect(source.attach([small, large])).toEqual([large])
    expect(small.lod).toBeUndefined()
    expect(isDeferredLevel(large.lod!.levels[0])).toBe(true)
    expect(large.lod!.levels[1]!.vertices).toBe(large.vertices)
    expect(idle.size).toBe(0)
    const done = source.load(large, 0)
    expect(idle.size).toBe(1)
    idle.flush()
    await done
    expect(isDeferredLevel(large.lod!.levels[0])).toBe(false)
    expect(large.lod!.levels[0]!.triangleCount).toBeLessThan(large.lod!.levels[1]!.triangleCount)
  })

  it('shares one level list between instances of the same geometry', () => {
    const source = new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, manualScheduler().schedule)
    const a = mesh()
    const b = { ...a, lod: undefined } as GMesh
    source.attach([a, b])
    expect(b.lod!.levels).toBe(a.lod!.levels)
  })

  it('gives up and drops the LOD state when clustering cannot reduce the mesh', async () => {
    const idle = manualScheduler()
    const source = new ClusterLevelSource({ minTriangles: 10, ratio: 1e6 }, idle.schedule)
    const flat = mesh(scattered(20))
    source.attach([flat])
    const done = source.load(flat, 0)
    idle.flush()
    await expect(done).rejects.toThrow()
    expect(flat.lod).toBeUndefined()
    expect(source.attach([flat])).toEqual([])
  })
})

describe('display controller with lazy clustered levels', () => {
  const frame = (dev: GPUDevice, meshes: GMesh[], key: string, residency: GpuResidencyCache) => ({
    dev,
    meshes,
    camera,
    selectionKey: key,
    sceneEpoch: 1,
    residency,
    nativePicking: picking(),
    drawable: () => true,
  })

  it('keeps the published buffers for the finest level and builds nothing', () => {
    selectLevel(count => count - 1)
    const idle = manualScheduler()
    const controller = new MeshDisplayController(
      vi.fn(),
      () => null,
      new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, idle.schedule),
    )
    const { dev, created } = device()
    const large = mesh()
    const vb = large.vb
    controller.update(frame(dev, [large], 'a', new GpuResidencyCache(1e9)))
    expect(large.lod!.current).toBe(1)
    expect(large.vb).toBe(vb)
    expect(created).toHaveLength(0)
    expect(idle.size).toBe(0)
  })

  it('builds, uploads and binds the coarse level when selection asks for it, then returns to the base', async () => {
    let finest = false
    selectLevel(count => (finest ? count - 1 : 0))
    const idle = manualScheduler()
    const requestRender = vi.fn()
    const controller = new MeshDisplayController(
      requestRender,
      () => null,
      new ClusterLevelSource({ minTriangles: 1000, ratio: 1 / 8 }, idle.schedule),
    )
    const { dev, created } = device()
    const large = mesh()
    const base = { vb: large.vb, vertices: large.vertices }
    const residency = new GpuResidencyCache(1e9)
    controller.update(frame(dev, [large], 'far', residency))
    expect(large.lod!.current).toBeNull()
    expect(idle.size).toBe(1)
    idle.flush()
    await flushTimers()
    await flushTimers()
    expect(requestRender).toHaveBeenCalled()
    const changed = controller.update(frame(dev, [large], 'far', residency))
    expect(changed).toEqual([large])
    expect(large.lod!.current).toBe(0)
    expect(large.vertices).not.toBe(base.vertices)
    expect(created).toHaveLength(2)
    finest = true
    controller.update(frame(dev, [large], 'near', residency))
    expect(large.lod!.current).toBe(1)
    expect(large.vb).toBe(base.vb)
    expect(large.vertices).toBe(base.vertices)
    expect(created).toHaveLength(2)
    releaseMeshLod(large, picking())
    expect(large.vb).toBe(base.vb)
  })
})

describe('bounding-box proxies default', () => {
  it('are on for the default and a fresh persisted budget', () => {
    resetDisplayBudgetSetting()
    expect(boxProxiesOn(DEFAULT_DISPLAY_BUDGET)).toBe(true)
    expect(boxProxiesOn(getDisplayBudget())).toBe(true)
    expect(boxProxiesOn(undefined)).toBe(true)
  })
})
