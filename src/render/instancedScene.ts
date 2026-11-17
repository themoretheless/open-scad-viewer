/**
 * Instanced scene staging: one vertex/index buffer per geometry asset plus a
 * per-batch instance-transform buffer, drawn with `instanceCount`. Linked
 * instances (for example solid instance arrays) and repeated meshes share the
 * asset upload instead of materializing duplicate geometry per entity.
 *
 * Inspection artifacts (edge index buffers) are deferred: they are uploaded
 * once per asset, only when a caller asks for them.
 *
 * Picking: an instanced draw writes `batch.pickBase + instance_index` as the
 * pick id, and `resolveInstancePick` maps it back to the individual entity.
 */
import type { MeshData } from '../core/mesh'

/** Resident geometry buffers owned by someone else (the renderer); adopted, never uploaded or destroyed. */
export interface AdoptedGeometry {
  readonly vb: GPUBuffer
  readonly ib: GPUBuffer
  readonly ic: number
}

/** Per-instance style: RGBA color and (alpha, selected, edge, hovered). */
export interface InstanceStyle {
  readonly color?: readonly number[] | undefined
  readonly styleAlpha?: number | undefined
  readonly styleSelected?: number | undefined
  readonly styleEdge?: number | undefined
  readonly styleHovered?: number | undefined
}

export type InstanceSource = Pick<MeshData, 'vertices' | 'indices' | 'edgeIndices' | 'transform'> &
  InstanceStyle & {
    readonly geometryAssetId?: string | undefined
    /** Adopted resident buffers; the batch key is then the vertex-buffer identity. */
    readonly gpu?: AdoptedGeometry | undefined
  }

/**
 * Instance-step record: column-major model (16 floats), color (4), style (4).
 * Bound at vertex slot 1 with `stepMode: 'instance'`.
 */
export const INSTANCE_RECORD_FLOATS = 24
export const INSTANCE_RECORD_BYTES = INSTANCE_RECORD_FLOATS * 4
const STYLE_OFFSET = 16

/** Vertex slot 1 layout of the instance-step shader variants (locations 3..8). */
export const INSTANCE_STEP_VERTEX_LAYOUT: GPUVertexBufferLayout = {
  arrayStride: INSTANCE_RECORD_BYTES,
  stepMode: 'instance',
  attributes: [
    { shaderLocation: 3, offset: 0, format: 'float32x4' },
    { shaderLocation: 4, offset: 16, format: 'float32x4' },
    { shaderLocation: 5, offset: 32, format: 'float32x4' },
    { shaderLocation: 6, offset: 48, format: 'float32x4' },
    { shaderLocation: 7, offset: 64, format: 'float32x4' },
    { shaderLocation: 8, offset: 80, format: 'float32x4' },
  ],
}

export interface InstanceBatch {
  /** Asset id, or an identity key for meshes without one. */
  readonly key: string
  /** Mesh indices drawn by this batch, in draw order; slot = position. */
  readonly members: readonly number[]
  /** First pick id of the batch; instance `slot` picks as `pickBase + slot`. */
  readonly pickBase: number
}

export interface InstancePlan {
  readonly batches: readonly InstanceBatch[]
  /** Mesh index to [batch index, instance slot]. */
  readonly slotOf: ReadonlyArray<readonly [number, number]>
}

const identityKeys = new WeakMap<object, string>()
let nextIdentity = 0
function identityKey(object: object, prefix: string) {
  let key = identityKeys.get(object)
  if (!key) identityKeys.set(object, (key = `${prefix}:${nextIdentity++}`))
  return key
}

/**
 * Asset key: the adopted vertex-buffer identity, else the content-addressed
 * asset id, else the vertex array identity.
 */
export const instanceAssetKey = (mesh: InstanceSource) =>
  mesh.gpu ? identityKey(mesh.gpu.vb, 'gpu') : (mesh.geometryAssetId ?? identityKey(mesh.vertices, 'identity'))

/** Groups meshes by asset in first-occurrence order. Pick id 0 stays "no hit". */
export function planInstanceBatches(meshes: readonly InstanceSource[]): InstancePlan {
  const byKey = new Map<string, number[]>()
  const order: string[] = []
  meshes.forEach((mesh, index) => {
    const key = instanceAssetKey(mesh)
    let members = byKey.get(key)
    if (!members) {
      byKey.set(key, (members = []))
      order.push(key)
    }
    members.push(index)
  })
  const slotOf: Array<readonly [number, number]> = new Array(meshes.length)
  let pickBase = 1
  const batches = order.map((key, batchIndex) => {
    const members = byKey.get(key)!
    members.forEach((mesh, slot) => (slotOf[mesh] = [batchIndex, slot]))
    const batch = { key, members, pickBase }
    pickBase += members.length
    return batch
  })
  return { batches, slotOf }
}

/** Mesh index for an instanced pick id, or null for background / stale ids. */
export function resolveInstancePick(plan: InstancePlan, pickId: number): number | null {
  if (!Number.isInteger(pickId) || pickId < 1) return null
  let lo = 0,
    hi = plan.batches.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const batch = plan.batches[mid]!
    if (pickId < batch.pickBase) hi = mid - 1
    else if (pickId >= batch.pickBase + batch.members.length) lo = mid + 1
    else return batch.members[pickId - batch.pickBase]!
  }
  return null
}

