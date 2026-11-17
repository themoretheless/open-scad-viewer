import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  DEEP_MESH_WGSL,
  EDGE_WGSL,
  MESH_TOON_WGSL,
  MESH_UNLIT_WGSL,
  MESH_WGSL,
  OBJ_STRUCT,
} from '../src/render/shaders'
import { instanceStepObjectShader } from '../src/render/shaders/variants'
import { PipelineFactory } from '../src/render/pipelineFactory'
import {
  GPU_INSTANCE_PICKING_WGSL,
  GpuPicker,
  decodePickId,
  instancedPickId,
  type GpuPickerMesh,
} from '../src/render/gpuPicker'
import { InstancedOpaquePass, type InstanceableMesh } from '../src/render/instancedOpaquePass'
import {
  INSTANCE_RECORD_BYTES,
  InstancedSceneBuffers,
  packInstanceRecords,
  planInstanceBatches,
} from '../src/render/instancedScene'
import { identityMat4 } from '../src/render/gpuCsgPreview'

/** Loosely typed fake-device records (index, call and iterate freely). */
type Loose = { [key: string]: Loose } & ((...args: unknown[]) => Loose) & Loose[]

function balanced(code: string, open: string, close: string) {
  return code.split(open).length === code.split(close).length
}

function stubGpuGlobals() {
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, STORAGE: 128, COPY_DST: 8, MAP_READ: 1 })
  vi.stubGlobal('GPUTextureUsage', { RENDER_ATTACHMENT: 16, COPY_SRC: 4 })
  vi.stubGlobal('GPUMapMode', { READ: 1 })
}

afterEach(() => vi.unstubAllGlobals())

describe('instance-step shader variant', () => {
  it.each([
    ['mesh', MESH_WGSL],
    ['meshToon', MESH_TOON_WGSL],
    ['meshUnlit', MESH_UNLIT_WGSL],
    ['deepMesh', DEEP_MESH_WGSL],
  ])('%s reads the model from instance attributes and style from flat varyings', (_name, source) => {
    const code = instanceStepObjectShader(source)
    expect(balanced(code, '{', '}')).toBe(true)
    expect(balanced(code, '(', ')')).toBe(true)
    expect(code).toContain(OBJ_STRUCT)
    expect(code).not.toContain('var<uniform> ob')
    expect(code).not.toContain('fromPos')
    expect(code).not.toContain('@group(2)')
    expect(code).toContain('@location(3) m0: vec4f')
    expect(code).toContain('@location(8) style: vec4f')
    expect(code).toContain('inst: InstanceIn) -> V {')
    expect(code).toContain('let ob = instanceObj(mat4x4f(inst.m0, inst.m1, inst.m2, inst.m3), inst.color, inst.style);')
    expect(code).toContain('@location(2) @interpolate(flat) icolor: vec4f')
    expect(code).toContain('let ob = instanceObj(mat4x4f(), v.icolor, v.istyle);')
    expect(code).toContain('return V(ob.color, ob.style, ')
  })

  it('moves the shadow map to group(1) for the scene + shadow layout', () => {
    const code = instanceStepObjectShader(MESH_WGSL)
    expect(code).toContain('@group(1) @binding(0) var shadowMap: texture_depth_2d;')
    expect(code).toContain('@group(1) @binding(1) var shadowSampler: sampler_comparison;')
  })

  it('rejects shaders outside the mesh-surface contract', () => {
    expect(() => instanceStepObjectShader(EDGE_WGSL)).toThrow('Instance-step shader contract changed')
    expect(() => instanceStepObjectShader('fn nope() {}')).toThrow('Instance-step shader contract changed')
  })

  it('the instanced pick shader writes pickBase + instance_index', () => {
    expect(balanced(GPU_INSTANCE_PICKING_WGSL, '{', '}')).toBe(true)
    expect(GPU_INSTANCE_PICKING_WGSL).toContain('@builtin(instance_index) instance: u32')
    expect(GPU_INSTANCE_PICKING_WGSL).toContain('batch.pickBase + instance')
    expect(GPU_INSTANCE_PICKING_WGSL).toContain('@location(3) m0: vec4f')
  })
})

