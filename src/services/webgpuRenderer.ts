import { remapNativeFaceSelection } from './nativeFaceSelection'
/**
 * WebGPU 3D renderer — Phong shading, orbit camera, grid floor, axis gizmo.
 */
import {
  type Aabb3, type Mat4, type Vec3,
} from './math3d'
import {
  CameraHistory,
  cameraStatesEqual,
  type CameraHistorySnapshot,
  type CameraState,
} from './cameraHistory'
import { NativePickingCache } from './nativePickingCache'
import { warmGeometryKernel } from './geometry/kernel'
import { clientRayInKernel, projectPointInKernel } from './geometry/viewport'
import type { MeshData } from '../core/mesh'
import {
  ISO_PITCH,
  ISO_YAW,
  standardViewOrientation,
  type ProjectionMode,
  type StandardView,
} from './viewportModel'
export { projectAxesToScreen } from './viewportModel'
export type { ProjectionMode, StandardView } from './viewportModel'
import type {
  CameraChangeHandler,
  CameraHistoryChangeHandler,
  DisplayMode,
  DistanceMeasurement,
  HoverChangeHandler,
  MeasurementChangeHandler,
  PickHit,
  RendererLifecycleEvent,
  RendererStatusChangeHandler,
  SelectionChangeHandler,
  SelectionMode,
  SetMeshesOptions,
  SceneUploadMetrics,
  ShadingModel,
} from './rendererContracts'
import { DEFAULT_THEME, getThemePreset, type RenderTheme } from './rendererContracts'
export type {
  DisplayMode,
  DistanceMeasurement,
  PickHit,
  RendererLifecycleEvent,
  RenderTheme,
  SelectionMode,
} from './rendererContracts'
import {
  clampGestureDistance,
  computeOrbitUpdate,
  computePanUpdate,
  computePinchUpdate,
  computeWheelDistance,
} from './cameraGestures'
export { computePinchUpdate } from './cameraGestures'
export type { PinchCameraState, PinchPoint } from './cameraGestures'
import {
  chooseDepthCandidate,
  type DepthCandidate,
  type DepthCycleState,
} from './selectionCycling'
import {
  buildSceneAabbIndex,
  disposeSceneAabbIndex,
  type SceneAabbIndex,
} from './sceneAabbIndex'
import { TransparentSortBuffer } from './transparentOrdering'
import { MeshDrawBundle } from './meshDrawBundle'
import { ViewFrustum } from './viewFrustum'
import { MeshInstances } from './meshInstances'
import { computeOrbitCameraFrame } from './orbitCameraProjection'
import {
  OBJECT_UNIFORM_LAYOUT,
  SCENE_UNIFORM_LAYOUT,
} from './shaders'
import {
  PipelineFactory,
} from './pipelineFactory'
import { TextureResources } from './textureResources'
import { GpuCsgRenderer, type GpuCsgTree } from './gpuCsgPreview'
export type { GpuCsgTree } from './gpuCsgPreview'
import { GpuPicker } from './gpuPicker'
export { GpuPicker } from './gpuPicker'
import { GpuWboitRenderer } from './gpuWboit'
export { GpuWboitRenderer } from './gpuWboit'
import { GpuBufferArena, createZeroCopyMeshBuffers } from './gpuZeroCopyPipeline'
export { GpuBufferArena, createZeroCopyMeshBuffers } from './gpuZeroCopyPipeline'
import { runGpuMeshSlicer, computeSliceArea, type SlicerResult } from './gpuSlicer'
export { runGpuMeshSlicer, computeSliceArea, type SlicerResult } from './gpuSlicer'
export { GeometryWorkerPool, detectHardwareConcurrency } from './geometryWorkerPool'
import {
  CLICK_MOVE_THRESHOLD,
  DEFAULT_DISTANCE,
  DEFAULT_GRID_STEP,
  FOV_Y,
  getGpuExecutionTopologyReport,
  GRID_FADE_DISTANCES,
  GRID_MIN_EXTENT,
  MAX_DEPTH_CANDIDATES,
  MAX_DPR,
  MAX_ORBIT_PITCH,
  WHEEL_HISTORY_IDLE_MS,
  type Bounds,
  type GMesh,
  type GpuExecutionTopologyReport,
  type OverlaySlot,
  type WebGPUFrameDiagnostics,
} from '../renderer/types'
export {
  getGpuExecutionTopologyReport,
  getGpuMemoryArchitectureProfile,
  type GpuExecutionTopologyReport,
  type GpuMemoryArchitecture,
  type GpuMemoryArchitectureProfile,
  type WebGPUFrameDiagnostics,
} from '../renderer/types'
import {
  sameTypedArray,
  sortEdgeDraws,
  sortOpaqueDraws,
} from '../renderer/drawSorting'
export { sortEdgeDraws, sortOpaqueDraws } from '../renderer/drawSorting'
import {
  pointForOverlay,
  rebuildMeasurementBufferData,
  rebuildSelectionOverlaySlots,
  rebuildSourceHighlightOverlaySlots,
  releaseOverlaySlot,
} from '../renderer/overlayBuilder'
import {
  destroySceneMeshes,
  ensureMeshEdgeBuffer,
  rebuildEdgeBufferCacheMap,
  reconcileMeshStyles,
  stageSceneMeshes,
} from '../renderer/meshBufferManager'
import {
  findHitCandidatesForRay,
} from '../renderer/pickingController'
import {
  advanceSceneGeometryAnimation,
  encodeAndSubmitFrame,
} from '../renderer/framePassEncoder'

function clampDistanceValue(distance: number) {
  return clampGestureDistance(distance)
}

/**
 * Conservative 6-plane Gribb-Hartmann frustum-AABB intersection test for a row-major
 * 4x4 view-projection matrix (`vpMatrix`) using WebGPU clip conventions (`-w..w` XY, `0..w` Z).
 */
export function isAabbInFrustum(
  vpMatrix: ArrayLike<number>,
  min: readonly [number, number, number],
  max: readonly [number, number, number],
): boolean {
  if (vpMatrix.length < 16) return true
  for (let plane = 0; plane < 6; plane++) {
    const row = plane < 4 ? (plane >> 1) * 4 : 8
    const sign = plane % 2 === 0 ? 1 : -1
    const nx = plane === 4 ? vpMatrix[8]! : vpMatrix[12]! + sign * vpMatrix[row]!
    const ny = plane === 4 ? vpMatrix[9]! : vpMatrix[13]! + sign * vpMatrix[row + 1]!
    const nz = plane === 4 ? vpMatrix[10]! : vpMatrix[14]! + sign * vpMatrix[row + 2]!
    const nw = plane === 4 ? vpMatrix[11]! : vpMatrix[15]! + sign * vpMatrix[row + 3]!
    const len = Math.hypot(nx, ny, nz)
    if (!Number.isFinite(len) || len === 0) continue
    const px = nx >= 0 ? max[0] : min[0]
    const py = ny >= 0 ? max[1] : min[1]
    const pz = nz >= 0 ? max[2] : min[2]
    const dist = (nx * px + ny * py + nz * pz + nw) / len
    if (dist < -1e-5) return false
  }
  return true
}


/* ── Renderer class ───────────────────────────────── */

export class WebGPURenderer {
  private canvas: HTMLCanvasElement | null = null
  private dev: GPUDevice | null = null
  private ctx: GPUCanvasContext | null = null
  private fmt: GPUTextureFormat = 'bgra8unorm'
  private backgroundColor: [number, number, number] = [0.09, 0.09, 0.11]

  /**
   * Bind group/pipeline layouts and the cached shader-module/render-pipeline
   * factories. Built per device in buildPipelines; texture resources embed
   * its layouts, so the factory is built first.
   */
  private readonly pipelines = new PipelineFactory()
  /**
   * Matcap/env/shadow texture + sampler + bind group lifecycles (dummies,
   * async loads, shadow-map allocate/recreate, teardown). Rebinds re-encode
   * draw bundles and request a frame via the hooks.
   */
  private readonly textures = new TextureResources({
    getDevice: () => this.dev,
    onTexturesRebound: () => {
      // Draw bundles bake bind group 2; force re-encoding with the new one.
      this.clearDrawCaches()
      this.requestRender()
    },
    onLoadError: (error) => {
      // A missing capture/map must not break rendering: stay on the current
      // one. Reported through the lifecycle status (not a bare callback) so
      // the next validated frame clears it instead of leaving it sticky.
      this.reportError('frame', error)
    },
  })
  /**
   * Shadow-map cache. The depth pass is light-space, so it is independent of
   * the view camera and re-renders only when something it draws changes:
   * setters that mutate geometry, visibility, isolation, or the
   * opaque/transparent classification bump `shadowMapEpoch`; the light VP
   * floats are captured per shadow render as a safety net for any bounds
   * change that missed a bump. Time-based morph and fade animations are
   * checked per frame and force a re-render while active.
   */
  private shadowMapEpoch = 0
  private shadowMapRenderedEpoch = -1
  private shadowMapVP = new Float32Array(16)
  private shadowMapVPValid = false
  private sceneUB: GPUBuffer | null = null
  private sceneBG!: GPUBindGroup
  private depth: GPUTexture | null = null
  private depthView: GPUTextureView | null = null
  private csgRenderer: GpuCsgRenderer | null = null
  private csgPreviewTree: GpuCsgTree | null = null
  private csgDepthStencil: GPUTexture | null = null
  private csgDepthStencilView: GPUTextureView | null = null
  private gpuPicker: GpuPicker | null = null
  private wboit: GpuWboitRenderer | null = null
  private gpuBufferArena: GpuBufferArena | null = null
  private transparencyMode: 'sorted' | 'wboit' = 'sorted'

