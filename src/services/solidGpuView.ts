import {TransparentBsp, type TransparentFragment} from './transparentBsp'
import {smoothTriangleListInKernel} from './geometry/meshAnalysis'
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
  /** RGB color used when the triangle hue is negative. */
  color?: [number,number,number]
  metallic?: number
  roughness?: number
  opacity?: number
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
  @location(2) color: vec3<f32>,
  @location(3) material: vec2<f32>,
  @location(4) opacity: f32,
};

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, @location(2) hue: f32, @location(3) color: vec3<f32>, @location(4) material: vec2<f32>, @location(5) opacity: f32) -> VertexOut {
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
  out.color = color;
  out.material = material;
  out.opacity = opacity;
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
  let lightness = clamp(48.0 + 20.0 * n.z - 15.0 * n.y + 8.0 * n.x, 24.0, 78.0) / 120.0;
  let rgb = select(hsl(in.hue, 0.45, lightness), in.color * lightness / 0.78, in.hue < 0.0);
  if (in.hue < 0.0 && in.material.y >= 0.0) {
    let base = pow(max(in.color, vec3<f32>(0.0)), vec3<f32>(2.2));
    let metallic = clamp(in.material.x, 0.0, 1.0);
    let rough = clamp(in.material.y, 0.04, 1.0);
    let v = vec3<f32>(0.0, 0.0, 1.0);
    let l = normalize(vec3<f32>(-0.4, -0.6, 1.0));
    let h = normalize(v+l);
    let nv = max(dot(n,v), 0.001);
    let nl = max(dot(n,l), 0.0);
    let nh = max(dot(n,h), 0.0);
    let a2 = pow(rough, 4.0);
    let denom = nh*nh*(a2-1.0)+1.0;
    let distribution = a2 / max(3.14159265*denom*denom, 0.000001);
    let k = (rough+1.0)*(rough+1.0)/8.0;
    let geometry = nv/(nv*(1.0-k)+k) * nl/(nl*(1.0-k)+k);
    let f0 = mix(vec3<f32>(0.04), base, metallic);
    let fresnel = f0+(vec3<f32>(1.0)-f0)*pow(1.0-max(dot(h,v),0.0),5.0);
    let specular = distribution*geometry*fresnel/max(4.0*nv*nl,0.0001);
    let diffuse = (vec3<f32>(1.0)-fresnel)*(1.0-metallic)*base/3.14159265;
    let linear = (diffuse+specular)*nl*2.5 + base*0.12;
    return vec4<f32>(pow(linear/(vec3<f32>(1.0)+linear),vec3<f32>(1.0/2.2))*in.opacity,in.opacity);
  }
  return vec4<f32>(rgb*in.opacity, in.opacity);
}
`

const scratchRotation = new Float32Array(12)
const scratchUniforms = new Float32Array(20)

/** Rotation rows reproduce projectDirectPoint: x' = x cy - y sy; y' = (x sy + y cy) sp - z cp; z' = (x sy + y cy) cp + z sp. */
export function solidRotationColumns(camera: OrbitCamera, out?: Float32Array): Float32Array {
  const cy = Math.cos(camera.yaw), sy = Math.sin(camera.yaw), cp = Math.cos(camera.pitch), sp = Math.sin(camera.pitch)
  // mat3x3 in WGSL is column-major with 16-byte column stride.
  const target = out ?? new Float32Array(12)
  target[0] = cy; target[1] = sy * sp; target[2] = sy * cp; target[3] = 0
  target[4] = -sy; target[5] = cy * sp; target[6] = cy * cp; target[7] = 0
  target[8] = 0; target[9] = -cp; target[10] = sp; target[11] = 0
  return target
}

export function isSolidGpuSupported(): boolean {
  return typeof navigator !== 'undefined' && 'gpu' in navigator && !!navigator.gpu
}

export class SolidGpuLayer {
  private device: GPUDevice | null = null
  private context: GPUCanvasContext | null = null
  private pipeline: GPURenderPipeline | null = null
  private transparentPipeline: GPURenderPipeline | null = null
  private transparentBindGroup: GPUBindGroup | null = null
  private transparentVertices: GPUBuffer | null = null
  private transparentTree: TransparentBsp | null = null
  private transparencyLimited = false
  private transparentData: Float32Array | null = null
  private transparentDirection: number[] | null = null
  private opaqueCount = 0
  private transparentTriangles: Array<{first:number;body:string}> = []
  private dragOffsets = new Map<string,readonly [number,number,number]>()
  private uniformBuffer: GPUBuffer | null = null
  private bindGroup: GPUBindGroup | null = null
  private vertexBuffer: GPUBuffer | null = null
  private vertexCount = 0
  private packed: Float32Array | null = null
  /** Wall time of the last completed GPU submission, for the viewport's frame readout. */
  private lastDrawMs = 0
  private lastDrawCalls = 0
  private lastTriangleCount = 0
  private deviceLost = false
  get drawMs(): number { return this.lastDrawMs }
  getFrameDiagnostics(): { drawMs: number; drawCalls: number; triangles: number; deviceLost: boolean } {
    return {
      drawMs: this.lastDrawMs,
      drawCalls: this.lastDrawCalls,
      triangles: this.lastTriangleCount,
      deviceLost: this.deviceLost,
    }
  }
  private ranges = new Map<string, { start: number; count: number }>()
  private depth: GPUTexture | null = null
  private depthView: GPUTextureView | null = null
  private dragScratch: Float32Array | null = null
  private format: GPUTextureFormat = 'bgra8unorm'
  private view: SolidGpuView = { camera: { yaw: 0, pitch: 0 }, viewBox: [-80, -80, 160] }
  private frame = 0
  private disposed = false

  constructor(private readonly canvas: HTMLCanvasElement,private readonly onUnavailable?:()=>void, private readonly onTransparencyLimit?:(limited:boolean)=>void) {}

  private fail(reason:unknown='WebGPU unavailable'):void {
    if(this.disposed)return
    this.canvas.setAttribute?.('data-gpu-error',reason instanceof Error?reason.message:String(reason))
    this.destroy()
    this.onUnavailable?.()
  }

  async init(): Promise<boolean> {
    if (!isSolidGpuSupported()) return false
    try {
      const adapter = await navigator.gpu.requestAdapter()
      if (!adapter || this.disposed) return false
      const device = await adapter.requestDevice()
      if (this.disposed) { device.destroy(); return false }
      this.device=device
      const context = this.canvas.getContext('webgpu')
      if (!context) {this.destroy();return false}
      this.context=context
      this.format = navigator.gpu.getPreferredCanvasFormat()
      context.configure({ device, format: this.format, alphaMode: 'premultiplied' })
      const module = device.createShaderModule({ code: SHADER })
      const makePipeline=(transparent:boolean)=>device.createRenderPipeline({
        layout: 'auto',
        vertex: {
          module, entryPoint: 'vs',
          buffers: [{
            arrayStride: 13 * 4,
            attributes: [
              { shaderLocation: 0, offset: 0, format: 'float32x3' },
              { shaderLocation: 1, offset: 12, format: 'float32x3' },
              { shaderLocation: 2, offset: 24, format: 'float32' },
              { shaderLocation: 3, offset: 28, format: 'float32x3' },
              { shaderLocation: 4, offset: 40, format: 'float32x2' },
              { shaderLocation: 5, offset: 48, format: 'float32' },
            ],
          }],
        },
        fragment: { module, entryPoint: 'fs', targets: [{ format: this.format, ...(transparent?{blend:{color:{srcFactor:'one',dstFactor:'one-minus-src-alpha',operation:'add'},alpha:{srcFactor:'one',dstFactor:'one-minus-src-alpha',operation:'add'}} as GPUBlendState}:{}) }] },
        primitive: { topology: 'triangle-list', cullMode: 'none' },
        depthStencil: { format: 'depth24plus', depthWriteEnabled: !transparent, depthCompare: 'less' },
      })
      this.pipeline=makePipeline(false)
      this.transparentPipeline=makePipeline(true)
      this.uniformBuffer = device.createBuffer({ size: 48 + 16 + 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST })
      this.bindGroup = device.createBindGroup({
        layout: this.pipeline.getBindGroupLayout(0),
        entries: [{ binding: 0, resource: { buffer: this.uniformBuffer } }],
      })
      this.transparentBindGroup=device.createBindGroup({layout:this.transparentPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:this.uniformBuffer}}]})
      this.deviceLost = false
      device.lost.then(info => { this.deviceLost = true; this.fail(info?.message??'WebGPU device lost') }).catch(error => { this.deviceLost = true; this.fail(error) })
      device.addEventListener('uncapturederror',event=>this.fail(event.error))
      this.device = device
      this.context = context
      return true
    } catch {
      this.destroy()
      return false
    }
  }

  get ready(): boolean { return !!this.device && !!this.pipeline }

  setBodies(bodies: readonly SolidGpuBody[]): void {
    try { this.uploadBodies(bodies) } catch(error) { this.fail(error) }
  }
  private uploadBodies(bodies: readonly SolidGpuBody[]): void {
    const device = this.device
    if (!device) return
    let total = 0
    for (const body of bodies) {const opacity=body.opacity??1;if(!Number.isFinite(opacity)||opacity<0||opacity>1)throw Error('Invalid material opacity');total += body.positions.length / 3}
    const data = new Float32Array(total * 13)
    this.opaqueCount=0;this.transparentTriangles=[];this.dragOffsets.clear();this.transparencyLimited=false;this.onTransparencyLimit?.(false)
    const ordered: SolidGpuBody[] = []
    for (let i = 0; i < bodies.length; i++) {
      const b = bodies[i]!
      if ((b.opacity ?? 1) >= 1) ordered.push(b)
    }
    for (let i = 0; i < bodies.length; i++) {
      const b = bodies[i]!
      const op = b.opacity ?? 1
      if (op > 0 && op < 1) ordered.push(b)
    }
    this.ranges = new Map()
    let offset = 0
    for (const body of ordered) {
      const count = body.positions.length / 3
      this.ranges.set(body.id, { start: offset / 13, count })
      const alpha=body.opacity??1
      if(!Number.isFinite(alpha)||alpha<0||alpha>1)throw Error('Invalid material opacity')
      if(alpha===1)this.opaqueCount+=count
      else if(alpha>0)for(let i=0;i<count;i+=3){
        this.transparentTriangles.push({first:offset/13+i,body:body.id})
      }
      const c0 = body.color ? body.color[0] : 1
      const c1 = body.color ? body.color[1] : 1
      const c2 = body.color ? body.color[2] : 1
      const met = body.metallic ?? 0
      const rough = body.roughness ?? (body.metallic !== undefined ? 0.5 : -1)
      for (let i = 0; i < count; i++) {
        const pIdx = i * 3
        data[offset++] = body.positions[pIdx]
        data[offset++] = body.positions[pIdx + 1]
        data[offset++] = body.positions[pIdx + 2]
        data[offset++] = body.normals[pIdx]
        data[offset++] = body.normals[pIdx + 1]
        data[offset++] = body.normals[pIdx + 2]
        data[offset++] = body.hues[(i / 3) | 0]
        data[offset++] = c0
        data[offset++] = c1
        data[offset++] = c2
        data[offset++] = met
        data[offset++] = rough
        data[offset++] = alpha
      }
    }
    this.vertexBuffer?.destroy()
    this.transparentVertices?.destroy();this.transparentVertices=null

    this.vertexBuffer = null
    this.vertexCount = total
    this.packed = data
    try { this.rebuildTransparency() } catch(error) { this.fail(error); return }
    if (total === 0) { this.requestFrame(); return }
    this.vertexBuffer = device.createBuffer({ size: Math.max(48, data.byteLength), usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
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
    const dx = delta[0], dy = delta[1], dz = delta[2]
    const deltaCopy: readonly [number, number, number] = [dx, dy, dz]
    for (const id of ids) {
      const range = this.ranges.get(id)
      if (!range) continue
      this.dragOffsets.set(id, deltaCopy)
      const needed = range.count * 13
      if (!this.dragScratch || this.dragScratch.length < needed) {
        this.dragScratch = new Float32Array(needed)
      }
      const slice = this.dragScratch.length === needed ? this.dragScratch : this.dragScratch.subarray(0, needed)
      for (let i = 0; i < range.count; i++) {
        const from = (range.start + i) * 13, to = i * 13
        slice[to] = packed[from] + dx
        slice[to + 1] = packed[from + 1] + dy
        slice[to + 2] = packed[from + 2] + dz
        for (let k = 3; k < 13; k++) slice[to + k] = packed[from + k]
      }
      device.queue.writeBuffer(buffer, range.start * 13 * 4, slice)
    }
    if(!this.transparencyLimited)try { this.rebuildTransparency() } catch(error) { this.fail(error); return }
    this.requestFrame()
  }

  private rebuildTransparency(): void {
    this.transparentVertices?.destroy();this.transparentVertices=null;this.transparentTree=null;this.transparentDirection=null;this.transparentData=null
    if(!this.device||!this.packed||!this.transparentTriangles.length)return
    const packed=this.packed
    const fragments:TransparentFragment[]=new Array(this.transparentTriangles.length)
    for(let ti=0;ti<this.transparentTriangles.length;ti++){
      const t=this.transparentTriangles[ti]!
      const delta=this.dragOffsets.get(t.body)
      const dx=delta?delta[0]:0, dy=delta?delta[1]:0, dz=delta?delta[2]:0
      const v0=new Array<number>(13), v1=new Array<number>(13), v2=new Array<number>(13)
      const b0=t.first*13, b1=b0+13, b2=b0+26
      v0[0]=packed[b0]!+dx; v0[1]=packed[b0+1]!+dy; v0[2]=packed[b0+2]!+dz
      v1[0]=packed[b1]!+dx; v1[1]=packed[b1+1]!+dy; v1[2]=packed[b1+2]!+dz
      v2[0]=packed[b2]!+dx; v2[1]=packed[b2+1]!+dy; v2[2]=packed[b2+2]!+dz
      for(let k=3;k<13;k++){
        v0[k]=packed[b0+k]!
        v1[k]=packed[b1+k]!
        v2[k]=packed[b2+k]!
      }
      fragments[ti]={owner:t.body,triangle:[v0,v1,v2] as unknown as TransparentFragment['triangle']}
    }
    try { this.transparentTree=new TransparentBsp(fragments) } catch(error) {
      if(!(error instanceof Error)||!/^Transparency (?:(?:input|operation|fragment) limit exceeded|(?:input|output) exceeds transport limit)$/.test(error.message))throw error
      // Native admission remains bounded. Show the same geometry opaque until the next
      // scene upload instead of destroying a healthy GPU device for a temporary preview.
      this.transparencyLimited=true;this.onTransparencyLimit?.(true)
      for(const triangle of this.transparentTriangles)for(let i=0;i<3;i++)packed[(triangle.first+i)*13+12]=1
      this.opaqueCount+=this.transparentTriangles.length*3
      if(this.vertexBuffer){
        const display=new Float32Array(packed)
        for(const [id,delta] of this.dragOffsets){
          const range=this.ranges.get(id)!
          for(let i=0;i<range.count;i++)for(let k=0;k<3;k++)display[(range.start+i)*13+k]+=delta[k]
        }
        this.device.queue.writeBuffer(this.vertexBuffer,0,display)
      }
      return
    }
    this.transparentData=new Float32Array(this.transparentTree.fragmentCount*3*13)
    if(this.transparentTree.fragmentCount)this.transparentVertices=this.device.createBuffer({size:this.transparentTree.fragmentCount*3*13*4,usage:GPUBufferUsage.VERTEX|GPUBufferUsage.COPY_DST})
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
      this.depthView = null
    }
    this.requestFrame()
  }

  requestFrame(): void {
    if (this.frame || this.disposed) return
    this.frame = requestAnimationFrame(() => { this.frame = 0; try { this.render() } catch(error) { this.fail(error) } })
  }

  private render(): void {
    const device = this.device, context = this.context, pipeline = this.pipeline
    if (this.deviceLost || !device || !context || !pipeline || !this.uniformBuffer || !this.bindGroup) return
    const width = this.canvas.width, height = this.canvas.height
    if (!this.depth || !this.depthView) {
      this.depth?.destroy()
      this.depth = device.createTexture({ size: [width, height], format: 'depth24plus', usage: GPUTextureUsage.RENDER_ATTACHMENT })
      this.depthView = this.depth.createView()
    }
    const [vx, vy, size] = this.view.viewBox
    const rotation = solidRotationColumns(this.view.camera, scratchRotation)
    const uniforms = scratchUniforms
    uniforms.set(rotation, 0)
    uniforms[12] = vx; uniforms[13] = vy; uniforms[14] = size; uniforms[15] = Math.max(size, 1) * 16
    uniforms[16] = width; uniforms[17] = height; uniforms[18] = 0; uniforms[19] = 0
    device.queue.writeBuffer(this.uniformBuffer, 0, uniforms)
    const encoder = device.createCommandEncoder()
    const pass = encoder.beginRenderPass({
      colorAttachments: [{ view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: 'clear', storeOp: 'store' }],
      depthStencilAttachment: { view: this.depthView, depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store' },
    })
    let drawCalls = 0
    let triangles = 0
    if (this.vertexBuffer && this.vertexCount) {
      pass.setPipeline(pipeline)
      pass.setBindGroup(0, this.bindGroup)
      pass.setVertexBuffer(0, this.vertexBuffer)
      pass.draw(this.opaqueCount)
      drawCalls++
      triangles += (this.opaqueCount / 3) | 0
      if(this.transparentVertices&&this.transparentPipeline&&this.transparentBindGroup){
        const direction=[rotation[2]!,rotation[6]!,rotation[10]!] as [number,number,number]
        if(!this.transparentDirection||direction[0]!==this.transparentDirection[0]||direction[1]!==this.transparentDirection[1]||direction[2]!==this.transparentDirection[2]){
          this.transparentTree!.writeOrdered(direction,this.transparentData!)
          device.queue.writeBuffer(this.transparentVertices,0,this.transparentData!)
          this.transparentDirection=direction
        }
        pass.setPipeline(this.transparentPipeline);pass.setBindGroup(0,this.transparentBindGroup)
        pass.setVertexBuffer(0,this.transparentVertices);pass.draw(this.transparentTree!.fragmentCount*3)
        drawCalls++
        triangles += this.transparentTree!.fragmentCount
      }
    }
    pass.end()
    this.lastDrawCalls = drawCalls
    this.lastTriangleCount = triangles
    const drawStarted = performance.now()
    device.queue.submit([encoder.finish()])
    // Resolves once the GPU has finished; measured even when frame callbacks are throttled.
    void device.queue.onSubmittedWorkDone().then(() => { this.lastDrawMs = performance.now() - drawStarted }).catch(error => this.fail(error))
  }

  destroy(): void {
    this.disposed = true
    if (this.frame) cancelAnimationFrame(this.frame)
    this.vertexBuffer?.destroy()
    this.depth?.destroy()
    this.transparentVertices?.destroy()
    this.uniformBuffer?.destroy()
    try { this.context?.unconfigure() } catch { /* context may be lost */ }
    this.device?.destroy()
    this.device = null
    this.context=null;this.pipeline=null;this.transparentPipeline=null
    this.bindGroup=null;this.transparentBindGroup=null;this.uniformBuffer=null
    this.vertexBuffer=null;this.transparentVertices=null;this.depth=null;this.depthView=null;this.dragScratch=null
    this.packed=null;this.transparentTree=null;this.transparentData=null;this.transparentDirection=null
    this.transparentTriangles=[];this.ranges.clear();this.dragOffsets.clear()
    this.vertexCount=0;this.opaqueCount=0;this.frame=0
  }
}

/**
 * Expands an indexed mesh into a flat triangle list with crease-aware smoothed normals: normals of triangles
 * meeting at the same position are averaged only when they differ by less than `creaseDegrees`, so curved
 * faces shade smoothly while box edges stay sharp.
 */
export function smoothTriangleList(positions: ArrayLike<number>, indices: ArrayLike<number>, creaseDegrees = 40): { positions: Float32Array; normals: Float32Array } {
  const posArray = positions instanceof Float64Array || positions instanceof Float32Array ? positions : Float64Array.from(positions)
  const idxArray = indices instanceof Uint32Array ? indices : Uint32Array.from(indices)
  return smoothTriangleListInKernel(posArray, idxArray, Math.cos(creaseDegrees * Math.PI / 180))
}

