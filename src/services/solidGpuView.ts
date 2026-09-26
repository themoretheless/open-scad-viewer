/**
 * Smooth GPU display for the Solid workspace.
 *
 * The Solid 3D pane is an SVG whose camera is a pure rotation (`projectDirectPoint`) followed by a square
 * `viewBox`. This layer draws the same bodies with WebGPU underneath that SVG using an identical mapping, so the
 * SVG keeps picking, gizmos and previews while the surfaces get dense tessellation and per-pixel shading.
 */
import type { OrbitCamera } from './directModelingTools'

export interface SolidGpuBody {
  /** Identifies this body so a drag can move it without re-uploading the scene. */
  id: string
  /** Flat, non-indexed triangle list: xyz per vertex. */
  positions: Float32Array
  /** Smoothed vertex normals matching `positions`. */
  normals: Float32Array
  /** HSL hue per triangle (220 base, 266 selected, 190 hovered, 40 selected face). */
  hues: Float32Array
}

export interface SolidGpuView {
  camera: OrbitCamera
  /** SVG viewBox: left, top, size (square). */
  viewBox: [number, number, number]
}

const SHADER = /* wgsl */ `
struct Uniforms {
  rotation: mat3x3<f32>,
  view: vec4<f32>,      // viewBox x, y, size, depth range
  screen: vec4<f32>,    // canvas width, height, unused, unused
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) normal: vec3<f32>,
  @location(1) hue: f32,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, @location(2) hue: f32) -> VertexOut {
  let p = u.rotation * pos;
  // The 3D SVG pane maps projectDirectPoint's x' and y' straight to viewBox units (no Y flip; that is 2D-only).
  let svgX = p.x;
  let svgY = p.y;
  let scale = min(u.screen.x, u.screen.y) / u.view.z;
  let offsetX = (u.screen.x - u.view.z * scale) * 0.5;
  let offsetY = (u.screen.y - u.view.z * scale) * 0.5;
  let sx = (svgX - u.view.x) * scale + offsetX;
  let sy = (svgY - u.view.y) * scale + offsetY;
  var out: VertexOut;
  out.position = vec4<f32>(sx / u.screen.x * 2.0 - 1.0, 1.0 - sy / u.screen.y * 2.0, 0.5 - p.z / u.view.w, 1.0);
  out.normal = u.rotation * nrm;
  out.hue = hue;
  return out;
}

fn hsl(h: f32, s: f32, l: f32) -> vec3<f32> {
  let c = (1.0 - abs(2.0 * l - 1.0)) * s;
  let hp = h / 60.0;
  let x = c * (1.0 - abs(hp % 2.0 - 1.0));
  var rgb = vec3<f32>(0.0);
  if (hp < 1.0) { rgb = vec3<f32>(c, x, 0.0); }
  else if (hp < 2.0) { rgb = vec3<f32>(x, c, 0.0); }
  else if (hp < 3.0) { rgb = vec3<f32>(0.0, c, x); }
  else if (hp < 4.0) { rgb = vec3<f32>(0.0, x, c); }
  else if (hp < 5.0) { rgb = vec3<f32>(x, 0.0, c); }
  else { rgb = vec3<f32>(c, 0.0, x); }
  return rgb + vec3<f32>(l - c * 0.5);
}

@fragment
fn fs(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
  var n = normalize(in.normal);
  if (n.z < 0.0) { n = -n; }
  // Same lightness formula as the SVG fallback (directFaceShade), so both displays match.
  let lightness = clamp(48.0 + 20.0 * n.z - 15.0 * n.y + 8.0 * n.x, 24.0, 78.0) / 100.0;
  let rgb = hsl(in.hue, 0.45, lightness);
  return vec4<f32>(rgb, 1.0);
}
`

/** Rotation rows reproduce projectDirectPoint: x' = x cy - y sy; y' = (x sy + y cy) sp - z cp; z' = (x sy + y cy) cp + z sp. */
export function solidRotationColumns(camera: OrbitCamera): Float32Array {
  const cy = Math.cos(camera.yaw), sy = Math.sin(camera.yaw), cp = Math.cos(camera.pitch), sp = Math.sin(camera.pitch)
  // mat3x3 in WGSL is column-major with 16-byte column stride.
  return new Float32Array([
    cy, sy * sp, sy * cp, 0,
    -sy, cy * sp, cy * cp, 0,
    0, -cp, sp, 0,
  ])
}

export function isSolidGpuSupported(): boolean {
  return typeof navigator !== 'undefined' && 'gpu' in navigator && !!navigator.gpu
}

