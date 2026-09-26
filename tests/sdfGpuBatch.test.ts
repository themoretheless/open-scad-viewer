import {describe, expect, it, vi} from 'vitest'

const calls = vi.hoisted(() => ({ rust: vi.fn(), gpu: vi.fn() }))
vi.mock('../src/services/geometry/kernel', () => ({ callGeometryRust: calls.rust }))
vi.mock('../src/services/webgpuCompute', () => ({ runGpuCompute: calls.gpu }))

import { prepareSdfGpuBatch, primeSdfGpuBatch, tessellateSdfGpuAware, type SdfField, type SdfGrid } from '../src/services/geometry/sdf'
import { runSdfSweepBatch, type SdfGpuBatchPayload } from '../src/services/sdfGpu'

const field: SdfField = { kind: 'sphere', center: [0, 0, 0], radius: 10 }
const grid: SdfGrid = { min: [-12, -12, -12], max: [12, 12, 12], cells: [8, 8, 8] }

const jobPayload = (id: number) => ({
  id,
  kinds: [0, 1],
  params: [1, 2, 3, 4],
  aux: [0],
  triangles: [0],
  min: grid.min,
  max: grid.max,
  cells: grid.cells,
})

describe('batch SDF GPU dispatch', () => {
  it('prepares all jobs in one kernel call and maps ids in input order', () => {
    calls.rust.mockImplementation((op: string) => {
      if (op === 'sdf_prepare_batch') return { jobs: [jobPayload(7), null, jobPayload(9)], wgsl: '@compute' }
      throw new Error(`unexpected ${op}`)
    })
    const batch = prepareSdfGpuBatch([
      { field, grid },
      { field: { kind: 'extrude', profile: { outer: [[0, 0], [1, 0], [1, 1]] }, half_height: 1 }, grid },
      { field, grid },
    ])
    expect(calls.rust).toHaveBeenCalledWith('sdf_prepare_batch', { jobs: expect.any(Array) })
    expect(batch?.ids).toEqual([7, null, 9])
    expect(batch?.payload.jobs.map(job => job?.id ?? null)).toEqual([7, null, 9])
    expect(batch?.payload.wgsl).toBe('@compute')
  })

  it('returns null when the kernel declines the batch', () => {
    calls.rust.mockImplementation((op: string) => {
      if (op === 'sdf_prepare_batch') return null
      throw new Error(`unexpected ${op}`)
    })
    expect(prepareSdfGpuBatch([{ field, grid }])).toBeNull()
  })

  it('runs every eligible job in one GPU session and restores input order', async () => {
    calls.gpu.mockImplementation(async (job: { dispatches: unknown[] }) => {
      expect(job.dispatches).toHaveLength(2)
      return [new Float32Array([1, 2]), new Float32Array([3, 4])]
    })
    const payload: SdfGpuBatchPayload = {
      jobs: [jobPayload(7), null, jobPayload(9)],
      wgsl: '@compute',
    }
    const values = await runSdfSweepBatch(payload)
    expect(calls.gpu).toHaveBeenCalledTimes(1)
    expect(values.map(part => part ? Array.from(part) : null)).toEqual([[1, 2], null, [3, 4]])
  })

  it('encodes the params uniform per job (n_nodes is the kinds length)', async () => {
    let seen: DataView | undefined
    calls.gpu.mockImplementation(async (job: { dispatches: { buffers: { binding: number, data: ArrayBuffer, uniform?: boolean }[] }[] }) => {
      const uniform = job.dispatches[0]!.buffers.find(buffer => buffer.uniform)!
      seen = new DataView(uniform.data)
      return [new Float32Array(0)]
    })
    await runSdfSweepBatch({ jobs: [jobPayload(7)], wgsl: '@compute' })
    expect(seen?.getUint32(0, true)).toBe(8)
    expect(seen?.getUint32(12, true)).toBe(2)
    expect(seen?.getFloat32(16, true)).toBe(-12)
  })

  it('skips the GPU session entirely when every slot is null', async () => {
    calls.gpu.mockClear()
    const values = await runSdfSweepBatch({ jobs: [null, null], wgsl: '@compute' })
    expect(calls.gpu).not.toHaveBeenCalled()
    expect(values).toEqual([null, null])
  })

  it('primes sampled jobs so tessellation finishes them via sdf_finish', () => {
    calls.rust.mockImplementation((op: string) => {
      if (op === 'sdf_prepare_batch') return { jobs: [jobPayload(7), null, jobPayload(9)], wgsl: '@compute' }
      if (op === 'sdf_finish') return { positions: [0, 0, 0, 1, 0, 0, 0, 1, 0], indices: [0, 1, 2] }
      throw new Error(`unexpected ${op}`)
    })
    const jobs = [
      { field, grid },
      { field, grid: { ...grid, cells: [4, 4, 4] as number[] } },
      { field, grid: { ...grid, cells: [6, 6, 6] as number[] } },
    ]
    const batch = prepareSdfGpuBatch(jobs)!
    primeSdfGpuBatch(jobs, batch.ids, [new Float32Array(4), null, new Float32Array(4)])
    tessellateSdfGpuAware(jobs[0]!.field, jobs[0]!.grid)
    tessellateSdfGpuAware(jobs[2]!.field, jobs[2]!.grid)
    expect(calls.rust).toHaveBeenCalledWith('sdf_finish', { id: 7, values: expect.any(Float32Array) })
    expect(calls.rust).toHaveBeenCalledWith('sdf_finish', { id: 9, values: expect.any(Float32Array) })
    expect(calls.rust).not.toHaveBeenCalledWith('sdf_finish', expect.objectContaining({ id: null }))
  })
})
