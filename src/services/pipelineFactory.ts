/**
 * Pipeline factory for the WebGPU renderer: owns every bind group layout,
 * pipeline layout, and vertex buffer layout, plus the cached shader-module
 * and render-pipeline factories driven by the shader registry. Texture and
 * sampler lifecycles live in textureResources.ts; the dummy textures those
 * bind groups embed must exist before extraBindGroupForShader consumers
 * build their groups, so the renderer constructs the factory first and
 * initialises textures right after (see WebGPURenderer.buildPipelines).
 */
import {
  MESH_VERTEX_STRIDE,
  MORPH_VERTEX_STRIDE,
  getShader,
  immediateObjectShader,
  instancedObjectShader,
  supportsImmediateAddressSpace,
} from './shaders'
import type { ShaderDepthSpec, ShaderVariant, ShaderVertexLayout } from './shaders'

/** Optional per-flavor overrides for the cached pipeline factory. */
export interface PipelineFlavor {
  variant?: ShaderVariant
  blend?: 'none' | 'alpha'
  depth?: ShaderDepthSpec
  topology?: 'triangle-list' | 'line-list'
}
/** Transparent-pass depth state (read-only depth, no writes). */
export const TRANSPARENT_DEPTH: ShaderDepthSpec = { writeEnabled: false, compare: 'less' }
/** Deep-pass depth state (xray overlays: no writes, always pass). */
export const DEEP_DEPTH: ShaderDepthSpec = { writeEnabled: false, compare: 'always' }
/** Premultiplied-alpha blend shared by every transparent pipeline target. */
const ALPHA_BLEND: GPUBlendState = {
  color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
  alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
}

/** Group(2) bind groups a shader may need beyond scene/object. */
export interface ExtraBindGroups {
  matcapBG: GPUBindGroup | null
  envBG: GPUBindGroup | null
  shadowBG: GPUBindGroup | null
}

export class PipelineFactory {
  sceneBGL!: GPUBindGroupLayout
  objBGL!: GPUBindGroupLayout
  instanceBGL!: GPUBindGroupLayout
  /** group(2): matcap capture texture + sampler, shared by all matcap draws. */
  matcapBGL!: GPUBindGroupLayout
  /** group(2): equirect environment map + sampler, shared by all PBR draws. */
  envBGL!: GPUBindGroupLayout
  /** group(2)/group(1): key-light depth map + comparison sampler. */
  shadowBGL!: GPUBindGroupLayout
  /** Grid: scene + shadow group(1). */
  sceneLayout!: GPUPipelineLayout
  /** Line/overlay pipelines: scene group only (they declare no group(1)). */
  sceneOnlyLayout!: GPUPipelineLayout
  objectLayout!: GPUPipelineLayout
  /** Depth-only shadow pass: scene+object groups only, no shadow group. */
  shadowObjectLayout!: GPUPipelineLayout
  matcapObjectLayout!: GPUPipelineLayout
  envObjectLayout!: GPUPipelineLayout
  immediateObjectLayout: GPUPipelineLayout | null = null
  instanceLayout!: GPUPipelineLayout
  vertexLayouts!: Record<ShaderVertexLayout, GPUVertexBufferLayout[]>
  immediateObjectStyle = false
  private fmt: GPUTextureFormat = 'bgra8unorm'
  private readonly shaderModuleCache = new Map<string, GPUShaderModule>()
  private readonly pipelineCache = new Map<string, GPURenderPipeline>()

