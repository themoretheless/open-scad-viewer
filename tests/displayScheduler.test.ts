import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { MeshLodLevel } from '../src/core/mesh'
import {
  ByteLruCache,
  DEFAULT_DISPLAY_BUDGET,
  displayBudgetFor,
  planDisplayCaps,
  ProgressiveLevelStore,
  representationCap,
  type IdleScheduler,
} from '../src/render/displayScheduler'
import { createMeshLodState, releaseMeshLod, updateMeshLods } from '../src/render/meshLod'
import type { NativePickingCache } from '../src/render/nativePickingCache'
import type { GMesh } from '../src/render/types'
import type { ViewerLodCamera } from '../src/render/viewerLod'
import * as viewerLod from '../src/render/viewerLod'

interface FakeBuffer {
  size: number
  destroyed: boolean
  destroy(): void
}
const fakeBuffer = (size: number): FakeBuffer => ({
  size,
  destroyed: false,
  destroy() {
    this.destroyed = true
  },
})

function device() {
  const created: FakeBuffer[] = []
  return {
    created,
    dev: {
      createBuffer: ({ size }: { size: number }) => {
        const buffer = fakeBuffer(size)
        created.push(buffer)
        return buffer
      },
      queue: { writeBuffer: () => {} },
    } as unknown as GPUDevice,
  }
}

/** Level of `triangles` triangles: 18 floats + 3 indices each, i.e. 84 bytes per triangle. */
const level = (triangles: number, errorUpper: number): MeshLodLevel => ({
  vertices: new Float32Array(triangles * 18),
  indices: new Uint32Array(triangles * 3),
  edgeIndices: new Uint32Array(2),
  faceIds: new Uint32Array(triangles),
  provenance: [],
  bvh: {} as never,
  triangleCount: triangles,
  errorUpper,
})

function mesh(levels: MeshLodLevel[], opts: { assetId?: string; center?: [number, number, number] } = {}): GMesh {
  const center = opts.center ?? [0, 0, 0]
  const m = {
    assetId: opts.assetId,
    vb: fakeBuffer(1),
    ib: fakeBuffer(1),
    ic: 3,
    edgeIB: null,
    edgeIC: 0,
    vertices: new Float32Array(18),
    indices: new Uint32Array(3),
    edgeIndices: new Uint32Array(0),
    faceIds: new Uint32Array(1),
    bvh: {},
    provenance: [],
    visible: true,
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    worldBounds: { center, radius: 1, min: [-1, -1, -1], max: [1, 1, 1] },
  } as unknown as GMesh
  m.lod = createMeshLodState(m, levels)
  return m
}

const camera: ViewerLodCamera = {
  eye: [0, 0, 10],
  distance: 10,
  eyeDistance: 10,
  fovY: Math.PI / 2,
  projection: 'perspective',
  viewportHeightPx: 1000,
}

/** Manual idle scheduler: tasks run only when the test flushes them. */
function manualScheduler() {
  const tasks: (() => void)[] = []
  const schedule: IdleScheduler = task => tasks.push(task)
  return {
    schedule,
    tasks,
    flush() {
      while (tasks.length) tasks.shift()!()
    },
  }
}

const picking = () => ({ release: vi.fn() }) as unknown as NativePickingCache

beforeEach(() => vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, COPY_DST: 4 }))
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('display representation planning', () => {
  it('maps representations onto coarse-to-fine levels', () => {
    expect(representationCap('bounding-box', 3)).toBe(0)
    expect(representationCap('preview', 3)).toBe(1)
    expect(representationCap('preview', 2)).toBe(0)
    expect(representationCap('exact', 3)).toBe(2)
  })

  it('refines near entities and drops far ones to the coarsest level', () => {
    const near = mesh([level(4, 0.4), level(16, 0.1)], { center: [0, 0, 5] })
    const far = mesh([level(4, 0.4), level(16, 0.1)], { center: [0, 0, -5000] })
    const plan = planDisplayCaps([far, near], camera, { budgetBytes: 1e9, targetErrorPixels: 24 })
    expect(plan.get(near)).toMatchObject({ representation: 'exact', maxLevel: 1 })
    expect(plan.get(far)).toMatchObject({ representation: 'bounding-box', maxLevel: 0 })
    expect(plan.get(near)!.projectedPixels).toBeGreaterThan(plan.get(far)!.projectedPixels)
  })

  it('spends the byte budget on the largest projection first', () => {
    const levels = () => [level(1, 1), level(10, 0.5), level(100, 0.1)]
    const near = mesh(levels(), { center: [0, 0, 5] })
    const mid = mesh(levels(), { center: [0, 0, 0] })
    // Near takes exact (8400 B); mid then only fits its preview level (840 B).
    const plan = planDisplayCaps([mid, near], camera, { budgetBytes: 8400 + 900, targetErrorPixels: 1 })
    expect(plan.get(near)?.representation).toBe('exact')
    expect(plan.get(mid)).toMatchObject({ representation: 'preview', maxLevel: 1 })
  })

  it('skips hidden and non-LOD meshes', () => {
    const hidden = mesh([level(4, 0.4), level(16, 0.1)])
    hidden.visible = false
    const plain = mesh([])
    expect(planDisplayCaps([hidden, plain], camera).size).toBe(0)
  })
})

