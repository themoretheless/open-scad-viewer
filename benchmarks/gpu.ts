import type { MeshData } from '../src/core/mesh'
import { identity } from '../src/services/math3d'
import { buildMeshBvh } from '../src/services/meshBvh'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

interface Options {
  triangles: number[]
  frames: number
  warmup: number
  uploads: number
  idleMs: number
  timestamps: boolean
}

interface Counters {
  buffersCreated: number
  bufferBytesCreated: number
  buffersDestroyed: number
  writes: number
  writeBytes: number
  submissions: number
  drawCalls: number
  indicesDrawn: number
}

const emptyCounters = (): Counters => ({
  buffersCreated: 0, bufferBytesCreated: 0, buffersDestroyed: 0,
  writes: 0, writeBytes: 0, submissions: 0, drawCalls: 0, indicesDrawn: 0,
})
const delta = (before: Counters, after: Counters): Counters => Object.fromEntries(
  Object.keys(before).map(key => [key, after[key as keyof Counters] - before[key as keyof Counters]]),
) as unknown as Counters
const sleep = (ms: number) => new Promise<void>(resolve => setTimeout(resolve, ms))

function summary(values: number[]) {
  if (!values.length) return null
  const ordered = values.toSorted((a, b) => a - b)
  const percentile = (p: number) => ordered[Math.min(ordered.length - 1, Math.ceil(ordered.length * p) - 1)]
  return { n: values.length, median: percentile(0.5), p95: percentile(0.95), min: ordered[0], max: ordered.at(-1)!, mean: values.reduce((a, b) => a + b, 0) / values.length }
}

interface FrameSample {
  renderCpuMs: number
  queueSubmitCpuMs: number
  queueCompletionMs: number
  renderPassGpuMs: number | null
}

/** Instrument native objects, retaining native device identity for canvas.configure. */
class GpuProbe {
  device!: GPUDevice
  adapterInfo: Record<string, unknown> = {}
  adapterFeatures: string[] = []
  timestampEnabled = false
  counters = emptyCounters()
  errors: string[] = []
  private activeFrame: { sample: FrameSample; resolve: (sample: FrameSample) => void; reject: (error: Error) => void } | null = null
  private originalRequestAdapter: GPU['requestAdapter'] | null = null
  private querySet: GPUQuerySet | null = null
  private queryResolve: GPUBuffer | null = null
  private queryReadback: GPUBuffer | null = null
  private timestampPassPending = false

  install(timestamps: boolean) {
    if (!navigator.gpu) throw new Error('WebGPU unavailable in this browser')
    const gpu = navigator.gpu
    this.originalRequestAdapter = gpu.requestAdapter.bind(gpu)
    gpu.requestAdapter = async (options?: GPURequestAdapterOptions) => {
      const adapter = await this.originalRequestAdapter!(options)
      if (!adapter) return null
      const info = adapter.info
      this.adapterInfo = Object.fromEntries(['vendor', 'architecture', 'device', 'description', 'isFallbackAdapter', 'subgroupMinSize', 'subgroupMaxSize'].map(key => [key, (info as unknown as Record<string, unknown>)[key] ?? null]))
      this.adapterFeatures = [...adapter.features]
      this.timestampEnabled = timestamps && adapter.features.has('timestamp-query')
      const requestDevice = adapter.requestDevice.bind(adapter)
      adapter.requestDevice = async (descriptor: GPUDeviceDescriptor = {}) => {
        const requiredFeatures = new Set(descriptor.requiredFeatures ?? [])
        if (this.timestampEnabled) requiredFeatures.add('timestamp-query')
        this.device = await requestDevice({ ...descriptor, requiredFeatures: [...requiredFeatures] })
        this.instrumentDevice()
        return this.device
      }
      return adapter
    }
  }

