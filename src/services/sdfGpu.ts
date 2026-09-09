/**
 * Browser WebGPU execution of the kernel's SDF grid sampler. The flat field
 * encoding and the shader text come from the kernel's sdf_prepare response, so
 * the browser runs exactly the qualified shader. Extraction (snap-to-zero,
 * boundary validation, marching tetrahedra) stays in the kernel on sdf_finish.
 */

export interface SdfGpuPayload {
  id: number
  kinds: number[]
  params: number[]
  aux: number[]
  triangles: number[]
  min: number[]
  max: number[]
  cells: number[]
  wgsl: string
}

/** Samples the whole grid with the kernel's SDF shader; returns f32 values. */
export async function runSdfSweep(payload: SdfGpuPayload): Promise<Float32Array> {
  const adapter = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance' })
  if (!adapter) throw new Error('WebGPU adapter unavailable')
  const device = await adapter.requestDevice()
  try {
    const [nx, ny, nz] = payload.cells as [number, number, number]
    const total = (nx + 1) * (ny + 1) * (nz + 1)
    const params = new ArrayBuffer(64)
    const view = new DataView(params)
    ;[nx, ny, nz, payload.kinds.length].forEach((v, i) => view.setUint32(i * 4, v, true))
    payload.min.forEach((v, i) => view.setFloat32(16 + i * 4, v, true))
    for (let i = 0; i < 3; i++) {
      view.setFloat32(28 + i * 4, (payload.max[i]! - payload.min[i]!) / payload.cells[i]!, true)
    }
    const module = device.createShaderModule({ code: payload.wgsl })
    const pipeline = device.createComputePipeline({
      layout: 'auto',
      compute: { module, entryPoint: 'main' },
    })
    const mk = (data: number[] | ArrayBuffer, usage: number) => {
      const isInt = Array.isArray(data)
      const bytes = isInt ? new Uint32Array(data as number[]) : data as ArrayBuffer
      const buffer = device.createBuffer({
        size: Math.max(16, bytes.byteLength),
        usage: usage | GPUBufferUsage.COPY_DST,
      })
      device.queue.writeBuffer(buffer, 0, bytes)
      return buffer
    }
    const paramsBuffer = mk(params, GPUBufferUsage.UNIFORM)
    const kindsBuffer = mk(payload.kinds, GPUBufferUsage.STORAGE)
    const nodesBuffer = mk(new Float32Array(payload.params).buffer, GPUBufferUsage.STORAGE)
    const auxBuffer = mk(payload.aux.length ? payload.aux : [0], GPUBufferUsage.STORAGE)
    const trisBuffer = mk(
      payload.triangles.length ? new Float32Array(payload.triangles).buffer : new Float32Array(1).buffer,
      GPUBufferUsage.STORAGE,
    )
    const outBuffer = device.createBuffer({
      size: total * 4,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    })
    const readBuffer = device.createBuffer({
      size: total * 4,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    })
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginComputePass()
    pass.setPipeline(pipeline)
    pass.setBindGroup(0, device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: paramsBuffer } },
        { binding: 1, resource: { buffer: kindsBuffer } },
        { binding: 2, resource: { buffer: nodesBuffer } },
        { binding: 3, resource: { buffer: auxBuffer } },
        { binding: 4, resource: { buffer: trisBuffer } },
        { binding: 5, resource: { buffer: outBuffer } },
      ],
    }))
    pass.dispatchWorkgroups(Math.ceil(total / 256))
    pass.end()
    encoder.copyBufferToBuffer(outBuffer, 0, readBuffer, 0, total * 4)
    device.queue.submit([encoder.finish()])
    await readBuffer.mapAsync(GPUMapMode.READ)
    const values = new Float32Array(readBuffer.getMappedRange().slice(0))
    readBuffer.unmap()
    for (const buffer of [paramsBuffer, kindsBuffer, nodesBuffer, auxBuffer, trisBuffer, outBuffer, readBuffer]) {
      buffer.destroy()
    }
    return values
  } finally {
    device.destroy()
  }
}
