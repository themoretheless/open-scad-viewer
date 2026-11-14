import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { MeshLodLevel } from '../src/core/mesh'
import { bindMeshLod, createMeshLodState, releaseMeshLod, updateMeshLods, worldScale } from '../src/render/meshLod'
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
  const created: FakeBuffer[] = [],
    writes: unknown[] = []
  return {
    created,
    writes,
    dev: {
      createBuffer: ({ size }: { size: number }) => {
        const buffer = fakeBuffer(size)
        created.push(buffer)
        return buffer
      },
      queue: { writeBuffer: (_buffer: unknown, _offset: number, data: unknown) => writes.push(data) },
    } as unknown as GPUDevice,
  }
}

const level = (triangles: number, errorUpper: number): MeshLodLevel => ({
  vertices: new Float32Array(triangles * 18),
  indices: new Uint32Array(triangles * 3).map((_, i) => i),
  edgeIndices: new Uint32Array(2),
  faceIds: new Uint32Array(triangles),
  provenance: [],
  bvh: {
    bounds: new Float32Array(6),
    nodes: new Uint32Array(2),
    triangles: new Uint32Array(triangles),
    nodeCount: 1,
    vertexStride: 6,
    leafSize: 4,
  } as never,
  triangleCount: triangles,
  errorUpper,
})

function mesh(levels: MeshLodLevel[]): GMesh {
  const base = level(10, 0.2)
  const m = {
    vb: fakeBuffer(1),
    ib: fakeBuffer(1),
    ic: base.indices.length,
    edgeIB: fakeBuffer(1),
    edgeIC: 2,
    vertices: base.vertices,
    indices: base.indices,
    edgeIndices: base.edgeIndices,
    faceIds: base.faceIds,
    bvh: base.bvh,
    provenance: [],
    visible: true,
    faceIndex: null,
    transform: new Float32Array([2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1]),
    worldBounds: { center: [0, 0, 0], radius: 1, min: [-1, -1, -1], max: [1, 1, 1] },
  } as unknown as GMesh
  m.lod = createMeshLodState(m, levels)
  return m
}

const picking = () => ({ release: vi.fn() }) as unknown as NativePickingCache & { release: ReturnType<typeof vi.fn> }

beforeEach(() => vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, COPY_DST: 4 }))
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('mesh LOD binding', () => {
  it('ignores missing or single-level publications', () => {
    expect(createMeshLodState(mesh([]), undefined)).toBeUndefined()
    expect(createMeshLodState(mesh([]), [level(4, 1)])).toBeUndefined()
  })

  it('uploads each level once, swaps views and keeps the shared base buffers alive', () => {
    const { dev, created } = device(),
      cache = picking()
    const levels = [level(4, 0.4), level(16, 0.1)]
    const m = mesh(levels),
      baseVb = m.vb,
      baseEdge = m.edgeIB as unknown as FakeBuffer
    expect(bindMeshLod(dev, m, 1, cache)).toBe(true)
    expect(m.indices).toBe(levels[1].indices)
    expect(m.ic).toBe(48)
    expect(m.faceIndex).toBeUndefined()
    expect(m.edgeIB).toBeNull()
    expect(baseEdge.destroyed).toBe(false)
    expect(cache.release).not.toHaveBeenCalled()
    expect(bindMeshLod(dev, m, 1, cache)).toBe(false)
    expect(bindMeshLod(dev, m, 0, cache)).toBe(true)
    expect(cache.release).toHaveBeenCalledTimes(1)
    expect(bindMeshLod(dev, m, 1, cache)).toBe(true)
    expect(created).toHaveLength(4)
    releaseMeshLod(m, cache)
    expect(m.vb).toBe(baseVb)
    expect(m.lod?.current).toBeNull()
    expect(created.every(buffer => buffer.destroyed)).toBe(true)
    expect((baseVb as unknown as FakeBuffer).destroyed).toBe(false)
  })

  it('selects by projected error with world scale and skips hidden meshes', () => {
    const { dev } = device(),
      cache = picking()
    const select = vi
      .spyOn(viewerLod, 'selectViewerLod')
      .mockImplementation((levels, current) => ({
        level: current === null ? levels.length - 1 : 0,
        projectedError: 0,
        withinPixelBudget: true,
        reason: 'initial',
      }))
    const m = mesh([level(4, 0.4), level(16, 0.1)]),
      hidden = mesh([level(4, 0.4), level(16, 0.1)])
    hidden.visible = false
    const camera: ViewerLodCamera = {
      eye: [0, 0, 10],
      distance: 10,
      eyeDistance: 10,
      fovY: Math.PI / 2,
      projection: 'perspective',
      viewportHeightPx: 1000,
    }
    expect(updateMeshLods(dev, [m, hidden], camera, cache)).toEqual([m])
    expect(select.mock.calls[0][0].map(l => l.errorUpper)).toEqual([0.8, 0.2])
    expect(select.mock.calls[0][2].pixelsPerUnit).toBeCloseTo((1000 * 10) / (2 * 9 * 10), 9)
    expect(m.lod?.current).toBe(1)
    expect(updateMeshLods(dev, [m], camera, cache)).toEqual([m])
    expect(m.lod?.current).toBe(0)
  })

  it('measures the largest axis scale of an affine transform', () => {
    expect(worldScale(new Float32Array([1, 0, 0, 5, 0, 3, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1]))).toBe(3)
    expect(worldScale(new Float32Array(16))).toBe(1)
  })
})
