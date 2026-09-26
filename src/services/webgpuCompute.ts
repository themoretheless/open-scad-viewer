/**
 * Shared WebGPU compute dispatch for the browser GPU stages (photogrammetry
 * sweep, SDF grid sampling). The kernels ship their own qualified WGSL; this
 * module only owns device/buffer/dispatch plumbing — no algorithm lives here.
 */
import {
  requiredWebGpuFeaturesForWgsl,
  requiredWgslLanguageFeatures,
  selectSupportedWgslVariant,
  unsupportedWebGpuFeatures,
  unsupportedWgslLanguageFeatures,
  type WgslVariant,
} from './webgpuFeatures'

export interface GpuDispatchBuffers {
  /** Bound by index; `uniform` marks the params buffer, `output` the scores. */
  binding: number
  data: ArrayBufferView | ArrayBuffer
  uniform?: boolean
  output?: boolean
  /** Byte size of this output buffer; defaults to the dispatch's `outputBytes`. */
  outputBytes?: number
}
export interface GpuComputeJob {
  wgsl: string
  /** Ordered from newest/fastest to most compatible. The first supported variant wins. */
  wgslVariants?: readonly WgslVariant[]
  entryPoint: string
  /** One entry per compute dispatch sharing the pipeline; each writes its own
   * output buffer(s). All dispatches go into one pass and one submit. */
  dispatches: {
    buffers: GpuDispatchBuffers[]
    /** Fallback byte size for output buffers without an explicit `outputBytes`. */
    outputBytes: number
    workgroups: [number, number, number]
    variantWorkgroups?: Record<string, [number, number, number]>
  }[]
}

/** Runs the job; returns one Float32Array per output buffer, in dispatch order. */
export async function runGpuCompute(job: GpuComputeJob): Promise<Float32Array[]> {
  const adapter = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance' })
  if (!adapter) throw new Error('WebGPU adapter unavailable')
  const selectedVariant = job.wgslVariants?.length
    ? selectSupportedWgslVariant(adapter, job.wgslVariants)
    : { label: 'baseline', wgsl: job.wgsl }
  const wgsl = selectedVariant.wgsl
  const requiredLanguageFeatures = requiredWgslLanguageFeatures(wgsl)
  const unsupportedLanguage = unsupportedWgslLanguageFeatures(requiredLanguageFeatures)
  if (unsupportedLanguage.length) throw new Error(`WebGPU WGSL lacks required feature(s): ${unsupportedLanguage.join(', ')}`)
  const requiredFeatures = requiredWebGpuFeaturesForWgsl(wgsl)
  const unsupported = unsupportedWebGpuFeatures(adapter, requiredFeatures)
  if (unsupported.length) throw new Error(`WebGPU adapter lacks required feature(s): ${unsupported.join(', ')}`)
  const device = await adapter.requestDevice({ requiredFeatures: requiredFeatures as GPUFeatureName[] })
  const scratch: GPUBuffer[] = []
  try {
    const module = device.createShaderModule({ code: wgsl })
    const layout = device.createBindGroupLayout({
      entries: job.dispatches[0]!.buffers.map(buffer => ({
        binding: buffer.binding,
        visibility: GPUShaderStage.COMPUTE,
        buffer: buffer.uniform
          ? { type: 'uniform' as GPUBufferBindingType }
          : { type: buffer.output ? 'storage' as GPUBufferBindingType : 'read-only-storage' as GPUBufferBindingType },
      })),
    })
    const pipeline = device.createComputePipeline({
      layout: device.createPipelineLayout({ bindGroupLayouts: [layout] }),
      compute: { module, entryPoint: job.entryPoint },
    })
    // Shared inputs (same buffer object in several dispatches) upload once.
    const sharedUploads = new Map<ArrayBufferView | ArrayBuffer, GPUBuffer>()
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginComputePass()
    pass.setPipeline(pipeline)
    const reads: { from: GPUBuffer; read: GPUBuffer | null; bytes: number }[] = []
    for (const dispatch of job.dispatches) {
      const entries: GPUBindGroupEntry[] = []
      for (const buffer of dispatch.buffers) {
        if (buffer.output) {
          const bytes = buffer.outputBytes ?? dispatch.outputBytes
          const output = device.createBuffer({
            size: Math.max(16, bytes),
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
          })
          scratch.push(output)
          entries.push({ binding: buffer.binding, resource: { buffer: output } })
          if (bytes === 0) {
            // Legal to have empty outputs (e.g. zero features); skip the copy.
            reads.push({ from: output, read: null, bytes })
            continue
          }
          const read = device.createBuffer({
            size: bytes,
            usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
          })
          scratch.push(read)
          reads.push({ from: output, read, bytes })
          continue
        }
        const bytes = buffer.data
        let storage = sharedUploads.get(bytes)
        if (!storage) {
          storage = device.createBuffer({
            size: Math.max(16, bytes.byteLength),
            usage: (buffer.uniform ? GPUBufferUsage.UNIFORM : GPUBufferUsage.STORAGE) | GPUBufferUsage.COPY_DST,
          })
          device.queue.writeBuffer(storage, 0, bytes)
          sharedUploads.set(bytes, storage)
          scratch.push(storage)
        }
        entries.push({ binding: buffer.binding, resource: { buffer: storage } })
      }
      pass.setBindGroup(0, device.createBindGroup({ layout, entries }))
      pass.dispatchWorkgroups(...(dispatch.variantWorkgroups?.[selectedVariant.label] ?? dispatch.workgroups))
    }
    pass.end()
    for (const { from, read, bytes } of reads) {
      if (read) encoder.copyBufferToBuffer(from, 0, read, 0, bytes)
    }
    device.queue.submit([encoder.finish()])
    await Promise.all(reads.map(({ read }) => read?.mapAsync(GPUMapMode.READ)))
    return reads.map(({ read, bytes }) => {
      if (!read) return new Float32Array(0)
      const values = new Float32Array(read.getMappedRange().slice(0, bytes))
      read.unmap()
      return values
    })
  } finally {
    scratch.forEach(buffer => buffer.destroy())
    device.destroy()
  }
}
