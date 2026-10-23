/**
 * Zero-Copy WebGPU Pipeline & Offscreen Worker Context Manager.
 *
 * Eliminates CPU-to-GPU and Worker-to-Main-Thread memory copies:
 * 1. Allocates dual-usage VRAM buffers (STORAGE | VERTEX and STORAGE | INDEX)
 *    so WebGPU Compute shaders write directly into renderable vertex/index buffers
 *    without reading back to CPU RAM.
 * 2. Provides a reusable 16-byte/256-byte aligned GPU Staging Arena to avoid
 *    per-frame buffer allocation overhead.
 * 3. Supports OffscreenCanvas WebGPU context initialization inside Web Workers.
 */

import {
  getGpuMemoryArchitectureProfile,
  type GpuMemoryArchitecture,
  type GpuMemoryArchitectureProfile,
} from '../renderer/types'

export { getGpuMemoryArchitectureProfile, type GpuMemoryArchitecture, type GpuMemoryArchitectureProfile }

// WebGPU standard GPUBufferUsage bitflags (fallback for headless Node.js environments)
const BUFFER_USAGE = {
  MAP_READ: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.MAP_READ : 0x0001,
  MAP_WRITE: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.MAP_WRITE : 0x0002,
  COPY_SRC: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.COPY_SRC : 0x0004,
  COPY_DST: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.COPY_DST : 0x0008,
  INDEX: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.INDEX : 0x0010,
  VERTEX: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.VERTEX : 0x0020,
  UNIFORM: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.UNIFORM : 0x0040,
  STORAGE: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.STORAGE : 0x0080,
  INDIRECT: typeof GPUBufferUsage !== 'undefined' ? GPUBufferUsage.INDIRECT : 0x0100,
}

/**
 * Detects whether a WebGPU device/adapter operates on Unified Memory Architecture (`'uma'`,
 * e.g. Apple Silicon Metal or integrated SoCs with mappable primary buffers) versus
 * Discrete / Decoupled Memory Architecture (`'duma'`, e.g. discrete PCIe GPUs or isolated staging pipelines).
 */
export function detectGpuMemoryArchitecture(
  adapterInfo?: { vendor?: string; architecture?: string; description?: string } | null,
  features?: { has(name: string): boolean } | null,
): GpuMemoryArchitecture {
  if (features?.has('mappable-primary-buffers')) {
    return 'uma'
  }
  const vendor = (adapterInfo?.vendor ?? '').toLowerCase()
  const arch = (adapterInfo?.architecture ?? '').toLowerCase()
  const desc = (adapterInfo?.description ?? '').toLowerCase()
  if (
    vendor.includes('apple') ||
    arch.includes('apple') ||
    arch.includes('metal') ||
    desc.includes('apple m')
  ) {
    return 'uma'
  }
  return 'duma'
}

/**
 * Upload a byte/typed-array payload into a newly allocated GPU buffer according to the
 * active memory architecture (`'uma'` vs `'duma'`):
 * - **UMA** (`mappedAtCreation: true` directly on the primary buffer): writes directly into
 *   unified CPU/GPU DRAM inside `getMappedRange()` and unmaps before returning — zero staging copy.
 * - **dUMA** (`mappedAtCreation: false` + `device.queue.writeBuffer`): allocates a private device-local
 *   VRAM buffer (`COPY_DST`) and schedules an asynchronous staging ring DMA transfer.
 */
