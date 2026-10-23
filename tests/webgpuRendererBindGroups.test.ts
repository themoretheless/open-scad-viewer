import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'
import { getShader, listShaders } from '../src/services/shaders'

/**
 * WebGPU draw validation the other renderer fakes skip: every group of the
 * bound pipeline's layout must be set (with a matching bind-group layout)
 * before each draw, render bundles start with empty state, executeBundles
 * resets the pass state, and a pass must not sample its own depth attachment.
 * The fake device keeps real object identities (bind-group layouts, pipeline
 * layouts, textures) so the renderer's actual layouts and bind groups are
 * checked, not string markers.
 */

interface FakeBindGroupLayout { entries: unknown[] }
interface FakePipelineLayout { groups: FakeBindGroupLayout[]; immediateSize: number }
interface FakeBindGroup { layout: FakeBindGroupLayout; entries: Array<{ resource?: unknown }> }
interface FakeTexture { size: unknown; createView(): { texture: FakeTexture }; destroy(): void }
interface FakePipeline { layout: FakePipelineLayout; code: string }

function fakeTexture(size: unknown): FakeTexture {
  const texture: FakeTexture = { size, createView: () => ({ texture }), destroy: () => undefined }
  return texture
}

function validatingEncoder(errors: string[], context: string, labelOf: (pipeline: FakePipeline) => string, attachment?: FakeTexture) {
  let pipeline: FakePipeline | null = null
  let groups: Array<FakeBindGroup | undefined> = []
  const check = (call: string) => {
    if (!pipeline) {
      errors.push(`${context}: ${call} without a pipeline`)
      return
    }
    const label = labelOf(pipeline)
    pipeline.layout.groups.forEach((layout, index) => {
      const group = groups[index]
      if (!group) errors.push(`${context}: ${call} with ${label}: no bind group set at group(${index})`)
      else if (group.layout !== layout) errors.push(`${context}: ${call} with ${label}: group(${index}) layout mismatch`)
    })
  }
  return {
    setPipeline: (next: FakePipeline) => { pipeline = next },
    setBindGroup: (index: number, group: FakeBindGroup) => {
      groups[index] = group
      const samplesAttachment = attachment && group.entries.some(entry =>
        (entry.resource as { texture?: unknown } | undefined)?.texture === attachment)
      if (samplesAttachment) errors.push(`${context}: group(${index}) samples the pass's own depth attachment`)
    },
    setVertexBuffer: () => undefined,
    setIndexBuffer: () => undefined,
    setImmediates: () => undefined,
    draw: () => check('draw'),
    drawIndexed: () => check('drawIndexed'),
    executeBundles: () => { pipeline = null; groups = [] },
    end: () => undefined,
    finish: () => ({}),
  }
}

function validatingDevice(errors: string[], labelOf: (pipeline: FakePipeline) => string) {
  return {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: { writeBuffer: () => undefined, writeTexture: () => undefined, submit: () => undefined },
    createTexture: (descriptor: { size: unknown }) => fakeTexture(descriptor.size),
    createSampler: () => ({}),
    createBuffer: () => ({ destroy: () => undefined }),
    createBindGroupLayout: (descriptor: { entries: unknown[] }): FakeBindGroupLayout => ({ entries: descriptor.entries }),
    createPipelineLayout: (descriptor: { bindGroupLayouts: FakeBindGroupLayout[]; immediateSize?: number }): FakePipelineLayout => ({
      groups: [...descriptor.bindGroupLayouts],
      immediateSize: descriptor.immediateSize ?? 0,
    }),
    createShaderModule: (descriptor: { code: string }) => ({ code: descriptor.code }),
    createBindGroup: (descriptor: FakeBindGroup) => ({ layout: descriptor.layout, entries: descriptor.entries }),
    createRenderPipeline: (descriptor: { layout: FakePipelineLayout; vertex: { module: { code: string } } }): FakePipeline => ({
      layout: descriptor.layout,
      code: descriptor.vertex.module.code,
    }),
    createRenderBundleEncoder: () => validatingEncoder(errors, 'render bundle', labelOf),
    createCommandEncoder: () => ({
      beginRenderPass: (descriptor: { colorAttachments: unknown[]; depthStencilAttachment?: { view: { texture?: FakeTexture } } }) =>
        validatingEncoder(errors, descriptor.colorAttachments.length ? 'main pass' : 'shadow pass', labelOf,
          descriptor.depthStencilAttachment?.view.texture),
      finish: () => ({}),
    }),
  }
}

function stubGpuGlobals() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16 })
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, STORAGE: 128 })
}

type Internal = Record<string, unknown> & {
  render(): void
  buildPipelines(): void
  buildSceneUB(): void
  buildGrid(): void
  getRenderPipeline(id: string, flavor?: { variant?: string }): FakePipeline
  pipelines: {
    pipelineCache: Map<string, FakePipeline>
    objBGL: FakeBindGroupLayout
    matcapBGL: FakeBindGroupLayout
    envBGL: FakeBindGroupLayout
    shadowBGL: FakeBindGroupLayout
  }
}

const IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]

