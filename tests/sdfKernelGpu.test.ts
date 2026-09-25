import {describe, expect, it, vi} from 'vitest'

const calls = vi.hoisted(() => ({ rust: vi.fn() }))
vi.mock('../src/services/geometry/kernel', () => ({ callGeometryRust: calls.rust }))

import { prepareSdfGpu, primeSdfGpu, tessellateSdfGpuAware, type SdfField, type SdfGrid } from '../src/services/geometry/sdf'

const field: SdfField = { kind: 'sphere', center: [0, 0, 0], radius: 10 }
const grid: SdfGrid = { min: [-12, -12, -12], max: [12, 12, 12], cells: [8, 8, 8] }

describe('GPU-aware SDF tessellation dispatch', () => {
  it('uses sdf_finish with the primed scores', () => {
    calls.rust.mockImplementation((op: string) => {
      if (op === 'sdf_prepare') return { id: 7 }
      if (op === 'sdf_finish') return { mesh: { indices: [0, 1, 2] } }
      throw new Error(`unexpected ${op}`)
    })
    const prepared = prepareSdfGpu(field, grid)
    expect(prepared?.id).toBe(7)
    primeSdfGpu({ field, grid }, 7, new Float32Array(4))
    tessellateSdfGpuAware(field, grid)
    // The primed Float32Array crosses the boundary as-is: encodeBinary reads
    // typed numeric arrays elementwise exactly like the equivalent plain array.
    expect(calls.rust).toHaveBeenCalledWith('sdf_finish', { id: 7, values: expect.any(Float32Array) })
  })

  it('falls back to sdf_tessellate when nothing was primed or prepared', () => {
    calls.rust.mockImplementation((op: string) => {
      if (op === 'sdf_prepare') return null
      if (op === 'sdf_tessellate') return { mesh: { indices: [] } }
      throw new Error(`unexpected ${op}`)
    })
    expect(prepareSdfGpu(field, grid)).toBeNull()
    tessellateSdfGpuAware(field, grid)
    expect(calls.rust).toHaveBeenCalledWith('sdf_tessellate', { field, grid })
  })
})
