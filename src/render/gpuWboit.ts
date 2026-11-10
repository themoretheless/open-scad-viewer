/**
 * Weighted Blended Order-Independent Transparency (WBOIT, McGuire & Bavoil 2013).
 *
 * Replaces CPU-side back-to-front centroid sorting with a dual-target offscreen
 * accumulation pass (RGBA16Float for color weights, R8Unorm for cumulative
 * transmittance) followed by a fullscreen composite pass over the opaque scene.
 *
 * Commutative blending guarantees identical pixel results regardless of polygon
 * arrival order, correctly solving intersecting and nested transparent CSG bodies.
 */

import { getShader } from './shaders'

const USAGE_RENDER_ATTACHMENT = typeof GPUTextureUsage !== 'undefined' ? GPUTextureUsage.RENDER_ATTACHMENT : 0x10
const USAGE_TEXTURE_BINDING = typeof GPUTextureUsage !== 'undefined' ? GPUTextureUsage.TEXTURE_BINDING : 0x04
const STAGE_FRAGMENT = typeof GPUShaderStage !== 'undefined' ? GPUShaderStage.FRAGMENT : 0x2

export const WBOIT_ACCUM_FORMAT: GPUTextureFormat = 'rgba16float'
export const WBOIT_REVEAL_FORMAT: GPUTextureFormat = 'r8unorm'

export const WBOIT_COMPOSITE_WGSL = /* wgsl */`
@group(0) @binding(0) var accumTexture: texture_2d<f32>;
@group(0) @binding(1) var revealTexture: texture_2d<f32>;

struct VertexOutput {
  @builtin(position) position: vec4f,
}

@vertex
fn vs_composite(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
  var pos = array<vec2f, 3>(
    vec2f(-1.0, -1.0),
    vec2f(3.0, -1.0),
    vec2f(-1.0, 3.0)
  );
  var out: VertexOutput;
  out.position = vec4f(pos[vertexIndex], 0.0, 1.0);
  return out;
}

@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4f {
  let coord = vec2i(in.position.xy);
  let reveal = textureLoad(revealTexture, coord, 0).r;
  // If nothing was drawn or pixel is completely transmissive, discard early.
  if (reveal >= 0.99999) {
    discard;
  }
  let accum = textureLoad(accumTexture, coord, 0);
  let maxVal = max(accum.a, 1e-5);
  let avgColor = accum.rgb / maxVal;
  let alpha = 1.0 - reveal;
  return vec4f(avgColor, alpha);
}
`

/**
 * Transforms standard single-target fragment shaders (@location(0) vec4f)
 * into WBOIT dual-target shaders (Accumulation @location(0), Revealage @location(1)).
 */
export function toWboitShader(source: string): string {
  const targetSignature = '@fragment fn fs(v: V) -> @location(0) vec4f {'
  if (!source.includes(targetSignature)) {
    throw new Error('Shader source does not match expected @fragment fn fs(v: V) -> @location(0) vec4f contract')
  }

  const prefix = /* wgsl */`
struct WboitOutput {
  @location(0) accum: vec4f,
  @location(1) reveal: vec4f,
}

fn wboitWeight(depth: f32, alpha: f32) -> f32 {
  let a = min(1.0, alpha) * 8.0 + 0.01;
  let b = 1.0 - depth * 0.95;
  return clamp(pow(a, 3.0) * 1e8 * pow(b, 3.0), 1e-2, 3e3);
}

fn fs_color(v: V) -> vec4f {
`

  let replaced = source.replace(targetSignature, prefix)

  const suffix = /* wgsl */`

@fragment fn fs(v: V) -> WboitOutput {
  let c = fs_color(v);
  let alpha = c.a;
  let w = wboitWeight(v.p.z, alpha);
  return WboitOutput(
    vec4f(c.rgb * alpha, alpha) * w,
    vec4f(alpha, 0.0, 0.0, 0.0)
  );
}
`
  return replaced + suffix
}

export interface WboitTextures {
  accumTexture: GPUTexture
  revealTexture: GPUTexture
  accumView: GPUTextureView
  revealView: GPUTextureView
  width: number
  height: number
}

export class GpuWboitRenderer {
  private textures: WboitTextures | null = null
  private compositeBGL: GPUBindGroupLayout | null = null
  private compositeBG: GPUBindGroup | null = null
  private compositePipeline: GPURenderPipeline | null = null
  private compositePipelineFormat: GPUTextureFormat | null = null
  private readonly wboitPipelineCache = new Map<string, GPURenderPipeline>()

  /** Ensures offscreen accumulation and revealage textures match target dimensions. */
  ensureTextures(device: GPUDevice, width: number, height: number): WboitTextures {
    const clampedW = Math.max(1, Math.round(width))
    const clampedH = Math.max(1, Math.round(height))

    if (this.textures && this.textures.width === clampedW && this.textures.height === clampedH) {
      return this.textures
    }

    this.destroyTextures()

    const accumTexture = device.createTexture({
      size: [clampedW, clampedH, 1],
      format: WBOIT_ACCUM_FORMAT,
      usage: USAGE_RENDER_ATTACHMENT | USAGE_TEXTURE_BINDING,
    })

    const revealTexture = device.createTexture({
      size: [clampedW, clampedH, 1],
      format: WBOIT_REVEAL_FORMAT,
      usage: USAGE_RENDER_ATTACHMENT | USAGE_TEXTURE_BINDING,
    })

    const accumView = accumTexture.createView()
    const revealView = revealTexture.createView()

    this.textures = {
      accumTexture,
      revealTexture,
      accumView,
      revealView,
      width: clampedW,
      height: clampedH,
    }

    this.compositeBG = null
    return this.textures
  }

