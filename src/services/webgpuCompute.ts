/**
 * Shared WebGPU compute dispatch for the browser GPU stages (photogrammetry
 * sweep, SDF grid sampling). The kernels ship their own qualified WGSL; this
 * module only owns device/buffer/dispatch plumbing — no algorithm lives here.
 */

export interface GpuDispatchBuffers {
  /** Bound by index; `uniform` marks the params buffer, `output` the scores. */
  binding: number
  data: ArrayBufferView | ArrayBuffer
  uniform?: boolean
  output?: boolean
}
export interface GpuComputeJob {
  wgsl: string
  entryPoint: string
  /** One entry per compute dispatch sharing the pipeline; each writes its own
   * output buffer. All dispatches go into one pass and one submit. */
  dispatches: { buffers: GpuDispatchBuffers[]; outputBytes: number; workgroups: [number, number, number] }[]
}

/** Runs the job; returns one Float32Array per dispatch, in order. */
export async function runGpuCompute(job: GpuComputeJob): Promise<Float32Array[]> {
  const adapter = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance' })
  if (!adapter) throw new Error('WebGPU adapter unavailable')
  const device = await adapter.requestDevice()
  const scratch: GPUBuffer[] = []
  try {
    const module = device.createShaderModule({ code: job.wgsl })
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
    const reads: { from: GPUBuffer; read: GPUBuffer; bytes: number }[] = []
    for (const dispatch of job.dispatches) {
      const entries: GPUBindGroupEntry[] = []
      let output: GPUBuffer | undefined
      for (const buffer of dispatch.buffers) {
        if (buffer.output) {
          output = device.createBuffer({
            size: dispatch.outputBytes,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
          })
          scratch.push(output)
        } else {
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
          continue
        }
        entries.push({ binding: buffer.binding, resource: { buffer: output! } })
      }
      const read = device.createBuffer({
        size: dispatch.outputBytes,
        usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
      })
      scratch.push(read)
      pass.setBindGroup(0, device.createBindGroup({ layout, entries }))
      pass.dispatchWorkgroups(...dispatch.workgroups)
      reads.push({ from: output!, read, bytes: dispatch.outputBytes })
    }
    pass.end()
    for (const { from, read, bytes } of reads) {
      encoder.copyBufferToBuffer(from, 0, read, 0, bytes)
    }
    device.queue.submit([encoder.finish()])
    await Promise.all(reads.map(({ read }) => read.mapAsync(GPUMapMode.READ)))
    return reads.map(({ read }) => {
      const values = new Float32Array(read.getMappedRange().slice(0))
      read.unmap()
      return values
    })
  } finally {
    scratch.forEach(buffer => buffer.destroy())
    device.destroy()
  }
}
