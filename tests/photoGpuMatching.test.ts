import {describe, expect, it, vi} from 'vitest'
import {parseMatchingPayload, runGpuMatching} from '../src/services/photogrammetry/gpuMatching'
import type {GpuComputeJob} from '../src/services/webgpuCompute'

vi.mock('../src/services/webgpuCompute', () => ({
  runGpuCompute: vi.fn(),
}))

import {runGpuCompute} from '../src/services/webgpuCompute'

/** Builds a minimal MAT1 blob matching the kernel's pack_payload layout. */
function payload(): Uint8Array {
  const counts = [2, 3, 1]
  const pairs: [number, number][] = [[0, 1], [0, 2]]
  const bytes = new Uint8Array(4 * 1024)
  const view = new DataView(bytes.buffer)
  let at = 0
  const u32 = (v: number) => { view.setUint32(at, v >>> 0, true); at += 4 }
  const f32 = (v: number) => { view.setFloat32(at, v, true); at += 4 }
  u32(0x314d4154) // 'MAT1'
  u32(counts.length)
  u32(pairs.length)
  u32(128)
  for (const count of counts) u32(count)
  let seed = 0
  for (const count of counts) {
    for (let i = 0; i < count * 128; i++) f32((seed += 0.001) % 1)
  }
  for (const [a, b] of pairs) { u32(a); u32(b) }
  return bytes.slice(0, at)
}

describe('matching payload parser', () => {
  it('reads the kernel layout field for field', () => {
    const parsed = parseMatchingPayload(payload())
    expect(parsed.imageCount).toBe(3)
    expect(parsed.pairCount).toBe(2)
    expect(parsed.featureCounts).toEqual([2, 3, 1])
    expect(parsed.featureOffset).toEqual([0, 2, 5])
    expect(parsed.descriptors.length).toBe(6 * 128)
    expect(parsed.pairs).toEqual([[0, 1], [0, 2]])
  })

  it('rejects a bad magic and a bad descriptor stride', () => {
    const blob = payload()
    blob[0] = 0
    expect(() => parseMatchingPayload(blob)).toThrow('magic')
    const blob2 = payload()
    blob2[12] = 64
    expect(() => parseMatchingPayload(blob2)).toThrow('stride')
  })
})

describe('gpu matching dispatch', () => {
  it('drives match_pair from the shared descriptor table and repacks the wire format', async () => {
    const mocked = vi.mocked(runGpuCompute)
    // rows output first, cols second, per dispatch: RowBest is 12 bytes (3 f32),
    // ColBest 8 bytes (2 f32).
    mocked.mockResolvedValue([
      new Float32Array(2 * 3), // pair (0,1) rows: 2 features
      new Float32Array(3 * 2), // pair (0,1) cols: 3 features
      new Float32Array(2 * 3), // pair (0,2) rows
      new Float32Array(1 * 2), // pair (0,2) cols
    ])
    const result = await runGpuMatching(payload(), 'wgsl-text')
    expect(mocked).toHaveBeenCalledTimes(1)
    const job = mocked.mock.calls[0]![0] as GpuComputeJob
    expect(job.entryPoint).toBe('match_pair')
    expect(job.wgsl).toBe('wgsl-text')
    expect(job.dispatches).toHaveLength(2)

    const first = job.dispatches[0]!
    expect(first.workgroups).toEqual([2 + 3, 1, 1])
    const params = new DataView(first.buffers[0]!.data as ArrayBuffer)
    expect(params.getUint32(0, true)).toBe(2)
    expect(params.getUint32(4, true)).toBe(3)
    expect(params.getUint32(8, true)).toBe(0) // image 0 offset
    expect(params.getUint32(12, true)).toBe(2) // image 1 offset
    // Both directions bind the same table object so the runtime uploads it once.
    expect(first.buffers[1]!.data).toBe(first.buffers[2]!.data)
    expect(first.buffers[3]!.outputBytes).toBe(2 * 12)
    expect(first.buffers[4]!.outputBytes).toBe(3 * 8)

    const second = job.dispatches[1]!
    expect(second.workgroups).toEqual([2 + 1, 1, 1])
    const params2 = new DataView(second.buffers[0]!.data as ArrayBuffer)
    expect(params2.getUint32(12, true)).toBe(5) // image 2 offset

    expect(result.length).toBe(2 * 12 + 3 * 8 + 2 * 12 + 1 * 8)
  })
})
