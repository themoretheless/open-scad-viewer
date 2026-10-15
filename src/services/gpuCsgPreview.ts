/**
 * GPU OpenCSG Preview — WebGPU Depth/Stencil Boolean Evaluation.
 *
 * Implements real-time Goldfeather / SCS (Sequenced Convex Subtractions)
 * evaluation on WebGPU hardware. Avoids CPU B-Rep polygonal re-tessellation
 * bottlenecks during interactive preview and parameter editing.
 */


import type { GpuCsgPrimitive, GpuCsgTerm, GpuCsgTree } from './gpuCsgTypes'
import { CSG_COMMON_WGSL, CSG_DEPTH_WGSL, CSG_SHADE_WGSL } from './gpuCsgShaders'
import { generateUnitCube, generateUnitCylinder, generateUnitSphere, normalMatrixFromModel, multiplyMat4, type GeometryData } from './gpuCsgPrimitives'
export * from './gpuCsgTypes'
export * from './gpuCsgPrimitives'
export * from './gpuCsgParse'
export * from './gpuCsgShaders'
export * from './gpuCsgGeometryCache'
import { GpuCsgGeometryCache } from './gpuCsgGeometryCache'

export interface CsgPipelines {
  sceneBGL: GPUBindGroupLayout
  objBGL: GPUBindGroupLayout
  pipelineLayout: GPUPipelineLayout
  /** Pass 1: Renders base primitive into depth buffer */
  depthPipeline: GPURenderPipeline
  /** Pass 2: Evaluates cutter parity in stencil buffer */
  parityPipeline: GPURenderPipeline
  /** Pass 3: Shades base primitive where stencil == 0 */
  shadeBasePipeline: GPURenderPipeline
  /** Pass 4: Shades cut surfaces (back faces of cutters) */
  shadeCutPipeline: GPURenderPipeline
  /** Pass 3 (Intersection): Shades primitive where stencil > 0 */
  shadeIntersectPipeline: GPURenderPipeline
}

export class GpuCsgPipelineManager {
  private pipelines: CsgPipelines | null = null
  private colorFormat: GPUTextureFormat = 'bgra8unorm'
  private depthFormat: GPUTextureFormat = 'depth24plus-stencil8'

  constructor(colorFormat: GPUTextureFormat = 'bgra8unorm') {
    this.colorFormat = colorFormat
  }

  getPipelines(dev: GPUDevice): CsgPipelines {
    if (this.pipelines) return this.pipelines

    const sceneBGL = dev.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      ],
    })

    const objBGL = dev.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      ],
    })

    const pipelineLayout = dev.createPipelineLayout({
      bindGroupLayouts: [sceneBGL, objBGL],
    })

    const depthModule = dev.createShaderModule({ code: CSG_DEPTH_WGSL })
    const shadeModule = dev.createShaderModule({ code: CSG_SHADE_WGSL })

    const vertexBuffers: GPUVertexBufferLayout[] = [{
      arrayStride: 24, // 6 floats * 4 bytes
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },  // position
        { shaderLocation: 1, offset: 12, format: 'float32x3' }, // normal
      ],
    }]

    // 1. Pass 1: Base Depth Pipeline
    const depthPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: depthModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: depthModule, entryPoint: 'fs', targets: [] },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'less',
        stencilFront: { passOp: 'keep' },
        stencilBack: { passOp: 'keep' },
      },
    })

    // 2. Pass 2: Cutter Parity Pipeline (Counts Jordan winding in stencil)
    const parityPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: depthModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: depthModule, entryPoint: 'fs', targets: [] },
      primitive: { topology: 'triangle-list', cullMode: 'none' }, // Front and back faces
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: false,
        depthCompare: 'less',
        stencilFront: {
          compare: 'always',
          passOp: 'increment-wrap',
          failOp: 'keep',
          depthFailOp: 'keep',
        },
        stencilBack: {
          compare: 'always',
          passOp: 'decrement-wrap',
          failOp: 'keep',
          depthFailOp: 'keep',
        },
      },
    })

    // 3. Pass 3: Base Shade Pipeline (where stencil == 0)
    const shadeBasePipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: shadeModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: shadeModule, entryPoint: 'fs', targets: [{ format: this.colorFormat }] },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'equal',
        stencilFront: {
          compare: 'equal',
          passOp: 'keep',
        },
        stencilBack: {
          compare: 'equal',
          passOp: 'keep',
        },
      },
    })

    // 4. Pass 4: Cut Shade Pipeline (back faces of cutters)
    const shadeCutPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: shadeModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: shadeModule, entryPoint: 'fs', targets: [{ format: this.colorFormat }] },
      primitive: { topology: 'triangle-list', cullMode: 'front' }, // Inside cutter faces
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'less',
        stencilFront: { passOp: 'keep' },
        stencilBack: { passOp: 'keep' },
      },
    })

    // 5. Pass 3 (Intersection): Base Shade where stencil > 0
    const shadeIntersectPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: shadeModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: shadeModule, entryPoint: 'fs', targets: [{ format: this.colorFormat }] },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'equal',
        stencilFront: {
          compare: 'not-equal', // stencil != 0 (i.e. > 0)
          passOp: 'keep',
        },
        stencilBack: {
          compare: 'not-equal',
          passOp: 'keep',
        },
      },
    })

    this.pipelines = {
      sceneBGL,
      objBGL,
      pipelineLayout,
      depthPipeline,
      parityPipeline,
      shadeBasePipeline,
      shadeCutPipeline,
      shadeIntersectPipeline,
    }
    return this.pipelines
  }
}
/* ── WebGPU CSG Preview Renderer ──────────────────────── */