describe('byte-bounded LRU cache', () => {
  it('evicts least recently used entries by bytes', () => {
    const evicted: string[] = []
    const cache = new ByteLruCache<number>(100, key => evicted.push(key))
    cache.set('a', 1, 40)
    cache.set('b', 2, 40)
    expect(cache.get('a')).toBe(1)
    cache.set('c', 3, 40)
    expect(evicted).toEqual(['b'])
    expect(cache.keys()).toEqual(['a', 'c'])
    expect(cache.bytes).toBe(80)
  })

  it('never evicts pinned entries or the entry just inserted', () => {
    const cache = new ByteLruCache<number>(50, undefined, key => key === 'a')
    cache.set('a', 1, 40)
    cache.set('b', 2, 40)
    expect(cache.keys()).toEqual(['a', 'b'])
    expect(cache.bytes).toBe(80)
  })

  it('replaces and invalidates entries with accounting', () => {
    const evicted: string[] = []
    const cache = new ByteLruCache<number>(1000, key => evicted.push(key))
    cache.set('x#0', 1, 10)
    cache.set('x#0', 2, 30)
    cache.set('y#0', 3, 5)
    expect(cache.bytes).toBe(35)
    expect(cache.invalidate(key => key.startsWith('x#'))).toBe(1)
    expect(evicted).toEqual(['x#0', 'x#0'])
    expect(cache.bytes).toBe(5)
  })
})

describe('progressive level store', () => {
  it('requests a missing level, keeps the frame unblocked and swaps when ready', () => {
    const { dev, created } = device()
    const idle = manualScheduler(),
      ready = vi.fn()
    const store = new ProgressiveLevelStore(1e6, ready, idle.schedule)
    const m = mesh([level(4, 0.4), level(16, 0.1)], { assetId: 'asset:a' })
    expect(store.resolve(dev, m, 1)).toBeNull()
    expect(store.state(m, 1)).toBe('pending')
    expect(created).toHaveLength(0)
    // A second request while pending does not queue another upload.
    store.resolve(dev, m, 1)
    idle.flush()
    expect(created).toHaveLength(2)
    expect(ready).toHaveBeenCalledTimes(1)
    expect(store.state(m, 1)).toBe('ready')
    expect(store.resolve(dev, m, 1)?.level).toBe(1)
  })

  it('offers the finest ready coarser level while a finer one loads', () => {
    const { dev } = device()
    const idle = manualScheduler()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule)
    const m = mesh([level(1, 1), level(4, 0.4), level(16, 0.1)], { assetId: 'asset:b' })
    store.resolve(dev, m, 0)
    idle.flush()
    const fallback = store.resolve(dev, m, 2)
    expect(fallback?.level).toBe(0)
    expect(store.state(m, 2)).toBe('pending')
    m.lod!.current = 0
    // Already showing the fallback: keep the binding until the target lands.
    expect(store.resolve(dev, m, 2)).toBeNull()
    idle.flush()
    expect(store.resolve(dev, m, 2)?.level).toBe(2)
  })

  it('uploads one level per idle callback', () => {
    const { dev } = device()
    const idle = manualScheduler()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule)
    store.resolve(dev, mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:c' }), 1)
    store.resolve(dev, mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:d' }), 1)
    expect(idle.tasks).toHaveLength(1)
    idle.tasks.shift()!()
    expect(idle.tasks).toHaveLength(1)
  })

  it('shares levels between instances of one asset', () => {
    const { dev, created } = device()
    const idle = manualScheduler()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule)
    const levels = [level(1, 1), level(4, 0.4)]
    const a = mesh(levels, { assetId: 'asset:shared' }),
      b = mesh(levels, { assetId: 'asset:shared' })
    store.resolve(dev, a, 1)
    idle.flush()
    expect(store.resolve(dev, b, 1)?.buffers).toBe(store.resolve(dev, a, 1)?.buffers)
    expect(created).toHaveLength(2)
  })

  it('invalidates cached levels when the geometry behind an asset id changes', () => {
    const { dev, created } = device()
    const idle = manualScheduler()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule)
    const before = mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:e' })
    store.resolve(dev, before, 1)
    idle.flush()
    const after = mesh([level(1, 1), level(8, 0.2)], { assetId: 'asset:e' })
    expect(store.state(after, 1)).toBe('missing')
    expect(store.resolve(dev, after, 1)).toBeNull()
    expect(created.every(buffer => buffer.destroyed)).toBe(true)
    expect(store.cachedBytes).toBe(0)
    idle.flush()
    expect(store.resolve(dev, after, 1)?.level).toBe(1)
  })

  it('drops levels of geometry that left the scene and discards stale uploads', () => {
    const { dev, created } = device()
    const idle = manualScheduler()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule)
    const kept = mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:kept' })
    const gone = mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:gone' })
    store.resolve(dev, kept, 1)
    store.resolve(dev, gone, 1)
    idle.tasks.shift()!()
    store.retain([kept])
    idle.flush()
    expect(store.state(kept, 1)).toBe('ready')
    expect(store.state(gone, 1)).toBe('missing')
    // Only the kept upload created buffers; the gone one was discarded before running.
    expect(created).toHaveLength(2)
  })

  it('evicts unbound levels past the byte budget but keeps bound ones', () => {
    const { dev, created } = device()
    const idle = manualScheduler()
    // Each 4-triangle level is 336 bytes; the budget fits one.
    const store = new ProgressiveLevelStore(400, undefined, idle.schedule)
    const a = mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:a' })
    const b = mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:b' })
    store.resolve(dev, a, 1)
    idle.flush()
    a.lod!.current = 1
    store.setBound([a])
    store.resolve(dev, b, 1)
    idle.flush()
    expect(store.state(a, 1)).toBe('ready')
    a.lod!.current = null
    store.setBound([a])
    store.resolve(dev, mesh([level(1, 1), level(4, 0.4)], { assetId: 'asset:c' }), 1)
    idle.flush()
    expect(store.state(a, 1)).toBe('missing')
    expect(created.filter(buffer => buffer.destroyed)).toHaveLength(4)
  })
})

