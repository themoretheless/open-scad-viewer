import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'
import {
  DEFAULT_MATCAP,
  MATCAP_PRESETS,
  MATERIAL_PRESETS,
  getMatcapPreset,
} from '../src/services/rendererContracts'

/**
 * Renderer-level checks for the textured matcap path: pipeline setup creates
 * the group(2) capture binding with a 1×1 procedural-fallback dummy, matcap
 * draws bind that group, and setMatcapTexture swaps the bound capture.
 */

function fakeTexture(label: string) {
  return { label, createView: () => ({ viewOf: label }), destroy: vi.fn() }
}

function fakeDevice(record: {
  samplers: unknown[]
  textures: Array<{ size: number[] }>
  bindGroups: Array<{ layout: unknown; entries: unknown[] }>
  pipelines: Array<{ layout: unknown }>
  externalCopies: unknown[]
}) {
  return {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: {
      writeBuffer: () => undefined,
      writeTexture: () => undefined,
      copyExternalImageToTexture: (source: unknown) => { record.externalCopies.push(source) },
      submit: () => undefined,
    },
    createTexture: (descriptor: { size: number[] | { width: number; height: number } }) => {
      const size = Array.isArray(descriptor.size) ? descriptor.size : [descriptor.size.width, descriptor.size.height]
      record.textures.push({ size })
      return fakeTexture(`tex${record.textures.length}`)
    },
    createSampler: () => { const sampler = {}; record.samplers.push(sampler); return sampler },
    createBuffer: () => ({}),
    createBindGroupLayout: () => ({}),
    createPipelineLayout: () => ({}),
    createShaderModule: () => ({}),
    createBindGroup: (descriptor: { layout: unknown; entries: unknown[] }) => {
      record.bindGroups.push(descriptor)
      return { bg: descriptor }
    },
    createRenderPipeline: (descriptor: { layout: unknown }) => {
      record.pipelines.push(descriptor)
      return { pipe: descriptor }
    },
    createCommandEncoder: () => ({ beginRenderPass: () => { throw new Error('unused') }, finish: () => ({}) }),
  }
}

function stubGpuGlobals() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16 })
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8 })
}