export interface RenderCsgSceneOptions {
  viewProjection: Float32Array
  eyePosition: [number, number, number]
  lightDirection?: [number, number, number]
  ambientColor?: [number, number, number]
  cutTintColor?: [number, number, number]
}

export class GpuCsgRenderer {
  readonly pipelineManager: GpuCsgPipelineManager
  readonly geometryCache = new GpuCsgGeometryCache()
  private sceneUB: GPUBuffer | null = null
  private sceneBG: GPUBindGroup | null = null
  private readonly sceneScratch = new Float32Array(64)
  private readonly objScratch = new Float32Array(64)
  private readonly nmatScratch = new Float32Array(16)

  constructor(colorFormat: GPUTextureFormat = 'bgra8unorm') {
    this.pipelineManager = new GpuCsgPipelineManager(colorFormat)
  }

  private ensureSceneUniforms(dev: GPUDevice, pipelines: CsgPipelines, opts: RenderCsgSceneOptions): GPUBindGroup {
    if (!this.sceneUB) {
      this.sceneUB = dev.createBuffer({
        size: 256, // Padded to uniform alignment
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      })
      this.sceneBG = dev.createBindGroup({
        layout: pipelines.sceneBGL,
        entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
      })
    }

    const data = this.sceneScratch
    // viewProjection: mat4x4f (16 floats)
    data.set(opts.viewProjection, 0)
    // eyePosition: vec4f (4 floats)
    data[16] = opts.eyePosition[0]
    data[17] = opts.eyePosition[1]
    data[18] = opts.eyePosition[2]
    data[19] = 1.0
    // lightDirection: vec4f
    const light = opts.lightDirection ?? [0.5, 0.8, 1.0]
    data[20] = light[0]
    data[21] = light[1]
    data[22] = light[2]
    data[23] = 0.0
    // ambientColor: vec4f
    const ambient = opts.ambientColor ?? [0.25, 0.25, 0.25]
    data[24] = ambient[0]
    data[25] = ambient[1]
    data[26] = ambient[2]
    data[27] = 1.0
    // cutTintColor: vec4f
    const cutTint = opts.cutTintColor ?? [0.95, 0.45, 0.15]
    data[28] = cutTint[0]
    data[29] = cutTint[1]
    data[30] = cutTint[2]
    data[31] = 1.0

    dev.queue.writeBuffer(this.sceneUB, 0, data)
    return this.sceneBG!
  }

  private createObjectBindGroup(
    dev: GPUDevice,
    pipelines: CsgPipelines,
    prim: GpuCsgPrimitive,
    isCutSurface: boolean,
  ): GPUBindGroup {
    const obUB = dev.createBuffer({
      size: 256,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    })

    const data = this.objScratch
    // model: mat4x4f (16 floats)
    data.set(prim.transform, 0)
    // normalMatrix: mat4x4f (16 floats)
    data.set(normalMatrixFromModel(prim.transform, this.nmatScratch), 16)
    // color: vec4f
    data[32] = prim.color[0]
    data[33] = prim.color[1]
    data[34] = prim.color[2]
    data[35] = prim.color[3]
    // params: vec4f
    data[36] = isCutSurface ? 1.0 : 0.0
    data[37] = prim.color[3]
    data[38] = 0.0
    data[39] = 0.0

    dev.queue.writeBuffer(obUB, 0, data)

    return dev.createBindGroup({
      layout: pipelines.objBGL,
      entries: [{ binding: 0, resource: { buffer: obUB } }],
    })
  }