export function createAndUploadBufferByArchitecture(
  device: GPUDevice,
  data: ArrayBufferView,
  usage: number,
  architecture: GpuMemoryArchitecture = 'duma',
): GPUBuffer {
  const byteSize = alignByteSize(data.byteLength, 16)
  if (architecture === 'uma') {
    const umaUsage = device.features?.has('mappable-primary-buffers')
      ? usage | BUFFER_USAGE.MAP_WRITE
      : usage
    const buffer = device.createBuffer({
      size: byteSize,
      usage: umaUsage,
      mappedAtCreation: true,
    })
    const dst = new Uint8Array(buffer.getMappedRange(0, byteSize))
    const src = new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
    dst.set(src)
    buffer.unmap()
    return buffer
  }

  const buffer = device.createBuffer({
    size: byteSize,
    usage: usage | BUFFER_USAGE.COPY_DST,
    mappedAtCreation: false,
  })
  device.queue.writeBuffer(
    buffer,
    0,
    data.buffer as ArrayBuffer,
    data.byteOffset,
    data.byteLength,
  )
  return buffer
}

export interface ZeroCopyMeshBuffers {
  /** GPU Buffer with STORAGE | VERTEX | COPY_SRC | COPY_DST usage */
  vertexBuffer: GPUBuffer
  /** GPU Buffer with STORAGE | INDEX | COPY_SRC | COPY_DST usage */
  indexBuffer: GPUBuffer
  /** GPU Buffer with STORAGE | INDIRECT | COPY_DST usage for drawIndexedIndirect */
  indirectDrawBuffer: GPUBuffer
  vertexByteSize: number
  indexByteSize: number
  maxVertices: number
  maxIndices: number
}

export interface IndirectIndexedDrawArgs {
  indexCount: number
  instanceCount: number
  firstIndex: number
  baseVertex: number
  firstInstance: number
}

/**
 * Aligns byte size upward to a power-of-two boundary (WebGPU requires 4-byte or 256-byte alignment).
 */
export function alignByteSize(bytes: number, alignment = 16): number {
  if (bytes <= 0) return alignment
  return Math.ceil(bytes / alignment) * alignment
}

/**
 * Packs drawIndexedIndirect arguments into a 20-byte (5 x uint32) buffer.
 */
export function packIndirectIndexedArgs(args: IndirectIndexedDrawArgs): Uint32Array {
  const arr = new Uint32Array(5)
  arr[0] = args.indexCount >>> 0
  arr[1] = args.instanceCount >>> 0
  arr[2] = args.firstIndex >>> 0
  // baseVertex is signed i32 in WebGPU drawIndexedIndirect
  arr[3] = args.baseVertex >>> 0
  arr[4] = args.firstInstance >>> 0
  return arr
}

/**
 * Allocates zero-copy VRAM buffers that can be written directly by a Compute Shader
 * (`STORAGE`) and immediately bound in a Render Pass (`VERTEX`, `INDEX`, `INDIRECT`)
 * with zero CPU readback.
 */
export function createZeroCopyMeshBuffers(
  device: GPUDevice,
  maxVertices: number,
  maxIndices: number,
  vertexStrideFloats = 8
): ZeroCopyMeshBuffers {
  const vertexByteSize = alignByteSize(maxVertices * vertexStrideFloats * 4, 16)
  const indexByteSize = alignByteSize(maxIndices * 4, 16)

  const vertexBuffer = device.createBuffer({
    size: vertexByteSize,
    usage:
      BUFFER_USAGE.STORAGE |
      BUFFER_USAGE.VERTEX |
      BUFFER_USAGE.COPY_SRC |
      BUFFER_USAGE.COPY_DST,
  })

  const indexBuffer = device.createBuffer({
    size: indexByteSize,
    usage:
      BUFFER_USAGE.STORAGE |
      BUFFER_USAGE.INDEX |
      BUFFER_USAGE.COPY_SRC |
      BUFFER_USAGE.COPY_DST,
  })

  const indirectDrawBuffer = device.createBuffer({
    size: 32, // 5 x u32 = 20 bytes, padded to 32 bytes
    usage:
      BUFFER_USAGE.STORAGE |
      BUFFER_USAGE.INDIRECT |
      BUFFER_USAGE.COPY_SRC |
      BUFFER_USAGE.COPY_DST,
  })

  return {
    vertexBuffer,
    indexBuffer,
    indirectDrawBuffer,
    vertexByteSize,
    indexByteSize,
    maxVertices,
    maxIndices,
  }
}

