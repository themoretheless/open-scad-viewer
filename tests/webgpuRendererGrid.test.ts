import { afterEach, describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

class FakeBuffer {
  destroyCalls = 0
  readonly contents: Float32Array
  constructor(readonly size: number) { this.contents = new Float32Array(size / 4) }
  destroy() { this.destroyCalls++ }
}

class FakeDevice {
  readonly buffers: FakeBuffer[] = []
  readonly queue = {
    writeBuffer: (buffer: FakeBuffer, offset: number, data: Float32Array) => {
      buffer.contents.set(data, offset / 4)
    },
  }
  createBuffer(descriptor: { size: number }) {
    const buffer = new FakeBuffer(descriptor.size)
    this.buffers.push(buffer)
    return buffer
  }
}

interface GridInternals {
  dev: GPUDevice
  dist: number
  tx: number; ty: number; tz: number
  gridStep: number
  gridVB: FakeBuffer | null
  gridVC: number
  gridQuadVB: FakeBuffer | null
  buildGrid(): void
  gridExtent(): number
  gridFadeDistance(): number
  requestRender(): void
}

function harness() {
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
  const renderer = new WebGPURenderer()
  const device = new FakeDevice()
  const internal = renderer as unknown as GridInternals
  internal.dev = device as unknown as GPUDevice
  internal.requestRender = vi.fn()
  internal.buildGrid()
  return { renderer, device, internal }
}

describe('WebGPURenderer grid', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('builds a unit quad for the plane and a line list for the Z axis only', () => {
    const { internal } = harness()
    expect(internal.gridQuadVB!.contents).toHaveLength(12)
    expect(Math.max(...internal.gridQuadVB!.contents)).toBe(1)
    expect(Math.min(...internal.gridQuadVB!.contents)).toBe(-1)
    expect(internal.gridVC).toBe(2)
    const axis = internal.gridVB!.contents
    expect([axis[0], axis[1], axis[7], axis[8]]).toEqual([0, 0, 0, 0])
    expect(axis[2]).toBeLessThan(0)
    expect(axis[9]).toBeGreaterThan(0)
  })

  it('updates the step without rebuilding geometry and requests a frame', () => {
    const { renderer, internal, device } = harness()
    renderer.setGridStep(5)
    expect(internal.gridStep).toBe(5)
    expect(device.buffers).toHaveLength(2)
    expect(internal.requestRender).toHaveBeenCalledTimes(1)
  })

  it('ignores invalid and unchanged steps', () => {
    const { renderer, internal } = harness()
    renderer.setGridStep(0)
    renderer.setGridStep(-1)
    renderer.setGridStep(Number.NaN)
    renderer.setGridStep(10)
    expect(internal.gridStep).toBe(10)
    expect(internal.requestRender).not.toHaveBeenCalled()
  })

  it('scales the fade distance and plane extent with the orbit distance and target', () => {
    const { internal } = harness()
    internal.dist = 10
    internal.tx = 0; internal.ty = 0; internal.tz = 0
    expect(internal.gridFadeDistance()).toBe(200)
    expect(internal.gridExtent()).toBe(200)
    internal.dist = 500
    expect(internal.gridFadeDistance()).toBe(3000)
    internal.tx = 300; internal.ty = 400
    expect(internal.gridExtent()).toBe(3500)
  })
})
