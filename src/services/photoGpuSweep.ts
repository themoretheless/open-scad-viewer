/**
 * Browser WebGPU execution of the kernel's NCC depth sweep. The WGSL shader
 * text and the payload layout come from the photogrammetry kernel itself
 * (photo_dense_prepare), so the browser runs exactly the qualified shader.
 * Everything before (sparse) and after (selection, consistency, fusion) runs
 * in the kernel as usual; this module only dispatches the scoring shader.
 */

export interface GpuSweepSource {
  image: number
  /** 16 f32: row-major rotation (9), translation (3), focal, cx, cy, pad. */
  floats: Float32Array
}
export interface GpuSweepView {
  mapWidth: number
  mapHeight: number
  needed: number
  refImage: number
  step: number
  refF: number
  refCx: number
  refCy: number
  hypotheses: Float32Array
  sources: GpuSweepSource[]
}
export interface GpuSweepPayload {
  imageWidth: number[]
  imageHeight: number[]
  imageOffset: number[]
  gray: Float32Array
  hypothesesPerView: number
  patchRadius: number
  views: (GpuSweepView | null)[]
}

/** Parses the kernel's SWP1 payload; all views share the concatenated grays. */
export function parseSweepPayload(blob: Uint8Array): GpuSweepPayload {
  const view = new DataView(blob.buffer, blob.byteOffset, blob.byteLength)
  let at = 0
  const u32 = () => {
    const v = view.getUint32(at, true)
    at += 4
    return v
  }
  const floats = (n: number) => {
    const out = new Float32Array(blob.buffer, blob.byteOffset + at, n)
    at += n * 4
    return out
  }
  if (u32() !== 0x31505753) throw new Error('Invalid sweep payload magic')
  const imageCount = u32()
  const hypothesesPerView = u32()
  const patchRadius = u32()
  const imageWidth: number[] = []
  const imageHeight: number[] = []
  const imageOffset: number[] = []
  let grayFloats = 0
  for (let i = 0; i < imageCount; i++) {
    imageWidth.push(u32())
    imageHeight.push(u32())
    imageOffset.push(grayFloats)
    grayFloats += imageWidth[i] * imageHeight[i]
  }
  // The gray block starts here; share one backing store with zero copies.
  const gray = floats(grayFloats)
  const viewCount = u32()
  const views: (GpuSweepView | null)[] = []
  for (let i = 0; i < viewCount; i++) {
    if (u32() === 0) {
      views.push(null)
      continue
    }
    const mapWidth = u32()
    const mapHeight = u32()
    const sourceCount = u32()
    const needed = u32()
    const refImage = u32()
    const [step, refF, refCx, refCy] = [...floats(4)]
    const hypotheses = floats(hypothesesPerView).slice()
    const sources: GpuSweepSource[] = []
    for (let s = 0; s < sourceCount; s++) {
      const floats16 = floats(16).slice()
      const image = u32()
      u32()
      u32()
      u32()
      sources.push({ image, floats: floats16 })
    }
    views.push({
      mapWidth, mapHeight, needed, refImage,
      step: step!, refF: refF!, refCx: refCx!, refCy: refCy!,
      hypotheses, sources,
    })
  }
  return { imageWidth, imageHeight, imageOffset, gray, hypothesesPerView, patchRadius, views }
}