  private meshes: GMesh[] = []
  private readonly nativePicking = new NativePickingCache()
  private geometryFade: { started: number; progress: number } | null = null
  private geometryGhosts: Array<{ meshes: GMesh[]; started: number; alphas: number[] }> = []
  private sceneAabbIndex: SceneAabbIndex = buildSceneAabbIndex([])
  private sceneAabbIndexDirty = false
  /** Z axis as a line list; gridQuadVB covers the viewport in clip space. */
  private gridVB: GPUBuffer | null = null
  private gridVC = 0
  private gridQuadVB: GPUBuffer | null = null
  private bounds: Bounds | null = null
  private initialFitDone = false
  private projection: ProjectionMode = 'perspective'
  private perspectiveFovY = FOV_Y
  private gridVisible = true
  private gridStep = DEFAULT_GRID_STEP
  private displayMode: DisplayMode = 'shaded'
  private selected: number | null = null
  private selectedHit: PickHit | null = null
  private hovered: number | null = null
  private hoveredHit: PickHit | null = null
  private isolated = false
  private selectionMode: SelectionMode = 'face'
  private sectionEnabled = false
  private sectionNormal: Vec3 = [0, 0, 1]
  private sectionOffset = 0
  private measureActive = false
  private measurementPoints: Vec3[] = []
  private measurementVB: GPUBuffer | null = null
  private measurementVC = 0
  private selectionFaceSlot: OverlaySlot = { buffer: null, capacity: 0, count: 0 }
  private selectionLineSlot: OverlaySlot = { buffer: null, capacity: 0, count: 0 }
  private deepSelectionLineSlot: OverlaySlot = { buffer: null, capacity: 0, count: 0 }
  private sourceHighlightId: number | null = null
  private sourceFaceSlot: OverlaySlot = { buffer: null, capacity: 0, count: 0 }
  private sourceLineSlot: OverlaySlot = { buffer: null, capacity: 0, count: 0 }
  private cameraHistory = new CameraHistory(32)
  private depthCycleState: DepthCycleState | null = null
  private styleScratch = new Float32Array(4)
  private objectUniformScratch = new Float32Array(OBJECT_UNIFORM_LAYOUT.floats)
  private morphScratch = new Float32Array(4)
  private materialTailScratch = new Float32Array(8)
  private sceneUniformScratch = new Float32Array(SCENE_UNIFORM_LAYOUT.floats)
  private lastSceneUniforms = new Float32Array(SCENE_UNIFORM_LAYOUT.floats)
  private lastSceneUniformsValid = false
  /** Active render theme; fills the Scene theme tail every frame. */
  private theme: RenderTheme = DEFAULT_THEME
  /** Scene-level default shading model for meshes without their own material. */
  private defaultShadingModel: ShadingModel = 'phong'
  /**
   * Scene-level default material tail for meshes without their own material;
   * identity defaults reproduce the legacy look.
   */
  private defaultMaterial = {
    baseColor: [1, 1, 1] as [number, number, number],
    metallic: 0,
    roughness: 0.7,
    emissive: [0, 0, 0] as [number, number, number],
    alpha: 1,
  }
  /** Zero positions bound at vertex slot 1 whenever a mesh is not morphing. */
  private morphDummyVB: GPUBuffer | null = null
  // Keyed by geometryAssetId (content) so republished equal meshes hit; per
  // content id a small list of transform snapshots covers instance edits.
  private meshBoundsCache = new Map<string, {
    transformSnapshot: Float32Array; result: { local: Aabb3; world: Bounds } | null
  }[]>()
  private readonly viewFrustum = new ViewFrustum()
  private readonly opaqueDraws: GMesh[] = []
  private readonly edgeDraws: GMesh[] = []
  private readonly transparentSort = new TransparentSortBuffer()
  private readonly transparentDraws: GMesh[] = []
  private readonly opaqueInstances = new MeshInstances()
  private readonly transparentInstances = new MeshInstances()
  private readonly edgeInstances = new MeshInstances()
  private readonly opaqueBundle = new MeshDrawBundle()
  private readonly sectionCapBundle = new MeshDrawBundle()
  private readonly edgeBundle = new MeshDrawBundle()
  private edgeBuffersByVertexBuffer = new Map<GPUBuffer, GMesh[]>()
  private edgeWarmQueue: GMesh[] = []
  private edgeWarmHandle: number | ReturnType<typeof setTimeout> | null = null
  private edgeWarmIsIdle = false

  private pendingFrameToken: number | null = null
  private uploadMetrics: SceneUploadMetrics = { geometryUploadBytes: 0, geometryBuffersCreated: 0, reusedEntities: 0 }
  get sceneUploadMetrics(): SceneUploadMetrics { return { ...this.uploadMetrics } }
  onFrameSubmitted: ((token: number, submittedAt: number) => void) | null = null

  onSelectionChange: SelectionChangeHandler | null = null
  onHoverChange: HoverChangeHandler | null = null
  onMeasurementChange: MeasurementChangeHandler | null = null
  onCameraHistoryChange: CameraHistoryChangeHandler | null = null
  /** Fired once per drawn frame whenever the camera pose or projection changed. */
  onCameraChange: CameraChangeHandler | null = null
  private lastNotifiedCamera: CameraState | null = null
  /** Device/render lifecycle notifications; existing callback APIs remain unchanged. */
  onStatusChange: RendererStatusChangeHandler | null = null

  private status: RendererLifecycleEvent = { status: 'idle' }

  yaw = ISO_YAW; pitch = ISO_PITCH; dist = DEFAULT_DISTANCE
  tx = 0; ty = 0; tz = 0

  private raf = 0
  private frameRetryCount = 0
  /** Frame counter for async validation results; a late clean result must not clear a newer failure. */
  private frameSequence = 0
  private lastFailedFrame = 0
  private dead = true
  private initialized = false
  private lost = false
  private generation = 0
  private drawable = false
  private resizeObserver: ResizeObserver | null = null
  private inputBound = false
  private previousTouchAction = ''
  private drag = false; private pan = false
  private activePointer: number | null = null
  private activePointerType = ''
  private pointers = new Map<number, { x: number; y: number }>()
  private pinching = false
  private mx = 0; private my = 0
  private downX = 0; private downY = 0
  private downButton = -1
  private gestureMoved = false
  private gestureCameraStart: CameraState | null = null
  private lastWheelHistoryAt = -Infinity
  private hoverRaf = 0
  private hoverGeneration = 0
  private hoverX = 0
  private hoverY = 0
  private lastHoverEvaluatedX = -1
  private lastHoverEvaluatedY = -1
  private lastHoverEpoch = -1
  private sceneEpoch = 0
  private lastCameraEpoch = 0
  private lastCameraPose = { yaw: NaN, pitch: NaN, dist: NaN, tx: NaN, ty: NaN, tz: NaN, aspect: NaN }

  private getCameraEpoch(): number {
    const aspect = this.getAspect()
    if (this.yaw !== this.lastCameraPose.yaw || this.pitch !== this.lastCameraPose.pitch
      || this.dist !== this.lastCameraPose.dist || this.tx !== this.lastCameraPose.tx
      || this.ty !== this.lastCameraPose.ty || this.tz !== this.lastCameraPose.tz
      || aspect !== this.lastCameraPose.aspect) {
      this.lastCameraPose.yaw = this.yaw
      this.lastCameraPose.pitch = this.pitch
      this.lastCameraPose.dist = this.dist
      this.lastCameraPose.tx = this.tx
      this.lastCameraPose.ty = this.ty
      this.lastCameraPose.tz = this.tz
      this.lastCameraPose.aspect = aspect
      this.lastCameraEpoch++
    }
    return this.lastCameraEpoch
  }

  private getInteractionEpoch(): number {
    return this.sceneEpoch + this.getCameraEpoch()
  }

  async init(canvas: HTMLCanvasElement): Promise<boolean> {
    if (this.canvas || this.dev || this.initialized) this.teardown()

    const generation = ++this.generation
    this.canvas = canvas
    this.dead = false
    this.lost = false
    this.updateStatus({ status: 'initializing' })

    try {
      if (typeof navigator === 'undefined' || !navigator.gpu) {
        this.teardown()
        this.updateStatus({
          status: 'unavailable',
          reason: 'webgpu',
          message: 'WebGPU is unavailable in this environment.',
        })
        return false
      }

      const gpu = navigator.gpu
      const adapter = await gpu.requestAdapter()
      if (!adapter || !this.isCurrentInit(generation, canvas)) {
        if (this.isCurrentInit(generation, canvas)) {
          this.teardown()
          this.updateStatus({
            status: 'unavailable',
            reason: 'adapter',
            message: 'No compatible WebGPU adapter was found.',
          })
        }
        return false
      }

      await warmGeometryKernel()
      if (!this.isCurrentInit(generation, canvas)) return false
      const device = await adapter.requestDevice()
      if (!this.isCurrentInit(generation, canvas)) {
        device.destroy()
        return false
      }

      // Own the device before any further operation that can throw, so the
      // outer failure path can always release it.
      this.dev = device
      const context = canvas.getContext('webgpu')
      if (!context) {
        this.teardown()
        this.updateStatus({
          status: 'unavailable',
          reason: 'context',
          message: 'The canvas could not create a WebGPU context.',
        })
        return false
      }

      this.ctx = context
      this.fmt = gpu.getPreferredCanvasFormat()
      context.configure({ device, format: this.fmt, alphaMode: 'premultiplied' })

      void device.lost.then(info => {
        if (this.dev !== device || this.dead) return
        this.lost = true
        this.initialized = false
        this.drawable = false
        this.cancelPendingHover()
        if (this.raf) cancelAnimationFrame(this.raf)
        this.raf = 0
        this.updateStatus({
          status: 'device-lost',
          reason: info.reason,
          message: info.message,
        })
      })
      // Errors outside the per-frame validation scope (pipeline creation,
      // texture uploads) would otherwise only reach the console.
      device.addEventListener?.('uncapturederror', event => {
        if (this.dev !== device || this.dead) return
        this.reportError('frame', new Error((event as GPUUncapturedErrorEvent).error.message))
      })

      this.buildPipelines()
      this.buildSceneUB()
      this.buildGrid()
      this.initialized = true
      this.bindInput()
      this.observeResize()
      this.resize()
      this.requestRender()
      this.updateStatus({ status: 'ready' })
      return true
    } catch (error) {
      if (this.isCurrentInit(generation, canvas)) {
        this.teardown()
        this.reportError('initialization', error)
      }
      return false
    }
  }

  private isCurrentInit(generation: number, canvas: HTMLCanvasElement) {
    return generation === this.generation && this.canvas === canvas && !this.dead
  }

