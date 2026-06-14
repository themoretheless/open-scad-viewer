/**
 * WebGPU 3D renderer — Phong shading, orbit camera, grid floor, axis gizmo.
 */
import {
  perspective, ortho, lookAt, transpose, invert, multiply, type Mat4,
} from './math3d'
import type { MeshData } from './openscadParser'

/* ── WGSL shaders ─────────────────────────────────── */

const MESH_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f }
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
  // Clipping plane
  if (sc.clip.y > 0.5 && v.w.y < sc.clip.x) { discard; }

  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  var c = sc.ambient.rgb * ob.color.rgb + d * ob.color.rgb + s * vec3f(0.25) + bd * ob.color.rgb * 0.5;

  // Fog
  if (sc.fogParams.z > 0.5) {
    let dist = length(sc.eye.xyz - v.w);
    let fogFactor = clamp((dist - sc.fogParams.x) / (sc.fogParams.y - sc.fogParams.x), 0.0, 1.0);
    c = mix(c, sc.fogColor.rgb, fogFactor);
  }

  return vec4f(c, ob.color.a);
}
`

const LINE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }
`

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
  private linePipe!: GPURenderPipeline
  private sceneBGL!: GPUBindGroupLayout
  private objBGL!: GPUBindGroupLayout
  private sceneUB!: GPUBuffer
  private sceneBG!: GPUBindGroup
  private depth!: GPUTexture

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

  /* grid toggle */
  showGrid = true

  /* auto-rotate */
  autoRotate = false

  /* lighting preset */
  lightingPreset = 'default'

  /* clipping plane */
  clipEnabled = false
  clipY = 0

  /* fog */
  fogEnabled = false

  /* reflection */
  showReflection = false

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
  private drag = false; private pan = false
  private snapOrbit = false
  private mx = 0; private my = 0

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
    const adapter = await navigator.gpu.requestAdapter()
    if (!adapter) return false
    this.dev = await adapter.requestDevice()
    this.ctx = canvas.getContext('webgpu') as GPUCanvasContext
    this.fmt = navigator.gpu.getPreferredCanvasFormat()
    this.ctx.configure({ device: this.dev, format: this.fmt, alphaMode: 'premultiplied' })

    this.buildPipelines()
    this.buildSceneUB()
    this.buildGrid()
    this.resize()
    this.bindInput()
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
    })
  }

  private buildSceneUB() {
    this.sceneUB = this.dev.createBuffer({ size: 192, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
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
      this.dev.queue.writeBuffer(ub, 128, new Float32Array(m.color))
      const bg = this.dev.createBindGroup({
        layout: this.objBGL,
        entries: [{ binding: 0, resource: { buffer: ub } }],
      })
      this.meshes.push({ vb, ib, ic: m.indices.length, ub, bg, transp: m.color[3] < 0.99 })
    }
    this.computeBounds(meshes)
    this.autoFit(meshes)
    if (this.wireframe) this.buildWireframeBuffer()
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
        mnx = Math.min(mnx, px); mxx = Math.max(mxx, px)
        mny = Math.min(mny, py); mxy = Math.max(mxy, py)
        mnz = Math.min(mnz, pz); mxz = Math.max(mxz, pz)
      }
    }
    if (!meshes.length) {
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
        mnx = Math.min(mnx,px); mxx = Math.max(mxx,px)
        mny = Math.min(mny,py); mxy = Math.max(mxy,py)
        mnz = Math.min(mnz,pz); mxz = Math.max(mxz,pz)
      }
    }
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
    }
  }

  /* cached view-projection for screen projection */
  private lastVP: Mat4 = new Float32Array(16)
  private lastW = 1
  private lastH = 1

  private render() {
    this.resize()
    const w = this.canvas.width, h = this.canvas.height, asp = w / h
    const cx = this.tx + this.dist * Math.cos(this.pitch) * Math.sin(this.yaw)
    const cy = this.ty + this.dist * Math.sin(this.pitch)
    const cz = this.tz + this.dist * Math.cos(this.pitch) * Math.cos(this.yaw)
    const view = lookAt([cx,cy,cz], [this.tx,this.ty,this.tz], [0,1,0])
    let proj: Mat4
    if (this.orthographic) {
      const halfH = this.dist * Math.tan(Math.PI / 8)
      const halfW = halfH * asp
      proj = ortho(-halfW, halfW, -halfH, halfH, 0.1, this.dist * 10)
    } else {
      proj = perspective(Math.PI / 4, asp, 0.1, this.dist * 10)
    }
    const vpMat = multiply(proj, view)
    this.lastVP = vpMat
    this.lastW = w
    this.lastH = h
    const vp = transpose(vpMat)
    const sd = new Float32Array(48)
    sd.set(vp, 0)
    sd.set([cx,cy,cz,1], 16)
    // Lighting (use preset values)
    const lp = this.getLightingValues()
    sd.set(lp.light, 20)
    sd.set(lp.ambient, 24)
    // Clipping plane
    sd.set([this.clipY, this.clipEnabled ? 1.0 : 0.0, 0, 0], 28)
    // Fog
    const fogNear = this.dist * 0.5
    const fogFar = this.dist * 3.0
    sd.set([fogNear, fogFar, this.fogEnabled ? 1.0 : 0.0, 0], 32)
    sd.set([this.clearR, this.clearG, this.clearB, 1], 36)
    this.dev.queue.writeBuffer(this.sceneUB, 0, sd)

    const enc = this.dev.createCommandEncoder()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: this.ctx.getCurrentTexture().createView(),
        clearValue: { r: this.clearR * this.clearA, g: this.clearG * this.clearA, b: this.clearB * this.clearA, a: this.clearA },
        loadOp: 'clear', storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: this.depth.createView(),
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

    pass.setPipeline(this.meshPipe)
    pass.setBindGroup(0, this.sceneBG)
    for (const g of this.meshes) {
      if (g.transp) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
    }

    pass.setPipeline(this.meshPipeT)
    pass.setBindGroup(0, this.sceneBG)
    for (const g of this.meshes) {
      if (!g.transp) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
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
    if (this.showReflection && this.meshes.length > 0) {
      const reflPass = enc.beginRenderPass({
        colorAttachments: [{
          view: this.ctx.getCurrentTexture().createView(),
          loadOp: 'load', storeOp: 'store',
        }],
        depthStencilAttachment: {
          view: this.depth.createView(),
          depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
        },
      })

      // Use transparent pipeline for reflections
      reflPass.setPipeline(this.meshPipeT)
      reflPass.setBindGroup(0, this.sceneBG)

      for (let mi = 0; mi < this.meshes.length; mi++) {
        const g = this.meshes[mi]
        if (g.transp) continue
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

    this.dev.queue.submit([enc.finish()])
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

    this.yaw = this.animStartYaw + (this.animTargetYaw - this.animStartYaw) * e
    this.pitch = this.animStartPitch + (this.animTargetPitch - this.animStartPitch) * e
    this.dist = this.animStartDist + (this.animTargetDist - this.animStartDist) * e
    this.tx = this.animStartTx + (this.animTargetTx - this.animStartTx) * e
    this.ty = this.animStartTy + (this.animTargetTy - this.animStartTy) * e
    this.tz = this.animStartTz + (this.animTargetTz - this.animStartTz) * e

    if (t >= 1) this.animating = false
  }

  private loop = () => {
    if (this.dead) return
    const now = performance.now()

    // FPS tracking
    if (this.lastFrameTime > 0) {
      this.frameTimes.push(now - this.lastFrameTime)
      if (this.frameTimes.length > 60) this.frameTimes.shift()
      const avg = this.frameTimes.reduce((a, b) => a + b, 0) / this.frameTimes.length
      this.currentFPS = avg > 0 ? Math.round(1000 / avg) : 0
    }
    this.lastFrameTime = now

    // Camera animation
    this.updateAnimation(now)

    if (this.autoRotate && !this.animating) this.yaw += 0.005
    this.render()
    this.raf = requestAnimationFrame(this.loop)
  }

  private onDown = (e: PointerEvent) => {
    this.drag = true; this.pan = e.button === 2 || e.shiftKey
    // Ctrl (without Shift) during an orbit drag = snap yaw/pitch to 15° steps.
    this.snapOrbit = !this.pan && e.ctrlKey
    this.mx = e.clientX; this.my = e.clientY
    this.canvas.setPointerCapture(e.pointerId)
  }
  private onMove = (e: PointerEvent) => {
    if (!this.drag) return
    const dx = e.clientX - this.mx, dy = e.clientY - this.my
    this.mx = e.clientX; this.my = e.clientY
    // Allow toggling Ctrl mid-drag for orbit snapping.
    if (!this.pan) this.snapOrbit = e.ctrlKey
    if (this.pan) {
      const sp = this.dist * 0.002
      const cy = Math.cos(this.yaw), sy = Math.sin(this.yaw)
      this.tx -= dx * cy * sp; this.tz += dx * sy * sp; this.ty += dy * sp
    } else {
      this.yaw -= dx * 0.005
      this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + dy * 0.005))
      if (this.snapOrbit) {
        // Snap yaw & pitch to 15° (PI/12) increments while dragging.
        const step = Math.PI / 12
        this.yaw = Math.round(this.yaw / step) * step
        this.pitch = Math.max(-1.5, Math.min(1.5, Math.round(this.pitch / step) * step))
      }
    }
  }
  private onUp = (e: PointerEvent) => { this.drag = false; this.snapOrbit = false; this.canvas.releasePointerCapture(e.pointerId) }
  private onWheel = (e: WheelEvent) => {
    e.preventDefault()
    this.dist = Math.max(1, Math.min(50000, this.dist * (1 + e.deltaY * 0.001)))
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
  }

  /** Get current camera yaw/pitch (for the orientation gizmo). */
  getOrientation(): { yaw: number; pitch: number } {
    return { yaw: this.yaw, pitch: this.pitch }
  }

  /**
   * Smoothly snap the camera to look straight down a principal axis.
   * axis: '+x' | '-x' | '+y' | '-y' | '+z' | '-z'
   */
  snapToAxis(axis: string) {
    // yaw/pitch are spherical angles where the eye is positioned at:
    //   x = tx + d*cos(pitch)*sin(yaw)
    //   y = ty + d*sin(pitch)
    //   z = tz + d*cos(pitch)*cos(yaw)
    // Looking down +X means the eye sits on +X looking toward origin.
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

  /** Public wrapper for autoFit — re-fits camera to current meshes' bounding box. */
  autoFitAll() {
    // Rebuild the bounding box from existing GPU meshes isn't practical,
    // so we store the last raw MeshData set for re-fitting.
    if (this.lastRawMeshes && this.lastRawMeshes.length) {
      this.autoFit(this.lastRawMeshes)
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
   * Take a screenshot at scale× the current backing-store resolution.
   * Temporarily resizes the canvas backing store + depth texture, renders one
   * frame, captures via toBlob, then restores the original size and re-renders.
   */
  screenshotScaled(scale: number) {
    if (scale <= 1) { this.screenshot(); return }
    const origW = this.canvas.width
    const origH = this.canvas.height
    const origDepth = this.depth

    const w = Math.max(1, Math.round(origW * scale))
    const h = Math.max(1, Math.round(origH * scale))
    this.canvas.width = w
    this.canvas.height = h
    this.depth = this.dev.createTexture({ size: [w, h], format: 'depth24plus', usage: GPUTextureUsage.RENDER_ATTACHMENT })

    // Render directly with the scaled buffers (bypass resize(), which would
    // snap the canvas back to its CSS size).
    this.renderScaled(w, h)

    const restore = () => {
      // Restore original backing store + depth, then force a normal re-render.
      this.canvas.width = origW
      this.canvas.height = origH
      this.depth?.destroy()
      this.depth = origDepth
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
      const halfH = this.dist * Math.tan(Math.PI / 8)
      const halfW = halfH * asp
      proj = ortho(-halfW, halfW, -halfH, halfH, 0.1, this.dist * 10)
    } else {
      proj = perspective(Math.PI / 4, asp, 0.1, this.dist * 10)
    }
    const vpMat = multiply(proj, view)
    const vp = transpose(vpMat)
    const sd = new Float32Array(48)
    sd.set(vp, 0)
    sd.set([cx, cy, cz, 1], 16)
    const lp = this.getLightingValues()
    sd.set(lp.light, 20)
    sd.set(lp.ambient, 24)
    sd.set([this.clipY, this.clipEnabled ? 1.0 : 0.0, 0, 0], 28)
    const fogNear = this.dist * 0.5
    const fogFar = this.dist * 3.0
    sd.set([fogNear, fogFar, this.fogEnabled ? 1.0 : 0.0, 0], 32)
    sd.set([this.clearR, this.clearG, this.clearB, 1], 36)
    this.dev.queue.writeBuffer(this.sceneUB, 0, sd)

    const enc = this.dev.createCommandEncoder()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: this.ctx.getCurrentTexture().createView(),
        clearValue: { r: this.clearR * this.clearA, g: this.clearG * this.clearA, b: this.clearB * this.clearA, a: this.clearA },
        loadOp: 'clear', storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: this.depth.createView(),
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

    pass.setPipeline(this.meshPipe)
    pass.setBindGroup(0, this.sceneBG)
    for (const g of this.meshes) {
      if (g.transp) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
    }
    pass.setPipeline(this.meshPipeT)
    pass.setBindGroup(0, this.sceneBG)
    for (const g of this.meshes) {
      if (!g.transp) continue
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
    }
    if (this.wireframe && this.wireframeVB && this.wireframeVC > 0) {
      pass.setPipeline(this.linePipe)
      pass.setBindGroup(0, this.sceneBG)
      pass.setVertexBuffer(0, this.wireframeVB)
      pass.draw(this.wireframeVC)
    }
    pass.end()
    this.dev.queue.submit([enc.finish()])
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
    return this.wireframe
  }

  /** Toggle grid floor on/off. */
  toggleGrid(): boolean {
    this.showGrid = !this.showGrid
    return this.showGrid
  }

  /** Toggle auto-rotate turntable on/off. */
  toggleAutoRotate(): boolean {
    this.autoRotate = !this.autoRotate
    return this.autoRotate
  }

  /** Toggle between perspective and orthographic projection. */
  toggleProjection(): boolean {
    this.orthographic = !this.orthographic
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
  }

  /** Set a lighting preset. */
  setLighting(preset: string) {
    this.lightingPreset = preset
  }

  /** Toggle clipping plane on/off. */
  toggleClip(): boolean {
    this.clipEnabled = !this.clipEnabled
    return this.clipEnabled
  }

  /** Set clipping plane enabled state. */
  setClipEnabled(v: boolean) {
    this.clipEnabled = v
  }

  /** Set clipping plane Y value. */
  setClipY(y: number) {
    this.clipY = y
  }

  /** Toggle fog on/off. */
  toggleFog(): boolean {
    this.fogEnabled = !this.fogEnabled
    return this.fogEnabled
  }

  /** Set fog enabled state. */
  setFogEnabled(v: boolean) {
    this.fogEnabled = v
  }

  /** Check whether fog is enabled. */
  isFogEnabled(): boolean {
    return this.fogEnabled
  }

  /** Toggle reflection on/off. */
  toggleReflection(): boolean {
    this.showReflection = !this.showReflection
    return this.showReflection
  }

  /** Set reflection enabled state. */
  setReflection(v: boolean) {
    this.showReflection = v
  }

  /** Check whether reflection is enabled. */
  isReflectionEnabled(): boolean {
    return this.showReflection
  }

  /** Get bounds for clipping plane range. */
  getClipRange(): { min: number; max: number } {
    return { min: this.boundsMin[1] - 5, max: this.boundsMax[1] + 5 }
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

  destroy() {
    this.dead = true
    cancelAnimationFrame(this.raf)
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
    this.depth?.destroy()
    this.sceneUB?.destroy()
    this.reflUB?.destroy()
    this.dev.destroy()
  }
}
