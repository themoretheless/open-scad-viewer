import { describe, it, expect } from 'vitest'
import {
  createBilinearNurbsPatch,
  createCylinderQuarterNurbsPatch,
  referenceNurbsTessellate,
  runGpuNurbsTessellate,
  generateQuadIndices,
  NURBS_TESSELLATE_WGSL,
  type GpuNurbsSurface,
} from '../src/services/gpuNurbsTessellate'

describe('WebGPU Compute NURBS Surface Tessellation', () => {
  it('generates correct quad index topology and winding', () => {
    const indices = generateQuadIndices(4, 4)
    // 3 x 3 quads = 9 quads, each quad = 2 triangles = 6 indices -> 54 indices
    expect(indices.length).toBe(3 * 3 * 6)

    // First quad indices: [0, 1, 4, 1, 5, 4]
    expect(indices[0]).toBe(0)
    expect(indices[1]).toBe(1)
    expect(indices[2]).toBe(4)

    expect(indices[3]).toBe(1)
    expect(indices[4]).toBe(5)
    expect(indices[5]).toBe(4)

    // All indices must be within 0..15
    for (const idx of indices) {
      expect(idx).toBeGreaterThanOrEqual(0)
      expect(idx).toBeLessThan(16)
    }
  })

  it('evaluates flat bilinear patch with exact geometry and upward normals', () => {
    const patch = createBilinearNurbsPatch(
      [0, 0, 0],
      [2, 0, 0],
      [0, 3, 0],
      [2, 3, 0]
    )

    const result = referenceNurbsTessellate(patch, 5, 5)
    expect(result.vertexCount).toBe(25)
    expect(result.triangleCount).toBe(32)

    // Verify corners
    // Vertex 0 (u=0, v=0)
    expect(result.vertices[0]).toBeCloseTo(0, 5)
    expect(result.vertices[1]).toBeCloseTo(0, 5)
    expect(result.vertices[2]).toBeCloseTo(0, 5)

    // Vertex 4 (u=1, v=0) -> stride 8 * 4 = 32
    expect(result.vertices[32]).toBeCloseTo(2, 5)
    expect(result.vertices[33]).toBeCloseTo(0, 5)
    expect(result.vertices[34]).toBeCloseTo(0, 5)

    // Vertex 20 (u=0, v=1) -> stride 8 * 20 = 160
    expect(result.vertices[160]).toBeCloseTo(0, 5)
    expect(result.vertices[161]).toBeCloseTo(3, 5)
    expect(result.vertices[162]).toBeCloseTo(0, 5)

    // Vertex 24 (u=1, v=1) -> stride 8 * 24 = 192
    expect(result.vertices[192]).toBeCloseTo(2, 5)
    expect(result.vertices[193]).toBeCloseTo(3, 5)
    expect(result.vertices[194]).toBeCloseTo(0, 5)

    // Check all normal vectors: must be (0, 0, 1)
    for (let i = 0; i < result.vertexCount; i++) {
      const off = i * 8
      const nx = result.vertices[off + 3]!
      const ny = result.vertices[off + 4]!
      const nz = result.vertices[off + 5]!
      expect(nx).toBeCloseTo(0, 5)
      expect(ny).toBeCloseTo(0, 5)
      expect(nz).toBeCloseTo(1, 5)
    }
  })

  it('evaluates exact rational quarter cylinder with constant radius and outward radial normals', () => {
    const radius = 2.5
    const height = 4.0
    const patch = createCylinderQuarterNurbsPatch(radius, height)

    const samplesU = 9
    const samplesV = 5
    const result = referenceNurbsTessellate(patch, samplesU, samplesV)

    expect(result.vertexCount).toBe(samplesU * samplesV)
    expect(result.triangleCount).toBe((samplesU - 1) * (samplesV - 1) * 2)

    for (let i = 0; i < result.vertexCount; i++) {
      const off = i * 8
      const x = result.vertices[off + 0]!
      const y = result.vertices[off + 1]!
      const z = result.vertices[off + 2]!
      const nx = result.vertices[off + 3]!
      const ny = result.vertices[off + 4]!
      const nz = result.vertices[off + 5]!

      // Circular radius x^2 + y^2 = radius^2
      const rComputed = Math.hypot(x, y)
      expect(rComputed).toBeCloseTo(radius, 4)

      // Z coordinate within [0, height]
      expect(z).toBeGreaterThanOrEqual(-1e-5)
      expect(z).toBeLessThanOrEqual(height + 1e-5)

      // Normal is purely radial: (x/r, y/r, 0)
      expect(nx).toBeCloseTo(x / radius, 4)
      expect(ny).toBeCloseTo(y / radius, 4)
      expect(nz).toBeCloseTo(0, 4)

      // Unit normal length
      expect(Math.hypot(nx, ny, nz)).toBeCloseTo(1.0, 4)
    }
  })

  it('evaluates bicubic B-spline surface with smooth variation', () => {
    // 4 x 4 control points for a bicubic saddle surface
    const cp: Array<[number, number, number]> = []
    for (let j = 0; j < 4; j++) {
      for (let i = 0; i < 4; i++) {
        const x = i
        const y = j
        const z = (i - 1.5) * (j - 1.5) // hyperbolic saddle
        cp.push([x, y, z])
      }
    }

    const surface: GpuNurbsSurface = {
      degreeU: 3,
      degreeV: 3,
      knotsU: [0, 0, 0, 0, 1, 1, 1, 1],
      knotsV: [0, 0, 0, 0, 1, 1, 1, 1],
      controlPoints: cp,
      numCpU: 4,
      numCpV: 4,
    }

    const result = referenceNurbsTessellate(surface, 16, 16)
    expect(result.vertexCount).toBe(256)
    expect(result.triangleCount).toBe(15 * 15 * 2)

    // Center point (u=0.5, v=0.5) must have z near 0 (since (1.5-1.5)=0)
    // Find vertex closest to tu=0.5, tv=0.5
    for (let i = 0; i < result.vertexCount; i++) {
      const off = i * 8
      const u = result.vertices[off + 6]!
      const v = result.vertices[off + 7]!
      if (Math.abs(u - 0.5) < 0.05 && Math.abs(v - 0.5) < 0.05) {
        const z = result.vertices[off + 2]!
        expect(Math.abs(z)).toBeLessThan(0.2)
      }
      // Normals must be unit length everywhere
      const nx = result.vertices[off + 3]!
      const ny = result.vertices[off + 4]!
      const nz = result.vertices[off + 5]!
      expect(Math.hypot(nx, ny, nz)).toBeCloseTo(1.0, 4)
    }
  })

  it('runGpuNurbsTessellate returns conformant vertices and indices', async () => {
    const patch = createBilinearNurbsPatch(
      [0, 0, 1],
      [1, 0, 1],
      [0, 1, 1],
      [1, 1, 1]
    )

    const result = await runGpuNurbsTessellate(patch, 8, 8)
    expect(result.vertexCount).toBe(64)
    expect(result.triangleCount).toBe(7 * 7 * 2)
    expect(result.vertices.length).toBe(64 * 8)
    expect(result.indices.length).toBe(7 * 7 * 6)

    // All Z coordinates must be 1.0
    for (let i = 0; i < result.vertexCount; i++) {
      expect(result.vertices[i * 8 + 2]).toBeCloseTo(1.0, 5)
    }
  })

  it('declares valid WGSL shader bindings and workgroup size', () => {
    expect(NURBS_TESSELLATE_WGSL).toContain('@compute @workgroup_size(16, 16, 1)')
    expect(NURBS_TESSELLATE_WGSL).toContain('fn find_knot_span_u')
    expect(NURBS_TESSELLATE_WGSL).toContain('fn find_knot_span_v')
    expect(NURBS_TESSELLATE_WGSL).toContain('fn eval_basis_u')
    expect(NURBS_TESSELLATE_WGSL).toContain('fn eval_basis_v')
    expect(NURBS_TESSELLATE_WGSL).toContain('out_vertices[v_off + 0u] = pos.x;')
    expect(NURBS_TESSELLATE_WGSL).toContain('out_indices[i_off + 0u] = v00;')
  })

  it('supports custom domain parameters [uMin, uMax] and [vMin, vMax]', () => {
    const patch = createBilinearNurbsPatch(
      [10, 20, 5],
      [30, 20, 5],
      [10, 40, 5],
      [30, 40, 5]
    )
    patch.domainU = [0.25, 0.75]
    patch.domainV = [0.25, 0.75]

    const result = referenceNurbsTessellate(patch, 3, 3)
    expect(result.vertexCount).toBe(9)

    // Center point at tu=0.5, tv=0.5 -> u=0.5, v=0.5 -> x=20, y=30, z=5
    // Vertex 4 (iu=1, iv=1) -> index 4 * 8 = 32
    expect(result.vertices[32 + 0]).toBeCloseTo(20, 5)
    expect(result.vertices[32 + 1]).toBeCloseTo(30, 5)
    expect(result.vertices[32 + 2]).toBeCloseTo(5, 5)
  })
})