export class SolidGpuLayer {
  private device: GPUDevice | null = null
  private context: GPUCanvasContext | null = null
  private pipeline: GPURenderPipeline | null = null
  private uniformBuffer: GPUBuffer | null = null
  private bindGroup: GPUBindGroup | null = null
  private vertexBuffer: GPUBuffer | null = null
  private vertexCount = 0
  private packed: Float32Array | null = null
  /** Wall time of the last completed GPU submission, for the viewport's frame readout. */
  private lastDrawMs = 0
  get drawMs(): number { return this.lastDrawMs }
  private ranges = new Map<string, { start: number; count: number }>()
  private depth: GPUTexture | null = null
  private format: GPUTextureFormat = 'bgra8unorm'
  private view: SolidGpuView = { camera: { yaw: 0, pitch: 0 }, viewBox: [-80, -80, 160] }
  private frame = 0
  private disposed = false

  constructor(private readonly canvas: HTMLCanvasElement) {}

  async init(): Promise<boolean> {
    if (!isSolidGpuSupported()) return false
    try {
      const adapter = await navigator.gpu.requestAdapter()
      if (!adapter || this.disposed) return false
      const device = await adapter.requestDevice()
      if (this.disposed) { device.destroy(); return false }
      const context = this.canvas.getContext('webgpu')
      if (!context) return false
      this.format = navigator.gpu.getPreferredCanvasFormat()
      context.configure({ device, format: this.format, alphaMode: 'premultiplied' })
      const module = device.createShaderModule({ code: SHADER })
      this.pipeline = device.createRenderPipeline({
        layout: 'auto',
        vertex: {
          module, entryPoint: 'vs',
          buffers: [{
            arrayStride: 7 * 4,
            attributes: [
              { shaderLocation: 0, offset: 0, format: 'float32x3' },
              { shaderLocation: 1, offset: 12, format: 'float32x3' },
              { shaderLocation: 2, offset: 24, format: 'float32' },
            ],
          }],
        },
        fragment: { module, entryPoint: 'fs', targets: [{ format: this.format }] },
        primitive: { topology: 'triangle-list', cullMode: 'none' },
        depthStencil: { format: 'depth24plus', depthWriteEnabled: true, depthCompare: 'less' },
      })
      this.uniformBuffer = device.createBuffer({ size: 48 + 16 + 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
      this.bindGroup = device.createBindGroup({
        layout: this.pipeline.getBindGroupLayout(0),
        entries: [{ binding: 0, resource: { buffer: this.uniformBuffer } }],
      })
      device.lost.then(() => { if (!this.disposed) this.device = null }).catch(() => {})
      this.device = device
      this.context = context
      return true
    } catch {
      return false
    }
  }

  get ready(): boolean { return !!this.device && !!this.pipeline }

  setBodies(bodies: readonly SolidGpuBody[]): void {
    const device = this.device
    if (!device) return
    let total = 0
    for (const body of bodies) total += body.positions.length / 3
    const data = new Float32Array(total * 7)
    this.ranges = new Map()
    let offset = 0
    for (const body of bodies) {
      const count = body.positions.length / 3
      this.ranges.set(body.id, { start: offset / 7, count })
      for (let i = 0; i < count; i++) {
        data[offset++] = body.positions[i * 3]
        data[offset++] = body.positions[i * 3 + 1]
        data[offset++] = body.positions[i * 3 + 2]
        data[offset++] = body.normals[i * 3]
        data[offset++] = body.normals[i * 3 + 1]
        data[offset++] = body.normals[i * 3 + 2]
        data[offset++] = body.hues[Math.floor(i / 3)]
      }
    }
    this.vertexBuffer?.destroy()
    this.vertexBuffer = null
    this.vertexCount = total
    this.packed = data
    if (total === 0) { this.requestFrame(); return }
    this.vertexBuffer = device.createBuffer({ size: Math.max(28, data.byteLength), usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    device.queue.writeBuffer(this.vertexBuffer, 0, data)
    this.requestFrame()
  }

  /**
   * Translates already-uploaded bodies without rebuilding the scene.
   *
   * A drag would otherwise re-derive smoothed normals for every body and recreate the
   * whole vertex buffer each frame. Translation leaves normals untouched, so only the
   * dragged body's positions are rewritten, in place. Call with a zero delta to restore.
   */
  setDragOffset(ids: readonly string[], delta: readonly [number, number, number]): void {
    const device = this.device, buffer = this.vertexBuffer, packed = this.packed
    if (!device || !buffer || !packed) return
    for (const id of ids) {
      const range = this.ranges.get(id)
      if (!range) continue
      const slice = new Float32Array(range.count * 7)
      for (let i = 0; i < range.count; i++) {
        const from = (range.start + i) * 7, to = i * 7
        slice[to] = packed[from] + delta[0]
        slice[to + 1] = packed[from + 1] + delta[1]
        slice[to + 2] = packed[from + 2] + delta[2]
        for (let k = 3; k < 7; k++) slice[to + k] = packed[from + k]
      }
      device.queue.writeBuffer(buffer, range.start * 7 * 4, slice)
    }
    this.requestFrame()
  }

  setView(view: SolidGpuView): void {
    this.view = { camera: { ...view.camera }, viewBox: [...view.viewBox] as [number, number, number] }
    this.requestFrame()
  }

  resize(width: number, height: number, ratio = 1): void {
    const w = Math.max(1, Math.round(width * ratio)), h = Math.max(1, Math.round(height * ratio))
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w
      this.canvas.height = h
      this.depth?.destroy()
      this.depth = null
    }
    this.requestFrame()
  }

  requestFrame(): void {
    if (this.frame || this.disposed) return
    this.frame = requestAnimationFrame(() => { this.frame = 0; this.render() })
  }

  private render(): void {
    const device = this.device, context = this.context, pipeline = this.pipeline
    if (!device || !context || !pipeline || !this.uniformBuffer || !this.bindGroup) return
    const width = this.canvas.width, height = this.canvas.height
    if (!this.depth) {
      this.depth = device.createTexture({ size: [width, height], format: 'depth24plus', usage: GPUTextureUsage.RENDER_ATTACHMENT })
    }
    const [vx, vy, size] = this.view.viewBox
    const uniforms = new Float32Array(20)
    uniforms.set(solidRotationColumns(this.view.camera), 0)
    uniforms.set([vx, vy, size, Math.max(size, 1) * 16], 12)
    uniforms.set([width, height, 0, 0], 16)
    device.queue.writeBuffer(this.uniformBuffer, 0, uniforms)
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginRenderPass({
      colorAttachments: [{ view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: 'clear', storeOp: 'store' }],
      depthStencilAttachment: { view: this.depth.createView(), depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store' },
    })
    if (this.vertexBuffer && this.vertexCount) {
      pass.setPipeline(pipeline)
      pass.setBindGroup(0, this.bindGroup)
      pass.setVertexBuffer(0, this.vertexBuffer)
      pass.draw(this.vertexCount)
    }
    pass.end()
    const drawStarted = performance.now()
    device.queue.submit([encoder.finish()])
    // Resolves once the GPU has finished; measured even when frame callbacks are throttled.
    void device.queue.onSubmittedWorkDone().then(() => { this.lastDrawMs = performance.now() - drawStarted })
  }

  destroy(): void {
    this.disposed = true
    if (this.frame) cancelAnimationFrame(this.frame)
    this.vertexBuffer?.destroy()
    this.depth?.destroy()
    this.uniformBuffer?.destroy()
    try { this.context?.unconfigure() } catch { /* context may be lost */ }
    this.device?.destroy()
    this.device = null
  }
}

/**
 * Expands an indexed mesh into a flat triangle list with crease-aware smoothed normals: normals of triangles
 * meeting at the same position are averaged only when they differ by less than `creaseDegrees`, so curved
 * faces shade smoothly while box edges stay sharp.
 */
export function smoothTriangleList(positions: ArrayLike<number>, indices: ArrayLike<number>, creaseDegrees = 40): { positions: Float32Array; normals: Float32Array } {
  const triangleCount = Math.floor(indices.length / 3)
  const faceNormals: number[][] = []
  const byPosition = new Map<string, number[]>()
  const key = (i: number) => `${positions[i * 3].toFixed(5)},${positions[i * 3 + 1].toFixed(5)},${positions[i * 3 + 2].toFixed(5)}`
  for (let t = 0; t < triangleCount; t++) {
    const [a, b, c] = [0, 1, 2].map(k => { const i = indices[t * 3 + k]; return [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]] })
    const u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]]
    const n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]]
    const len = Math.hypot(n[0], n[1], n[2]) || 1
    faceNormals.push([n[0] / len, n[1] / len, n[2] / len])
    for (let k = 0; k < 3; k++) {
      const id = key(indices[t * 3 + k])
      const list = byPosition.get(id)
      if (list) list.push(t); else byPosition.set(id, [t])
    }
  }
  const cosCrease = Math.cos(creaseDegrees * Math.PI / 180)
  const outPositions = new Float32Array(triangleCount * 9), outNormals = new Float32Array(triangleCount * 9)
  for (let t = 0; t < triangleCount; t++) {
    const fn = faceNormals[t]
    for (let k = 0; k < 3; k++) {
      const i = indices[t * 3 + k]
      const sum = [0, 0, 0]
      for (const other of byPosition.get(key(i)) ?? []) {
        const on = faceNormals[other]
        if (fn[0] * on[0] + fn[1] * on[1] + fn[2] * on[2] >= cosCrease) { sum[0] += on[0]; sum[1] += on[1]; sum[2] += on[2] }
      }
      const len = Math.hypot(sum[0], sum[1], sum[2])
      const n = len > 1e-9 ? [sum[0] / len, sum[1] / len, sum[2] / len] : fn
      const o = (t * 3 + k) * 3
      outPositions[o] = positions[i * 3]; outPositions[o + 1] = positions[i * 3 + 1]; outPositions[o + 2] = positions[i * 3 + 2]
      outNormals[o] = n[0]; outNormals[o + 1] = n[1]; outNormals[o + 2] = n[2]
    }
  }
  return { positions: outPositions, normals: outNormals }
}
