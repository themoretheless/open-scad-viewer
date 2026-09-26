import { geometryTransformTransition } from './geometryTransformTransition'
import {remapNativeFaceSelection} from './nativeFaceSelection'
/**
 * WebGPU 3D renderer — Phong shading, orbit camera, grid floor, axis gizmo.
 */
import {
  invert,
  type Aabb3, type Mat4, type Vec3,
} from './math3d'
import {
  CameraHistory,
  cameraStatesEqual,
  type CameraHistorySnapshot,
  type CameraState,
} from './cameraHistory'
import type { MeshBvh, MeshBvhHit } from './meshBvh'
import { NativePickingCache } from './nativePickingCache'
import { warmGeometryKernel } from './geometry/kernel'
import { clientRayInKernel, projectPointInKernel, selectedCornerInKernel } from './geometry/viewport'
import {
  buildFaceOverlayGeometry,
  buildFaceTriangleIndex,
  buildSourceOverlayGeometry,
  MAX_SOURCE_OVERLAY_TRIANGLES,
  pointOverlayPosition,
  type FaceTriangleIndex,
} from './meshSelectionOverlay'
import type { MeshData, MeshProvenanceRun, MeshSourceReference } from '../core/mesh'
import type { GeometryAssetId } from '../core/scene'
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
  MaterialDef,
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
import { resolveMeshShaderId, DEFAULT_THEME, getThemePreset, type RenderTheme } from './rendererContracts'
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
  normalizeDepthCandidates,
  type DepthCandidate,
  type DepthCycleState,
} from './selectionCycling'
import {
  buildSceneAabbIndex,
  disposeSceneAabbIndex,
  querySceneAabbIndex,
  type SceneAabbIndex,
} from './sceneAabbIndex'
import { TransparentSortBuffer } from './transparentOrdering'
import { MeshDrawBundle } from './meshDrawBundle'
import { ViewFrustum } from './viewFrustum'
import { MeshInstances } from './meshInstances'
import { effectiveDisplayAlpha, isTransparentAlpha } from './backendQuality'
import { computeOrbitCameraFrame } from './orbitCameraProjection'
import {
  MESH_VERTEX_STRIDE,
  MORPH_VERTEX_STRIDE,
  OBJECT_UNIFORM_LAYOUT,
  SCENE_UNIFORM_LAYOUT,
  getShader,
  immediateObjectShader,
  instancedObjectShader,
  supportsImmediateAddressSpace,
} from './shaders'
import type { ShaderDepthSpec, ShaderVariant, ShaderVertexLayout } from './shaders'

/* ── GPU mesh handle ──────────────────────────────── */

interface GMesh {
  entityId?: MeshData['entityId']
  /**
   * Vertex morph state, blended on the GPU: the vertex shader mixes the
   * position attribute toward `from` (vertex slot 1) by `ob.morph.x`, so the
   * CPU never rewrites vertex buffers per frame.
   */
  morph?: { from: Float32Array; target: Float32Array; started: number; matrix?: (t: number) => Mat4; currentMatrix?: Mat4 }
  /** GPU copy of `morph.from` (positions only, stride 3); capacity-grown. */
  morphVB: GPUBuffer | null
  morphVBCapacity: number
  /** Buffer bound at vertex slot 1: morph source while morphing, else the shared dummy. */
  morphSlot: GPUBuffer | null
  nativeGeometry?: MeshData['nativeGeometry']
  faceIdsAuthoritative?: boolean
  assetId?: GeometryAssetId
  vb: GPUBuffer; ib: GPUBuffer; ic: number
  ub: GPUBuffer; bg: GPUBindGroup
  edgeIB: GPUBuffer | null; edgeIC: number
  vertices: Float32Array; indices: Uint32Array
  edgeIndices: Uint32Array; faceIds: Uint32Array; bvh: MeshBvh
  provenance: MeshProvenanceRun[]
  transform: Mat4; inverseTransform: Mat4
  color: [number, number, number, number]
  alpha: number
  localBounds: Aabb3
  worldBounds: Bounds
  visible: boolean
  /** Lazily built faceId → triangles index; undefined = not built yet, null = unavailable. */
  faceIndex?: FaceTriangleIndex | null
  /** Last style vector written to the uniform buffer (alpha, selected, edge, hovered). */
  styleAlpha: number
  styleSelected: number
  styleEdge: number
  styleHovered: number
  /** Presentation material from MeshData; undefined = legacy Phong defaults. */
  material?: MaterialDef
  /** Resolved shading model; drives mesh pipeline selection at draw time. */
  shadingModel: ShadingModel
}

/** Shared empty list so render() never allocates when there are no ghosts. */
const NO_GHOST_MESHES: GMesh[] = []

/** Persistent capacity-grown vertex buffer for transient overlay geometry. */
interface OverlaySlot {
  buffer: GPUBuffer | null
  /** Allocated size in bytes; may exceed the currently written data. */
  capacity: number
  /** Vertices to draw this frame (stride 28 bytes). */
  count: number
}

interface Bounds {
  center: [number, number, number]
  radius: number
  min: [number, number, number]
  max: [number, number, number]
}

/**
 * Buffer equality for scene publication. Buffers handed to setMeshes are
 * immutable snapshots: the reference fast path below already treats identity
 * as equality, and nothing in the renderer mutates a published buffer.
 *
 * Cost model: a full elementwise scan over up to ~1M floats ran on every
 * rebuild, even for a transform-only edit. Two cheap gates now avoid it:
 *  1. Sampled-lane fingerprint (first/last plus 62 evenly spaced lanes) —
 *     a mismatch proves inequality in O(64), so changed buffers are rejected
 *     without scanning to the first differing element.
 *  2. A verified-pair cache: once a full scan has proven two distinct buffer
 *     instances equal, later comparisons of the same pair are O(1). This is
 *     what makes repeated transform-only rebuilds cheap when the publisher
 *     hands over fresh instances with identical content. Immutability of
 *     published snapshots keeps cached verdicts valid.
 * A fingerprint match without a cached verdict still falls through to the
 * full scan, so a false "changed" answer is possible only as a cheap early
 * rejection; a false "unchanged" answer is impossible.
 */
const verifiedEqualPairs = new WeakMap<object, WeakSet<object>>()

function sampledLanesMatch(left: Float32Array | Uint32Array, right: Float32Array | Uint32Array) {
  const length = left.length
  if (left[0] !== right[0] || left[length - 1] !== right[length - 1]) return false
  const lanes = Math.min(62, length)
  const step = length / lanes
  for (let lane = 0; lane < lanes; lane++) {
    const index = Math.floor(lane * step)
    if (left[index] !== right[index]) return false
  }
  return true
}

function sameTypedArray(left: Float32Array | Uint32Array, right: Float32Array | Uint32Array) {
  if (left === right) return true
  if (left.constructor !== right.constructor || left.length !== right.length) return false
  if (left.length === 0) return true
  if (!sampledLanesMatch(left, right)) return false
  if (verifiedEqualPairs.get(left)?.has(right)) return true
  for (let index = 0; index < left.length; index++) if (left[index] !== right[index]) return false
  let verified = verifiedEqualPairs.get(left)
  if (!verified) verifiedEqualPairs.set(left, verified = new WeakSet())
  verified.add(right)
  return true
}

/** Positions-only (stride 3) copy of an interleaved position+normal vertex array. */
function extractPositions(vertices: Float32Array): Float32Array {
  const positions = new Float32Array(vertices.length / 2)
  for (let source = 0, target = 0; source < vertices.length; source += 6, target += 3) {
    positions[target] = vertices[source]
    positions[target + 1] = vertices[source + 1]
    positions[target + 2] = vertices[source + 2]
  }
  return positions
}

/** Local positions a morph displays at `now`, for chaining a new morph mid-flight. */
function blendMorphPositions(morph: { from: Float32Array; target: Float32Array; started: number }, now: number): Float32Array {
  const t = Math.min(1, Math.max(0, (now - morph.started) / 180))
  const eased = t * t * (3 - 2 * t)
  const target = morph.target
  const out = new Float32Array(morph.from.length)
  for (let index = 0, vertex = 0; index < out.length; index += 3, vertex += 6) {
    out[index] = morph.from[index] + (target[vertex] - morph.from[index]) * eased
    out[index + 1] = morph.from[index + 1] + (target[vertex + 1] - morph.from[index + 1]) * eased
    out[index + 2] = morph.from[index + 2] + (target[vertex + 2] - morph.from[index + 2]) * eased
  }
  return out
}


