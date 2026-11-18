import { afterEach, describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/render/webgpuRenderer'
import { PipelineFactory } from '../src/render/pipelineFactory'
import { InstanceStepRuns, instanceStepEligible, type InstanceStepMesh } from '../src/render/instanceStepRuns'
import { INSTANCE_RECORD_BYTES } from '../src/render/instancedScene'
import { EDGE_WGSL, MESH_SECTION_CAP_WGSL, MESH_SHADOW_WGSL, OBJ_STRUCT } from '../src/render/shaders'
import {
  instanceStepEdgeShader,
  instanceStepObjectShader,
  instanceStepShadowShader,
} from '../src/render/shaders/variants'

// Instance-step runs for the shadow, edge, transparent and section-cap passes
// (opt-in with the instancing flag), plus deferred overlay rebuilds. Fake
// device harness as in tests/webgpuRendererInstancing.test.ts.

type Loose = { [key: string]: Loose } & ((...args: unknown[]) => Loose) & Loose[]

const balanced = (code: string, open: string, close: string) => code.split(open).length === code.split(close).length

function stubGpuGlobals() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16, COPY_SRC: 1 })
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, STORAGE: 128 })
}

afterEach(() => vi.unstubAllGlobals())

describe('instance-step shader variants for edges, shadow and caps', () => {
  it('edge variant reads the model from slot 1 and style from a flat varying', () => {
    const code = instanceStepEdgeShader(EDGE_WGSL)
    expect(balanced(code, '{', '}')).toBe(true)
    expect(balanced(code, '(', ')')).toBe(true)
    expect(code).toContain(OBJ_STRUCT)
    expect(code).not.toContain('var<uniform> ob')
    expect(code).not.toContain('fromPos')
    expect(code).toContain('inst: InstanceIn) -> EdgeV {')
    expect(code).toContain('@location(2) @interpolate(flat) istyle: vec4f')
    expect(code).toContain('return EdgeV(ob.color, ob.style, ')
    expect(code).toContain('let ob = instanceObj(mat4x4f(), v.icolor, v.istyle);')
  })

  it('shadow variant has no object group and no morph input', () => {
    const code = instanceStepShadowShader(MESH_SHADOW_WGSL)
    expect(balanced(code, '{', '}')).toBe(true)
    expect(code).not.toContain('@group(1)')
    expect(code).not.toContain('fromPos')
    expect(code).toContain('inst: InstanceIn) -> @builtin(position) vec4f {')
    expect(code).toContain('let local = pos;')
  })

  it('section cap uses the surface variant without a shadow group', () => {
    const code = instanceStepObjectShader(MESH_SECTION_CAP_WGSL)
    expect(code).not.toContain('@group(1)')
    expect(code).not.toContain('@group(2)')
  })

  it('rejects shaders outside each contract', () => {
    expect(() => instanceStepEdgeShader(MESH_SHADOW_WGSL)).toThrow('Instance-step edge shader contract changed')
    expect(() => instanceStepShadowShader(EDGE_WGSL)).toThrow('Instance-step shadow shader contract changed')
  })
})

function fakeFactoryDevice() {
  const modules: string[] = []
  const dev: Loose = {
    createBindGroupLayout: (desc: Loose) => ({ kind: 'bgl', desc }),
    createPipelineLayout: (desc: Loose) => ({ kind: 'layout', desc }),
    createShaderModule: (desc: { code: string }) => (modules.push(desc.code), { code: desc.code }),
    createRenderPipeline: (desc: Loose) => ({ desc }),
  }
  return { dev: dev as unknown as GPUDevice, modules }
}

