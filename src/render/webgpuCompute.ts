/**
 * Shared WebGPU compute dispatch for the browser GPU stages (photogrammetry
 * sweep, SDF grid sampling). The kernels ship their own qualified WGSL; this
 * module only owns device/buffer/dispatch plumbing — no algorithm lives here.
 */
import {
  detectAdapterMemoryArchitecture,
  memoryArchitectureProfile,
  requiredWebGpuFeaturesForWgsl,
  requiredWgslLanguageFeatures,
  selectSupportedWgslVariant,
  unsupportedWebGpuFeatures,
  unsupportedWgslLanguageFeatures,
  type WebGpuMemoryArchitecture,
  type WebGpuMemoryArchitectureProfile,
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

interface CachedComputePipeline {
  layout: GPUBindGroupLayout
  pipeline: GPUComputePipeline
}

interface CachedComputeContext {
  adapter: GPUAdapter
  device: GPUDevice
  memoryArchitecture: WebGpuMemoryArchitecture
  memoryProfile: WebGpuMemoryArchitectureProfile
  featuresKey: string
  pipelines: Map<string, CachedComputePipeline>
}

let activeComputeContext: CachedComputeContext | null = null
let pendingContextPromise: Promise<CachedComputeContext> | null = null
let pendingFeaturesKey: string | null = null
let adapterCache = new WeakMap<GPU, Promise<GPUAdapter | null>>()

const wgslIdCache = new Map<string, number>()
let nextWgslId = 1

function getWgslId(wgsl: string): number {
  let id = wgslIdCache.get(wgsl)
  if (id === undefined) {
    id = nextWgslId++
    wgslIdCache.set(wgsl, id)
  }
  return id
}

function acquireGpuAdapter(gpu: GPU | undefined): Promise<GPUAdapter | null> {
  if (!gpu) return Promise.resolve(null)
  let promise = adapterCache.get(gpu)
  if (!promise) {
    promise = gpu.requestAdapter({ powerPreference: 'high-performance' }).then(
      adapter => {
        if (!adapter) adapterCache.delete(gpu)
        return adapter
      },
      err => {
        adapterCache.delete(gpu)
        throw err
      },
    )
    adapterCache.set(gpu, promise)
  }
  return promise
}

export function activeGpuMemoryArchitecture(): WebGpuMemoryArchitecture | null {
  return activeComputeContext?.memoryArchitecture ?? null
}

export function activeGpuMemoryProfile(): WebGpuMemoryArchitectureProfile | null {
  return activeComputeContext?.memoryProfile ?? null
}

export function resetGpuComputeContext(): void {
  const prev = activeComputeContext
  activeComputeContext = null
  pendingContextPromise = null
  pendingFeaturesKey = null
  adapterCache = new WeakMap<GPU, Promise<GPUAdapter | null>>()
  try {
    prev?.device.destroy?.()
  } catch {
    // Ignore errors when destroying lost or mock devices
  }
}

async function acquireComputeContext(
  adapter: GPUAdapter,
  requiredFeatures: ReturnType<typeof requiredWebGpuFeaturesForWgsl>,
  featuresKey: string,
): Promise<CachedComputeContext> {
  if (
    activeComputeContext &&
    activeComputeContext.adapter === adapter &&
    activeComputeContext.featuresKey === featuresKey
  ) {
    return activeComputeContext
  }
  if (pendingContextPromise && pendingFeaturesKey === featuresKey) {
    return pendingContextPromise
  }

  pendingFeaturesKey = featuresKey
  const promise = (async (): Promise<CachedComputeContext> => {
    const unsupported = unsupportedWebGpuFeatures(adapter, requiredFeatures)
    if (unsupported.length) {
      throw new Error(`WebGPU adapter lacks required feature(s): ${unsupported.join(', ')}`)
    }
    if (
      activeComputeContext &&
      activeComputeContext.adapter === adapter &&
      activeComputeContext.featuresKey === featuresKey
    ) {
      return activeComputeContext
    }
    const prevDevice = activeComputeContext?.device
    const device = await adapter.requestDevice({ requiredFeatures: requiredFeatures as GPUFeatureName[] })
    const memoryArchitecture = detectAdapterMemoryArchitecture(adapter)
    const context: CachedComputeContext = {
      adapter,
      device,
      memoryArchitecture,
      memoryProfile: memoryArchitectureProfile(memoryArchitecture),
      featuresKey,
      pipelines: new Map(),
    }
    device.lost?.then(() => {
      if (activeComputeContext?.device === device) {
        activeComputeContext = null
      }
    })
    activeComputeContext = context
    if (prevDevice && prevDevice !== device) {
      try {
        prevDevice.destroy?.()
      } catch {
        // Ignore cleanup errors on superseded device
      }
    }
    return context
  })()

  pendingContextPromise = promise
  try {
    return await promise
  } finally {
    if (pendingContextPromise === promise) {
      pendingContextPromise = null
      pendingFeaturesKey = null
    }
  }
}

/** Runs the job; returns one Float32Array per output buffer, in dispatch order. */
export async function runGpuCompute(job: GpuComputeJob): Promise<Float32Array[]> {
  const adapter = await acquireGpuAdapter(navigator.gpu)
  if (!adapter) throw new Error('WebGPU adapter unavailable')
  const selectedVariant = job.wgslVariants?.length
    ? selectSupportedWgslVariant(adapter, job.wgslVariants)
    : { label: 'baseline', wgsl: job.wgsl }
  const wgsl = selectedVariant.wgsl
  const requiredLanguageFeatures = requiredWgslLanguageFeatures(wgsl)
  const unsupportedLanguage = unsupportedWgslLanguageFeatures(requiredLanguageFeatures)
  if (unsupportedLanguage.length) {
    throw new Error(`WebGPU WGSL lacks required feature(s): ${unsupportedLanguage.join(', ')}`)
  }
  const requiredFeatures = requiredWebGpuFeaturesForWgsl(wgsl)
  const featuresKey = [...requiredFeatures].sort().join(',')

  const context = await acquireComputeContext(adapter, requiredFeatures, featuresKey)
  const device = context.device
  const scratch: GPUBuffer[] = []
  try {
    const layoutKey = job.dispatches[0]!.buffers
      .map(b => `${b.binding}:${b.uniform ? 'u' : b.output ? 's' : 'ro'}`)
      .join(',')
    const pipelineKey = `${job.entryPoint}:${layoutKey}:${getWgslId(wgsl)}`
    let cached = context.pipelines.get(pipelineKey)
    if (!cached) {
      const module = device.createShaderModule({ code: wgsl })
      const layout = device.createBindGroupLayout({
        entries: job.dispatches[0]!.buffers.map(buffer => ({
          binding: buffer.binding,
          visibility: GPUShaderStage.COMPUTE,
          buffer: buffer.uniform
            ? { type: 'uniform' as GPUBufferBindingType }
            : {
                type: buffer.output
                  ? ('storage' as GPUBufferBindingType)
                  : ('read-only-storage' as GPUBufferBindingType),
              },
        })),
      })
      const pipeline = device.createComputePipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [layout] }),
        compute: { module, entryPoint: job.entryPoint },
      })
      cached = { layout, pipeline }
      context.pipelines.set(pipelineKey, cached)
    }
    const { layout, pipeline } = cached

    // Shared inputs (same buffer object in several dispatches) upload once.
    const sharedUploads = new Map<ArrayBufferView | ArrayBuffer, GPUBuffer>()
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginComputePass()
    pass.setPipeline(pipeline)
    const reads: { from: GPUBuffer; read: GPUBuffer | null; bytes: number; alignedBytes: number }[] = []
    for (const dispatch of job.dispatches) {
      const entries: GPUBindGroupEntry[] = []
      for (const buffer of dispatch.buffers) {
        if (buffer.output) {
          const bytes = buffer.outputBytes ?? dispatch.outputBytes
          const alignedBytes = (bytes + 3) & ~3
          const output = device.createBuffer({
            size: Math.max(16, alignedBytes),
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
          })
          scratch.push(output)
          entries.push({ binding: buffer.binding, resource: { buffer: output } })
          if (bytes === 0) {
            // Legal to have empty outputs (e.g. zero features); skip the copy.
            reads.push({ from: output, read: null, bytes: 0, alignedBytes: 0 })
            continue
          }
          const read = device.createBuffer({
            size: alignedBytes,
            usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
          })
          scratch.push(read)
          reads.push({ from: output, read, bytes, alignedBytes })
          continue
        }
        const bytes = buffer.data
        let storage = sharedUploads.get(bytes)
        if (!storage) {
          const alignedInBytes = (bytes.byteLength + 3) & ~3
          const size = Math.max(16, alignedInBytes)
          const usage = buffer.uniform ? GPUBufferUsage.UNIFORM : GPUBufferUsage.STORAGE
          if (context.memoryProfile.mappedAtCreationPrimaryUpload) {
            // UMA (Unified Memory Architecture, e.g. Apple Silicon Metal):
            // initialize the primary buffer via mappedAtCreation so the host
            // writes directly into unified shared memory without an extra queue
            // staging copy when supported by the runtime.
            storage = device.createBuffer({
              size,
              usage: usage | GPUBufferUsage.COPY_DST,
              mappedAtCreation: true,
            })
            if (typeof storage.getMappedRange === 'function' && typeof storage.unmap === 'function') {
              const mapped = storage.getMappedRange(0, size)
              if (mapped && mapped.byteLength >= bytes.byteLength) {
                const src = ArrayBuffer.isView(bytes)
                  ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength)
                  : new Uint8Array(bytes)
                new Uint8Array(mapped, 0, src.byteLength).set(src)
                storage.unmap()
              } else {
                storage.unmap()
                device.queue.writeBuffer(storage, 0, bytes)
              }
            } else {
              storage.unmap?.()
              device.queue.writeBuffer(storage, 0, bytes)
            }
          } else {
            // dUMA (Discrete Memory Architecture): allocate device-local VRAM
            // and transfer via the queue staging ring.
            storage = device.createBuffer({
              size,
              usage: usage | GPUBufferUsage.COPY_DST,
            })
            device.queue.writeBuffer(storage, 0, bytes)
          }
          sharedUploads.set(bytes, storage)
          scratch.push(storage)
        }
        entries.push({ binding: buffer.binding, resource: { buffer: storage } })
      }
      pass.setBindGroup(0, device.createBindGroup({ layout, entries }))
      pass.dispatchWorkgroups(...(dispatch.variantWorkgroups?.[selectedVariant.label] ?? dispatch.workgroups))
    }
    pass.end()
    for (const { from, read, alignedBytes } of reads) {
      if (read) encoder.copyBufferToBuffer(from, 0, read, 0, alignedBytes)
    }
    device.queue.submit([encoder.finish()])
    await Promise.all(reads.map(({ read }) => read?.mapAsync(GPUMapMode.READ)))
    return reads.map(({ read, alignedBytes }) => {
      if (!read) return new Float32Array(0)
      const values = new Float32Array(read.getMappedRange().slice(0, alignedBytes))
      read.unmap()
      return values
    })
  } finally {
    scratch.forEach(buffer => buffer.destroy())
  }
}
