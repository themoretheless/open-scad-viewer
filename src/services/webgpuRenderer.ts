/**
 * WebGPU 3D renderer — Phong shading, orbit camera, grid floor, axis gizmo.
 */
import {
  perspective, lookAt, transpose, invert, multiply, type Mat4,
} from './math3d'
import type { MeshData } from './openscadParser'

/* ── WGSL shaders ─────────────────────────────────── */

const MESH_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f }
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
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  let c = sc.ambient.rgb * ob.color.rgb + d * ob.color.rgb + s * vec3f(0.25) + bd * ob.color.rgb * 0.5;
  return vec4f(c, ob.color.a);
}
`

const LINE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f }
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

  /* wireframe overlay */
  private wireframeVB: GPUBuffer | null = null
  private wireframeVC = 0
  wireframe = false

  /* grid toggle */
  showGrid = true

  /* auto-rotate */
  autoRotate = false

  /* clear color (viewport background) */
  private clearR = 0.09
  private clearG = 0.09
  private clearB = 0.11

  yaw = 0.6; pitch = 0.4; dist = 50
  tx = 0; ty = 0; tz = 0

  private raf = 0
  private dead = false
  private drag = false; private pan = false
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
    this.sceneUB = this.dev.createBuffer({ size: 112, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
    this.sceneBG = this.dev.createBindGroup({
      layout: this.sceneBGL,
      entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
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
    this.autoFit(meshes)
    if (this.wireframe) this.buildWireframeBuffer()
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

  private render() {
    this.resize()
    const w = this.canvas.width, h = this.canvas.height, asp = w / h
    const cx = this.tx + this.dist * Math.cos(this.pitch) * Math.sin(this.yaw)
    const cy = this.ty + this.dist * Math.sin(this.pitch)
    const cz = this.tz + this.dist * Math.cos(this.pitch) * Math.cos(this.yaw)
    const view = lookAt([cx,cy,cz], [this.tx,this.ty,this.tz], [0,1,0])
    const proj = perspective(Math.PI / 4, asp, 0.1, this.dist * 10)
    const vp = transpose(multiply(proj, view))
    const sd = new Float32Array(28)
    sd.set(vp, 0)
    sd.set([cx,cy,cz,1], 16)
    sd.set([0.55,0.75,0.45,0], 20)
    sd.set([0.22,0.22,0.24,1], 24)
    this.dev.queue.writeBuffer(this.sceneUB, 0, sd)

    const enc = this.dev.createCommandEncoder()
    const pass = enc.beginRenderPass({
      colorAttachments: [{
        view: this.ctx.getCurrentTexture().createView(),
        clearValue: { r: this.clearR, g: this.clearG, b: this.clearB, a: 1 },
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
    this.dev.queue.submit([enc.finish()])
  }

  private loop = () => {
    if (this.dead) return
    if (this.autoRotate) this.yaw += 0.005
    this.render()
    this.raf = requestAnimationFrame(this.loop)
  }

  private onDown = (e: PointerEvent) => {
    this.drag = true; this.pan = e.button === 2 || e.shiftKey
    this.mx = e.clientX; this.my = e.clientY
    this.canvas.setPointerCapture(e.pointerId)
  }
  private onMove = (e: PointerEvent) => {
    if (!this.drag) return
    const dx = e.clientX - this.mx, dy = e.clientY - this.my
    this.mx = e.clientX; this.my = e.clientY
    if (this.pan) {
      const sp = this.dist * 0.002
      const cy = Math.cos(this.yaw), sy = Math.sin(this.yaw)
      this.tx -= dx * cy * sp; this.tz += dx * sy * sp; this.ty += dy * sp
    } else {
      this.yaw -= dx * 0.005
      this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch + dy * 0.005))
    }
  }
  private onUp = (e: PointerEvent) => { this.drag = false; this.canvas.releasePointerCapture(e.pointerId) }
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

  /** Set the viewport background (clear) color. r, g, b in 0-1 range. */
  setClearColor(r: number, g: number, b: number) {
    this.clearR = r
    this.clearG = g
    this.clearB = b
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
    this.wireframeVB?.destroy()
    this.depth?.destroy()
    this.sceneUB?.destroy()
    this.dev.destroy()
  }
}