function fakeFactoryDevice() {
  const pipelines: Loose[] = []
  const modules: string[] = []
  const dev: Loose = {
    createBindGroupLayout: (desc: Loose) => ({ kind: 'bgl', desc }),
    createPipelineLayout: (desc: Loose) => ({ kind: 'layout', desc }),
    createShaderModule: (desc: { code: string }) => (modules.push(desc.code), { code: desc.code }),
    createRenderPipeline: (desc: Loose) => (pipelines.push(desc), { desc }),
  }
  return { dev: dev as GPUDevice, pipelines, modules }
}

describe('pipeline factory: instance-step variant', () => {
  it('uses a scene + shadow layout and an instance-step vertex buffer at slot 1', () => {
    stubGpuGlobals()
    const { dev } = fakeFactoryDevice()
    const factory = new PipelineFactory()
    factory.build(dev, 'bgra8unorm')
    const pipeline = factory.getRenderPipeline(dev, 'mesh', { variant: 'instanceStep' }) as unknown as { desc: Loose }
    const desc = pipeline.desc
    expect(desc.layout).toBe(factory.instanceStepLayout)
    expect((factory.instanceStepLayout as unknown as Loose).desc.bindGroupLayouts).toEqual([
      factory.sceneBGL,
      factory.shadowBGL,
    ])
    expect(desc.vertex.module.code).toBe(instanceStepObjectShader(MESH_WGSL))
    const [vertices, instances] = desc.vertex.buffers
    expect(vertices.stepMode ?? 'vertex').toBe('vertex')
    expect(vertices.attributes.map((a: Loose) => a.shaderLocation)).toEqual([0, 1])
    expect(instances.stepMode).toBe('instance')
    expect(instances.arrayStride).toBe(INSTANCE_RECORD_BYTES)
    expect(instances.attributes.map((a: Loose) => [a.shaderLocation, a.offset])).toEqual([
      [3, 0],
      [4, 16],
      [5, 32],
      [6, 48],
      [7, 64],
      [8, 80],
    ])
    // The per-entity variant is unchanged: object group at 1, morph slot at 1.
    const uniform = (factory.getRenderPipeline(dev, 'mesh') as unknown as { desc: Loose }).desc
    expect(uniform.layout).toBe(factory.objectLayout)
    expect(uniform.vertex.buffers[1].attributes[0].shaderLocation).toBe(2)
    expect(uniform.vertex.buffers[1].stepMode).toBeUndefined()
  })

  it('warmup builds the instance-step pipeline and rejects it for edge shaders', () => {
    stubGpuGlobals()
    const { dev, modules } = fakeFactoryDevice()
    const factory = new PipelineFactory()
    factory.build(dev, 'bgra8unorm')
    factory.warmup(dev)
    expect(modules).toContain(instanceStepObjectShader(MESH_WGSL))
    expect(() => factory.getRenderPipeline(dev, 'edge', { variant: 'instanceStep' })).toThrow(
      "does not support the 'instanceStep'",
    )
    expect(() => factory.getRenderPipeline(dev, 'meshPbr', { variant: 'instanceStep' })).toThrow()
  })
})

const buffer = (name: string) => ({ name, destroy: vi.fn() }) as unknown as GPUBuffer
const translation = (x: number) => new Float32Array([1, 0, 0, x, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])

function gmesh(vb: GPUBuffer, x: number, extra: Partial<InstanceableMesh> = {}): InstanceableMesh {
  return {
    vb,
    ib: buffer('ib'),
    ic: 36,
    vertices: new Float32Array(18),
    indices: new Uint32Array(36),
    edgeIndices: new Uint32Array(0),
    transform: translation(x),
    color: [0.5, 0.6, 0.7, 1],
    styleAlpha: 1,
    styleSelected: 0,
    styleEdge: 0,
    styleHovered: 0,
    shadingModel: 'phong',
    ...extra,
  }
}

function fakeDevice() {
  const created: Loose[] = []
  const device: Loose = {
    createBuffer: vi.fn((desc: Loose) => {
      const created_ = { ...desc, destroy: vi.fn() }
      created.push(created_)
      return created_
    }),
    queue: { writeBuffer: vi.fn() },
  }
  return { device: device as GPUDevice, created }
}

function fakePass() {
  return {
    setPipeline: vi.fn(),
    setBindGroup: vi.fn(),
    setVertexBuffer: vi.fn(),
    setIndexBuffer: vi.fn(),
    drawIndexed: vi.fn(),
  }
}

