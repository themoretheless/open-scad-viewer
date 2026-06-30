/**
 * WebGPU 3D renderer — Phong shading, orbit camera, grid floor, axis gizmo.
 */
import {
  perspective, ortho, lookAt, transpose, invert, multiply, type Mat4,
} from './math3d'
import type { MeshData } from './openscadParser'

/* ── WGSL shaders ─────────────────────────────────── */

const MESH_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f, sectionBox: vec4f }
struct Obj   { model: mat4x4f, nmat: mat4x4f, color: vec4f }

@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> V {
  let wp = (ob.model * vec4f(pos,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  // Multi-axis clipping plane
  if (sc.clip.y > 0.5) {
    let clipAxis = i32(sc.clip.z);
    var clipCoord = v.w.y;
    if (clipAxis == 0) { clipCoord = v.w.x; }
    else if (clipAxis == 2) { clipCoord = v.w.z; }
    if (clipCoord < sc.clip.x) { discard; }
  }

  // Section box: clip on all 3 axes simultaneously
  if (sc.sectionBox.w > 0.5) {
    if (v.w.x < sc.sectionBox.x) { discard; }
    if (v.w.y < sc.sectionBox.y) { discard; }
    if (v.w.z < sc.sectionBox.z) { discard; }
  }

  // Flat or smooth shading
  var N = normalize(v.n);
  if (sc._pad0.x > 0.5) {
    N = normalize(cross(dpdx(v.w), dpdy(v.w)));
  }

  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  var d = max(dot(N, L), 0.0);

  // Toon / cel shading: quantize diffuse into discrete steps
  if (sc._pad1.x > 0.5) {
    d = floor(d * 4.0) / 4.0;
  }

  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  var c = sc.ambient.rgb * ob.color.rgb + d * ob.color.rgb + s * vec3f(0.25) + bd * ob.color.rgb * 0.5;

  // Fog
  if (sc.fogParams.z > 0.5) {
    let dist = length(sc.eye.xyz - v.w);
    let fogFactor = clamp((dist - sc.fogParams.x) / (sc.fogParams.y - sc.fogParams.x), 0.0, 1.0);
    c = mix(c, sc.fogColor.rgb, fogFactor);
  }

  // SSAO approximation
  if (sc._pad0.y > 0.5) {
    let ao = 0.5 + 0.5 * max(dot(N, V2), 0.0);
    c = c * ao;
  }

  // Gooch shading
  if (sc._pad0.z > 0.5) {
    let gooch_cool = vec3f(0.0, 0.0, 0.55) + 0.25 * ob.color.rgb;
    let gooch_warm = vec3f(0.3, 0.3, 0.0) + 0.25 * ob.color.rgb;
    let gooch_t = (1.0 + dot(N, L)) * 0.5;
    let gooch_c = mix(gooch_cool, gooch_warm, gooch_t);
    c = gooch_c + s * vec3f(0.25);
  }

  return vec4f(c, ob.color.a);
}
`

const LINE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f, sectionBox: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }
`

const OUTLINE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f, sectionBox: vec4f }
struct Obj   { model: mat4x4f, nmat: mat4x4f, color: vec4f }

@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> @builtin(position) vec4f {
  let wp = (ob.model * vec4f(pos,1)).xyz + normalize((ob.nmat * vec4f(norm,0)).xyz) * 0.3;
  return sc.vp * vec4f(wp,1);
}

@fragment fn fs() -> @location(0) vec4f {
  return vec4f(0.0, 0.0, 0.0, 1.0);
}
`

const SKY_WGSL = /* wgsl */`
struct SkyParams { topColor: vec4f, bottomColor: vec4f }
@group(0) @binding(0) var<uniform> sky: SkyParams;

struct V { @builtin(position) p: vec4f, @location(0) uv: f32 }

@vertex fn vs(@builtin(vertex_index) vi: u32) -> V {
  let x = f32(i32(vi) / 2) * 4.0 - 1.0;
  let y = f32(i32(vi) % 2) * 4.0 - 1.0;
  return V(vec4f(x, y, 0.999, 1.0), (y + 1.0) * 0.5);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  return mix(sky.bottomColor, sky.topColor, v.uv);
}
`

const SKY_PRESETS: Record<string, { top: [number,number,number,number], bottom: [number,number,number,number] }> = {
  none: { top: [0,0,0,0], bottom: [0,0,0,0] },
  clearSky: { top: [0.4, 0.6, 0.9, 1], bottom: [0.85, 0.9, 0.95, 1] },
  sunset: { top: [0.15, 0.05, 0.3, 1], bottom: [0.85, 0.4, 0.15, 1] },
  studio: { top: [0.05, 0.05, 0.05, 1], bottom: [0.2, 0.2, 0.22, 1] },
  neutral: { top: [0.4, 0.4, 0.42, 1], bottom: [0.4, 0.4, 0.42, 1] },
}

/* ── GPU mesh handle ──────────────────────────────── */

interface GMesh {
  vb: GPUBuffer; ib: GPUBuffer; ic: number
  ub: GPUBuffer; bg: GPUBindGroup; transp: boolean
}

/* ── Renderer class ───────────────────────────────── */

export class WebGPURenderer {
  private canvas!: HTMLCanvasElement
  private dev!: GPUDevice
  private ctx!: GPUCanvasContext
  private fmt!: GPUTextureFormat

  private meshPipe!: GPURenderPipeline
  private meshPipeT!: GPURenderPipeline
  private outlinePipe!: GPURenderPipeline
  private skyPipe: GPURenderPipeline | null = null
  private skyUB: GPUBuffer | null = null
  private skyBG: GPUBindGroup | null = null
  private skyBGL: GPUBindGroupLayout | null = null
  private linePipe!: GPURenderPipeline
  private sceneBGL!: GPUBindGroupLayout
  private objBGL!: GPUBindGroupLayout
  private sceneUB!: GPUBuffer
  private sceneBG!: GPUBindGroup
  private depth!: GPUTexture

  /* MSAA textures (4x) */
  private msaaColorTex!: GPUTexture
  private msaaDepthTex!: GPUTexture

  private meshes: GMesh[] = []
  private lastRawMeshes: MeshData[] = []
  private gridVB: GPUBuffer | null = null
  private gridVC = 0

  /* build plate (print bed) outline */
  private plateVB: GPUBuffer | null = null
  private plateVC = 0
  showBuildPlate = false
  buildPlateX = 220
  buildPlateZ = 220

  /* reusable reflection uniform buffer + bind group */
  private reflUB!: GPUBuffer
  private reflBG!: GPUBindGroup

  /* wireframe overlay */
  private wireframeVB: GPUBuffer | null = null
  private wireframeVC = 0
  wireframe = false

  /* edge overlay (CAD-style) */
  private edgeVB: GPUBuffer | null = null
  private edgeVC = 0
  showEdges = false

  /* render mode */
  renderMode: 'solid' | 'solid+edges' | 'wireframe' | 'xray' | 'hidden-line' = 'solid'

  /* flat shading */
  flatShading = false

  /* grid toggle */
  showGrid = true

  /* auto-rotate */
  autoRotate = false

  /* lighting preset */
  lightingPreset = 'default'

  /* clipping plane */
  clipEnabled = false
  clipAxis = 1 // 0=X, 1=Y, 2=Z
  private clipValue = 0

  /** @deprecated Use clipValue via setClipValue(). Kept for backward compat. */
  get clipY(): number { return this.clipValue }
  set clipY(v: number) { this.clipValue = v }

  /* fog */
  fogEnabled = false

  /* reflection */
  showReflection = false

  /* SSAO */
  ssao = false

  /* outline */
  showOutline = false

  /* normal smoothing */
  normalSmoothing = false

  /* skybox */
  skyPreset = 'none'

  /* toon/cel shading */
  toonShading = false

  /* gooch shading */
  goochShading = false

  /* ground shadow */
  showGroundShadow = false

  /* section box (3-axis clip) */
  sectionBoxEnabled = false
  sectionBoxX = 0
  sectionBoxY = 0
  sectionBoxZ = 0

  /* per-mesh color overrides */
  meshColorOverrides: Map<number, [number, number, number, number]> = new Map()

  /* per-mesh visibility */
  meshVisibility: boolean[] = []

  /* exploded view */
  explodeFactor = 0

  /* field of view (radians, default PI/4 = 45deg) */
  fov = Math.PI / 4

  /* fly camera mode */
  flyMode = false

  /* orbit inertia */
  inertiaEnabled = false
  private inertiaYawVel = 0
  private inertiaPitchVel = 0
  private inertiaActive = false
  private lastMoveTime = 0

  /* orthographic projection */
  orthographic = false

  /* clear color (viewport background) */
  private clearR = 0.09
  private clearG = 0.09
  private clearB = 0.11
  private clearA = 1

  yaw = 0.6; pitch = 0.4; dist = 50
  tx = 0; ty = 0; tz = 0

  /* ── Camera animation state ── */
  private animating = false
  private animStartTime = 0
  private animDuration = 300 // ms
  private animStartYaw = 0
  private animStartPitch = 0
  private animStartDist = 0
  private animStartTx = 0
  private animStartTy = 0
  private animStartTz = 0
  private animTargetYaw = 0
  private animTargetPitch = 0
  private animTargetDist = 0
  private animTargetTx = 0
  private animTargetTy = 0
  private animTargetTz = 0

  /* ── FPS tracking ── */
  private frameTimes: number[] = []
  private lastFrameTime = 0
  private currentFPS = 0

  /* ── Bounding box cache ── */
  private boundsMin: [number, number, number] = [0, 0, 0]
  private boundsMax: [number, number, number] = [0, 0, 0]
  private totalVertexCount = 0

  private raf = 0
  private dead = false
  private dirty = true
  private drag = false; private pan = false
  private snapOrbit = false
  private mx = 0; private my = 0

  /* ── Resize observer ── */
  private resizeObserver: ResizeObserver | null = null
  private resizeDebounceTimer: ReturnType<typeof setTimeout> | null = null

  /* ── Device lost callback ── */
  onDeviceLost: ((reason: string) => void) | null = null

  /* ── Adapter info cache ── */
  private adapterInfoCache: GPUAdapterInfo | null = null

  /* ── Touch state ── */
  private touchIds: number[] = []
  private touchStartDist = 0
  private touchStartMidX = 0
  private touchStartMidY = 0
  private touchStartDist0 = 0
  private touchMode: 'none' | 'rotate' | 'pinch' = 'none'
  private lastTouchX = 0
  private lastTouchY = 0

  async init(canvas: HTMLCanvasElement): Promise<boolean> {
    this.canvas = canvas
    if (!navigator.gpu) return false

    // Acquire adapter + device. Any rejection here is treated as an init
    // failure (returns false), matching the other failure paths above.
    try {
      const adapter = await navigator.gpu.requestAdapter()
      if (!adapter) return false

      // Cache adapter info for perf panel
      try {
        this.adapterInfoCache = adapter.info
      } catch {
        this.adapterInfoCache = null
      }

      this.dev = await adapter.requestDevice()
    } catch (err) {
      console.error('[WebGPU] Failed to acquire device:', err)
      return false
    }

    this.ctx = canvas.getContext('webgpu') as GPUCanvasContext
    this.fmt = navigator.gpu.getPreferredCanvasFormat()
    this.ctx.configure({ device: this.dev, format: this.fmt, alphaMode: 'premultiplied' })

    // Handle device lost. The `lost` promise also resolves when we call
    // device.destroy() ourselves (reason === 'destroyed'); in that case this
    // is an intentional teardown, not a real loss. In all cases stop the loop
    // (mark dead + cancel any pending frame) so we never issue GPU work on a
    // dead device; only surface a real loss to the user callback.
    this.dev.lost.then((info) => {
      console.error('[WebGPU] Device lost:', info.reason, info.message)
      const wasDead = this.dead
      this.dead = true
      if (this.raf) {
        cancelAnimationFrame(this.raf)
        this.raf = 0
      }
      if (info.reason !== 'destroyed' && !wasDead && this.onDeviceLost) {
        this.onDeviceLost(info.reason + (info.message ? ': ' + info.message : ''))
      }
    })

    this.buildPipelines()
    this.buildSceneUB()
    this.buildGrid()
    this.resize()
    this.bindInput()
    this.setupResizeObserver()
    this.dirty = true
    this.loop()
    return true
  }

  private buildPipelines() {
    this.sceneBGL = this.dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })
    this.objBGL = this.dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })

    const meshMod = this.dev.createShaderModule({ code: MESH_WGSL })
    const meshLayout = this.dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL] })

    const vbl: GPUVertexBufferLayout = {
      arrayStride: 24,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x3' },
      ],
    }
    const ds: GPUDepthStencilState = { format: 'depth24plus', depthWriteEnabled: true, depthCompare: 'less' }

    this.meshPipe = this.dev.createRenderPipeline({
      layout: meshLayout,
      vertex: { module: meshMod, entryPoint: 'vs', buffers: [vbl] },
      fragment: { module: meshMod, entryPoint: 'fs', targets: [{ format: this.fmt }] },
      primitive: { topology: 'triangle-list', cullMode: 'none' },
      depthStencil: ds,
      multisample: { count: 4 },
    })

    this.meshPipeT = this.dev.createRenderPipeline({
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
      multisample: { count: 4 },
    })

    const lineMod = this.dev.createShaderModule({ code: LINE_WGSL })
    const lineLayout = this.dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL] })
    const lineVBL: GPUVertexBufferLayout = {
      arrayStride: 28,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },
        { shaderLocation: 1, offset: 12, format: 'float32x4' },
      ],
    }
    this.linePipe = this.dev.createRenderPipeline({
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
      multisample: { count: 4 },
    })

    // Outline pipeline (same layout as mesh, but cull front faces)
    const outlineMod = this.dev.createShaderModule({ code: OUTLINE_WGSL })
    this.outlinePipe = this.dev.createRenderPipeline({
      layout: meshLayout,
      vertex: { module: outlineMod, entryPoint: 'vs', buffers: [vbl] },
      fragment: { module: outlineMod, entryPoint: 'fs', targets: [{ format: this.fmt }] },
      primitive: { topology: 'triangle-list', cullMode: 'front' },
      depthStencil: ds,
      multisample: { count: 4 },
    })

    // Sky pipeline
    this.skyBGL = this.dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ] })
    const skyMod = this.dev.createShaderModule({ code: SKY_WGSL })
    const skyLayout = this.dev.createPipelineLayout({ bindGroupLayouts: [this.skyBGL] })
    this.skyPipe = this.dev.createRenderPipeline({
      layout: skyLayout,
      vertex: { module: skyMod, entryPoint: 'vs', buffers: [] },
      fragment: { module: skyMod, entryPoint: 'fs', targets: [{ format: this.fmt }] },
      primitive: { topology: 'triangle-list' },
      depthStencil: { format: 'depth24plus', depthWriteEnabled: false, depthCompare: 'always' },
      multisample: { count: 4 },
    })
  }

  private buildSceneUB() {
    this.sceneUB = this.dev.createBuffer({ size: 208, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.sceneBG = this.dev.createBindGroup({
      layout: this.sceneBGL,
      entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
    })
    // Reusable per-object UB and bind group for reflections
    this.reflUB = this.dev.createBuffer({ size: 144, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.reflBG = this.dev.createBindGroup({
      layout: this.objBGL,
      entries: [{ binding: 0, resource: { buffer: this.reflUB } }],
    })

    // Sky uniform buffer (2 x vec4f = 32 bytes)
    this.skyUB = this.dev.createBuffer({ size: 32, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.skyBG = this.dev.createBindGroup({
      layout: this.skyBGL!,
      entries: [{ binding: 0, resource: { buffer: this.skyUB } }],
    })
  }

  private buildGrid() {
    const d: number[] = []
    const gs = 200, step = 10
    const gc = [0.35, 0.35, 0.35, 0.4]
    const xc = [0.85, 0.2, 0.2, 0.8]
    const zc = [0.2, 0.2, 0.85, 0.8]
    const yc = [0.2, 0.85, 0.2, 0.8]

    for (let i = -gs; i <= gs; i += step) {
      const c = i === 0 ? xc : gc
      d.push(i,0,-gs,...c, i,0,gs,...c)
      const c2 = i === 0 ? zc : gc
      d.push(-gs,0,i,...c2, gs,0,i,...c2)
    }
    d.push(0,0,0,...yc, 0,gs,0,...yc)

    this.gridVC = d.length / 7
    this.gridVB = this.dev.createBuffer({ size: d.length * 4, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    this.dev.queue.writeBuffer(this.gridVB, 0, new Float32Array(d))
  }

  /**
   * Configure & (re)build the print bed outline. Draws a rectangle on the
   * ground plane (Y=0) of size x*z centered on the origin, plus a few border
   * ticks. Uses the existing LINE pipeline (pos(3)+color(4), stride 28).
   */
  setBuildPlate(enabled: boolean, x?: number, z?: number) {
    this.showBuildPlate = enabled
    if (x !== undefined && x > 0) this.buildPlateX = x
    if (z !== undefined && z > 0) this.buildPlateZ = z
    this.buildPlateBuffer()
    this.requestRender()
  }

  private buildPlateBuffer() {
    this.plateVB?.destroy()
    this.plateVB = null
    this.plateVC = 0
    if (!this.showBuildPlate) return

    const hx = this.buildPlateX / 2
    const hz = this.buildPlateZ / 2
    const d: number[] = []
    const main = [0.30, 0.62, 1.0, 0.9]   // bed outline (accent blue)
    const tick = [0.30, 0.62, 1.0, 0.45]  // border ticks

    // Rectangle outline (4 edges).
    const corners: [number, number][] = [
      [-hx, -hz], [hx, -hz], [hx, hz], [-hx, hz],
    ]
    for (let i = 0; i < 4; i++) {
      const a = corners[i], b = corners[(i + 1) % 4]
      d.push(a[0], 0.02, a[1], ...main)
      d.push(b[0], 0.02, b[1], ...main)
    }

    // Border ticks every 10mm along each edge, pointing slightly inward.
    const tickLen = Math.max(2, Math.min(hx, hz) * 0.04)
    const tickStep = 10
    for (let x = -hx + tickStep; x < hx; x += tickStep) {
      // bottom & top edges
      d.push(x, 0.02, -hz, ...tick); d.push(x, 0.02, -hz + tickLen, ...tick)
      d.push(x, 0.02, hz, ...tick);  d.push(x, 0.02, hz - tickLen, ...tick)
    }
    for (let z = -hz + tickStep; z < hz; z += tickStep) {
      // left & right edges
      d.push(-hx, 0.02, z, ...tick); d.push(-hx + tickLen, 0.02, z, ...tick)
      d.push(hx, 0.02, z, ...tick);  d.push(hx - tickLen, 0.02, z, ...tick)
    }

    this.plateVC = d.length / 7
    this.plateVB = this.dev.createBuffer({ size: d.length * 4, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    this.dev.queue.writeBuffer(this.plateVB, 0, new Float32Array(d))
  }

  setMeshes(meshes: MeshData[]) {
    this.lastRawMeshes = meshes

    // Apply normal smoothing if enabled
    if (this.normalSmoothing) {
      meshes = this.smoothNormals(meshes)
    }

    for (const g of this.meshes) { g.vb.destroy(); g.ib.destroy(); g.ub.destroy() }
    this.meshes = []

    for (const m of meshes) {
      const vb = this.dev.createBuffer({ size: m.vertices.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
      this.dev.queue.writeBuffer(vb, 0, m.vertices)
      const ib = this.dev.createBuffer({ size: m.indices.byteLength, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST })
      this.dev.queue.writeBuffer(ib, 0, m.indices)
      const ub = this.dev.createBuffer({ size: 144, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
      this.dev.queue.writeBuffer(ub, 0, transpose(m.transform))
      const nm = transpose(invert(m.transform))
      this.dev.queue.writeBuffer(ub, 64, transpose(nm))

      // Apply per-object color override if present, else xray / original color
      const meshIdx = this.meshes.length
      const colorOverride = this.meshColorOverrides.get(meshIdx)
      const color = colorOverride
        ? colorOverride
        : this.renderMode === 'xray'
          ? [m.color[0], m.color[1], m.color[2], 0.3]
          : m.color
      this.dev.queue.writeBuffer(ub, 128, new Float32Array(color))

      const bg = this.dev.createBindGroup({
        layout: this.objBGL,
        entries: [{ binding: 0, resource: { buffer: ub } }],
      })
      const transp = colorOverride
        ? colorOverride[3] < 0.99
        : this.renderMode === 'xray' ? true : m.color[3] < 0.99
      this.meshes.push({ vb, ib, ic: m.indices.length, ub, bg, transp })
    }
    this.meshVisibility = new Array(this.meshes.length).fill(true)
    this.computeBounds(meshes)
    this.autoFit(meshes)
    if (this.wireframe) this.buildWireframeBuffer()
    if (this.showEdges || this.renderMode === 'hidden-line') this.buildEdgeBuffer()
    this.requestRender()
  }

  private computeBounds(meshes: MeshData[]) {
    let mnx = Infinity, mny = Infinity, mnz = Infinity
    let mxx = -Infinity, mxy = -Infinity, mxz = -Infinity
    let totalVerts = 0
    for (const m of meshes) {
      const t = m.transform
      totalVerts += m.vertices.length / 6
      for (let i = 0; i < m.vertices.length; i += 6) {
        const x = m.vertices[i], y = m.vertices[i + 1], z = m.vertices[i + 2]
        const px = t[0] * x + t[1] * y + t[2] * z + t[3]
        const py = t[4] * x + t[5] * y + t[6] * z + t[7]
        const pz = t[8] * x + t[9] * y + t[10] * z + t[11]
        // Skip non-finite coords (e.g. a model dimension from a /0 expression)
        // so a single bad mesh can't poison the whole scene's bounds with NaN.
        if (!Number.isFinite(px) || !Number.isFinite(py) || !Number.isFinite(pz)) continue
        mnx = Math.min(mnx, px); mxx = Math.max(mxx, px)
        mny = Math.min(mny, py); mxy = Math.max(mxy, py)
        mnz = Math.min(mnz, pz); mxz = Math.max(mxz, pz)
      }
    }
    if (!meshes.length || !Number.isFinite(mnx)) {
      this.boundsMin = [0, 0, 0]
      this.boundsMax = [0, 0, 0]
    } else {
      this.boundsMin = [mnx, mny, mnz]
      this.boundsMax = [mxx, mxy, mxz]
    }
    this.totalVertexCount = totalVerts
  }

  private autoFit(meshes: MeshData[]) {
    if (!meshes.length) return
    let mnx = Infinity, mny = Infinity, mnz = Infinity
    let mxx = -Infinity, mxy = -Infinity, mxz = -Infinity
    for (const m of meshes) {
      const t = m.transform
      for (let i = 0; i < m.vertices.length; i += 6) {
        const x = m.vertices[i], y = m.vertices[i+1], z = m.vertices[i+2]
        const px = t[0]*x+t[1]*y+t[2]*z+t[3]
        const py = t[4]*x+t[5]*y+t[6]*z+t[7]
        const pz = t[8]*x+t[9]*y+t[10]*z+t[11]
        if (!Number.isFinite(px) || !Number.isFinite(py) || !Number.isFinite(pz)) continue
        mnx = Math.min(mnx,px); mxx = Math.max(mxx,px)
        mny = Math.min(mny,py); mxy = Math.max(mxy,py)
        mnz = Math.min(mnz,pz); mxz = Math.max(mxz,pz)
      }
    }
    // If no finite vertices were found, keep the current camera rather than
    // setting NaN target/distance (which would break the whole viewport).
    if (!Number.isFinite(mnx)) return
    this.tx = (mnx+mxx)/2; this.ty = (mny+mxy)/2; this.tz = (mnz+mxz)/2
    this.dist = Math.max(Math.max(mxx-mnx, mxy-mny, mxz-mnz) * 1.8, 5)
  }

  resize() {
    const dpr = devicePixelRatio || 1
    const w = this.canvas.clientWidth * dpr | 0
    const h = this.canvas.clientHeight * dpr | 0
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w; this.canvas.height = h
      this.depth?.destroy()
      this.depth = this.dev.createTexture({ size: [w, h], format: 'depth24plus', usage: GPUTextureUsage.RENDER_ATTACHMENT })
      // MSAA textures
      this.msaaColorTex?.destroy()
      this.msaaColorTex = this.dev.createTexture({
        size: [w, h], format: this.fmt,
        sampleCount: 4, usage: GPUTextureUsage.RENDER_ATTACHMENT,
      })
      this.msaaDepthTex?.destroy()
      this.msaaDepthTex = this.dev.createTexture({
        size: [w, h], format: 'depth24plus',
        sampleCount: 4, usage: GPUTextureUsage.RENDER_ATTACHMENT,
      })
    }
  }

  /* cached view-projection for screen projection */
  private lastVP: Mat4 = new Float32Array(16)
  private lastW = 1
  private lastH = 1

  /** Fill a 48-float scene-data array with current camera/lighting/clip state. */
  private fillSceneData(sd: Float32Array, cx: number, cy: number, cz: number) {
    sd.set([cx, cy, cz, 1], 16)
    const lp = this.getLightingValues()
    sd.set(lp.light, 20)
    sd.set(lp.ambient, 24)
    // Clipping plane: [clipValue, enabled, axis, 0]
    sd.set([this.clipValue, this.clipEnabled ? 1.0 : 0.0, this.clipAxis, 0], 28)
    // Fog
    const fogNear = this.dist * 0.5
    const fogFar = this.dist * 3.0
    sd.set([fogNear, fogFar, this.fogEnabled ? 1.0 : 0.0, 0], 32)
    sd.set([this.clearR, this.clearG, this.clearB, 1], 36)
    // Flat shading flag in _pad0.x
    sd[40] = this.flatShading ? 1.0 : 0.0
    // SSAO flag in _pad0.y
    sd[41] = this.ssao ? 1.0 : 0.0
    // Gooch shading flag in _pad0.z
    sd[42] = this.goochShading ? 1.0 : 0.0
    // Toon shading flag in _pad1.x
    sd[44] = this.toonShading ? 1.0 : 0.0
    // Ground shadow flag in _pad1.y
    sd[45] = this.showGroundShadow ? 1.0 : 0.0
    // Section box: [clipX, clipY, clipZ, enabled]
    sd[48] = this.sectionBoxX
    sd[49] = this.sectionBoxY
    sd[50] = this.sectionBoxZ
    sd[51] = this.sectionBoxEnabled ? 1.0 : 0.0
  }

  private render() {
    if (this.dead) return
    this.resize()
    const w = this.canvas.width, h = this.canvas.height, asp = w / h
    const cx = this.tx + this.dist * Math.cos(this.pitch) * Math.sin(this.yaw)
    const cy = this.ty + this.dist * Math.sin(this.pitch)
    const cz = this.tz + this.dist * Math.cos(this.pitch) * Math.cos(this.yaw)
    const view = lookAt([cx,cy,cz], [this.tx,this.ty,this.tz], [0,1,0])
    let proj: Mat4
    if (this.orthographic) {
      const halfH = this.dist * Math.tan(this.fov / 2)
      const halfW = halfH * asp
      proj = ortho(-halfW, halfW, -halfH, halfH, 0.1, this.dist * 10)
    } else {
      proj = perspective(this.fov, asp, 0.1, this.dist * 10)
    }
    const vpMat = multiply(proj, view)
    this.lastVP = vpMat
    this.lastW = w
    this.lastH = h
    const vp = transpose(vpMat)
    const sd = new Float32Array(52)
    sd.set(vp, 0)
    this.fillSceneData(sd, cx, cy, cz)
    this.dev.queue.writeBuffer(this.sceneUB, 0, sd)

    const msaaView = this.msaaColorTex.createView()
    const msaaDepthView = this.msaaDepthTex.createView()
    const resolveView = this.ctx.getCurrentTexture().createView()

    const enc = this.dev.createCommandEncoder()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: msaaView,
        resolveTarget: resolveView,
        clearValue: { r: this.clearR * this.clearA, g: this.clearG * this.clearA, b: this.clearB * this.clearA, a: this.clearA },
        loadOp: 'clear', storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: msaaDepthView,
        depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
      },
    })

    if (this.showGrid && this.gridVB) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.gridVB)
      pass.draw(this.gridVC)
    }

    if (this.showBuildPlate && this.plateVB && this.plateVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.plateVB)
      pass.draw(this.plateVC)
    }

    // Sky background
    if (this.skyPreset !== 'none' && this.skyPipe && this.skyBG) {
      pass.setPipeline(this.skyPipe)
      pass.setBindGroup(0, this.skyBG)
      pass.draw(3)
    }

    // Exploded view: offset each mesh's model matrix away from scene center
    if (this.explodeFactor > 0 && this.lastRawMeshes.length > 0 && this.meshes.length > 0) {
      const scx = (this.boundsMin[0] + this.boundsMax[0]) / 2
      const scy = (this.boundsMin[1] + this.boundsMax[1]) / 2
      const scz = (this.boundsMin[2] + this.boundsMax[2]) / 2
      for (let mi = 0; mi < this.meshes.length && mi < this.lastRawMeshes.length; mi++) {
        const rm = this.lastRawMeshes[mi]
        const tt = rm.transform
        let mnx2 = Infinity, mny2 = Infinity, mnz2 = Infinity
        let mxx2 = -Infinity, mxy2 = -Infinity, mxz2 = -Infinity
        for (let i = 0; i < rm.vertices.length; i += 6) {
          const x = rm.vertices[i], y = rm.vertices[i+1], z = rm.vertices[i+2]
          const px = tt[0]*x+tt[1]*y+tt[2]*z+tt[3]
          const py = tt[4]*x+tt[5]*y+tt[6]*z+tt[7]
          const pz = tt[8]*x+tt[9]*y+tt[10]*z+tt[11]
          mnx2 = Math.min(mnx2,px); mxx2 = Math.max(mxx2,px)
          mny2 = Math.min(mny2,py); mxy2 = Math.max(mxy2,py)
          mnz2 = Math.min(mnz2,pz); mxz2 = Math.max(mxz2,pz)
        }
        const mcx = (mnx2+mxx2)/2, mcy = (mny2+mxy2)/2, mcz = (mnz2+mxz2)/2
        let edx = mcx - scx, edy = mcy - scy, edz = mcz - scz
        const elen = Math.sqrt(edx*edx + edy*edy + edz*edz)
        if (elen > 0.001) { edx /= elen; edy /= elen; edz /= elen }
        else { edx = 0; edy = 1; edz = 0 }
        const escale = Math.max(this.boundsMax[0]-this.boundsMin[0], this.boundsMax[1]-this.boundsMin[1], this.boundsMax[2]-this.boundsMin[2])
        const eoff = escale * this.explodeFactor
        const explT = new Float32Array(tt)
        explT[3] += edx * eoff; explT[7] += edy * eoff; explT[11] += edz * eoff
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 0, transpose(explT))
        const enm = transpose(invert(explT))
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 64, transpose(enm))
      }
    }

    // Outline pass (also enable when toon shading is active for thick outlines)
    if ((this.showOutline || this.toonShading) && this.renderMode !== 'wireframe') {
      pass.setPipeline(this.outlinePipe)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        if (g.transp) continue
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    }

    // Render mode dispatch
    if (this.renderMode === 'wireframe') {
      // wireframe only — skip solid meshes
    } else if (this.renderMode === 'hidden-line') {
      // hidden-line: render meshes in background color to occlude, then edges on top
      pass.setPipeline(this.meshPipe)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        // Temporarily write clear color to mesh UB for occlusion
        this.dev.queue.writeBuffer(g.ub, 128, new Float32Array([this.clearR, this.clearG, this.clearB, 1.0]))
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    } else if (this.renderMode === 'xray') {
      // xray: render ALL meshes with meshPipeT (alpha already set to 0.3 in UB)
      pass.setPipeline(this.meshPipeT)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    } else {
      // solid / solid+edges: normal opaque then transparent
      pass.setPipeline(this.meshPipe)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        if (g.transp) continue
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }

      pass.setPipeline(this.meshPipeT)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        if (!g.transp) continue
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    }

    /* edge overlay (CAD-style) */
    if (this.showEdges && this.edgeVB && this.edgeVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.edgeVB)
      pass.draw(this.edgeVC)
    }

    /* wireframe edge overlay */
    if (this.wireframe && this.wireframeVB && this.wireframeVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.wireframeVB)
      pass.draw(this.wireframeVC)
    }

    pass.end()

    // Reflection pass - render mirrored meshes with low alpha
    if (this.showReflection && this.meshes.length > 0 && this.renderMode !== 'wireframe') {
      const reflPass = enc.beginRenderPass({
        colorAttachments: [{
          view: msaaView,
          resolveTarget: resolveView,
          loadOp: 'load', storeOp: 'store',
        }],
        depthStencilAttachment: {
          view: msaaDepthView,
          depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
        },
      })

      // Use transparent pipeline for reflections
      reflPass.setPipeline(this.meshPipeT)
      reflPass.setBindGroup(0, this.sceneBG)

      for (let mi = 0; mi < this.meshes.length; mi++) {
        if (this.meshVisibility[mi] === false) continue
        const g = this.meshes[mi]
        if (g.transp && this.renderMode !== 'xray') continue
        const m = this.lastRawMeshes[mi]
        if (!m) continue

        // Mirror the model matrix: scale Y by -1
        const mirrorY: Mat4 = new Float32Array([
          1, 0, 0, 0,
          0,-1, 0, 0,
          0, 0, 1, 0,
          0, 0, 0, 1,
        ])
        const mirroredModel = multiply(mirrorY, m.transform)
        const mirroredNormal = transpose(invert(mirroredModel))
        this.dev.queue.writeBuffer(this.reflUB, 0, transpose(mirroredModel))
        this.dev.queue.writeBuffer(this.reflUB, 64, transpose(mirroredNormal))
        this.dev.queue.writeBuffer(this.reflUB, 128, new Float32Array([m.color[0], m.color[1], m.color[2], 0.15]))
        reflPass.setBindGroup(1, this.reflBG)
        reflPass.setVertexBuffer(0, g.vb)
        reflPass.setIndexBuffer(g.ib, 'uint32')
        reflPass.drawIndexed(g.ic)
      }

      reflPass.end()
    }

    // Ground shadow pass - render flattened meshes as dark semi-transparent
    if (this.showGroundShadow && this.meshes.length > 0 && this.renderMode !== 'wireframe') {
      const shadowPass = enc.beginRenderPass({
        colorAttachments: [{
          view: msaaView,
          resolveTarget: resolveView,
          loadOp: 'load', storeOp: 'store',
        }],
        depthStencilAttachment: {
          view: msaaDepthView,
          depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
        },
      })

      shadowPass.setPipeline(this.meshPipeT)
      shadowPass.setBindGroup(0, this.sceneBG)

      for (let mi = 0; mi < this.meshes.length; mi++) {
        if (this.meshVisibility[mi] === false) continue
        const g = this.meshes[mi]
        if (g.transp && this.renderMode !== 'xray') continue
        const m = this.lastRawMeshes[mi]
        if (!m) continue

        // Flatten Y to ground (Y=0.01)
        const flattenY: Mat4 = new Float32Array([
          1, 0, 0, 0,
          0, 0, 0, 0,
          0, 0, 1, 0,
          0, 0.01, 0, 1,
        ])
        const flatModel = multiply(flattenY, m.transform)
        const flatNormal = transpose(invert(flatModel))
        this.dev.queue.writeBuffer(this.reflUB, 0, transpose(flatModel))
        this.dev.queue.writeBuffer(this.reflUB, 64, transpose(flatNormal))
        this.dev.queue.writeBuffer(this.reflUB, 128, new Float32Array([0.0, 0.0, 0.0, 0.25]))
        shadowPass.setBindGroup(1, this.reflBG)
        shadowPass.setVertexBuffer(0, g.vb)
        shadowPass.setIndexBuffer(g.ib, 'uint32')
        shadowPass.drawIndexed(g.ic)
      }

      shadowPass.end()
    }

    this.dev.queue.submit([enc.finish()])

    // Restore original colors after hidden-line render
    if (this.renderMode === 'hidden-line' && this.lastRawMeshes.length > 0) {
      for (let i = 0; i < this.meshes.length && i < this.lastRawMeshes.length; i++) {
        const raw = this.lastRawMeshes[i]
        this.dev.queue.writeBuffer(this.meshes[i].ub, 128, new Float32Array(raw.color))
      }
    }

    // Restore original model matrices after exploded view
    if (this.explodeFactor > 0 && this.lastRawMeshes.length > 0 && this.meshes.length > 0) {
      for (let mi = 0; mi < this.meshes.length && mi < this.lastRawMeshes.length; mi++) {
        const rm = this.lastRawMeshes[mi]
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 0, transpose(rm.transform))
        const nm = transpose(invert(rm.transform))
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 64, transpose(nm))
      }
    }
  }

  private getLightingValues(): { light: [number, number, number, number]; ambient: [number, number, number, number] } {
    switch (this.lightingPreset) {
      case 'studio':
        return { light: [0.6, 0.8, 0.3, 0], ambient: [0.30, 0.28, 0.32, 1] }
      case 'outdoor':
        return { light: [0.4, 0.9, 0.3, 0], ambient: [0.18, 0.22, 0.35, 1] }
      case 'dramatic':
        return { light: [0.8, 0.5, 0.1, 0], ambient: [0.08, 0.08, 0.10, 1] }
      case 'soft':
        return { light: [0.3, 0.6, 0.5, 0], ambient: [0.40, 0.38, 0.42, 1] }
      default: // 'default'
        return { light: [0.55, 0.75, 0.45, 0], ambient: [0.22, 0.22, 0.24, 1] }
    }
  }

  private easeInOutCubic(t: number): number {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
  }

  private updateAnimation(now: number) {
    if (!this.animating) return
    const elapsed = now - this.animStartTime
    const t = Math.min(elapsed / this.animDuration, 1)
    const e = this.easeInOutCubic(t)

    // Interpolate yaw along the shortest angular path so we never spin the
    // long way around ±π (e.g. 0.6 → π). Pitch is clamped and does not wrap.
    const dYaw = Math.atan2(
      Math.sin(this.animTargetYaw - this.animStartYaw),
      Math.cos(this.animTargetYaw - this.animStartYaw),
    )
    this.yaw = this.animStartYaw + dYaw * e
    this.pitch = this.animStartPitch + (this.animTargetPitch - this.animStartPitch) * e
    this.dist = this.animStartDist + (this.animTargetDist - this.animStartDist) * e
    this.tx = this.animStartTx + (this.animTargetTx - this.animStartTx) * e
    this.ty = this.animStartTy + (this.animTargetTy - this.animStartTy) * e
    this.tz = this.animStartTz + (this.animTargetTz - this.animStartTz) * e

    if (t >= 1) this.animating = false
  }

  /** Mark the scene as needing a repaint. Schedules exactly one rAF. */
  requestRender() {
    if (this.dead) return
    this.dirty = true
    if (!this.raf) {
      this.raf = requestAnimationFrame(this.loop)
    }
  }

  private setupResizeObserver() {
    this.resizeObserver = new ResizeObserver(() => {
      if (this.resizeDebounceTimer) clearTimeout(this.resizeDebounceTimer)
      this.resizeDebounceTimer = setTimeout(() => {
        this.resize()
        this.requestRender()
      }, 100)
    })
    this.resizeObserver.observe(this.canvas)
  }

  private loop = () => {
    if (this.dead) return
    this.raf = 0
    const now = performance.now()

    // Camera animation
    const wasAnimating = this.animating
    this.updateAnimation(now)
    if (this.animating || wasAnimating) this.dirty = true

    // Auto-rotate
    if (this.autoRotate && !this.animating) {
      this.yaw += 0.005
      this.dirty = true
    }

    // Orbit inertia
    if (this.inertiaActive && !this.drag) {
      this.yaw += this.inertiaYawVel
      this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + this.inertiaPitchVel))
      this.inertiaYawVel *= 0.95
      this.inertiaPitchVel *= 0.95
      if (Math.abs(this.inertiaYawVel) < 0.0001 && Math.abs(this.inertiaPitchVel) < 0.0001) {
        this.inertiaActive = false
        this.inertiaYawVel = 0
        this.inertiaPitchVel = 0
      } else {
        this.dirty = true
      }
    }

    // Only render if something changed
    if (this.dirty) {
      this.dirty = false
      this.render()

      // FPS tracking — measure time between actual renders
      if (this.lastFrameTime > 0) {
        this.frameTimes.push(now - this.lastFrameTime)
        if (this.frameTimes.length > 60) this.frameTimes.shift()
        const avg = this.frameTimes.reduce((a, b) => a + b, 0) / this.frameTimes.length
        this.currentFPS = avg > 0 ? Math.round(1000 / avg) : 0
      }
      this.lastFrameTime = now
    }

    // Keep the loop running while auto-rotate, animating, or inertia active; otherwise stop
    if (this.autoRotate || this.animating || this.inertiaActive) {
      this.raf = requestAnimationFrame(this.loop)
    }
  }

  private onDown = (e: PointerEvent) => {
    this.drag = true; this.pan = e.button === 2 || e.shiftKey
    // Ctrl (without Shift) during an orbit drag = snap yaw/pitch to 15 deg steps.
    this.snapOrbit = !this.pan && e.ctrlKey
    this.mx = e.clientX; this.my = e.clientY
    this.canvas.setPointerCapture(e.pointerId)
    this.requestRender()
  }
  private onMove = (e: PointerEvent) => {
    if (!this.drag) return
    const dx = e.clientX - this.mx, dy = e.clientY - this.my
    this.mx = e.clientX; this.my = e.clientY
    // Allow toggling Ctrl mid-drag for orbit snapping.
    if (!this.pan) this.snapOrbit = e.ctrlKey
    if (this.flyMode && !this.pan) {
      // Fly mode: mouse drag rotates the view direction, keeping camera position fixed
      this.yaw -= dx * 0.003
      this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + dy * 0.003))
    } else if (this.pan) {
      const sp = this.dist * 0.002
      const cy = Math.cos(this.yaw), sy = Math.sin(this.yaw)
      this.tx -= dx * cy * sp; this.tz += dx * sy * sp; this.ty += dy * sp
    } else {
      this.yaw -= dx * 0.005
      this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + dy * 0.005))
      if (this.snapOrbit) {
        // Snap yaw & pitch to 15 deg (PI/12) increments while dragging.
        const step = Math.PI / 12
        this.yaw = Math.round(this.yaw / step) * step
        this.pitch = Math.max(-1.5, Math.min(1.5, Math.round(this.pitch / step) * step))
      }
      // Track velocity for inertia
      if (this.inertiaEnabled) {
        const now = performance.now()
        this.inertiaYawVel = -dx * 0.005
        this.inertiaPitchVel = dy * 0.005
        this.lastMoveTime = now
      }
    }
    this.requestRender()
  }
  private onUp = (e: PointerEvent) => {
    const wasDragging = this.drag && !this.pan
    this.drag = false; this.snapOrbit = false; this.canvas.releasePointerCapture(e.pointerId)
    // Start inertia animation on pointer up after orbiting
    if (wasDragging && this.inertiaEnabled) {
      const timeSinceMove = performance.now() - this.lastMoveTime
      if (timeSinceMove < 100 && (Math.abs(this.inertiaYawVel) > 0.0005 || Math.abs(this.inertiaPitchVel) > 0.0005)) {
        this.inertiaActive = true
        this.requestRender()
      } else {
        this.inertiaYawVel = 0
        this.inertiaPitchVel = 0
      }
    }
  }
  private onWheel = (e: WheelEvent) => {
    e.preventDefault()
    this.dist = Math.max(1, Math.min(50000, this.dist * (1 + e.deltaY * 0.001)))
    this.requestRender()
  }
  private noCtx = (e: Event) => e.preventDefault()

  /* ── Touch handlers ── */
  private getTouchById(touches: TouchList, id: number): Touch | null {
    for (let i = 0; i < touches.length; i++) {
      if (touches[i].identifier === id) return touches[i]
    }
    return null
  }

  private onTouchStart = (e: TouchEvent) => {
    e.preventDefault()
    const touches = e.touches
    if (touches.length === 1) {
      this.touchMode = 'rotate'
      this.lastTouchX = touches[0].clientX
      this.lastTouchY = touches[0].clientY
      this.touchIds = [touches[0].identifier]
    } else if (touches.length >= 2) {
      this.touchMode = 'pinch'
      this.touchIds = [touches[0].identifier, touches[1].identifier]
      const dx = touches[1].clientX - touches[0].clientX
      const dy = touches[1].clientY - touches[0].clientY
      this.touchStartDist = Math.sqrt(dx * dx + dy * dy)
      this.touchStartDist0 = this.dist
      this.touchStartMidX = (touches[0].clientX + touches[1].clientX) / 2
      this.touchStartMidY = (touches[0].clientY + touches[1].clientY) / 2
      this.lastTouchX = this.touchStartMidX
      this.lastTouchY = this.touchStartMidY
    }
    this.requestRender()
  }

  private onTouchMove = (e: TouchEvent) => {
    e.preventDefault()
    const touches = e.touches
    if (this.touchMode === 'rotate' && touches.length === 1) {
      const t = touches[0]
      const dx = t.clientX - this.lastTouchX
      const dy = t.clientY - this.lastTouchY
      this.lastTouchX = t.clientX
      this.lastTouchY = t.clientY
      this.yaw -= dx * 0.005
      this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + dy * 0.005))
    } else if (this.touchMode === 'pinch' && touches.length >= 2) {
      const t0 = this.getTouchById(touches, this.touchIds[0])
      const t1 = this.getTouchById(touches, this.touchIds[1])
      if (!t0 || !t1) return

      // Pinch zoom
      const dx = t1.clientX - t0.clientX
      const dy = t1.clientY - t0.clientY
      const curDist = Math.sqrt(dx * dx + dy * dy)
      const ratio = this.touchStartDist / Math.max(curDist, 1)
      this.dist = Math.max(1, Math.min(50000, this.touchStartDist0 * ratio))

      // Two-finger pan
      const midX = (t0.clientX + t1.clientX) / 2
      const midY = (t0.clientY + t1.clientY) / 2
      const panDx = midX - this.lastTouchX
      const panDy = midY - this.lastTouchY
      this.lastTouchX = midX
      this.lastTouchY = midY
      const sp = this.dist * 0.002
      const cosY = Math.cos(this.yaw), sinY = Math.sin(this.yaw)
      this.tx -= panDx * cosY * sp
      this.tz += panDx * sinY * sp
      this.ty += panDy * sp
    }
    this.requestRender()
  }

  private onTouchEnd = (e: TouchEvent) => {
    e.preventDefault()
    const touches = e.touches
    if (touches.length === 0) {
      this.touchMode = 'none'
      this.touchIds = []
    } else if (touches.length === 1) {
      // Went from 2 fingers to 1, switch to rotate
      this.touchMode = 'rotate'
      this.lastTouchX = touches[0].clientX
      this.lastTouchY = touches[0].clientY
      this.touchIds = [touches[0].identifier]
    }
  }

  private bindInput() {
    const c = this.canvas
    c.addEventListener('pointerdown', this.onDown)
    c.addEventListener('pointermove', this.onMove)
    c.addEventListener('pointerup', this.onUp)
    c.addEventListener('wheel', this.onWheel, { passive: false })
    c.addEventListener('contextmenu', this.noCtx)

    // Touch events
    c.addEventListener('touchstart', this.onTouchStart, { passive: false })
    c.addEventListener('touchmove', this.onTouchMove, { passive: false })
    c.addEventListener('touchend', this.onTouchEnd, { passive: false })
  }

  /** Set camera yaw and pitch directly (e.g. for view presets). */
  setCamera(yaw: number, pitch: number) {
    this.yaw = yaw
    this.pitch = pitch
    this.requestRender()
  }

  /** Get current camera yaw/pitch (for the orientation gizmo). */
  getOrientation(): { yaw: number; pitch: number } {
    return { yaw: this.yaw, pitch: this.pitch }
  }

  /** Get full camera info (position, target, distance). */
  getCameraInfo(): { yaw: number; pitch: number; dist: number; tx: number; ty: number; tz: number } {
    return { yaw: this.yaw, pitch: this.pitch, dist: this.dist, tx: this.tx, ty: this.ty, tz: this.tz }
  }

  /** Unproject screen coordinates to a world-space ray using inverse VP matrix. */
  unproject(screenX: number, screenY: number): { origin: [number,number,number]; direction: [number,number,number] } {
    const invVP = invert(this.lastVP)
    // Convert screen coords to NDC
    const ndcX = (screenX / this.lastW) * 2 - 1
    const ndcY = 1 - (screenY / this.lastH) * 2  // flip Y

    // Unproject near point (NDC z = -1) and far point (NDC z = 1)
    function unproj4(nx: number, ny: number, nz: number, m: Float32Array): [number,number,number] {
      const x = m[0]*nx + m[1]*ny + m[2]*nz + m[3]
      const y = m[4]*nx + m[5]*ny + m[6]*nz + m[7]
      const z = m[8]*nx + m[9]*ny + m[10]*nz + m[11]
      const w = m[12]*nx + m[13]*ny + m[14]*nz + m[15]
      const iw = w !== 0 ? 1/w : 1
      return [x*iw, y*iw, z*iw]
    }

    const near = unproj4(ndcX, ndcY, -1, invVP)
    const far = unproj4(ndcX, ndcY, 1, invVP)

    const dx = far[0] - near[0]
    const dy = far[1] - near[1]
    const dz = far[2] - near[2]
    const len = Math.sqrt(dx*dx + dy*dy + dz*dz) || 1

    return {
      origin: near,
      direction: [dx/len, dy/len, dz/len],
    }
  }

  /**
   * Smoothly snap the camera to look straight down a principal axis.
   * axis: '+x' | '-x' | '+y' | '-y' | '+z' | '-z'
   */
  snapToAxis(axis: string) {
    const HALF = Math.PI / 2
    switch (axis) {
      case '+x': this.animateTo(HALF, 0); break
      case '-x': this.animateTo(-HALF, 0); break
      case '+y': this.animateTo(0, HALF); break
      case '-y': this.animateTo(0, -HALF); break
      case '+z': this.animateTo(0, 0); break
      case '-z': this.animateTo(Math.PI, 0); break
    }
  }

  /** Smoothly animate camera to a new position using easeInOutCubic. */
  animateTo(
    targetYaw: number,
    targetPitch: number,
    targetDist?: number,
    targetTx?: number,
    targetTy?: number,
    targetTz?: number,
  ) {
    this.animStartYaw = this.yaw
    this.animStartPitch = this.pitch
    this.animStartDist = this.dist
    this.animStartTx = this.tx
    this.animStartTy = this.ty
    this.animStartTz = this.tz

    this.animTargetYaw = targetYaw
    this.animTargetPitch = targetPitch
    this.animTargetDist = targetDist ?? this.dist
    this.animTargetTx = targetTx ?? this.tx
    this.animTargetTy = targetTy ?? this.ty
    this.animTargetTz = targetTz ?? this.tz

    this.animStartTime = performance.now()
    this.animating = true
    this.requestRender()
  }

  /** Get current FPS value. */
  getFPS(): number {
    return this.currentFPS
  }

  /** Get total vertex count across all meshes. */
  getVertexCount(): number {
    return this.totalVertexCount
  }

  /** Get scene bounding box info. */
  getBounds(): { min: [number, number, number]; max: [number, number, number]; size: [number, number, number] } {
    return {
      min: [...this.boundsMin],
      max: [...this.boundsMax],
      size: [
        this.boundsMax[0] - this.boundsMin[0],
        this.boundsMax[1] - this.boundsMin[1],
        this.boundsMax[2] - this.boundsMin[2],
      ],
    }
  }

  /** Public wrapper for autoFit -- re-fits camera to current meshes' bounding box. */
  autoFitAll() {
    if (this.lastRawMeshes && this.lastRawMeshes.length) {
      this.autoFit(this.lastRawMeshes)
      this.requestRender()
    }
  }

  /** Take a screenshot of the current canvas and download as PNG. */
  screenshot() {
    // Render one frame to ensure the canvas has content
    this.render()
    this.canvas.toBlob((blob) => {
      if (!blob) return
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `openscad-screenshot-${Date.now()}.png`
      a.click()
      URL.revokeObjectURL(url)
    }, 'image/png')
  }

  /**
   * Take a screenshot at scale x the current backing-store resolution.
   * Temporarily resizes the canvas backing store + depth texture, renders one
   * frame, captures via toBlob, then restores the original size and re-renders.
   */
  screenshotScaled(scale: number) {
    if (scale <= 1) { this.screenshot(); return }
    const origW = this.canvas.width
    const origH = this.canvas.height
    const origDepth = this.depth
    const origMsaaColor = this.msaaColorTex
    const origMsaaDepth = this.msaaDepthTex

    const w = Math.max(1, Math.round(origW * scale))
    const h = Math.max(1, Math.round(origH * scale))
    this.canvas.width = w
    this.canvas.height = h
    this.depth = this.dev.createTexture({ size: [w, h], format: 'depth24plus', usage: GPUTextureUsage.RENDER_ATTACHMENT })
    this.msaaColorTex = this.dev.createTexture({
      size: [w, h], format: this.fmt,
      sampleCount: 4, usage: GPUTextureUsage.RENDER_ATTACHMENT,
    })
    this.msaaDepthTex = this.dev.createTexture({
      size: [w, h], format: 'depth24plus',
      sampleCount: 4, usage: GPUTextureUsage.RENDER_ATTACHMENT,
    })

    // Render directly with the scaled buffers (bypass resize(), which would
    // snap the canvas back to its CSS size).
    this.renderScaled(w, h)

    const restore = () => {
      // Restore original backing store + depth + MSAA, then force a normal re-render.
      this.canvas.width = origW
      this.canvas.height = origH
      this.depth?.destroy()
      this.msaaColorTex?.destroy()
      this.msaaDepthTex?.destroy()
      this.depth = origDepth
      this.msaaColorTex = origMsaaColor
      this.msaaDepthTex = origMsaaDepth
      this.render()
    }

    this.canvas.toBlob((blob) => {
      if (blob) {
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `model@${scale}x.png`
        a.click()
        URL.revokeObjectURL(url)
      }
      restore()
    }, 'image/png')
  }

  /** Render a single frame using explicit pixel dimensions (for scaled capture). */
  private renderScaled(w: number, h: number) {
    const asp = w / h
    const cx = this.tx + this.dist * Math.cos(this.pitch) * Math.sin(this.yaw)
    const cy = this.ty + this.dist * Math.sin(this.pitch)
    const cz = this.tz + this.dist * Math.cos(this.pitch) * Math.cos(this.yaw)
    const view = lookAt([cx, cy, cz], [this.tx, this.ty, this.tz], [0, 1, 0])
    let proj: Mat4
    if (this.orthographic) {
      const halfH = this.dist * Math.tan(this.fov / 2)
      const halfW = halfH * asp
      proj = ortho(-halfW, halfW, -halfH, halfH, 0.1, this.dist * 10)
    } else {
      proj = perspective(this.fov, asp, 0.1, this.dist * 10)
    }
    const vpMat = multiply(proj, view)
    const vp = transpose(vpMat)
    const sd = new Float32Array(52)
    sd.set(vp, 0)
    this.fillSceneData(sd, cx, cy, cz)
    this.dev.queue.writeBuffer(this.sceneUB, 0, sd)

    const msaaView = this.msaaColorTex.createView()
    const msaaDepthView = this.msaaDepthTex.createView()
    const resolveView = this.ctx.getCurrentTexture().createView()

    const enc = this.dev.createCommandEncoder()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: msaaView,
        resolveTarget: resolveView,
        clearValue: { r: this.clearR * this.clearA, g: this.clearG * this.clearA, b: this.clearB * this.clearA, a: this.clearA },
        loadOp: 'clear', storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: msaaDepthView,
        depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
      },
    })

    if (this.showGrid && this.gridVB) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.gridVB)
      pass.draw(this.gridVC)
    }
    if (this.showBuildPlate && this.plateVB && this.plateVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.plateVB)
      pass.draw(this.plateVC)
    }

    // Sky background
    if (this.skyPreset !== 'none' && this.skyPipe && this.skyBG) {
      pass.setPipeline(this.skyPipe)
      pass.setBindGroup(0, this.skyBG)
      pass.draw(3)
    }

    // Exploded view for scaled render
    if (this.explodeFactor > 0 && this.lastRawMeshes.length > 0 && this.meshes.length > 0) {
      const scx = (this.boundsMin[0] + this.boundsMax[0]) / 2
      const scy = (this.boundsMin[1] + this.boundsMax[1]) / 2
      const scz = (this.boundsMin[2] + this.boundsMax[2]) / 2
      for (let mi = 0; mi < this.meshes.length && mi < this.lastRawMeshes.length; mi++) {
        const rm = this.lastRawMeshes[mi]
        const tt = rm.transform
        let mnx2 = Infinity, mny2 = Infinity, mnz2 = Infinity
        let mxx2 = -Infinity, mxy2 = -Infinity, mxz2 = -Infinity
        for (let i = 0; i < rm.vertices.length; i += 6) {
          const x = rm.vertices[i], y = rm.vertices[i+1], z = rm.vertices[i+2]
          const px = tt[0]*x+tt[1]*y+tt[2]*z+tt[3]
          const py = tt[4]*x+tt[5]*y+tt[6]*z+tt[7]
          const pz = tt[8]*x+tt[9]*y+tt[10]*z+tt[11]
          mnx2 = Math.min(mnx2,px); mxx2 = Math.max(mxx2,px)
          mny2 = Math.min(mny2,py); mxy2 = Math.max(mxy2,py)
          mnz2 = Math.min(mnz2,pz); mxz2 = Math.max(mxz2,pz)
        }
        const mcx = (mnx2+mxx2)/2, mcy = (mny2+mxy2)/2, mcz = (mnz2+mxz2)/2
        let edx = mcx - scx, edy = mcy - scy, edz = mcz - scz
        const elen = Math.sqrt(edx*edx + edy*edy + edz*edz)
        if (elen > 0.001) { edx /= elen; edy /= elen; edz /= elen }
        else { edx = 0; edy = 1; edz = 0 }
        const escale = Math.max(this.boundsMax[0]-this.boundsMin[0], this.boundsMax[1]-this.boundsMin[1], this.boundsMax[2]-this.boundsMin[2])
        const eoff = escale * this.explodeFactor
        const explT = new Float32Array(tt)
        explT[3] += edx * eoff; explT[7] += edy * eoff; explT[11] += edz * eoff
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 0, transpose(explT))
        const enm = transpose(invert(explT))
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 64, transpose(enm))
      }
    }

    // Outline pass (also enable when toon shading is active)
    if ((this.showOutline || this.toonShading) && this.renderMode !== 'wireframe') {
      pass.setPipeline(this.outlinePipe)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        if (g.transp) continue
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    }

    // Render mode dispatch (same logic as render())
    if (this.renderMode === 'wireframe') {
      // wireframe only
    } else if (this.renderMode === 'hidden-line') {
      // hidden-line: render meshes in background color to occlude, then edges on top
      pass.setPipeline(this.meshPipe)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        // Temporarily write clear color to mesh UB for occlusion
        this.dev.queue.writeBuffer(g.ub, 128, new Float32Array([this.clearR, this.clearG, this.clearB, 1.0]))
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    } else if (this.renderMode === 'xray') {
      pass.setPipeline(this.meshPipeT)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    } else {
      pass.setPipeline(this.meshPipe)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        if (g.transp) continue
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
      pass.setPipeline(this.meshPipeT)
      pass.setBindGroup(0, this.sceneBG)
      for (let i = 0; i < this.meshes.length; i++) {
        if (this.meshVisibility[i] === false) continue
        const g = this.meshes[i]
        if (!g.transp) continue
        pass.setBindGroup(1, g.bg)
        pass.setVertexBuffer(0, g.vb)
        pass.setIndexBuffer(g.ib, 'uint32')
        pass.drawIndexed(g.ic)
      }
    }

    /* edge overlay */
    if (this.showEdges && this.edgeVB && this.edgeVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.edgeVB)
      pass.draw(this.edgeVC)
    }

    if (this.wireframe && this.wireframeVB && this.wireframeVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.wireframeVB)
      pass.draw(this.wireframeVC)
    }
    pass.end()
    this.dev.queue.submit([enc.finish()])

    // Restore original colors after hidden-line render
    if (this.renderMode === 'hidden-line' && this.lastRawMeshes.length > 0) {
      for (let i = 0; i < this.meshes.length && i < this.lastRawMeshes.length; i++) {
        const raw = this.lastRawMeshes[i]
        this.dev.queue.writeBuffer(this.meshes[i].ub, 128, new Float32Array(raw.color))
      }
    }

    // Restore original model matrices after exploded view in scaled render
    if (this.explodeFactor > 0 && this.lastRawMeshes.length > 0 && this.meshes.length > 0) {
      for (let mi = 0; mi < this.meshes.length && mi < this.lastRawMeshes.length; mi++) {
        const rm = this.lastRawMeshes[mi]
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 0, transpose(rm.transform))
        const nm = transpose(invert(rm.transform))
        this.dev.queue.writeBuffer(this.meshes[mi].ub, 64, transpose(nm))
      }
    }
  }

  /** Copy the current canvas to clipboard as PNG. Returns true on success. */
  async copyToClipboard(): Promise<boolean> {
    this.render()
    return new Promise((resolve) => {
      this.canvas.toBlob(async (blob) => {
        if (!blob) { resolve(false); return }
        try {
          await navigator.clipboard.write([
            new ClipboardItem({ 'image/png': blob })
          ])
          resolve(true)
        } catch {
          resolve(false)
        }
      }, 'image/png')
    })
  }

  /** Toggle wireframe edge overlay on/off. */
  toggleWireframe(): boolean {
    this.wireframe = !this.wireframe
    if (this.wireframe) this.buildWireframeBuffer()
    this.requestRender()
    return this.wireframe
  }

  /** Toggle CAD-style edge overlay on/off. */
  toggleEdges(): boolean {
    this.showEdges = !this.showEdges
    if (this.showEdges) this.buildEdgeBuffer()
    this.requestRender()
    return this.showEdges
  }

  /** Toggle grid floor on/off. */
  toggleGrid(): boolean {
    this.showGrid = !this.showGrid
    this.requestRender()
    return this.showGrid
  }

  /** Toggle auto-rotate turntable on/off. */
  toggleAutoRotate(): boolean {
    this.autoRotate = !this.autoRotate
    if (this.autoRotate) this.requestRender()
    return this.autoRotate
  }

  /** Toggle between perspective and orthographic projection. */
  toggleProjection(): boolean {
    this.orthographic = !this.orthographic
    this.requestRender()
    return this.orthographic
  }

  /** Zoom in by reducing dist by 20%, animated. */
  zoomIn() {
    const targetDist = Math.max(1, this.dist * 0.8)
    this.animateTo(this.yaw, this.pitch, targetDist)
  }

  /** Zoom out by increasing dist by 20%, animated. */
  zoomOut() {
    const targetDist = Math.min(50000, this.dist * 1.2)
    this.animateTo(this.yaw, this.pitch, targetDist)
  }

  /** Project a 3D world point to 2D screen coordinates (CSS pixels relative to canvas). */
  getScreenPosition(worldX: number, worldY: number, worldZ: number): { x: number; y: number; behind: boolean } {
    const vp = this.lastVP
    // Multiply: clip = VP * [worldX, worldY, worldZ, 1]
    const cx = vp[0] * worldX + vp[1] * worldY + vp[2] * worldZ + vp[3]
    const cy = vp[4] * worldX + vp[5] * worldY + vp[6] * worldZ + vp[7]
    // const cz = vp[8] * worldX + vp[9] * worldY + vp[10] * worldZ + vp[11]
    const cw = vp[12] * worldX + vp[13] * worldY + vp[14] * worldZ + vp[15]

    const behind = cw <= 0
    const ndcX = cx / (Math.abs(cw) < 1e-6 ? 1e-6 : cw)
    const ndcY = cy / (Math.abs(cw) < 1e-6 ? 1e-6 : cw)

    const dpr = devicePixelRatio || 1
    const screenX = ((ndcX + 1) / 2) * (this.lastW / dpr)
    const screenY = ((1 - ndcY) / 2) * (this.lastH / dpr)

    return { x: screenX, y: screenY, behind }
  }

  /** Set the viewport background (clear) color. r, g, b in 0-1 range. */
  setClearColor(r: number, g: number, b: number, a = 1) {
    this.clearR = r
    this.clearG = g
    this.clearB = b
    this.clearA = a
    this.requestRender()
  }

  /** Set a lighting preset. */
  setLighting(preset: string) {
    this.lightingPreset = preset
    this.requestRender()
  }

  /** Toggle clipping plane on/off. */
  toggleClip(): boolean {
    this.clipEnabled = !this.clipEnabled
    this.requestRender()
    return this.clipEnabled
  }

  /** Set clipping plane enabled state. */
  setClipEnabled(v: boolean) {
    this.clipEnabled = v
    this.requestRender()
  }

  /** Set clipping plane Y value. Kept for backward compat. */
  setClipY(y: number) {
    this.clipValue = y
    this.requestRender()
  }

  /** Set clipping plane value on the current axis. */
  setClipValue(v: number) {
    this.clipValue = v
    this.requestRender()
  }

  /** Set the clipping plane axis (0=X, 1=Y, 2=Z). */
  setClipAxis(axis: number) {
    this.clipAxis = Math.max(0, Math.min(2, Math.round(axis)))
    this.requestRender()
  }

  /** Toggle fog on/off. */
  toggleFog(): boolean {
    this.fogEnabled = !this.fogEnabled
    this.requestRender()
    return this.fogEnabled
  }

  /** Set fog enabled state. */
  setFogEnabled(v: boolean) {
    this.fogEnabled = v
    this.requestRender()
  }

  /** Check whether fog is enabled. */
  isFogEnabled(): boolean {
    return this.fogEnabled
  }

  /** Toggle reflection on/off. */
  toggleReflection(): boolean {
    this.showReflection = !this.showReflection
    this.requestRender()
    return this.showReflection
  }

  /** Set reflection enabled state. */
  setReflection(v: boolean) {
    this.showReflection = v
    this.requestRender()
  }

  /** Check whether reflection is enabled. */
  isReflectionEnabled(): boolean {
    return this.showReflection
  }

  /** Toggle SSAO on/off. */
  toggleSsao(): boolean {
    this.ssao = !this.ssao
    this.requestRender()
    return this.ssao
  }

  /** Set SSAO enabled state. */
  setSsao(v: boolean) {
    this.ssao = v
    this.requestRender()
  }

  /** Check whether SSAO is enabled. */
  isSsaoEnabled(): boolean {
    return this.ssao
  }

  /** Toggle outline on/off. */
  toggleOutline(): boolean {
    this.showOutline = !this.showOutline
    this.requestRender()
    return this.showOutline
  }

  /** Set outline enabled state. */
  setOutline(v: boolean) {
    this.showOutline = v
    this.requestRender()
  }

  /** Check whether outline is enabled. */
  isOutlineEnabled(): boolean {
    return this.showOutline
  }

  /** Toggle normal smoothing on/off. Re-uploads meshes. */
  toggleNormalSmoothing(): boolean {
    this.normalSmoothing = !this.normalSmoothing
    if (this.lastRawMeshes.length) {
      this.setMeshes(this.lastRawMeshes)
    }
    return this.normalSmoothing
  }

  /** Set normal smoothing enabled state. Re-uploads meshes. */
  setNormalSmoothing(v: boolean) {
    if (this.normalSmoothing === v) return
    this.normalSmoothing = v
    if (this.lastRawMeshes.length) {
      this.setMeshes(this.lastRawMeshes)
    }
  }

  /** Check whether normal smoothing is enabled. */
  isNormalSmoothingEnabled(): boolean {
    return this.normalSmoothing
  }

  /**
   * Smooth normals by welding vertices at the same position and averaging normals.
   * Gives a smooth appearance instead of faceted.
   */
  private smoothNormals(meshes: MeshData[]): MeshData[] {
    return meshes.map(m => {
      const verts = m.vertices
      const vertCount = verts.length / 6

      // Build a map from position key to list of vertex indices
      const posMap = new Map<string, number[]>()
      for (let i = 0; i < vertCount; i++) {
        const x = verts[i * 6], y = verts[i * 6 + 1], z = verts[i * 6 + 2]
        // Round to avoid floating-point mismatches
        const key = `${(x * 1000 | 0)},${(y * 1000 | 0)},${(z * 1000 | 0)}`
        let list = posMap.get(key)
        if (!list) { list = []; posMap.set(key, list) }
        list.push(i)
      }

      // Average normals for shared positions
      const newVerts = new Float32Array(verts)
      for (const group of posMap.values()) {
        if (group.length <= 1) continue
        let nx = 0, ny = 0, nz = 0
        for (const vi of group) {
          nx += verts[vi * 6 + 3]
          ny += verts[vi * 6 + 4]
          nz += verts[vi * 6 + 5]
        }
        const len = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
        nx /= len; ny /= len; nz /= len
        for (const vi of group) {
          newVerts[vi * 6 + 3] = nx
          newVerts[vi * 6 + 4] = ny
          newVerts[vi * 6 + 5] = nz
        }
      }

      return { ...m, vertices: newVerts, indices: new Uint32Array(m.indices) }
    })
  }

  /** Set the sky/environment background preset. */
  setSkyPreset(preset: string) {
    this.skyPreset = preset
    if (this.skyUB) {
      const p = SKY_PRESETS[preset] || SKY_PRESETS.none
      this.dev.queue.writeBuffer(this.skyUB, 0, new Float32Array([...p.top, ...p.bottom]))
    }
    this.requestRender()
  }

  /** Get current sky preset name. */
  getSkyPreset(): string {
    return this.skyPreset
  }

  /** Get bounds for clipping plane range on the current (or specified) axis. */
  getClipRange(axis?: number): { min: number; max: number } {
    const a = axis !== undefined ? axis : this.clipAxis
    return { min: this.boundsMin[a] - 5, max: this.boundsMax[a] + 5 }
  }

  /** Set render mode: 'solid', 'solid+edges', 'wireframe', 'xray'. */
  setRenderMode(mode: string) {
    const m = mode as typeof this.renderMode
    if (m === this.renderMode) return
    const prevMode = this.renderMode
    this.renderMode = m

    switch (m) {
      case 'solid':
        this.wireframe = false
        this.showEdges = false
        break
      case 'solid+edges':
        this.wireframe = false
        this.showEdges = true
        this.buildEdgeBuffer()
        break
      case 'wireframe':
        this.wireframe = true
        this.showEdges = false
        this.buildWireframeBuffer()
        break
      case 'xray':
        this.wireframe = false
        this.showEdges = true
        this.buildEdgeBuffer()
        break
      case 'hidden-line':
        this.wireframe = false
        this.showEdges = true
        this.buildEdgeBuffer()
        break
    }

    // Handle xray alpha transitions
    if (m === 'xray' && prevMode !== 'xray') {
      // Entering xray: set all mesh colors to alpha 0.3
      for (let i = 0; i < this.meshes.length; i++) {
        const g = this.meshes[i]
        const raw = this.lastRawMeshes[i]
        if (!raw) continue
        this.dev.queue.writeBuffer(g.ub, 128, new Float32Array([raw.color[0], raw.color[1], raw.color[2], 0.3]))
        g.transp = true
      }
    } else if (m !== 'xray' && prevMode === 'xray') {
      // Leaving xray: restore original alphas
      for (let i = 0; i < this.meshes.length; i++) {
        const g = this.meshes[i]
        const raw = this.lastRawMeshes[i]
        if (!raw) continue
        this.dev.queue.writeBuffer(g.ub, 128, new Float32Array(raw.color))
        g.transp = raw.color[3] < 0.99
      }
    }
    this.requestRender()
  }

  /** Get current render mode. */
  getRenderMode(): string {
    return this.renderMode
  }

  /** Set flat shading on or off. */
  setFlatShading(v: boolean) {
    this.flatShading = v
    this.requestRender()
  }

  /** Check whether flat shading is enabled. */
  isFlatShading(): boolean {
    return this.flatShading
  }

  /** Toggle toon/cel shading. */
  toggleToonShading(): boolean {
    this.toonShading = !this.toonShading
    this.requestRender()
    return this.toonShading
  }

  /** Set toon shading on or off. */
  setToonShading(v: boolean) {
    this.toonShading = v
    this.requestRender()
  }

  /** Check whether toon shading is enabled. */
  isToonShading(): boolean {
    return this.toonShading
  }

  /** Toggle gooch shading on/off. */
  toggleGoochShading(): boolean {
    this.goochShading = !this.goochShading
    this.requestRender()
    return this.goochShading
  }

  /** Set gooch shading on or off. */
  setGoochShading(v: boolean) {
    this.goochShading = v
    this.requestRender()
  }

  /** Check whether gooch shading is enabled. */
  isGoochShading(): boolean {
    return this.goochShading
  }

  /** Toggle ground shadow on/off. */
  toggleGroundShadow(): boolean {
    this.showGroundShadow = !this.showGroundShadow
    this.requestRender()
    return this.showGroundShadow
  }

  /** Set ground shadow on or off. */
  setGroundShadow(v: boolean) {
    this.showGroundShadow = v
    this.requestRender()
  }

  /** Check whether ground shadow is enabled. */
  isGroundShadowEnabled(): boolean {
    return this.showGroundShadow
  }

  /** Set visibility of a specific mesh by index. */
  setMeshVisibility(index: number, visible: boolean) {
    if (index >= 0 && index < this.meshes.length) {
      this.meshVisibility[index] = visible
      this.requestRender()
    }
  }

  /** Set visibility of all meshes. */
  setAllVisibility(visible: boolean) {
    for (let i = 0; i < this.meshVisibility.length; i++) {
      this.meshVisibility[i] = visible
    }
    this.requestRender()
  }

  /** Set the exploded view factor (0 = normal, 1 = fully exploded). */
  setExplode(factor: number) {
    this.explodeFactor = Math.max(0, Math.min(2, factor))
    this.requestRender()
  }

  /** Get the current explode factor. */
  getExplodeFactor(): number {
    return this.explodeFactor
  }

  /** Set field of view in radians. */
  setFov(fovRad: number) {
    this.fov = Math.max(15 * Math.PI / 180, Math.min(120 * Math.PI / 180, fovRad))
    this.requestRender()
  }

  /** Get the current field of view in radians. */
  getFov(): number {
    return this.fov
  }

  /** Toggle orbit inertia. */
  toggleInertia(): boolean {
    this.inertiaEnabled = !this.inertiaEnabled
    if (!this.inertiaEnabled) {
      this.inertiaActive = false
      this.inertiaYawVel = 0
      this.inertiaPitchVel = 0
    }
    return this.inertiaEnabled
  }

  /** Set orbit inertia on or off. */
  setInertia(v: boolean) {
    this.inertiaEnabled = v
    if (!v) {
      this.inertiaActive = false
      this.inertiaYawVel = 0
      this.inertiaPitchVel = 0
    }
  }

  /** Check whether orbit inertia is enabled. */
  isInertiaEnabled(): boolean {
    return this.inertiaEnabled
  }

  /** Build a line-list vertex buffer with edges from the current meshes. */
  private buildWireframeBuffer() {
    this.wireframeVB?.destroy()
    this.wireframeVB = null
    this.wireframeVC = 0
    if (!this.lastRawMeshes.length) return

    // 7 floats per vertex: pos(3) + color(4)
    const d: number[] = []
    const edgeColor = [0.9, 0.9, 1.0, 0.35]

    for (const m of this.lastRawMeshes) {
      const verts = m.vertices  // interleaved pos(3) + normal(3)
      const idx = m.indices
      const t = m.transform

      // Transform positions by the mesh transform
      const transformedPos = (vi: number): [number, number, number] => {
        const x = verts[vi * 6], y = verts[vi * 6 + 1], z = verts[vi * 6 + 2]
        return [
          t[0]*x + t[1]*y + t[2]*z + t[3],
          t[4]*x + t[5]*y + t[6]*z + t[7],
          t[8]*x + t[9]*y + t[10]*z + t[11],
        ]
      }

      // Each triangle ABC -> edges AB, BC, CA
      for (let i = 0; i < idx.length; i += 3) {
        const a = idx[i], b = idx[i + 1], c = idx[i + 2]
        const pa = transformedPos(a), pb = transformedPos(b), pc = transformedPos(c)
        // AB
        d.push(pa[0], pa[1], pa[2], ...edgeColor)
        d.push(pb[0], pb[1], pb[2], ...edgeColor)
        // BC
        d.push(pb[0], pb[1], pb[2], ...edgeColor)
        d.push(pc[0], pc[1], pc[2], ...edgeColor)
        // CA
        d.push(pc[0], pc[1], pc[2], ...edgeColor)
        d.push(pa[0], pa[1], pa[2], ...edgeColor)
      }
    }
    if (!d.length) return

    this.wireframeVC = d.length / 7
    this.wireframeVB = this.dev.createBuffer({
      size: d.length * 4,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    })
    this.dev.queue.writeBuffer(this.wireframeVB, 0, new Float32Array(d))
  }

  /** Build a deduplicated edge buffer with dark CAD-style edges. */
  private buildEdgeBuffer() {
    this.edgeVB?.destroy()
    this.edgeVB = null
    this.edgeVC = 0
    if (!this.lastRawMeshes.length) return

    const edgeColor: [number, number, number, number] = [0.1, 0.1, 0.1, 0.6]
    const seen = new Set<string>()
    const d: number[] = []

    for (const m of this.lastRawMeshes) {
      const verts = m.vertices
      const idx = m.indices
      const t = m.transform

      const transformedPos = (vi: number): [number, number, number] => {
        const x = verts[vi * 6], y = verts[vi * 6 + 1], z = verts[vi * 6 + 2]
        return [
          t[0]*x + t[1]*y + t[2]*z + t[3],
          t[4]*x + t[5]*y + t[6]*z + t[7],
          t[8]*x + t[9]*y + t[10]*z + t[11],
        ]
      }

      for (let i = 0; i < idx.length; i += 3) {
        const tri = [idx[i], idx[i + 1], idx[i + 2]]
        const edges: [number, number][] = [
          [tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]],
        ]
        for (const [a, b] of edges) {
          const lo = Math.min(a, b)
          const hi = Math.max(a, b)
          const key = `${lo},${hi}`
          if (seen.has(key)) continue
          seen.add(key)
          const pa = transformedPos(a)
          const pb = transformedPos(b)
          d.push(pa[0], pa[1], pa[2], ...edgeColor)
          d.push(pb[0], pb[1], pb[2], ...edgeColor)
        }
      }
    }
    if (!d.length) return

    this.edgeVC = d.length / 7
    this.edgeVB = this.dev.createBuffer({
      size: d.length * 4,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    })
    this.dev.queue.writeBuffer(this.edgeVB, 0, new Float32Array(d))
  }

  /** Get GPU adapter/device info for the performance panel. */
  getDeviceInfo(): { adapter: string; description: string; vendor: string; architecture: string; maxBufferSize: number; maxTextureSize: number } {
    const info = this.adapterInfoCache
    return {
      adapter: info?.device ?? 'unknown',
      description: info?.description ?? '',
      vendor: info?.vendor ?? 'unknown',
      architecture: info?.architecture ?? '',
      maxBufferSize: this.dev?.limits?.maxBufferSize ?? 0,
      maxTextureSize: this.dev?.limits?.maxTextureDimension2D ?? 0,
    }
  }

  /** Get buffer statistics for the performance panel. */
  getBufferStats(): { totalVertexBytes: number; totalIndexBytes: number; meshCount: number; triangleCount: number } {
    let totalVB = 0, totalIB = 0, triCount = 0
    for (const m of this.lastRawMeshes) {
      totalVB += m.vertices.byteLength
      totalIB += m.indices.byteLength
      triCount += m.indices.length / 3
    }
    return { totalVertexBytes: totalVB, totalIndexBytes: totalIB, meshCount: this.lastRawMeshes.length, triangleCount: triCount }
  }

  /** Set fly camera mode on or off. */
  setFlyMode(v: boolean) {
    this.flyMode = v
  }

  /* ── Section Box (3-axis clip) ── */

  /** Toggle section box on/off. */
  toggleSectionBox(): boolean {
    this.sectionBoxEnabled = !this.sectionBoxEnabled
    this.requestRender()
    return this.sectionBoxEnabled
  }

  setSectionBoxEnabled(v: boolean) {
    this.sectionBoxEnabled = v
    this.requestRender()
  }

  isSectionBoxEnabled(): boolean {
    return this.sectionBoxEnabled
  }

  setSectionBoxValues(x: number, y: number, z: number) {
    this.sectionBoxX = x
    this.sectionBoxY = y
    this.sectionBoxZ = z
    this.requestRender()
  }

  setSectionBoxAxis(axis: number, value: number) {
    if (axis === 0) this.sectionBoxX = value
    else if (axis === 1) this.sectionBoxY = value
    else if (axis === 2) this.sectionBoxZ = value
    this.requestRender()
  }

  getSectionBoxRange(axis: number): { min: number; max: number } {
    return { min: this.boundsMin[axis] - 5, max: this.boundsMax[axis] + 5 }
  }

  /* ── Per-Object Color Override ── */

  /** Override the color of a specific mesh by index. */
  setMeshColorOverride(index: number, color: [number, number, number, number]) {
    this.meshColorOverrides.set(index, color)
    if (index >= 0 && index < this.meshes.length) {
      this.dev.queue.writeBuffer(this.meshes[index].ub, 128, new Float32Array(color))
      this.meshes[index].transp = color[3] < 0.99
      this.requestRender()
    }
  }

  /** Clear the color override for a specific mesh, restoring its original color. */
  clearMeshColorOverride(index: number) {
    this.meshColorOverrides.delete(index)
    if (index >= 0 && index < this.meshes.length && index < this.lastRawMeshes.length) {
      const m = this.lastRawMeshes[index]
      const color = this.renderMode === 'xray'
        ? [m.color[0], m.color[1], m.color[2], 0.3]
        : m.color
      this.dev.queue.writeBuffer(this.meshes[index].ub, 128, new Float32Array(color))
      this.meshes[index].transp = this.renderMode === 'xray' ? true : m.color[3] < 0.99
      this.requestRender()
    }
  }

  /** Get the current color of a mesh (override or original). */
  getMeshColor(index: number): [number, number, number, number] | null {
    const ov = this.meshColorOverrides.get(index)
    if (ov) return ov
    if (index >= 0 && index < this.lastRawMeshes.length) {
      const c = this.lastRawMeshes[index].color
      return [c[0], c[1], c[2], c[3]]
    }
    return null
  }

  /* ── Turntable Export ── */

  /** Capture a single frame as Blob (used for turntable ZIP export). */
  captureFrameAsBlob(): Promise<Blob | null> {
    this.render()
    return new Promise((resolve) => {
      this.canvas.toBlob((blob) => resolve(blob), 'image/png')
    })
  }

  destroy() {
    this.dead = true
    if (this.raf) cancelAnimationFrame(this.raf)
    if (this.resizeObserver) {
      this.resizeObserver.disconnect()
      this.resizeObserver = null
    }
    if (this.resizeDebounceTimer) {
      clearTimeout(this.resizeDebounceTimer)
      this.resizeDebounceTimer = null
    }
    const c = this.canvas
    c.removeEventListener('pointerdown', this.onDown)
    c.removeEventListener('pointermove', this.onMove)
    c.removeEventListener('pointerup', this.onUp)
    c.removeEventListener('wheel', this.onWheel)
    c.removeEventListener('contextmenu', this.noCtx)
    c.removeEventListener('touchstart', this.onTouchStart)
    c.removeEventListener('touchmove', this.onTouchMove)
    c.removeEventListener('touchend', this.onTouchEnd)
    for (const g of this.meshes) { g.vb.destroy(); g.ib.destroy(); g.ub.destroy() }
    this.meshes = []
    this.gridVB?.destroy()
    this.plateVB?.destroy()
    this.wireframeVB?.destroy()
    this.edgeVB?.destroy()
    this.msaaColorTex?.destroy()
    this.msaaDepthTex?.destroy()
    this.depth?.destroy()
    this.sceneUB?.destroy()
    this.reflUB?.destroy()
    this.skyUB?.destroy()
    this.dev.destroy()
  }
}