const FOV_Y = Math.PI / 4
const DEFAULT_DISTANCE = 50
const MAX_ORBIT_PITCH = Math.PI / 2 - 0.001
const MAX_DPR = 2
const DEFAULT_GRID_STEP = 10
/** Minimum plane half-extent and Z-axis length in model units. */
const GRID_MIN_EXTENT = 200
/** Grid lines fade out this many orbit distances away from the eye. */
const GRID_FADE_DISTANCES = 6
/** Premultiplied-alpha blend shared by every transparent pipeline target. */
const ALPHA_BLEND: GPUBlendState = {
  color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
  alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
}
const CLICK_MOVE_THRESHOLD = 3
const MAX_EDGE_BUFFER_BYTES = 32 * 1024 * 1024
const MAX_OVERLAY_BUFFER_BYTES = 16 * 1024 * 1024
const MAX_DEPTH_CANDIDATES = 32
const MAX_DEPTH_CONTINUATIONS = 256
const WHEEL_HISTORY_IDLE_MS = 250

function clampDistanceValue(distance: number) {
  return clampGestureDistance(distance)
}


/* ── Renderer class ───────────────────────────────── */

export class WebGPURenderer {
  private canvas: HTMLCanvasElement | null = null
  private dev: GPUDevice | null = null
  private ctx: GPUCanvasContext | null = null
  private fmt: GPUTextureFormat = 'bgra8unorm'
  private backgroundColor: [number, number, number] = [0.09, 0.09, 0.11]

