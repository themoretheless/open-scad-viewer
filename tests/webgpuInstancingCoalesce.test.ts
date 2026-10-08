import { describe, expect, it, vi, afterEach } from 'vitest'
import { WebGPURenderer, sortOpaqueDraws, sortEdgeDraws } from '../src/services/webgpuRenderer'
import { MeshInstances, type InstanceMesh } from '../src/services/meshInstances'
import { identity } from '../src/services/math3d'

describe('GPU Instancing Coalescing and Hover Optimization', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('sortOpaqueDraws clusters interleaved shared geometry into contiguous batches while preserving stability', () => {
    const vbA = {} as GPUBuffer
    const ibA = {} as GPUBuffer
    const vbB = {} as GPUBuffer
    const ibB = {} as GPUBuffer

    const meshA1 = { vb: vbA, ib: ibA, ic: 30, shadingModel: 'phong', name: 'A1' } as any
    const meshA2 = { vb: vbA, ib: ibA, ic: 30, shadingModel: 'phong', name: 'A2' } as any
    const meshB1 = { vb: vbB, ib: ibB, ic: 60, shadingModel: 'phong', name: 'B1' } as any
    const meshB2 = { vb: vbB, ib: ibB, ic: 60, shadingModel: 'phong', name: 'B2' } as any

    // Interleaved document order: A1, B1, A2, B2
    const draws = [meshA1, meshB1, meshA2, meshB2]
    sortOpaqueDraws(draws)

    // Verify that identical geometries are clustered together
    expect(draws[0].vb).toBe(draws[1].vb)
    expect(draws[2].vb).toBe(draws[3].vb)
    expect(draws[0].vb).not.toBe(draws[2].vb)

    // Verify stability: A1 comes before A2, B1 comes before B2
    const aIndices = draws.filter(m => m.vb === vbA).map(m => m.name)
    expect(aIndices).toEqual(['A1', 'A2'])
    const bIndices = draws.filter(m => m.vb === vbB).map(m => m.name)
    expect(bIndices).toEqual(['B1', 'B2'])
  })

  it('sortEdgeDraws clusters interleaved shared edge buffers into contiguous batches', () => {
    const vbA = {} as GPUBuffer
    const edgeA = {} as GPUBuffer
    const vbB = {} as GPUBuffer
    const edgeB = {} as GPUBuffer

    const edgeMeshA1 = { vb: vbA, edgeIB: edgeA, edgeIC: 12, name: 'EA1' } as any
    const edgeMeshA2 = { vb: vbA, edgeIB: edgeA, edgeIC: 12, name: 'EA2' } as any
    const edgeMeshB1 = { vb: vbB, edgeIB: edgeB, edgeIC: 24, name: 'EB1' } as any
    const edgeMeshB2 = { vb: vbB, edgeIB: edgeB, edgeIC: 24, name: 'EB2' } as any

    const draws = [edgeMeshA1, edgeMeshB1, edgeMeshA2, edgeMeshB2]
    sortEdgeDraws(draws)

    expect(draws[0].edgeIB).toBe(draws[1].edgeIB)
    expect(draws[2].edgeIB).toBe(draws[3].edgeIB)
    expect(draws[0].edgeIB).not.toBe(draws[2].edgeIB)
  })

  it('interleaved instances qualify and execute as coalesced instanced draws in MeshInstances', () => {
    vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_DST: 2, VERTEX: 4 })
    const buffers: { destroy: ReturnType<typeof vi.fn> }[] = []
    const device = {
      limits: { maxStorageBufferBindingSize: 1 << 24 },
      createBuffer: vi.fn(() => { const buffer = { destroy: vi.fn() }; buffers.push(buffer); return buffer }),
      createBindGroup: vi.fn(() => ({})),
      queue: { writeBuffer: vi.fn() },
    }
    const pass = {
      setPipeline: vi.fn(),
      setBindGroup: vi.fn(),
      setVertexBuffer: vi.fn(),
      setIndexBuffer: vi.fn(),
      drawIndexed: vi.fn(),
    }

    const vbA = {} as GPUBuffer
    const ibA = {} as GPUBuffer
    const vbB = {} as GPUBuffer
    const ibB = {} as GPUBuffer

    // Create 32 meshes alternating: A, B, A, B ...
    const interleaved: InstanceMesh[] = Array.from({ length: 32 }, (_, i) => {
      const isA = i % 2 === 0
      const transform = identity()
      transform[12] = i * 2
      return {
        vb: isA ? vbA : vbB,
        ib: isA ? ibA : ibB,
        ic: isA ? 10 : 20,
        bg: {} as GPUBindGroup,
        edgeIB: null,
        edgeIC: 0,
        morphSlot: null,
        transform,
        inverseTransform: identity(),
        color: [1, 1, 1, 1],
        styleAlpha: 1,
        styleSelected: 0,
        styleEdge: 0,
        styleHovered: 0,
        shadingModel: 'phong',
      } as unknown as InstanceMesh
    })

    const instances = new MeshInstances()

    // Without sorting, 32 alternating meshes have 32 runs of 1, failing eligible (32 * 4 > 32)
    const successUnsorted = instances.draw(
      pass as unknown as GPURenderPassEncoder,
      device as unknown as GPUDevice,
      {} as GPUBindGroupLayout,
      {} as GPURenderPipeline,
      {} as GPUBindGroup,
      interleaved,
      false,
    )
    expect(successUnsorted).toBe(false)
    expect(pass.drawIndexed).not.toHaveBeenCalled()

    // With sortOpaqueDraws, meshes are coalesced into 2 contiguous runs of 16
    const coalesced = [...interleaved]
    sortOpaqueDraws(coalesced as any)

    const successSorted = instances.draw(
      pass as unknown as GPURenderPassEncoder,
      device as unknown as GPUDevice,
      {} as GPUBindGroupLayout,
      {} as GPURenderPipeline,
      {} as GPUBindGroup,
      coalesced,
      false,
    )
    expect(successSorted).toBe(true)
    // Exactly 2 instanced draw calls: 16 instances of A, then 16 instances of B
    expect(pass.drawIndexed).toHaveBeenCalledTimes(2)
    const calls = pass.drawIndexed.mock.calls
    const firstCount = calls[0][1]
    const secondCount = calls[1][1]
    expect(firstCount).toBe(16)
    expect(secondCount).toBe(16)
    const indexCounts = new Set([calls[0][0], calls[1][0]])
    expect(indexCounts).toEqual(new Set([10, 20]))
    expect(calls[0][4]).toBe(0)
    expect(calls[1][4]).toBe(16)
  })

  it('skips redundant hover evaluations when coordinates and interaction epoch are unchanged', () => {
    const renderer = new WebGPURenderer()
    const internal = renderer as any
    const findHitSpy = vi.fn(() => null)
    internal.findHit = findHitSpy

    // First call evaluates hit
    internal.updateHoverAt(100, 200)
    expect(findHitSpy).toHaveBeenCalledTimes(1)

    // Second call with same coordinates and unchanged epoch skips findHit
    internal.updateHoverAt(100, 200)
    expect(findHitSpy).toHaveBeenCalledTimes(1)

    // Moving to new coordinates triggers evaluation
    internal.updateHoverAt(101, 200)
    expect(findHitSpy).toHaveBeenCalledTimes(2)

    // Stationary call at new coordinates skips evaluation
    internal.updateHoverAt(101, 200)
    expect(findHitSpy).toHaveBeenCalledTimes(2)

    // Changing sceneEpoch (e.g. geometry or section update) invalidates cache and triggers evaluation
    internal.sceneEpoch++
    internal.updateHoverAt(101, 200)
    expect(findHitSpy).toHaveBeenCalledTimes(3)
  })

  it('rejects ray cast early when ray misses the entire scene bounds without querying the scene index', () => {
    const renderer = new WebGPURenderer()
    const internal = renderer as any
    internal.bounds = {
      min: [-1, -1, -1],
      max: [1, 1, 1],
      center: [0, 0, 0],
      radius: 1.73,
    }

    // Ray pointing away from the bounds (origin at [10, 10, 10] pointing towards [1, 0, 0])
    internal.rayForClientPoint = vi.fn(() => ({
      origin: [10, 10, 10],
      direction: [1, 0, 0],
    }))

    internal.currentSceneAabbIndex = vi.fn(() => ({}))

    const candidates = internal.findHitCandidates(500, 500, 1)
    expect(candidates).toEqual([])
    expect(internal.currentSceneAabbIndex).not.toHaveBeenCalled()
  })
})
