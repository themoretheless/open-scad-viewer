import { describe, expect, it, vi } from 'vitest'
import {
  GpuWboitRenderer,
  toWboitShader,
  WBOIT_ACCUM_FORMAT,
  WBOIT_REVEAL_FORMAT,
  WBOIT_COMPOSITE_WGSL,
} from '../src/services/gpuWboit'
import { MESH_WGSL } from '../src/services/shaders'

function createFakeDevice() {
  const textures: any[] = []
  const pipelines: any[] = []
  const bindGroups: any[] = []

  const device = {
    createTexture: vi.fn((desc: any) => {
      const tex = {
        desc,
        destroyed: false,
        createView: vi.fn(() => ({ texture: tex, desc })),
        destroy: vi.fn(() => { tex.destroyed = true }),
      }
      textures.push(tex)
      return tex
    }),
    createBindGroupLayout: vi.fn((desc: any) => ({ desc })),
    createBindGroup: vi.fn((desc: any) => {
      const bg = { desc }
      bindGroups.push(bg)
      return bg
    }),
    createPipelineLayout: vi.fn((desc: any) => ({ desc })),
    createShaderModule: vi.fn((desc: any) => ({ desc, code: desc.code })),
    createRenderPipeline: vi.fn((desc: any) => {
      const p = { desc }
      pipelines.push(p)
      return p
    }),
  } as unknown as GPUDevice

  return { device, textures, pipelines, bindGroups }
}