function harness(options: { meshes?: number; sharedGeometry?: boolean; shadingModel?: string; alpha?: number } = {}) {
  stubGpuGlobals()
  const errors: string[] = []
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as Internal
  const labelOf = (pipeline: FakePipeline) =>
    [...internal.pipelines.pipelineCache].find(([, cached]) => cached === pipeline)?.[0] ?? 'unregistered pipeline'
  internal.dev = validatingDevice(errors, labelOf)
  internal.buildPipelines()
  internal.buildSceneUB()
  internal.buildGrid()
  internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
  internal.ctx = { getCurrentTexture: () => fakeTexture([100, 100]) }
  internal.depthView = {}
  internal.initialized = true
  internal.lost = false
  internal.dead = false
  internal.bounds = { center: [0, 0, 0], radius: 10, min: [-1, -1, -1], max: [1, 1, 1] }
  const objectGroup = { layout: internal.pipelines.objBGL, entries: [] }
  const sharedVB = {}
  internal.meshes = Array.from({ length: options.meshes ?? 1 }, () => ({
    morph: undefined,
    morphSlot: {},
    vb: options.sharedGeometry ? sharedVB : {},
    ib: {},
    ic: 3,
    bg: objectGroup,
    edgeIB: {},
    edgeIC: 2,
    alpha: options.alpha ?? 1,
    visible: true,
    shadingModel: options.shadingModel ?? 'phong',
    transform: IDENTITY,
    inverseTransform: IDENTITY,
    color: [1, 1, 1, 1],
    styleAlpha: options.alpha ?? 1,
    styleSelected: 0,
    styleEdge: 0,
    styleHovered: 0,
    worldBounds: { center: [0, 0, 0], radius: 1e6, min: [-1, -1, -1], max: [1, 1, 1] },
  }))
  return { renderer, internal, errors }
}

describe('WebGPURenderer bind-group validation', () => {
  it('pipeline layouts cover the groups each shader declares, and only the grid adds group(1) to scene pipelines', () => {
    const { internal } = harness()
    const { matcapBGL, envBGL, shadowBGL } = internal.pipelines
    const declaredGroup2: Record<string, FakeBindGroupLayout> = { meshMatcap: matcapBGL, meshPbr: envBGL }
    for (const spec of listShaders()) {
      for (const variant of spec.supportsVariants ? ['uniform', 'instanced'] : ['uniform']) {
        const pipeline = internal.getRenderPipeline(spec.id, { variant })
        const declared = [...pipeline.code.matchAll(/@group\((\d+)\)/g)].map(match => Number(match[1]))
        const label = `${spec.id}/${variant}`
        expect(pipeline.layout.groups.length, label).toBeGreaterThanOrEqual(Math.max(...declared) + 1)
        if (spec.kind !== 'object') expect(pipeline.layout.groups.length, label).toBe(Math.max(...declared) + 1)
        if (declared.includes(2)) expect(pipeline.layout.groups[2], label).toBe(declaredGroup2[spec.id] ?? shadowBGL)
        if (spec.kind === 'grid') expect(pipeline.layout.groups[1], label).toBe(shadowBGL)
      }
    }
    expect(getShader('selectionOverlay').kind).toBe('overlay')
  })

  it('the default frame binds every layout group', () => {
    const { internal, errors } = harness()
    internal.render()
    expect(errors).toEqual([])
  })

  it('the shadow pass binds its groups without sampling its own depth map', () => {
    const { renderer, internal, errors } = harness()
    renderer.setShadowsEnabled(true)
    internal.render()
    expect(errors).toEqual([])
  })

  it.each(['phong', 'matcap', 'pbr', 'toon', 'unlit'])('section caps after %s surfaces bind every layout group', model => {
    const { internal, errors } = harness({ shadingModel: model })
    internal.sectionEnabled = true
    internal.render()
    expect(errors).toEqual([])
  })

  it.each(['phong', 'matcap', 'pbr', 'toon', 'unlit'])('transparent %s surfaces bind every layout group', model => {
    const { internal, errors } = harness({ shadingModel: model, alpha: 0.4 })
    internal.render()
    expect(errors).toEqual([])
  })

  it('render-bundle paths (16+ meshes) bind every group for surfaces, section caps and edges', () => {
    const { internal, errors } = harness({ meshes: 20 })
    internal.sectionEnabled = true
    internal.displayMode = 'edges'
    internal.render()
    expect(errors).toEqual([])
  })

  it('instanced paths bind every group for surfaces and edges', () => {
    const { renderer, internal, errors } = harness({ meshes: 20, sharedGeometry: true })
    internal.displayMode = 'edges'
    renderer.setShadowsEnabled(true)
    internal.render()
    expect(errors).toEqual([])
  })

  it('deep selection after transparent matcap/PBR surfaces binds every layout group', () => {
    for (const model of ['matcap', 'pbr']) {
      const { internal, errors } = harness({ shadingModel: model, alpha: 0.4 })
      internal.selectionMode = 'object'
      internal.selected = 0
      internal.selectedHit = { cycleIndex: 1 }
      internal.render()
      expect(errors, model).toEqual([])
    }
  })

  it('overlay and measurement lines bind every group, with or without the grid', () => {
    for (const gridVisible of [true, false]) {
      const { internal, errors } = harness()
      internal.gridVisible = gridVisible
      internal.measurementVB = {}
      internal.measurementVC = 2
      for (const slot of ['sourceFaceSlot', 'sourceLineSlot', 'selectionFaceSlot', 'selectionLineSlot', 'deepSelectionLineSlot']) {
        internal[slot] = { buffer: {}, capacity: 1, count: 2 }
      }
      internal.render()
      expect(errors, `gridVisible=${gridVisible}`).toEqual([])
    }
  })
})