  /**
   * Renders a GpuCsgTree into a render pass using the Goldfeather / SCS stencil algorithm.
   */
  renderCsg(
    dev: GPUDevice,
    encoder: GPUCommandEncoder,
    colorView: GPUTextureView,
    depthStencilView: GPUTextureView,
    tree: GpuCsgTree,
    opts: RenderCsgSceneOptions,
  ) {
    if (tree.terms.length === 0) return

    const pipelines = this.pipelineManager.getPipelines(dev)
    const sceneBG = this.ensureSceneUniforms(dev, pipelines, opts)

    for (let termIdx = 0; termIdx < tree.terms.length; termIdx++) {
      const term = tree.terms[termIdx]
      const isFirstTerm = termIdx === 0
      const baseMesh = this.geometryCache.getPrimitiveMesh(dev, term.positive)
      const baseBG = this.createObjectBindGroup(dev, pipelines, term.positive, false)

      // Fast path: pure union primitive without cutters
      if (term.cutters.length === 0) {
        const pass = encoder.beginRenderPass({
          colorAttachments: [{
            view: colorView,
            loadOp: isFirstTerm ? 'clear' : 'load',
            storeOp: 'store',
          }],
          depthStencilAttachment: {
            view: depthStencilView,
            depthClearValue: 1.0,
            depthLoadOp: isFirstTerm ? 'clear' : 'load',
            depthStoreOp: 'store',
            stencilClearValue: 0,
            stencilLoadOp: 'clear',
            stencilStoreOp: 'store',
          },
        })

        pass.setPipeline(pipelines.shadeBasePipeline)
        pass.setBindGroup(0, sceneBG)
        pass.setBindGroup(1, baseBG)
        pass.setVertexBuffer(0, baseMesh.vb)
        pass.setIndexBuffer(baseMesh.ib, 'uint32')
        pass.setStencilReference(0)
        pass.drawIndexed(baseMesh.indexCount)
        pass.end()
        continue
      }

      // Boolean Operation: Goldfeather / SCS Multi-Pass Evaluation

      // Pass 1: Base Depth
      const depthPass = encoder.beginRenderPass({
        colorAttachments: [],
        depthStencilAttachment: {
          view: depthStencilView,
          depthClearValue: 1.0,
          depthLoadOp: isFirstTerm ? 'clear' : 'load',
          depthStoreOp: 'store',
          stencilClearValue: 0,
          stencilLoadOp: 'clear',
          stencilStoreOp: 'store',
        },
      })
      depthPass.setPipeline(pipelines.depthPipeline)
      depthPass.setBindGroup(0, sceneBG)
      depthPass.setBindGroup(1, baseBG)
      depthPass.setVertexBuffer(0, baseMesh.vb)
      depthPass.setIndexBuffer(baseMesh.ib, 'uint32')
      depthPass.drawIndexed(baseMesh.indexCount)
      depthPass.end()

      // Pass 2: Cutter Parity Counting (depth write OFF, stencil parity wrap)
      const parityPass = encoder.beginRenderPass({
        colorAttachments: [],
        depthStencilAttachment: {
          view: depthStencilView,
          depthLoadOp: 'load',
          depthStoreOp: 'store',
          stencilLoadOp: 'load',
          stencilStoreOp: 'store',
        },
      })
      parityPass.setPipeline(pipelines.parityPipeline)
      parityPass.setBindGroup(0, sceneBG)
      for (const cutter of term.cutters) {
        const cutterMesh = this.geometryCache.getPrimitiveMesh(dev, cutter)
        const cutterBG = this.createObjectBindGroup(dev, pipelines, cutter, true)
        parityPass.setBindGroup(1, cutterBG)
        parityPass.setVertexBuffer(0, cutterMesh.vb)
        parityPass.setIndexBuffer(cutterMesh.ib, 'uint32')
        parityPass.drawIndexed(cutterMesh.indexCount)
      }
      parityPass.end()

      // Pass 3: Shade Visible Base Surface
      const shadePass = encoder.beginRenderPass({
        colorAttachments: [{
          view: colorView,
          loadOp: isFirstTerm ? 'clear' : 'load',
          storeOp: 'store',
        }],
        depthStencilAttachment: {
          view: depthStencilView,
          depthLoadOp: 'load',
          depthStoreOp: 'store',
          stencilLoadOp: 'load',
          stencilStoreOp: 'store',
        },
      })
      const isIntersection = term.operation === 'intersection'
      shadePass.setPipeline(isIntersection ? pipelines.shadeIntersectPipeline : pipelines.shadeBasePipeline)
      shadePass.setBindGroup(0, sceneBG)
      shadePass.setBindGroup(1, baseBG)
      shadePass.setVertexBuffer(0, baseMesh.vb)
      shadePass.setIndexBuffer(baseMesh.ib, 'uint32')
      shadePass.setStencilReference(0)
      shadePass.drawIndexed(baseMesh.indexCount)

      // Pass 4: Shade Cut Surfaces (back faces of cutters inside base volume)
      if (!isIntersection) {
        shadePass.setPipeline(pipelines.shadeCutPipeline)
        for (const cutter of term.cutters) {
          const cutterMesh = this.geometryCache.getPrimitiveMesh(dev, cutter)
          const cutterBG = this.createObjectBindGroup(dev, pipelines, cutter, true)
          shadePass.setBindGroup(1, cutterBG)
          shadePass.setVertexBuffer(0, cutterMesh.vb)
          shadePass.setIndexBuffer(cutterMesh.ib, 'uint32')
          shadePass.drawIndexed(cutterMesh.indexCount)
        }
      }

      shadePass.end()
    }
  }

  destroy() {
    this.geometryCache.destroy()
    this.sceneUB?.destroy()
    this.sceneUB = null
    this.sceneBG = null
  }
}
