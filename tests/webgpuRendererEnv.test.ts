import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'
import {
  DEFAULT_ENV,
  ENV_PRESETS,
  getEnvPreset,
} from '../src/services/rendererContracts'

/**
 * Renderer-level checks for the PBR environment-map (IBL) path: pipeline
 * setup creates the group(2) env binding with a 1×1 analytic-fallback dummy,
 * PBR draws bind that group, and setEnvMap swaps the bound panorama. Mirrors
 * the matcap fake-device pattern in webgpuRendererMatcap.test.ts.
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

describe('WebGPURenderer environment-map binding', () => {
  it('buildPipelines creates the group(2) env layout, a sampler, and a 1×1 analytic dummy', () => {
    stubGpuGlobals()
    const record = { samplers: [], textures: [], bindGroups: [], pipelines: [], externalCopies: [] }
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      dev: unknown
      envBGL: unknown
      envBG: unknown
      envDummyTexture: unknown
      buildPipelines(): void
    }
    internal.dev = fakeDevice(record)
    internal.buildPipelines()

    // Three samplers: shadow comparison + matcap + env. Three 1×1 dummies:
    // shadow depth + matcap + env.
    expect(record.samplers).toHaveLength(3)
    expect(record.textures.filter(t => t.size.length === 2 && t.size[0] === 1 && t.size[1] === 1)).toHaveLength(3)
    expect(internal.envDummyTexture).not.toBeNull()
    expect(internal.envBG).not.toBeNull()
    // The dummy is bound in the env bind group (texture view + sampler).
    const envGroup = record.bindGroups.find(group => group.layout === internal.envBGL)
    expect(envGroup).toBeDefined()
    expect(envGroup!.entries).toHaveLength(4)
    // The PBR pipeline uses the three-group env layout; plain mesh does not.
    const internalLayouts = renderer as unknown as {
      objectLayout: unknown
      envObjectLayout: unknown
      getRenderPipeline(id: string): unknown
    }
    internalLayouts.getRenderPipeline('meshPbr')
    expect(record.pipelines.some(pipeline => pipeline.layout === internalLayouts.envObjectLayout)).toBe(true)
    expect(record.pipelines.some(pipeline => pipeline.layout === internalLayouts.objectLayout)).toBe(true)
  })

  it('opaque PBR draws bind the env bind group at group(2)', () => {
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
    internal.envBG = 'envBG'
    internal.meshPipe = 'mesh'
    internal.meshPipeT = 'meshT'
    internal.meshImmediatePipeT = null
    internal.sectionCapPipe = 'cap'
    internal.envObjectLayout = 'envLayout'
    internal.objectLayout = 'objectLayout'
    internal.immediateObjectLayout = null
    internal.instanceLayout = 'instanceLayout'
    internal.sceneLayout = 'sceneLayout'
    internal.vertexLayouts = { mesh: [], edge: [], line: [], grid: [] }
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
      shadingModel: 'pbr',
      worldBounds: { center: [0, 0, 0], radius: 1e6, min: [-1, -1, -1], max: [1, 1, 1] },
    }]
    internal.render()

    const pbrPipeline = record.pipelines.find(pipeline => pipeline.layout === 'envLayout')
    expect(pbrPipeline).toBeDefined()
    expect(pipelines.some(pipeline => (pipeline as { pipe?: unknown })?.pipe === pbrPipeline)).toBe(true)
    expect(bindGroups).toContainEqual([2, 'envBG'])
    expect(bindGroups).toContainEqual([0, 'sceneBG'])
    expect(bindGroups).toContainEqual([1, 'objBG'])
  })

  it('setEnvMap switches between the analytic dummy and fetched panoramas', async () => {
    stubGpuGlobals()
    const record = { samplers: [], textures: [], bindGroups: [], pipelines: [], externalCopies: [] }
    const dev = fakeDevice(record)
    const bitmap = { width: 512, height: 256, close: vi.fn() }
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, blob: async () => ({}) })))
    vi.stubGlobal('createImageBitmap', vi.fn(async () => bitmap))

    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as Record<string, unknown>
    internal.dev = dev
    internal.envBGL = {}
    internal.envSampler = {}
    internal.envDummyTexture = fakeTexture('dummy')
    internal.envTexture = internal.envDummyTexture
    // The group(2) env bind group embeds the shadow map at bindings 2/3.
    internal.shadowTexture = fakeTexture('shadow')
    internal.shadowSampler = {}

    expect(renderer.currentEnv).toBe('none')
    await expect(renderer.setEnvMap('nope')).rejects.toThrow("unknown environment map 'nope'")

    await renderer.setEnvMap('studio-softbox')
    expect(renderer.currentEnv).toBe('studio-softbox')
    expect(fetch).toHaveBeenCalledWith('env/studio-softbox.png')
    expect(record.textures).toContainEqual({ size: [512, 256] })
    expect(record.externalCopies).toHaveLength(1)
    expect(bitmap.close).toHaveBeenCalled()
    expect(internal.envBG).not.toBeNull()

    // Switching back to 'none' rebinds the 1×1 dummy (analytic fallback).
    await renderer.setEnvMap('none')
    expect(renderer.currentEnv).toBe('none')
    expect(internal.envTexture).toBe(internal.envDummyTexture)
  })
})

describe('environment-map preset contracts', () => {
  it("'none' stays the default and carries no URL; others ship small equirect PNGs", () => {
    expect(DEFAULT_ENV.id).toBe('none')
    expect(ENV_PRESETS[0].id).toBe('none')
    expect(ENV_PRESETS[0].url).toBeUndefined()
    for (const preset of ENV_PRESETS.slice(1)) {
      expect(preset.url).toMatch(/^env\/.+\.png$/)
    }
    expect(getEnvPreset('studio-softbox')?.name).toBe('Studio Softbox')
    expect(getEnvPreset('nope')).toBeUndefined()
  })
})
