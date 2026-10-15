import { describe, it, expect } from 'vitest'
import {
  buildMeshEdgeAdjacency,
  referenceSemanticEdges,
  runGpuSemanticEdges,
  SEMANTIC_EDGES_WGSL,
} from '../src/services/gpuSemanticEdges'

describe('WebGPU Semantic & Crease Edges Detection', () => {
  it('correctly builds mesh edge adjacency', () => {
    // Single quad split into 2 triangles: (0, 1, 2) and (0, 2, 3)
    const indices = new Uint32Array([
      0, 1, 2,
      0, 2, 3,
    ])

    const adj = buildMeshEdgeAdjacency(indices)
    // 5 unique edges in total: (0,1), (1,2), (0,2), (2,3), (0,3)
    expect(adj.length).toBe(5 * 4)

    let sharedEdgeFound = false
    let boundaryCount = 0

    for (let i = 0; i < 5; i++) {
      const v0 = adj[i * 4 + 0]!
      const v1 = adj[i * 4 + 1]!
      const tri0 = adj[i * 4 + 2]!
      const tri1 = adj[i * 4 + 3]!

      if (v0 === 0 && v1 === 2) {
        sharedEdgeFound = true
        expect(tri0).toBe(0)
        expect(tri1).toBe(1)
      } else {
        expect(tri1).toBe(0xffffffff)
        boundaryCount++
      }
    }

    expect(sharedEdgeFound).toBe(true)
    expect(boundaryCount).toBe(4)
  })

  it('filters out coplanar diagonal edges and retains only 12 sharp edges of a cube', () => {
    // Standard unit cube: 8 vertices, 12 triangles (2 triangles per face)
    const vertices = new Float32Array([
      0, 0, 0, // 0
      1, 0, 0, // 1
      1, 1, 0, // 2
      0, 1, 0, // 3
      0, 0, 1, // 4
      1, 0, 1, // 5
      1, 1, 1, // 6
      0, 1, 1, // 7
    ])

    const indices = new Uint32Array([
      // -Z face
      0, 2, 1, 0, 3, 2,
      // +Z face
      4, 5, 6, 4, 6, 7,
      // -Y face
      0, 1, 5, 0, 5, 4,
      // +Y face
      2, 3, 7, 2, 7, 6,
      // -X face
      0, 4, 7, 0, 7, 3,
      // +X face
      1, 2, 6, 1, 6, 5,
    ])

    const result = referenceSemanticEdges(vertices, indices, {
      creaseAngleDegrees: 30, // 90 degree edges will have dot=0 < cos(30)=0.866
    })

    // Total 18 edges: 12 cube edges (90 deg) + 6 face diagonals (0 deg)
    // All 6 diagonals must be rejected, exactly 12 sharp edges must be kept!
    expect(result.edgeCount).toBe(12)
    expect(result.indices.length).toBe(24)

    // Verify none of the diagonals are present
    // Diagonals: (0, 2), (4, 6), (0, 5), (2, 7), (0, 7), (1, 6)
    const diagonals = new Set(['0-2', '4-6', '0-5', '2-7', '0-7', '1-6'])
    for (let i = 0; i < result.edgeCount; i++) {
      const v0 = result.indices[i * 2 + 0]!
      const v1 = result.indices[i * 2 + 1]!
      const key = `${Math.min(v0, v1)}-${Math.max(v0, v1)}`
      expect(diagonals.has(key)).toBe(false)
    }
  })

  it('extracts all boundary edges from an open planar surface', () => {
    // 2x2 quad mesh on Z=0: 9 vertices, 8 triangles
    const vertices = new Float32Array([
      0, 0, 0,  1, 0, 0,  2, 0, 0,
      0, 1, 0,  1, 1, 0,  2, 1, 0,
      0, 2, 0,  1, 2, 0,  2, 2, 0,
    ])

    const indices = new Uint32Array([
      // Quad (0,0)-(1,1)
      0, 1, 4,  0, 4, 3,
      // Quad (1,0)-(2,1)
      1, 2, 5,  1, 5, 4,
      // Quad (0,1)-(1,2)
      3, 4, 7,  3, 7, 6,
      // Quad (1,1)-(2,2)
      4, 5, 8,  4, 8, 7,
    ])

    const result = referenceSemanticEdges(vertices, indices, {
      creaseAngleDegrees: 30,
    })

    // In a 2x2 grid, boundary perimeter has 8 edges (2 on each of the 4 outer sides)
    // All interior edges are coplanar, so only the 8 perimeter boundary edges should be emitted!
    expect(result.edgeCount).toBe(8)
    expect(result.indices.length).toBe(16)
  })

  it('declares valid WGSL compute shader with atomic counter and uniform params', () => {
    expect(SEMANTIC_EDGES_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(SEMANTIC_EDGES_WGSL).toContain('compute_triangle_normal')
    expect(SEMANTIC_EDGES_WGSL).toContain('crease_dot_threshold')
    expect(SEMANTIC_EDGES_WGSL).toContain('atomicAdd(&edge_counter[0], 1u)')
    expect(SEMANTIC_EDGES_WGSL).toContain('out_edge_indices[out_off + 0u] = edge.v0;')
  })

  it('runGpuSemanticEdges provides fallback in headless test environment', async () => {
    const vertices = new Float32Array([
      0, 0, 0,
      1, 0, 0,
      0, 1, 0,
    ])
    const indices = new Uint32Array([0, 1, 2])

    const result = await runGpuSemanticEdges(vertices, indices)
    // Single triangle has 3 boundary edges
    expect(result.edgeCount).toBe(3)
    expect(result.indices.length).toBe(6)
  })
})
