import { describe, expect, it } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import {createSelectionSurfacePublisher} from '../src/services/selectionSurfacePublisher'
import {withSelectionSurfaces} from '../src/services/meshSurfaceGroups'
import {
  assertGeometryScene,
  geometrySceneFromMeshes,
  geometrySceneTransferables,
  meshesFromGeometryScene,
} from '../src/core/scene'

function fixture(entityId: `entity:${string}`, transformX: number): MeshData {
  const transform = new Float32Array(16)
  transform[0] = transform[5] = transform[10] = transform[15] = 1
  transform[12] = transformX
  return {
    entityId,
    vertices: new Float32Array([0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1]),
    indices: new Uint32Array([0, 1, 2]),
    edgeIndices: new Uint32Array([0, 1, 1, 2, 2, 0]),
    transform,
    faceIds: new Uint32Array([1]),
    bvh: { version: 1, vertexStride: 6, leafSize: 8, nodeCount: 1, bounds: new Float32Array(6), nodes: new Uint32Array(2), triangles: new Uint32Array([0]) },
    color: [1, 0, 0, 1], provenance: [],
    topology: { boundary: 3, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

describe('entity/geometry asset split', () => {
  it('retains inferred selection metadata through scene publication', () => {
    const mesh = {...fixture('entity:selection', 0), faceIdsInferred: true}
    const restored = meshesFromGeometryScene(geometrySceneFromMeshes([mesh]))[0]
    expect(restored.faceIdsInferred).toBe(true)
    expect(withSelectionSurfaces(restored)).toBe(restored)
  })

  it('caches groups without transferring or mutating the retained cache entry', () => {
    let calls = 0
    const publish = createSelectionSurfacePublisher(() => { calls++; return new Uint32Array([7]) })
    const mesh = {...fixture('entity:selection', 0), geometryAssetId: 'mesh:fixture'}
    const first = publish(mesh)
    const sent = structuredClone(first.faceIds, {transfer: [first.faceIds.buffer]})
    expect(sent[0]).toBe(7)
    const second = publish(mesh)
    expect(second.faceIds[0]).toBe(7)
    second.faceIds[0] = 99
    expect(publish(mesh).faceIds[0]).toBe(7)
    expect(calls).toBe(1)
    expect(publish({...mesh, faceIdsAuthoritative: true}).faceIds).toBe(mesh.faceIds)
  })
  it('deduplicates exact tessellation while preserving independent entity state', () => {
    const first = fixture('entity:first', 0)
    const second = fixture('entity:second', 10)
    second.vertices = first.vertices
    second.indices = first.indices
    const scene = geometrySceneFromMeshes([first, second])

    expect(scene.assets).toHaveLength(1)
    expect(scene.entities).toHaveLength(2)
    expect(scene.entities[0].geometryAssetId).toBe(scene.entities[1].geometryAssetId)
    expect(scene.entities.map(entity => entity.transform[12])).toEqual([0, 10])
    first.geometryAssetId = scene.assets[0].id
    second.geometryAssetId = scene.assets[0].id
    expect(meshesFromGeometryScene(scene)).toEqual([first, second])
  })

  it('allows lazy inspection artifacts and fails closed for eager legacy consumers', () => {
    const full = geometrySceneFromMeshes([fixture('entity:first', 0)])
    const lazy = { ...full, entities: [{ ...full.entities[0], inspection: { state: 'absent' as const, version: 1 as const } }] }
    expect(() => meshesFromGeometryScene(lazy)).toThrow('missing eager geometry or inspection artifacts')
  })

  it('transfers shared geometry buffers only once', () => {
    const first = fixture('entity:first', 0)
    const second = fixture('entity:second', 10)
    second.vertices = first.vertices
    second.indices = first.indices
    const scene = geometrySceneFromMeshes([first, second])
    const buffers = geometrySceneTransferables(scene)
    expect(buffers.filter(buffer => buffer === first.vertices.buffer)).toHaveLength(1)
    expect(new Set(buffers).size).toBe(buffers.length)
  })

  it('rejects duplicate entities, dangling assets and partial backing-buffer views', () => {
    const scene = geometrySceneFromMeshes([fixture('entity:first', 0)])
    expect(() => assertGeometryScene({ ...scene, entities: [scene.entities[0], scene.entities[0]] })).toThrow('Duplicate scene entity')
    expect(() => assertGeometryScene({
      ...scene,
      entities: [{ ...scene.entities[0], geometryAssetId: 'asset:missing' }],
    })).toThrow('missing geometry asset')

    const backing = new Float32Array(24)
    const partial = new Float32Array(backing.buffer, 0, 18)
    const partialScene = geometrySceneFromMeshes([{ ...fixture('entity:partial', 0), vertices: partial }])
    expect(() => geometrySceneTransferables(partialScene)).toThrow('complete backing buffer')
  })
})