  private instrumentDevice() {
    const device = this.device
    device.addEventListener('uncapturederror', event => this.errors.push(event.error.message))
    void device.lost.then(info => { if (info.reason !== 'destroyed') this.errors.push(`Device lost: ${info.reason}: ${info.message}`) })
    // Query buffers are deliberately allocated before counters are installed.
    if (this.timestampEnabled) {
      this.querySet = device.createQuerySet({ type: 'timestamp', count: 2 })
      this.queryResolve = device.createBuffer({ size: 256, usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC })
      this.queryReadback = device.createBuffer({ size: 16, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ })
    }
    const createBuffer = device.createBuffer.bind(device)
    device.createBuffer = descriptor => {
      const buffer = createBuffer(descriptor)
      this.counters.buffersCreated++
      this.counters.bufferBytesCreated += descriptor.size
      const destroy = buffer.destroy.bind(buffer)
      let destroyed = false
      buffer.destroy = () => { if (!destroyed) this.counters.buffersDestroyed++; destroyed = true; destroy() }
      return buffer
    }
    const writeBuffer = device.queue.writeBuffer.bind(device.queue)
    device.queue.writeBuffer = (...args: Parameters<GPUQueue['writeBuffer']>) => {
      const [, , data, dataOffset = 0, size] = args
      const elementBytes = 'BYTES_PER_ELEMENT' in data ? Number(data.BYTES_PER_ELEMENT) : 1
      this.counters.writes++
      this.counters.writeBytes += size === undefined ? data.byteLength - dataOffset * elementBytes : size * elementBytes
      writeBuffer(...args)
    }
    const createCommandEncoder = device.createCommandEncoder.bind(device)
    device.createCommandEncoder = descriptor => {
      const encoder = createCommandEncoder(descriptor)
      const beginRenderPass = encoder.beginRenderPass.bind(encoder)
      let timestamped = false
      encoder.beginRenderPass = passDescriptor => {
        if (this.activeFrame && this.querySet && !timestamped) {
          passDescriptor = { ...passDescriptor, timestampWrites: { querySet: this.querySet, beginningOfPassWriteIndex: 0, endOfPassWriteIndex: 1 } }
          timestamped = true
        }
        const pass = beginRenderPass(passDescriptor)
        const draw = pass.draw.bind(pass), drawIndexed = pass.drawIndexed.bind(pass)
        pass.draw = (...args: Parameters<GPURenderPassEncoder['draw']>) => { this.counters.drawCalls++; draw(...args) }
        pass.drawIndexed = (...args: Parameters<GPURenderPassEncoder['drawIndexed']>) => { this.counters.drawCalls++; this.counters.indicesDrawn += args[0] * (args[1] ?? 1); drawIndexed(...args) }
        return pass
      }
      const finish = encoder.finish.bind(encoder)
      encoder.finish = finishDescriptor => {
        if (timestamped) {
          encoder.resolveQuerySet(this.querySet!, 0, 2, this.queryResolve!, 0)
          encoder.copyBufferToBuffer(this.queryResolve!, 0, this.queryReadback!, 0, 16)
          this.timestampPassPending = true
        }
        return finish(finishDescriptor)
      }
      return encoder
    }
    const submit = device.queue.submit.bind(device.queue)
    device.queue.submit = commandBuffers => {
      const started = performance.now()
      submit(commandBuffers)
      const submitted = performance.now()
      this.counters.submissions++
      const frame = this.activeFrame
      if (!frame) return
      this.activeFrame = null
      frame.sample.queueSubmitCpuMs = submitted - started
      const timestamped = this.timestampPassPending
      this.timestampPassPending = false
      void device.queue.onSubmittedWorkDone().then(async () => {
        frame.sample.queueCompletionMs = performance.now() - submitted
        if (timestamped) {
          await this.queryReadback!.mapAsync(GPUMapMode.READ)
          const values = new BigUint64Array(this.queryReadback!.getMappedRange())
          frame.sample.renderPassGpuMs = Number(values[1] - values[0]) / 1e6
          this.queryReadback!.unmap()
        }
        frame.resolve(frame.sample)
      }).catch(error => frame.reject(error instanceof Error ? error : new Error(String(error))))
    }
  }