describe('meshLod with display planning', () => {
  it('shows the base, refines progressively and caps far meshes', () => {
    vi.spyOn(viewerLod, 'selectViewerLod').mockImplementation(levels => ({
      level: levels.length - 1,
      projectedError: 0,
      withinPixelBudget: true,
      reason: 'initial',
    }))
    const { dev } = device()
    const idle = manualScheduler(),
      cache = picking()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule)
    const budget = { budgetBytes: 1e9, targetErrorPixels: 24 }
    const near = mesh([level(4, 0.4), level(16, 0.1)], { assetId: 'asset:near', center: [0, 0, 5] })
    const far = mesh([level(4, 0.4), level(16, 0.1)], { assetId: 'asset:far', center: [0, 0, -5000] })
    expect(updateMeshLods(dev, [near, far], camera, cache, undefined, { budget, store })).toEqual([])
    expect(near.lod?.current).toBeNull()
    idle.flush()
    expect(updateMeshLods(dev, [near, far], camera, cache, undefined, { budget, store })).toEqual([near, far])
    expect(near.lod?.current).toBe(1)
    expect(far.lod?.current).toBe(0)
    // Releasing restores the base and leaves the shared cached buffers to the store.
    const bound = near.vb as unknown as FakeBuffer
    releaseMeshLod(near, cache)
    expect(near.lod?.current).toBeNull()
    expect(bound.destroyed).toBe(false)
  })

  it('marks bounding-box entities as box proxies unless the budget turns them off', () => {
    vi.spyOn(viewerLod, 'selectViewerLod').mockImplementation(levels => ({
      level: levels.length - 1,
      projectedError: 0,
      withinPixelBudget: true,
      reason: 'initial',
    }))
    const { dev } = device()
    const cache = picking()
    const near = mesh([level(4, 0.4), level(16, 0.1)], { center: [0, 0, 5] })
    const far = mesh([level(4, 0.4), level(16, 0.1)], { center: [0, 0, -5000] })
    const plain = { budgetBytes: 1e9, targetErrorPixels: 24, boundingBoxProxies: false }
    updateMeshLods(dev, [near, far], camera, cache, undefined, { budget: plain })
    expect(far.lod?.current).toBe(0)
    expect(far.displayBox).toBe(false)
    // Turning proxies on changes no level; only the box flag the encoder reads flips.
    const proxied = { budgetBytes: 1e9, targetErrorPixels: 24 }
    expect(updateMeshLods(dev, [near, far], camera, cache, undefined, { budget: proxied })).toEqual([])
    expect(far.displayBox).toBe(true)
    expect(near.displayBox).toBe(false)
    expect(updateMeshLods(dev, [near, far], camera, cache, undefined, { budget: plain })).toEqual([])
    expect(far.displayBox).toBe(false)
  })
})

describe('display budget steps', () => {
  it('snaps requested budgets to the offered megabyte steps', () => {
    expect(displayBudgetFor(256)).toEqual(DEFAULT_DISPLAY_BUDGET)
    expect(displayBudgetFor(300).budgetBytes).toBe(256 * 1024 * 1024)
    expect(displayBudgetFor(10).budgetBytes).toBe(64 * 1024 * 1024)
    expect(displayBudgetFor(1e6, true)).toMatchObject({ budgetBytes: 1024 * 1024 * 1024, boundingBoxProxies: true })
    expect(displayBudgetFor(Number.NaN)).toEqual(DEFAULT_DISPLAY_BUDGET)
  })
})
