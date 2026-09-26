import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { geometryAssetId } from '../src/core/scene'
import { WebGPURenderer } from '../src/services/webgpuRenderer'
import * as sceneBounds from '../src/services/sceneAabbIndex'
import * as meshAnalysis from '../src/services/geometry/meshAnalysis'
import {callGeometryRust} from '../src/services/geometry/kernel'

class FakeBuffer {
  destroyCalls = 0
  readonly contents: Uint8Array
  readonly written: Uint8Array
  constructor(readonly size: number, readonly usage: number) {
    this.contents = new Uint8Array(size)
    this.written = new Uint8Array(size)
  }
  destroy() { this.destroyCalls++ }
}

class FakeDevice {
  readonly buffers: FakeBuffer[] = []
  readonly writes: Array<{ buffer: FakeBuffer; offset: number; bytes: number }> = []
  readonly limits = { maxBufferSize: 1 << 28 }
  failCreateAt: number | null = null
  failBindGroupAt: number | null = null
  bindGroupCalls = 0
  readonly queue = {
    writeBuffer: (buffer: FakeBuffer, offset: number, data: ArrayBuffer | ArrayBufferView) => {
      this.writes.push({ buffer, offset, bytes: data.byteLength })
      // WebGPU snapshots the supplied bytes when writeBuffer is called. Keep
      // that behavior when the renderer reuses one mutable uniform scratch.
      const bytes = ArrayBuffer.isView(data)
        ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
        : new Uint8Array(data)
      buffer.contents.set(bytes, offset)
      buffer.written.fill(1, offset, offset + bytes.length)
    },
  }
  createBuffer(descriptor: { size: number; usage: number }) {
    if (this.failCreateAt === this.buffers.length + 1) throw new Error('injected allocation failure')
    const buffer = new FakeBuffer(descriptor.size, descriptor.usage)
    this.buffers.push(buffer)
    return buffer
  }
  createBindGroup() {
    this.bindGroupCalls++
    if (this.failBindGroupAt === this.bindGroupCalls) throw new Error('injected bind-group failure')
    return {}
  }
}