/** Pick id the instanced draw writes for a mesh index. */
export function instancePickId(plan: InstancePlan, meshIndex: number): number | null {
  const slot = plan.slotOf[meshIndex]
  return slot ? plan.batches[slot[0]]!.pickBase + slot[1] : null
}

function writeStyle(data: Float32Array, offset: number, mesh: InstanceStyle) {
  const color = mesh.color
  data[offset] = color?.[0] ?? 1
  data[offset + 1] = color?.[1] ?? 1
  data[offset + 2] = color?.[2] ?? 1
  data[offset + 3] = color?.[3] ?? 1
  data[offset + 4] = mesh.styleAlpha ?? 1
  data[offset + 5] = mesh.styleSelected ?? 0
  data[offset + 6] = mesh.styleEdge ?? 0
  data[offset + 7] = mesh.styleHovered ?? 0
}

/**
 * Instance records of a batch for the instance-step vertex buffer
 * (`INSTANCE_RECORD_FLOATS` each). `transform` is the row-major `Mat4`; the
 * record stores it column-major as WGSL `mat4x4f` expects.
 */
export function packInstanceRecords(batch: InstanceBatch, meshes: readonly InstanceSource[], out?: Float32Array) {
  const floats = batch.members.length * INSTANCE_RECORD_FLOATS
  const data = out && out.length >= floats ? out : new Float32Array(floats)
  batch.members.forEach((index, slot) => {
    const mesh = meshes[index]!,
      transform = mesh.transform,
      offset = slot * INSTANCE_RECORD_FLOATS
    for (let row = 0; row < 4; row++)
      for (let column = 0; column < 4; column++)
        data[offset + column * 4 + row] = transform ? transform[row * 4 + column]! : row === column ? 1 : 0
    writeStyle(data, offset + STYLE_OFFSET, mesh)
  })
  return data
}

export interface InstancingMetrics {
  /** Distinct assets whose vertex/index buffers were uploaded by the last stage. */
  geometryUploads: number
  geometryUploadBytes: number
  /** Instance draws encoded per pass (one per batch). */
  drawCalls: number
  instances: number
  /** Deferred edge buffers uploaded so far (once per asset, on demand). */
  edgeUploads: number
}

interface AssetBuffers {
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  /** Adopted from the caller: never destroyed here. */
  adopted: boolean
  edgeIB: GPUBuffer | null
  edgeIC: number
  edges: Uint32Array
}

interface BatchBuffers {
  instances: GPUBuffer
  capacity: number
  count: number
  asset: AssetBuffers
  /** CPU mirror of the instance records, diffed by `updateStyles`. */
  records: Float32Array
}

/** What one instanced draw binds; used by the GPU picker. */
export interface InstanceDrawBatch {
  readonly vb: GPUBuffer
  readonly ib: GPUBuffer
  readonly ic: number
  readonly instances: GPUBuffer
  readonly instanceCount: number
  readonly pickBase: number
}

type InstancingDevice = Pick<GPUDevice, 'createBuffer' | 'queue'>

const VERTEX = 0x20,
  INDEX = 0x10,
  COPY_DST = 0x8
const usage = (name: 'VERTEX' | 'INDEX' | 'COPY_DST', fallback: number) =>
  typeof GPUBufferUsage === 'undefined' ? fallback : GPUBufferUsage[name]

/**
 * Owns per-asset geometry and per-batch instance buffers across publications.
 * An asset already resident is not uploaded again; assets that leave the
 * scene are destroyed on the next stage.
 */
export class InstancedSceneBuffers {
  private assets = new Map<string, AssetBuffers>()
  private batches: BatchBuffers[] = []
  plan: InstancePlan = { batches: [], slotOf: [] }
  readonly metrics: InstancingMetrics = {
    geometryUploads: 0,
    geometryUploadBytes: 0,
    drawCalls: 0,
    instances: 0,
    edgeUploads: 0,
  }

  constructor(private readonly device: InstancingDevice) {}

