/**
 * GPU Hardware Picking — Offscreen 1x1 R32Uint ID-Buffer Readback.
 *
 * Implements hardware-accelerated O(1) object picking on WebGPU.
 * Uses a 1x1 pixel scissor/projection pick matrix so the GPU rasterizer
 * evaluates only the single ray/pixel under the cursor with full depth
 * testing and section plane clipping, completely bypassing CPU BVH
 * traversal overhead.
 */

import { multiplyMat4, identityMat4 } from './gpuCsgPreview'

/* ── Pick Matrix Math ─────────────────────────────────── */

/**
 * Creates an orthographic/zoom pick matrix that maps a 1x1 (or WxH) window around
 * (pixelX, pixelY) on the viewport directly to WebGPU NDC [-1, 1]^2.
 * Depth (Z) remains unchanged in [0, 1].
 */
export function createPickMatrix(
  viewportWidth: number,
  viewportHeight: number,
  pixelX: number,
  pixelY: number,
  windowSize = 1,
): Float32Array {
  const w = Math.max(1, viewportWidth)
  const h = Math.max(1, viewportHeight)
  const win = Math.max(1e-4, windowSize)

  // Center of picked pixel in normalized device coordinates [-1, 1]
  const cx = ((pixelX + 0.5) / w) * 2 - 1
  const cy = 1 - ((pixelY + 0.5) / h) * 2

  // Scaling factor: a pixel of NDC width 2/w expands to full NDC width 2
  const sx = w / win
  const sy = h / win

  const m = identityMat4()
  // Row 0: x' = sx * (x - cx)
  m[0] = sx
  m[3] = -sx * cx
  // Row 1: y' = sy * (y - cy)
  m[5] = sy
  m[7] = -sy * cy
  // Row 2 & 3: z and w pass through
  m[10] = 1
  m[15] = 1
  return m
}

/* ── WebGPU Picking WGSL Shader ───────────────────────── */

export const GPU_PICKING_WGSL = /* wgsl */`
struct PickScene {
  vp: mat4x4f,
  section: vec4f, // xyz: normal, w: offset
  options: vec4f, // x: sectionEnabled (1.0 or 0.0)
};

struct PickObj {
  model: mat4x4f,
  id: u32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
};

@group(0) @binding(0) var<uniform> sc: PickScene;
@group(1) @binding(0) var<uniform> ob: PickObj;

struct V {
  @builtin(position) p: vec4f,
  @location(0) w: vec3f,
};

@vertex fn vs(@location(0) pos: vec3f) -> V {
  let wp = (ob.model * vec4f(pos, 1.0)).xyz;
  return V(sc.vp * vec4f(wp, 1.0), wp);
}

@fragment fn fs(v: V) -> @location(0) u32 {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) {
    discard;
  }
  return ob.id;
}
`

/* ── Picking Types & Options ──────────────────────────── */

export interface GpuPickerMesh {
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  transform: Float32Array
  visible: boolean
}

export interface GpuPickerSceneParams {
  cameraViewProjection: Float32Array
  viewportWidth: number
  viewportHeight: number
  sectionNormal?: [number, number, number]
  sectionOffset?: number
  sectionEnabled?: boolean
}

/* ── GpuPicker Class ──────────────────────────────────── */

export class GpuPicker {
  private dev: GPUDevice | null = null
  private colorTexture: GPUTexture | null = null
  private depthTexture: GPUTexture | null = null
  private readbackBuffer: GPUBuffer | null = null
  private pipeline: GPURenderPipeline | null = null
  private sceneBGL: GPUBindGroupLayout | null = null
  private objBGL: GPUBindGroupLayout | null = null
  private sceneUB: GPUBuffer | null = null
  private sceneBG: GPUBindGroup | null = null
  private objUBs: GPUBuffer[] = []
  private objBGs: GPUBindGroup[] = []