describe('instanced opaque pass', () => {
  it('packs row-major transforms column-major with color and style per record', () => {
    const meshes = [{ ...gmesh(buffer('vb'), 3), styleSelected: 1, styleHovered: 1, styleAlpha: 0.5 }]
    const plan = planInstanceBatches(meshes)
    const record = packInstanceRecords(plan.batches[0]!, meshes as unknown as Loose)
    expect(record.length).toBe(24)
    expect(Array.from(record.subarray(12, 16))).toEqual([3, 0, 0, 1])
    expect(Array.from(record.subarray(16, 20))).toEqual([0.5, 0.6, 0.7, 1].map(Math.fround))
    expect(Array.from(record.subarray(20, 24))).toEqual([0.5, 1, 0, 1])
  })

  it('draws shared assets once with instanceCount and leaves morph/LOD/material entities per-entity', () => {
    const { device, created } = fakeDevice()
    const bolt = buffer('bolt'),
      nut = buffer('nut')
    const morphing = gmesh(bolt, 9, { morph: {} })
    const lod = gmesh(bolt, 10, { lod: {} })
    const material = gmesh(bolt, 11, { material: {} })
    const draws = [
      ...Array.from({ length: 5 }, (_, index) => gmesh(bolt, index)),
      gmesh(nut, 20),
      gmesh(nut, 21),
      morphing,
      lod,
      material,
    ]
    const pass = new InstancedOpaquePass()
    expect(pass.prepare(device, draws, mesh => draws.indexOf(mesh))).toBe(true)
    expect(pass.perEntity).toEqual([morphing, lod, material])
    // Adopted geometry: only the two instance-record buffers are created.
    expect(created.map(entry => entry.size)).toEqual([5 * INSTANCE_RECORD_BYTES, 2 * INSTANCE_RECORD_BYTES])
    expect(pass.metrics).toMatchObject({ geometryUploads: 0, drawCalls: 2, instances: 7 })
    const encoder = fakePass()
    const pipeline = {} as GPURenderPipeline,
      scene = {} as GPUBindGroup,
      shadow = {} as GPUBindGroup
    pass.encode(encoder as unknown as GPURenderPassEncoder, pipeline, scene, shadow)
    expect(encoder.setPipeline).toHaveBeenCalledWith(pipeline)
    expect(encoder.setBindGroup.mock.calls).toEqual([
      [0, scene],
      [1, shadow],
    ])
    expect(encoder.drawIndexed.mock.calls).toEqual([
      [36, 5],
      [36, 2],
    ])
    expect(encoder.setVertexBuffer.mock.calls[0]).toEqual([0, bolt])
    expect(encoder.setVertexBuffer.mock.calls[1]![0]).toBe(1)
    expect(encoder.setVertexBuffer.mock.calls[1]![1].size).toBe(5 * INSTANCE_RECORD_BYTES)
    pass.clear()
    expect(bolt.destroy).not.toHaveBeenCalled()
  })

  it('falls back when no asset is shared', () => {
    const { device } = fakeDevice()
    const pass = new InstancedOpaquePass()
    const draws = [gmesh(buffer('a'), 0), gmesh(buffer('b'), 1)]
    expect(pass.prepare(device, draws, mesh => draws.indexOf(mesh))).toBe(false)
    expect(pass.pickInstances()).toBeUndefined()
  })

  it('a selection change rewrites only that instance style slice', () => {
    const { device } = fakeDevice()
    const vb = buffer('vb')
    const draws = Array.from({ length: 4 }, (_, index) => gmesh(vb, index))
    const pass = new InstancedOpaquePass()
    pass.prepare(device, draws, mesh => draws.indexOf(mesh))
    const write = device.queue.writeBuffer as ReturnType<typeof vi.fn>
    write.mockClear()
    draws[2]!.styleSelected = 1
    pass.prepare(device, draws, mesh => draws.indexOf(mesh))
    expect(write).toHaveBeenCalledTimes(1)
    const [, byteOffset, , floatOffset, floats] = write.mock.calls[0]!
    expect(byteOffset).toBe(2 * INSTANCE_RECORD_BYTES + 64)
    expect(floatOffset).toBe(2 * 24 + 16)
    expect(floats).toBe(8)
  })
})