describe('WebGPURenderer matcap texture binding', () => {
  it('buildPipelines creates the group(2) capture layout, a sampler, and a 1×1 procedural dummy', () => {
    stubGpuGlobals()
    const record = { samplers: [], textures: [], bindGroups: [], pipelines: [], externalCopies: [] }
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      dev: unknown
      textures: Record<string, unknown>
      buildPipelines(): void
    }
    internal.dev = fakeDevice(record)
    internal.buildPipelines()

    // One matcap sampler (a second sampler belongs to the env-map binding).
    expect(record.samplers.length).toBeGreaterThanOrEqual(1)
    expect(record.textures).toContainEqual({ size: [1, 1] })
    expect(internal.textures.matcapDummyTexture).not.toBeNull()
    expect(internal.textures.matcapBG).not.toBeNull()
    // The dummy is bound in the matcap bind group (texture view + sampler).
    const matcapGroup = record.bindGroups.find(group => group.layout === internal.textures.matcapBGL)
    expect(matcapGroup).toBeDefined()
    expect(matcapGroup!.entries).toHaveLength(4)
    // The matcap pipeline uses the three-group layout; plain mesh does not.
    // (meshMatcap pipelines are created lazily, so request one explicitly.)
    const internalLayouts = renderer as unknown as {
      pipelines: Record<string, unknown>
      getRenderPipeline(id: string): unknown
    }
    internalLayouts.getRenderPipeline('meshMatcap')
    expect(record.pipelines.some(pipeline => pipeline.layout === internalLayouts.pipelines.matcapObjectLayout)).toBe(true)
    expect(record.pipelines.some(pipeline => pipeline.layout === internalLayouts.pipelines.objectLayout)).toBe(true)
  })

  it('opaque matcap draws bind the capture bind group at group(2)', () => {
    stubGpuGlobals()
    const bindGroups: Array<[number, unknown]> = []
    const pipelines: unknown[] = []
    const pass = {
      setPipeline: (pipeline: unknown) => { pipelines.push(pipeline) },
      setBindGroup: (index: number, group: unknown) => { bindGroups.push([index, group]) },
      setVertexBuffer: () => undefined,
      setIndexBuffer: () => undefined,
      draw: () => undefined,
      drawIndexed: () => undefined,
      executeBundles: () => undefined,
      setImmediates: () => undefined,
      end: () => undefined,
    }
    const record = { samplers: [], textures: [], bindGroups: [], pipelines: [], externalCopies: [] }
    const dev = {
      ...fakeDevice(record),
      createCommandEncoder: () => ({ beginRenderPass: () => pass, finish: () => ({}) }),
    }
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as Record<string, unknown> & { render(): void }
    internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
    internal.dev = dev
    internal.ctx = { getCurrentTexture: () => ({ createView: () => ({}) }) }
    internal.initialized = true
    internal.lost = false
    internal.dead = false
    internal.gridVisible = false
    internal.sceneUB = {}
    internal.sceneBG = 'sceneBG'
    const textures = internal.textures as Record<string, unknown>
    const factory = internal.pipelines as Record<string, unknown>
    textures.matcapBG = 'matcapBG'
    internal.meshPipe = 'mesh'
    internal.meshPipeT = 'meshT'
    internal.meshImmediatePipeT = null
    internal.sectionCapPipe = 'cap'
    factory.matcapObjectLayout = 'matcapLayout'
    factory.objectLayout = 'objectLayout'
    factory.immediateObjectLayout = null
    factory.instanceLayout = 'instanceLayout'
    factory.sceneLayout = 'sceneLayout'
    factory.vertexLayouts = { mesh: [], edge: [], line: [], grid: [] }
    internal.meshes = [{
      morph: undefined,
      morphSlot: {},
      vb: {},
      ib: {},
      ic: 3,
      bg: 'objBG',
      edgeIB: null,
      edgeIC: 0,
      alpha: 1,
      visible: true,
      shadingModel: 'matcap',
      worldBounds: { center: [0, 0, 0], radius: 1e6, min: [-1, -1, -1], max: [1, 1, 1] },
    }]
    internal.render()

    const matcapPipeline = record.pipelines.find(pipeline => pipeline.layout === 'matcapLayout')
    expect(matcapPipeline).toBeDefined()
    expect(pipelines.some(pipeline => (pipeline as { pipe?: unknown })?.pipe === matcapPipeline)).toBe(true)
    expect(bindGroups).toContainEqual([2, 'matcapBG'])
    expect(bindGroups).toContainEqual([0, 'sceneBG'])
    expect(bindGroups).toContainEqual([1, 'objBG'])
  })

  it('setMatcapTexture switches between the procedural dummy and fetched captures', async () => {
    stubGpuGlobals()
    const record = { samplers: [], textures: [], bindGroups: [], pipelines: [], externalCopies: [] }
    const dev = fakeDevice(record)
    const bitmap = { width: 256, height: 256, close: vi.fn() }
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, blob: async () => ({}) })))
    vi.stubGlobal('createImageBitmap', vi.fn(async () => bitmap))

    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as Record<string, unknown>
    internal.dev = dev
    const textures = internal.textures as Record<string, unknown>
    textures.matcapBGL = {}
    textures.matcapSampler = {}
    textures.matcapDummyTexture = fakeTexture('dummy')
    textures.matcapTexture = textures.matcapDummyTexture
    // The group(2) matcap bind group embeds the shadow map at bindings 2/3.
    textures.shadowTexture = fakeTexture('shadow')
    textures.shadowSampler = {}

    expect(renderer.currentMatcap).toBe('procedural')
    await expect(renderer.setMatcapTexture('nope')).rejects.toThrow("unknown matcap 'nope'")

    await renderer.setMatcapTexture('studio')
    expect(renderer.currentMatcap).toBe('studio')
    expect(fetch).toHaveBeenCalledWith('matcaps/studio.png')
    expect(record.textures).toContainEqual({ size: [256, 256] })
    expect(record.externalCopies).toHaveLength(1)
    expect(bitmap.close).toHaveBeenCalled()
    expect(textures.matcapBG).not.toBeNull()

    // Switching back to procedural rebinds the 1×1 dummy (shader fallback).
    await renderer.setMatcapTexture('procedural')
    expect(renderer.currentMatcap).toBe('procedural')
    expect(textures.matcapTexture).toBe(textures.matcapDummyTexture)
  })
})

describe('matcap and material preset contracts', () => {
  it('procedural matcap stays the default and carries no URL', () => {
    expect(DEFAULT_MATCAP.id).toBe('procedural')
    expect(MATCAP_PRESETS[0].id).toBe('procedural')
    expect(MATCAP_PRESETS[0].url).toBeUndefined()
    for (const preset of MATCAP_PRESETS.slice(1)) {
      expect(preset.url).toMatch(/^matcaps\/.+\.png$/)
    }
    expect(getMatcapPreset('studio')?.name).toBe('Studio')
    expect(getMatcapPreset('nope')).toBeUndefined()
  })

  it('ships the plastic, rubber, glass, and anodized-aluminum presets', () => {
    const byId = new Map(MATERIAL_PRESETS.map(preset => [preset.id, preset]))
    expect(byId.get('plastic')).toMatchObject({ metallic: 0, shadingModel: 'pbr' })
    expect(byId.get('rubber')).toMatchObject({ metallic: 0, shadingModel: 'pbr' })
    expect(byId.get('rubber')!.roughness).toBeGreaterThan(0.9)
    // Glass is translucent PBR: alpha < 1 routes it into the transparent pass.
    expect(byId.get('glass')).toMatchObject({ metallic: 0, shadingModel: 'pbr' })
    expect(byId.get('glass')!.alpha!).toBeLessThan(1)
    expect(byId.get('anodized-aluminum')).toMatchObject({ metallic: 1, shadingModel: 'pbr' })
    // Default appearance unchanged: the first preset remains the legacy look.
    expect(MATERIAL_PRESETS[0]).toMatchObject({ id: 'default', shadingModel: 'phong', metallic: 0, roughness: 0.7 })
  })
})