  private meshPipe!: GPURenderPipeline
  private meshPipeT!: GPURenderPipeline
  private meshImmediatePipeT: GPURenderPipeline | null = null
  private deepMeshPipe!: GPURenderPipeline
  private deepMeshImmediatePipe: GPURenderPipeline | null = null
  private linePipe!: GPURenderPipeline
  private gridPipe!: GPURenderPipeline
  private edgePipe!: GPURenderPipeline
  private deepEdgePipe!: GPURenderPipeline
  private deepEdgeImmediatePipe: GPURenderPipeline | null = null
  private selectionFacePipe!: GPURenderPipeline
  private selectionLinePipe!: GPURenderPipeline
  private deepSelectionLinePipe!: GPURenderPipeline
  private sceneBGL!: GPUBindGroupLayout
  private objBGL!: GPUBindGroupLayout
  private sceneLayout!: GPUPipelineLayout
  private objectLayout!: GPUPipelineLayout
  private immediateObjectLayout: GPUPipelineLayout | null = null
  private vertexLayouts!: Record<ShaderVertexLayout, GPUVertexBufferLayout[]>
  private readonly shaderModuleCache = new Map<string, GPUShaderModule>()
  private readonly pipelineCache = new Map<string, GPURenderPipeline>()
  private sceneUB: GPUBuffer | null = null
  private sceneBG!: GPUBindGroup
  private depth: GPUTexture | null = null
  private depthView: GPUTextureView | null = null

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
  /** Active render theme; fills the Scene theme tail every frame. */
  private theme: RenderTheme = DEFAULT_THEME
  /** Scene-level default shading model for meshes without their own material. */
  private defaultShadingModel: ShadingModel = 'phong'
  /**
   * Scene-level default material tail for meshes without their own material;
   * identity defaults reproduce the legacy look.
   */
  private defaultMaterial = { baseColor: [1, 1, 1] as [number, number, number], metallic: 0, roughness: 0.7 }
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
  private instanceBGL!: GPUBindGroupLayout
  private instanceLayout!: GPUPipelineLayout
  private instanceMeshPipe!: GPURenderPipeline
  private instanceMeshPipeT!: GPURenderPipeline
  private instanceEdgePipe!: GPURenderPipeline
  private readonly opaqueBundle = new MeshDrawBundle()
  private readonly edgeBundle = new MeshDrawBundle()
  private edgeBuffersByVertexBuffer = new Map<GPUBuffer, GMesh[]>()
  private edgeWarmQueue: GMesh[] = []
  private edgeWarmHandle: number | ReturnType<typeof setTimeout> | null = null
  private edgeWarmIsIdle = false
  private immediateObjectStyle = false

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
    this.immediateObjectStyle = supportsImmediateAddressSpace()
    this.sceneBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })
    this.objBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })
    this.instanceBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } },
    ] })
    this.sceneLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL] })
    this.objectLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL] })
    this.immediateObjectLayout = this.immediateObjectStyle
      ? dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL], immediateSize: 16 })
      : null
    this.instanceLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.instanceBGL] })
    this.morphDummyVB?.destroy()
    this.morphDummyVB = dev.createBuffer({ size: 12, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })

    const meshVBL: GPUVertexBufferLayout = {
      arrayStride: MESH_VERTEX_STRIDE,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x3' },
      ],
    }
    // Slot 1 carries the morph source positions (stride 3); a shared zero
    // buffer is bound while a mesh is not morphing.
    const morphVBL: GPUVertexBufferLayout = {
      arrayStride: MORPH_VERTEX_STRIDE,
      attributes: [{ shaderLocation: 2, offset: 0, format: 'float32x3' }],
    }
    const lineVBL: GPUVertexBufferLayout = {
      arrayStride: 28,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x4' },
      ],
    }
    this.vertexLayouts = {
      mesh: [meshVBL, morphVBL],
      edge: [
        { arrayStride: MESH_VERTEX_STRIDE, attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x3' }] },
        { arrayStride: MORPH_VERTEX_STRIDE, attributes: [{ shaderLocation: 1, offset: 0, format: 'float32x3' }] },
      ],
      line: [lineVBL],
      grid: [{
        arrayStride: 8,
        attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x2' }],
      }],
    }
    this.shaderModuleCache.clear()
    this.pipelineCache.clear()

    const transparentDepth: ShaderDepthSpec = { writeEnabled: false, compare: 'less' }
    this.meshPipe = this.getRenderPipeline('mesh')
    this.meshPipeT = this.getRenderPipeline('mesh', { blend: 'alpha', depth: transparentDepth })
    this.meshImmediatePipeT = this.immediateObjectLayout
      ? this.getRenderPipeline('mesh', { variant: 'immediate', blend: 'alpha', depth: transparentDepth })
      : null
    this.deepMeshPipe = this.getRenderPipeline('deepMesh')
    this.deepMeshImmediatePipe = this.immediateObjectLayout
      ? this.getRenderPipeline('deepMesh', { variant: 'immediate' })
      : null
    this.linePipe = this.getRenderPipeline('line')
    this.gridPipe = this.getRenderPipeline('grid')
    this.edgePipe = this.getRenderPipeline('edge')
    this.instanceMeshPipe = this.getRenderPipeline('mesh', { variant: 'instanced' })
    this.instanceMeshPipeT = this.getRenderPipeline('mesh', { variant: 'instanced', blend: 'alpha', depth: transparentDepth })
    this.instanceEdgePipe = this.getRenderPipeline('edge', { variant: 'instanced' })
    this.deepEdgePipe = this.getRenderPipeline('edge', { depth: { writeEnabled: false, compare: 'always' } })
    this.deepEdgeImmediatePipe = this.immediateObjectLayout
      ? this.getRenderPipeline('edge', { variant: 'immediate', depth: { writeEnabled: false, compare: 'always' } })
      : null
    this.selectionFacePipe = this.getRenderPipeline('selectionOverlay')
    this.selectionLinePipe = this.getRenderPipeline('selectionOverlay', { topology: 'line-list' })
    this.deepSelectionLinePipe = this.getRenderPipeline('selectionOverlay', { topology: 'line-list', depth: { writeEnabled: false, compare: 'always' } })
  }

  /** Cached shader modules, one per (shader id, variant). */
  private shaderModule(id: string, variant: ShaderVariant): GPUShaderModule {
    const key = `${id}|${variant}`
    let module = this.shaderModuleCache.get(key)
    if (!module) {
      const spec = getShader(id)
      const source = variant === 'immediate' ? immediateObjectShader(spec.source)
        : variant === 'instanced'
          ? instancedObjectShader(spec.source, spec.vertexLayout === 'edge' ? 'EdgeV' : 'V')
          : spec.source
      module = this.dev!.createShaderModule({ code: source })
      this.shaderModuleCache.set(key, module)
    }
    return module
  }

  /**
   * Cached render-pipeline factory driven by the shader registry. The
   * registry entry supplies the default blend/depth/topology/vertex layout;
   * `flavor` overrides them for the transparent/deep/line flavors of a
   * shader. Descriptors are identical to the hand-wired pipelines this
   * replaces.
   */
  private getRenderPipeline(id: string, flavor: {
    variant?: ShaderVariant
    blend?: 'none' | 'alpha'
    depth?: ShaderDepthSpec
    topology?: 'triangle-list' | 'line-list'
  } = {}): GPURenderPipeline {
    const spec = getShader(id)
    const variant = flavor.variant ?? 'uniform'
    if (variant !== 'uniform' && !spec.supportsVariants) {
      throw new Error(`Shader '${id}' does not support the '${variant}' variant`)
    }
    const blend = flavor.blend ?? spec.blend
    const depth = flavor.depth ?? spec.depth
    const topology = flavor.topology ?? spec.topology
    const key = `${id}|${variant}|${blend}|${depth.writeEnabled ? 1 : 0}:${depth.compare}|${topology}`
    const cached = this.pipelineCache.get(key)
    if (cached) return cached
    const module = this.shaderModule(id, variant)
    const layout = spec.kind === 'object'
      ? variant === 'instanced' ? this.instanceLayout
        : variant === 'immediate' ? this.immediateObjectLayout!
        : this.objectLayout
      : this.sceneLayout
    const pipeline = this.dev!.createRenderPipeline({
      layout,
      vertex: { module, entryPoint: 'vs', buffers: this.vertexLayouts[spec.vertexLayout] },
      fragment: { module, entryPoint: 'fs', targets: [{
        format: this.fmt,
        ...(blend === 'alpha' ? { blend: ALPHA_BLEND } : {}),
      }] },
      primitive: { topology, cullMode: spec.cullMode },
      depthStencil: { format: 'depth24plus', depthWriteEnabled: depth.writeEnabled, depthCompare: depth.compare },
    })
    this.pipelineCache.set(key, pipeline)
    return pipeline
  }

  private buildSceneUB() {
    const dev = this.dev!
    this.sceneUB = dev.createBuffer({ size: SCENE_UNIFORM_LAYOUT.bytes, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.sceneBG = dev.createBindGroup({
      layout: this.sceneBGL,
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

  setMeshes(meshes: MeshData[], options: SetMeshesOptions = {}) {
    this.cancelPendingHover()
    const dev = this.dev
    if (!dev || !this.initialized || this.dead || this.lost) return

    const metrics: SceneUploadMetrics = { geometryUploadBytes: 0, geometryBuffersCreated: 0, reusedEntities: 0 }
    const previous = this.meshes
    const previousSet = new Set(previous)
    const animate = (options.animate || this.geometryFade !== null || this.meshes.some(mesh => mesh.morph)) && !(typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches)

    const previousById = new Map(previous.filter(mesh => mesh.entityId).map(mesh => [mesh.entityId, mesh]))
    const started = performance.now()
    const reusableByAsset = new Map<GeometryAssetId, GMesh[]>()
    for (const mesh of previous) {
      if (!mesh.assetId) continue
      const candidates = reusableByAsset.get(mesh.assetId) ?? []
      candidates.push(mesh)
      reusableByAsset.set(mesh.assetId, candidates)
    }
    const retainedGeometryBuffers = new Set<GPUBuffer>()
    const next: GMesh[] = []
    let nextSceneIndex: SceneAabbIndex
    try {
      for (const m of meshes) {
        if (!m.indices.length && !m.vertices.length) continue
        if (m.vertices.length < 6 || m.vertices.length % 6 !== 0 || !m.indices.length) {
          throw new Error('Invalid mesh buffer layout')
        }

        const measured = this.measureMeshBounds(m.geometryAssetId ?? null, m.vertices, m.transform)
        if (!measured) throw new Error('Mesh contains no finite positions')
        const transform = new Float32Array(m.transform)
        const inverseTransform = invert(transform)

        const old = m.entityId ? previousById.get(m.entityId) : undefined
        // Equal index counts do not establish vertex correspondence after CSG.
        // Animate only rigid transforms of unchanged geometry, per entity.
        const matrix = animate && old && sameTypedArray(old.vertices, m.vertices)
          && sameTypedArray(old.indices, m.indices) ? geometryTransformTransition(old.morph?.currentMatrix ?? old.transform, transform) : null
        const morphFrom = animate && old && matrix
          && (!sameTypedArray(old.morph?.target ?? old.vertices, m.vertices) || !sameTypedArray(old.morph?.currentMatrix ?? old.transform, transform))
          ? (old.morph ? blendMorphPositions(old.morph, started) : extractPositions(old.vertices))
          : null
        const retainedMorph = matrix && old?.morph && sameTypedArray(old.transform, transform) ? old.morph : null
        // Animated geometry must own its buffer: instances may have different start shapes.
        const candidates = !animate && m.geometryAssetId ? reusableByAsset.get(m.geometryAssetId) : undefined
        // Prefer the already verified views staged earlier in this publication.
        // A newly built shared asset needs one content comparison, not one per instance.
        const reusable = candidates?.find(candidate => candidate.vertices === m.vertices && candidate.indices === m.indices)
          ?? candidates?.find(candidate => (
              sameTypedArray(candidate.vertices, m.vertices)
              && sameTypedArray(candidate.indices, m.indices)
            ))
        let vb: GPUBuffer | null = reusable?.vb ?? null
        let ib: GPUBuffer | null = reusable?.ib ?? null
        let ub: GPUBuffer | null = null
        // The morph source buffer is capacity-grown and inherited across the
        // publications of one entity, so repeated edits do not realloc it.
        let morphVB = old?.morphVB ?? null
        let morphVBCapacity = old?.morphVBCapacity ?? 0
        const ownsGeometryBuffers = !reusable
        try {
          if (!vb) vb = dev.createBuffer({ size: m.vertices.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
          if (!ib) ib = dev.createBuffer({ size: m.indices.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
          ub = dev.createBuffer({ size: OBJECT_UNIFORM_LAYOUT.bytes, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
          const initialAlpha = effectiveDisplayAlpha(m.color[3], this.displayMode)
          const initialEdge = this.displayMode === 'edges' ? 0.7 : 0
          if (ownsGeometryBuffers) {
            metrics.geometryBuffersCreated += 2
            metrics.geometryUploadBytes += m.vertices.byteLength + m.indices.byteLength
            // The vertex buffer always holds the destination geometry; the
            // shader blends the morph source from slot 1 toward it.
            dev.queue.writeBuffer(vb, 0, m.vertices)
            dev.queue.writeBuffer(ib, 0, m.indices)
          }
          if (!retainedMorph && morphFrom) {
            if (!morphVB || morphVBCapacity < morphFrom.byteLength) {
              morphVB = dev.createBuffer({ size: morphFrom.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
              morphVBCapacity = morphFrom.byteLength
            }
            dev.queue.writeBuffer(morphVB, 0, morphFrom)
          }
          // One contiguous upload per entity. writeBuffer snapshots the data,
          // so the same bounded scratch storage can serve the next entity.
          const uniform = this.objectUniformScratch
          for (let row = 0; row < 4; row++) {
            for (let column = 0; column < 4; column++) uniform[column * 4 + row] = transform[row * 4 + column]
          }
          // Column-major bytes for inverse-transpose(row-major model).
          uniform.set(inverseTransform, 16)
          uniform.set(m.color, 32)
          uniform[36] = initialAlpha; uniform[37] = 0
          uniform[38] = initialEdge; uniform[39] = 0
          // morph.x drives the GPU vertex blend; rest is 1 (target vertices),
          // which keeps the zero slot-1 dummy inert: mix(dummy, pos, 1) = pos.
          uniform[40] = 1; uniform[41] = 0; uniform[42] = 0; uniform[43] = 0
          // Material tail: identity defaults (white base color, non-metal, no
          // emission, roughness 0.7) reproduce the legacy shading look; a
          // provided MaterialDef overrides them, and the scene-level default
          // material applies to meshes without one.
          const material = m.material
          if (material) {
            uniform[44] = material.baseColor[0]; uniform[45] = material.baseColor[1]; uniform[46] = material.baseColor[2]
            uniform[47] = material.metallic
            uniform[48] = material.emissive[0]; uniform[49] = material.emissive[1]; uniform[50] = material.emissive[2]
            uniform[51] = material.roughness
            uniform[52] = 0; uniform[53] = 0; uniform[54] = 0; uniform[55] = 0
          } else {
            const dm = this.defaultMaterial
            uniform[44] = dm.baseColor[0]; uniform[45] = dm.baseColor[1]; uniform[46] = dm.baseColor[2]
            uniform[47] = dm.metallic
            uniform[48] = 0; uniform[49] = 0; uniform[50] = 0; uniform[51] = dm.roughness
            uniform[52] = 0; uniform[53] = 0; uniform[54] = 0; uniform[55] = 0
          }
          dev.queue.writeBuffer(ub, 0, uniform)
          const bg = dev.createBindGroup({
            layout: this.objBGL,
            entries: [{ binding: 0, resource: { buffer: ub } }],
          })
          if (reusable) metrics.reusedEntities++
          const reuseEdges = reusable && sameTypedArray(reusable.edgeIndices, m.edgeIndices)
          const morph = retainedMorph ?? (morphFrom ? { from: morphFrom, target: m.vertices, started, matrix: matrix ?? undefined, currentMatrix: matrix?.(0) } : undefined)
          next.push({
            entityId: m.entityId,
            morph,
            morphVB, morphVBCapacity,
            morphSlot: morph ? morphVB : this.morphDummyVB,
            assetId: animate ? undefined : m.geometryAssetId,
            nativeGeometry:m.nativeGeometry,faceIdsAuthoritative:m.faceIdsAuthoritative,
            vb, ib, ic: m.indices.length, ub, bg,
            edgeIB: reuseEdges ? reusable.edgeIB : null,
            edgeIC: reuseEdges ? reusable.edgeIC : 0,
            vertices: m.vertices, indices: m.indices,
            edgeIndices: m.edgeIndices, faceIds: m.faceIds, bvh: m.bvh,
            provenance: m.provenance,
            transform, inverseTransform,
            color: [...m.color],
            alpha: initialAlpha,
            localBounds: measured.local,
            worldBounds: measured.world,
            visible: true,
            styleAlpha: initialAlpha,
            styleSelected: 0,
            styleEdge: initialEdge,
            styleHovered: 0,
            material,
            shadingModel: material?.shadingModel ?? this.defaultShadingModel,
          })
          const added = next[next.length - 1]
          if (added.assetId) {
            const candidates = reusableByAsset.get(added.assetId) ?? []
            candidates.push(added)
            reusableByAsset.set(added.assetId, candidates)
          }
          if (reusable && previousSet.has(reusable)) {
            retainedGeometryBuffers.add(reusable.vb)
            retainedGeometryBuffers.add(reusable.ib)
            if (reusable.edgeIB && added.edgeIB === reusable.edgeIB) retainedGeometryBuffers.add(reusable.edgeIB)
          }
          if (old && previousSet.has(old) && old.morphVB && morphVB === old.morphVB) {
            // The new mesh took over the morph source buffer.
            retainedGeometryBuffers.add(old.morphVB)
          }
        } catch (error) {
          if (ownsGeometryBuffers) { vb?.destroy(); ib?.destroy() }
          // A morph buffer created for the failed staged mesh must not leak;
          // an inherited one still belongs to the live previous mesh.
          if (morphVB && morphVB !== old?.morphVB) morphVB.destroy()
          ub?.destroy()
          throw error
        }
      }
      nextSceneIndex = buildSceneAabbIndex(next.map((mesh, id) => ({
        id, bounds: { min: mesh.worldBounds.min, max: mesh.worldBounds.max },
      })))
    } catch (error) {
      this.destroyMeshes(next, retainedGeometryBuffers)
      throw error
    }

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
   * Scene-level default material tail (baseColor/metallic/roughness) for
   * meshes without their own material; rewrites their uniform tails in place.
   */
  setDefaultMaterial(defaults: { baseColor?: readonly [number, number, number]; metallic?: number; roughness?: number }) {
    const current = this.defaultMaterial
    if (defaults.baseColor) current.baseColor = [...defaults.baseColor]
    if (defaults.metallic !== undefined) current.metallic = defaults.metallic
    if (defaults.roughness !== undefined) current.roughness = defaults.roughness
    if (!this.dev) return
    const tail = this.materialTailScratch
    for (const mesh of this.meshes) {
      if (mesh.material) continue
      tail[0] = current.baseColor[0]; tail[1] = current.baseColor[1]; tail[2] = current.baseColor[2]
      tail[3] = current.metallic
      tail[4] = 0; tail[5] = 0; tail[6] = 0
      tail[7] = current.roughness
      this.dev.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.materialByteOffset, tail)
    }
    this.requestRender()
  }

  clearSelection() {
    if (this.selected === null && !this.isolated) return
    const sourceVisibilityChanged = this.isolated
    this.resetDepthCycle(false)
    this.selected = null
    this.selectedHit = null
    this.isolated = false
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
    this.edgeBuffersByVertexBuffer.clear()
    for (const mesh of this.meshes) {
      if (!mesh.edgeIB) continue
      const candidates = this.edgeBuffersByVertexBuffer.get(mesh.vb) ?? []
      if (!candidates.some(candidate => candidate.edgeIB === mesh.edgeIB)) candidates.push(mesh)
      this.edgeBuffersByVertexBuffer.set(mesh.vb, candidates)
    }
  }

  private ensureEdgeBuffer(mesh: GMesh) {
    const dev = this.dev
    if (!dev || mesh.edgeIB || !mesh.edgeIndices.length) return
    const byteLength = mesh.edgeIndices.byteLength
    if (byteLength > MAX_EDGE_BUFFER_BYTES || byteLength > dev.limits.maxBufferSize) return

    const candidates = this.edgeBuffersByVertexBuffer.get(mesh.vb)
    const shared = candidates?.find(candidate => candidate.ib === mesh.ib
      && sameTypedArray(candidate.edgeIndices, mesh.edgeIndices))
    if (shared?.edgeIB) {
      mesh.edgeIB = shared.edgeIB
      mesh.edgeIC = shared.edgeIC
      return
    }

    let buffer: GPUBuffer | null = null
    try {
      buffer = dev.createBuffer({ size: byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
      dev.queue.writeBuffer(buffer, 0, mesh.edgeIndices)
      mesh.edgeIB = buffer
      mesh.edgeIC = mesh.edgeIndices.length
      if (candidates) candidates.push(mesh)
      else this.edgeBuffersByVertexBuffer.set(mesh.vb, [mesh])
    } catch {
      buffer?.destroy()
    }
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
    const dev = this.dev
    if (!dev) return
    for (let index = 0; index < this.meshes.length; index++) {
      const mesh = this.meshes[index]
      const selected = this.usesObjectSelectionStyle(index) ? 1 : 0
      const hovered = this.usesObjectHoverStyle(index) && !selected ? 1 : 0
      const progress = this.geometryFade?.progress ?? 1
      const alpha = effectiveDisplayAlpha(mesh.color[3], this.displayMode) * progress * progress * (3 - 2 * progress)
      const edgeOpacity = (selected || hovered ? 1 : this.displayMode === 'edges' ? 0.7 : 0) * progress * progress * (3 - 2 * progress)
      mesh.alpha = alpha
      if (mesh.styleAlpha === alpha && mesh.styleSelected === selected
        && mesh.styleEdge === edgeOpacity && mesh.styleHovered === hovered) continue
      mesh.styleAlpha = alpha
      mesh.styleSelected = selected
      mesh.styleEdge = edgeOpacity
      mesh.styleHovered = hovered
      this.styleScratch[0] = alpha
      this.styleScratch[1] = selected
      this.styleScratch[2] = edgeOpacity
      this.styleScratch[3] = hovered
      dev.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.styleByteOffset, this.styleScratch)
      if (edgeOpacity > 0) this.ensureEdgeBuffer(mesh)
    }
  }

  private setSelection(index: number | null, hit: PickHit | null = null) {
    if (index !== null && (!this.meshes[index] || !this.meshes[index].visible)) index = null
    if (this.selected === index && this.selectedHit === hit && (!this.isolated || index !== null)) return
    const previousSelected = this.selected
    const wasIsolated = this.isolated
    this.selected = index
    this.selectedHit = index === null ? null : hit
    if (index === null) this.isolated = false
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
    this.edgeBundle.clear()
    this.opaqueDraws.length = this.edgeDraws.length = 0
    this.transparentSort.clear()
  }

  private render() {
    this.updateSize()
    const canvas = this.canvas, dev = this.dev, ctx = this.ctx, sceneUB = this.sceneUB
    if (!canvas || !dev || !ctx || !this.depth || !sceneUB || !this.drawable || !canvas.width || !canvas.height) return

    const { eye, viewProjection } = this.cameraState()
    // Hoisted once per frame; the morph set cannot change mid-render.
    const hasMorph = this.meshes.some(mesh => mesh.morph)
    const sd = this.sceneUniformScratch
    const inverseVP = invert(viewProjection)
    for (let row = 0; row < 4; row++) {
      for (let column = 0; column < 4; column++) {
        sd[column * 4 + row] = viewProjection[row * 4 + column]
        sd[36 + column * 4 + row] = inverseVP[row * 4 + column]
      }
    }
    sd[16] = eye[0]; sd[17] = eye[1]; sd[18] = eye[2]; sd[19] = 1
    sd[20] = 0.55; sd[21] = 0.75; sd[22] = 0.45; sd[23] = 0
    sd[24] = 0.22; sd[25] = 0.22; sd[26] = 0.24; sd[27] = 1
    sd[28] = this.sectionNormal[0]; sd[29] = this.sectionNormal[1]
    sd[30] = this.sectionNormal[2]; sd[31] = this.sectionOffset
    sd[32] = this.sectionEnabled ? 1 : 0
    sd[33] = this.gridStep
    sd[34] = this.gridExtent()
    sd[35] = this.gridFadeDistance()
    // Theme tail (see SCENE_UNIFORM_LAYOUT): vec3 + pad per color.
    const theme = this.theme
    sd.set(theme.selectionColor, SCENE_UNIFORM_LAYOUT.themeFloatOffset)
    sd.set(theme.hoverColor, SCENE_UNIFORM_LAYOUT.hoverFloatOffset)
    sd.set(theme.edgeColor, SCENE_UNIFORM_LAYOUT.edgeFloatOffset)
    sd.set(theme.xrayColor, SCENE_UNIFORM_LAYOUT.xrayFloatOffset)
    sd.set(theme.gridColor, SCENE_UNIFORM_LAYOUT.gridFloatOffset)
    dev.queue.writeBuffer(sceneUB, 0, sd)

    const enc = dev.createCommandEncoder()
    // Cached in updateSize; the fallback covers depth textures the cache miss
    // predates (tests and external texture swaps) without per-frame allocation.
    const depthView = this.depthView ?? this.depth.createView()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: ctx.getCurrentTexture().createView(),
        clearValue: {
          r: this.backgroundColor[0],
          g: this.backgroundColor[1],
          b: this.backgroundColor[2],
          a: 1,
        },
        loadOp: 'clear', storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: depthView,
        depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
      },
    })

    if (this.gridVisible && this.gridQuadVB) {
      pass.setPipeline(this.gridPipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.gridQuadVB)
      pass.draw(6)
    }
    if (this.gridVisible && this.gridVB) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.gridVB)
      pass.draw(this.gridVC)
    }
    if (this.measurementVB && this.measurementVC) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.measurementVB)
      pass.draw(this.measurementVC)
    }

    const transitioning = this.geometryGhosts.length > 0 || hasMorph
    this.viewFrustum.update(viewProjection)
    this.opaqueDraws.length = this.edgeDraws.length = 0
    this.transparentSort.begin()
    for (let index = 0; index < this.meshes.length; index++) {
      const mesh = this.meshes[index]
      if (!this.isMeshVisible(index) || (!mesh.morph && !this.viewFrustum.intersects(mesh.worldBounds.center, mesh.worldBounds.radius))) continue
      if (isTransparentAlpha(mesh.alpha)) this.transparentSort.add(index, mesh.worldBounds.center)
      else this.opaqueDraws.push(mesh)
      const highlighted = this.usesObjectSelectionStyle(index) || this.usesObjectHoverStyle(index)
      if (mesh.edgeIB && (this.displayMode === 'edges' || highlighted)) this.edgeDraws.push(mesh)
    }
    const customOpaque = this.opaqueDraws.some(mesh => mesh.shadingModel !== 'phong')
    if (!customOpaque && (transitioning || !this.opaqueInstances.draw(pass, dev, this.instanceBGL, this.instanceMeshPipe, this.sceneBG, this.opaqueDraws))) {
      this.opaqueBundle.draw(pass, dev, this.fmt, this.meshPipe, this.sceneBG, this.opaqueDraws)
    } else if (customOpaque) {
      // Mixed materials: draw each shading-model group with its resolved mesh
      // pipeline (source order preserved); instancing serves the default path.
      for (const group of this.groupByShadingModel(this.opaqueDraws)) {
        this.opaqueBundle.draw(pass, dev, this.fmt, this.getRenderPipeline(resolveMeshShaderId(group[0].shadingModel)), this.sceneBG, group)
      }
    }

    pass.setPipeline(this.meshImmediatePipeT ?? this.meshPipeT)
    pass.setBindGroup(0, this.sceneBG)
    const ghostMeshes = this.geometryGhosts.length
      ? this.geometryGhosts.flatMap(ghost => ghost.meshes).filter(mesh => mesh.alpha > 0)
      : NO_GHOST_MESHES
    ghostMeshes.forEach((mesh, index) => this.transparentSort.add(this.meshes.length + index, mesh.worldBounds.center))
    const transparentOrder = this.transparentSort.sort(eye, this.tx, this.ty, this.tz)
    this.transparentDraws.length = 0
    for (const { index } of transparentOrder) this.transparentDraws.push(index < this.meshes.length ? this.meshes[index] : ghostMeshes[index - this.meshes.length])
    const customTransparent = this.transparentDraws.some(mesh => mesh.shadingModel !== 'phong')
    if (!customTransparent && (transitioning || !this.transparentInstances.draw(pass, dev, this.instanceBGL, this.instanceMeshPipeT, this.sceneBG, this.transparentDraws))) {
      for (const g of this.transparentDraws) {
        pass.setBindGroup(1, g.bg)
        this.setObjectStyleImmediate(pass, g)
        pass.setVertexBuffer(0, g.vb)
        pass.setVertexBuffer(1, g.morphSlot!)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    } else if (customTransparent) {
      // Mixed materials: per-entity pass with the resolved transparent pipeline
      // for each shading model (uniform variant; immediates serve 'mesh' only).
      let activeModel: ShadingModel | null = null
      for (const g of this.transparentDraws) {
        if (g.shadingModel !== activeModel) {
          activeModel = g.shadingModel
          pass.setPipeline(activeModel === 'phong'
            ? this.meshImmediatePipeT ?? this.meshPipeT
            : this.getRenderPipeline(resolveMeshShaderId(activeModel), { blend: 'alpha', depth: { writeEnabled: false, compare: 'less' } }))
        }
        pass.setBindGroup(1, g.bg)
        if (activeModel === 'phong') this.setObjectStyleImmediate(pass, g)
        pass.setVertexBuffer(0, g.vb)
        pass.setVertexBuffer(1, g.morphSlot!)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    }

    const deepSelectedIndex = this.selectionMode === 'object'
      && (this.selectedHit?.cycleIndex ?? 0) > 0
      ? this.selected
      : null
    if (deepSelectedIndex !== null) {
      const selectedMesh = this.meshes[deepSelectedIndex]
      if (this.isMeshVisible(deepSelectedIndex) && selectedMesh) {
        pass.setPipeline(this.deepMeshImmediatePipe ?? this.deepMeshPipe)
        pass.setBindGroup(0, this.sceneBG)
        pass.setBindGroup(1, selectedMesh.bg)
        this.setObjectStyleImmediate(pass, selectedMesh)
        pass.setVertexBuffer(0, selectedMesh.vb)
        pass.setVertexBuffer(1, selectedMesh.morphSlot!)
        pass.setIndexBuffer(selectedMesh.ib, 'uint32')
        pass.drawIndexed(selectedMesh.ic)
      }
    }

    if (!this.geometryFade && !this.geometryGhosts.length && !hasMorph && this.sourceFaceSlot.buffer && this.sourceFaceSlot.count) {
      pass.setPipeline(this.selectionFacePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.sourceFaceSlot.buffer)
      pass.draw(this.sourceFaceSlot.count)
    }

    if (!this.geometryFade && !this.geometryGhosts.length && !hasMorph && this.selectionFaceSlot.buffer && this.selectionFaceSlot.count) {
      pass.setPipeline(this.selectionFacePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.selectionFaceSlot.buffer)
      pass.draw(this.selectionFaceSlot.count)
    }

    if (transitioning || !this.edgeInstances.draw(pass, dev, this.instanceBGL, this.instanceEdgePipe, this.sceneBG, this.edgeDraws, true)) {
      this.edgeBundle.draw(pass, dev, this.fmt, this.edgePipe, this.sceneBG, this.edgeDraws, true)
    }

    if (deepSelectedIndex !== null) {
      const selectedMesh = this.meshes[deepSelectedIndex]
      if (this.isMeshVisible(deepSelectedIndex) && selectedMesh?.edgeIB) {
        pass.setPipeline(this.deepEdgeImmediatePipe ?? this.deepEdgePipe)
        pass.setBindGroup(0, this.sceneBG)
        pass.setBindGroup(1, selectedMesh.bg)
        this.setObjectStyleImmediate(pass, selectedMesh)
        pass.setVertexBuffer(0, selectedMesh.vb)
        pass.setVertexBuffer(1, selectedMesh.morphSlot!)
        pass.setIndexBuffer(selectedMesh.edgeIB, 'uint32')
        pass.drawIndexed(selectedMesh.edgeIC)
      }
    }

    if (!this.geometryFade && !this.geometryGhosts.length && !hasMorph && this.sourceLineSlot.buffer && this.sourceLineSlot.count) {
      pass.setPipeline(this.selectionLinePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.sourceLineSlot.buffer)
      pass.draw(this.sourceLineSlot.count)
    }

    if (!this.geometryFade && !this.geometryGhosts.length && !hasMorph && this.selectionLineSlot.buffer && this.selectionLineSlot.count) {
      pass.setPipeline(this.selectionLinePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.selectionLineSlot.buffer)
      pass.draw(this.selectionLineSlot.count)
    }

    if (!this.geometryFade && !this.geometryGhosts.length && !hasMorph && this.deepSelectionLineSlot.buffer && this.deepSelectionLineSlot.count) {
      pass.setPipeline(this.deepSelectionLinePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.deepSelectionLineSlot.buffer)
      pass.draw(this.deepSelectionLineSlot.count)
    }

    pass.end()
    dev.queue.submit([enc.finish()])
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
    let active = false
    if (this.geometryFade) {
      if (finish || now - this.geometryFade.started >= 180) this.geometryFade = null
      else { this.geometryFade.progress = Math.max(0, (now - this.geometryFade.started) / 180); active = true }
      this.updateMeshStyles()
    }
    for (const ghost of this.geometryGhosts) {
      const t = finish ? 1 : Math.min(1, Math.max(0, (now - ghost.started) / 180))
      if (t === 1) this.destroyMeshes(ghost.meshes)
      else {
        active = true
        ghost.meshes.forEach((mesh, i) => {
          mesh.alpha = ghost.alphas[i] * (1 - t*t*(3-2*t))
          this.styleScratch[0] = mesh.alpha
          this.styleScratch[1] = 0
          this.styleScratch[2] = 0
          this.styleScratch[3] = 0
          this.dev?.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.styleByteOffset, this.styleScratch)
        })
      }
    }
    this.geometryGhosts = this.geometryGhosts.filter(ghost => !finish && now - ghost.started < 180)
    let morphFinished = false
    for (const mesh of this.meshes) {
      const morph = mesh.morph
      if (!morph) continue
      const t = finish ? 1 : Math.min(1, Math.max(0, (now - morph.started) / 180))
      const eased = t * t * (3 - 2 * t)
      if (morph.matrix) {
        morph.currentMatrix = morph.matrix(eased)
        const uniform = this.objectUniformScratch
        for (let row = 0; row < 4; row++) for (let column = 0; column < 4; column++) uniform[column*4+row] = morph.currentMatrix[row*4+column]
        invert(morph.currentMatrix, uniform.subarray(16, 32))
        this.dev?.queue.writeBuffer(mesh.ub, 0, uniform.subarray(0, 32))
      }
      if (t === 1) {
        // The vertex buffer already holds the destination; retire the morph
        // and restore the slot-1 dummy with the weight back at rest (1), so
        // the stale blend state is inert: mix(dummy, pos, 1) = pos.
        mesh.morph = undefined
        mesh.morphSlot = this.morphDummyVB
        this.morphScratch[0] = 1; this.morphScratch[1] = 0; this.morphScratch[2] = 0; this.morphScratch[3] = 0
        this.dev?.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.morphByteOffset, this.morphScratch)
        morphFinished = true
      } else {
        // The vertex shader interpolates from the morph source; only the
        // blend weight travels to the GPU each frame.
        this.morphScratch[0] = eased; this.morphScratch[1] = 0; this.morphScratch[2] = 0; this.morphScratch[3] = 0
        this.dev?.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.morphByteOffset, this.morphScratch)
        active = true
      }
    }
    // Finished morphs flip slot 1 back to the dummy; retained draw bundles and
    // instance caches key on the binding and must be rebuilt.
    if (morphFinished) this.clearDrawCaches()
    return active
  }

  /**
   * Groups draws by shading model, preserving first-seen order. Used only when
   * a scene mixes materials; the all-default path skips the allocation.
   */
  private groupByShadingModel(draws: readonly GMesh[]): GMesh[][] {
    const groups = new Map<ShadingModel, GMesh[]>()
    for (const mesh of draws) {
      const group = groups.get(mesh.shadingModel)
      if (group) group.push(mesh)
      else groups.set(mesh.shadingModel, [mesh])
    }
    return [...groups.values()]
  }

  private setObjectStyleImmediate(pass: GPURenderPassEncoder, mesh: GMesh) {
    if (!this.immediateObjectStyle) return
    this.styleScratch[0] = mesh.styleAlpha
    this.styleScratch[1] = mesh.styleSelected
    this.styleScratch[2] = mesh.styleEdge
    this.styleScratch[3] = mesh.styleHovered
    pass.setImmediates(0, this.styleScratch)
  }

  private drawFrame = () => {
    this.raf = 0
    if (this.dead || this.lost || !this.initialized) return
    this.notifyCameraChange()
    try {
      const animating = this.advanceGeometryAnimation(performance.now())
      this.render()
      if (animating) this.requestRender(false)
      this.frameRetryCount = 0
      if (this.status.status === 'error' && this.status.phase === 'frame') {
        this.updateStatus({ status: 'ready' })
      }
    } catch (error) {
      // A lost/outdated surface can still be retried on the next invalidation,
      // but the failure must remain observable to both UI and developers.
      this.reportError('frame', error)
      if (this.frameRetryCount < 1) {
        this.frameRetryCount++
        this.requestRender(false)
      }
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

  private sourceForTriangle(mesh: GMesh, triangleIndex: number): MeshProvenanceRun | null {
    let low = 0
    let high = mesh.provenance.length - 1
    while (low <= high) {
      const middle = (low + high) >>> 1
      const run = mesh.provenance[middle]
      if (triangleIndex < run.triangleStart) high = middle - 1
      else if (triangleIndex >= run.triangleEnd) low = middle + 1
      else return run
    }
    return null
  }

  private pickHitFromBvh(meshIndex: number, mesh: GMesh, hit: MeshBvhHit): PickHit | null {
    const run = this.sourceForTriangle(mesh, hit.triangleIndex)
    let point = hit.worldPoint as Vec3
    if (this.selectionMode === 'point') {
      const vertices = hit.triangleVertexIndices.map(index => {
        const offset = index * mesh.bvh.vertexStride
        return [mesh.vertices[offset], mesh.vertices[offset + 1], mesh.vertices[offset + 2]] as Vec3
      })
      const corner = selectedCornerInKernel(mesh.transform, vertices, hit.barycentric)
      if (!corner) return null
      point = corner
    }
    return {
      meshIndex,
      triangleIndex: hit.triangleIndex,
      faceId: mesh.faceIds[hit.triangleIndex] ?? null,
      point,
      normal: hit.worldNormal,
      barycentric: hit.barycentric,
      source: run?.source ?? null,
      backside: run?.backside ?? false,
    }
  }

  private isSectionClipped(point: readonly number[]) {
    return this.sectionEnabled && (
      point[0] * this.sectionNormal[0]
      + point[1] * this.sectionNormal[1]
      + point[2] * this.sectionNormal[2]
    ) < this.sectionOffset
  }

  private selectionCandidateKey(hit: PickHit) {
    if (this.selectionMode === 'object') return `o:${hit.meshIndex}`
    if (this.selectionMode === 'face') {
      return hit.faceId === null
        ? `f:${hit.meshIndex}:t${hit.triangleIndex}`
        : `f:${hit.meshIndex}:${hit.faceId}`
    }
    // Flat-shaded meshes may duplicate a geometric vertex for each face. A
    // world-space key avoids multiple visually identical point-cycle stops.
    return `p:${hit.meshIndex}:${hit.point.map(value => value.toPrecision(12)).join(':')}`
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
    const limit = Math.max(0, Math.min(MAX_DEPTH_CANDIDATES, Math.trunc(maximum)))
    if (!limit) return []
    const ray = this.rayForClientPoint(clientX, clientY)
    if (!ray) return []

    type HitCursor = {
      meshIndex: number
      mesh: GMesh
      hit: MeshBvhHit
      /** Reusable growing buffer; only the first `excludedCount` entries are live. */
      excludedTriangles: Uint32Array
      excludedCount: number
    }
    const frontier: HitCursor[] = []
    const cursorBefore = (a: HitCursor, b: HitCursor) => a.hit.t < b.hit.t
      || (a.hit.t === b.hit.t && (
        a.meshIndex < b.meshIndex
        || (a.meshIndex === b.meshIndex && a.hit.triangleIndex < b.hit.triangleIndex)
      ))
    const insertFrontier = (cursor: HitCursor) => {
      frontier.push(cursor)
      let child = frontier.length - 1
      while (child > 0) {
        const parent = (child - 1) >>> 1
        if (!cursorBefore(frontier[child], frontier[parent])) break
        ;[frontier[parent], frontier[child]] = [frontier[child], frontier[parent]]
        child = parent
      }
    }
    const takeNearest = () => {
      const nearest = frontier[0]
      const last = frontier.pop()!
      if (frontier.length) {
        frontier[0] = last
        let parent = 0
        while (true) {
          const left = parent * 2 + 1
          const right = left + 1
          if (left >= frontier.length) break
          let child = left
          if (right < frontier.length && cursorBefore(frontier[right], frontier[left])) child = right
          if (!cursorBefore(frontier[child], frontier[parent])) break
          ;[frontier[parent], frontier[child]] = [frontier[child], frontier[parent]]
          parent = child
        }
      }
      return nearest
    }

    const boundsCandidates = querySceneAabbIndex(this.currentSceneAabbIndex(), ray)
    let boundsCursor = 0
    const addNextBoundsCandidate = () => {
      const candidate = boundsCandidates[boundsCursor++]
      if (!candidate) return false
      const index = candidate.id
      if (!this.isMeshVisible(index)) return true
      const mesh = this.meshes[index]
      const hit = this.nativePicking.query(mesh.vb, mesh.vertices, mesh.indices, mesh.bvh.vertexStride, mesh.bvh.leafSize, ray, {
        localFromWorld: mesh.inverseTransform,
      })
      if (hit) insertFrontier({ meshIndex: index, mesh, hit, excludedTriangles: new Uint32Array(8), excludedCount: 0 })
      return true
    }

    const candidates: DepthCandidate<PickHit>[] = []
    const seen = new Set<string>()
    let continuations = 0
    while (candidates.length < limit) {
      // Bounds are front-to-back lower bounds. Touch triangle BVHs lazily until
      // no unopened object can beat the exact hit at the frontier. This makes
      // hover/measurement (`limit = 1`) independent of farther overlapping
      // bodies while preserving deterministic depth cycling for larger limits.
      while (!frontier.length && boundsCursor < boundsCandidates.length) addNextBoundsCandidate()
      while (
        frontier.length
        && boundsCursor < boundsCandidates.length
        && boundsCandidates[boundsCursor].distance <= frontier[0].hit.t
      ) addNextBoundsCandidate()
      if (!frontier.length) break

      const cursor = takeNearest()
      const clipped = this.isSectionClipped(cursor.hit.worldPoint)
      if (!clipped) {
        const { meshIndex, mesh, hit } = cursor
        const value = this.pickHitFromBvh(meshIndex, mesh, hit)
        const key = value ? this.selectionCandidateKey(value) : null
        if (value && key !== null && !seen.has(key)) {
          seen.add(key)
          candidates.push({ key, distance: hit.t, value })
        }
      }

      const needsContinuation = clipped || this.selectionMode !== 'object'
      if (!needsContinuation || continuations >= MAX_DEPTH_CONTINUATIONS || candidates.length >= limit) continue
      continuations++
      // Append-only exclusions: grow the buffer amortized instead of rebuilding
      // an Array.from(Set) on every continuation query.
      if (cursor.excludedCount === cursor.excludedTriangles.length) {
        const grown = new Uint32Array(cursor.excludedTriangles.length * 2)
        grown.set(cursor.excludedTriangles)
        cursor.excludedTriangles = grown
      }
      cursor.excludedTriangles[cursor.excludedCount++] = cursor.hit.triangleIndex
      const next = this.nativePicking.query(cursor.mesh.vb, cursor.mesh.vertices, cursor.mesh.indices, cursor.mesh.bvh.vertexStride, cursor.mesh.bvh.leafSize, ray, {
        minT: cursor.hit.t,
        excludedTriangles: cursor.excludedTriangles.subarray(0, cursor.excludedCount),
        localFromWorld: cursor.mesh.inverseTransform,
      })
      if (next) insertFrontier({ ...cursor, hit: next })
    }
    return normalizeDepthCandidates(candidates, limit)
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
    this.measurementVB = null
    this.measurementVC = 0
    const dev = this.dev
    if (!dev || !this.measurementPoints.length) return
    const data: number[] = []
    const markerSize = Math.max(0.01, (this.bounds?.radius ?? 10) * 0.018)
    const colors = [[1, 0.55, 0.08, 1], [0.1, 0.82, 1, 1]]
    const line = (a: Vec3, b: Vec3, color: number[]) => data.push(...a, ...color, ...b, ...color)
    this.measurementPoints.forEach((point, index) => {
      const color = colors[index] ?? colors[0]
      line([point[0] - markerSize, point[1], point[2]], [point[0] + markerSize, point[1], point[2]], color)
      line([point[0], point[1] - markerSize, point[2]], [point[0], point[1] + markerSize, point[2]], color)
      line([point[0], point[1], point[2] - markerSize], [point[0], point[1], point[2] + markerSize], color)
    })
    if (this.measurementPoints.length === 2) line(this.measurementPoints[0], this.measurementPoints[1], [1, 0.83, 0.26, 1])
    const values = new Float32Array(data)
    this.measurementVB = dev.createBuffer({ size: values.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    dev.queue.writeBuffer(this.measurementVB, 0, values)
    this.measurementVC = values.length / 7
  }

  private releaseOverlaySlot(slot: OverlaySlot) {
    slot.buffer?.destroy()
    slot.buffer = null
    slot.capacity = 0
    slot.count = 0
  }

  private clearSelectionOverlayBuffers() {
    this.releaseOverlaySlot(this.selectionFaceSlot)
    this.releaseOverlaySlot(this.selectionLineSlot)
    this.releaseOverlaySlot(this.deepSelectionLineSlot)
  }

  private clearSourceHighlightOverlayBuffers() {
    this.releaseOverlaySlot(this.sourceFaceSlot)
    this.releaseOverlaySlot(this.sourceLineSlot)
  }

  /**
   * Uploads overlay vertices into a persistent buffer, growing its capacity
   * with headroom only when the current allocation is too small. Per-frame
   * hover updates therefore reuse the same GPU buffer via queue.writeBuffer
   * instead of a destroy/create pair.
   */
  private writeOverlaySlot(slot: OverlaySlot, values: number[]) {
    slot.count = 0
    const dev = this.dev
    if (!dev || !values.length) return
    const data = new Float32Array(values)
    const maxBytes = Math.min(MAX_OVERLAY_BUFFER_BYTES, dev.limits.maxBufferSize)
    if (data.byteLength > maxBytes) return
    if (!slot.buffer || slot.capacity < data.byteLength) {
      const capacity = Math.min(maxBytes, Math.max(4096, data.byteLength * 2))
      let replacement: GPUBuffer | null = null
      try {
        replacement = dev.createBuffer({ size: capacity, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
        dev.queue.writeBuffer(replacement, 0, data)
      } catch {
        replacement?.destroy()
        return
      }
      const previous = slot.buffer
      slot.buffer = replacement
      slot.capacity = capacity
      slot.count = values.length / 7
      previous?.destroy()
      return
    }
    try {
      dev.queue.writeBuffer(slot.buffer, 0, data)
      slot.count = values.length / 7
    } catch {
      slot.count = 0
    }
  }

  private appendColoredPositions(target: number[], positions: Float32Array, color: readonly number[]) {
    for (let offset = 0; offset + 2 < positions.length; offset += 3) {
      target.push(positions[offset], positions[offset + 1], positions[offset + 2], ...color)
    }
  }

  private pointForOverlay(hit: PickHit): Vec3 | null {
    const mesh = this.meshes[hit.meshIndex]
    if (!mesh) return null
    return pointOverlayPosition(
      mesh.vertices,
      mesh.indices,
      mesh.transform,
      hit.triangleIndex,
      hit.barycentric,
    ) ?? (hit.point.every(Number.isFinite) ? [...hit.point] : null)
  }

  private sameOverlayTarget(a: PickHit | null, b: PickHit | null) {
    if (!a || !b || a.meshIndex !== b.meshIndex) return false
    if (this.selectionMode === 'object') return true
    if (this.selectionMode === 'face') {
      return a.faceId !== null && b.faceId !== null
        ? a.faceId === b.faceId
        : a.triangleIndex === b.triangleIndex
    }
    const ap = this.pointForOverlay(a)
    const bp = this.pointForOverlay(b)
    if (!ap || !bp) return false
    const scale = Math.max(1, ...ap.map(Math.abs), ...bp.map(Math.abs))
    return Math.hypot(ap[0] - bp[0], ap[1] - bp[1], ap[2] - bp[2]) <= scale * 1e-7
  }

  private appendFaceOverlay(
    hit: PickHit,
    faceValues: number[],
    lineValues: number[],
    fillColor: readonly number[],
    lineColor: readonly number[],
  ) {
    const mesh = this.meshes[hit.meshIndex]
    if (!mesh || !this.isMeshVisible(hit.meshIndex)) return
    if (mesh.faceIndex === undefined) {
      // Built once per mesh on the first face-mode hover; every later overlay
      // rebuild then costs O(face) instead of scanning the whole mesh.
      mesh.faceIndex = buildFaceTriangleIndex(mesh.faceIds, Math.floor(mesh.indices.length / 3))
    }
    const geometry = buildFaceOverlayGeometry(
      mesh.vertices,
      mesh.indices,
      mesh.faceIds,
      mesh.transform,
      hit.triangleIndex,
      hit.faceId,
      undefined,
      mesh.faceIndex,
    )
    this.appendColoredPositions(faceValues, geometry.triangles, fillColor)
    this.appendColoredPositions(lineValues, geometry.boundaryLines, lineColor)
  }

  private appendPointOverlay(hit: PickHit, lineValues: number[], color: readonly number[]) {
    if (!this.isMeshVisible(hit.meshIndex)) return
    const point = this.pointForOverlay(hit)
    if (!point) return
    const markerSize = Math.max(0.002, (this.bounds?.radius ?? 10) * 0.022)
    const positions = new Float32Array([
      point[0] - markerSize, point[1], point[2], point[0] + markerSize, point[1], point[2],
      point[0], point[1] - markerSize, point[2], point[0], point[1] + markerSize, point[2],
      point[0], point[1], point[2] - markerSize, point[0], point[1], point[2] + markerSize,
    ])
    this.appendColoredPositions(lineValues, positions, color)
  }

  private rebuildSelectionOverlays() {
    this.selectionFaceSlot.count = 0
    this.selectionLineSlot.count = 0
    this.deepSelectionLineSlot.count = 0
    if (!this.dev || this.selectionMode === 'object') return

    const faceValues: number[] = []
    const lineValues: number[] = []
    const deepLineValues: number[] = []
    const add = (hit: PickHit, preselected: boolean) => {
      const targetLines = !preselected && (hit.cycleIndex ?? 0) > 0
        ? deepLineValues
        : lineValues
      if (this.selectionMode === 'face') {
        this.appendFaceOverlay(
          hit,
          faceValues,
          targetLines,
          preselected ? [0.05, 0.82, 1, 0.27] : [1, 0.47, 0.04, 0.38],
          preselected ? [0.08, 0.88, 1, 1] : [1, 0.58, 0.08, 1],
        )
      } else {
        this.appendPointOverlay(hit, targetLines, preselected ? [0.08, 0.88, 1, 1] : [1, 0.58, 0.08, 1])
      }
    }

    if (this.selectedHit) add(this.selectedHit, false)
    if (this.hoveredHit && !this.sameOverlayTarget(this.selectedHit, this.hoveredHit)) add(this.hoveredHit, true)

    this.writeOverlaySlot(this.selectionFaceSlot, faceValues)
    this.writeOverlaySlot(this.selectionLineSlot, lineValues)
    this.writeOverlaySlot(this.deepSelectionLineSlot, deepLineValues)
  }

  private rebuildSourceHighlightOverlay() {
    this.sourceFaceSlot.count = 0
    this.sourceLineSlot.count = 0
    if (!this.dev || this.sourceHighlightId === null) return

    const faceValues: number[] = []
    const lineValues: number[] = []
    let remainingTriangles = MAX_SOURCE_OVERLAY_TRIANGLES
    for (let index = 0; index < this.meshes.length && remainingTriangles > 0; index++) {
      if (!this.isMeshVisible(index)) continue
      const mesh = this.meshes[index]
      const geometry = buildSourceOverlayGeometry(
        mesh.vertices,
        mesh.indices,
        mesh.provenance,
        mesh.transform,
        this.sourceHighlightId,
        remainingTriangles,
      )
      this.appendColoredPositions(faceValues, geometry.triangles, [0.68, 0.24, 1, 0.32])
      this.appendColoredPositions(lineValues, geometry.boundaryLines, [0.82, 0.42, 1, 1])
      remainingTriangles -= geometry.triangleCount
      if (geometry.truncated) break
    }

    this.writeOverlaySlot(this.sourceFaceSlot, faceValues)
    this.writeOverlaySlot(this.sourceLineSlot, lineValues)
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
    const destroyed = new Set<GPUBuffer>()
    const destroy = (buffer: GPUBuffer | null) => {
      if (!buffer || preserved.has(buffer) || destroyed.has(buffer)) return
      destroyed.add(buffer)
      buffer.destroy()
    }
    for (const g of meshes) {
      if (!preserved.has(g.vb)) this.nativePicking.release(g.vb)
      destroy(g.edgeIB)
      destroy(g.vb); destroy(g.ib); destroy(g.ub); destroy(g.morphVB)
    }
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
    this.sceneUB?.destroy()
    this.sceneUB = null
    this.morphDummyVB?.destroy()
    this.morphDummyVB = null
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

  private rebuildSceneAabbIndex() {
    const replacement = buildSceneAabbIndex(this.meshes.flatMap((mesh, index) => (
      mesh.visible
        ? [{ id: index, bounds: { min: mesh.worldBounds.min, max: mesh.worldBounds.max } }]
        : []
    )))
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
