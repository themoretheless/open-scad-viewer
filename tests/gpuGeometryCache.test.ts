import { afterEach, describe, expect, it, vi } from 'vitest'
import { GpuGeometryCache } from '../src/render/gpuGeometryCache'
import { GpuResidencyCache } from '../src/render/gpuResidencyCache'
import { ProgressiveLevelStore } from '../src/render/displayScheduler'
import type { MeshLodLevel } from '../src/core/mesh'
import type { GMesh } from '../src/render/types'

interface FakeBuffer {
  size: number
  destroyed: number
  destroy(): void
}
const buffer = (size: number): FakeBuffer => ({
  size,
  destroyed: 0,
  destroy() {
    this.destroyed++
  },
})
const gpu = (value: FakeBuffer) => value as unknown as GPUBuffer

function asset(bytes: number) {
  return { vertices: new Float32Array(bytes / 8), indices: new Uint32Array(bytes / 8) }
}

function upload(cache: GpuGeometryCache, id: string, bytes: number) {
  const { vertices, indices } = asset(bytes)
  const vb = buffer(vertices.byteLength),
    ib = buffer(indices.byteLength)
  cache.adopt(id, vertices, indices, gpu(vb), gpu(ib))
  return { vertices, indices, vb, ib }
}

describe('GpuGeometryCache', () => {
  it('retains assets that left the scene and reuses them when they return', () => {
    const cache = new GpuGeometryCache(new GpuResidencyCache(1000))
    const a = cache.stage(['asset:a'], () => upload(cache, 'asset:a', 96))
    cache.publish(['asset:a'])
    cache.stage(['asset:b'], () => upload(cache, 'asset:b', 96))
    cache.publish(['asset:b'])
    // asset:a is unreferenced but under budget: still resident and still owned.
    expect(cache.has('asset:a')).toBe(true)
    expect(cache.owns(gpu(a.vb))).toBe(true)
    expect(a.vb.destroyed).toBe(0)
    const hit = cache.lookup('asset:a', a.vertices, a.indices)
    expect(hit?.vb).toBe(a.vb)
    // Equal content in different arrays still hits; different content misses.
    expect(cache.lookup('asset:a', new Float32Array(a.vertices), new Uint32Array(a.indices))?.vb).toBe(a.vb)
    expect(cache.lookup('asset:a', new Float32Array(12).fill(1), a.indices)).toBeNull()
  })

  it('evicts least recently used unreferenced assets over budget and never live ones', () => {
    const evicted: GPUBuffer[] = []
    const cache = new GpuGeometryCache(new GpuResidencyCache(200), ({ vb }) => evicted.push(vb))
    const a = upload(cache, 'asset:a', 96)
    const b = upload(cache, 'asset:b', 96)
    cache.publish(['asset:a', 'asset:b'])
    // Both live: a third live asset exceeds the budget without evicting them.
    const c = cache.stage(['asset:c'], () => upload(cache, 'asset:c', 96))
    expect(cache.size).toBe(3)
    cache.publish(['asset:c'])
    // a and b are now unreferenced; the oldest goes first until the budget holds.
    expect(a.vb.destroyed).toBe(1)
    expect(a.ib.destroyed).toBe(1)
    expect(b.vb.destroyed).toBe(0)
    expect(c.vb.destroyed).toBe(0)
    expect(evicted).toEqual([gpu(a.vb)])
    expect(cache.owns(gpu(a.vb))).toBe(false)
    expect(cache.bytes).toBe(192)
  })

  it('releases buffers adopted by a failed staging', () => {
    const cache = new GpuGeometryCache(new GpuResidencyCache(1000))
    let staged: ReturnType<typeof upload> | undefined
    expect(() =>
      cache.stage(['asset:a'], () => {
        staged = upload(cache, 'asset:a', 96)
        throw new Error('boom')
      }),
    ).toThrow('boom')
    expect(staged!.vb.destroyed).toBe(1)
    expect(cache.has('asset:a')).toBe(false)
  })

  afterEach(() => vi.unstubAllGlobals())

  it('shares one byte budget with the LOD level store', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const residency = new GpuResidencyCache(300)
    const geometry = new GpuGeometryCache(residency)
    const created: FakeBuffer[] = []
    const dev = {
      createBuffer: ({ size }: { size: number }) => {
        const value = buffer(size)
        created.push(value)
        return value
      },
      queue: { writeBuffer: () => {} },
    } as unknown as GPUDevice
    const tasks: (() => void)[] = []
    const store = new ProgressiveLevelStore(residency, undefined, task => tasks.push(task))
    const level = (triangles: number): MeshLodLevel => ({
      vertices: new Float32Array(triangles * 18),
      indices: new Uint32Array(triangles * 3),
      edgeIndices: new Uint32Array(0),
      faceIds: new Uint32Array(triangles),
      provenance: [],
      triangleCount: triangles,
      errorUpper: 1 / triangles,
      bvh: {
        version: 1,
        vertexStride: 6,
        leafSize: 8,
        nodeCount: 0,
        bounds: new Float32Array(0),
        nodes: new Uint32Array(0),
        triangles: new Uint32Array(0),
      },
    })
    const levels = [level(1), level(2)]
    const mesh = { assetId: 'asset:lod', lod: { levels, current: null } } as unknown as GMesh
    const a = upload(geometry, 'asset:a', 96)
    geometry.publish([])
    // One 84-byte level plus a 96-byte unreferenced geometry fits in 300.
    store.resolve(dev, mesh, 0)
    tasks.shift()!()
    expect(residency.bytes).toBe(96 + 84)
    expect(store.cachedBytes).toBe(84)
    expect(geometry.bytes).toBe(96)
    // The 168-byte finer level pushes the total over budget: the oldest unpinned entry (geometry) goes.
    store.resolve(dev, mesh, 1)
    while (tasks.length) tasks.shift()!()
    expect(a.vb.destroyed).toBe(1)
    expect(geometry.bytes).toBe(0)
    expect(residency.bytes).toBe(84 + 168)
    expect(created).toHaveLength(4)
  })
})