function fixture(offset = 0): MeshData {
  const vertices = new Float32Array([
    offset, 0, 0, 0, 0, 1,
    offset + 1, 0, 0, 0, 0, 1,
    offset, 1, 0, 0, 0, 1,
  ])
  const indices = new Uint32Array([0, 1, 2])
  return {
    entityId: `entity:${offset}`,
    geometryAssetId: geometryAssetId(vertices, indices),
    vertices, indices,
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    faceIds: new Uint32Array([1]),
    bvh: { version: 1, vertexStride: 6, leafSize: 8, nodeCount: 1, bounds: new Float32Array(6), nodes: new Uint32Array(2), triangles: new Uint32Array([0]) },
    color: [1, 0, 0, 1], provenance: [],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function harness() {
  const renderer = new WebGPURenderer()
  const device = new FakeDevice()
  const internal = renderer as unknown as {
    dev: GPUDevice
    initialized: boolean
    dead: boolean
    lost: boolean
    objBGL: GPUBindGroupLayout
    initialFitDone: boolean
    meshes: Array<{ vb: FakeBuffer; ib: FakeBuffer; ub: FakeBuffer; edgeIB: FakeBuffer | null; edgeIC: number }>
    scheduleEdgeBufferWarmup(): void
    requestRender(): void
  }
  internal.dev = device as unknown as GPUDevice
  internal.initialized = true
  internal.dead = false
  internal.lost = false
  internal.objBGL = {} as GPUBindGroupLayout
  internal.initialFitDone = true
  internal.scheduleEdgeBufferWarmup = vi.fn()
  internal.requestRender = vi.fn()
  return { renderer, device, internal }
}

describe('WebGPURenderer retained geometry resources', () => {
  it('uses one native picking snapshot for repeated hits and frees it on teardown',()=>{
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const upload=vi.spyOn(meshAnalysis,'createPickingSnapshotInKernel')
    const {renderer,internal}=harness()
    const picking=internal as unknown as {rayForClientPoint(x:number,y:number):unknown;findHitCandidates(x:number,y:number,n:number):Array<{value:{triangleIndex:number}}>}
    picking.rayForClientPoint=()=>({origin:[0.2,0.2,2],direction:[0,0,-1]})
    try{
      renderer.setMeshes([fixture()])
      for(let i=0;i<20;i++)expect(picking.findHitCandidates(0,0,1)[0]?.value.triangleIndex).toBe(0)
      expect(upload).toHaveBeenCalledTimes(1)
      renderer.setMeshes([fixture()])
      expect(picking.findHitCandidates(0,0,1)).toHaveLength(1)
      expect(upload).toHaveBeenCalledTimes(1)
      const handle=upload.mock.results[0].value
      renderer.destroy()
      expect(()=>callGeometryRust('mesh_picking',{action:'query',handle,origin:[0,0,2],direction:[0,0,-1],excludedTriangles:[]})).toThrow()
    }finally{renderer.destroy();upload.mockRestore()}
  })
  afterEach(() => vi.unstubAllGlobals())

  it('restores native face selection after mesh publication and rejects changed geometry',()=>{
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const {renderer}=harness()
    const previous=fixture()
    previous.nativeGeometry=createNativeGeometryArtifact('patch','surface',{controls:[1]}, {})
    previous.faceIdsAuthoritative=true
    const next=fixture();next.nativeGeometry=previous.nativeGeometry;next.faceIdsAuthoritative=true
    renderer.setMeshes([next]);renderer.setSelectionMode('face')
    const hit={meshIndex:0,triangleIndex:0,faceId:1,point:[0,0,0] as [number,number,number],normal:[0,0,1] as [number,number,number],barycentric:[1,0,0] as [number,number,number],source:null,backside:false}
    expect(renderer.restoreNativeFaceSelection(previous,hit,0)).toBe(true)
    expect(renderer.currentHit?.faceId).toBe(1)
    next.nativeGeometry=createNativeGeometryArtifact('patch','surface',{controls:[2]}, {})
    renderer.setMeshes([next])
    expect(renderer.restoreNativeFaceSelection(previous,hit,0)).toBe(false)
    expect(renderer.currentHit).toBeNull()
  })

  it.each([
    ['shaded', 0.625, 0],
    ['edges', 0.625, 0.7],
    ['xray', 0.24, 0],
  ] as const)('writes complete model, normal, color and %s style uniforms', (mode, alpha, edge) => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, internal } = harness()
    renderer.setDisplayMode(mode)
    const mesh = fixture()
    // A rotated, mirrored, nonuniformly scaled object with translation catches
    // both matrix-layout mistakes and incorrectly using the model for normals.
    mesh.transform = new Float32Array([
      0, -3, 0, 5,
      -2, 0, 0, -7,
      0, 0, 4, 11,
      0, 0, 0, 1,
    ])
    mesh.color = [0.125, 0.375, 0.75, 0.625]
    renderer.setMeshes([mesh])
    const uniform = internal.meshes[0].ub
    const expected = [
      // Column-major model matrix.
      0, -2, 0, 0, -3, 0, 0, 0, 0, 0, 4, 0, 5, -7, 11, 1,
      // Column-major inverse-transpose, including the homogeneous row.
      0, -0.5, 0, -3.5, -1 / 3, 0, 0, 5 / 3, 0, 0, 0.25, -2.75, 0, 0, 0, 1,
      0.125, 0.375, 0.75, 0.625,
      alpha, 0, edge, 0,
      // Morph at rest, then material defaults: white base color, metallic 0,
      // black emissive, roughness 0.7, material id 0, padding.
      1, 0, 0, 0,
      1, 1, 1, 0,
      0, 0, 0, 0.7,
      0, 0, 0, 0,
    ]
    expect(uniform.size).toBe(224)
    expect(uniform.written.every(byte => byte === 1)).toBe(true)
    const values = new Float32Array(uniform.contents.buffer)
    expected.forEach((value, index) => expect(values[index]).toBeCloseTo(value, 6))

    const presentationBefore = uniform.contents.slice(0, 144)
    renderer.setDisplayMode(mode === 'xray' ? 'shaded' : 'xray')
    expect(uniform.contents.slice(0, 144)).toEqual(presentationBefore)
    expect(values[36]).toBeCloseTo(mode === 'xray' ? 0.625 : 0.24, 6)
  })

  it('reuses verified vertex/index buffers and only replaces entity uniforms', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    const first = fixture()
    renderer.setMeshes([first])
    expect(renderer.sceneUploadMetrics).toEqual({ geometryUploadBytes: 84, geometryBuffersCreated: 2, reusedEntities: 0 })
    const initial = internal.meshes[0]
    expect(device.buffers).toHaveLength(3)

    const replacement = fixture()
    renderer.setMeshes([replacement])
    expect(renderer.sceneUploadMetrics).toEqual({ geometryUploadBytes: 0, geometryBuffersCreated: 0, reusedEntities: 1 })
    expect(device.buffers).toHaveLength(4)
    expect(internal.meshes[0].vb).toBe(initial.vb)
    expect(internal.meshes[0].ib).toBe(initial.ib)
    expect(initial.vb.destroyCalls).toBe(0)
    expect(initial.ib.destroyCalls).toBe(0)
    expect(initial.ub.destroyCalls).toBe(1)
  })

  it('keeps colliding asset identifiers separate while reusing verified staged views', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, internal } = harness()
    const first = fixture()
    renderer.setMeshes([first])
    const originalVertexBuffer = internal.meshes[0].vb
    const replacement = fixture()
    const collision = fixture(10)
    collision.geometryAssetId = first.geometryAssetId
    renderer.setMeshes([replacement, collision, { ...replacement, entityId: 'entity:third' }])
    expect(internal.meshes[0].vb).toBe(originalVertexBuffer)
    expect(internal.meshes[1].vb).not.toBe(originalVertexBuffer)
    expect(internal.meshes[2].vb).toBe(originalVertexBuffer)
    expect(renderer.sceneUploadMetrics).toEqual({ geometryUploadBytes: 84, geometryBuffersCreated: 2, reusedEntities: 2 })
    const changedIndex = { ...replacement, indices: new Uint32Array([0, 2, 1]) }
    renderer.setMeshes([replacement, changedIndex])
    expect(internal.meshes[1].ib).not.toBe(internal.meshes[0].ib)
    expect(internal.meshes[1].vb).not.toBe(originalVertexBuffer)
  })

  it('uploads one edge buffer for three shared-asset instances, including equal edge-array copies', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    renderer.setDisplayMode('edges')
    const mesh = fixture()
    renderer.setMeshes([
      { ...mesh, entityId: 'entity:first' },
      { ...mesh, entityId: 'entity:second', edgeIndices: mesh.edgeIndices.slice() },
      { ...mesh, entityId: 'entity:third' },
    ])

    const first = internal.meshes[0]
    expect(first.edgeIB).not.toBeNull()
    for (const instance of internal.meshes) {
      expect(instance.vb).toBe(first.vb)
      expect(instance.ib).toBe(first.ib)
      expect(instance.edgeIB).toBe(first.edgeIB)
      expect(instance.edgeIC).toBe(mesh.edgeIndices.length)
    }
    const edge = first.edgeIB!
    expect(new Uint32Array(edge.contents.buffer)).toEqual(mesh.edgeIndices)
    expect(device.writes.filter(write => write.buffer === edge)).toHaveLength(1)
    // One vertex buffer, one triangle index buffer, three entity uniforms and
    // one shared edge buffer: no hidden per-instance duplicate edge upload.
    expect(device.buffers).toHaveLength(6)
  })

  it('keeps distinct edge topology separate even when vertex and triangle buffers are shared', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    renderer.setDisplayMode('edges')
    const mesh = fixture()
    const firstEdges = new Uint32Array([0, 1, 1, 2])
    const otherEdges = new Uint32Array([0, 2, 2, 1])
    renderer.setMeshes([
      { ...mesh, entityId: 'entity:first', edgeIndices: firstEdges },
      { ...mesh, entityId: 'entity:other-topology', edgeIndices: otherEdges },
      { ...mesh, entityId: 'entity:copy', edgeIndices: firstEdges.slice() },
    ])

    const [first, other, copy] = internal.meshes
    expect(first.vb).toBe(other.vb)
    expect(first.ib).toBe(other.ib)
    expect(first.edgeIB).not.toBeNull()
    expect(other.edgeIB).not.toBeNull()
    expect(first.edgeIB).not.toBe(other.edgeIB)
    expect(copy.edgeIB).toBe(first.edgeIB)
    expect(new Uint32Array(first.edgeIB!.contents.buffer)).toEqual(firstEdges)
    expect(new Uint32Array(other.edgeIB!.contents.buffer)).toEqual(otherEdges)
    expect(device.buffers).toHaveLength(7)
  })

  it('releases shared edges once and recreates them after topology disappears from the publication', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    renderer.setDisplayMode('edges')
    const mesh = fixture()
    const instances = Array.from({ length: 3 }, (_, index) => ({ ...mesh, entityId: `entity:instance-${index}` as const }))
    renderer.setMeshes(instances)
    const originalVertex = internal.meshes[0].vb
    const originalEdges = internal.meshes[0].edgeIB!
    expect(originalEdges).not.toBeNull()

    renderer.setMeshes(instances.map(instance => ({ ...instance, edgeIndices: instance.edgeIndices.slice() })))
    expect(internal.meshes.every(instance => instance.edgeIB === originalEdges)).toBe(true)
    expect(originalEdges.destroyCalls).toBe(0)

    renderer.setMeshes(instances.map(instance => ({ ...instance, edgeIndices: new Uint32Array() })))
    expect(internal.meshes.every(instance => instance.edgeIB === null && instance.edgeIC === 0)).toBe(true)
    expect(originalEdges.destroyCalls).toBe(1)
    expect(internal.meshes[0].vb).toBe(originalVertex)
    expect(originalVertex.destroyCalls).toBe(0)

    // The VB/IB pair remains live, so a stale cache keyed only by geometry
    // would incorrectly resurrect the edge buffer destroyed just above.
    renderer.setMeshes(instances)
    const replacementEdges = internal.meshes[0].edgeIB!
    expect(replacementEdges).not.toBeNull()
    expect(replacementEdges).not.toBe(originalEdges)
    expect(replacementEdges.destroyCalls).toBe(0)
    expect(internal.meshes.every(instance => instance.edgeIB === replacementEdges)).toBe(true)
    expect(new Uint32Array(replacementEdges.contents.buffer)).toEqual(mesh.edgeIndices)

    renderer.destroy()
    renderer.destroy()
    expect(originalEdges.destroyCalls).toBe(1)
    expect(replacementEdges.destroyCalls).toBe(1)
    for (const buffer of device.buffers) expect(buffer.destroyCalls).toBe(1)
  })

  it('rolls back a failed staged entity allocation without destroying live geometry', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    renderer.setMeshes([fixture()])
    const live = internal.meshes[0]
    device.failCreateAt = device.buffers.length + 1

    expect(() => renderer.setMeshes([fixture()])).toThrow('injected allocation failure')
    expect(internal.meshes[0]).toBe(live)
    expect(live.vb.destroyCalls).toBe(0)
    expect(live.ib.destroyCalls).toBe(0)
    expect(live.ub.destroyCalls).toBe(0)
  })

  it('preserves published geometry and uniforms when a later staged bind group fails', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, device, internal } = harness()
    renderer.setMeshes([fixture()])
    const live = internal.meshes[0]
    const publishedUniform = live.ub.contents.slice()
    const metrics = renderer.sceneUploadMetrics
    const stagedStart = device.buffers.length
    device.failBindGroupAt = device.bindGroupCalls + 2
    const replacement = fixture()
    replacement.color = [0, 0.5, 1, 0.25]
    replacement.transform[3] = 12

    expect(() => renderer.setMeshes([replacement, fixture(2)])).toThrow('injected bind-group failure')
    expect(internal.meshes).toEqual([live])
    expect(renderer.sceneUploadMetrics).toEqual(metrics)
    expect(live.ub.contents).toEqual(publishedUniform)
    for (const buffer of [live.vb, live.ib, live.ub]) expect(buffer.destroyCalls).toBe(0)
    for (const buffer of device.buffers.slice(stagedStart)) expect(buffer.destroyCalls).toBe(1)
  })
})


