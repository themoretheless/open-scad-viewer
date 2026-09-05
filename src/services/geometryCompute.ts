import type { MeshData } from '../core/mesh'

export type SurfaceInput = Pick<MeshData, 'vertices' | 'indices' | 'transform'>
export interface GeometryComputeResult {
  operation: 'world-surface-area'
  backend: 'webgpu-compute' | 'cpu'
  area: number
  cpuMs: number
  gpuMs: number | null
  relativeError: number | null
  fallback: 'unavailable' | 'gpu-failed' | 'verification-failed' | null
}
export interface GeometryComputeBackend {
  readonly id: 'cpu' | 'webgpu-compute'
  surfaceArea(input: SurfaceInput, signal: AbortSignal): Promise<number>
}

function checkAbort(signal: AbortSignal) { signal.throwIfAborted() }

/** Affine world-space area; translation cancels before arithmetic, on both backends. */
export async function cpuSurfaceArea(input: SurfaceInput, signal: AbortSignal): Promise<number> {
  const { vertices: v, indices: indices, transform: m } = input
  if (!v.length || v.length % 6 || !indices.length || indices.length % 3
    || indices.length > 3_000_000 || v.byteLength > 64 * 1024 * 1024
    || m.length !== 16 || !Array.from(m).every(Number.isFinite)
    || m[12] !== 0 || m[13] !== 0 || m[14] !== 0 || m[15] !== 1) throw new Error('invalid-compute-input')
  let sum = 0, correction = 0
  for (let i = 0; i < indices.length; i += 3) {
    if (i % 6144 === 0) {
      checkAbort(signal)
      if (i) await new Promise<void>(resolve => setTimeout(resolve, 0))
      checkAbort(signal)
    }
    const a = indices[i] * 6, b = indices[i + 1] * 6, c = indices[i + 2] * 6
    if (a >= v.length || b >= v.length || c >= v.length) throw new Error('invalid-compute-input')
    const ux = v[b] - v[a], uy = v[b + 1] - v[a + 1], uz = v[b + 2] - v[a + 2]
    const vx = v[c] - v[a], vy = v[c + 1] - v[a + 1], vz = v[c + 2] - v[a + 2]
    const x = m[0] * ux + m[1] * uy + m[2] * uz
    const y = m[4] * ux + m[5] * uy + m[6] * uz
    const z = m[8] * ux + m[9] * uy + m[10] * uz
    const X = m[0] * vx + m[1] * vy + m[2] * vz
    const Y = m[4] * vx + m[5] * vy + m[6] * vz
    const Z = m[8] * vx + m[9] * vy + m[10] * vz
    const area = Math.hypot(y * Z - z * Y, z * X - x * Z, x * Y - y * X) / 2
    if (!Number.isFinite(area)) throw new Error('invalid-compute-input')
    const corrected = area - correction
    const next = sum + corrected
    correction = (next - sum) - corrected
    sum = next
  }
  checkAbort(signal)
  if (!Number.isFinite(sum)) throw new Error('invalid-compute-input')
  return sum
}

export const SURFACE_AREA_WGSL = /* wgsl */`
struct Params { r0: vec4f, r1: vec4f, r2: vec4f, info: vec4u }
@group(0) @binding(0) var<storage, read> vertices: array<f32>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<storage, read_write> partials: array<f32>;
var<workgroup> areas: array<f32, 128>;
fn point(index: u32) -> vec3f {
  let offset = index * 6u;
  return vec3f(vertices[offset], vertices[offset + 1u], vertices[offset + 2u]);
}
fn direction(v: vec3f) -> vec3f {
  return vec3f(dot(params.r0.xyz, v), dot(params.r1.xyz, v), dot(params.r2.xyz, v));
}
@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) global: vec3u,
        @builtin(local_invocation_id) local: vec3u,
        @builtin(workgroup_id) group: vec3u) {
  var area = 0.0;
  if (global.x < params.info.x) {
    let offset = global.x * 3u;
    let a = point(indices[offset]);
    let u = direction(point(indices[offset + 1u]) - a);
    let v = direction(point(indices[offset + 2u]) - a);
    area = length(cross(u, v)) * 0.5;
  }
  areas[local.x] = area;
  workgroupBarrier();
  for (var stride = 64u; stride > 0u; stride = stride / 2u) {
    if (local.x < stride) { areas[local.x] += areas[local.x + stride]; }
    workgroupBarrier();
  }
  if (local.x == 0u) { partials[group.x] = areas[0]; }
}
`

