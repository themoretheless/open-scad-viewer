/**
 * Instance-step staging of the recording renderer. With `?instancing=1` the
 * WebGPU frame encoder draws the shadow, section cap, sorted transparent and
 * edge passes as `InstanceStepRuns`; the recorder plans and uploads the same
 * runs against a buffer-less device so smoke metrics cover those passes.
 *
 * Pass membership follows `encodeAndSubmitFrame` without frustum culling (the
 * recorder has no view): shadow casters are visible opaque meshes, the cap
 * draws the opaque set only when every member is eligible, the transparent
 * pass runs only when every transparent mesh is eligible and Phong, and edges
 * are drawn in `edges` mode or for the selected mesh. Instances of one
 * geometry asset share one stand-in vertex/index buffer, as resident
 * `GpuGeometryCache` buffers do.
 */
import type { MeshData } from '../core/mesh'
import { effectiveDisplayAlpha, isTransparentAlpha } from './backendQuality'
import { instanceAssetKey } from './instancedScene'
import { InstanceStepRuns, instanceStepEligible, type InstanceStepMesh } from './instanceStepRuns'
import type { DisplayMode, ShadingModel } from './rendererContracts'

export type InstanceStepPass = 'shadow' | 'sectionCap' | 'transparent' | 'edges'

export interface InstanceStepPassMetrics {
  /** `drawIndexed` calls the pass encoded (one per run). */
  readonly drawCalls: number
  readonly instances: number
  /** Instance record writes of the last staging: a full write, or one per changed record. */
  readonly recordWrites: number
  /** The pass fell back to the per-entity path (an ineligible member). */
  readonly perEntity: boolean
}

export type InstanceStepMetrics = Readonly<Record<InstanceStepPass, InstanceStepPassMetrics>>

export interface InstanceStepFrame {
  readonly meshes: readonly MeshData[]
  readonly visible: (index: number) => boolean
  readonly selected: number | null
  readonly displayMode: DisplayMode
  readonly shadingModel: ShadingModel
  readonly shadows: boolean
  readonly section: boolean
  readonly transparency: 'sorted' | 'wboit'
}

const PASSES: readonly InstanceStepPass[] = ['shadow', 'sectionCap', 'transparent', 'edges']
const IDLE: InstanceStepPassMetrics = { drawCalls: 0, instances: 0, recordWrites: 0, perEntity: false }

interface AssetStandIns {
  readonly vb: GPUBuffer
  readonly ib: GPUBuffer
  readonly edgeIB: GPUBuffer
}

const standIn = () => ({ destroy() {} }) as unknown as GPUBuffer

export class RecordingInstanceSteps {
  private readonly runs = {
    shadow: new InstanceStepRuns(),
    sectionCap: new InstanceStepRuns(),
    transparent: new InstanceStepRuns(),
    edges: new InstanceStepRuns(),
  }
  private readonly writes: Record<InstanceStepPass, number> = { shadow: 0, sectionCap: 0, transparent: 0, edges: 0 }
  private readonly devices = Object.fromEntries(
    PASSES.map(pass => [
      pass,
      {
        createBuffer: standIn,
        queue: {
          writeBuffer: () => {
            this.writes[pass]++
          },
        } as unknown as GPUQueue,
      },
    ]),
  ) as unknown as Record<InstanceStepPass, Pick<GPUDevice, 'createBuffer' | 'queue'>>
  private assets = new Map<string, AssetStandIns>()
  private current: Record<InstanceStepPass, InstanceStepPassMetrics> = {
    shadow: IDLE,
    sectionCap: IDLE,
    transparent: IDLE,
    edges: IDLE,
  }

  get metrics(): InstanceStepMetrics {
    return { ...this.current }
  }

  /** Plan and upload every instance-step pass of one frame. */
  stage(frame: InstanceStepFrame) {
    const live = new Map<string, AssetStandIns>()
    const all: InstanceStepMesh[] = frame.meshes.map((mesh, index) => {
      const key = instanceAssetKey(mesh)
      let asset = live.get(key) ?? this.assets.get(key)
      if (!asset) asset = { vb: standIn(), ib: standIn(), edgeIB: standIn() }
      live.set(key, asset)
      const alpha = effectiveDisplayAlpha(mesh.color[3], frame.displayMode)
      return {
        vb: asset.vb,
        ib: asset.ib,
        ic: mesh.indices.length,
        edgeIB: mesh.edgeIndices.length ? asset.edgeIB : null,
        edgeIC: mesh.edgeIndices.length,
        transform: mesh.transform,
        color: mesh.color,
        styleAlpha: alpha,
        styleSelected: index === frame.selected ? 1 : 0,
        styleEdge: frame.displayMode === 'edges' ? 1 : 0,
        styleHovered: 0,
        material: mesh.material,
      }
    })
    this.assets = live
    const opaque: InstanceStepMesh[] = []
    const transparent: InstanceStepMesh[] = []
    const edges: InstanceStepMesh[] = []
    all.forEach((mesh, index) => {
      if (!frame.visible(index)) return
      if (isTransparentAlpha(mesh.styleAlpha)) transparent.push(mesh)
      else opaque.push(mesh)
      if (mesh.edgeIB && (frame.displayMode === 'edges' || index === frame.selected)) edges.push(mesh)
    })
    const next = { ...this.current }
    next.shadow = frame.shadows
      ? this.pass(
          'shadow',
          opaque.filter(mesh => mesh.ic > 0),
          false,
          'asset',
        )
      : this.idle('shadow')
    if (frame.section && frame.displayMode !== 'xray' && opaque.length)
      next.sectionCap = opaque.every(mesh => instanceStepEligible(mesh))
        ? this.pass('sectionCap', opaque, false, 'asset')
        : this.idle('sectionCap', true)
    else next.sectionCap = this.idle('sectionCap')
    if (frame.transparency === 'sorted' && transparent.length) {
      const custom = frame.shadingModel !== 'phong'
      next.transparent =
        !custom && transparent.every(mesh => instanceStepEligible(mesh))
          ? this.pass('transparent', transparent, false, 'consecutive')
          : this.idle('transparent', true)
    } else next.transparent = this.idle('transparent')
    next.edges = this.pass('edges', edges, true, 'asset')
    this.current = next
  }

  clear() {
    for (const pass of PASSES) this.runs[pass].clear()
    this.assets.clear()
    this.current = { shadow: IDLE, sectionCap: IDLE, transparent: IDLE, edges: IDLE }
  }

  private pass(
    pass: InstanceStepPass,
    meshes: readonly InstanceStepMesh[],
    edges: boolean,
    order: 'asset' | 'consecutive',
  ): InstanceStepPassMetrics {
    const runs = this.runs[pass].plan(meshes, { edges, order })
    if (!runs.length) return IDLE
    this.writes[pass] = 0
    this.runs[pass].upload(this.devices[pass])
    return {
      drawCalls: runs.length,
      instances: runs.reduce((sum, run) => sum + run.count, 0),
      recordWrites: this.writes[pass],
      perEntity: false,
    }
  }

  private idle(pass: InstanceStepPass, perEntity = false): InstanceStepPassMetrics {
    this.runs[pass].plan([], { order: 'asset' })
    return perEntity ? { ...IDLE, perEntity: true } : IDLE
  }
}