  private ensureResources(dev: GPUDevice) {
    if (this.dev === dev && this.pipeline && this.colorTexture) return
    this.destroy()
    this.dev = dev

    // 1x1 Color attachment for R32Uint ID readback
    this.colorTexture = dev.createTexture({
      size: [1, 1],
      format: 'r32uint',
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC,
    })

    // 1x1 Depth attachment
    this.depthTexture = dev.createTexture({
      size: [1, 1],
      format: 'depth24plus',
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    })

    // Readback buffer (WebGPU requires bytesPerRow >= 256 for buffer copies)
    this.readbackBuffer = dev.createBuffer({
      size: 256,
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    })

    this.sceneBGL = dev.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      ],
    })

    this.objBGL = dev.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      ],
    })

    const pipelineLayout = dev.createPipelineLayout({
      bindGroupLayouts: [this.sceneBGL, this.objBGL],
    })

    const module = dev.createShaderModule({ code: GPU_PICKING_WGSL })

    this.pipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: {
        module,
        entryPoint: 'vs',
        buffers: [{
          arrayStride: 24, // Stride 24 matching standard mesh vertex layout
          attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x3' }],
        }],
      },
      fragment: {
        module,
        entryPoint: 'fs',
        targets: [{ format: 'r32uint' }],
      },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: 'depth24plus',
        depthWriteEnabled: true,
        depthCompare: 'less',
      },
    })

    this.sceneUB = dev.createBuffer({
      size: 256,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    })

    this.sceneBG = dev.createBindGroup({
      layout: this.sceneBGL,
      entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
    })
  }

  private ensureObjectBindGroup(dev: GPUDevice, index: number, mesh: GpuPickerMesh): GPUBindGroup {
    if (!this.objUBs[index]) {
      this.objUBs[index] = dev.createBuffer({
        size: 256,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      })
      this.objBGs[index] = dev.createBindGroup({
        layout: this.objBGL!,
        entries: [{ binding: 0, resource: { buffer: this.objUBs[index] } }],
      })
    }

    const data = new ArrayBuffer(256)
    const f32 = new Float32Array(data)
    const u32 = new Uint32Array(data)

    // model: mat4x4f (16 floats = 64 bytes)
    f32.set(mesh.transform, 0)
    // id: u32 (1-indexed, so 0 is background)
    u32[16] = index + 1

    dev.queue.writeBuffer(this.objUBs[index], 0, data)
    return this.objBGs[index]
  }

  /**
   * Performs an asynchronous hardware picking pass at (pixelX, pixelY).
   * Returns the zero-indexed meshIndex hit by the ray, or null if background.
   */
  async pick(
    dev: GPUDevice,
    meshes: readonly GpuPickerMesh[],
    sceneParams: GpuPickerSceneParams,
    pixelX: number,
    pixelY: number,
  ): Promise<number | null> {
    if (meshes.length === 0) return null
    this.ensureResources(dev)

    // Compute 1x1 pick projection: VP_pick = M_pick * VP_camera
    const pickMat = createPickMatrix(sceneParams.viewportWidth, sceneParams.viewportHeight, pixelX, pixelY, 1)
    const pickVP = multiplyMat4(pickMat, sceneParams.cameraViewProjection)

    // Upload Scene uniforms
    const sceneData = new Float32Array(32)
    // vp: mat4x4f
    sceneData.set(pickVP, 0)
    // section: vec4f
    const normal = sceneParams.sectionNormal ?? [0, 0, 1]
    sceneData[16] = normal[0]
    sceneData[17] = normal[1]
    sceneData[18] = normal[2]
    sceneData[19] = sceneParams.sectionOffset ?? 0
    // options: vec4f (x: sectionEnabled)
    sceneData[20] = sceneParams.sectionEnabled ? 1 : 0
    dev.queue.writeBuffer(this.sceneUB!, 0, sceneData)

    const encoder = dev.createCommandEncoder()
    const colorView = this.colorTexture!.createView()
    const depthView = this.depthTexture!.createView()

    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: colorView,
        clearValue: { r: 0, g: 0, b: 0, a: 0 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
      depthStencilAttachment: {
        view: depthView,
        depthClearValue: 1.0,
        depthLoadOp: 'clear',
        depthStoreOp: 'store',
      },
    })

    pass.setPipeline(this.pipeline!)
    pass.setBindGroup(0, this.sceneBG!)

    for (let index = 0; index < meshes.length; index++) {
      const mesh = meshes[index]
      if (!mesh.visible || mesh.ic === 0) continue
      const objBG = this.ensureObjectBindGroup(dev, index, mesh)
      pass.setBindGroup(1, objBG)
      pass.setVertexBuffer(0, mesh.vb)
      pass.setIndexBuffer(mesh.ib, 'uint32')
      pass.drawIndexed(mesh.ic)
    }

    pass.end()

    // Copy 1x1 pixel into readback buffer
    encoder.copyTextureToBuffer(
      { texture: this.colorTexture!, origin: [0, 0, 0] },
      { buffer: this.readbackBuffer!, bytesPerRow: 256, rowsPerImage: 1 },
      [1, 1, 1],
    )

    dev.queue.submit([encoder.finish()])

    // Read back 4 bytes from GPU memory
    await this.readbackBuffer!.mapAsync(GPUMapMode.READ)
    const array = new Uint32Array(this.readbackBuffer!.getMappedRange())
    const rawId = array[0]
    this.readbackBuffer!.unmap()

    return rawId > 0 ? rawId - 1 : null
  }

  destroy() {
    this.colorTexture?.destroy()
    this.depthTexture?.destroy()
    this.readbackBuffer?.destroy()
    this.sceneUB?.destroy()
    for (const ub of this.objUBs) ub?.destroy()
    this.objUBs = []
    this.objBGs = []
    this.colorTexture = null
    this.depthTexture = null
    this.readbackBuffer = null
    this.sceneUB = null
    this.sceneBG = null
    this.pipeline = null
    this.dev = null
  }
}
