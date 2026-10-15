import { describe, it, expect, vi } from 'vitest'
import {
  alignByteSize,
  packIndirectIndexedArgs,
  createZeroCopyMeshBuffers,
  destroyZeroCopyMeshBuffers,
  dispatchComputeToZeroCopyMesh,
  supportsOffscreenWebGpu,
  GpuBufferArena,
} from '../src/services/gpuZeroCopyPipeline'

function createMockGpuDevice() {
  const createdBuffers: Array<{ size: number; usage: number; destroyed: boolean }> = []

  const device = {
    createBuffer: vi.fn(({ size, usage }: { size: number; usage: number }) => {
      const record = { size, usage, destroyed: false }
      createdBuffers.push(record)
      return {
        size,
        usage,
        destroy: vi.fn(() => {
          record.destroyed = true
        }),
      } as unknown as GPUBuffer
    }),
    queue: {
      writeBuffer: vi.fn(),
    },
  } as unknown as GPUDevice

  return { device, createdBuffers }
}

describe('Zero-Copy WebGPU Pipeline & VRAM Buffer Arena', () => {
  it('aligns byte sizes upward to the specified power-of-two boundary', () => {
    expect(alignByteSize(0, 16)).toBe(16)
    expect(alignByteSize(1, 16)).toBe(16)
    expect(alignByteSize(16, 16)).toBe(16)
    expect(alignByteSize(17, 16)).toBe(32)
    expect(alignByteSize(250, 256)).toBe(256)
    expect(alignByteSize(260, 256)).toBe(512)
  })

  it('packs drawIndexedIndirect arguments into 5 uint32 words', () => {
    const packed = packIndirectIndexedArgs({
      indexCount: 360,
      instanceCount: 12,
      firstIndex: 0,
      baseVertex: 0,
      firstInstance: 2,
    })

    expect(packed.length).toBe(5)
    expect(packed[0]).toBe(360)
    expect(packed[1]).toBe(12)
    expect(packed[2]).toBe(0)
    expect(packed[3]).toBe(0)
    expect(packed[4]).toBe(2)
  })

  it('allocates dual-usage zero-copy mesh buffers (STORAGE | VERTEX / INDEX / INDIRECT)', () => {
    const { device, createdBuffers } = createMockGpuDevice()

    const buffers = createZeroCopyMeshBuffers(device, 100, 300, 8)
    expect(createdBuffers.length).toBe(3)

    // 1. Vertex buffer: STORAGE (0x80) | VERTEX (0x20)
    expect(createdBuffers[0]!.usage & 0x80).toBe(0x80)
    expect(createdBuffers[0]!.usage & 0x20).toBe(0x20)
    expect(buffers.vertexByteSize).toBe(100 * 8 * 4)

    // 2. Index buffer: STORAGE (0x80) | INDEX (0x10)
    expect(createdBuffers[1]!.usage & 0x80).toBe(0x80)
    expect(createdBuffers[1]!.usage & 0x10).toBe(0x10)
    expect(buffers.indexByteSize).toBe(300 * 4)

    // 3. Indirect buffer: STORAGE (0x80) | INDIRECT (0x100)
    expect(createdBuffers[2]!.usage & 0x80).toBe(0x80)
    expect(createdBuffers[2]!.usage & 0x100).toBe(0x100)

    destroyZeroCopyMeshBuffers(buffers)
    expect(createdBuffers.every(b => b.destroyed)).toBe(true)
  })

  it('dispatches compute shader directly into zero-copy buffers and writes indirect draw args', () => {
    const { device } = createMockGpuDevice()
    const buffers = createZeroCopyMeshBuffers(device, 64, 192, 8)

    const passMock = {
      setPipeline: vi.fn(),
      setBindGroup: vi.fn(),
      dispatchWorkgroups: vi.fn(),
      end: vi.fn(),
    }
    const encoderMock = {
      beginComputePass: vi.fn(() => passMock),
    } as unknown as GPUCommandEncoder

    const pipelineMock = {} as GPUComputePipeline
    const bindGroupMock = {} as GPUBindGroup

    dispatchComputeToZeroCopyMesh(
      device,
      encoderMock,
      pipelineMock,
      bindGroupMock,
      [4, 4, 1],
      buffers,
      {
        indexCount: 192,
        instanceCount: 1,
        firstIndex: 0,
        baseVertex: 0,
        firstInstance: 0,
      }
    )

    expect(device.queue.writeBuffer).toHaveBeenCalledTimes(1)
    expect(passMock.setPipeline).toHaveBeenCalledWith(pipelineMock)
    expect(passMock.setBindGroup).toHaveBeenCalledWith(0, bindGroupMock)
    expect(passMock.dispatchWorkgroups).toHaveBeenCalledWith(4, 4, 1)
    expect(passMock.end).toHaveBeenCalledTimes(1)
  })

  it('reuses pooled buffers in GpuBufferArena without re-allocating on GPUDevice', () => {
    const { device, createdBuffers } = createMockGpuDevice()
    const arena = new GpuBufferArena(device)

    // Acquire a 300-byte buffer -> rounds up to 512-byte bucket
    const b1 = arena.acquire(300, 0x80)
    expect(createdBuffers.length).toBe(1)
    expect(createdBuffers[0]!.size).toBe(512)
    expect(arena.getStats().activeBuffers).toBe(1)

    // Release back to pool
    arena.release(300, 0x80, b1)
    expect(arena.getStats().activeBuffers).toBe(0)
    expect(arena.getStats().pooledBuffers).toBe(1)

    // Acquire a 450-byte buffer with same usage -> hits the 512-byte bucket!
    const b2 = arena.acquire(450, 0x80)
    expect(b2).toBe(b1)
    expect(createdBuffers.length).toBe(1) // Zero new GPU allocations!

    arena.release(450, 0x80, b2)
    arena.destroy()
    expect(createdBuffers[0]!.destroyed).toBe(true)
  })

  it('reports OffscreenCanvas WebGPU support boolean', () => {
    expect(typeof supportsOffscreenWebGpu()).toBe('boolean')
  })
})
