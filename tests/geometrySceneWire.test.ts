import { describe, expect, it } from 'vitest'
import { planGeometrySourceExecution } from '../src/core/geometryExecution'
import type { MeshData } from '../src/core/mesh'
import { geometrySceneFromMeshes, geometrySceneTransferables, type GeometryScene } from '../src/core/scene'
import {
  GEOMETRY_WORKER_PROTOCOL_VERSION,
  isGeometryWorkerEvent,
  materializeGeometryWorkerEvent,
  type GeometryBuildSuccess,
  type GeometryBuildSuccessWire,
} from '../src/kernel/geometryWorkerProtocol'

function triangle(entityId: string): MeshData {
  return {
    entityId,
    vertices: new Float32Array([0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1]),
    indices: new Uint32Array([0, 1, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    faceIds: new Uint32Array([0]),
    color: [1, 0.5, 0.25, 1],
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    bvh: {
      version: 1,
      vertexStride: 6,
      leafSize: 8,
      nodeCount: 1,
      bounds: new Float32Array([0, 0, 0, 1, 1, 0]),
      nodes: new Uint32Array([0, 0x80000001]),
      triangles: new Uint32Array([0]),
    },
    provenance: [{ triangleStart: 0, triangleEnd: 1, backside: false, source: null }],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

/** Two entities sharing one tessellation (equal bytes in separate arrays). */
function instancedScene(): GeometryScene {
  return geometrySceneFromMeshes([triangle('entity:a'), triangle('entity:b')])
}

function success(scene: unknown): GeometryBuildSuccessWire {
  return {
    protocolVersion: GEOMETRY_WORKER_PROTOCOL_VERSION,
    documentRevision: 1,
    jobId: 1,
    quality: 'preview',
    sourceSha256: '0'.repeat(64),
    status: 'succeeded',
    phase: 'complete',
    execution: {
      ...planGeometrySourceExecution('cube(1);', { quality: 'preview', purpose: 'preview' }),
      evidence: 'runtime',
    },
    scene: scene as GeometryScene,
    warnings: [],
    volume: 0,
    surfaceArea: 0,
    reduced: false,
    timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    durationMs: 1,
  }
}

describe('GeometryScene in the geometry Worker protocol', () => {
  it('accepts shared assets referenced by several entities and materializes zero-copy meshes', () => {
    const scene = instancedScene()
    expect(scene.assets).toHaveLength(1)
    const event = success(scene)
    expect(isGeometryWorkerEvent(event)).toBe(true)
    const host = materializeGeometryWorkerEvent(event) as GeometryBuildSuccess
    expect(host).not.toHaveProperty('scene')
    expect(host.meshes.map(mesh => mesh.entityId)).toEqual(['entity:a', 'entity:b'])
    expect(host.meshes[0].vertices).toBe(scene.assets[0].vertices)
    expect(host.meshes[1].vertices).toBe(scene.assets[0].vertices)
    expect(host.meshes[0].geometryAssetId).toBe(scene.assets[0].id)
  })

  it('transfers every shared asset buffer exactly once', () => {
    const scene = instancedScene()
    const transfer = geometrySceneTransferables(scene)
    expect(new Set(transfer).size).toBe(transfer.length)
    expect(transfer.filter(buffer => buffer === scene.assets[0].vertices.buffer)).toHaveLength(1)
    // One asset (2 buffers) plus, per entity, transform + edges + face ids + 3 BVH arrays.
    expect(transfer).toHaveLength(2 + 2 * 6)
    const cloned = structuredClone(success(scene), { transfer })
    expect(scene.assets[0].vertices.byteLength).toBe(0)
    expect(isGeometryWorkerEvent(cloned)).toBe(true)
  })

  it('rejects the legacy eager mesh payload and inexact scene records', () => {
    const { scene: _scene, ...withoutScene } = success(instancedScene())
    expect(isGeometryWorkerEvent({ ...withoutScene, meshes: [triangle('entity:a')] })).toBe(false)
    expect(isGeometryWorkerEvent(success({ ...instancedScene(), extra: true }))).toBe(false)
    expect(isGeometryWorkerEvent(success({ ...instancedScene(), version: 2 }))).toBe(false)
    const scene = instancedScene()
    expect(isGeometryWorkerEvent(success({ ...scene, assets: [{ ...scene.assets[0], extra: 1 }] }))).toBe(false)
    expect(isGeometryWorkerEvent(success({ ...scene, entities: [{ ...scene.entities[0], extra: 1 }] }))).toBe(false)
  })

  it('rejects dangling, duplicate and aliased assets and non-ready inspection artifacts', () => {
    const scene = instancedScene()
    const [asset] = scene.assets
    const [entity] = scene.entities
    expect(
      isGeometryWorkerEvent(success({ ...scene, entities: [{ ...entity, geometryAssetId: 'asset:missing' }] })),
    ).toBe(false)
    expect(isGeometryWorkerEvent(success({ ...scene, assets: [asset, asset] }))).toBe(false)
    const aliased = { ...asset, id: 'asset:alias' as const }
    expect(isGeometryWorkerEvent(success({ ...scene, assets: [asset, aliased] }))).toBe(false)
    expect(isGeometryWorkerEvent(success({ ...scene, entities: [entity, { ...entity }] }))).toBe(false)
    expect(
      isGeometryWorkerEvent(
        success({ ...scene, entities: [{ ...entity, inspection: { state: 'absent', version: 1 } }] }),
      ),
    ).toBe(false)
    const outOfRange = { ...asset, indices: new Uint32Array([0, 1, 3]) }
    expect(isGeometryWorkerEvent(success({ ...scene, assets: [outOfRange] }))).toBe(false)
  })

  it('still rejects buffers shared between an asset and entity-owned artifacts', () => {
    const scene = instancedScene()
    const [entity] = scene.entities
    if (entity.inspection?.state !== 'ready') throw new Error('fixture must be eager')
    const edgeAlias = new Uint32Array(scene.assets[0].indices.buffer)
    const inspection = { ...entity.inspection, edgeIndices: edgeAlias }
    expect(isGeometryWorkerEvent(success({ ...scene, entities: [{ ...entity, inspection }, scene.entities[1]] }))).toBe(
      false,
    )
  })
})
