import { describe, expect, it, vi } from 'vitest'
import { createSelectionSurfacePublisher } from '../src/services/selectionSurfacePublisher'
import type { MeshData } from '../src/core/mesh'

function mesh(): MeshData {
  return {
    geometryAssetId: 'asset:publisher-test',
    vertices: new Float32Array([0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1]),
    indices: new Uint32Array([0, 1, 2]),
    faceIds: new Uint32Array([0]),
    bvh: { version: 1, vertexStride: 6, leafSize: 8, nodeCount: 0, bounds: new Float32Array(), nodes: new Uint32Array(), triangles: new Uint32Array() },
    edgeIndices: new Uint32Array(),
    color: [1, 1, 1, 1],
    transform: new Float32Array(16),
    provenance: [],
    topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

describe('selection surface publisher ownership', () => {
  it('reuses cached ids without cloning for the main-thread publisher', () => {
    const infer = vi.fn(() => new Uint32Array([7]))
    const publish = createSelectionSurfacePublisher(infer, { cloneResult: false })
    const first = publish(mesh())
    const second = publish(mesh())

    expect(infer).toHaveBeenCalledTimes(1)
    expect(second.faceIds).toBe(first.faceIds)
  })

  it('detaches cached ids for the worker publisher', () => {
    const infer = vi.fn(() => new Uint32Array([7]))
    const publish = createSelectionSurfacePublisher(infer)
    const first = publish(mesh())
    const second = publish(mesh())

    expect(infer).toHaveBeenCalledTimes(1)
    expect(second.faceIds).not.toBe(first.faceIds)
    expect(second.faceIds).toEqual(first.faceIds)
  })
})
