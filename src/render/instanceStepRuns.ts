/**
 * Instance-step draws for the passes beyond the opaque surface pass: the
 * shadow depth pass, edges, the sorted transparent pass and section caps.
 * Replaces the storage-buffer `MeshInstances` path when instancing is on.
 *
 * Meshes that share a vertex buffer and index range become one
 * `drawIndexed(ic, count, 0, 0, first)` over a per-pass instance record
 * buffer bound at vertex slot 1 (`INSTANCE_STEP_VERTEX_LAYOUT`). `order:
 * 'consecutive'` merges only adjacent meshes, which keeps the sorted
 * transparent order intact; `order: 'asset'` groups by geometry, for passes
 * whose result does not depend on draw order (shadow depth, caps, edges).
 */
import { INSTANCE_RECORD_BYTES, INSTANCE_RECORD_FLOATS } from './instancedScene'

/** The subset of `GMesh` an instance-step pass reads. */
export interface InstanceStepMesh {
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  edgeIB: GPUBuffer | null
  edgeIC: number
  transform: ArrayLike<number>
  color: readonly number[]
  styleAlpha: number
  styleSelected: number
  styleEdge: number
  styleHovered: number
  morph?: unknown
  material?: unknown
}

export interface InstanceStepRun {
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  first: number
  count: number
}

type RunDevice = Pick<GPUDevice, 'createBuffer' | 'queue'>
type RunPass = Pick<
  GPURenderPassEncoder,
  'setPipeline' | 'setBindGroup' | 'setVertexBuffer' | 'setIndexBuffer' | 'drawIndexed'
>

export interface InstanceStepDrawOptions {
  /** Draw edge index buffers instead of triangles. */
  edges?: boolean
  order: 'consecutive' | 'asset'
  /** Extra group bound at index 1 (the shadow map for surface pipelines). */
  group1?: GPUBindGroup | undefined
}

/** Morphing meshes keep the per-entity path; material tails are not in the record. */
export const instanceStepEligible = (mesh: InstanceStepMesh, edges = false) =>
  !mesh.morph && (edges || !mesh.material) && (edges ? !!mesh.edgeIB && mesh.edgeIC > 0 : mesh.ic > 0)

export class InstanceStepRuns {
  private buffer: GPUBuffer | null = null
  private device: RunDevice | null = null
  private data = new Float32Array(0)
  private uploaded = new Float32Array(0)
  private uploadedCount = 0
  private ordered: InstanceStepMesh[] = []
  runs: InstanceStepRun[] = []

  /** Groups `meshes` into runs; the instance order of `ordered` follows the runs. */
  plan(meshes: readonly InstanceStepMesh[], options: Pick<InstanceStepDrawOptions, 'edges' | 'order'>) {
    const edges = options.edges === true
    const runs: InstanceStepRun[] = []
    const ordered: InstanceStepMesh[] = []
    if (options.order === 'consecutive') {
      for (const mesh of meshes) {
        const ib = (edges ? mesh.edgeIB : mesh.ib)!,
          ic = edges ? mesh.edgeIC : mesh.ic
        const last = runs[runs.length - 1]
        if (last && last.vb === mesh.vb && last.ib === ib && last.ic === ic) last.count++
        else runs.push({ vb: mesh.vb, ib, ic, first: ordered.length, count: 1 })
        ordered.push(mesh)
      }
    } else {
      const byKey = new Map<GPUBuffer, Map<GPUBuffer, InstanceStepMesh[]>>()
      for (const mesh of meshes) {
        const ib = (edges ? mesh.edgeIB : mesh.ib)!
        let inner = byKey.get(mesh.vb)
        if (!inner) byKey.set(mesh.vb, (inner = new Map()))
        const list = inner.get(ib)
        if (list) list.push(mesh)
        else inner.set(ib, [mesh])
      }
      for (const [vb, inner] of byKey) {
        for (const [ib, list] of inner) {
          // Index counts within one index buffer can differ only across LOD
          // swaps that reuse the buffer; split on count to stay exact.
          for (const mesh of list) {
            const ic = edges ? mesh.edgeIC : mesh.ic
            const last = runs[runs.length - 1]
            if (last && last.vb === vb && last.ib === ib && last.ic === ic) last.count++
            else runs.push({ vb, ib, ic, first: ordered.length, count: 1 })
            ordered.push(mesh)
          }
        }
      }
    }
    this.runs = runs
    this.ordered = ordered
    return runs
  }