describe('parameter geometry animation', () => {
  afterEach(() => vi.unstubAllGlobals())
  it('replaces rebuilt vertices without interpolating unrelated CSG vertices', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, internal } = harness()
    const state = renderer as unknown as { meshes: Array<{ morph?: unknown }> }
    renderer.setMeshes([fixture(0)])
    const next = fixture(10)
    renderer.setMeshes([next], { animate: true })
    expect(new Float32Array(internal.meshes[0].vb.contents.buffer)).toEqual(next.vertices)
    expect(state.meshes[0].morph).toBeUndefined()
    expect(new Float32Array(internal.meshes[0].ub.contents.buffer)[36]).toBe(1)
  })
  it('retains exact destination geometry for topology changes and honors reduced motion', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, internal } = harness()
    const first = { ...fixture(), entityId: 'entity:part' as const }
    renderer.setMeshes([first])
    const changed = { ...fixture(10), entityId: first.entityId, indices: new Uint32Array([2, 1, 0]) }
    renderer.setMeshes([changed], { animate: true })
    expect(new Float32Array(internal.meshes[0].vb.contents.buffer)).toEqual(changed.vertices)
    vi.stubGlobal('matchMedia', () => ({ matches: true }))
    const reduced = { ...fixture(20), entityId: first.entityId, indices: changed.indices }
    renderer.setMeshes([reduced], { animate: true })
    expect(new Float32Array(internal.meshes[0].vb.contents.buffer)).toEqual(reduced.vertices)
  })

  it('drives vertex morphs from a source buffer and blend uniform without rewriting vertices', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
    const { renderer, internal } = harness()
    const state = renderer as unknown as {
      meshes: Array<{
        morph?: { from: Float32Array; started: number }
        morphVB: { contents: Uint8Array } | null
        morphSlot: unknown
        vb: { contents: Uint8Array }
        ub: { contents: Uint8Array }
      }>
      advanceGeometryAnimation(now: number): boolean
    }
    const first = { ...fixture(), entityId: 'entity:part' as const }
    renderer.setMeshes([first])
    const moved = { ...fixture(), entityId: first.entityId }
    moved.transform = new Float32Array(first.transform)
    moved.transform[3] = 20
    renderer.setMeshes([moved], { animate: true })

    const mesh = state.meshes[0]
    expect(mesh.morph).toBeDefined()
    // The destination stays in the vertex buffer; the morph source (positions
    // only, stride 3) lives in a dedicated slot-1 buffer.
    expect(new Float32Array(mesh.vb.contents.buffer)).toEqual(moved.vertices)
    expect(mesh.morphVB).not.toBeNull()
    expect(new Float32Array(mesh.morphVB!.contents.buffer)).toEqual(new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]))
    expect(mesh.morphSlot).toBe(mesh.morphVB)

    const start = mesh.morph!.started
    expect(state.advanceGeometryAnimation(start + 90)).toBe(true)
    // Only the blend weight uniform changes per frame; vertex bytes are untouched.
    expect(new Float32Array(mesh.vb.contents.buffer)).toEqual(moved.vertices)
    expect(new Float32Array(mesh.ub.contents.buffer)[40]).toBeCloseTo(0.5, 6)

    expect(state.advanceGeometryAnimation(start + 1000)).toBe(false)
    expect(state.meshes[0].morph).toBeUndefined()
    expect(state.meshes[0].morphSlot).toBeNull()
    expect(new Float32Array(state.meshes[0].ub.contents.buffer)[40]).toBe(1)
  })
})


