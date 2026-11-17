import { describe, expect, it, vi } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import {
  InstancedSceneBuffers,
  instancePickId,
  planInstanceBatches,
  resolveInstancePick,
} from '../src/render/instancedScene'
import { RecordingRenderer } from '../src/render/recordingRenderer'

const translation = (x: number) => new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, x, 0, 0, 1])

const sharedGeometry = {
  vertices: new Float32Array(18),
  indices: new Uint32Array([0, 1, 2]),
  edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
  color: [1, 1, 1, 1],
}

function instance(index: number, asset = 'asset:bolt'): MeshData {
  return {
    ...sharedGeometry,
    entityId: `entity:${asset}:${index}`,
    geometryAssetId: asset,
    transform: translation(index),
    faceIds: new Uint32Array([0]),
  } as unknown as MeshData
}

function fakeDevice() {
  const buffers: { size: number; destroy: ReturnType<typeof vi.fn> }[] = []
  const device = {
    createBuffer: vi.fn((desc: { size: number }) => {
      const buffer = { size: desc.size, destroy: vi.fn() }
      buffers.push(buffer)
      return buffer
    }),
    queue: { writeBuffer: vi.fn() },
  }
  return { device: device as unknown as GPUDevice, buffers }
}

function fakePass() {
  return { setVertexBuffer: vi.fn(), setIndexBuffer: vi.fn(), drawIndexed: vi.fn() }
}

const hundred = () => Array.from({ length: 100 }, (_, index) => instance(index))

describe('instanced scene staging', () => {
  it('uploads one geometry buffer pair for 100 instances of one asset and draws once with instanceCount', () => {
    const { device, buffers } = fakeDevice()
    const scene = new InstancedSceneBuffers(device)
    scene.stage(hundred())
    expect(scene.metrics).toMatchObject({ geometryUploads: 1, drawCalls: 1, instances: 100, edgeUploads: 0 })
    // Vertex + index + one instance-record buffer of 100 records (mat4 + color + style).
    expect(buffers.map(buffer => buffer.size)).toEqual([72, 12, 9600])
    const pass = fakePass()
    scene.encode(pass as unknown as GPURenderPassEncoder)
    expect(pass.drawIndexed).toHaveBeenCalledTimes(1)
    expect(pass.drawIndexed).toHaveBeenCalledWith(3, 100)
  })

  it('keeps resident assets across publications and drops assets that left', () => {
    const { device, buffers } = fakeDevice()
    const scene = new InstancedSceneBuffers(device)
    scene.stage(hundred())
    scene.stage([...hundred(), instance(100, 'asset:nut')])
    expect(scene.metrics).toMatchObject({ geometryUploads: 1, drawCalls: 2, instances: 101 })
    scene.stage([instance(0, 'asset:nut')])
    expect(scene.metrics.geometryUploads).toBe(0)
    expect(buffers[0]!.destroy).toHaveBeenCalled()
    expect(buffers[1]!.destroy).toHaveBeenCalled()
  })

  it('groups meshes without an asset id by vertex identity', () => {
    const other = { ...sharedGeometry, vertices: new Float32Array(18) }
    const plan = planInstanceBatches([sharedGeometry, sharedGeometry, other] as never)
    expect(plan.batches.map(batch => batch.members)).toEqual([[0, 1], [2]])
  })

  it('defers edge buffers until requested and builds them once per asset', () => {
    const { device } = fakeDevice()
    const scene = new InstancedSceneBuffers(device)
    scene.stage(hundred())
    expect(scene.metrics.edgeUploads).toBe(0)
    const pass = fakePass()
    scene.encode(pass as unknown as GPURenderPassEncoder, true)
    scene.encode(pass as unknown as GPURenderPassEncoder, true)
    expect(scene.metrics.edgeUploads).toBe(1)
    expect(pass.drawIndexed).toHaveBeenLastCalledWith(6, 100)
  })

  it('resolves an instanced pick id to the individual instance', () => {
    const meshes = [instance(0, 'asset:nut'), ...hundred(), instance(1, 'asset:nut')]
    const plan = planInstanceBatches(meshes)
    for (const index of [0, 38, 101]) expect(resolveInstancePick(plan, instancePickId(plan, index)!)).toBe(index)
    expect(resolveInstancePick(plan, 0)).toBeNull()
    expect(resolveInstancePick(plan, 10_000)).toBeNull()
  })
})

describe('recording renderer instancing metrics', () => {
  it('counts one geometry upload for 100 instances and picks instance 37', async () => {
    const renderer = new RecordingRenderer()
    await renderer.init({ clientHeight: 100 } as HTMLCanvasElement)
    renderer.setMeshes(hundred())
    expect(renderer.instancingMetrics).toMatchObject({
      geometryUploads: 1,
      drawCalls: 1,
      instances: 100,
      edgeUploads: 0,
    })
    const pickId = renderer.instancePickId(37)!
    expect(renderer.resolveInstancePick(pickId)).toBe(37)
    renderer.selectMesh(renderer.resolveInstancePick(pickId))
    expect(renderer.selectedIndex).toBe(37)
    renderer.setDisplayMode('edges')
    renderer.setDisplayMode('edges')
    expect(renderer.instancingMetrics.edgeUploads).toBe(1)
    renderer.setMeshes(hundred())
    expect(renderer.instancingMetrics.geometryUploads).toBe(0)
  })

  it('stages the shadow, cap, transparent and edge instance-step passes only with instancing on', async () => {
    const renderer = new RecordingRenderer()
    await renderer.init({ clientHeight: 100 } as HTMLCanvasElement)
    const glass = (index: number) => {
      const mesh = instance(index, 'asset:glass')
      return { ...mesh, color: [1, 1, 1, 0.5] } as MeshData
    }
    renderer.setShadowsEnabled(true)
    renderer.setSection(true, [0, 0, 1], 0)
    renderer.setMeshes([...hundred(), glass(200), glass(201)])
    expect(renderer.instanceStepMetrics.shadow.drawCalls).toBe(0)
    renderer.setInstancingEnabled(true)
    const on = renderer.instanceStepMetrics
    expect(on.shadow).toMatchObject({ drawCalls: 1, instances: 100, recordWrites: 1 })
    expect(on.sectionCap).toMatchObject({ drawCalls: 1, instances: 100 })
    expect(on.transparent).toMatchObject({ drawCalls: 1, instances: 2, perEntity: false })
    expect(on.edges.instances).toBe(0)
    renderer.selectMesh(37)
    const selected = renderer.instanceStepMetrics
    expect(selected.edges).toMatchObject({ drawCalls: 1, instances: 1 })
    // Selection changes one record of the shadow runs: one slice write, not a full upload.
    expect(selected.shadow.recordWrites).toBe(1)
    renderer.setDisplayMode('edges')
    expect(renderer.instanceStepMetrics.edges).toMatchObject({ drawCalls: 2, instances: 102 })
    renderer.setInstancingEnabled(false)
    expect(renderer.instanceStepMetrics.shadow.drawCalls).toBe(0)
  })

  it('reports per-entity fallback for transparent meshes with a material', async () => {
    const renderer = new RecordingRenderer()
    await renderer.init({ clientHeight: 100 } as HTMLCanvasElement)
    renderer.setInstancingEnabled(true)
    const tinted = { ...instance(0, 'asset:tint'), color: [1, 1, 1, 0.5], material: {} } as unknown as MeshData
    renderer.setMeshes([...hundred(), tinted])
    expect(renderer.instanceStepMetrics.transparent).toMatchObject({ drawCalls: 0, perEntity: true })
  })
})