  /** Writes the instance records, uploading only records that changed since the last frame. */
  upload(device: RunDevice) {
    if (this.device !== device) {
      // A new device owns new buffers; keep the plan, drop the old buffer.
      this.buffer?.destroy()
      this.buffer = null
      this.uploadedCount = 0
      this.device = device
    }
    const count = this.ordered.length
    const floats = count * INSTANCE_RECORD_FLOATS
    if (!this.buffer || this.data.length < floats) {
      const capacity = Math.max(floats, this.data.length * 2, INSTANCE_RECORD_FLOATS * 16)
      this.buffer?.destroy()
      this.buffer = device.createBuffer({
        size: capacity * 4,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
      })
      this.data = new Float32Array(capacity)
      this.uploaded = new Float32Array(capacity)
      this.uploadedCount = 0
    }
    const data = this.data
    for (let slot = 0; slot < count; slot++) packRecord(data, slot * INSTANCE_RECORD_FLOATS, this.ordered[slot]!)
    if (this.uploadedCount !== count) {
      device.queue.writeBuffer(this.buffer, 0, data, 0, floats)
    } else {
      // Hover/selection touches a few records: write those slices only.
      for (let slot = 0; slot < count; slot++) {
        const offset = slot * INSTANCE_RECORD_FLOATS
        let same = true
        for (let i = 0; i < INSTANCE_RECORD_FLOATS && same; i++) same = data[offset + i] === this.uploaded[offset + i]
        if (!same)
          device.queue.writeBuffer(this.buffer, slot * INSTANCE_RECORD_BYTES, data, offset, INSTANCE_RECORD_FLOATS)
      }
    }
    this.uploaded.set(data.subarray(0, floats))
    this.uploadedCount = count
  }

  /**
   * Plans, uploads and encodes `meshes` in one call. The pipeline must be an
   * `instanceStep` variant; `sceneBG` binds at group 0.
   */
  draw(
    pass: RunPass,
    device: RunDevice,
    pipeline: GPURenderPipeline,
    sceneBG: GPUBindGroup,
    meshes: readonly InstanceStepMesh[],
    options: InstanceStepDrawOptions,
  ) {
    this.plan(meshes, options)
    if (!this.runs.length) return
    this.upload(device)
    pass.setPipeline(pipeline)
    pass.setBindGroup(0, sceneBG)
    if (options.group1) pass.setBindGroup(1, options.group1)
    pass.setVertexBuffer(1, this.buffer!)
    for (const run of this.runs) {
      pass.setVertexBuffer(0, run.vb)
      pass.setIndexBuffer(run.ib, 'uint32')
      pass.drawIndexed(run.ic, run.count, 0, 0, run.first)
    }
  }

  clear() {
    this.buffer?.destroy()
    this.buffer = null
    this.device = null
    this.data = new Float32Array(0)
    this.uploaded = new Float32Array(0)
    this.uploadedCount = 0
    this.ordered = []
    this.runs = []
  }
}

/** Record layout of `INSTANCE_STEP_VERTEX_LAYOUT`: column-major model, color, style. */
function packRecord(data: Float32Array, offset: number, mesh: InstanceStepMesh) {
  const transform = mesh.transform
  for (let row = 0; row < 4; row++)
    for (let column = 0; column < 4; column++) data[offset + column * 4 + row] = transform[row * 4 + column]!
  const color = mesh.color
  data[offset + 16] = color[0] ?? 1
  data[offset + 17] = color[1] ?? 1
  data[offset + 18] = color[2] ?? 1
  data[offset + 19] = color[3] ?? 1
  data[offset + 20] = mesh.styleAlpha
  data[offset + 21] = mesh.styleSelected
  data[offset + 22] = mesh.styleEdge
  data[offset + 23] = mesh.styleHovered
}