describe('pipeline factory: instance-step edge, shadow and cap pipelines', () => {
  it('builds each with an instance-step slot 1 and the matching layout', () => {
    stubGpuGlobals()
    const { dev } = fakeFactoryDevice()
    const factory = new PipelineFactory()
    factory.build(dev, 'bgra8unorm')
    const desc = (id: string) =>
      (factory.getRenderPipeline(dev, id, { variant: 'instanceStep' }) as unknown as { desc: Loose }).desc
    const edge = desc('edge')
    expect(edge.layout).toBe(factory.sceneOnlyLayout)
    expect(edge.vertex.module.code).toBe(instanceStepEdgeShader(EDGE_WGSL))
    expect(edge.vertex.buffers[0].attributes.map((a: Loose) => a.shaderLocation)).toEqual([0])
    expect(edge.vertex.buffers[1].stepMode).toBe('instance')
    expect(edge.primitive.topology).toBe('line-list')
    const shadow = desc('meshShadow')
    expect(shadow.layout).toBe(factory.sceneOnlyLayout)
    expect(shadow.fragment).toBeUndefined()
    expect(shadow.depthStencil.format).toBe('depth32float')
    expect(shadow.vertex.buffers[1].arrayStride).toBe(INSTANCE_RECORD_BYTES)
    const cap = desc('meshSectionCap')
    expect(cap.layout).toBe(factory.sceneOnlyLayout)
    expect(cap.primitive.cullMode).toBe('front')
    // The uniform variants keep their layouts.
    const uniformShadow = (factory.getRenderPipeline(dev, 'meshShadow') as unknown as { desc: Loose }).desc
    expect(uniformShadow.layout).toBe(factory.shadowObjectLayout)
    expect(() => factory.getRenderPipeline(dev, 'meshShadow', { variant: 'instanced' })).toThrow()
  })
})

function runMesh(vb: object, ib: object, overrides: Partial<InstanceStepMesh> = {}): InstanceStepMesh {
  return {
    vb: vb as GPUBuffer,
    ib: ib as GPUBuffer,
    ic: 3,
    edgeIB: null,
    edgeIC: 0,
    transform: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    color: [1, 1, 1, 1],
    styleAlpha: 1,
    styleSelected: 0,
    styleEdge: 0,
    styleHovered: 0,
    ...overrides,
  }
}

function runDevice() {
  const writes: Array<{ offset: number; floats: number }> = []
  const device = {
    createBuffer: (desc: { size: number }) => ({ size: desc.size, destroy: () => undefined }),
    queue: {
      writeBuffer: (_b: unknown, offset: number, _d: Float32Array, _o = 0, size?: number) =>
        writes.push({ offset, floats: size ?? 0 }),
    },
  }
  return { device: device as unknown as GPUDevice, writes }
}

describe('InstanceStepRuns', () => {
  const a = {},
    b = {},
    ia = {},
    ib = {}

  it('consecutive order keeps the input order; asset order groups shared geometry', () => {
    const meshes = [runMesh(a, ia), runMesh(b, ib), runMesh(a, ia)]
    const runs = new InstanceStepRuns()
    expect(runs.plan(meshes, { order: 'consecutive' }).map(r => [r.first, r.count])).toEqual([
      [0, 1],
      [1, 1],
      [2, 1],
    ])
    expect(runs.plan(meshes, { order: 'asset' }).map(r => [r.vb === a, r.first, r.count])).toEqual([
      [true, 0, 2],
      [false, 2, 1],
    ])
  })

  it('edge runs use the edge index buffer and count', () => {
    const edgeIB = {}
    const meshes = [
      runMesh(a, ia, { edgeIB: edgeIB as GPUBuffer, edgeIC: 8 }),
      runMesh(a, ia, { edgeIB: edgeIB as GPUBuffer, edgeIC: 8 }),
    ]
    const runs = new InstanceStepRuns()
    const [run] = runs.plan(meshes, { order: 'asset', edges: true })
    expect(run).toMatchObject({ ib: edgeIB, ic: 8, count: 2 })
  })

  it('a style change rewrites only that instance record', () => {
    stubGpuGlobals()
    const { device, writes } = runDevice()
    const meshes = [runMesh(a, ia), runMesh(a, ia), runMesh(a, ia)]
    const runs = new InstanceStepRuns()
    runs.plan(meshes, { order: 'asset' })
    runs.upload(device)
    expect(writes).toEqual([{ offset: 0, floats: 72 }])
    writes.length = 0
    meshes[2]!.styleSelected = 1
    runs.upload(device)
    expect(writes).toEqual([{ offset: 2 * INSTANCE_RECORD_BYTES, floats: 24 }])
    writes.length = 0
    runs.upload(device)
    expect(writes).toEqual([])
  })

  it('morphing meshes and material tails are not eligible', () => {
    expect(instanceStepEligible(runMesh(a, ia))).toBe(true)
    expect(instanceStepEligible(runMesh(a, ia, { morph: {} }))).toBe(false)
    expect(instanceStepEligible(runMesh(a, ia, { material: {} }))).toBe(false)
    expect(instanceStepEligible(runMesh(a, ia, { material: {}, edgeIB: {} as GPUBuffer, edgeIC: 2 }), true)).toBe(true)
  })
})

