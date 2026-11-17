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

export type InstanceSource = Pick<MeshData, 'vertices' | 'indices' | 'edgeIndices' | 'transform'> & {
  readonly geometryAssetId?: string | undefined
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
function identityKey(mesh: InstanceSource) {
  let key = identityKeys.get(mesh.vertices)
  if (!key) identityKeys.set(mesh.vertices, (key = `identity:${nextIdentity++}`))
  return key
}

/** Asset key: the content-addressed asset id, else the vertex array identity. */
export const instanceAssetKey = (mesh: InstanceSource) => mesh.geometryAssetId ?? identityKey(mesh)

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

/** Column transforms of a batch packed for an instance-step vertex buffer (16 floats each). */
export function packInstanceTransforms(batch: InstanceBatch, meshes: readonly InstanceSource[], out?: Float32Array) {
  const data = out && out.length >= batch.members.length * 16 ? out : new Float32Array(batch.members.length * 16)
  batch.members.forEach((mesh, slot) => {
    const transform = meshes[mesh]!.transform
    if (transform) data.set(transform.subarray(0, 16), slot * 16)
    else data.set(IDENTITY, slot * 16)
  })
  return data
}

const IDENTITY = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])

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
  edgeIB: GPUBuffer | null
  edgeIC: number
  edges: Uint32Array
}

interface BatchBuffers {
  instances: GPUBuffer
  capacity: number
  count: number
  asset: AssetBuffers
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
  private scratch = new Float32Array(0)
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
      if (!asset) {
        const mesh = meshes[batch.members[0]!]!
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
          edgeIB: null,
          edgeIC: 0,
          edges: mesh.edgeIndices ?? new Uint32Array(0),
        }
        uploads++
        bytes += mesh.vertices.byteLength + mesh.indices.byteLength
      }
      live.set(batch.key, asset)
      const reused = previousBatches[index]
      const needed = batch.members.length * 64
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
      if (this.scratch.length < batch.members.length * 16) this.scratch = new Float32Array(batch.members.length * 16)
      packInstanceTransforms(batch, meshes, this.scratch)
      this.device.queue.writeBuffer(instances, 0, this.scratch, 0, batch.members.length * 16)
      return { instances, capacity, count: batch.members.length, asset }
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

  /** One instanced draw per batch: slot 0 = asset vertices, slot 1 = instance transforms. */
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
  asset.vb.destroy()
  asset.ib.destroy()
  asset.edgeIB?.destroy()
}