describe('GPU picker: instanced id math', () => {
  it('offsets instanced ids past per-entity ids and resolves the instance', () => {
    const resolve = (id: number) => (id >= 1 && id <= 100 ? 1000 + id : null)
    const instances = { batches: [], covers: () => false, resolve }
    expect(instancedPickId(10, 1, 36)).toBe(47)
    expect(decodePickId(0, 10, instances)).toBeNull()
    expect(decodePickId(4, 10, instances)).toBe(3)
    expect(decodePickId(instancedPickId(10, 1, 36), 10, instances)).toBe(1000 + 37)
    expect(decodePickId(47, 10)).toBeNull()
  })

  it('draws covered meshes as instanced batches with pickBase in the batch uniform', async () => {
    stubGpuGlobals()
    let readback = 0
    const passes: Loose[] = []
    const writes: Array<[unknown, number, ArrayBufferView]> = []
    const dev: Loose = {
      createTexture: () => ({ createView: () => ({}), destroy: vi.fn() }),
      createBuffer: (desc: Loose) => ({
        desc,
        getMappedRange: () => new Uint32Array([readback, 0, 0, 0]).buffer,
        mapAsync: vi.fn(async () => {}),
        unmap: vi.fn(),
        destroy: vi.fn(),
      }),
      createBindGroupLayout: (desc: Loose) => ({ desc }),
      createPipelineLayout: (desc: Loose) => ({ desc }),
      createShaderModule: (desc: Loose) => ({ desc }),
      createRenderPipeline: (desc: Loose) => ({ desc }),
      createBindGroup: (desc: Loose) => ({ desc }),
      createCommandEncoder: () => ({
        beginRenderPass: () => {
          const pass = {
            setPipeline: vi.fn(),
            setBindGroup: vi.fn(),
            setVertexBuffer: vi.fn(),
            setIndexBuffer: vi.fn(),
            drawIndexed: vi.fn(),
            end: vi.fn(),
          }
          passes.push(pass)
          return pass
        },
        copyTextureToBuffer: vi.fn(),
        finish: () => ({}),
      }),
      queue: {
        writeBuffer: (target: unknown, offset: number, data: ArrayBufferView) =>
          writes.push([target, offset, Uint32Array.from(data as Uint32Array)]),
        submit: vi.fn(),
      },
    }
    const meshes: GpuPickerMesh[] = Array.from({ length: 3 }, () => ({
      vb: {} as GPUBuffer,
      ib: {} as GPUBuffer,
      ic: 36,
      transform: identityMat4(),
      visible: true,
    }))
    // Mesh 1 and 2 are instances of one asset; staged as plan sources 0 and 1.
    const sceneBuffers = new InstancedSceneBuffers({
      createBuffer: dev.createBuffer,
      queue: { writeBuffer: vi.fn() },
    } as unknown as Loose)
    const vb = {} as GPUBuffer,
      ib = {} as GPUBuffer
    const sources = [1, 2].map(x => ({
      vertices: new Float32Array(18),
      indices: new Uint32Array(36),
      edgeIndices: new Uint32Array(0),
      transform: translation(x) as unknown as Loose,
      gpu: { vb, ib, ic: 36 },
    }))
    sceneBuffers.stage(sources)
    const sourceIndex = [1, 2]
    const instances = {
      batches: sceneBuffers.drawBatches(),
      covers: (index: number) => sourceIndex.includes(index),
      resolve: (id: number) => {
        const batch = sceneBuffers.plan.batches.find(b => id >= b.pickBase && id < b.pickBase + b.members.length)
        return batch ? sourceIndex[batch.members[id - batch.pickBase]!]! : null
      },
    }
    const picker = new GpuPicker()
    // Instance slot 1 of the batch (pickBase 1) writes 3 + 1 + 1 = 5 and resolves to mesh 2.
    readback = instancedPickId(meshes.length, 1, 1)
    const hit = await picker.pick(
      dev,
      meshes,
      { cameraViewProjection: identityMat4(), viewportWidth: 10, viewportHeight: 10 },
      5,
      5,
      instances,
    )
    expect(hit).toBe(2)
    const pass = passes[0]
    expect(pass.drawIndexed.mock.calls).toEqual([[36], [36, 2]])
    expect(pass.setPipeline).toHaveBeenCalledTimes(2)
    const instancePipeline = pass.setPipeline.mock.calls[1][0]
    expect(instancePipeline.desc.vertex.buffers[1].stepMode).toBe('instance')
    expect(instancePipeline.desc.vertex.module.desc.code).toBe(GPU_INSTANCE_PICKING_WGSL)
    const batchWrite = writes.find(([, , data]) => data.byteLength === 16)!
    expect(batchWrite[2][0]).toBe(meshes.length + 1)
    picker.destroy()
  })
})
