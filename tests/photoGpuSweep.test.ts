import {describe, expect, it} from 'vitest'
import {parseSweepPayload} from '../src/services/photoGpuSweep'

/** Builds a minimal SWP1 blob matching the kernel's pack_sweep_payload layout. */
function payload(): Uint8Array {
  const bytes = new Uint8Array(4 * 200)
  const view = new DataView(bytes.buffer)
  let at = 0
  const u32 = (v: number) => { view.setUint32(at, v >>> 0, true); at += 4 }
  const f32 = (v: number) => { view.setFloat32(at, v, true); at += 4 }
  // Header: magic, 2 images, 16 hypotheses, patch radius 1.
  u32(0x31505753); u32(2); u32(16); u32(1)
  // Image headers first (0: 4x4, 1: 2x2), then the contiguous gray block.
  u32(4); u32(4); u32(2); u32(2)
  for (let i = 0; i < 16; i++) f32(i / 255)
  for (let i = 0; i < 4; i++) f32(1 - i / 255)
  // Views: view 0 absent, view 1 present with one source (image 0).
  u32(2)
  u32(0)
  u32(1); u32(64); u32(48); u32(1); u32(1); u32(1)
  f32(1.25); f32(2266.5); f32(480); f32(320)
  for (let i = 0; i < 16; i++) f32(2 + i / 16)
  for (let i = 0; i < 16; i++) f32(i)
  u32(0); u32(0); u32(0); u32(0)
  bytes.set([0], at) // touch to keep `at` honest about the written length
  return bytes.slice(0, at)
}

describe('sweep payload parser', () => {
  it('reads the kernel layout field for field', () => {
    const parsed = parseSweepPayload(payload())
    expect(parsed.imageWidth).toEqual([4, 2])
    expect(parsed.imageHeight).toEqual([4, 2])
    expect(parsed.imageOffset).toEqual([0, 16])
    expect(parsed.gray.length).toBe(20)
    expect(parsed.gray[19]).toBeCloseTo(1 - 3 / 255)
    expect(parsed.hypothesesPerView).toBe(16)
    expect(parsed.patchRadius).toBe(1)
    expect(parsed.views.length).toBe(2)
    expect(parsed.views[0]).toBeNull()
    const view = parsed.views[1]!
    expect(view.mapWidth).toBe(64)
    expect(view.mapHeight).toBe(48)
    expect(view.needed).toBe(1)
    expect(view.refImage).toBe(1)
    expect(view.step).toBeCloseTo(1.25)
    expect(view.hypotheses[0]).toBeCloseTo(2)
    expect(view.sources).toHaveLength(1)
    expect(view.sources[0]!.image).toBe(0)
    expect(view.sources[0]!.floats[15]).toBe(15)
  })

  it('rejects a bad magic', () => {
    const blob = payload()
    blob[0] = 0
    expect(() => parseSweepPayload(blob)).toThrow('magic')
  })
})