  instrumentRenderer(renderer: WebGPURenderer) {
    // Benchmark-only hook around the exact production render method, never a replica.
    const internal = renderer as unknown as { render: () => void }
    const render = internal.render.bind(renderer)
    internal.render = () => {
      const frame = this.activeFrame
      const start = performance.now()
      render()
      if (frame) frame.sample.renderCpuMs = performance.now() - start
    }
  }

  async frame(action: () => void): Promise<FrameSample> {
    if (this.activeFrame) throw new Error('Concurrent measured frames are unsupported')
    let timer: ReturnType<typeof setTimeout> | undefined
    try {
      return await new Promise<FrameSample>((resolve, reject) => {
        timer = setTimeout(() => { this.activeFrame = null; reject(new Error('Renderer did not submit a frame within 10 seconds')) }, 10_000)
        this.activeFrame = { sample: { renderCpuMs: 0, queueSubmitCpuMs: 0, queueCompletionMs: 0, renderPassGpuMs: null }, resolve, reject }
        action()
      })
    } finally { clearTimeout(timer) }
  }

  async settle(renderer: WebGPURenderer) {
    // Drain the renderer's actual idle warmup before steady-state measurements.
    const internal = renderer as unknown as { edgeWarmQueue: unknown[]; edgeWarmHandle: number | null; raf: number }
    const deadline = performance.now() + 10_000
    while (internal.edgeWarmQueue.length || internal.edgeWarmHandle !== null || internal.raf) {
      if (performance.now() > deadline) throw new Error('Renderer warmup failed to settle')
      await sleep(20)
    }
    await this.device.queue.onSubmittedWorkDone()
  }

  restore() {
    if (this.originalRequestAdapter) navigator.gpu.requestAdapter = this.originalRequestAdapter
    this.querySet?.destroy(); this.queryResolve?.destroy(); this.queryReadback?.destroy()
  }
}

/** Deterministic indexed sphere; all tessellation edges exercise the real edges mode. */
function sphereMesh(targetTriangles: number, variant = 0): MeshData {
  const rings = Math.max(4, Math.round(Math.sqrt(targetTriangles / 4)))
  const segments = rings * 2
  const vertices = new Float32Array((rings + 1) * (segments + 1) * 6)
  for (let y = 0; y <= rings; y++) for (let x = 0; x <= segments; x++) {
    const theta = y / rings * Math.PI, phi = x / segments * Math.PI * 2
    const nx = Math.sin(theta) * Math.cos(phi), ny = Math.sin(theta) * Math.sin(phi), nz = Math.cos(theta)
    const offset = (y * (segments + 1) + x) * 6
    vertices.set([nx * (10 + variant * 0.01), ny * 10, nz * 10, nx, ny, nz], offset)
  }
  const indices = new Uint32Array(rings * segments * 6)
  const edges = new Uint32Array(rings * segments * 6)
  let cursor = 0
  for (let y = 0; y < rings; y++) for (let x = 0; x < segments; x++) {
    const a = y * (segments + 1) + x, b = a + segments + 1
    indices.set([a, b, a + 1, a + 1, b, b + 1], cursor)
    edges.set([a, b, a, a + 1, a + 1, b], cursor)
    cursor += 6
  }
  return {
    entityId: 'entity:benchmark-sphere', geometryAssetId: `asset:benchmark-${targetTriangles}-${variant}`,
    vertices, indices, edgeIndices: edges, bvh: buildMeshBvh(vertices, indices),
    faceIds: new Uint32Array(indices.length / 3), provenance: [],
    color: [0.9, 0.56, 0.2, 1], transform: identity(),
    topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: segments * 2 },
  }
}

function instances(mesh: MeshData, count: number): MeshData[] {
  const columns = Math.ceil(Math.sqrt(count))
  return Array.from({ length: count }, (_, index) => {
    const transform = identity()
    transform[3] = (index % columns - (columns - 1) / 2) * 24
    transform[7] = (Math.floor(index / columns) - (Math.ceil(count / columns) - 1) / 2) * 24
    return { ...mesh, entityId: `entity:benchmark-${index}`, transform }
  })
}

