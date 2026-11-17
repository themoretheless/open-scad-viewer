/**
 * Opaque-surface instancing for the WebGPU renderer: stages the eligible
 * opaque draws into `InstancedSceneBuffers` (adopting each mesh's resident
 * vertex/index buffers) and encodes one `drawIndexed(ic, instanceCount)` per
 * shared asset with the instance-step pipeline. Morph, LOD, material and
 * non-Phong entities stay on the per-entity path.
 *
 * The staged plan also feeds the GPU picker, whose instanced draws write
 * `pickBase + instance_index`.
 */
import type { GpuPickerInstances } from './gpuPicker'
import { InstancedSceneBuffers, resolveInstancePick, type InstanceSource } from './instancedScene'

/** The subset of `GMesh` the instanced pass reads. */
export interface InstanceableMesh {
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  vertices: Float32Array
  indices: Uint32Array
  edgeIndices: Uint32Array
  transform: Float32Array | ArrayLike<number>
  color: readonly number[]
  styleAlpha: number
  styleSelected: number
  styleEdge: number
  styleHovered: number
  morph?: unknown
  lod?: unknown
  material?: unknown
  shadingModel: string
}

/** Instanced entities never morph, switch LOD levels, or carry a material tail. */
export const isInstanceable = (mesh: InstanceableMesh) =>
  !mesh.morph && !mesh.lod && !mesh.material && mesh.shadingModel === 'phong' && mesh.ic > 0

type PassDevice = Pick<GPUDevice, 'createBuffer' | 'queue'>
type Pass = Pick<
  GPURenderPassEncoder,
  'setPipeline' | 'setBindGroup' | 'setVertexBuffer' | 'setIndexBuffer' | 'drawIndexed'
>

export class InstancedOpaquePass {
  private scene: InstancedSceneBuffers | null = null
  private device: PassDevice | null = null
  private members: InstanceableMesh[] = []
  private sources: InstanceSource[] = []
  /** Renderer mesh index of each staged source, for picking. */
  private sourceIndex: number[] = []
  private covered = new Set<number>()
  readonly perEntity: InstanceableMesh[] = []

  /**
   * Splits `draws` into instanced batches and the per-entity remainder
   * (`perEntity`). Returns false, staging nothing, when no asset is shared by
   * two or more eligible draws; the caller then keeps its per-entity path.
   */
  prepare<M extends InstanceableMesh>(device: PassDevice, draws: readonly M[], indexOf: (mesh: M) => number): boolean {
    if (this.device !== device) {
      this.clear()
      this.device = device
      this.scene = new InstancedSceneBuffers(device)
    }
    const eligible: M[] = []
    this.perEntity.length = 0
    for (const mesh of draws) (isInstanceable(mesh) ? eligible : this.perEntity).push(mesh)
    const distinct = new Set(eligible.map(mesh => mesh.vb)).size
    if (eligible.length < 2 || distinct === eligible.length) {
      this.reset()
      this.perEntity.length = 0
      return false
    }
    const same =
      eligible.length === this.members.length &&
      eligible.every(
        (mesh, index) =>
          mesh === this.members[index] &&
          mesh.transform === this.sources[index]!.transform &&
          mesh.vb === this.sources[index]!.gpu!.vb,
      )
    if (same) {
      this.scene!.updateStyles(this.members)
      return true
    }
    this.members = eligible.slice()
    this.sources = eligible.map(mesh => ({
      vertices: mesh.vertices,
      indices: mesh.indices,
      edgeIndices: mesh.edgeIndices,
      transform: mesh.transform as InstanceSource['transform'],
      gpu: { vb: mesh.vb, ib: mesh.ib, ic: mesh.ic },
      color: mesh.color,
      styleAlpha: mesh.styleAlpha,
      styleSelected: mesh.styleSelected,
      styleEdge: mesh.styleEdge,
      styleHovered: mesh.styleHovered,
    }))
    this.sourceIndex = eligible.map(indexOf)
    this.covered = new Set(this.sourceIndex)
    this.scene!.stage(this.sources)
    return true
  }

  /** One instanced draw per batch: slot 0 = asset vertices, slot 1 = instance records, shadow at group 1. */
  encode(pass: Pass, pipeline: GPURenderPipeline, sceneBG: GPUBindGroup, shadowBG?: GPUBindGroup) {
    if (!this.scene || !this.members.length) return
    pass.setPipeline(pipeline)
    pass.setBindGroup(0, sceneBG)
    if (shadowBG) pass.setBindGroup(1, shadowBG)
    this.scene.encode(pass)
  }

  /** Instanced members for the GPU picker, or undefined when nothing is staged. */
  pickInstances(): GpuPickerInstances | undefined {
    const scene = this.scene
    if (!scene || !this.members.length) return undefined
    const plan = scene.plan,
      sourceIndex = this.sourceIndex,
      covered = this.covered
    return {
      batches: scene.drawBatches(),
      covers: meshIndex => covered.has(meshIndex),
      resolve: pickId => {
        const source = resolveInstancePick(plan, pickId)
        return source === null ? null : (sourceIndex[source] ?? null)
      },
    }
  }

  get metrics() {
    return this.scene?.metrics
  }

  /** Forget the staged set (buffers adopted from meshes are not destroyed). */
  reset() {
    this.scene?.stage([])
    this.members = []
    this.sources = []
    this.sourceIndex = []
    this.covered = new Set()
  }

  clear() {
    this.scene?.destroy()
    this.scene = null
    this.device = null
    this.members = []
    this.sources = []
    this.sourceIndex = []
    this.covered = new Set()
    this.perEntity.length = 0
  }
}