  stage(meshes: readonly InstanceSource[]): InstancePlan {
    const plan = planInstanceBatches(meshes)
    const live = new Map<string, AssetBuffers>()
    let uploads = 0,
      bytes = 0
    const previousBatches = this.batches
    this.batches = plan.batches.map((batch, index) => {
      let asset = this.assets.get(batch.key) ?? live.get(batch.key)
      const first = meshes[batch.members[0]!]!
      if (!asset && first.gpu) {
        asset = { ...first.gpu, adopted: true, edgeIB: null, edgeIC: 0, edges: first.edgeIndices ?? new Uint32Array(0) }
      } else if (!asset) {
        const mesh = first
        const vb = this.device.createBuffer({
          size: mesh.vertices.byteLength,
          usage: usage('VERTEX', VERTEX) | usage('COPY_DST', COPY_DST),
        })
        const ib = this.device.createBuffer({
          size: mesh.indices.byteLength,
          usage: usage('INDEX', INDEX) | usage('COPY_DST', COPY_DST),
        })
        this.device.queue.writeBuffer(vb, 0, mesh.vertices)
        this.device.queue.writeBuffer(ib, 0, mesh.indices)
        asset = {
          vb,
          ib,
          ic: mesh.indices.length,
          adopted: false,
          edgeIB: null,
          edgeIC: 0,
          edges: mesh.edgeIndices ?? new Uint32Array(0),
        }
        uploads++
        bytes += mesh.vertices.byteLength + mesh.indices.byteLength
      }
      live.set(batch.key, asset)
      const reused = previousBatches[index]
      const needed = batch.members.length * INSTANCE_RECORD_BYTES
      let instances = reused?.instances ?? null,
        capacity = reused?.capacity ?? 0
      if (!instances || capacity < needed) {
        instances?.destroy()
        instances = this.device.createBuffer({
          size: needed,
          usage: usage('VERTEX', VERTEX) | usage('COPY_DST', COPY_DST),
        })
        capacity = needed
      }
      const records = packInstanceRecords(
        batch,
        meshes,
        reused && reused.records.length >= batch.members.length * INSTANCE_RECORD_FLOATS ? reused.records : undefined,
      )
      this.device.queue.writeBuffer(instances, 0, records, 0, batch.members.length * INSTANCE_RECORD_FLOATS)
      return { instances, capacity, count: batch.members.length, asset, records }
    })
    for (let i = this.batches.length; i < previousBatches.length; i++) previousBatches[i]!.instances.destroy()
    for (const [key, asset] of this.assets) if (!live.has(key)) destroyAsset(asset)
    this.assets = live
    this.plan = plan
    Object.assign(this.metrics, {
      geometryUploads: uploads,
      geometryUploadBytes: bytes,
      drawCalls: plan.batches.length,
      instances: meshes.length,
    })
    return plan
  }

  /**
   * Rewrites only the color/style slice of instances whose style changed
   * (selection, hover, alpha) without restaging. `meshes` must be parallel to
   * the list of the last `stage` call. Returns the number of 32-byte slices written.
   */
  updateStyles(meshes: readonly InstanceStyle[]): number {
    let writes = 0
    const style = new Float32Array(8)
    this.plan.batches.forEach((batch, batchIndex) => {
      const buffers = this.batches[batchIndex]!
      batch.members.forEach((index, slot) => {
        writeStyle(style, 0, meshes[index]!)
        const offset = slot * INSTANCE_RECORD_FLOATS + STYLE_OFFSET
        let same = true
        for (let i = 0; i < 8 && same; i++) same = buffers.records[offset + i] === style[i]
        if (same) return
        buffers.records.set(style, offset)
        this.device.queue.writeBuffer(buffers.instances, offset * 4, buffers.records, offset, 8)
        writes++
      })
    })
    return writes
  }

  /** Draw batches with their pick bases, in plan order. */
  drawBatches(): InstanceDrawBatch[] {
    return this.batches.map((batch, index) => ({
      vb: batch.asset.vb,
      ib: batch.asset.ib,
      ic: batch.asset.ic,
      instances: batch.instances,
      instanceCount: batch.count,
      pickBase: this.plan.batches[index]!.pickBase,
    }))
  }

  /** Upload the edge index buffer of one batch's asset on first use. */
  ensureEdges(batchIndex: number): GPUBuffer | null {
    const asset = this.batches[batchIndex]?.asset
    if (!asset || !asset.edges.length) return null
    if (!asset.edgeIB) {
      asset.edgeIB = this.device.createBuffer({
        size: asset.edges.byteLength,
        usage: usage('INDEX', INDEX) | usage('COPY_DST', COPY_DST),
      })
      this.device.queue.writeBuffer(asset.edgeIB, 0, asset.edges)
      asset.edgeIC = asset.edges.length
      this.metrics.edgeUploads++
    }
    return asset.edgeIB
  }

  /** One instanced draw per batch: slot 0 = asset vertices, slot 1 = instance records. */
  encode(pass: Pick<GPURenderPassEncoder, 'setVertexBuffer' | 'setIndexBuffer' | 'drawIndexed'>, edges = false) {
    this.batches.forEach((batch, index) => {
      const ib = edges ? this.ensureEdges(index) : batch.asset.ib
      if (!ib) return
      pass.setVertexBuffer(0, batch.asset.vb)
      pass.setVertexBuffer(1, batch.instances)
      pass.setIndexBuffer(ib, 'uint32')
      pass.drawIndexed(edges ? batch.asset.edgeIC : batch.asset.ic, batch.count)
    })
  }

  destroy() {
    for (const asset of this.assets.values()) destroyAsset(asset)
    for (const batch of this.batches) batch.instances.destroy()
    this.assets.clear()
    this.batches = []
    this.plan = { batches: [], slotOf: [] }
  }
}

function destroyAsset(asset: AssetBuffers) {
  if (!asset.adopted) {
    asset.vb.destroy()
    asset.ib.destroy()
  }
  asset.edgeIB?.destroy()
}