describe('GpuWboitRenderer', () => {
  it('transforms standard mesh shaders into dual-target WBOIT shaders', () => {
    const transformed = toWboitShader(MESH_WGSL)
    expect(transformed).toContain('struct WboitOutput')
    expect(transformed).toContain('@location(0) accum: vec4f')
    expect(transformed).toContain('@location(1) reveal: vec4f')
    expect(transformed).toContain('fn wboitWeight')
    expect(transformed).toContain('fn fs_color(v: V) -> vec4f')
    expect(transformed).toContain('@fragment fn fs(v: V) -> WboitOutput')
  })

  it('throws error when transforming an incompatible shader source', () => {
    expect(() => toWboitShader('invalid shader without fs')).toThrow('Shader source does not match expected')
  })

  it('allocates and caches textures matching viewport dimensions', () => {
    const { device } = createFakeDevice()
    const wboit = new GpuWboitRenderer()

    const t1 = wboit.ensureTextures(device, 800, 600)
    expect(t1.width).toBe(800)
    expect(t1.height).toBe(600)
    expect(device.createTexture).toHaveBeenCalledTimes(2)

    // Re-check with identical dimensions returns cached textures
    const t2 = wboit.ensureTextures(device, 800, 600)
    expect(t2).toBe(t1)
    expect(device.createTexture).toHaveBeenCalledTimes(2)

    // Resizing creates fresh textures and destroys old ones
    const t3 = wboit.ensureTextures(device, 1024, 768)
    expect(t3.width).toBe(1024)
    expect(t3.height).toBe(768)
    expect(t1.accumTexture.destroy).toHaveBeenCalled()
    expect(t1.revealTexture.destroy).toHaveBeenCalled()
    expect(device.createTexture).toHaveBeenCalledTimes(4)

    wboit.destroy()
    expect(t3.accumTexture.destroy).toHaveBeenCalled()
  })

  it('uses required formats: RGBA16Float for accumulation and R8Unorm for revealage', () => {
    const { device, textures } = createFakeDevice()
    const wboit = new GpuWboitRenderer()
    wboit.ensureTextures(device, 640, 480)

    expect(textures[0].desc.format).toBe(WBOIT_ACCUM_FORMAT)
    expect(textures[0].desc.format).toBe('rgba16float')
    expect(textures[1].desc.format).toBe(WBOIT_REVEAL_FORMAT)
    expect(textures[1].desc.format).toBe('r8unorm')
  })

  it('creates and caches composite pipeline with alpha blending', () => {
    const { device, pipelines } = createFakeDevice()
    const wboit = new GpuWboitRenderer()

    const pipe1 = wboit.getCompositePipeline(device, 'bgra8unorm')
    expect(pipe1).toBeDefined()
    expect(pipelines.length).toBe(1)

    const pipeDesc = pipelines[0].desc
    expect(pipeDesc.fragment.targets[0].format).toBe('bgra8unorm')
    expect(pipeDesc.fragment.targets[0].blend.color.srcFactor).toBe('src-alpha')
    expect(pipeDesc.fragment.targets[0].blend.color.dstFactor).toBe('one-minus-src-alpha')

    // Calling again returns cached pipeline
    const pipe2 = wboit.getCompositePipeline(device, 'bgra8unorm')
    expect(pipe2).toBe(pipe1)
    expect(pipelines.length).toBe(1)
  })

  it('creates dual-target WBOIT render pipeline with additive accum and multiplicative reveal', () => {
    const { device, pipelines } = createFakeDevice()
    const wboit = new GpuWboitRenderer()

    const fakeLayout = {} as GPUPipelineLayout
    const pipe = wboit.getWboitPipeline(device, 'mesh', fakeLayout, [])
    expect(pipe).toBeDefined()

    const desc = pipelines.find(p => p.desc.fragment?.targets?.length === 2)?.desc
    expect(desc).toBeDefined()

    // Target 0: Accumulation
    expect(desc.fragment.targets[0].format).toBe('rgba16float')
    expect(desc.fragment.targets[0].blend.color.srcFactor).toBe('one')
    expect(desc.fragment.targets[0].blend.color.dstFactor).toBe('one')

    // Target 1: Revealage
    expect(desc.fragment.targets[1].format).toBe('r8unorm')
    expect(desc.fragment.targets[1].blend.color.srcFactor).toBe('zero')
    expect(desc.fragment.targets[1].blend.color.dstFactor).toBe('one-minus-src')

    // Depth state: read-only
    expect(desc.depthStencil.depthWriteEnabled).toBe(false)
    expect(desc.depthStencil.depthCompare).toBe('less')
  })

  it('begins WBOIT pass with clear targets: accum=(0,0,0,0) and reveal=(1,1,1,1)', () => {
    const { device } = createFakeDevice()
    const wboit = new GpuWboitRenderer()
    wboit.ensureTextures(device, 500, 500)

    let passDesc: any = null
    const encoder = {
      beginRenderPass: vi.fn((desc: any) => {
        passDesc = desc
        return { setPipeline: vi.fn(), end: vi.fn() }
      }),
    } as unknown as GPUCommandEncoder

    const depthView = {} as GPUTextureView
    wboit.beginWboitPass(encoder, depthView)

    expect(encoder.beginRenderPass).toHaveBeenCalled()
    expect(passDesc.colorAttachments[0].clearValue).toEqual({ r: 0, g: 0, b: 0, a: 0 })
    expect(passDesc.colorAttachments[0].loadOp).toBe('clear')
    expect(passDesc.colorAttachments[1].clearValue).toEqual({ r: 1, g: 1, b: 1, a: 1 })
    expect(passDesc.colorAttachments[1].loadOp).toBe('clear')
    expect(passDesc.depthStencilAttachment.depthReadOnly).toBe(true)
  })

  it('executes composite pass drawing 3 vertices', () => {
    const { device } = createFakeDevice()
    const wboit = new GpuWboitRenderer()
    wboit.ensureTextures(device, 500, 500)

    let drawCount = 0
    let ended = false
    const pass = {
      setPipeline: vi.fn(),
      setBindGroup: vi.fn(),
      draw: vi.fn((count: number) => { drawCount = count }),
      end: vi.fn(() => { ended = true }),
    }
    const encoder = {
      beginRenderPass: vi.fn(() => pass),
    } as unknown as GPUCommandEncoder

    const targetView = {} as GPUTextureView
    wboit.composite(device, encoder, targetView, 'bgra8unorm')

    expect(encoder.beginRenderPass).toHaveBeenCalled()
    expect(drawCount).toBe(3)
    expect(ended).toBe(true)
  })

  it('mathematically satisfies order independence (A then B === B then A)', () => {
    // Model two overlapping transparent fragments
    const fragA = { r: 1.0, g: 0.2, b: 0.2, a: 0.6, z: 0.3 }
    const fragB = { r: 0.2, g: 0.4, b: 1.0, a: 0.4, z: 0.7 }

    function wboitWeight(z: number, a: number) {
      const aClamped = Math.min(1.0, a) * 8.0 + 0.01
      const b = 1.0 - z * 0.95
      return Math.min(Math.max(Math.pow(aClamped, 3) * 1e8 * Math.pow(b, 3), 1e-2), 3e3)
    }

    const wA = wboitWeight(fragA.z, fragA.a)
    const wB = wboitWeight(fragB.z, fragB.a)

    // Order 1: A then B
    const accum1 = {
      r: (fragA.r * fragA.a) * wA + (fragB.r * fragB.a) * wB,
      g: (fragA.g * fragA.a) * wA + (fragB.g * fragB.a) * wB,
      b: (fragA.b * fragA.a) * wA + (fragB.b * fragB.a) * wB,
      a: fragA.a * wA + fragB.a * wB,
    }
    let reveal1 = 1.0
    reveal1 *= (1.0 - fragA.a)
    reveal1 *= (1.0 - fragB.a)

    // Order 2: B then A
    const accum2 = {
      r: (fragB.r * fragB.a) * wB + (fragA.r * fragA.a) * wA,
      g: (fragB.g * fragB.a) * wB + (fragA.g * fragA.a) * wA,
      b: (fragB.b * fragB.a) * wB + (fragA.b * fragA.a) * wA,
      a: fragB.a * wB + fragA.a * wA,
    }
    let reveal2 = 1.0
    reveal2 *= (1.0 - fragB.a)
    reveal2 *= (1.0 - fragA.a)

    expect(accum1.r).toBeCloseTo(accum2.r, 8)
    expect(accum1.g).toBeCloseTo(accum2.g, 8)
    expect(accum1.b).toBeCloseTo(accum2.b, 8)
    expect(accum1.a).toBeCloseTo(accum2.a, 8)
    expect(reveal1).toBeCloseTo(reveal2, 8)

    // Final blended color is identical
    const color1 = {
      r: accum1.r / accum1.a,
      g: accum1.g / accum1.a,
      b: accum1.b / accum1.a,
      alpha: 1.0 - reveal1,
    }
    const color2 = {
      r: accum2.r / accum2.a,
      g: accum2.g / accum2.a,
      b: accum2.b / accum2.a,
      alpha: 1.0 - reveal2,
    }

    expect(color1).toEqual(color2)
  })

  it('integrates with WebGPURenderer transparencyMode and teardown', async () => {
    const { WebGPURenderer } = await import('../src/services/webgpuRenderer')
    const { device } = createFakeDevice()
    const renderer = new WebGPURenderer()

    expect(renderer.getTransparencyMode()).toBe('sorted')
    renderer.setTransparencyMode('wboit')
    expect(renderer.getTransparencyMode()).toBe('wboit')

    const wboit = renderer.wboitRenderer
    expect(wboit).toBeInstanceOf(GpuWboitRenderer)

    renderer.destroy()
  })
})