  getCompositeBindGroupLayout(device: GPUDevice): GPUBindGroupLayout {
    if (!this.compositeBGL) {
      this.compositeBGL = device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: STAGE_FRAGMENT, texture: { sampleType: 'unfilterable-float' } },
          { binding: 1, visibility: STAGE_FRAGMENT, texture: { sampleType: 'unfilterable-float' } },
        ],
      })
    }
    return this.compositeBGL
  }

  getCompositeBindGroup(device: GPUDevice): GPUBindGroup {
    if (!this.compositeBG) {
      if (!this.textures) throw new Error('WBOIT textures must be initialized before creating composite bind group')
      const bgl = this.getCompositeBindGroupLayout(device)
      this.compositeBG = device.createBindGroup({
        layout: bgl,
        entries: [
          { binding: 0, resource: this.textures.accumView },
          { binding: 1, resource: this.textures.revealView },
        ],
      })
    }
    return this.compositeBG
  }

  getCompositePipeline(device: GPUDevice, targetFormat: GPUTextureFormat): GPURenderPipeline {
    if (this.compositePipeline && this.compositePipelineFormat === targetFormat) {
      return this.compositePipeline
    }

    const bgl = this.getCompositeBindGroupLayout(device)
    const layout = device.createPipelineLayout({ bindGroupLayouts: [bgl] })
    const module = device.createShaderModule({ code: WBOIT_COMPOSITE_WGSL })

    this.compositePipeline = device.createRenderPipeline({
      layout,
      vertex: {
        module,
        entryPoint: 'vs_composite',
      },
      fragment: {
        module,
        entryPoint: 'fs_composite',
        targets: [{
          format: targetFormat,
          blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          },
        }],
      },
    })
    this.compositePipelineFormat = targetFormat
    return this.compositePipeline
  }

  /**
   * Retrieves or builds a cached WBOIT render pipeline for a given shader specification.
   */
  getWboitPipeline(
    device: GPUDevice,
    shaderId: string,
    layout: GPUPipelineLayout,
    vertexLayouts: GPUVertexBufferLayout[]
  ): GPURenderPipeline {
    let pipeline = this.wboitPipelineCache.get(shaderId)
    if (pipeline) return pipeline

    const spec = getShader(shaderId)
    const transformedCode = toWboitShader(spec.source)
    const module = device.createShaderModule({ code: transformedCode })

    pipeline = device.createRenderPipeline({
      layout,
      vertex: {
        module,
        entryPoint: 'vs',
        buffers: vertexLayouts,
      },
      fragment: {
        module,
        entryPoint: 'fs',
        targets: [
          // Target 0: Accumulation (RGBA16Float, additive blend)
          {
            format: WBOIT_ACCUM_FORMAT,
            blend: {
              color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
              alpha: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
            },
          },
          // Target 1: Revealage (R8Unorm, multiplicative transmittance)
          {
            format: WBOIT_REVEAL_FORMAT,
            blend: {
              color: { srcFactor: 'zero', dstFactor: 'one-minus-src', operation: 'add' },
              alpha: { srcFactor: 'zero', dstFactor: 'one-minus-src', operation: 'add' },
            },
          },
        ],
      },
      primitive: {
        topology: 'triangle-list',
        cullMode: 'none',
      },
      depthStencil: {
        format: 'depth24plus',
        depthWriteEnabled: false,
        depthCompare: 'less',
      },
    })

    this.wboitPipelineCache.set(shaderId, pipeline)
    return pipeline
  }

  /**
   * Begins the offscreen accumulation pass for transparent geometry.
   */
  beginWboitPass(
    encoder: GPUCommandEncoder,
    depthView: GPUTextureView
  ): GPURenderPassEncoder {
    if (!this.textures) throw new Error('WBOIT textures must be initialized before beginWboitPass')

    return encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.textures.accumView,
          clearValue: { r: 0, g: 0, b: 0, a: 0 },
          loadOp: 'clear',
          storeOp: 'store',
        },
        {
          view: this.textures.revealView,
          clearValue: { r: 1, g: 1, b: 1, a: 1 },
          loadOp: 'clear',
          storeOp: 'store',
        },
      ],
      depthStencilAttachment: {
        view: depthView,
        depthReadOnly: true,
        depthLoadOp: 'load',
        depthStoreOp: 'store',
      },
    })
  }

  /**
   * Renders the composite pass: samples accumulated weights and blends over the opaque frame.
   */
  composite(
    device: GPUDevice,
    encoder: GPUCommandEncoder,
    targetView: GPUTextureView,
    targetFormat: GPUTextureFormat
  ): void {
    const pipeline = this.getCompositePipeline(device, targetFormat)
    const bindGroup = this.getCompositeBindGroup(device)

    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: targetView,
          loadOp: 'load',
          storeOp: 'store',
        },
      ],
    })

    pass.setPipeline(pipeline)
    pass.setBindGroup(0, bindGroup)
    pass.draw(3)
    pass.end()
  }

  destroyTextures(): void {
    if (this.textures) {
      try { this.textures.accumTexture.destroy() } catch { /* ignore */ }
      try { this.textures.revealTexture.destroy() } catch { /* ignore */ }
      this.textures = null
      this.compositeBG = null
    }
  }

  destroy(): void {
    this.destroyTextures()
    this.wboitPipelineCache.clear()
    this.compositePipeline = null
    this.compositeBGL = null
  }
}
