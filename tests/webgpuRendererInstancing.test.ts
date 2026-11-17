import { afterEach, describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/render/webgpuRenderer'
import { instancingRequested } from '../src/render/instancedOpaquePass'

// Opt-in gate for the instanced opaque path: off by default (old per-entity
// path), on via `?instancing=1` or setInstancingEnabled. Harness mirrors
// tests/webgpuRendererShadows.test.ts.

interface FakeTexture {
  label: string
  view: object
  destroyed: boolean
  destroy(): void
}

function fakeTexture(label: string): FakeTexture & { createView(): object } {
  const view = { viewOf: label }
  return {
    label,
    view,
    destroyed: false,
    createView: () => view,
    destroy() {
      this.destroyed = true
    },
  }
}

const sharedVB = {},
  sharedIB = {}
function fakeMesh() {
  return {
    vertices: new Float32Array(18),
    indices: new Uint32Array(3),
    edgeIndices: new Uint32Array(0),
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    color: [1, 1, 1, 1],
    styleAlpha: 1,
    styleSelected: 0,
    styleEdge: 0,
    styleHovered: 0,
    morph: undefined,
    morphSlot: {},
    vb: sharedVB,
    ib: sharedIB,
    ic: 3,
    bg: 'objBG',
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

function fakeDevice(record: {
  textures: Array<{ size: number[]; format: string; usage: number }>
  samplers: unknown[]
  bindGroups: Array<{ layout: unknown; entries: unknown[] }>
  pipelines: Array<{
    depthStencil?: { format: string; depthBiasSlopeScale?: number }
    fragment?: unknown
    layout?: { groups: number }
  }>
  passes: Array<{ colors: number; depthFormat: string | null; indexedDraws: number }>
  writes: Array<{ buffer: unknown; data: Float32Array }>
  drawArgs: number[][]
}) {
  const makePass = (descriptor: { colorAttachments: unknown[]; depthStencilAttachment?: { view: object } }) => {
    const pass = {
      descriptor,
      setPipeline: () => undefined,
      setBindGroup: () => undefined,
      setVertexBuffer: () => undefined,
      setIndexBuffer: () => undefined,
      draw: () => undefined,
      drawIndexed: () => undefined,
      executeBundles: () => undefined,
      setImmediates: () => undefined,
      end: () => undefined,
    }
    const entry = {
      colors: descriptor.colorAttachments.length,
      depthFormat: descriptor.depthStencilAttachment
        ? ((descriptor.depthStencilAttachment.view as { viewOf?: string }).viewOf ?? null)
        : null,
      indexedDraws: 0,
    }
    record.passes.push(entry)
    pass.drawIndexed = ((...args: number[]) => {
      entry.indexedDraws++
      record.drawArgs.push(args)
    }) as () => undefined
    return pass
  }
  return {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: {
      writeBuffer: (buffer: unknown, _offset: number, data: Float32Array) => {
        record.writes.push({ buffer, data: Float32Array.from(data) })
      },
      writeTexture: () => undefined,
      submit: () => undefined,
    },
    createTexture: (descriptor: { size: number[]; format: string; usage: number }) => {
      record.textures.push(descriptor)
      return fakeTexture(`tex${record.textures.length}`)
    },
    createSampler: (descriptor: unknown) => {
      const sampler = { descriptor }
      record.samplers.push(sampler)
      return sampler
    },
    createBuffer: (descriptor: { size: number }) => ({ size: descriptor.size, destroy: () => undefined }),
    createBindGroupLayout: () => ({}),
    createPipelineLayout: (descriptor: { bindGroupLayouts: unknown[] }) => ({
      groups: descriptor.bindGroupLayouts.length,
    }),
    createShaderModule: () => ({}),
    createBindGroup: (descriptor: { layout: unknown; entries: unknown[] }) => {
      record.bindGroups.push(descriptor)
      return { bg: descriptor }
    },
    createRenderPipeline: (descriptor: {
      depthStencil?: { format: string; depthBiasSlopeScale?: number }
      fragment?: unknown
    }) => {
      record.pipelines.push(descriptor)
      return { pipe: descriptor }
    },
    createCommandEncoder: () => ({ beginRenderPass: makePass, finish: () => ({}) }),
  }
}

function recordShape() {
  return {
    textures: [] as Array<{ size: number[]; format: string; usage: number }>,
    samplers: [] as unknown[],
    bindGroups: [] as Array<{ layout: unknown; entries: unknown[] }>,
    pipelines: [] as Array<{
      depthStencil?: { format: string; depthBiasSlopeScale?: number }
      fragment?: unknown
      layout?: { groups: number }
    }>,
    passes: [] as Array<{ colors: number; depthFormat: string | null; indexedDraws: number }>,
    writes: [] as Array<{ buffer: unknown; data: Float32Array }>,
    drawArgs: [] as number[][],
  }
}

function stubGpuGlobals() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16 })
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, STORAGE: 128 })
}

function harness() {
  stubGpuGlobals()
  const record = recordShape()
  const dev = fakeDevice(record)
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as Record<string, unknown> & { render(): void; buildPipelines(): void }
  internal.dev = dev
  // Real setup: shadow dummy, bind groups, vertex layouts; the draw-path
  // pipelines are replaced with string markers below for pass assertions.
  internal.buildPipelines()
  internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
  internal.ctx = { getCurrentTexture: () => ({ createView: () => ({}) }) }
  internal.initialized = true
  internal.lost = false
  internal.dead = false
  internal.gridVisible = false
  internal.sceneUB = 'sceneUB'
  internal.sceneBG = 'sceneBG'
  internal.meshPipe = 'mesh'
  internal.meshPipeT = 'meshT'
  internal.meshImmediatePipeT = null
  internal.sectionCapPipe = 'cap'
  internal.shadowPipe = 'shadow'
  ;(internal.textures as Record<string, unknown>).shadowSampler = {}
  internal.meshes = [fakeMesh(), fakeMesh(), fakeMesh()]
  internal.bounds = { center: [0, 0, 0], radius: 10, min: [-1, -1, -1], max: [1, 1, 1] }
  return { renderer, internal, record }
}

afterEach(() => vi.unstubAllGlobals())

describe('WebGPURenderer instanced opaque opt-in', () => {
  it('reads ?instancing=1 and nothing else', () => {
    expect(instancingRequested('')).toBe(false)
    expect(instancingRequested('?instancing=0')).toBe(false)
    expect(instancingRequested('?foo=1&instancing=1')).toBe(true)
  })

  it('flag off (default): three shared-asset meshes draw per entity, no instance count', () => {
    const { renderer, internal, record } = harness()
    expect(renderer.isInstancingEnabled()).toBe(false)
    internal.render()
    expect(record.drawArgs).toEqual([[3], [3], [3]])
  })

  it('flag on: the shared asset draws once with instanceCount 3', () => {
    const { renderer, internal, record } = harness()
    renderer.setInstancingEnabled(true)
    internal.render()
    expect(record.drawArgs).toEqual([[3, 3]])
    const instancedPipeline = record.pipelines.find(
      p => (p as { vertex?: { buffers?: Array<{ stepMode?: string }> } }).vertex?.buffers?.[1]?.stepMode === 'instance',
    )
    expect(instancedPipeline).toBeDefined()
  })
})