it('replaces changed topology opaquely and releases retired buffers', () => {
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
  try {
    const { renderer, internal } = harness()
    const state = renderer as unknown as {
      geometryFade: { started: number } | null
      geometryGhosts: unknown[]
      advanceGeometryAnimation(now: number, finish?: boolean): boolean
      updateHoverAt(x: number, y: number): void
    }
    renderer.setMeshes([fixture()])
    const old = internal.meshes[0].vb
    renderer.setMeshes([{ ...fixture(10), indices: new Uint32Array([2,1,0]) }], { animate: true })
    expect(state.geometryFade).toBeNull()
    expect(old.destroyCalls).toBe(1)
    expect(state.geometryGhosts).toHaveLength(0)
    expect(new Float32Array(internal.meshes[0].ub.contents.buffer)[36]).toBe(1)
    renderer.destroy()
    expect(old.destroyCalls).toBe(1)
  } finally { vi.unstubAllGlobals() }
})

it('animates transform-only changes and continues through quality publications', () => {
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
  try {
    const { renderer, internal } = harness()
    const state = renderer as unknown as {
      meshes: Array<{ morph?: { started: number } }>
      advanceGeometryAnimation(now: number): boolean
    }
    const first = fixture()
    renderer.setMeshes([first])
    const moved = { ...fixture(), transform: new Float32Array(first.transform) }
    moved.transform[3] = 20
    renderer.setMeshes([moved], { animate: true })
    const start = state.meshes[0].morph!.started
    state.advanceGeometryAnimation(start + 90)
    expect(new Float32Array(internal.meshes[0].ub.contents.buffer)[12]).toBeCloseTo(10)
    renderer.setMeshes([moved])
    expect(state.meshes[0].morph?.started).toBe(start)
    state.advanceGeometryAnimation(start + 135)
    expect(new Float32Array(internal.meshes[0].ub.contents.buffer)[12]).toBeCloseTo(16.875)
    state.advanceGeometryAnimation(start + 1000)
    expect(new Float32Array(internal.meshes[0].ub.contents.buffer)[12]).toBe(20)
    expect(moved.transform[3]).toBe(20)
  } finally { vi.unstubAllGlobals() }
})

