/**
 * WebGPU 3D renderer — Phong shading, orbit camera, grid floor, axis gizmo.
 */
import {
  perspective, orthographic, lookAt, transpose, invert, multiply,
  transformPoint, unprojectRay,
  type Aabb3, type Mat4, type Vec3,
} from './math3d'
import { raycastMeshBvh, type MeshBvh } from './meshBvh'
import { buildFaceOverlayGeometry, pointOverlayPosition } from './meshSelectionOverlay'
import type { MeshData, MeshProvenanceRun, MeshSourceReference } from './openscadParser'

/* ── WGSL shaders ─────────────────────────────────── */

const MESH_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj   { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f }

@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> V {
  let wp = (ob.model * vec4f(pos,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  let selected = mix(ob.color.rgb, vec3f(1.0, 0.52, 0.06), ob.style.y * 0.48);
  let base = mix(selected, vec3f(0.12, 0.78, 1.0), ob.style.w * 0.38);
  let c = sc.ambient.rgb * base + d * base + s * vec3f(0.25) + bd * base * 0.5;
  return vec4f(c, ob.style.x);
}
`

const LINE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }
`

const SELECTION_OVERLAY_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  var clip = sc.vp * vec4f(pos, 1);
  clip.z -= 0.00018 * clip.w;
  return V(clip, col, pos);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  return v.c;
}
`

const EDGE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj   { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct EdgeV { @builtin(position) p: vec4f, @location(0) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f) -> EdgeV {
  let world = (ob.model * vec4f(pos, 1)).xyz;
  var p = sc.vp * ob.model * vec4f(pos, 1);
  p.z -= 0.00008 * p.w;
  return EdgeV(p, world);
}

@fragment fn fs(v: EdgeV) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  let selected = mix(vec3f(0.025, 0.03, 0.04), vec3f(1.0, 0.55, 0.08), ob.style.y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), ob.style.w);
  return vec4f(color, ob.style.z);
}
`

/* ── GPU mesh handle ──────────────────────────────── */

interface GMesh {
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
}

interface Bounds {
  center: [number, number, number]
  radius: number
  min: [number, number, number]
  max: [number, number, number]
}

export type ProjectionMode = 'perspective' | 'orthographic'
export type StandardView = 'iso' | 'front' | 'back' | 'left' | 'right' | 'top' | 'bottom'
export type DisplayMode = 'shaded' | 'edges' | 'xray'
export type SelectionMode = 'object' | 'face' | 'point'
export interface PickHit {
  meshIndex: number
  triangleIndex: number
  faceId: number | null
  point: Vec3
  normal: Vec3
  barycentric: Vec3
  source: MeshSourceReference | null
  backside: boolean
}
export interface DistanceMeasurement {
  points: Vec3[]
  distance: number | null
}
export type SelectionChangeHandler = (selectedIndex: number | null, isIsolated: boolean, hit: PickHit | null) => void
export type HoverChangeHandler = (hit: PickHit | null) => void
export type MeasurementChangeHandler = (measurement: DistanceMeasurement | null, active: boolean) => void
export interface SetMeshesOptions {
  /** Keep world-space inspection measurements while swapping equivalent geometry. */
  preserveMeasurement?: boolean
}

const FOV_Y = Math.PI / 4
const ISO_YAW = Math.PI / 4
const ISO_PITCH = Math.atan(1 / Math.sqrt(2))
const DEFAULT_DISTANCE = 50
const MIN_DISTANCE = 0.01
const MAX_DISTANCE = 1e12
const MAX_ORBIT_PITCH = Math.PI / 2 - 0.001
const MAX_DPR = 2
const GRID_SIZE = 200
const GRID_STEP = 10
const CLICK_MOVE_THRESHOLD = 3
const MAX_EDGE_BUFFER_BYTES = 32 * 1024 * 1024
const MAX_OVERLAY_BUFFER_BYTES = 16 * 1024 * 1024

/* ── Renderer class ───────────────────────────────── */

export class WebGPURenderer {
  private canvas: HTMLCanvasElement | null = null
  private dev: GPUDevice | null = null
  private ctx: GPUCanvasContext | null = null
  private fmt: GPUTextureFormat = 'bgra8unorm'

  private meshPipe!: GPURenderPipeline
  private meshPipeT!: GPURenderPipeline
  private linePipe!: GPURenderPipeline
  private edgePipe!: GPURenderPipeline
  private selectionFacePipe!: GPURenderPipeline
  private selectionLinePipe!: GPURenderPipeline
  private sceneBGL!: GPUBindGroupLayout
  private objBGL!: GPUBindGroupLayout
  private sceneUB: GPUBuffer | null = null
  private sceneBG!: GPUBindGroup
  private depth: GPUTexture | null = null

  private meshes: GMesh[] = []
  private gridVB: GPUBuffer | null = null
  private gridVC = 0
  private bounds: Bounds | null = null
  private initialFitDone = false
  private projection: ProjectionMode = 'perspective'
  private gridVisible = true
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
  private selectionFaceVB: GPUBuffer | null = null
  private selectionFaceVC = 0
  private selectionLineVB: GPUBuffer | null = null
  private selectionLineVC = 0

  onSelectionChange: SelectionChangeHandler | null = null
  onHoverChange: HoverChangeHandler | null = null
  onMeasurementChange: MeasurementChangeHandler | null = null

  yaw = ISO_YAW; pitch = ISO_PITCH; dist = DEFAULT_DISTANCE
  tx = 0; ty = 0; tz = 0

  private raf = 0
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
  private mx = 0; private my = 0
  private downX = 0; private downY = 0
  private downButton = -1
  private gestureMoved = false
  private hoverRaf = 0
  private hoverGeneration = 0
  private hoverX = 0
  private hoverY = 0

  async init(canvas: HTMLCanvasElement): Promise<boolean> {
    if (this.canvas || this.dev || this.initialized) this.destroy()

    const generation = ++this.generation
    this.canvas = canvas
    this.dead = false
    this.lost = false

    try {
      if (typeof navigator === 'undefined' || !navigator.gpu) {
        this.destroy()
        return false
      }

      const gpu = navigator.gpu
      const adapter = await gpu.requestAdapter()
      if (!adapter || !this.isCurrentInit(generation, canvas)) {
        if (this.isCurrentInit(generation, canvas)) this.destroy()
        return false
      }

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
        this.destroy()
        return false
      }

      this.ctx = context
      this.fmt = gpu.getPreferredCanvasFormat()
      context.configure({ device, format: this.fmt, alphaMode: 'premultiplied' })

      void device.lost.then(() => {
        if (this.dev !== device || this.dead) return
        this.lost = true
        this.initialized = false
        this.drawable = false
        this.cancelPendingHover()
        if (this.raf) cancelAnimationFrame(this.raf)
        this.raf = 0
      })

      this.buildPipelines()
      this.buildSceneUB()
      this.buildGrid()
      this.initialized = true
      this.bindInput()
      this.observeResize()
      this.resize()
      this.requestRender()
      return true
    } catch {
      if (this.isCurrentInit(generation, canvas)) this.destroy()
      return false
    }
  }

  private isCurrentInit(generation: number, canvas: HTMLCanvasElement) {
    return generation === this.generation && this.canvas === canvas && !this.dead
  }

  private buildPipelines() {
    const dev = this.dev!
    this.sceneBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })
    this.objBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })

    const meshMod = dev.createShaderModule({ code: MESH_WGSL })
    const meshLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL] })

    const vbl: GPUVertexBufferLayout = {
      arrayStride: 24,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x3' },
      ],
    }
    const ds: GPUDepthStencilState = { format: 'depth24plus', depthWriteEnabled: true, depthCompare: 'less' }

    this.meshPipe = dev.createRenderPipeline({
      layout: meshLayout,
      vertex: { module: meshMod, entryPoint: 'vs', buffers: [vbl] },
      fragment: { module: meshMod, entryPoint: 'fs', targets: [{ format: this.fmt }] },
      primitive: { topology: 'triangle-list', cullMode: 'none' },
      depthStencil: ds,
    })

    this.meshPipeT = dev.createRenderPipeline({
      layout: meshLayout,
      vertex: { module: meshMod, entryPoint: 'vs', buffers: [vbl] },
      fragment: { module: meshMod, entryPoint: 'fs', targets: [{
        format: this.fmt,
        blend: {
          color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        },
      }] },
      primitive: { topology: 'triangle-list', cullMode: 'none' },
      depthStencil: { ...ds, depthWriteEnabled: false },
    })

    const lineMod = dev.createShaderModule({ code: LINE_WGSL })
    const lineLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL] })
    const lineVBL: GPUVertexBufferLayout = {
      arrayStride: 28,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x4' },
      ],
    }
    this.linePipe = dev.createRenderPipeline({
      layout: lineLayout,
      vertex: { module: lineMod, entryPoint: 'vs', buffers: [lineVBL] },
      fragment: { module: lineMod, entryPoint: 'fs', targets: [{
        format: this.fmt,
        blend: {
          color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        },
      }] },
      primitive: { topology: 'line-list' },
      depthStencil: ds,
    })

    const edgeMod = dev.createShaderModule({ code: EDGE_WGSL })
    this.edgePipe = dev.createRenderPipeline({
      layout: meshLayout,
      vertex: {
        module: edgeMod,
        entryPoint: 'vs',
        buffers: [{
          arrayStride: 24,
          attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x3' }],
        }],
      },
      fragment: {
        module: edgeMod,
        entryPoint: 'fs',
        targets: [{
          format: this.fmt,
          blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          },
        }],
      },
      primitive: { topology: 'line-list' },
      depthStencil: { ...ds, depthWriteEnabled: false, depthCompare: 'less-equal' },
    })

    const selectionMod = dev.createShaderModule({ code: SELECTION_OVERLAY_WGSL })
    const selectionLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL] })
    const selectionVBL: GPUVertexBufferLayout = {
      arrayStride: 28,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x4' },
      ],
    }
    const selectionTarget: GPUColorTargetState = {
      format: this.fmt,
      blend: {
        color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      },
    }
    const selectionDepth: GPUDepthStencilState = {
      ...ds,
      depthWriteEnabled: false,
      depthCompare: 'less-equal',
    }
    this.selectionFacePipe = dev.createRenderPipeline({
      layout: selectionLayout,
      vertex: { module: selectionMod, entryPoint: 'vs', buffers: [selectionVBL] },
      fragment: { module: selectionMod, entryPoint: 'fs', targets: [selectionTarget] },
      primitive: { topology: 'triangle-list', cullMode: 'none' },
      depthStencil: selectionDepth,
    })
    this.selectionLinePipe = dev.createRenderPipeline({
      layout: selectionLayout,
      vertex: { module: selectionMod, entryPoint: 'vs', buffers: [selectionVBL] },
      fragment: { module: selectionMod, entryPoint: 'fs', targets: [selectionTarget] },
      primitive: { topology: 'line-list' },
      depthStencil: selectionDepth,
    })
  }

  private buildSceneUB() {
    const dev = this.dev!
    this.sceneUB = dev.createBuffer({ size: 144, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.sceneBG = dev.createBindGroup({
      layout: this.sceneBGL,
      entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
    })
  }

  private buildGrid() {
    const dev = this.dev!
    const d: number[] = []
    const gc = [0.35, 0.35, 0.35, 0.4]
    const xc = [0.95, 0.18, 0.16, 0.9]
    const yc = [0.2, 0.85, 0.25, 0.9]
    const zc = [0.2, 0.45, 1, 0.95]

    // OpenSCAD is Z-up: its work plane is XY.
    for (let i = -GRID_SIZE; i <= GRID_SIZE; i += GRID_STEP) {
      if (i === 0) continue
      d.push(i,-GRID_SIZE,0,...gc, i,GRID_SIZE,0,...gc)
      d.push(-GRID_SIZE,i,0,...gc, GRID_SIZE,i,0,...gc)
    }
    d.push(-GRID_SIZE,0,0,...xc, GRID_SIZE,0,0,...xc)
    d.push(0,-GRID_SIZE,0,...yc, 0,GRID_SIZE,0,...yc)
    d.push(0,0,-GRID_SIZE,...zc, 0,0,GRID_SIZE,...zc)

    this.gridVC = d.length / 7
    this.gridVB = dev.createBuffer({ size: d.length * 4, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    dev.queue.writeBuffer(this.gridVB, 0, new Float32Array(d))
  }

  setMeshes(meshes: MeshData[], options: SetMeshesOptions = {}) {
    this.cancelPendingHover()
    const dev = this.dev
    if (!dev || !this.initialized || this.dead || this.lost) return

    const next: GMesh[] = []
    try {
      for (const m of meshes) {
        if (!m.indices.length && !m.vertices.length) continue
        if (m.vertices.length < 6 || m.vertices.length % 6 !== 0 || !m.indices.length) {
          throw new Error('Invalid mesh buffer layout')
        }

        const measured = this.measureMeshBounds(m.vertices, m.transform)
        if (!measured) throw new Error('Mesh contains no finite positions')
        const transform = new Float32Array(m.transform)
        const inverseTransform = invert(transform)

        let vb: GPUBuffer | null = null
        let ib: GPUBuffer | null = null
        let ub: GPUBuffer | null = null
        try {
          vb = dev.createBuffer({ size: m.vertices.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
          ib = dev.createBuffer({ size: m.indices.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
          ub = dev.createBuffer({ size: 160, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
          dev.queue.writeBuffer(vb, 0, m.vertices)
          dev.queue.writeBuffer(ib, 0, m.indices)
          dev.queue.writeBuffer(ub, 0, transpose(transform))
          // Column-major bytes for inverse-transpose(row-major model).
          dev.queue.writeBuffer(ub, 64, inverseTransform)
          dev.queue.writeBuffer(ub, 128, new Float32Array(m.color))
          dev.queue.writeBuffer(ub, 144, new Float32Array([
            this.displayMode === 'xray' ? Math.min(m.color[3], 0.24) : m.color[3],
            0,
            this.displayMode === 'edges' ? 0.7 : 0,
            0,
          ]))
          const bg = dev.createBindGroup({
            layout: this.objBGL,
            entries: [{ binding: 0, resource: { buffer: ub } }],
          })
          next.push({
            vb, ib, ic: m.indices.length, ub, bg,
            edgeIB: null, edgeIC: 0,
            vertices: m.vertices, indices: m.indices,
            edgeIndices: m.edgeIndices, faceIds: m.faceIds, bvh: m.bvh,
            provenance: m.provenance,
            transform, inverseTransform,
            color: [...m.color],
            alpha: this.displayMode === 'xray' ? Math.min(m.color[3], 0.24) : m.color[3],
            localBounds: measured.local,
            worldBounds: measured.world,
            visible: true,
          })
        } catch (error) {
          vb?.destroy(); ib?.destroy(); ub?.destroy()
          throw error
        }
      }
    } catch (error) {
      this.destroyMeshes(next)
      throw error
    }

    const nextBounds = this.combineBounds(next.map(mesh => mesh.worldBounds))
    const previous = this.meshes
    const selectionChanged = this.selected !== null || this.isolated
    const hoverChanged = this.hovered !== null || this.hoveredHit !== null
    this.meshes = next
    this.bounds = nextBounds
    this.selected = null
    this.selectedHit = null
    this.hovered = null
    this.hoveredHit = null
    this.isolated = false
    if (!options.preserveMeasurement) {
      this.measurementPoints = []
      this.measureActive = false
    }
    this.rebuildMeasurementBuffer()
    this.clearSelectionOverlayBuffers()
    this.emitMeasurementChange()
    this.destroyMeshes(previous)
    if (this.displayMode === 'edges') {
      for (const mesh of next) this.ensureEdgeBuffer(mesh)
    }
    if (selectionChanged) this.emitSelectionChange()
    if (hoverChanged) {
      try { this.onHoverChange?.(null) } catch { /* UI callbacks must not break rendering. */ }
    }

    if (nextBounds && !this.initialFitDone) {
      this.initialFitDone = true
      this.fitView()
    } else {
      this.requestRender()
    }
  }

  private measureMeshBounds(vertices: Float32Array, transform: Mat4): { local: Aabb3; world: Bounds } | null {
    const localMin: Vec3 = [Infinity, Infinity, Infinity]
    const localMax: Vec3 = [-Infinity, -Infinity, -Infinity]
    const worldMin: Vec3 = [Infinity, Infinity, Infinity]
    const worldMax: Vec3 = [-Infinity, -Infinity, -Infinity]
    for (let i = 0; i + 2 < vertices.length; i += 6) {
      const point: Vec3 = [vertices[i], vertices[i+1], vertices[i+2]]
      if (!point.every(Number.isFinite)) continue
      const world = transformPoint(transform, point)
      if (!world.every(Number.isFinite)) continue
      for (let axis = 0; axis < 3; axis++) {
        localMin[axis] = Math.min(localMin[axis], point[axis])
        localMax[axis] = Math.max(localMax[axis], point[axis])
        worldMin[axis] = Math.min(worldMin[axis], world[axis])
        worldMax[axis] = Math.max(worldMax[axis], world[axis])
      }
    }
    if (!Number.isFinite(localMin[0])) return null
    const center: Vec3 = [
      (worldMin[0]+worldMax[0])/2,
      (worldMin[1]+worldMax[1])/2,
      (worldMin[2]+worldMax[2])/2,
    ]
    return {
      local: { min: localMin, max: localMax },
      world: {
        center,
        radius: Math.hypot(worldMax[0]-worldMin[0], worldMax[1]-worldMin[1], worldMax[2]-worldMin[2]) / 2,
        min: worldMin,
        max: worldMax,
      },
    }
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

  fitView() {
    if (this.bounds) this.fitBounds(this.bounds)
  }

  private fitBounds(bounds: Bounds) {
    const [x, y, z] = bounds.center
    this.tx = x; this.ty = y; this.tz = z
    const aspect = this.getAspect()
    const halfVertical = FOV_Y / 2
    const halfHorizontal = Math.atan(Math.tan(halfVertical) * aspect)
    const limitingHalfFov = Math.max(1e-6, Math.min(halfVertical, halfHorizontal))
    const radius = Math.max(bounds.radius, 0.5)
    this.dist = this.clampDistance(radius * 1.15 / Math.sin(limitingHalfFov))
    this.requestRender()
  }

  get currentDisplayMode(): DisplayMode { return this.displayMode }
  get selectedIndex(): number | null { return this.selected }
  get currentHit(): PickHit | null { return this.selectedHit }
  get isIsolated(): boolean { return this.isolated }

  setDisplayMode(mode: DisplayMode) {
    if (this.displayMode === mode) return
    this.displayMode = mode
    if (mode === 'edges') {
      for (const mesh of this.meshes) this.ensureEdgeBuffer(mesh)
    }
    this.updateMeshStyles()
    this.requestRender()
  }

  clearSelection() {
    if (this.selected === null && !this.isolated) return
    this.selected = null
    this.selectedHit = null
    this.isolated = false
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
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
    this.rebuildSelectionOverlays()
    this.emitSelectionChange()
    this.requestRender()
    return this.isolated
  }

  resetView() {
    this.yaw = ISO_YAW
    this.pitch = ISO_PITCH
    if (this.bounds) this.fitView()
    else {
      this.tx = 0; this.ty = 0; this.tz = 0
      this.dist = DEFAULT_DISTANCE
      this.requestRender()
    }
  }

  setProjection(projection: ProjectionMode) {
    if (this.projection === projection) return
    this.projection = projection
    this.requestRender()
  }

  setGridVisible(visible: boolean) {
    if (this.gridVisible === visible) return
    this.gridVisible = visible
    this.requestRender()
  }

  setSelectionMode(mode: SelectionMode) {
    if (this.selectionMode === mode) return
    this.selectionMode = mode
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
    this.requestRender()
  }

  selectMesh(index: number | null) {
    this.setSelection(index, null)
  }

  setMeshVisibility(index: number, visible: boolean) {
    const mesh = this.meshes[index]
    if (!mesh || mesh.visible === visible) return
    mesh.visible = visible
    if (!visible && this.selected === index) this.setSelection(null, null)
    if (!visible && this.hovered === index) {
      this.hovered = null
      this.hoveredHit = null
      this.updateMeshStyles()
      try { this.onHoverChange?.(null) } catch { /* UI callbacks must not break rendering. */ }
    }
    this.bounds = this.combineBounds(this.meshes.filter(candidate => candidate.visible).map(candidate => candidate.worldBounds))
    this.rebuildSelectionOverlays()
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

  private ensureEdgeBuffer(mesh: GMesh) {
    const dev = this.dev
    if (!dev || mesh.edgeIB || !mesh.edgeIndices.length) return
    const byteLength = mesh.edgeIndices.byteLength
    if (byteLength > MAX_EDGE_BUFFER_BYTES || byteLength > dev.limits.maxBufferSize) return

    let buffer: GPUBuffer | null = null
    try {
      buffer = dev.createBuffer({ size: byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
      dev.queue.writeBuffer(buffer, 0, mesh.edgeIndices)
      mesh.edgeIB = buffer
      mesh.edgeIC = mesh.edgeIndices.length
    } catch {
      buffer?.destroy()
    }
  }

  private updateMeshStyles() {
    const dev = this.dev
    if (!dev) return
    for (let index = 0; index < this.meshes.length; index++) {
      const mesh = this.meshes[index]
      const selected = this.usesObjectSelectionStyle(index)
      const hovered = this.usesObjectHoverStyle(index) && !selected
      mesh.alpha = this.displayMode === 'xray' ? Math.min(mesh.color[3], 0.24) : mesh.color[3]
      const edgeOpacity = selected || hovered ? 1 : this.displayMode === 'edges' ? 0.7 : 0
      dev.queue.writeBuffer(mesh.ub, 144, new Float32Array([mesh.alpha, selected ? 1 : 0, edgeOpacity, hovered ? 1 : 0]))
      if (edgeOpacity > 0) this.ensureEdgeBuffer(mesh)
    }
  }

  private setSelection(index: number | null, hit: PickHit | null = null) {
    if (index !== null && !this.meshes[index]) index = null
    if (this.selected === index && this.selectedHit === hit && (!this.isolated || index !== null)) return
    this.selected = index
    this.selectedHit = index === null ? null : hit
    if (index === null) this.isolated = false
    this.updateMeshStyles()
    this.rebuildSelectionOverlays()
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
    switch (view) {
      case 'iso': this.yaw = ISO_YAW; this.pitch = ISO_PITCH; break
      case 'front': this.yaw = 0; this.pitch = 0; break
      case 'back': this.yaw = Math.PI; this.pitch = 0; break
      case 'left': this.yaw = -Math.PI / 2; this.pitch = 0; break
      case 'right': this.yaw = Math.PI / 2; this.pitch = 0; break
      case 'top': this.yaw = 0; this.pitch = Math.PI / 2; break
      case 'bottom': this.yaw = 0; this.pitch = -Math.PI / 2; break
    }
    this.requestRender()
  }

  resize() {
    if (this.updateSize()) this.requestRender()
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

  private clipPlanes(): [number, number] {
    let extent = this.gridVisible ? Math.hypot(GRID_SIZE, GRID_SIZE, GRID_SIZE) : 1
    const activeBounds = this.isolated && this.selected !== null
      ? this.meshes[this.selected]?.worldBounds
      : this.bounds
    if (activeBounds) {
      const [x, y, z] = activeBounds.center
      extent = Math.max(extent, Math.hypot(x-this.tx, y-this.ty, z-this.tz) + activeBounds.radius)
    }
    const nearest = this.dist - extent * 1.1
    const near = Math.max(0.001, Math.min(this.dist * 0.01, nearest > 0 ? nearest : 0.001))
    const far = Math.max(near + 1, this.dist + extent * 1.1)
    return [near, far]
  }

  private cameraState() {
    const asp = this.getAspect()
    const cp = Math.cos(this.pitch), sp = Math.sin(this.pitch)
    const eye: Vec3 = [
      this.tx + this.dist * cp * Math.sin(this.yaw),
      this.ty - this.dist * cp * Math.cos(this.yaw),
      this.tz + this.dist * sp,
    ]
    const view = lookAt(eye, [this.tx,this.ty,this.tz], [0,0,1])
    const [near, far] = this.clipPlanes()
    const halfHeight = Math.max(MIN_DISTANCE, this.dist * Math.tan(FOV_Y / 2))
    const projection = this.projection === 'perspective'
      ? perspective(FOV_Y, asp, near, far)
      : orthographic(-halfHeight*asp, halfHeight*asp, -halfHeight, halfHeight, near, far)
    return { eye, viewProjection: multiply(projection, view) }
  }

  private render() {
    this.updateSize()
    const canvas = this.canvas, dev = this.dev, ctx = this.ctx, depth = this.depth, sceneUB = this.sceneUB
    if (!canvas || !dev || !ctx || !depth || !sceneUB || !this.drawable || !canvas.width || !canvas.height) return

    const { eye, viewProjection } = this.cameraState()
    const vp = transpose(viewProjection)
    const sd = new Float32Array(36)
    sd.set(vp, 0)
    sd.set([eye[0],eye[1],eye[2],1], 16)
    sd.set([0.55,0.75,0.45,0], 20)
    sd.set([0.22,0.22,0.24,1], 24)
    sd.set([this.sectionNormal[0], this.sectionNormal[1], this.sectionNormal[2], this.sectionOffset], 28)
    sd.set([this.sectionEnabled ? 1 : 0, 0, 0, 0], 32)
    dev.queue.writeBuffer(sceneUB, 0, sd)

    const enc = dev.createCommandEncoder()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: ctx.getCurrentTexture().createView(),
        clearValue: { r: 0.09, g: 0.09, b: 0.11, a: 1 },
        loadOp: 'clear', storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: depth.createView(),
        depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
      },
    })

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

    pass.setPipeline(this.meshPipe)
    pass.setBindGroup(0, this.sceneBG)
    for (let index = 0; index < this.meshes.length; index++) {
      const g = this.meshes[index]
      if (!this.isMeshVisible(index) || g.alpha < 0.99) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
    }

    pass.setPipeline(this.meshPipeT)
    pass.setBindGroup(0, this.sceneBG)
    for (let index = 0; index < this.meshes.length; index++) {
      const g = this.meshes[index]
      if (!this.isMeshVisible(index) || g.alpha >= 0.99) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
    }

    if (this.selectionFaceVB && this.selectionFaceVC) {
      pass.setPipeline(this.selectionFacePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.selectionFaceVB)
      pass.draw(this.selectionFaceVC)
    }

    pass.setPipeline(this.edgePipe)
    pass.setBindGroup(0, this.sceneBG)
    for (let index = 0; index < this.meshes.length; index++) {
      const g = this.meshes[index]
      const objectHighlight = this.usesObjectSelectionStyle(index) || this.usesObjectHoverStyle(index)
      if (!this.isMeshVisible(index) || !g.edgeIB || (this.displayMode !== 'edges' && !objectHighlight)) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.edgeIB, 'uint32')
      pass.drawIndexed(g.edgeIC)
    }


    if (this.selectionLineVB && this.selectionLineVC) {
      pass.setPipeline(this.selectionLinePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.selectionLineVB)
      pass.draw(this.selectionLineVC)
    }

    pass.end()
    dev.queue.submit([enc.finish()])
  }

  requestRender() {
    if (this.dead || this.lost || !this.initialized || this.raf) return
    this.raf = requestAnimationFrame(this.drawFrame)
  }

  private drawFrame = () => {
    this.raf = 0
    if (this.dead || this.lost || !this.initialized) return
    try { this.render() } catch { /* A lost/outdated surface is retried on the next invalidation. */ }
  }

  private rayForClientPoint(clientX: number, clientY: number) {
    const canvas = this.canvas
    if (!canvas) return null
    const rect = canvas.getBoundingClientRect()
    if (!(rect.width > 0) || !(rect.height > 0)) return null
    const ndcX = ((clientX - rect.left) / rect.width) * 2 - 1
    const ndcY = 1 - ((clientY - rect.top) / rect.height) * 2
    if (!Number.isFinite(ndcX) || !Number.isFinite(ndcY)) return null
    const { viewProjection } = this.cameraState()
    return unprojectRay(invert(viewProjection), ndcX, ndcY)
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

  private findHit(clientX: number, clientY: number): PickHit | null {
    const ray = this.rayForClientPoint(clientX, clientY)
    if (!ray) return null
    let nearest = Infinity
    let picked: PickHit | null = null
    for (let index = 0; index < this.meshes.length; index++) {
      if (!this.isMeshVisible(index)) continue
      const mesh = this.meshes[index]
      let minT = 0
      for (let attempt = 0; attempt < 32; attempt++) {
        const hit = raycastMeshBvh(mesh.bvh, mesh.vertices, mesh.indices, ray, {
          minT,
          maxT: nearest,
          localFromWorld: mesh.inverseTransform,
        })
        if (!hit) break
        if (this.sectionEnabled && (
          hit.worldPoint[0] * this.sectionNormal[0]
          + hit.worldPoint[1] * this.sectionNormal[1]
          + hit.worldPoint[2] * this.sectionNormal[2]
        ) < this.sectionOffset) {
          minT = hit.t + Math.max(1e-8, Math.abs(hit.t) * 1e-8)
          continue
        }
        nearest = hit.t
        const run = this.sourceForTriangle(mesh, hit.triangleIndex)
        let point = hit.worldPoint as Vec3
        if (this.selectionMode === 'point') {
          let corner = 0
          if (hit.barycentric[1] > hit.barycentric[corner]) corner = 1
          if (hit.barycentric[2] > hit.barycentric[corner]) corner = 2
          const vertexIndex = hit.triangleVertexIndices[corner]
          const offset = vertexIndex * 6
          point = transformPoint(mesh.transform, [mesh.vertices[offset], mesh.vertices[offset + 1], mesh.vertices[offset + 2]])
        }
        picked = {
          meshIndex: index,
          triangleIndex: hit.triangleIndex,
          faceId: mesh.faceIds[hit.triangleIndex] ?? null,
          point,
          normal: hit.worldNormal,
          barycentric: hit.barycentric,
          source: run?.source ?? null,
          backside: run?.backside ?? false,
        }
        break
      }
    }
    return picked
  }

  private pickAt(clientX: number, clientY: number) {
    const hit = this.findHit(clientX, clientY)
    if (hit && this.measureActive) this.addMeasurementPoint(hit.point)
    this.setSelection(hit?.meshIndex ?? null, hit)
  }

  private updateHoverAt(clientX: number, clientY: number) {
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
    try { this.onHoverChange?.(hit) } catch { /* UI callbacks must not break rendering. */ }
    if (styleChanged || overlayChanged) this.requestRender()
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

  private clearSelectionOverlayBuffers() {
    this.selectionFaceVB?.destroy()
    this.selectionFaceVB = null
    this.selectionFaceVC = 0
    this.selectionLineVB?.destroy()
    this.selectionLineVB = null
    this.selectionLineVC = 0
  }

  private createSelectionOverlayBuffer(values: number[]): GPUBuffer | null {
    const dev = this.dev
    if (!dev || !values.length) return null
    const data = new Float32Array(values)
    if (data.byteLength > MAX_OVERLAY_BUFFER_BYTES || data.byteLength > dev.limits.maxBufferSize) return null
    let buffer: GPUBuffer | null = null
    try {
      buffer = dev.createBuffer({ size: data.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
      dev.queue.writeBuffer(buffer, 0, data)
      return buffer
    } catch {
      buffer?.destroy()
      return null
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
    const geometry = buildFaceOverlayGeometry(
      mesh.vertices,
      mesh.indices,
      mesh.faceIds,
      mesh.transform,
      hit.triangleIndex,
      hit.faceId,
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
    this.clearSelectionOverlayBuffers()
    if (!this.dev || this.selectionMode === 'object') return

    const faceValues: number[] = []
    const lineValues: number[] = []
    const add = (hit: PickHit, preselected: boolean) => {
      if (this.selectionMode === 'face') {
        this.appendFaceOverlay(
          hit,
          faceValues,
          lineValues,
          preselected ? [0.05, 0.82, 1, 0.27] : [1, 0.47, 0.04, 0.38],
          preselected ? [0.08, 0.88, 1, 1] : [1, 0.58, 0.08, 1],
        )
      } else {
        this.appendPointOverlay(hit, lineValues, preselected ? [0.08, 0.88, 1, 1] : [1, 0.58, 0.08, 1])
      }
    }

    if (this.selectedHit) add(this.selectedHit, false)
    if (this.hoveredHit && !this.sameOverlayTarget(this.selectedHit, this.hoveredHit)) add(this.hoveredHit, true)

    this.selectionFaceVB = this.createSelectionOverlayBuffer(faceValues)
    this.selectionFaceVC = this.selectionFaceVB ? faceValues.length / 7 : 0
    this.selectionLineVB = this.createSelectionOverlayBuffer(lineValues)
    this.selectionLineVC = this.selectionLineVB ? lineValues.length / 7 : 0
  }

  private onDown = (e: PointerEvent) => {
    const canvas = this.canvas
    if (!canvas || this.activePointer !== null || (e.pointerType === 'mouse' && e.button > 2)) return
    try { canvas.focus({ preventScroll: true }) } catch { canvas.focus() }
    this.drag = true; this.pan = e.button !== 0 || e.shiftKey
    this.activePointer = e.pointerId
    this.mx = e.clientX; this.my = e.clientY
    this.downX = e.clientX; this.downY = e.clientY
    this.downButton = e.button
    this.gestureMoved = false
    try { canvas.setPointerCapture(e.pointerId) } catch { /* Pointer may already be gone. */ }
    e.preventDefault()
  }
  private onMove = (e: PointerEvent) => {
    if (!this.drag) { this.scheduleHover(e.clientX, e.clientY); return }
    if (this.activePointer !== e.pointerId) return
    if (!this.gestureMoved) {
      this.gestureMoved = Math.hypot(e.clientX - this.downX, e.clientY - this.downY) >= CLICK_MOVE_THRESHOLD
      if (!this.gestureMoved) return
    }
    const dx = e.clientX - this.mx, dy = e.clientY - this.my
    this.mx = e.clientX; this.my = e.clientY
    if (this.pan) {
      const scale = 2 * this.dist * Math.tan(FOV_Y / 2) / Math.max(1, this.canvas?.clientHeight || 1)
      const cy = Math.cos(this.yaw), sy = Math.sin(this.yaw)
      const cp = Math.cos(this.pitch), sp = Math.sin(this.pitch)
      const rx = cy, ry = sy
      const ux = -sp*sy, uy = sp*cy, uz = cp
      this.tx += (-dx*rx + dy*ux) * scale
      this.ty += (-dx*ry + dy*uy) * scale
      this.tz += dy*uz * scale
    } else {
      this.yaw -= dx * 0.005
      if (this.yaw > Math.PI || this.yaw < -Math.PI) this.yaw = ((this.yaw + Math.PI) % (2*Math.PI) + 2*Math.PI) % (2*Math.PI) - Math.PI
      this.pitch = Math.max(-MAX_ORBIT_PITCH, Math.min(MAX_ORBIT_PITCH, this.pitch + dy * 0.005))
    }
    this.requestRender()
  }
  private onPointerEnd = (e: PointerEvent) => {
    if (this.activePointer !== e.pointerId) return
    const shouldPick = e.type === 'pointerup'
      && !this.gestureMoved
      && Math.hypot(e.clientX - this.downX, e.clientY - this.downY) < CLICK_MOVE_THRESHOLD
      && !this.pan
      && this.downButton === 0
    this.activePointer = null
    this.drag = false
    this.pan = false
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
    const delta = Math.max(-1000, Math.min(1000, e.deltaY * unit))
    this.dist = this.clampDistance(this.dist * Math.exp(delta * 0.001))
    this.requestRender()
  }
  private noCtx = (e: Event) => e.preventDefault()
  private onWindowResize = () => this.resize()

  private clampDistance(distance: number) {
    return Math.max(MIN_DISTANCE, Math.min(MAX_DISTANCE, Number.isFinite(distance) ? distance : DEFAULT_DISTANCE))
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

  private destroyMeshes(meshes: GMesh[]) {
    for (const g of meshes) {
      g.edgeIB?.destroy()
      g.vb.destroy(); g.ib.destroy(); g.ub.destroy()
    }
  }

  destroy() {
    ++this.generation
    this.dead = true
    this.initialized = false
    if (this.raf) cancelAnimationFrame(this.raf)
    this.raf = 0
    this.cancelPendingHover()
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
    this.drag = false
    this.pan = false
    this.gestureMoved = false

    this.destroyMeshes(this.meshes)
    this.meshes = []
    this.gridVB?.destroy()
    this.gridVB = null
    this.measurementVB?.destroy()
    this.measurementVB = null
    this.measurementVC = 0
    this.clearSelectionOverlayBuffers()
    this.depth?.destroy()
    this.depth = null
    this.sceneUB?.destroy()
    this.sceneUB = null
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
    this.initialFitDone = false
    this.drawable = false
    this.lost = false
    try { device?.destroy() } catch { /* Repeated/lost-device cleanup is harmless. */ }
    if (selectionChanged) this.emitSelectionChange()
  }
}