function frameSummary(samples: FrameSample[]) {
  return {
    renderCpuMs: summary(samples.map(sample => sample.renderCpuMs)),
    queueSubmitCpuMs: summary(samples.map(sample => sample.queueSubmitCpuMs)),
    queueCompletionMs: summary(samples.map(sample => sample.queueCompletionMs)),
    renderPassGpuMs: summary(samples.flatMap(sample => sample.renderPassGpuMs === null ? [] : [sample.renderPassGpuMs])),
  }
}

let disposeCurrent: (() => void) | null = null
let heapRenderer: WebGPURenderer | null = null
let heapErrors: string[] = []

async function prepareHeapWorkload(mode: 'shaded' | 'edges' | 'xray') {
  disposeCurrent?.()
  heapRenderer?.destroy()
  heapErrors = []
  heapRenderer = new WebGPURenderer()
  heapRenderer.onStatusChange = event => {
    if (event.status === 'error') heapErrors.push(event.error.message)
    if (event.status === 'device-lost') heapErrors.push(`Device lost: ${event.reason}: ${event.message}`)
  }
  if (!await heapRenderer.init(document.querySelector<HTMLCanvasElement>('#viewport')!)) throw new Error('Heap-pass renderer initialization failed')
  const meshes = instances(sphereMesh(5_000), 128)
  heapRenderer.setDisplayMode(mode)
  heapRenderer.setMeshes(meshes)
  heapRenderer.resetView()
  const internal = heapRenderer as unknown as { edgeWarmQueue: unknown[]; edgeWarmHandle: number | null; raf: number; dev: GPUDevice }
  internal.dev.addEventListener('uncapturederror', event => heapErrors.push(event.error.message))
  const deadline = performance.now() + 10_000
  while (internal.edgeWarmQueue.length || internal.edgeWarmHandle !== null || internal.raf) {
    if (performance.now() > deadline) throw new Error('Heap-pass renderer failed to settle')
    await sleep(20)
  }
  await internal.dev.queue.onSubmittedWorkDone()
  return { entities: meshes.length, trianglesPerAsset: meshes[0].indices.length / 3, mode, instrumentation: 'Native renderer and device; no GPU counter, timestamp, or render method wrappers' }
}

function runHeapWorkload(activity: 'camera' | 'scan-plane', frames: number): Promise<{ requestedFrames: number }> {
  const renderer = heapRenderer
  if (!renderer) return Promise.reject(new Error('Prepare a heap workload first'))
  const sectionNormal: [number, number, number] = [0, 0, 1]
  // One stable scheduler closure and one promise per pass; no per-frame sample arrays.
  return new Promise((resolve, reject) => {
    let completed = 0
    const tick = () => {
      try {
        // Previous tick's renderer callback was registered before this callback.
        if (heapErrors.length) { reject(new Error(`Heap-pass GPU error: ${heapErrors.join('; ')}`)); return }
        if (completed === frames) { resolve({ requestedFrames: frames }); return }
        if (activity === 'camera') { renderer.yaw += 0.012; renderer.requestRender() }
        else renderer.setSection(true, sectionNormal, -8 + 16 * (completed % 180) / 180)
        completed++
        requestAnimationFrame(tick)
      } catch (error) { reject(error) }
    }
    requestAnimationFrame(tick)
  })
}

