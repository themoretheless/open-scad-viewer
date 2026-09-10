/**
 * Browser WebGPU execution of the kernel's SDF grid sampler. The flat field
 * encoding and the shader text come from the kernel's sdf_prepare response, so
 * the browser runs exactly the qualified shader. Extraction (snap-to-zero,
 * boundary validation, marching tetrahedra) stays in the kernel on sdf_finish.
 * Dispatch plumbing lives in webgpuCompute.ts.
 */
import { runGpuCompute } from './webgpuCompute'

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
  const [nx, ny, nz] = payload.cells as [number, number, number]
  const total = (nx + 1) * (ny + 1) * (nz + 1)
  const params = new ArrayBuffer(64)
  const view = new DataView(params)
  ;[nx, ny, nz, payload.kinds.length].forEach((v, i) => view.setUint32(i * 4, v, true))
  payload.min.forEach((v, i) => view.setFloat32(16 + i * 4, v, true))
  for (let i = 0; i < 3; i++) {
    view.setFloat32(28 + i * 4, (payload.max[i]! - payload.min[i]!) / payload.cells[i]!, true)
  }
  const [values] = await runGpuCompute({
    wgsl: payload.wgsl,
    entryPoint: 'main',
    dispatches: [{
      buffers: [
        { binding: 0, data: params, uniform: true },
        { binding: 1, data: new Uint32Array(payload.kinds) },
        { binding: 2, data: new Float32Array(payload.params) },
        { binding: 3, data: new Uint32Array(payload.aux.length ? payload.aux : [0]) },
        { binding: 4, data: new Float32Array(payload.triangles.length ? payload.triangles : [0]) },
        { binding: 5, data: new Float32Array(0), output: true },
      ],
      outputBytes: total * 4,
      workgroups: [Math.ceil(total / 256), 1, 1],
    }],
  })
  return values
}