/* ── Renderer harness ─────────────────────────────── */

const sharedVB = {},
  sharedIB = {},
  sharedEdgeIB = {}

function fakeMesh(alpha = 1) {
  return {
    vertices: new Float32Array(18),
    indices: new Uint32Array(3),
    edgeIndices: new Uint32Array(4),
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    inverseTransform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    color: [1, 1, 1, 1],
    styleAlpha: alpha,
    styleSelected: 0,
    styleEdge: 1,
    styleHovered: 0,
    morph: undefined,
    morphSlot: {},
    vb: sharedVB,
    ib: sharedIB,
    ic: 3,
    ub: {},
    bg: 'objBG',
    edgeIB: sharedEdgeIB,
    edgeIC: 4,
    alpha,
    visible: true,
    shadingModel: 'phong',
    worldBounds: { center: [0, 0, 0], radius: 1e6, min: [-1, -1, -1], max: [1, 1, 1] },
  }
}

interface PassRecord {
  colors: number
  draws: number[][]
}

function harness(meshes = [fakeMesh(), fakeMesh(), fakeMesh(), fakeMesh(0.5), fakeMesh(0.5)]) {
  stubGpuGlobals()
  const passes: PassRecord[] = []
  const pipelines: Array<{ vertex?: { buffers?: Array<{ stepMode?: string }> } }> = []
  const makePass = (descriptor: { colorAttachments: unknown[] }) => {
    const entry: PassRecord = { colors: descriptor.colorAttachments.length, draws: [] }
    passes.push(entry)
    const noop = () => undefined
    return {
      setPipeline: noop,
      setBindGroup: noop,
      setVertexBuffer: noop,
      setIndexBuffer: noop,
      draw: noop,
      drawIndexed: (...args: number[]) => entry.draws.push(args),
      executeBundles: noop,
      setImmediates: noop,
      end: noop,
    }
  }
  const view = {}
  const dev = {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: { writeBuffer: () => undefined, writeTexture: () => undefined, submit: () => undefined },
    createTexture: () => ({ createView: () => view, destroy: () => undefined }),
    createSampler: () => ({}),
    createBuffer: (d: { size: number }) => ({ size: d.size, destroy: () => undefined }),
    createBindGroupLayout: () => ({}),
    createPipelineLayout: () => ({}),
    createShaderModule: () => ({}),
    createBindGroup: () => ({}),
    createRenderPipeline: (d: (typeof pipelines)[number]) => (pipelines.push(d), { pipe: d }),
    createRenderBundleEncoder: () => ({
      setPipeline: () => undefined,
      setBindGroup: () => undefined,
      setVertexBuffer: () => undefined,
      setIndexBuffer: () => undefined,
      drawIndexed: () => undefined,
      finish: () => ({}),
    }),
    createCommandEncoder: () => ({ beginRenderPass: makePass, finish: () => ({}) }),
  }
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as Record<string, unknown> & { render(): void; buildPipelines(): void }
  internal.dev = dev
  internal.buildPipelines()
  internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
  internal.ctx = { getCurrentTexture: () => ({ createView: () => ({}) }) }
  internal.initialized = true
  internal.lost = false
  internal.dead = false
  internal.gridVisible = false
  internal.sceneUB = 'sceneUB'
  internal.sceneBG = 'sceneBG'
  internal.meshes = meshes
  internal.bounds = { center: [0, 0, 0], radius: 10, min: [-1, -1, -1], max: [1, 1, 1] }
  internal.displayMode = 'edges'
  internal.sectionEnabled = true
  renderer.setShadowsEnabled(true)
  const instanceStepPipelines = () => pipelines.filter(p => p.vertex?.buffers?.[1]?.stepMode === 'instance').length
  return { renderer, internal, passes, instanceStepPipelines }
}