/** Scores every present view with the kernel's shader; views stay in image order. */
export async function runGpuSweep(blob: Uint8Array, wgsl: string): Promise<Float32Array> {
  const payload = parseSweepPayload(blob)
  const adapter = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance' })
  if (!adapter) throw new Error('WebGPU adapter unavailable')
  const device = await adapter.requestDevice()
  const module = device.createShaderModule({ code: wgsl })
  const layout = device.createBindGroupLayout({
    entries: [0, 1, 2, 3, 4, 5].map(binding => ({
      binding,
      visibility: GPUShaderStage.COMPUTE,
      buffer: binding === 0
        ? { type: 'uniform' as GPUBufferBindingType }
        : { type: binding === 5 ? 'storage' as GPUBufferBindingType : 'read-only-storage' as GPUBufferBindingType },
    })),
  })
  const pipeline = device.createComputePipeline({
    layout: device.createPipelineLayout({ bindGroupLayouts: [layout] }),
    compute: { module, entryPoint: 'sweep' },
  })
  const grayBuffer = device.createBuffer({
    size: payload.gray.byteLength,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  })
  device.queue.writeBuffer(grayBuffer, 0, payload.gray)

  const patchLen = (payload.patchRadius * 2 + 1) ** 2
  // One encoder for all views: a single submit, then all readbacks map
  // concurrently instead of serializing the CPU against each view's GPU run.
  const scratch: GPUBuffer[] = [grayBuffer]
  const reads: { from: GPUBuffer, read: GPUBuffer, floats: number }[] = []
  try {
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginComputePass()
    pass.setPipeline(pipeline)
    for (const view of payload.views) {
      if (!view) continue
      const scoresFloats = view.mapWidth * view.mapHeight * payload.hypothesesPerView
      const params = new ArrayBuffer(64)
      const pu = new DataView(params)
      ;[
        view.mapWidth, view.mapHeight, payload.hypothesesPerView, view.sources.length,
        patchLen, payload.patchRadius, view.needed,
        payload.imageWidth[view.refImage]!, payload.imageHeight[view.refImage]!,
        payload.imageOffset[view.refImage]!, 0, 0,
      ].forEach((v, i) => pu.setUint32(i * 4, v, true))
      ;[view.step, view.refF, view.refCx, view.refCy].forEach((v, i) => pu.setFloat32(48 + i * 4, v, true))
      const srcf = new Float32Array(view.sources.length * 16)
      const srcm = new Uint32Array(view.sources.length * 4)
      view.sources.forEach((source, s) => {
        srcf.set(source.floats, s * 16)
        srcm.set([payload.imageOffset[source.image]!, payload.imageWidth[source.image]!, payload.imageHeight[source.image]!, 0], s * 4)
      })
      const mk = (data: Float32Array | Uint32Array | ArrayBuffer, usage: number) => {
        const buffer = device.createBuffer({
          size: Math.max(16, data.byteLength),
          usage: usage | GPUBufferUsage.COPY_DST,
        })
        device.queue.writeBuffer(buffer, 0, data)
        scratch.push(buffer)
        return buffer
      }
      const paramsBuffer = mk(params, GPUBufferUsage.UNIFORM)
      const hypBuffer = mk(view.hypotheses, GPUBufferUsage.STORAGE)
      const srcfBuffer = mk(srcf, GPUBufferUsage.STORAGE)
      const srcmBuffer = mk(srcm, GPUBufferUsage.STORAGE)
      const scoresBuffer = device.createBuffer({
        size: scoresFloats * 4,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
      })
      const readBuffer = device.createBuffer({
        size: scoresFloats * 4,
        usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
      })
      scratch.push(scoresBuffer, readBuffer)
      pass.setBindGroup(0, device.createBindGroup({
        layout,
        entries: [
          { binding: 0, resource: { buffer: paramsBuffer } },
          { binding: 1, resource: { buffer: hypBuffer } },
          { binding: 2, resource: { buffer: srcfBuffer } },
          { binding: 3, resource: { buffer: srcmBuffer } },
          { binding: 4, resource: { buffer: grayBuffer } },
          { binding: 5, resource: { buffer: scoresBuffer } },
        ],
      }))
      pass.dispatchWorkgroups(Math.ceil(view.mapWidth / 16), Math.ceil(view.mapHeight / 16))
      // Copies are recorded only after the pass ends; while it is open the
      // encoder rejects them and the readback would stay zeroed.
      reads.push({ from: scoresBuffer, read: readBuffer, floats: scoresFloats })
    }
    pass.end()
    for (const { from, read, floats } of reads) {
      encoder.copyBufferToBuffer(from, 0, read, 0, floats * 4)
    }
    device.queue.submit([encoder.finish()])
    await Promise.all(reads.map(({ read }) => read.mapAsync(GPUMapMode.READ)))
    const out = reads.map(({ read, floats }) => {
      const values = new Float32Array(read.getMappedRange().slice(0))
      read.unmap()
      return values
    })
    return flatten(out)
  } finally {
    scratch.forEach(buffer => buffer.destroy())
    device.destroy()
  }
}

function flatten(parts: Float32Array[]): Float32Array {
  const total = parts.reduce((sum, scores) => sum + scores.length, 0)
  const flat = new Float32Array(total)
  let offset = 0
  for (const scores of parts) {
    flat.set(scores, offset)
    offset += scores.length
  }
  return flat
}