async function run(options: Options) {
  disposeCurrent?.()
  const probe = new GpuProbe()
  const renderer = new WebGPURenderer()
  const lifecycle: unknown[] = []
  renderer.onStatusChange = event => lifecycle.push(event.status === 'error' ? { ...event, error: event.error.message } : event)
  const scenarios: unknown[] = []
  const startedAt = new Date().toISOString()
  let succeeded = false
  const dispose = () => { probe.restore(); renderer.destroy(); disposeCurrent = null }
  try {
    probe.install(options.timestamps)
    const initStarted = performance.now()
    if (!await renderer.init(document.querySelector<HTMLCanvasElement>('#viewport')!)) throw new Error(`Renderer initialization failed: ${JSON.stringify(lifecycle)}`)
    const initCpuAndDeviceMs = performance.now() - initStarted
    probe.instrumentRenderer(renderer)
    await probe.settle(renderer)
    for (const requestedTriangles of options.triangles) {
      const mesh = sphereMesh(requestedTriangles)
      renderer.setDisplayMode('shaded')
      await probe.settle(renderer)
      const setupCounters = { ...probe.counters }
      const setStart = performance.now()
      renderer.setMeshes([mesh])
      const coldSetMeshesCpuMs = performance.now() - setStart
      const upload = renderer.sceneUploadMetrics
      renderer.resetView()
      await probe.settle(renderer)
      scenarios.push({ kind: 'cold-publication', requestedTriangles, triangles: mesh.indices.length / 3, coldSetMeshesCpuMs, upload, countersIncludingEdgeWarmup: delta(setupCounters, probe.counters) })
      for (const mode of ['shaded', 'edges', 'xray'] as const) {
        renderer.setDisplayMode(mode)
        await probe.settle(renderer)
        for (const activity of ['static-redraw', 'camera', 'scan-plane'] as const) {
          const action = (index: number) => {
            if (activity === 'camera') renderer.yaw += 0.012
            if (activity === 'scan-plane') renderer.setSection(true, [0, 0, 1], -8 + 16 * (index % options.frames) / options.frames)
            else renderer.requestRender()
          }
          for (let index = 0; index < options.warmup; index++) await probe.frame(() => action(index))
          const before = { ...probe.counters }
          const samples: FrameSample[] = []
          for (let index = 0; index < options.frames; index++) samples.push(await probe.frame(() => action(index)))
          scenarios.push({ kind: 'frames', requestedTriangles, triangles: mesh.indices.length / 3, mode, activity, ...frameSummary(samples), counters: delta(before, probe.counters), samples })
          renderer.setSection(false, [0, 0, 1], 0)
          await probe.settle(renderer)
        }
      }
      const beforeIdle = { ...probe.counters }, idleStart = performance.now()
      await sleep(options.idleMs)
      scenarios.push({ kind: 'idle', triangles: mesh.indices.length / 3, elapsedMs: performance.now() - idleStart, counters: delta(beforeIdle, probe.counters) })
    }

    const shared = sphereMesh(5_000)
    const retained = instances(shared, 128)
    const publicationCount = options.uploads + 1
    // Every recompilation-like publication owns fresh array/transform identities.
    // Prepare them all outside measured setMeshes calls; never alternate two warmed cache entries.
    const equalClones = Array.from({ length: publicationCount }, () => instances({ ...shared, vertices: shared.vertices.slice(), indices: shared.indices.slice(), edgeIndices: shared.edgeIndices.slice() }, 128))
    const changed = Array.from({ length: publicationCount }, (_, index) => instances(sphereMesh(5_000, index + 1), 128))
    renderer.setDisplayMode('edges')
    renderer.setMeshes(retained)
    renderer.resetView()
    await probe.settle(renderer)
    for (const [activity, scenes] of [
      ['retained-arrays', Array.from({ length: publicationCount }, () => retained)],
      ['fresh-equal-arrays', equalClones],
      ['changed-geometry', changed],
    ] as const) {
      const samples: (FrameSample & { setMeshesCpuMs: number; upload: unknown; counters: Counters })[] = []
      for (let index = 0; index < options.uploads + 1; index++) {
        const before = { ...probe.counters }
        let setMeshesCpuMs = 0
        const sample = await probe.frame(() => {
          const start = performance.now()
          renderer.setMeshes(scenes[index])
          setMeshesCpuMs = performance.now() - start
        })
        await probe.settle(renderer)
        if (index) samples.push({ ...sample, setMeshesCpuMs, upload: renderer.sceneUploadMetrics, counters: delta(before, probe.counters) })
      }
      scenarios.push({ kind: 'shared-asset-publication', activity, warmupPublications: 1, entities: retained.length, trianglesPerAsset: shared.indices.length / 3, renderedTriangles: retained.length * shared.indices.length / 3, setMeshesCpuMs: summary(samples.map(sample => sample.setMeshesCpuMs)), ...frameSummary(samples), samples })
    }
    // Draw-call, opaque traversal and transparent sorting pressure at the same geometry scale.
    for (const mode of ['shaded', 'edges', 'xray'] as const) {
      renderer.setDisplayMode(mode)
      await probe.settle(renderer)
      for (const activity of ['camera', 'scan-plane'] as const) {
        const action = (index: number) => {
          if (activity === 'camera') { renderer.yaw += 0.012; renderer.requestRender() }
          else renderer.setSection(true, [0, 0, 1], -8 + 16 * (index % options.frames) / options.frames)
        }
        for (let index = 0; index < options.warmup; index++) await probe.frame(() => action(index))
        const sharedBefore = { ...probe.counters }, sharedSamples: FrameSample[] = []
        for (let index = 0; index < options.frames; index++) sharedSamples.push(await probe.frame(() => action(index)))
        scenarios.push({ kind: 'shared-asset-frames', entities: 128, mode, activity, ...frameSummary(sharedSamples), counters: delta(sharedBefore, probe.counters), samples: sharedSamples })
        renderer.setSection(false, [0, 0, 1], 0)
        await probe.settle(renderer)
      }
    }
    if (probe.errors.length) throw new Error(`WebGPU errors: ${probe.errors.join('; ')}`)
    succeeded = true
    disposeCurrent = dispose
    return {
      schema: 'open-scad-viewer-gpu-benchmark-v1', startedAt, finishedAt: new Date().toISOString(), options,
      metadata: {
        userAgent: navigator.userAgent, platform: navigator.platform, hardwareConcurrency: navigator.hardwareConcurrency, crossOriginIsolated,
        adapter: probe.adapterInfo, adapterFeatures: probe.adapterFeatures, deviceFeatures: [...probe.device.features],
        timestampQueries: probe.timestampEnabled ? 'enabled; native render-pass beginning/end timestamps, nanoseconds converted to milliseconds' : 'unavailable or disabled; queue completion is not GPU execution time',
        canvas: { width: document.querySelector('canvas')!.width, height: document.querySelector('canvas')!.height, devicePixelRatio },
        initCpuAndDeviceMs,
      },
      methodology: [
        'Production WebGPURenderer rendered in an actual browser, with native method counters and one wrapper around render().',
        'Frame samples are serialized through requestAnimationFrame and queue completion; these are latency samples, not maximum throughput or end-to-end application FPS.',
        'Queue completion includes browser scheduling, driver, copies, and GPU work. Only renderPassGpuMs is device execution time.',
        'Timestamp resolve/copy/readback and instrument wrappers add overhead. Query resources are excluded from renderer allocation counters.',
        'Chromium may quantize timestamp-query results (observed increments around 65.536 microseconds); a zero duration can be below timestamp resolution, not zero GPU work.',
        'Meshes/BVH and equal-clone arrays are prepared before timed publication. Edge fixtures contain all tessellation edges, intentionally stressing edges mode.',
        'Cold publication has one sample per size; steady-state frames have warmup. Idle starts after pending frames and background edge uploads settle.',
        'Counters measure GPU buffer API allocations and writes, not JavaScript heap allocations or physical GPU memory residency.',
        'Shared-asset publication uses 128 entities sharing one dense asset, with one warmup publication. Retained arrays reuse the same scene. Each fresh-equal sample has newly allocated equal arrays and fresh transforms; each changed-geometry sample has a distinct payload, asset identity and transforms. All fixtures are prepared before timing.',
      ],
      scenarios, lifecycle, errors: probe.errors,
    }
  } finally {
    // Keep the last real frame alive for the runner screenshot; cleanup on failure remains immediate.
    if (!succeeded) dispose()
  }
}

Object.assign(window, { gpuBenchmark: {
  run, dispose: () => disposeCurrent?.(), prepareHeapWorkload, runHeapWorkload,
  disposeHeapWorkload: () => { heapRenderer?.destroy(); heapRenderer = null },
} })
document.querySelector('#status')!.textContent = 'Benchmark ready'