describe('WebGPURenderer instance-step passes', () => {
  it('flag off: shadow, cap, transparent and edge passes draw per entity with no instance-step pipeline', () => {
    const { internal, passes, instanceStepPipelines } = harness()
    // Warmup already holds the opaque instance-step pipeline; the frame adds none.
    const warm = instanceStepPipelines()
    internal.render()
    const shadow = passes.find(p => p.colors === 0)!
    expect(shadow.draws).toEqual([[3], [3], [3]])
    const main = passes.find(p => p.colors === 1)!
    expect(main.draws.length).toBeGreaterThan(5)
    expect(main.draws.every(d => d.length === 1)).toBe(true)
    expect(instanceStepPipelines()).toBe(warm)
  })

  it('flag on: shadow, cap, transparent and edges draw instanced runs', () => {
    const { renderer, internal, passes } = harness()
    renderer.setInstancingEnabled(true)
    internal.render()
    const shadow = passes.find(p => p.colors === 0)!
    expect(shadow.draws).toEqual([[3, 3, 0, 0, 0]])
    const main = passes.find(p => p.colors === 1)!
    expect(main.draws).toEqual([
      [3, 3], // opaque surfaces (instancedOpaque)
      [3, 3, 0, 0, 0], // section caps
      [3, 2, 0, 0, 0], // sorted transparent, one consecutive run
      [4, 5, 0, 0, 0], // edges of all five meshes
    ])
  })

  it('flag on: a morphing caster leaves the instanced passes for the per-entity path', () => {
    const meshes = [fakeMesh(), fakeMesh(), fakeMesh()]
    ;(meshes[2] as { morph: unknown }).morph = { started: 0 }
    const { renderer, internal, passes } = harness(meshes)
    renderer.setInstancingEnabled(true)
    internal.render()
    const shadow = passes.find(p => p.colors === 0)!
    expect(shadow.draws).toEqual([[3, 2, 0, 0, 0], [3]])
    const main = passes.find(p => p.colors === 1)!
    // Transitioning frames skip every instanced pass, as the opaque path does.
    expect(main.draws.every(d => d.length === 1)).toBe(true)
  })
})

describe('WebGPURenderer deferred overlays', () => {
  it('flag off rebuilds overlays at once; flag on rebuilds them at the next frame', () => {
    const { renderer, internal } = harness()
    const overlays = internal.overlays as { rebuildSelection: (...a: unknown[]) => void }
    const spy = vi.spyOn(overlays, 'rebuildSelection').mockImplementation(() => undefined)
    const rebuild = () => (internal as unknown as { rebuildSelectionOverlays(): void }).rebuildSelectionOverlays()
    rebuild()
    expect(spy).toHaveBeenCalledTimes(1)
    renderer.setInstancingEnabled(true)
    rebuild()
    rebuild()
    expect(spy).toHaveBeenCalledTimes(1)
    internal.render()
    expect(spy).toHaveBeenCalledTimes(2)
    internal.render()
    expect(spy).toHaveBeenCalledTimes(2)
    rebuild()
    renderer.setInstancingEnabled(false)
    expect(spy).toHaveBeenCalledTimes(3)
  })
})
