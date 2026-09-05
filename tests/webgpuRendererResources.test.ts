import { afterEach, describe, expect, it, vi } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { geometryAssetId } from '../src/core/scene'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

class FakeBuffer {
  destroyCalls = 0
  constructor(readonly size: number, readonly usage: number) {}
  destroy() { this.destroyCalls++ }
}

class FakeDevice {
  readonly buffers: FakeBuffer[] = []
  readonly writes: Array<{ buffer: FakeBuffer; offset: number; bytes: number }> = []
  readonly limits = { maxBufferSize: 1 << 28 }
  failCreateAt: number | null = null
  readonly queue = {
    writeBuffer: (buffer: FakeBuffer, offset: number, data: ArrayBufferView) => {
      this.writes.push({ buffer, offset, bytes: data.byteLength })
    },
  }
  createBuffer(descriptor: { size: number; usage: number }) {
    if (this.failCreateAt === this.buffers.length + 1) throw new Error('injected allocation failure')
    const buffer = new FakeBuffer(descriptor.size, descriptor.usage)
    this.buffers.push(buffer)
    return buffer
  }
  createBindGroup() { return {} }
}

function fixture(offset = 0): MeshData {
  const vertices = new Float32Array([
    offset, 0, 0, 0, 0, 1,
    offset + 1, 0, 0, 0, 0, 1,
    offset, 1, 0, 0, 0, 1,
  ])
  const indices = new Uint32Array([0, 1, 2])
  return {
    entityId: `entity:${offset}`,
    geometryAssetId: geometryAssetId(vertices, indices),
    vertices, indices,
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    faceIds: new Uint32Array([1]),
    bvh: { version: 1, vertexStride: 6, leafSize: 8, nodeCount: 1, bounds: new Float32Array(6), nodes: new Uint32Array(2), triangles: new Uint32Array([0]) },
    color: [1, 0, 0, 1], provenance: [],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function harness() {
  const renderer = new WebGPURenderer()
  const device = new FakeDevice()
  const internal = renderer as unknown as {
    dev: GPUDevice
    initialized: boolean
    dead: boolean
    lost: boolean
    objBGL: GPUBindGroupLayout
    initialFitDone: boolean
    meshes: Array<{ vb: FakeBuffer; ib: FakeBuffer; ub: FakeBuffer }>
    scheduleEdgeBufferWarmup(): void
    requestRender(): void
  }
  internal.dev = device as unknown as GPUDevice
  internal.initialized = true
  internal.dead = false
  internal.lost = false
  internal.objBGL = {} as GPUBindGroupLayout
  internal.initialFitDone = true
  internal.scheduleEdgeBufferWarmup = vi.fn()
  internal.requestRender = vi.fn()
  return { renderer, device, internal }
}

describe('WebGPURenderer retained geometry resources', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('reuses verified vertex/index buffers and only replaces entity uniforms', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    const first = fixture()
    renderer.setMeshes([first])
    expect(renderer.sceneUploadMetrics).toEqual({ geometryUploadBytes: 84, geometryBuffersCreated: 2, reusedEntities: 0 })
    const initial = internal.meshes[0]
    expect(device.buffers).toHaveLength(3)

    const replacement = fixture()
    renderer.setMeshes([replacement])
    expect(renderer.sceneUploadMetrics).toEqual({ geometryUploadBytes: 0, geometryBuffersCreated: 0, reusedEntities: 1 })
    expect(device.buffers).toHaveLength(4)
    expect(internal.meshes[0].vb).toBe(initial.vb)
    expect(internal.meshes[0].ib).toBe(initial.ib)
    expect(initial.vb.destroyCalls).toBe(0)
    expect(initial.ib.destroyCalls).toBe(0)
    expect(initial.ub.destroyCalls).toBe(1)
  })

  it('rolls back a failed staged entity allocation without destroying live geometry', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    renderer.setMeshes([fixture()])
    const live = internal.meshes[0]
    device.failCreateAt = device.buffers.length + 1

    expect(() => renderer.setMeshes([fixture()])).toThrow('injected allocation failure')
    expect(internal.meshes[0]).toBe(live)
    expect(live.vb.destroyCalls).toBe(0)
    expect(live.ib.destroyCalls).toBe(0)
    expect(live.ub.destroyCalls).toBe(0)
  })
})