  /**
   * (Re)creates every layout against the current device and clears the
   * module/pipeline caches. Called once per renderer init; textures are
   * initialised afterwards because the matcap/env groups embed the shadow
   * dummy view.
   */
  build(dev: GPUDevice, fmt: GPUTextureFormat) {
    this.fmt = fmt
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
    this.matcapBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: {} },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: {} },
      // Shadow map shares the group: bindings 2/3 (see mesh_matcap.wgsl).
      { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'depth' } },
      { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'comparison' } },
    ] })
    this.envBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: {} },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: {} },
      // Shadow map shares the group: bindings 2/3 (see mesh_pbr.wgsl).
      { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'depth' } },
      { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'comparison' } },
    ] })
    this.shadowBGL = dev.createBindGroupLayout({ entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'depth' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'comparison' } },
    ] })
    // Every layout group must be bound before each draw. Only the grid samples
    // the shadow map among scene-kind shaders; line and overlay pipelines are
    // drawn after object groups occupy group(1), so they get a scene-only layout.
    this.sceneLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.shadowBGL] })
    this.sceneOnlyLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL] })
    this.objectLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL, this.shadowBGL] })
    this.shadowObjectLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL] })
    this.matcapObjectLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL, this.matcapBGL] })
    this.envObjectLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL, this.envBGL] })
    this.immediateObjectLayout = this.immediateObjectStyle
      ? dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.objBGL, this.shadowBGL], immediateSize: 16 })
      : null
    this.instanceLayout = dev.createPipelineLayout({ bindGroupLayouts: [this.sceneBGL, this.instanceBGL, this.shadowBGL] })

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
  }

  /** Cached shader modules, one per (shader id, variant). */
  private shaderModule(dev: GPUDevice, id: string, variant: ShaderVariant): GPUShaderModule {
    const key = `${id}|${variant}`
    let module = this.shaderModuleCache.get(key)
    if (!module) {
      const spec = getShader(id)
      const source = variant === 'immediate' ? immediateObjectShader(spec.source)
        : variant === 'instanced'
          ? instancedObjectShader(spec.source, spec.vertexLayout === 'edge' ? 'EdgeV' : 'V')
          : spec.source
      module = dev.createShaderModule({ code: source })
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
  getRenderPipeline(dev: GPUDevice, id: string, flavor: PipelineFlavor = {}): GPURenderPipeline {
    const spec = getShader(id)
    const variant = flavor.variant ?? 'uniform'
    if (variant !== 'uniform' && !spec.supportsVariants) {
      throw new Error(`Shader '${id}' does not support the '${variant}' variant`)
    }
    if (variant === 'immediate' && !this.immediateObjectLayout) {
      throw new Error(`Device does not support the 'immediate' variant (no immediate address space)`)
    }
    const blend = flavor.blend ?? spec.blend
    const depth = flavor.depth ?? spec.depth
    const topology = flavor.topology ?? spec.topology
    const depthOnly = spec.depthOnly === true
    const key = `${id}|${variant}|${blend}|${depth.writeEnabled ? 1 : 0}:${depth.compare}|${topology}${depthOnly ? '|shadow' : ''}`
    const cached = this.pipelineCache.get(key)
    if (cached) return cached
    const module = this.shaderModule(dev, id, variant)
    const layout = spec.kind === 'object'
      ? depthOnly ? this.shadowObjectLayout
        : spec.usesMatcapBinding && variant === 'uniform' ? this.matcapObjectLayout
        : spec.usesEnvBinding && variant === 'uniform' ? this.envObjectLayout
        : variant === 'instanced' ? this.instanceLayout
        : variant === 'immediate' ? this.immediateObjectLayout!
        : this.objectLayout
      : spec.usesShadowBinding ? this.sceneLayout : this.sceneOnlyLayout
    const pipeline = dev.createRenderPipeline(depthOnly ? {
      // Depth-only shadow-map pass: no color targets, no fragment stage;
      // renders mesh depth from the key light into a depth32float texture.
      layout,
      vertex: { module, entryPoint: 'vs', buffers: this.vertexLayouts[spec.vertexLayout] },
      primitive: { topology, cullMode: spec.cullMode },
      depthStencil: { format: 'depth32float', depthWriteEnabled: true, depthCompare: depth.compare },
    } : {
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

  /**
   * Extra group(2) bind group for a shader, driven by the registry's
   * binding flags: matcap texture for meshMatcap, environment map for
   * meshPbr, shadow map for the shadow-sampling surface shaders. Uniform
   * object pipelines are built against the three-group objectLayout, so
   * group(2) must be bound even when the shader never samples the shadow
   * map (meshUnlit) — Dawn rejects draws with an unset layout slot.
   */
  extraBindGroupForShader(id: string, groups: ExtraBindGroups): GPUBindGroup | undefined {
    const spec = getShader(id)
    if (spec.usesMatcapBinding) return groups.matcapBG ?? undefined
    if (spec.usesEnvBinding) return groups.envBG ?? undefined
    // mesh/meshToon sample the shadow map at group(2); for other object
    // shaders the inert shadow group simply satisfies the pipeline layout.
    return spec.kind === 'object' ? groups.shadowBG ?? undefined : undefined
  }

  /**
   * Warm up every pipeline the render path can request so first-frame cost
   * lands here, not mid-interaction. Declarative over registry ids plus
   * flavors; render() itself goes through the same cached factory.
   */
  warmup(dev: GPUDevice) {
    const warmup: Array<[id: string, flavor?: PipelineFlavor]> = [
      ['mesh'],
      ['meshSectionCap'],
      ['mesh', { blend: 'alpha', depth: TRANSPARENT_DEPTH }],
      ['deepMesh'],
      ['line'],
      ['grid'],
      ['meshShadow'],
      ['edge'],
      ['mesh', { variant: 'instanced' }],
      ['mesh', { variant: 'instanced', blend: 'alpha', depth: TRANSPARENT_DEPTH }],
      ['edge', { variant: 'instanced' }],
      ['edge', { depth: DEEP_DEPTH }],
      ['selectionOverlay'],
      ['selectionOverlay', { topology: 'line-list' }],
      ['selectionOverlay', { topology: 'line-list', depth: DEEP_DEPTH }],
    ]
    if (this.immediateObjectLayout) {
      warmup.push(
        ['mesh', { variant: 'immediate', blend: 'alpha', depth: TRANSPARENT_DEPTH }],
        ['deepMesh', { variant: 'immediate' }],
        ['edge', { variant: 'immediate', depth: DEEP_DEPTH }],
      )
    }
    for (const [id, flavor] of warmup) this.getRenderPipeline(dev, id, flavor)
  }
}
