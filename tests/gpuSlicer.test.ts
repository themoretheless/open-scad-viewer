import { describe, it, expect } from 'vitest'
import {
  referenceMeshSlicer,
  runGpuMeshSlicer,
  computeSliceArea,
  groupSegmentsByLayer,
  MESH_SLICER_WGSL,
} from '../src/services/gpuSlicer'

/** Helper: Creates a closed watertight box from [0,0,0] to [sx, sy, sz] with outward normals */
function createBoxMesh(sx: number, sy: number, sz: number) {
  const vertices = new Float32Array([
    0, 0, 0,   // 0
    sx, 0, 0,  // 1
    sx, sy, 0, // 2
    0, sy, 0,  // 3
    0, 0, sz,  // 4
    sx, 0, sz, // 5
    sx, sy, sz,// 6
    0, sy, sz, // 7
  ])

  const indices = new Uint32Array([
    // Bottom (-Z)
    0, 2, 1,  0, 3, 2,
    // Top (+Z)
    4, 5, 6,  4, 6, 7,
    // Front (-Y, normal [0, -1, 0])
    0, 1, 5,  0, 5, 4,
    // Back (+Y, normal [0, +1, 0])
    2, 3, 7,  2, 7, 6,
    // Left (-X, normal [-1, 0, 0])
    3, 0, 4,  3, 4, 7,
    // Right (+X, normal [+1, 0, 0])
    1, 2, 6,  1, 6, 5,
  ])

  return { vertices, indices }
}

describe('WebGPU Compute Multi-Plane Mesh Slicer', () => {
  it('slices a rectangular box and computes exact cross-sectional area via Green theorem', () => {
    // Box of dimensions 4.0 x 6.0 x 10.0 (cross-section in XY is 4 * 6 = 24.0)
    const { vertices, indices } = createBoxMesh(4.0, 6.0, 10.0)

    const result = referenceMeshSlicer(vertices, indices, {
      planeNormal: [0, 0, 1],
      firstLayerDistance: 5.0,
      numLayers: 1,
    })

    // 4 vertical walls * 2 triangles per wall = 8 intersected triangles = 8 segments
    expect(result.segmentCount).toBe(8)

    // Verify all points lie on Z = 5.0
    for (const seg of result.segments) {
      expect(seg.p0[2]).toBeCloseTo(5.0, 5)
      expect(seg.p1[2]).toBeCloseTo(5.0, 5)
    }

    // Green's theorem area must equal 4.0 * 6.0 = 24.0
    const area = computeSliceArea(result.segments, [0, 0, 1])
    expect(area).toBeCloseTo(24.0, 4)
  })

  it('slices across multiple parallel Z layers simultaneously in a single pass', () => {
    const { vertices, indices } = createBoxMesh(3.0, 5.0, 10.0)

    const result = referenceMeshSlicer(vertices, indices, {
      planeNormal: [0, 0, 1],
      firstLayerDistance: 1.0,
      layerStep: 2.0,
      numLayers: 4, // z = 1.0, 3.0, 5.0, 7.0
    })

    // 4 layers * 8 segments per layer = 32 segments
    expect(result.segmentCount).toBe(32)

    const grouped = groupSegmentsByLayer(result)
    expect(grouped.size).toBe(4)

    for (let layer = 0; layer < 4; layer++) {
      const layerSegs = grouped.get(layer)!
      expect(layerSegs.length).toBe(8)
      const area = computeSliceArea(layerSegs, [0, 0, 1])
      expect(area).toBeCloseTo(15.0, 4)
    }
  })

  it('slices a square pyramid and verifies quadratic area scaling across height', () => {
    // Square base [-2, 2] x [-2, 2] at z = 0 (area = 16), apex at [0, 0, 4]
    const vertices = new Float32Array([
      -2, -2, 0, // 0
       2, -2, 0, // 1
       2,  2, 0, // 2
      -2,  2, 0, // 3
       0,  0, 4, // 4 (apex)
    ])

    const indices = new Uint32Array([
      // Base (-Z)
      0, 2, 1,  0, 3, 2,
      // 4 side triangles (CCW outward)
      0, 1, 4,
      1, 2, 4,
      2, 3, 4,
      3, 0, 4,
    ])

    // Slice at z = 1.0 (3/4 of base width => side = 3, area = 9)
    // and z = 2.0 (1/2 of base width => side = 2, area = 4)
    // and z = 3.0 (1/4 of base width => side = 1, area = 1)
    const result = referenceMeshSlicer(vertices, indices, {
      planeNormal: [0, 0, 1],
      firstLayerDistance: 1.0,
      layerStep: 1.0,
      numLayers: 3,
    })

    expect(result.segmentCount).toBe(12) // 4 side triangles * 3 layers

    expect(computeSliceArea(result.segments, [0, 0, 1], 0)).toBeCloseTo(9.0, 4)
    expect(computeSliceArea(result.segments, [0, 0, 1], 1)).toBeCloseTo(4.0, 4)
    expect(computeSliceArea(result.segments, [0, 0, 1], 2)).toBeCloseTo(1.0, 4)
  })

  it('declares valid WGSL compute shader with layer interval culling and atomic append', () => {
    expect(MESH_SLICER_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(MESH_SLICER_WGSL).toContain('interp_edge')
    expect(MESH_SLICER_WGSL).toContain('atomicAdd(&segment_counter[0], 1u)')
    expect(MESH_SLICER_WGSL).toContain('cross(plane_n, tri_n)')
  })

  it('runGpuMeshSlicer executes and returns valid contour segments', async () => {
    const { vertices, indices } = createBoxMesh(2.0, 2.0, 2.0)
    const res = await runGpuMeshSlicer(vertices, indices, {
      firstLayerDistance: 1.0,
      numLayers: 1,
    })

    expect(res.segmentCount).toBe(8)
    expect(computeSliceArea(res.segments, [0, 0, 1])).toBeCloseTo(4.0, 4)
  })
})