it('keeps the published scene intact if native broadphase admission fails', () => {
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
  const { renderer, internal, device } = harness()
  try {
    renderer.setMeshes([fixture()])
    const previous = internal.meshes[0]
    const stagedStart = device.buffers.length
    const admission = vi.spyOn(sceneBounds, 'buildSceneAabbIndex').mockImplementationOnce(() => { throw new Error('native admission refused') })
    try { expect(() => renderer.setMeshes([fixture(2)])).toThrow('native admission refused') }
    finally { admission.mockRestore() }
    expect(internal.meshes).toEqual([previous])
    expect(previous.vb.destroyCalls).toBe(0)
    for (const buffer of device.buffers.slice(stagedStart)) expect(buffer.destroyCalls).toBe(1)
    // More than the registry slot limit: each publication must retire its index.
    for (let n = 0; n < 80; n++) renderer.setMeshes([fixture(n)])
  } finally { renderer.destroy(); vi.unstubAllGlobals() }
})

it('snaps point selection to a transformed corner through Rust', () => {
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
  const { renderer, internal } = harness()
  const picking = internal as unknown as {
    rayForClientPoint(x: number, y: number): unknown
    findHitCandidates(x: number, y: number, n: number): Array<{ value: { point: number[] } }>
  }
  picking.rayForClientPoint = () => ({ origin: [5.2, 0.2, 2], direction: [0, 0, -1] })
  try {
    const mesh = fixture()
    mesh.transform[3] = 5
    renderer.setMeshes([mesh])
    renderer.setSelectionMode('point')
    expect(picking.findHitCandidates(0, 0, 1)[0]?.value.point).toEqual([5, 0, 0])
  } finally { renderer.destroy(); vi.unstubAllGlobals() }
})

it('refuses singular geometry transforms before replacing the published scene', () => {
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, UNIFORM: 4, COPY_DST: 8 })
  const { renderer, internal, device } = harness()
  try {
    renderer.setMeshes([fixture()])
    const live = internal.meshes[0]
    const count = device.buffers.length
    const invalid = fixture(2)
    invalid.transform.fill(0)
    expect(() => renderer.setMeshes([invalid])).toThrow('Singular')
    expect(internal.meshes).toEqual([live])
    expect(live.vb.destroyCalls).toBe(0)
    expect(device.buffers).toHaveLength(count)
  } finally { renderer.destroy(); vi.unstubAllGlobals() }
})