/** One private compute device per pilot job; never destroys the viewport device. */
export async function webgpuSurfaceArea(input: SurfaceInput, signal: AbortSignal): Promise<number> {
  checkAbort(signal)
  if (typeof navigator === 'undefined' || !navigator.gpu) throw new Error('unavailable')
  let device: GPUDevice | null = null
  const buffers: GPUBuffer[] = []
  let stopped = false
  const cleanup = () => {
    stopped = true
    for (const buffer of buffers) buffer.destroy()
    device?.destroy()
  }
  const check = () => { checkAbort(signal); if (stopped) throw new Error('gpu-failed') }
  let timer: ReturnType<typeof setTimeout> | undefined
  let abort: () => void = () => {}
  const interrupted = new Promise<never>((_, reject) => {
    abort = () => { cleanup(); reject(signal.reason ?? new Error('cancelled')) }
    signal.addEventListener('abort', abort, { once: true })
    timer = setTimeout(() => { cleanup(); reject(new Error('gpu-failed')) }, 5000)
  })
  const work = async () => {
    try {
      const adapter = await navigator.gpu.requestAdapter()
      check()
      if (!adapter) throw new Error('unavailable')
      const created = await adapter.requestDevice()
      if (stopped || signal.aborted) { created.destroy(); check() }
      device = created
      check()
      const groups = Math.ceil(input.indices.length / 3 / 128)
      if (groups > device.limits.maxComputeWorkgroupsPerDimension
        || Math.max(input.vertices.byteLength, input.indices.byteLength) > Math.min(device.limits.maxStorageBufferBindingSize, device.limits.maxBufferSize)) throw new Error('gpu-failed')
      device.pushErrorScope('validation')
      device.pushErrorScope('out-of-memory')
      const make = (size: number, usage: number) => {
        const buffer = device!.createBuffer({ size, usage })
        buffers.push(buffer)
        return buffer
      }
      const vertexBuffer = make(input.vertices.byteLength, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST)
      const indexBuffer = make(input.indices.byteLength, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST)
      const parameters = make(64, GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST)
      const output = make(groups * 4, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC)
      const readback = make(groups * 4, GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST)
      device.queue.writeBuffer(vertexBuffer, 0, input.vertices)
      device.queue.writeBuffer(indexBuffer, 0, input.indices)
      const uniforms = new ArrayBuffer(64)
      new Float32Array(uniforms).set(input.transform.subarray(0, 12))
      new Uint32Array(uniforms)[12] = input.indices.length / 3
      device.queue.writeBuffer(parameters, 0, uniforms)
      const pipeline = await device.createComputePipelineAsync({ layout: 'auto', compute: { module: device.createShaderModule({ code: SURFACE_AREA_WGSL }), entryPoint: 'main' } })
      check()
      const bindGroup = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [vertexBuffer, indexBuffer, parameters, output].map((buffer, binding) => ({ binding, resource: { buffer } })) })
      const encoder = device.createCommandEncoder()
      const pass = encoder.beginComputePass()
      pass.setPipeline(pipeline)
      pass.setBindGroup(0, bindGroup)
      pass.dispatchWorkgroups(groups)
      pass.end()
      encoder.copyBufferToBuffer(output, 0, readback, 0, groups * 4)
      device.queue.submit([encoder.finish()])
      await readback.mapAsync(GPUMapMode.READ)
      check()
      const values = new Float32Array(readback.getMappedRange())
      let sum = 0
      for (const value of values) {
        if (!Number.isFinite(value) || value < 0) throw new Error('gpu-failed')
        sum += value
      }
      readback.unmap()
      const memoryError = await device.popErrorScope()
      const validationError = await device.popErrorScope()
      check()
      if (memoryError || validationError || !Number.isFinite(sum)) throw new Error('gpu-failed')
      return sum
    } finally { cleanup() }
  }
  try { return await Promise.race([work(), interrupted]) }
  finally { clearTimeout(timer); signal.removeEventListener('abort', abort); cleanup() }
}

export const CPU_COMPUTE_BACKEND: GeometryComputeBackend = { id: 'cpu', surfaceArea: cpuSurfaceArea }
export const WEBGPU_COMPUTE_BACKEND: GeometryComputeBackend = { id: 'webgpu-compute', surfaceArea: webgpuSurfaceArea }

/** Qualification pilot: GPU result is published only after a full CPU comparison. */
export async function analyzeSurfaceArea(input: SurfaceInput, signal: AbortSignal, backend: GeometryComputeBackend = WEBGPU_COMPUTE_BACKEND): Promise<GeometryComputeResult> {
  const cpuStart = performance.now()
  const reference = await cpuSurfaceArea(input, signal)
  const cpuMs = performance.now() - cpuStart
  const start = performance.now()
  try {
    const area = await backend.surfaceArea(input, signal)
    checkAbort(signal)
    const relativeError = Math.abs(area - reference) / Math.max(Math.abs(reference), 1e-12)
    const verified = Math.abs(area - reference) <= 1e-8 + 5e-4 * Math.abs(reference)
    return { operation: 'world-surface-area', backend: verified ? backend.id : 'cpu', area: verified ? area : reference, cpuMs, gpuMs: performance.now() - start, relativeError, fallback: verified ? null : 'verification-failed' }
  } catch (error) {
    checkAbort(signal)
    return { operation: 'world-surface-area', backend: 'cpu', area: reference, cpuMs, gpuMs: null, relativeError: null, fallback: error instanceof Error && error.message === 'unavailable' ? 'unavailable' : 'gpu-failed' }
  }
}
