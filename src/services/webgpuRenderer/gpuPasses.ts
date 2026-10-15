/**
 * Optional hardware passes owned by the WebGPU renderer: GPU CSG preview,
 * ID-buffer picking, weighted-blended OIT, the zero-copy buffer arena and the
 * multi-plane slicer. Every pass is created lazily, so a renderer that never
 * enables them pays nothing, and CPU fallbacks stay in the pass modules.
 */
import { GpuCsgRenderer } from '../gpuCsgRenderer'
import type { GpuCsgTree } from '../gpuCsgTypes'
import { GpuPicker, type GpuPickerMesh } from '../gpuPicker'
import { GpuWboitRenderer } from '../gpuWboit'
import { GpuBufferArena } from '../gpuZeroCopyPipeline'
import { runGpuMeshSlicer, type SlicerResult } from '../gpuSlicer'
import { resolveMeshShaderId, type ShadingModel } from '../rendererContracts'
import type { Mat4 } from '../math3d'

export type TransparencyMode = 'sorted' | 'wboit'

export interface PassMesh {
  shadingModel: ShadingModel
  bg: GPUBindGroup
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  morphSlot?: GPUBuffer | null
}

export interface SliceMesh {
  visible: boolean
  vertices: Float32Array
  indices: Uint32Array
}

export interface WboitEncodeInput {
  dev: GPUDevice
  enc: GPUCommandEncoder
  depthView: GPUTextureView
  colorView: GPUTextureView
  fmt: GPUTextureFormat
  width: number
  height: number
  sceneBG: GPUBindGroup
  shadowBG?: GPUBindGroup | null
  objectLayout: GPUPipelineLayout
  vertexLayouts: GPUVertexBufferLayout[]
  extraBindGroupForShader(shaderId: ReturnType<typeof resolveMeshShaderId>): GPUBindGroup | undefined
}

export class RendererGpuPasses {
  csgTree: GpuCsgTree | null = null
  transparencyMode: TransparencyMode = 'sorted'
  private csgRenderer: GpuCsgRenderer | null = null
  private csgDepthStencil: GPUTexture | null = null
  private csgDepthStencilView: GPUTextureView | null = null
  private picker: GpuPicker | null = null
  private wboit: GpuWboitRenderer | null = null
  private arena: GpuBufferArena | null = null

  get csgActive(): boolean {
    return this.csgTree !== null && this.csgTree.terms.length > 0
  }

  private previewSequence = 0

  /**
   * Parses live source into a CSG preview tree; syntax errors during typing
   * are not fatal. The OpenSCAD lowering is loaded on first use, and a stale
   * parse never overwrites a newer one.
   */
  async previewSource(source: string): Promise<boolean> {
    const sequence = ++this.previewSequence
    try {
      const { parseOpenScadCsgTree } = await import('../gpuCsgParse')
      const tree = parseOpenScadCsgTree(source)
      if (!tree || sequence !== this.previewSequence) return false
      this.csgTree = tree
      return true
    } catch {
      return false
    }
  }

  get wboitRenderer(): GpuWboitRenderer {
    if (!this.wboit) this.wboit = new GpuWboitRenderer()
    return this.wboit
  }

  bufferArena(dev: GPUDevice): GpuBufferArena {
    if (!this.arena) this.arena = new GpuBufferArena(dev)
    return this.arena
  }

  pick(dev: GPUDevice, meshes: readonly GpuPickerMesh[], options: Parameters<GpuPicker['pick']>[2], px: number, py: number) {
    if (!this.picker) this.picker = new GpuPicker()
    return this.picker.pick(dev, meshes as GpuPickerMesh[], options, px, py)
  }

  async slice(meshes: readonly SliceMesh[], options: {
    planeNormal: [number, number, number]
    firstPlaneOffset: number
    layerThickness: number
    numLayers: number
  }): Promise<Array<{ meshIndex: number; slice: SlicerResult }>> {
    const results: Array<{ meshIndex: number; slice: SlicerResult }> = []
    for (let i = 0; i < meshes.length; i++) {
      const m = meshes[i]
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

  /** Drops the CSG depth/stencil target after the canvas resized or lost its depth texture. */
  releaseDepth() {
    this.csgDepthStencil?.destroy()
    this.csgDepthStencil = null
    this.csgDepthStencilView = null
  }

  encodeCsg(dev: GPUDevice, enc: GPUCommandEncoder, colorView: GPUTextureView, fmt: GPUTextureFormat,
    width: number, height: number, viewProjection: Mat4, eye: ArrayLike<number>) {
    if (!this.csgTree) return
    if (!this.csgDepthStencil || this.csgDepthStencil.width !== width || this.csgDepthStencil.height !== height) {
      this.csgDepthStencil?.destroy()
      this.csgDepthStencil = dev.createTexture({
        size: [width, height],
        format: 'depth24plus-stencil8',
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      })
      this.csgDepthStencilView = this.csgDepthStencil.createView()
    }
    if (!this.csgDepthStencilView) return
    if (!this.csgRenderer) this.csgRenderer = new GpuCsgRenderer(fmt)
    this.csgRenderer.renderCsg(dev, enc, colorView, this.csgDepthStencilView, this.csgTree, {
      viewProjection,
      eyePosition: [eye[0], eye[1], eye[2]],
      lightDirection: [0.55, 0.75, 0.45],
    })
  }

  /**
   * Runs after the main pass ends so the encoder never has two render passes
   * open and the depth view holds full opaque depth. Accumulation is
   * order-independent, so draws need no per-frame sort.
   */
  encodeWboit(draws: readonly PassMesh[], input: WboitEncodeInput) {
    if (!draws.length) return
    const wboit = this.wboitRenderer
    wboit.ensureTextures(input.dev, input.width, input.height)
    const pass = wboit.beginWboitPass(input.enc, input.depthView)
    pass.setBindGroup(0, input.sceneBG)
    if (input.shadowBG) pass.setBindGroup(2, input.shadowBG)
    let activeModel: ShadingModel | null = null
    for (const g of draws) {
      if (g.shadingModel !== activeModel) {
        activeModel = g.shadingModel
        const shaderId = resolveMeshShaderId(activeModel)
        pass.setPipeline(wboit.getWboitPipeline(input.dev, shaderId, input.objectLayout, input.vertexLayouts))
        const extraGroup = input.extraBindGroupForShader(shaderId)
        if (extraGroup) pass.setBindGroup(2, extraGroup)
      }
      pass.setBindGroup(1, g.bg)
      pass.setVertexBuffer(0, g.vb)
      pass.setVertexBuffer(1, g.morphSlot!)
      pass.setIndexBuffer(g.ib, 'uint32')
      pass.drawIndexed(g.ic)
    }
    pass.end()
    wboit.composite(input.dev, input.enc, input.colorView, input.fmt)
  }

  /** Drops the live preview, including any parse still in flight. */
  clearCsg() {
    this.previewSequence++
    this.csgTree = null
  }

  destroy() {
    this.previewSequence++
    this.csgRenderer?.destroy()
    this.csgRenderer = null
    this.releaseDepth()
    this.csgTree = null
    this.picker?.destroy()
    this.picker = null
    this.wboit?.destroy()
    this.wboit = null
    this.arena?.destroy()
    this.arena = null
  }
}
