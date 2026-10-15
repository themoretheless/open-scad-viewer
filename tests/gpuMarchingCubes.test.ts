import { describe, expect, it } from 'vitest'
import {
  CUBE_TETRAHEDRA,
  CORNER_OFFSETS,
  MARCHING_TETRAHEDRA_WGSL,
  referenceMarchingTetrahedra,
  type MarchingGrid,
} from '../src/services/gpuMarchingCubes'

describe('gpuMarchingCubes', () => {
  it('has 6 valid tetrahedra covering every cube corner', () => {
    expect(CUBE_TETRAHEDRA.length).toBe(6)
    const cornersUsed = new Set<number>()
    for (const tet of CUBE_TETRAHEDRA) {
      expect(tet.length).toBe(4)
      for (const c of tet) cornersUsed.add(c)
    }
    // All 8 corners must participate
    expect(cornersUsed.size).toBe(8)
  })

  it('declares 8 corner offsets in (x, y, z) bit order', () => {
    expect(CORNER_OFFSETS.length).toBe(8)
    for (let c = 0; c < 8; c++) {
      const [dx, dy, dz] = CORNER_OFFSETS[c]!
      expect(dx).toBe(c & 1)
      expect(dy).toBe((c >> 1) & 1)
      expect(dz).toBe((c >> 2) & 1)
    }
  })

  it('contains valid WGSL compute entrypoint with 256 workgroup size', () => {
    expect(MARCHING_TETRAHEDRA_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(MARCHING_TETRAHEDRA_WGSL).toContain('const WG: u32 = 256;')
    expect(MARCHING_TETRAHEDRA_WGSL).toContain('fn main(@builtin(global_invocation_id)')
    expect(MARCHING_TETRAHEDRA_WGSL).toContain('emit_tri')
    expect(MARCHING_TETRAHEDRA_WGSL).toContain('interpolate_point')
  })

  it('extracts a closed spherical isosurface from a sample SDF grid', () => {
    const radius = 5.0
    const grid: MarchingGrid = {
      min: [-8, -8, -8],
      max: [8, 8, 8],
      cells: [16, 16, 16],
      isoLevel: 0.0,
    }

    const [nx, ny, nz] = grid.cells
    const row = nx + 1
    const slice = row * (ny + 1)
    const totalPoints = row * (ny + 1) * (nz + 1)

    const stepX = (grid.max[0] - grid.min[0]) / nx
    const stepY = (grid.max[1] - grid.min[1]) / ny
    const stepZ = (grid.max[2] - grid.min[2]) / nz

    const values = new Float32Array(totalPoints)

    // Fill signed distance field of sphere: length(p) - radius (negative inside)
    for (let z = 0; z <= nz; z++) {
      for (let y = 0; y <= ny; y++) {
        for (let x = 0; x <= nx; x++) {
          const px = grid.min[0] + x * stepX
          const py = grid.min[1] + y * stepY
          const pz = grid.min[2] + z * stepZ
          const d = Math.hypot(px, py, pz) - radius
          values[x + y * row + z * slice] = d
        }
      }
    }

    const result = referenceMarchingTetrahedra(grid, values)
    expect(result.triangleCount).toBeGreaterThan(100)
    expect(result.indices.length).toBe(result.triangleCount * 3)
    expect(result.positions.length).toBe(result.triangleCount * 9)
    expect(result.normals.length).toBe(result.triangleCount * 9)

    // Verify all generated vertex positions lie approximately on the sphere surface (radius = 5.0)
    for (let i = 0; i < result.positions.length; i += 3) {
      const vx = result.positions[i]!
      const vy = result.positions[i + 1]!
      const vz = result.positions[i + 2]!
      const dist = Math.hypot(vx, vy, vz)
      expect(dist).toBeCloseTo(radius, 0.5)
    }

    // Verify divergence-theorem signed volume is positive and approximates 4/3 * pi * r^3
    let volume = 0
    for (let t = 0; t < result.triangleCount; t++) {
      const base = t * 9
      const ax = result.positions[base + 0]!
      const ay = result.positions[base + 1]!
      const az = result.positions[base + 2]!

      const bx = result.positions[base + 3]!
      const by = result.positions[base + 4]!
      const bz = result.positions[base + 5]!

      const cx = result.positions[base + 6]!
      const cy = result.positions[base + 7]!
      const cz = result.positions[base + 8]!

      // a . (b x c)
      const crossX = by * cz - bz * cy
      const crossY = bz * cx - bx * cz
      const crossZ = bx * cy - by * cx
      volume += (ax * crossX + ay * crossY + az * crossZ) / 6.0
    }

    const expectedVolume = (4.0 / 3.0) * Math.PI * Math.pow(radius, 3)
    expect(volume).toBeGreaterThan(0)
    expect(volume).toBeCloseTo(expectedVolume, -2) // Within 5% error on 16^3 grid
  })

  it('produces 0 triangles when whole field is positive or whole field is negative', () => {
    const grid: MarchingGrid = {
      min: [0, 0, 0],
      max: [10, 10, 10],
      cells: [4, 4, 4],
      isoLevel: 0.0,
    }

    const total = 5 * 5 * 5
    const allPositive = new Float32Array(total).fill(2.5)
    const resPos = referenceMarchingTetrahedra(grid, allPositive)
    expect(resPos.triangleCount).toBe(0)

    const allNegative = new Float32Array(total).fill(-2.5)
    const resNeg = referenceMarchingTetrahedra(grid, allNegative)
    expect(resNeg.triangleCount).toBe(0)
  })
})
