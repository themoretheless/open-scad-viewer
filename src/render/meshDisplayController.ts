/**
 * Display scheduling state of one renderer: the user's budget preset, the
 * progressive level store of the current device, levels produced on demand by
 * the display level worker and the batched bounding-box layer. The renderer
 * calls `update` once per frame; the controller decides whether LOD selection
 * has to run again and returns the meshes whose bound geometry changed.
 */
import {
  DEFAULT_DISPLAY_BUDGET,
  ProgressiveLevelStore,
  boxProxiesOn,
  geometryKey,
  sameDisplayBudget,
  type DeferredLevelLoader,
  type DisplayBudget,
} from './displayScheduler'
import type { GpuResidencyCache } from './gpuResidencyCache'
import { BoundingBoxLayer } from './boundingBoxDisplay'
import { ClusterLevelSource } from './clusterLevels'
import { updateMeshLods } from './meshLod'
import type { NativePickingCache } from './nativePickingCache'
import { createDisplayLevelProvider, OnDemandLevelSource, type DisplayLevelProvider } from './onDemandLevels'
import type { GMesh } from './types'
import type { ViewerLodCamera } from './viewerLod'

export interface MeshDisplayFrame {
  readonly dev: GPUDevice
  readonly meshes: readonly GMesh[]
  readonly camera: ViewerLodCamera
  /** Everything besides the budget that LOD selection depends on (camera, viewport, visibility). */
  readonly selectionKey: string
  readonly sceneEpoch: number
  readonly residency: GpuResidencyCache
  readonly nativePicking: NativePickingCache
  /** Whether the frame draws mesh `index` at all (visibility, isolation). */
  readonly drawable: (index: number) => boolean
}

export class MeshDisplayController {
  readonly boxes = new BoundingBoxLayer()
  private budget: DisplayBudget = DEFAULT_DISPLAY_BUDGET
  private lastKey = ''
  private store: { dev: GPUDevice; store: ProgressiveLevelStore; epoch: number } | null = null
  private onDemand: OnDemandLevelSource | null = null
  /** Stable loader handed to level stores; the on-demand source may appear after the store. */
  private readonly loader: DeferredLevelLoader = {
    load: (mesh, level) =>
      this.clusters.owns(mesh)
        ? this.clusters.load(mesh, level)
        : (this.onDemand?.load(mesh, level) ?? Promise.reject(new Error('No display level source'))),
    cancelStale: (mesh, keep) => {
      if (!this.clusters.owns(mesh)) this.onDemand?.cancelStale(mesh, keep)
    },
  }

  constructor(
    /** Ask for a frame; called when a level or summary landed after the frame that wanted it. */
    private readonly requestRender: () => void,
    private readonly createProvider: () => DisplayLevelProvider | null = createDisplayLevelProvider,
    /** Coarse levels built lazily for large plain meshes published without `lods`. */
    private readonly clusters: ClusterLevelSource = new ClusterLevelSource(),
  ) {}

  get displayBudget(): DisplayBudget {
    return this.budget
  }

  /** Returns true when the budget changed and selection will re-run. */
  setBudget(budget: DisplayBudget): boolean {
    if (sameDisplayBudget(budget, this.budget)) return false
    this.budget = budget
    this.lastKey = ''
    return true
  }

  /** Force LOD reselection on the next frame. */
  invalidate() {
    this.lastKey = ''
  }

  /** Bind the levels the scheduler wants; returns meshes whose geometry changed. */
  update(frame: MeshDisplayFrame): GMesh[] {
    const source = this.levelSource()
    let attached = 0
    if (source) {
      if (this.store && this.store.epoch !== frame.sceneEpoch) source.retain(frame.meshes)
      attached = source.attach(frame.meshes).length
    }
    attached += this.clusters.attach(frame.meshes).length
    let changed: GMesh[] = []
    if (frame.meshes.some(mesh => mesh.lod)) {
      const key = `${frame.selectionKey}|${this.budget.budgetBytes}|${this.budget.targetErrorPixels}|${boxProxiesOn(this.budget)}`
      if (key !== this.lastKey || attached) {
        this.lastKey = key
        const store = this.levelStore(frame)
        try {
          changed = updateMeshLods(frame.dev, frame.meshes, frame.camera, frame.nativePicking, undefined, {
            budget: this.budget,
            store,
          })
        } catch {
          // A refused selection keeps the bound geometry; it is still a valid display.
          changed = []
        }
      }
    }
    this.boxes.update(frame.dev, frame.meshes, frame.drawable)
    return changed
  }

  /** The device's residency cache was replaced: drop levels uploaded into the old one. */
  dropDeviceState() {
    this.store?.store.dispose()
    this.store = null
    this.boxes.release()
    this.lastKey = ''
  }

  dispose() {
    this.dropDeviceState()
    this.onDemand?.dispose()
    this.onDemand = null
  }

  private levelSource(): OnDemandLevelSource | null {
    if (!this.onDemand) {
      // Retried each frame until the app registers a provider.
      const provider = this.createProvider()
      this.onDemand = provider
        ? new OnDemandLevelSource(provider, geometryKey, () => {
            this.lastKey = ''
            this.requestRender()
          })
        : null
    }
    return this.onDemand
  }

  private levelStore(frame: MeshDisplayFrame): ProgressiveLevelStore {
    if (this.store?.dev !== frame.dev) {
      this.store?.store.dispose()
      const store = new ProgressiveLevelStore(
        frame.residency,
        () => {
          // A finer level landed: force reselection on the next frame.
          this.lastKey = ''
          this.requestRender()
        },
        undefined,
        this.loader,
      )
      this.store = { dev: frame.dev, store, epoch: frame.sceneEpoch }
    } else if (this.store.epoch !== frame.sceneEpoch) {
      this.store.store.retain(frame.meshes)
      this.store.epoch = frame.sceneEpoch
    }
    return this.store.store
  }
}
