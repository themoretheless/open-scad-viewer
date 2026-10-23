/**
 * Browser WebGPU execution of the kernel's SDF grid sampler. The flat field
 * encoding and the shader text come from the kernel's sdf_prepare response, so
 * the browser runs exactly the qualified shader. Extraction (snap-to-zero,
 * boundary validation, marching tetrahedra) stays in the kernel on sdf_finish.
 * Dispatch plumbing lives in webgpuCompute.ts.
 */
import { runGpuCompute } from './webgpuCompute'
import type { WgslVariant } from './webgpuFeatures'

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
  wgslVariants?: WgslVariant[]
}

/** One grid job inside a batch payload; `id` pairs it with the kernel's
 * pending registration for the later `sdf_finish`. */
export interface SdfGpuJobPayload {
  id: number
  kinds: number[]
  params: number[]
  aux: number[]
  triangles: number[]
  min: number[]
  max: number[]
  cells: number[]
}

/** Many grids against the shared SDF shader, sampled in one WebGPU session.
 * Slots mirror the `sdf_prepare_batch` request order; null slots stay on the
 * CPU reference path. */
export interface SdfGpuBatchPayload {
  jobs: (SdfGpuJobPayload | null)[]
  wgsl: string
  wgslVariants?: WgslVariant[]
}

const SDF_LINEAR_INDEX_SIGNATURE = 'fn main(@builtin(global_invocation_id) id: vec3<u32>)'

export function sdfWgslVariants(wgsl: string): WgslVariant[] {
  const linear = wgsl.includes(SDF_LINEAR_INDEX_SIGNATURE) && !/\bid\.(y|z)\b/.test(wgsl)
    ? `requires linear_indexing;\n${wgsl
      .replace(SDF_LINEAR_INDEX_SIGNATURE, 'fn main(@builtin(global_invocation_index) index: u32)')
      .replace(/\bid\.x\b/g, 'index')}`
    : null
  return [
    ...(linear ? [{ label: 'sdf-linear-indexing', wgsl: linear }] : []),
    { label: 'sdf-baseline', wgsl },
  ]
}

/** 64-byte params uniform shared by every SDF grid dispatch. */
function buildSdfParams(job: Pick<SdfGpuJobPayload, 'cells' | 'min' | 'max' | 'kinds'>): ArrayBuffer {
  const [nx, ny, nz] = job.cells as [number, number, number]
  const params = new ArrayBuffer(64)
  const view = new DataView(params)
  ;[nx, ny, nz, job.kinds.length].forEach((v, i) => view.setUint32(i * 4, v, true))
  job.min.forEach((v, i) => view.setFloat32(16 + i * 4, v, true))
  for (let i = 0; i < 3; i++) {
    view.setFloat32(28 + i * 4, (job.max[i]! - job.min[i]!) / job.cells[i]!, true)
  }
  return params
}

function buildSdfDispatch(payload: SdfGpuJobPayload) {
  const [nx, ny, nz] = payload.cells as [number, number, number]
  const total = (nx + 1) * (ny + 1) * (nz + 1)
  return {
    dispatch: {
      buffers: [
        { binding: 0, data: buildSdfParams(payload), uniform: true },
        { binding: 1, data: new Uint32Array(payload.kinds) },
        { binding: 2, data: new Float32Array(payload.params) },
        { binding: 3, data: new Uint32Array(payload.aux.length ? payload.aux : [0]) },
        { binding: 4, data: new Float32Array(payload.triangles.length ? payload.triangles : [0]) },
        { binding: 5, data: new Float32Array(0), output: true },
      ],
      outputBytes: total * 4,
      workgroups: [Math.ceil(total / 256), 1, 1] as [number, number, number],
    },
    total,
  }
}

/** Samples the whole grid with the kernel's SDF shader; returns f32 values. */
export async function runSdfSweep(payload: SdfGpuPayload): Promise<Float32Array> {
  const { dispatch } = buildSdfDispatch(payload)
  const [values] = await runGpuCompute({
    wgsl: payload.wgsl,
    wgslVariants: [
      ...(payload.wgslVariants ?? []),
      ...sdfWgslVariants(payload.wgsl),
    ],
    entryPoint: 'main',
    dispatches: [dispatch],
  })
  return values
}

/**
 * Samples every eligible grid in one WebGPU session: one adapter, one
 * pipeline, one submit — the shared shader text uploads and compiles once.
 * The result mirrors `payload.jobs`; null slots come back null.
 */
export async function runSdfSweepBatch(payload: SdfGpuBatchPayload): Promise<(Float32Array | null)[]> {
  const dispatches: ReturnType<typeof buildSdfDispatch>['dispatch'][] = []
  const indices: number[] = []
  payload.jobs.forEach((job, index) => {
    if (!job) return
    dispatches.push(buildSdfDispatch(job).dispatch)
    indices.push(index)
  })
  const results = payload.jobs.map(() => null as Float32Array | null)
  if (!dispatches.length) return results
  const parts = await runGpuCompute({
    wgsl: payload.wgsl,
    wgslVariants: [
      ...(payload.wgslVariants ?? []),
      ...sdfWgslVariants(payload.wgsl),
    ],
    entryPoint: 'main',
    dispatches,
  })
  parts.forEach((values, part) => {
    results[indices[part]!] = values
  })
  return results
}