/**
 * Destroys zero-copy mesh buffers and releases VRAM.
 */
export function destroyZeroCopyMeshBuffers(buffers: ZeroCopyMeshBuffers): void {
  buffers.vertexBuffer.destroy()
  buffers.indexBuffer.destroy()
  buffers.indirectDrawBuffer.destroy()
}

/**
 * Dispatches a Compute Shader that writes directly into `ZeroCopyMeshBuffers`
 * in VRAM without any CPU readback.
 */
export function dispatchComputeToZeroCopyMesh(
  device: GPUDevice,
  encoder: GPUCommandEncoder,
  pipeline: GPUComputePipeline,
  bindGroup: GPUBindGroup,
  workgroups: [number, number, number],
  targetBuffers: ZeroCopyMeshBuffers,
  drawArgs?: IndirectIndexedDrawArgs
): void {
  if (drawArgs) {
    const packed = packIndirectIndexedArgs(drawArgs)
    device.queue.writeBuffer(targetBuffers.indirectDrawBuffer, 0, packed)
  }

  const pass = encoder.beginComputePass()
  pass.setPipeline(pipeline)
  pass.setBindGroup(0, bindGroup)
  pass.dispatchWorkgroups(workgroups[0], workgroups[1], workgroups[2])
  pass.end()
}

/**
 * Checks whether the current runtime environment supports OffscreenCanvas + WebGPU
 * for direct rendering from a Web Worker without touching the Main Thread.
 */
export function supportsOffscreenWebGpu(): boolean {
  return (
    typeof OffscreenCanvas !== 'undefined' &&
    typeof navigator !== 'undefined' &&
    typeof navigator.gpu !== 'undefined'
  )
}

/**
 * Reusable VRAM Buffer Pool / Arena to avoid repeated `device.createBuffer` allocations.
 */
export class GpuBufferArena {
  private readonly device: GPUDevice
  private readonly pool = new Map<number, GPUBuffer[]>()
  private allocatedBytes = 0
  private activeCount = 0

  constructor(device: GPUDevice) {
    this.device = device
  }

  private bucketSize(sizeBytes: number): number {
    const n = Math.max(256, Math.ceil(sizeBytes))
    return 1 << (32 - Math.clz32(n - 1))
  }

  private bucketKey(sizeBytes: number, usage: number): number {
    return this.bucketSize(sizeBytes) * 1024 + (usage & 0x3ff)
  }

  acquire(sizeBytes: number, usage: number): GPUBuffer {
    const key = this.bucketKey(sizeBytes, usage)
    const bucket = this.pool.get(key)
    if (bucket && bucket.length > 0) {
      this.activeCount++
      return bucket.pop()!
    }

    const actualSize = this.bucketSize(sizeBytes)
    const buf = this.device.createBuffer({
      size: actualSize,
      usage,
    })
    this.allocatedBytes += actualSize
    this.activeCount++
    return buf
  }

  release(sizeBytes: number, usage: number, buffer: GPUBuffer): void {
    const key = this.bucketKey(sizeBytes, usage)
    let bucket = this.pool.get(key)
    if (!bucket) {
      bucket = []
      this.pool.set(key, bucket)
    }
    bucket.push(buffer)
    this.activeCount = Math.max(0, this.activeCount - 1)
  }


  getStats(): { allocatedBytes: number; activeBuffers: number; pooledBuffers: number } {
    let pooled = 0
    for (const b of this.pool.values()) pooled += b.length
    return {
      allocatedBytes: this.allocatedBytes,
      activeBuffers: this.activeCount,
      pooledBuffers: pooled,
    }
  }

  destroy(): void {
    for (const bucket of this.pool.values()) {
      for (const buf of bucket) {
        buf.destroy()
      }
    }
    this.pool.clear()
    this.allocatedBytes = 0
    this.activeCount = 0
  }
}