  private buildPipelines() {
    const dev = this.dev!
    this.pipelines.build(dev, this.fmt)
    this.morphDummyVB?.destroy()
    this.morphDummyVB = dev.createBuffer({ size: 12, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    // The matcap/env groups embed the shadow view at bindings 2/3, so texture
    // init creates the shadow resources before those bind groups.
    this.textures.init(dev, {
      matcapBGL: this.pipelines.matcapBGL,
      envBGL: this.pipelines.envBGL,
      shadowBGL: this.pipelines.shadowBGL,
    })
    this.pipelines.warmup(dev)
  }

  private buildSceneUB() {
    const dev = this.dev!
    this.sceneUB = dev.createBuffer({ size: SCENE_UNIFORM_LAYOUT.bytes, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.lastSceneUniformsValid = false
    this.sceneBG = dev.createBindGroup({
      layout: this.pipelines.sceneBGL,
      entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
    })
  }

  /** Distance from the eye at which grid lines have fully faded out. */
  private gridFadeDistance() {
    return Math.max(GRID_MIN_EXTENT, this.dist * GRID_FADE_DISTANCES)
  }

  /** Half-extent of the plane quad and the Z axis: covers everything up to the fade. */
  private gridExtent() {
    return this.gridFadeDistance() + Math.hypot(this.tx, this.ty, this.tz)
  }

  private buildGrid() {
    const dev = this.dev!
    this.gridVB?.destroy()
    this.gridQuadVB?.destroy()
    // The grid reconstructs the Z-up XY plane from this full-screen quad.
    const quad = new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1])
    this.gridQuadVB = dev.createBuffer({ size: quad.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    dev.queue.writeBuffer(this.gridQuadVB, 0, quad)

    const zc = [0.2, 0.45, 1, 0.95]
    const axis = new Float32Array([0, 0, -GRID_MIN_EXTENT, ...zc, 0, 0, GRID_MIN_EXTENT, ...zc])
    this.gridVC = axis.length / 7
    this.gridVB = dev.createBuffer({ size: axis.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    dev.queue.writeBuffer(this.gridVB, 0, axis)
  }

  setCsgPreviewTree(tree: GpuCsgTree | null) {
    this.csgPreviewTree = tree
    this.requestRender()
  }

  getCsgPreviewTree(): GpuCsgTree | null {
    return this.csgPreviewTree
  }

  private ensureCsgDepthStencil(dev: GPUDevice, width: number, height: number) {
    if (!this.csgDepthStencil || this.csgDepthStencil.width !== width || this.csgDepthStencil.height !== height) {
      this.csgDepthStencil?.destroy()
      this.csgDepthStencil = dev.createTexture({
        size: [width, height],
        format: 'depth24plus-stencil8',
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      })
      this.csgDepthStencilView = this.csgDepthStencil.createView()
    }
  }

  async gpuPickAt(clientX: number, clientY: number): Promise<number | null> {
    const canvas = this.canvas, dev = this.dev
    if (!canvas || !dev || !this.initialized || this.dead || this.lost || this.meshes.length === 0) return null
    const rect = canvas.getBoundingClientRect()
    const px = clientX - rect.left
    const py = clientY - rect.top
    if (px < 0 || py < 0 || px >= rect.width || py >= rect.height) return null

    if (!this.gpuPicker) {
      this.gpuPicker = new GpuPicker()
    }

    const cam = this.cameraState()
    return this.gpuPicker.pick(dev, this.meshes, {
      cameraViewProjection: cam.viewProjection,
      viewportWidth: rect.width,
      viewportHeight: rect.height,
      sectionNormal: this.sectionNormal,
      sectionOffset: this.sectionOffset,
      sectionEnabled: this.sectionEnabled,
    }, px, py)
  }

  getTransparencyMode(): 'sorted' | 'wboit' {
    return this.transparencyMode
  }

  setTransparencyMode(mode: 'sorted' | 'wboit'): void {
    if (this.transparencyMode !== mode) {
      this.transparencyMode = mode
      this.requestRender()
    }
  }

  get wboitRenderer(): GpuWboitRenderer {
    if (!this.wboit) {
      this.wboit = new GpuWboitRenderer()
    }
    return this.wboit
  }

  getBufferArena(): GpuBufferArena | null {
    if (!this.dev || this.dead || this.lost) return null
    if (!this.gpuBufferArena) {
      this.gpuBufferArena = new GpuBufferArena(this.dev)
    }
    return this.gpuBufferArena
  }

  async sliceSceneMeshesGpu(options: {
    planeNormal: [number, number, number]
    firstPlaneOffset: number
    layerThickness: number
    numLayers: number
  }): Promise<Array<{ meshIndex: number; slice: SlicerResult }>> {
    const results: Array<{ meshIndex: number; slice: SlicerResult }> = []
    for (let i = 0; i < this.meshes.length; i++) {
      const m = this.meshes[i]
      if (!m.visible) continue
      const slice = await runGpuMeshSlicer(m.vertices, m.indices, {
        strideFloats: 6,
        planeNormal: options.planeNormal,
        firstLayerDistance: options.firstPlaneOffset,
        layerStep: options.layerThickness,
        numLayers: options.numLayers,
      })
      results.push({ meshIndex: i, slice })
    }
    return results
  }

  setMeshes(meshes: MeshData[], options: SetMeshesOptions = {}) {
    this.cancelPendingHover()
    const dev = this.dev
    if (!dev || !this.initialized || this.dead || this.lost) return

    const previous = this.meshes
    const animate = (options.animate || this.geometryFade !== null || this.meshes.some(mesh => mesh.morph)) && !(typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches)

    const { next, nextSceneIndex, retainedGeometryBuffers, metrics } = stageSceneMeshes(meshes, {
      dev,
      previous,
      animate,
      displayMode: this.displayMode,
      defaultShadingModel: this.defaultShadingModel,
      defaultMaterial: this.defaultMaterial,
      objBGL: this.pipelines.objBGL,
      morphDummyVB: this.morphDummyVB,
      objectUniformScratch: this.objectUniformScratch,
      nativePicking: this.nativePicking,
      measureMeshBounds: (assetId, vertices, transform) =>
        this.measureMeshBounds(assetId, vertices, transform),
    })

    const nextBounds = this.combineBounds(next.map(mesh => mesh.worldBounds))
    const selectionChanged = this.selected !== null || this.isolated
    const hoverChanged = this.hovered !== null || this.hoveredHit !== null
    this.clearDrawCaches()
    this.meshes = next
    this.rebuildEdgeBufferCache()
    this.pendingFrameToken = options.frameToken ?? null
    this.uploadMetrics = metrics
    disposeSceneAabbIndex(this.sceneAabbIndex)
    this.sceneAabbIndex = nextSceneIndex
    this.sceneAabbIndexDirty = false
    this.bounds = nextBounds
    this.sceneEpoch++
    this.invalidateShadowMap()
    this.selected = null
    this.selectedHit = null
    this.hovered = null
    this.hoveredHit = null
    this.isolated = false
    this.resetDepthCycle()
    if (!options.preserveMeasurement) {
      this.measurementPoints = []
      this.measureActive = false
    }
    this.rebuildMeasurementBuffer()
    this.clearSelectionOverlayBuffers()
    this.sourceHighlightId = null
    this.clearSourceHighlightOverlayBuffers()
    this.emitMeasurementChange()
    this.destroyMeshes(previous, retainedGeometryBuffers)
    this.geometryFade = null
    if (this.displayMode === 'edges') {
      for (const mesh of next) this.ensureEdgeBuffer(mesh)
    }
    // Pre-build the remaining edge buffers off the critical path so the first
    // hover highlight does not pay a synchronous multi-megabyte upload.
    this.scheduleEdgeBufferWarmup()
    if (selectionChanged) this.emitSelectionChange()
    if (hoverChanged) {
      try { this.onHoverChange?.(null) } catch { /* UI callbacks must not break rendering. */ }
    }

    if (nextBounds && !this.initialFitDone) {
      this.initialFitDone = true
      this.applyFitBounds(nextBounds)
      this.requestRender()
    } else {
      this.requestRender()
    }
  }

  /**
   * Scalar min/max scan over the interleaved vertex stream (no per-vertex
   * allocations or closures). Cache each immutable geometry/instance pair so
   * shared assets do not evict each other's transformed bounds. Weak keys do
   * not keep previous scene instances alive; snapshots detect transform edits.
   */
  private measureMeshBounds(assetId: string | null, vertices: Float32Array, transform: Mat4): { local: Aabb3; world: Bounds } | null {
    // Meshes without a content identity compute bounds without caching.
    const entries = assetId === null ? undefined : this.meshBoundsCache.get(assetId)
    if (entries) {
      for (const entry of entries) {
        if (sameTypedArray(entry.transformSnapshot, transform)) return entry.result
      }
    }

    const m0 = transform[0], m1 = transform[1], m2 = transform[2], m3 = transform[3]
    const m4 = transform[4], m5 = transform[5], m6 = transform[6], m7 = transform[7]
    const m8 = transform[8], m9 = transform[9], m10 = transform[10], m11 = transform[11]
    const m12 = transform[12], m13 = transform[13], m14 = transform[14], m15 = transform[15]

    let lMinX = Infinity, lMinY = Infinity, lMinZ = Infinity
    let lMaxX = -Infinity, lMaxY = -Infinity, lMaxZ = -Infinity
    let wMinX = Infinity, wMinY = Infinity, wMinZ = Infinity
    let wMaxX = -Infinity, wMaxY = -Infinity, wMaxZ = -Infinity
    for (let i = 0; i + 2 < vertices.length; i += 6) {
      const x = vertices[i], y = vertices[i+1], z = vertices[i+2]
      if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) continue
      const w = m12*x + m13*y + m14*z + m15
      const iw = Number.isFinite(w) && Math.abs(w) > 1e-12 ? 1 / w : 1
      const wx = (m0*x + m1*y + m2*z + m3) * iw
      const wy = (m4*x + m5*y + m6*z + m7) * iw
      const wz = (m8*x + m9*y + m10*z + m11) * iw
      if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) continue
      if (x < lMinX) lMinX = x
      if (y < lMinY) lMinY = y
      if (z < lMinZ) lMinZ = z
      if (x > lMaxX) lMaxX = x
      if (y > lMaxY) lMaxY = y
      if (z > lMaxZ) lMaxZ = z
      if (wx < wMinX) wMinX = wx
      if (wy < wMinY) wMinY = wy
      if (wz < wMinZ) wMinZ = wz
      if (wx > wMaxX) wMaxX = wx
      if (wy > wMaxY) wMaxY = wy
      if (wz > wMaxZ) wMaxZ = wz
    }
    if (!Number.isFinite(lMinX)) return null
    const result = {
      local: { min: [lMinX, lMinY, lMinZ] as Vec3, max: [lMaxX, lMaxY, lMaxZ] as Vec3 },
      world: {
        center: [(wMinX+wMaxX)/2, (wMinY+wMaxY)/2, (wMinZ+wMaxZ)/2] as [number, number, number],
        radius: Math.hypot(wMaxX-wMinX, wMaxY-wMinY, wMaxZ-wMinZ) / 2,
        min: [wMinX, wMinY, wMinZ] as [number, number, number],
        max: [wMaxX, wMaxY, wMaxZ] as [number, number, number],
      },
    }
    if (assetId === null) return result
    let list = this.meshBoundsCache.get(assetId)
    if (!list) {
      list = []
      this.meshBoundsCache.set(assetId, list)
    }
    if (list.length >= 8) list.shift()
    list.push({ transformSnapshot: new Float32Array(transform), result })
    return result
  }

  private combineBounds(bounds: Bounds[]): Bounds | null {
    if (!bounds.length) return null
    const min: Vec3 = [Infinity, Infinity, Infinity]
    const max: Vec3 = [-Infinity, -Infinity, -Infinity]
    for (const bound of bounds) {
      for (let axis = 0; axis < 3; axis++) {
        min[axis] = Math.min(min[axis], bound.min[axis])
        max[axis] = Math.max(max[axis], bound.max[axis])
      }
    }
    const center: Vec3 = [(min[0]+max[0])/2, (min[1]+max[1])/2, (min[2]+max[2])/2]
    return { center, radius: Math.hypot(max[0]-min[0], max[1]-min[1], max[2]-min[2]) / 2, min, max }
  }

  /** Match a recovered photo camera without changing the default CAD field of view. */
  setPerspectiveFieldOfView(radians: number) {
    if (!Number.isFinite(radians) || radians < 0.005 || radians > Math.PI * 0.85) return
    this.perspectiveFovY = radians
    this.resetDepthCycle()
    this.requestRender()
  }

  fitView() {
    if (this.bounds) this.fitBounds(this.bounds)
  }

  private fitBounds(bounds: Bounds) {
    this.commitCameraChange(() => this.applyFitBounds(bounds))
  }

  private applyFitBounds(bounds: Bounds) {
    const [x, y, z] = bounds.center
    this.tx = x; this.ty = y; this.tz = z
    const aspect = this.getAspect()
    const halfVertical = this.perspectiveFovY / 2
    const halfHorizontal = Math.atan(Math.tan(halfVertical) * aspect)
    const limitingHalfFov = Math.max(1e-6, Math.min(halfVertical, halfHorizontal))
    const radius = Math.max(bounds.radius, 0.5)
    this.dist = this.clampDistance(radius * 1.15 / Math.sin(limitingHalfFov))
  }

  get currentDisplayMode(): DisplayMode { return this.displayMode }
  get selectedIndex(): number | null { return this.selected }
  get currentHit(): PickHit | null { return this.selectedHit }
  get isIsolated(): boolean { return this.isolated }
  get canGoToPreviousView(): boolean { return this.cameraHistory.size > 0 }

  getCameraState(): CameraState {
    return {
      yaw: this.yaw,
      pitch: this.pitch,
      distance: this.dist,
      target: [this.tx, this.ty, this.tz],
      projection: this.projection,
    }
  }

  getCameraHistorySnapshot(): CameraHistorySnapshot {
    return this.cameraHistory.snapshot()
  }

  /** Restore Previous View entries after renderer/device re-initialization. */
  restoreCameraHistory(snapshot: CameraHistorySnapshot): boolean {
    const wasAvailable = this.canGoToPreviousView
    if (!this.cameraHistory.restore(snapshot)) return false
    if (wasAvailable !== this.canGoToPreviousView) this.emitCameraHistoryChange()
    return true
  }

  /**
   * Restore a previously captured camera without creating a navigation-history
   * entry. This is intended for renderer/device re-initialization.
   */
  restoreCameraState(state: CameraState): boolean {
    if (!Number.isFinite(state.yaw)
        || !Number.isFinite(state.pitch)
        || !Number.isFinite(state.distance)
        || !state.target.every(Number.isFinite)
        || (state.projection !== 'perspective' && state.projection !== 'orthographic')) return false
    const before = this.getCameraState()
    this.yaw = state.yaw
    this.pitch = Math.max(-MAX_ORBIT_PITCH, Math.min(MAX_ORBIT_PITCH, state.pitch))
    this.dist = this.clampDistance(state.distance)
    ;[this.tx, this.ty, this.tz] = state.target
    this.projection = state.projection
    if (cameraStatesEqual(before, this.getCameraState())) return false
    this.gestureCameraStart = null
    this.lastWheelHistoryAt = -Infinity
    this.resetDepthCycle()
    this.requestRender()
    return true
  }

  previousView(): CameraState | null {
    this.flushActiveGestureCameraSnapshot()
    const state = this.cameraHistory.previous()
    if (!state) return null
    this.yaw = state.yaw
    this.pitch = state.pitch
    this.dist = this.clampDistance(state.distance)
    ;[this.tx, this.ty, this.tz] = state.target
    this.projection = state.projection
    if (this.gestureCameraStart) this.gestureCameraStart = this.getCameraState()
    this.lastWheelHistoryAt = -Infinity
    this.resetDepthCycle()
    this.emitCameraHistoryChange()
    this.requestRender()
    return state
  }

  setDisplayMode(mode: DisplayMode) {
    if (this.displayMode === mode) return
    this.displayMode = mode
    if (mode === 'edges') {
      for (const mesh of this.meshes) this.ensureEdgeBuffer(mesh)
    }
    // X-ray/edges modes reclassify the opaque/transparent split.
    this.invalidateShadowMap()
    this.updateMeshStyles()
    this.requestRender()
  }

  setBackgroundColor(color: readonly [number, number, number]) {
    if (!color.every(value => Number.isFinite(value) && value >= 0 && value <= 1)) {
      throw new RangeError('Renderer background channels must be finite values in [0, 1]')
    }
    this.backgroundColor = [...color]
    this.requestRender()
  }

  get currentTheme(): RenderTheme { return this.theme }

  /**
   * Applies a render theme preset to the Scene uniform theme tail. A preset
   * with a backgroundColor also takes over the clear color; otherwise the
   * app-driven background (setBackgroundColor) is left untouched.
   */
  setTheme(themeId: string) {
    const theme = getThemePreset(themeId)
    if (!theme) throw new Error(`Renderer: unknown theme '${themeId}'`)
    this.theme = theme
    if (theme.backgroundColor) this.backgroundColor = [...theme.backgroundColor]
    this.requestRender()
  }

  get currentDefaultShadingModel(): ShadingModel { return this.defaultShadingModel }

  /**
   * Scene-level default shading model: applies to every mesh that does not
   * carry its own material; per-entity materials always win.
   */
  setDefaultShadingModel(model: ShadingModel) {
    if (this.defaultShadingModel === model) return
    this.defaultShadingModel = model
    for (const mesh of this.meshes) {
      if (!mesh.material) mesh.shadingModel = model
    }
    this.requestRender()
  }

  /**
   * Scene-level default material tail (baseColor/metallic/roughness, plus an
   * optional alpha multiplier) for meshes without their own material; rewrites
   * their uniform tails in place. Alpha below 1 routes those meshes through
   * the transparent pass via Obj.style.x.
   */
  setDefaultMaterial(defaults: {
    baseColor?: readonly [number, number, number]
    metallic?: number
    roughness?: number
    emissive?: readonly [number, number, number]
    alpha?: number
  }) {
    const current = this.defaultMaterial
    if (defaults.baseColor) current.baseColor = [...defaults.baseColor]
    if (defaults.emissive) current.emissive = [...defaults.emissive]
    if (defaults.metallic !== undefined) current.metallic = defaults.metallic
    if (defaults.roughness !== undefined) current.roughness = defaults.roughness
    if (defaults.alpha !== undefined && Number.isFinite(defaults.alpha)) {
      current.alpha = Math.min(1, Math.max(0, defaults.alpha))
    }
    if (!this.dev) return
    const tail = this.materialTailScratch
    for (const mesh of this.meshes) {
      if (mesh.material) continue
      tail[0] = current.baseColor[0]; tail[1] = current.baseColor[1]; tail[2] = current.baseColor[2]
      tail[3] = current.metallic
      tail[4] = current.emissive[0]; tail[5] = current.emissive[1]; tail[6] = current.emissive[2]
      tail[7] = current.roughness
      this.dev.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.materialByteOffset, tail)
    }
    // Alpha lives in the style vector, not the material tail; restyle so the
    // opaque/transparent draw split observes the new default opacity.
    this.invalidateShadowMap()
    this.updateMeshStyles()
    this.requestRender()
  }

  /** Current matcap preset id ('procedural' = shader fallback path). */
  get currentMatcap(): string { return this.textures.matcapId }

  /**
   * Selects the matcap capture for the meshMatcap shader. 'procedural' binds
   * the 1×1 dummy (the shader's procedural fallback, pixel-identical to the
   * pre-texture look); other presets fetch their PNG and upload it as a
   * GPUTexture. Async: the frame is requested once the texture is bound.
   */
  setMatcapTexture(id: string): Promise<void> {
    return this.textures.setMatcapTexture(id)
  }

  /** Current environment-map preset id ('none' = analytic-lighting path). */
  get currentEnv(): string { return this.textures.envId }

  /** Mark the cached shadow map stale: geometry/visibility/opacity changed. */
  private invalidateShadowMap() {
    this.shadowMapEpoch++
  }

  /** Drop the cached shadow map outright (map texture recreated or destroyed). */
  private resetShadowMapCache() {
    this.shadowMapRenderedEpoch = -1
    this.shadowMapVPValid = false
  }

  /**
   * Selects the equirect environment map for the meshPbr shader (IBL).
   * 'none' binds the 1×1 dummy (the shader's analytic-lighting fallback,
   * pixel-identical to the pre-IBL look); other presets fetch their PNG and
   * upload it as a GPUTexture. Async: the frame is requested once bound.
   */
  setEnvMap(id: string): Promise<void> {
    return this.textures.setEnvMap(id)
  }

  clearSelection() {
    if (this.selected === null && !this.isolated) return
    const sourceVisibilityChanged = this.isolated
    this.resetDepthCycle(false)
    this.selected = null
    this.selectedHit = null
    this.isolated = false
    if (sourceVisibilityChanged) this.invalidateShadowMap()
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    if (sourceVisibilityChanged) this.rebuildSourceHighlightOverlay()
    this.emitSelectionChange()
    this.requestRender()
  }

  fitSelection(): boolean {
    if (this.selected === null || !this.meshes[this.selected]) return false
    this.fitBounds(this.meshes[this.selected].worldBounds)
    return true
  }

  /** Toggle isolation and return its new state. Requires a current selection. */
  toggleIsolateSelection(): boolean {
    if (this.isolated) {
      this.isolated = false
    } else {
      if (this.selected === null || !this.meshes[this.selected]) return false
      this.isolated = true
    }
    // Isolation changes the visible set the shadow pass draws.
    this.invalidateShadowMap()
    this.resetDepthCycle()
    this.rebuildSelectionOverlays()
    this.rebuildSourceHighlightOverlay()
    this.emitSelectionChange()
    this.requestRender()
    return this.isolated
  }

  resetView() {
    this.commitCameraChange(() => {
      this.yaw = ISO_YAW
      this.pitch = ISO_PITCH
      if (this.bounds) this.applyFitBounds(this.bounds)
      else {
        this.tx = 0; this.ty = 0; this.tz = 0
        this.dist = DEFAULT_DISTANCE
      }
    })
  }

  setProjection(projection: ProjectionMode) {
    if (this.projection === projection) return
    this.commitCameraChange(() => { this.projection = projection })
  }

  setGridVisible(visible: boolean) {
    if (this.gridVisible === visible) return
    this.gridVisible = visible
    this.requestRender()
  }

  /** Key-light contact shadows (mesh surfaces + grid floor). Default off. */
  get shadowsOn(): boolean { return this.textures.shadowsEnabled }

  /**
   * Toggles the contact-shadow pass. Enabling lazily allocates the real
   * 1024² depth map and rebinds every group that embeds its view; disabling
   * rebinds the 1×1 dummy and clears the enable flag, restoring the exact
   * pre-shadow look.
   */
  setShadowsEnabled(enabled: boolean) {
    if (this.textures.shadowsEnabled === enabled) return
    if (this.textures.setShadowsEnabled(enabled, this.dev)) {
      // The (re)created map starts unrendered: enabling forces one depth pass.
      this.resetShadowMapCache()
      // Draw bundles bake bind groups; force re-encoding with the new ones.
      this.clearDrawCaches()
    }
    this.requestRender()
  }

  /** Spacing between grid lines in model units (OpenSCAD millimetres). */
  setGridStep(step: number) {
    if (!Number.isFinite(step) || step <= 0 || this.gridStep === step) return
    this.gridStep = step
    this.requestRender()
  }

  setSelectionMode(mode: SelectionMode) {
    if (this.selectionMode === mode) return
    this.resetDepthCycle()
    this.selectionMode = mode
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    this.requestRender()
  }

  /**
   * Highlights every surviving triangle produced by a source operation.
   * This state is independent from viewport selection and preselection.
   */
  setSourceHighlight(sourceId: number | null) {
    const next = sourceId !== null && Number.isInteger(sourceId) && sourceId >= 0 ? sourceId : null
    if (this.sourceHighlightId === next) return
    this.sourceHighlightId = next
    this.rebuildSourceHighlightOverlay()
    this.requestRender()
  }

  restoreNativeFaceSelection(previous:MeshData,hit:PickHit,index:number):boolean {
    const next=this.meshes[index]
    if(this.selectionMode!=='face'||!next)return false
    const mapped=remapNativeFaceSelection(previous,next,hit,index)
    if(!mapped)return false
    this.resetDepthCycle(false)
    this.setSelection(index,mapped)
    return true
  }

  selectMesh(index: number | null) {
    this.resetDepthCycle(false)
    this.setSelection(index, null)
  }

  setMeshVisibility(index: number, visible: boolean) {
    const mesh = this.meshes[index]
    if (!mesh || mesh.visible === visible) return
    mesh.visible = visible
    this.finishMeshVisibilityChange(
      this.meshes.filter(candidate => candidate.visible).map(candidate => candidate.worldBounds),
    )
  }

  /**
   * Restore object visibility as one scene transaction. Missing array entries
   * leave their mesh unchanged; extra entries are ignored.
   */
  setMeshVisibilityBatch(visibility: readonly boolean[]): boolean {
    let changed = false
    const visibleBounds: Bounds[] = []
    for (let index = 0; index < this.meshes.length; index++) {
      const mesh = this.meshes[index]
      const nextVisible = visibility[index]
      if (typeof nextVisible === 'boolean' && mesh.visible !== nextVisible) {
        mesh.visible = nextVisible
        changed = true
      }
      if (mesh.visible) visibleBounds.push(mesh.worldBounds)
    }
    if (!changed) return false
    this.finishMeshVisibilityChange(visibleBounds)
    return true
  }

  private finishMeshVisibilityChange(visibleBounds: Bounds[]) {
    this.cancelPendingHover()
    const selectionMetadataChanged = this.clearDepthCycleState()
    let selectionChanged = selectionMetadataChanged
    let stylesChanged = false
    const selectedHidden = this.selected !== null && !this.meshes[this.selected]?.visible
    const selectedHitHidden = this.selectedHit !== null
      && !this.meshes[this.selectedHit.meshIndex]?.visible
    if (selectedHidden) {
      this.selected = null
      this.selectedHit = null
      this.isolated = false
      selectionChanged = true
      stylesChanged = true
    } else if (selectedHitHidden) {
      this.selectedHit = null
      selectionChanged = true
      stylesChanged = true
    }

    const hoverHidden = (this.hovered !== null && !this.meshes[this.hovered]?.visible)
      || (this.hoveredHit !== null && !this.meshes[this.hoveredHit.meshIndex]?.visible)
    if (hoverHidden) {
      this.hovered = null
      this.hoveredHit = null
      stylesChanged = true
    }

    // Rebuild the O(n log n) index lazily on the next pointer query, even when
    // a recovery transaction changes hundreds of object visibility flags.
    this.sceneAabbIndexDirty = true
    this.bounds = this.combineBounds(visibleBounds)
    this.invalidateShadowMap()
    if (stylesChanged) this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    this.rebuildSourceHighlightOverlay()
    if (selectionChanged) this.emitSelectionChange()
    if (hoverHidden) {
      try { this.onHoverChange?.(null) } catch { /* UI callbacks must not break rendering. */ }
    }
    this.requestRender()
  }

  setPreselection(index: number | null) {
    this.cancelPendingHover()
    if (index !== null && (!this.meshes[index] || !this.meshes[index].visible)) index = null
    if (this.hovered === index && this.hoveredHit === null) return
    this.hovered = index
    this.hoveredHit = null
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    try { this.onHoverChange?.(null) } catch { /* UI callbacks must not break rendering. */ }
    this.requestRender()
  }

  setSection(enabled: boolean, normal: Vec3, offset: number) {
    this.resetDepthCycle()
    const length = Math.hypot(...normal)
    this.sectionEnabled = enabled && length > 0 && Number.isFinite(length) && Number.isFinite(offset)
    this.sectionNormal = this.sectionEnabled
      ? [normal[0] / length, normal[1] / length, normal[2] / length]
      : [0, 0, 1]
    this.sectionOffset = Number.isFinite(offset) ? offset : 0
    this.sceneEpoch++
    this.requestRender()
  }

  setMeasureMode(active: boolean) {
    if (this.measureActive === active) return
    this.resetDepthCycle()
    this.measureActive = active
    this.emitMeasurementChange()
  }

  clearMeasurement() {
    if (!this.measurementPoints.length) return
    this.measurementPoints = []
    this.rebuildMeasurementBuffer()
    this.emitMeasurementChange()
    this.requestRender()
  }

  /** Rehydrate inspection state after a WebGPU device/context rebuild. */
  restoreMeasurement(measurement: DistanceMeasurement | null, active = false): boolean {
    const points = measurement?.points ?? []
    if (points.length > 2 || points.some(point => (
      point.length !== 3 || !point.every(Number.isFinite)
    ))) return false
    this.measurementPoints = points.map(point => [...point] as Vec3)
    this.measureActive = active
    this.rebuildMeasurementBuffer()
    this.emitMeasurementChange()
    this.requestRender()
    return true
  }

  /** Rebuild at publication so cached handles never outlive their scene. */
  private rebuildEdgeBufferCache() {
    rebuildEdgeBufferCacheMap(this.edgeBuffersByVertexBuffer, this.meshes)
  }

  private ensureEdgeBuffer(mesh: GMesh) {
    ensureMeshEdgeBuffer(this.dev, mesh, this.edgeBuffersByVertexBuffer)
  }

  /** Queues idle-time edge-buffer creation for visible meshes that lack one. */
  private scheduleEdgeBufferWarmup() {
    this.cancelEdgeBufferWarmup()
    if (!this.dev || this.dead || this.lost) return
    const pending: GMesh[] = []
    for (const mesh of this.meshes) {
      if (mesh.visible && !mesh.edgeIB && mesh.edgeIndices.length) pending.push(mesh)
    }
    this.edgeWarmQueue = pending
    if (pending.length) this.requestEdgeWarmSlice()
  }

  private requestEdgeWarmSlice() {
    if (this.edgeWarmHandle !== null) return
    // requestIdleCallback does not exist in workers or in most test runtimes.
    if (typeof requestIdleCallback === 'function') {
      this.edgeWarmIsIdle = true
      this.edgeWarmHandle = requestIdleCallback(deadline => this.runEdgeWarmSlice(deadline))
    } else {
      this.edgeWarmIsIdle = false
      this.edgeWarmHandle = setTimeout(() => this.runEdgeWarmSlice(null), 0)
    }
  }

  private runEdgeWarmSlice(deadline: IdleDeadline | null) {
    this.edgeWarmHandle = null
    if (this.dead || this.lost || !this.dev) {
      this.edgeWarmQueue = []
      return
    }
    do {
      const mesh = this.edgeWarmQueue.shift()
      if (!mesh) return
      this.ensureEdgeBuffer(mesh)
    } while (this.edgeWarmQueue.length && deadline !== null && deadline.timeRemaining() > 3)
    if (this.edgeWarmQueue.length) this.requestEdgeWarmSlice()
  }

  private cancelEdgeBufferWarmup() {
    this.edgeWarmQueue = []
    if (this.edgeWarmHandle === null) return
    if (this.edgeWarmIsIdle) {
      if (typeof cancelIdleCallback === 'function') cancelIdleCallback(this.edgeWarmHandle as number)
    } else {
      clearTimeout(this.edgeWarmHandle)
    }
    this.edgeWarmHandle = null
  }

  /**
   * Reconciles every mesh's style uniform, but only writes to the GPU for
   * meshes whose style vector actually changed since the last write — a hover
   * or selection change touches at most two uniform buffers. Bulk transitions
   * (display mode, new scenes) naturally dirty every mesh and fall back to a
   * full pass through the same loop.
   */
  private updateMeshStyles() {
    reconcileMeshStyles(
      this.dev,
      this.meshes,
      this.displayMode,
      this.defaultMaterial.alpha,
      this.geometryFade?.progress ?? 1,
      this.styleScratch,
      index => this.usesObjectSelectionStyle(index),
      index => this.usesObjectHoverStyle(index),
      mesh => this.ensureEdgeBuffer(mesh),
    )
  }

  private setSelection(index: number | null, hit: PickHit | null = null) {
    if (index !== null && (!this.meshes[index] || !this.meshes[index].visible)) index = null
    if (this.selected === index && this.selectedHit === hit && (!this.isolated || index !== null)) return
    const previousSelected = this.selected
    const wasIsolated = this.isolated
    this.selected = index
    this.selectedHit = index === null ? null : hit
    if (index === null) this.isolated = false
    if (wasIsolated && !this.isolated) this.invalidateShadowMap()
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    if (wasIsolated && (this.selected !== previousSelected || !this.isolated)) {
      this.rebuildSourceHighlightOverlay()
    }
    this.emitSelectionChange()
    this.requestRender()
  }

  private usesObjectSelectionStyle(index: number) {
    return index === this.selected && (this.selectionMode === 'object' || this.selectedHit === null)
  }

  private usesObjectHoverStyle(index: number) {
    return index === this.hovered && (this.selectionMode === 'object' || this.hoveredHit === null)
  }

  private emitSelectionChange() {
    try { this.onSelectionChange?.(this.selected, this.isolated, this.selectedHit) } catch { /* UI callbacks must not break rendering. */ }
  }

  private isMeshVisible(index: number) {
    const mesh = this.meshes[index]
    return !!mesh?.visible && (!this.isolated || index === this.selected)
  }

  setView(view: StandardView) {
    this.setCameraPreset(view, this.projection)
  }

  /** Apply orientation and projection as one history-bearing camera action. */
  setCameraPreset(view: StandardView, projection: ProjectionMode) {
    this.commitCameraChange(() => {
      this.projection = projection
      ;[this.yaw, this.pitch] = standardViewOrientation(view)
    })
  }

  private commitCameraChange(mutator: () => void): boolean {
    this.flushActiveGestureCameraSnapshot()
    const before = this.getCameraState()
    mutator()
    if (cameraStatesEqual(before, this.getCameraState())) return false
    this.recordCameraSnapshot(before)
    if (this.gestureCameraStart) this.gestureCameraStart = this.getCameraState()
    this.lastWheelHistoryAt = -Infinity
    this.resetDepthCycle()
    this.requestRender()
    return true
  }

  private recordCameraSnapshot(state: CameraState) {
    if (this.cameraHistory.record(state)) this.emitCameraHistoryChange()
  }

  private flushActiveGestureCameraSnapshot() {
    if (!this.gestureCameraStart) return
    const current = this.getCameraState()
    if (!cameraStatesEqual(this.gestureCameraStart, current)) {
      this.recordCameraSnapshot(this.gestureCameraStart)
    }
    this.gestureCameraStart = current
  }

  private emitCameraHistoryChange() {
    try { this.onCameraHistoryChange?.(this.canGoToPreviousView) } catch { /* UI callbacks must not break rendering. */ }
  }

  private clearDepthCycleState(invalidateSelectedHit = true): boolean {
    this.depthCycleState = null
    if (!invalidateSelectedHit || !this.selectedHit
        || (this.selectedHit.cycleIndex === undefined && this.selectedHit.cycleCount === undefined)) return false
    const { cycleIndex: _cycleIndex, cycleCount: _cycleCount, ...hit } = this.selectedHit
    this.selectedHit = hit
    return true
  }

  private resetDepthCycle(invalidateSelectedHit = true) {
    if (!this.clearDepthCycleState(invalidateSelectedHit)) return
    this.rebuildSelectionOverlays()
    this.emitSelectionChange()
    this.requestRender()
  }

  resize() {
    if (this.updateSize()) {
      this.resetDepthCycle()
      this.requestRender()
    }
  }

  private updateSize(): boolean {
    const canvas = this.canvas, dev = this.dev
    if (!canvas || !dev || !this.initialized || this.lost) return false

    const cssWidth = Number.isFinite(canvas.clientWidth) ? Math.max(0, canvas.clientWidth) : 0
    const cssHeight = Number.isFinite(canvas.clientHeight) ? Math.max(0, canvas.clientHeight) : 0
    if (cssWidth <= 0 || cssHeight <= 0) {
      const changed = this.drawable || !!this.depth
      this.drawable = false
      this.depth?.destroy()
      this.depth = null
      this.depthView = null
      this.csgDepthStencil?.destroy()
      this.csgDepthStencil = null
      this.csgDepthStencilView = null
      return changed
    }

    const rawDpr = typeof devicePixelRatio === 'number' && Number.isFinite(devicePixelRatio) ? devicePixelRatio : 1
    const dpr = Math.min(MAX_DPR, Math.max(1, rawDpr))
    const maxDimension = Math.max(1, dev.limits.maxTextureDimension2D)
    const scale = Math.min(dpr, maxDimension / cssWidth, maxDimension / cssHeight)
    const width = Math.max(1, Math.min(maxDimension, Math.round(cssWidth * scale)))
    const height = Math.max(1, Math.min(maxDimension, Math.round(cssHeight * scale)))
    const changed = canvas.width !== width || canvas.height !== height || !this.depth

    if (canvas.width !== width) canvas.width = width
    if (canvas.height !== height) canvas.height = height
    if (changed) {
      this.depth?.destroy()
      this.depth = dev.createTexture({
        size: [width, height],
        format: 'depth24plus',
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      })
      this.depthView = this.depth.createView()
      if (this.csgDepthStencil) {
        this.csgDepthStencil.destroy()
        this.csgDepthStencil = null
        this.csgDepthStencilView = null
      }
    }
    this.drawable = true
    return changed
  }

  private getAspect() {
    const canvas = this.canvas
    if (!canvas) return 1
    if (canvas.width > 0 && canvas.height > 0) return canvas.width / canvas.height
    return canvas.clientWidth > 0 && canvas.clientHeight > 0 ? canvas.clientWidth / canvas.clientHeight : 1
  }

  private cameraState() {
    const activeBounds = this.isolated && this.selected !== null
      ? this.meshes[this.selected]?.worldBounds
      : this.bounds
    return computeOrbitCameraFrame({
      yaw: this.yaw,
      pitch: this.pitch,
      distance: this.dist,
      target: [this.tx, this.ty, this.tz],
      aspect: this.getAspect(),
      fovY: this.perspectiveFovY,
      projection: this.projection,
      bounds: activeBounds ?? null,
      backgroundRadius: this.gridVisible ? Math.hypot(this.gridExtent(), this.gridExtent(), this.gridExtent()) : 1,
    })
  }

  private clearDrawCaches() {
    this.opaqueInstances.clear()
    this.transparentInstances.clear()
    this.edgeInstances.clear()
    this.transparentDraws.length = 0
    this.opaqueBundle.clear()
    this.sectionCapBundle.clear()
    this.edgeBundle.clear()
    this.opaqueDraws.length = this.edgeDraws.length = 0
    this.transparentSort.clear()
  }

  private render() {
    this.updateSize()
    const canvas = this.canvas, dev = this.dev, ctx = this.ctx, sceneUB = this.sceneUB
    if (!canvas || !dev || !ctx || !this.depth || !sceneUB || !this.drawable || !canvas.width || !canvas.height) return

    const { eye, viewProjection } = this.cameraState()
    encodeAndSubmitFrame({
      canvas,
      dev,
      ctx,
      fmt: this.fmt,
      depth: this.depth,
      depthView: this.depthView,
      sceneUB,
      sceneBG: this.sceneBG,
      pipelines: this.pipelines,
      textures: this.textures,
      backgroundColor: this.backgroundColor,
      eye,
      viewProjection,
      tx: this.tx,
      ty: this.ty,
      tz: this.tz,
      sectionNormal: this.sectionNormal,
      sectionOffset: this.sectionOffset,
      sectionEnabled: this.sectionEnabled,
      gridVisible: this.gridVisible,
      gridStep: this.gridStep,
      gridExtent: this.gridExtent(),
      gridFadeDistance: this.gridFadeDistance(),
      gridQuadVB: this.gridQuadVB,
      gridVB: this.gridVB,
      gridVC: this.gridVC,
      measurementVB: this.measurementVB,
      measurementVC: this.measurementVC,
      theme: this.theme,
      bounds: this.bounds,
      displayMode: this.displayMode,
      transparencyMode: this.transparencyMode,
      selectionMode: this.selectionMode,
      selected: this.selected,
      selectedHit: this.selectedHit,
      meshes: this.meshes,
      geometryFade: this.geometryFade,
      geometryGhosts: this.geometryGhosts,
      sceneUniformScratch: this.sceneUniformScratch,
      lastSceneUniforms: this.lastSceneUniforms,
      lastSceneUniformsValid: this.lastSceneUniformsValid,
      setLastSceneUniformsValid: valid => { this.lastSceneUniformsValid = valid },
      shadowMapEpoch: this.shadowMapEpoch,
      shadowMapRenderedEpoch: this.shadowMapRenderedEpoch,
      setShadowMapRenderedEpoch: epoch => { this.shadowMapRenderedEpoch = epoch },
      shadowMapVP: this.shadowMapVP,
      shadowMapVPValid: this.shadowMapVPValid,
      setShadowMapVPValid: valid => { this.shadowMapVPValid = valid },
      viewFrustum: this.viewFrustum,
      opaqueDraws: this.opaqueDraws,
      edgeDraws: this.edgeDraws,
      transparentSort: this.transparentSort,
      transparentDraws: this.transparentDraws,
      opaqueInstances: this.opaqueInstances,
      transparentInstances: this.transparentInstances,
      edgeInstances: this.edgeInstances,
      opaqueBundle: this.opaqueBundle,
      sectionCapBundle: this.sectionCapBundle,
      edgeBundle: this.edgeBundle,
      sourceFaceSlot: this.sourceFaceSlot,
      selectionFaceSlot: this.selectionFaceSlot,
      sourceLineSlot: this.sourceLineSlot,
      selectionLineSlot: this.selectionLineSlot,
      deepSelectionLineSlot: this.deepSelectionLineSlot,
      styleScratch: this.styleScratch,
      csgPreviewTree: this.csgPreviewTree,
      ensureCsgDepthStencil: (device, width, height) => {
        this.ensureCsgDepthStencil(device, width, height)
        return this.csgDepthStencilView
      },
      getCsgRenderer: () => {
        if (!this.csgRenderer) {
          this.csgRenderer = new GpuCsgRenderer(this.fmt)
        }
        return this.csgRenderer
      },
      getWboitRenderer: () => this.wboitRenderer,
      isMeshVisible: index => this.isMeshVisible(index),
      usesObjectSelectionStyle: index => this.usesObjectSelectionStyle(index),
      usesObjectHoverStyle: index => this.usesObjectHoverStyle(index),
    })

    const token = this.pendingFrameToken
    this.pendingFrameToken = null
    if (token !== null) {
      try { this.onFrameSubmitted?.(token, performance.now()) } catch { /* Diagnostics cannot break rendering. */ }
    }
  }

  requestRender(resetFrameRetry = true) {
    if (resetFrameRetry) this.frameRetryCount = 0
    if (this.dead || this.lost || !this.initialized || this.raf || typeof requestAnimationFrame !== 'function') return
    this.raf = requestAnimationFrame(this.drawFrame)
  }

  private advanceGeometryAnimation(now: number, finish = false) {
    return advanceSceneGeometryAnimation({
      dev: this.dev,
      meshes: this.meshes,
      geometryFade: this.geometryFade,
      geometryGhosts: this.geometryGhosts,
      morphDummyVB: this.morphDummyVB,
      styleScratch: this.styleScratch,
      objectUniformScratch: this.objectUniformScratch,
      morphScratch: this.morphScratch,
      setGeometryFade: fade => { this.geometryFade = fade },
      setGeometryGhosts: ghosts => { this.geometryGhosts = ghosts },
      invalidateShadowMap: () => this.invalidateShadowMap(),
      updateMeshStyles: () => this.updateMeshStyles(),
      destroyMeshes: meshes => this.destroyMeshes(meshes),
      clearDrawCaches: () => this.clearDrawCaches(),
    }, now, finish)
  }

  private drawFrame = () => {
    this.raf = 0
    if (this.dead || this.lost || !this.initialized) return
    this.notifyCameraChange()
    // WebGPU validation errors (invalid pipeline, missing bind group, ...) are
    // asynchronous: encoding succeeds and the whole frame is silently dropped
    // while the canvas keeps the previous image. A per-frame validation scope
    // turns them into a frame error; only a frame that validated clears one.
    const device = this.dev
    const scoped = typeof device?.pushErrorScope === 'function'
    if (scoped) device!.pushErrorScope('validation')
    const frame = ++this.frameSequence
    let rendered = false
    try {
      const animating = this.advanceGeometryAnimation(performance.now())
      this.render()
      rendered = true
      if (animating) this.requestRender(false)
      this.frameRetryCount = 0
      if (!scoped) this.clearFrameError()
    } catch (error) {
      // A lost/outdated surface can still be retried on the next invalidation,
      // but the failure must remain observable to both UI and developers.
      this.lastFailedFrame = frame
      this.reportError('frame', error)
      if (this.frameRetryCount < 1) {
        this.frameRetryCount++
        this.requestRender(false)
      }
    } finally {
      if (scoped) {
        void device!.popErrorScope().then(gpuError => {
          if (device !== this.dev || this.dead) return
          // No retry: a validation error reproduces on every frame until the
          // state that caused it changes.
          if (gpuError) {
            this.lastFailedFrame = Math.max(this.lastFailedFrame, frame)
            this.reportError('frame', new Error(gpuError.message))
          } else if (rendered && this.lastFailedFrame < frame) {
            this.clearFrameError()
          }
        }, () => { /* Device lost: reported through device.lost. */ })
      }
    }
  }

  private clearFrameError() {
    if (this.status.status === 'error' && this.status.phase === 'frame') {
      this.updateStatus({ status: 'ready' })
    }
  }

  /** Every camera mutation requests a frame, so per-frame diffing observes them all. */
  private notifyCameraChange() {
    if (!this.onCameraChange) return
    const state = this.getCameraState()
    if (this.lastNotifiedCamera && cameraStatesEqual(this.lastNotifiedCamera, state)) return
    this.lastNotifiedCamera = state
    try { this.onCameraChange(state) } catch { /* UI callbacks must not break rendering. */ }
  }

  /** CSS-pixel projection shared by interactive tools drawn over the native viewport. */
  projectWorldPoint(point: readonly number[]): [number, number] | null {
    if(!this.canvas)return null
    return projectPointInKernel(this.cameraState().viewProjection, point,
      [this.canvas.clientWidth, this.canvas.clientHeight])
  }
  worldRay(clientX:number,clientY:number){return this.rayForClientPoint(clientX,clientY)}

  private rayForClientPoint(clientX: number, clientY: number) {
    const canvas = this.canvas
    if (!canvas) return null
    const rect = canvas.getBoundingClientRect()
    return clientRayInKernel(this.cameraState().viewProjection,
      [rect.left, rect.top, rect.width, rect.height], [clientX, clientY])
  }

  private findHitCandidates(
    clientX: number,
    clientY: number,
    maximum = MAX_DEPTH_CANDIDATES,
  ): DepthCandidate<PickHit>[] {
    // Picking uses the authoritative destination BVH. Finish the visual transition
    // before interaction so a transient display shape never supplies CAD identity.
    if (this.geometryFade || this.geometryGhosts.length || this.meshes.some(mesh => mesh.morph)) {
      this.advanceGeometryAnimation(performance.now(), true)
      this.requestRender(false)
    }
    return findHitCandidatesForRay({
      ray: this.rayForClientPoint(clientX, clientY),
      bounds: this.bounds,
      meshes: this.meshes,
      selectionMode: this.selectionMode,
      sectionEnabled: this.sectionEnabled,
      sectionNormal: this.sectionNormal,
      sectionOffset: this.sectionOffset,
      sceneAabbIndex: this.currentSceneAabbIndex(),
      nativePicking: this.nativePicking,
      isMeshVisible: index => this.isMeshVisible(index),
    }, maximum)
  }

  private findHit(clientX: number, clientY: number): PickHit | null {
    return this.findHitCandidates(clientX, clientY, 1)[0]?.value ?? null
  }

  private pickAt(clientX: number, clientY: number) {
    if (this.measureActive) {
      this.resetDepthCycle()
      const hit = this.findHit(clientX, clientY)
      if (hit) this.addMeasurementPoint(hit.point)
      this.setSelection(hit?.meshIndex ?? null, hit)
      return
    }

    const candidates = this.findHitCandidates(clientX, clientY)
    const timestamp = typeof performance === 'undefined' ? Date.now() : performance.now()
    const choice = chooseDepthCandidate(
      candidates,
      clientX,
      clientY,
      this.depthCycleState,
      timestamp,
    )
    this.depthCycleState = choice.state
    const hit = choice.candidate
      ? {
          ...choice.candidate.value,
          cycleIndex: choice.state?.index ?? 0,
          cycleCount: candidates.length,
        }
      : null
    this.setSelection(hit?.meshIndex ?? null, hit)
  }

  private updateHoverAt(clientX: number, clientY: number) {
    // A passing pointer must not prematurely finish parameter animation.
    if (this.geometryFade || this.geometryGhosts.length || this.meshes.some(mesh => mesh.morph)) return
    const epoch = this.getInteractionEpoch()
    if (this.lastHoverEvaluatedX === clientX && this.lastHoverEvaluatedY === clientY && this.lastHoverEpoch === epoch) return
    this.lastHoverEvaluatedX = clientX
    this.lastHoverEvaluatedY = clientY
    this.lastHoverEpoch = epoch
    const hit = this.findHit(clientX, clientY)
    const index = hit?.meshIndex ?? null
    const styleChanged = index !== this.hovered
      || (this.selectionMode !== 'object' && (this.hoveredHit === null) !== (hit === null))
    const overlayChanged = this.hoveredHit === null && hit === null
      ? false
      : !this.sameOverlayTarget(this.hoveredHit, hit)
    this.hovered = index
    this.hoveredHit = hit
    if (styleChanged) this.updateMeshStyles()
    if (overlayChanged) this.rebuildSelectionOverlays()
    if (styleChanged || overlayChanged) {
      // Only notify when the hovered target actually changed (mesh index, face
      // id / snapped point, or a null transition); a resting pointer no longer
      // allocates a fresh hit object per animation frame. Measurement points
      // are captured on click, so no consumer needs continuous hover hits.
      try { this.onHoverChange?.(hit) } catch { /* UI callbacks must not break rendering. */ }
      this.requestRender()
    }
  }

  private cancelPendingHover() {
    this.hoverGeneration++
    if (this.hoverRaf) cancelAnimationFrame(this.hoverRaf)
    this.hoverRaf = 0
  }

  private scheduleHover(clientX: number, clientY: number) {
    this.hoverX = clientX
    this.hoverY = clientY
    if (this.hoverRaf) return
    const generation = this.hoverGeneration
    this.hoverRaf = requestAnimationFrame(() => {
      this.hoverRaf = 0
      if (generation === this.hoverGeneration && !this.dead && !this.drag) {
        this.updateHoverAt(this.hoverX, this.hoverY)
      }
    })
  }

  private clearHover = () => {
    this.cancelPendingHover()
    this.lastHoverEvaluatedX = -1
    this.lastHoverEvaluatedY = -1
    if (this.hovered === null && this.hoveredHit === null) return
    this.hovered = null
    this.hoveredHit = null
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    try { this.onHoverChange?.(null) } catch { /* UI callbacks must not break rendering. */ }
    this.requestRender()
  }

  private addMeasurementPoint(point: Vec3) {
    this.measurementPoints = this.measurementPoints.length >= 2 ? [[...point]] : [...this.measurementPoints, [...point]]
    this.rebuildMeasurementBuffer()
    this.emitMeasurementChange()
    this.requestRender()
  }

  private emitMeasurementChange() {
    const points = this.measurementPoints.map(point => [...point] as Vec3)
    const distance = points.length === 2
      ? Math.hypot(points[1][0] - points[0][0], points[1][1] - points[0][1], points[1][2] - points[0][2])
      : null
    try {
      this.onMeasurementChange?.(points.length ? { points, distance } : null, this.measureActive)
    } catch { /* UI callbacks must not break rendering. */ }
  }

  private rebuildMeasurementBuffer() {
    this.measurementVB?.destroy()
    const { buffer, count } = rebuildMeasurementBufferData(
      this.dev,
      this.measurementPoints,
      this.bounds?.radius,
    )
    this.measurementVB = buffer
    this.measurementVC = count
  }

  private clearSelectionOverlayBuffers() {
    releaseOverlaySlot(this.selectionFaceSlot)
    releaseOverlaySlot(this.selectionLineSlot)
    releaseOverlaySlot(this.deepSelectionLineSlot)
  }

  private clearSourceHighlightOverlayBuffers() {
    releaseOverlaySlot(this.sourceFaceSlot)
    releaseOverlaySlot(this.sourceLineSlot)
  }

  private sameOverlayTarget(a: PickHit | null, b: PickHit | null) {
    if (!a || !b || a.meshIndex !== b.meshIndex) return false
    if (this.selectionMode === 'object') return true
    if (this.selectionMode === 'face') {
      return a.faceId !== null && b.faceId !== null
        ? a.faceId === b.faceId
        : a.triangleIndex === b.triangleIndex
    }
    const ap = pointForOverlay(this.meshes, a)
    const bp = pointForOverlay(this.meshes, b)
    if (!ap || !bp) return false
    const scale = Math.max(1, ...ap.map(Math.abs), ...bp.map(Math.abs))
    return Math.hypot(ap[0] - bp[0], ap[1] - bp[1], ap[2] - bp[2]) <= scale * 1e-7
  }

  private rebuildSelectionOverlays() {
    rebuildSelectionOverlaySlots({
      dev: this.dev,
      selectionMode: this.selectionMode,
      meshes: this.meshes,
      boundsRadius: this.bounds?.radius,
      selectedHit: this.selectedHit,
      hoveredHit: this.hoveredHit,
      sameOverlayTarget: (a, b) => this.sameOverlayTarget(a, b),
      isMeshVisible: index => this.isMeshVisible(index),
      selectionFaceSlot: this.selectionFaceSlot,
      selectionLineSlot: this.selectionLineSlot,
      deepSelectionLineSlot: this.deepSelectionLineSlot,
    })
  }

  private rebuildSourceHighlightOverlay() {
    rebuildSourceHighlightOverlaySlots({
      dev: this.dev,
      sourceHighlightId: this.sourceHighlightId,
      meshes: this.meshes,
      isMeshVisible: index => this.isMeshVisible(index),
      sourceFaceSlot: this.sourceFaceSlot,
      sourceLineSlot: this.sourceLineSlot,
    })
  }

  private onDown = (e: PointerEvent) => {
    const canvas = this.canvas
    if (!canvas) return
    if (this.activePointer !== null) {
      // A second concurrent touch turns the gesture into a two-finger
      // pinch/pan; any further pointers (or non-touch devices) are ignored.
      // BOTH pointers must be touches — a stray touch landing during an
      // active mouse/pen drag must not hijack it into a surprise pinch.
      if (this.pinching || e.pointerId === this.activePointer || e.pointerType !== 'touch' || this.activePointerType !== 'touch') return
      this.pointers.clear()
      this.pointers.set(this.activePointer, { x: this.mx, y: this.my })
      this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY })
      this.pinching = true
      this.gestureMoved = true
      this.lastWheelHistoryAt = -Infinity
      this.resetDepthCycle()
      try { canvas.setPointerCapture(e.pointerId) } catch { /* Pointer may already be gone. */ }
      e.preventDefault()
      return
    }
    if (e.pointerType === 'mouse' && e.button > 2) return
    try { canvas.focus({ preventScroll: true }) } catch { canvas.focus() }
    this.drag = true; this.pan = e.button !== 0 || e.shiftKey
    this.activePointer = e.pointerId
    this.activePointerType = e.pointerType
    this.mx = e.clientX; this.my = e.clientY
    this.downX = e.clientX; this.downY = e.clientY
    this.downButton = e.button
    this.gestureMoved = false
    this.gestureCameraStart = this.getCameraState()
    try { canvas.setPointerCapture(e.pointerId) } catch { /* Pointer may already be gone. */ }
    e.preventDefault()
  }
  private onMove = (e: PointerEvent) => {
    if (this.pinching) {
      const entry = this.pointers.get(e.pointerId)
      if (!entry) return
      let other: { x: number; y: number } | null = null
      for (const [id, point] of this.pointers) {
        if (id !== e.pointerId) { other = point; break }
      }
      if (!other) return
      const prevSelf = { x: entry.x, y: entry.y }
      entry.x = e.clientX
      entry.y = e.clientY
      const next = computePinchUpdate(
        prevSelf, other,
        { x: e.clientX, y: e.clientY }, other,
        { yaw: this.yaw, pitch: this.pitch, dist: this.dist, tx: this.tx, ty: this.ty, tz: this.tz },
        this.canvas?.clientHeight || 1,
        this.perspectiveFovY,
      )
      this.dist = next.dist
      this.tx = next.tx; this.ty = next.ty; this.tz = next.tz
      this.requestRender()
      return
    }
    if (!this.drag) { this.scheduleHover(e.clientX, e.clientY); return }
    if (this.activePointer !== e.pointerId) return
    if (!this.gestureMoved) {
      this.gestureMoved = Math.hypot(e.clientX - this.downX, e.clientY - this.downY) >= CLICK_MOVE_THRESHOLD
      if (!this.gestureMoved) return
      this.lastWheelHistoryAt = -Infinity
      this.resetDepthCycle()
    }
    const dx = e.clientX - this.mx, dy = e.clientY - this.my
    this.mx = e.clientX; this.my = e.clientY
    // A camera move between wheel events starts a new chronological segment;
    // it must not be swallowed by the previous wheel burst's coalescing window.
    if (dx !== 0 || dy !== 0) this.lastWheelHistoryAt = -Infinity
    const camera = { yaw: this.yaw, pitch: this.pitch, dist: this.dist, tx: this.tx, ty: this.ty, tz: this.tz }
    const next = this.pan
      ? computePanUpdate(camera, dx, dy, this.canvas?.clientHeight || 1, this.perspectiveFovY)
      : computeOrbitUpdate(camera, dx, dy)
    this.yaw = next.yaw; this.pitch = next.pitch; this.dist = next.dist
    this.tx = next.tx; this.ty = next.ty; this.tz = next.tz
    this.requestRender()
  }
  private onPointerEnd = (e: PointerEvent) => {
    if (this.pinching && this.pointers.has(e.pointerId)) {
      // Leave pinch mode and resume a clean one-pointer orbit from the
      // remaining finger; re-anchoring to its last position avoids a jump.
      this.pointers.delete(e.pointerId)
      this.pinching = false
      const canvas = this.canvas
      if (e.type !== 'lostpointercapture' && canvas?.hasPointerCapture(e.pointerId)) {
        try { canvas.releasePointerCapture(e.pointerId) } catch { /* Capture can be released asynchronously. */ }
      }
      const remaining = this.pointers.entries().next().value
      if (remaining) {
        const [remainingId, point] = remaining
        this.activePointer = remainingId
        this.mx = point.x
        this.my = point.y
        this.drag = true
        this.pan = false
      } else {
        this.activePointer = null
        this.drag = false
        this.pan = false
      }
      return
    }
    if (this.activePointer !== e.pointerId) return
    const gestureCameraStart = this.gestureCameraStart
    const shouldPick = e.type === 'pointerup'
      && !this.gestureMoved
      && Math.hypot(e.clientX - this.downX, e.clientY - this.downY) < CLICK_MOVE_THRESHOLD
      && !this.pan
      && this.downButton === 0
    this.activePointer = null
    this.pointers.clear()
    this.drag = false
    this.pan = false
    this.gestureCameraStart = null
    if (gestureCameraStart && !cameraStatesEqual(gestureCameraStart, this.getCameraState())) {
      this.recordCameraSnapshot(gestureCameraStart)
    }
    const canvas = this.canvas
    if (e.type !== 'lostpointercapture' && canvas?.hasPointerCapture(e.pointerId)) {
      try { canvas.releasePointerCapture(e.pointerId) } catch { /* Capture can be released asynchronously. */ }
    }
    if (shouldPick) this.pickAt(e.clientX, e.clientY)
  }
  private onWheel = (e: WheelEvent) => {
    e.preventDefault()
    const unit = e.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16
      : e.deltaMode === WheelEvent.DOM_DELTA_PAGE ? Math.max(1, this.canvas?.clientHeight || 1)
      : 1
    const nextDistance = computeWheelDistance(this.dist, e.deltaY * unit)
    if (nextDistance === this.dist) return
    this.flushActiveGestureCameraSnapshot()
    const now = typeof performance === 'undefined' ? Date.now() : performance.now()
    if (now - this.lastWheelHistoryAt > WHEEL_HISTORY_IDLE_MS) {
      this.recordCameraSnapshot(this.getCameraState())
    }
    this.lastWheelHistoryAt = now
    this.resetDepthCycle()
    this.dist = nextDistance
    if (this.gestureCameraStart) this.gestureCameraStart = this.getCameraState()
    this.requestRender()
  }
  private noCtx = (e: Event) => e.preventDefault()
  private onWindowResize = () => this.resize()

  private clampDistance(distance: number) {
    return clampDistanceValue(distance)
  }

  private bindInput() {
    const c = this.canvas
    if (!c || this.inputBound) return
    this.inputBound = true
    this.previousTouchAction = c.style.touchAction
    c.style.touchAction = 'none'
    c.addEventListener('pointerdown', this.onDown)
    c.addEventListener('pointermove', this.onMove)
    c.addEventListener('pointerup', this.onPointerEnd)
    c.addEventListener('pointercancel', this.onPointerEnd)
    c.addEventListener('lostpointercapture', this.onPointerEnd)
    c.addEventListener('pointerleave', this.clearHover)
    c.addEventListener('wheel', this.onWheel, { passive: false })
    c.addEventListener('contextmenu', this.noCtx)
    if (typeof window !== 'undefined') window.addEventListener('resize', this.onWindowResize)
  }

  private observeResize() {
    if (!this.canvas || typeof ResizeObserver === 'undefined') return
    this.resizeObserver = new ResizeObserver(() => this.resize())
    this.resizeObserver.observe(this.canvas)
  }

  private destroyMeshes(meshes: GMesh[], preserved: ReadonlySet<GPUBuffer> = new Set()) {
    destroySceneMeshes(meshes, this.nativePicking, preserved)
  }

  destroy() {
    this.pendingFrameToken = null
    this.teardown()
    this.updateStatus({ status: 'destroyed' })
  }

  private teardown() {
    ++this.generation
    this.dead = true
    this.initialized = false
    if (this.raf) cancelAnimationFrame(this.raf)
    this.raf = 0
    this.frameRetryCount = 0
    this.cancelPendingHover()
    this.cancelEdgeBufferWarmup()
    this.resizeObserver?.disconnect()
    this.resizeObserver = null

    const c = this.canvas
    if (c && this.inputBound) {
      c.removeEventListener('pointerdown', this.onDown)
      c.removeEventListener('pointermove', this.onMove)
      c.removeEventListener('pointerup', this.onPointerEnd)
      c.removeEventListener('pointercancel', this.onPointerEnd)
      c.removeEventListener('lostpointercapture', this.onPointerEnd)
      c.removeEventListener('pointerleave', this.clearHover)
      c.removeEventListener('wheel', this.onWheel)
      c.removeEventListener('contextmenu', this.noCtx)
      if (c.style.touchAction === 'none') c.style.touchAction = this.previousTouchAction
    }
    if (typeof window !== 'undefined') window.removeEventListener('resize', this.onWindowResize)
    this.inputBound = false
    this.activePointer = null
    this.pointers.clear()
    this.pinching = false
    this.drag = false
    this.pan = false
    this.gestureMoved = false
    this.gestureCameraStart = null
    this.lastWheelHistoryAt = -Infinity
    this.resetDepthCycle(false)
    const hadCameraHistory = this.canGoToPreviousView
    this.cameraHistory.clear()

    this.clearDrawCaches()
    for (const ghost of this.geometryGhosts) this.destroyMeshes(ghost.meshes)
    this.geometryGhosts = []
    this.geometryFade = null
    this.destroyMeshes(this.meshes)
    this.nativePicking.clear()
    this.edgeBuffersByVertexBuffer.clear()
    this.meshes = []
    disposeSceneAabbIndex(this.sceneAabbIndex)
    this.sceneAabbIndex = buildSceneAabbIndex([])
    this.sceneAabbIndexDirty = false
    this.gridVB?.destroy()
    this.gridVB = null
    this.gridQuadVB?.destroy()
    this.gridQuadVB = null
    this.measurementVB?.destroy()
    this.measurementVB = null
    this.measurementVC = 0
    this.clearSelectionOverlayBuffers()
    this.sourceHighlightId = null
    this.clearSourceHighlightOverlayBuffers()
    this.depth?.destroy()
    this.depth = null
    this.depthView = null
    this.csgRenderer?.destroy()
    this.csgRenderer = null
    this.csgDepthStencil?.destroy()
    this.csgDepthStencil = null
    this.csgDepthStencilView = null
    this.csgPreviewTree = null
    this.gpuPicker?.destroy()
    this.gpuPicker = null
    this.wboit?.destroy()
    this.wboit = null
    this.gpuBufferArena?.destroy()
    this.gpuBufferArena = null
    this.sceneUB?.destroy()
    this.sceneUB = null
    this.lastSceneUniformsValid = false
    this.morphDummyVB?.destroy()
    this.morphDummyVB = null
    this.textures.teardown()
    this.resetShadowMapCache()
    try { this.ctx?.unconfigure() } catch { /* Context may already be lost. */ }
    const device = this.dev
    this.dev = null
    this.ctx = null
    this.canvas = null
    this.bounds = null
    const selectionChanged = this.selected !== null || this.isolated
    this.selected = null
    this.selectedHit = null
    this.hovered = null
    this.hoveredHit = null
    this.isolated = false
    this.measurementPoints = []
    this.measureActive = false
    this.initialFitDone = false
    this.drawable = false
    this.lost = false
    try { device?.destroy() } catch { /* Repeated/lost-device cleanup is harmless. */ }
    if (selectionChanged) this.emitSelectionChange()
    if (hadCameraHistory) this.emitCameraHistoryChange()
  }

  get currentStatus(): RendererLifecycleEvent { return this.status }

  getFrameDiagnostics(): WebGPUFrameDiagnostics {
    return {
      opaqueMeshCount: this.opaqueDraws.length,
      transparentMeshCount: this.transparentDraws.length,
      edgeMeshCount: this.edgeDraws.length,
      totalMeshCount: this.meshes.length,
      deviceLost: this.lost,
    }
  }

  private rebuildSceneAabbIndex() {
    const entries: { id: number; bounds: Aabb3 }[] = []
    for (let index = 0; index < this.meshes.length; index++) {
      const mesh = this.meshes[index]!
      if (mesh.visible) {
        entries.push({ id: index, bounds: { min: mesh.worldBounds.min, max: mesh.worldBounds.max } })
      }
    }
    const replacement = buildSceneAabbIndex(entries)
    disposeSceneAabbIndex(this.sceneAabbIndex)
    this.sceneAabbIndex = replacement
    this.sceneAabbIndexDirty = false
  }

  private currentSceneAabbIndex() {
    if (this.sceneAabbIndexDirty) this.rebuildSceneAabbIndex()
    return this.sceneAabbIndex
  }

  private updateStatus(event: RendererLifecycleEvent) {
    this.status = event
    try { this.onStatusChange?.(event) } catch { /* UI callbacks must not break rendering. */ }
  }

  private reportError(phase: 'initialization' | 'frame', caught: unknown) {
    const error = caught instanceof Error ? caught : new Error(String(caught))
    this.updateStatus({ status: 'error', phase, error })
    if (phase === 'frame' && typeof console !== 'undefined') {
      console.error('[WebGPURenderer] frame failed', error)
    }
  }
}
