import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

/**
 * Renderer-level check that the section-cap pass is issued exactly when the
 * section plane clips a surfaced scene: the frame must draw opaque meshes
 * with the mesh pipeline, then redraw them with the front-culled cap pipeline.
 * Pipelines come from the renderer's cached factory (buildPipelines warmup);
 * the fake device returns descriptor-carrying stand-ins.
 */

interface FakeInternals {
  canvas: unknown
  dev: unknown
  ctx: unknown
  initialized: boolean
  lost: boolean
  dead: boolean
  drawable: boolean
  gridVisible: boolean
  sceneUB: unknown
  sceneBG: unknown
  sectionEnabled: boolean
  sectionNormal: [number, number, number]
  sectionOffset: number
  displayMode: string
  meshes: unknown[]
  render(): void
  buildPipelines(): void
  getRenderPipeline(id: string): unknown
}

function fakeMesh() {
  return {
    morph: undefined,
    morphSlot: {},
    vb: {},
    ib: {},
    ic: 3,
    bg: {},
    edgeIB: null,
    edgeIC: 0,
    alpha: 1,
    visible: true,
    shadingModel: 'phong',
    worldBounds: {
      center: [0, 0, 0],
      radius: 1e6, // never frustum-culled, whatever the default camera
      min: [-1, -1, -1],
      max: [1, 1, 1],
    },
  }
}

function harness() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16 })
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, STORAGE: 128 })
  const pipelines: unknown[] = []
  const pass = {
    setPipeline: (pipeline: unknown) => { pipelines.push(pipeline) },
    setBindGroup: () => undefined,
    setVertexBuffer: () => undefined,
    setIndexBuffer: () => undefined,
    draw: () => undefined,
    drawIndexed: () => undefined,
    executeBundles: () => undefined,
    setImmediates: () => undefined,
    end: () => undefined,
  }
  const encoder = { beginRenderPass: () => pass, finish: () => ({}) }
  const dev = {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: { writeBuffer: () => undefined, writeTexture: () => undefined, submit: () => undefined },
    createTexture: () => ({ createView: () => ({}), destroy: () => undefined }),
    createSampler: () => ({}),
    createBuffer: () => ({}),
    createBindGroupLayout: () => ({}),
    createPipelineLayout: () => ({}),
    createShaderModule: () => ({}),
    createBindGroup: () => ({}),
    createRenderPipeline: (descriptor: unknown) => ({ pipe: descriptor }),
    createCommandEncoder: () => encoder,
  }
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as FakeInternals
  internal.dev = dev
  // Real setup: bind group/pipeline layouts, dummy textures, pipeline warmup.
  internal.buildPipelines()
  internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
  internal.ctx = { getCurrentTexture: () => ({ createView: () => ({}) }) }
  internal.initialized = true
  internal.lost = false
  internal.dead = false
  internal.gridVisible = false
  internal.sceneUB = {}
  internal.sceneBG = {}
  internal.meshes = [fakeMesh()]
  internal.sectionNormal = [0, 0, 1]
  internal.sectionOffset = 0
  // Pipeline stand-ins as set during the warmup; identity marks the pass role.
  const meshPipe = internal.getRenderPipeline('mesh')
  const capPipe = internal.getRenderPipeline('meshSectionCap')
  return { internal, pipelines, meshPipe, capPipe }
}

describe('WebGPURenderer section-cap pass', () => {
  it('redraws opaque meshes with the cap pipeline after the surface pass when the section plane is enabled', () => {
    const { internal, pipelines, meshPipe, capPipe } = harness()
    internal.sectionEnabled = true
    internal.render()
    const meshAt = pipelines.indexOf(meshPipe)
    const capAt = pipelines.indexOf(capPipe)
    expect(meshAt).toBeGreaterThanOrEqual(0)
    expect(capAt).toBeGreaterThan(meshAt)
  })

  it('skips the cap pass when the section plane is disabled', () => {
    const { internal, pipelines, meshPipe, capPipe } = harness()
    internal.sectionEnabled = false
    internal.render()
    expect(pipelines).toContain(meshPipe)
    expect(pipelines).not.toContain(capPipe)
  })

  it('skips the cap pass in xray mode (surfaces route through the transparent pass)', () => {
    const { internal, pipelines, capPipe } = harness()
    internal.sectionEnabled = true
    internal.displayMode = 'xray'
    internal.render()
    expect(pipelines).not.toContain(capPipe)
  })
})
