import {isNativeGeometryArtifact,type NativeGeometryArtifact} from './nativeGeometry'
import type {
  MeshBvh,
  MeshData,
  MeshProvenanceRun,
  MeshTopologyDiagnostics,
  SceneEntityId,
} from './mesh'

export type GeometryAssetId = `asset:${string}`

/** Immutable tessellation shared by one or more positioned scene entities. */
export interface GeometryAsset {
  readonly version: 1
  readonly id: GeometryAssetId
  readonly vertices: Float32Array
  readonly indices: Uint32Array
}

/** Optional inspection data can be requested/transported independently later. */
export interface ReadyGeometryInspectionArtifacts {
  readonly state: 'ready'
  readonly version: 1
  readonly bvh: MeshBvh
  readonly edgeIndices: Uint32Array
  readonly faceIds: Uint32Array
  readonly faceIdsAuthoritative?: boolean
  readonly faceIdsInferred?: boolean
  readonly provenance: readonly MeshProvenanceRun[]
  readonly topology: MeshTopologyDiagnostics
}

export type GeometryInspectionArtifacts = ReadyGeometryInspectionArtifacts
  | { readonly state: 'absent'; readonly version: 1 }
  | { readonly state: 'pending'; readonly version: 1; readonly requestId: string }
  | { readonly state: 'failed'; readonly version: 1; readonly errorCode: string }

export interface SceneEntity {
  readonly nativeGeometry?: NativeGeometryArtifact
  readonly id: SceneEntityId
  readonly geometryAssetId: GeometryAssetId
  readonly color: readonly [number, number, number, number]
  readonly transform: Float32Array
  readonly inspection?: GeometryInspectionArtifacts
}

export interface GeometryScene {
  readonly version: 1
  readonly assets: readonly GeometryAsset[]
  readonly entities: readonly SceneEntity[]
}

function hashBytes(hash: number, bytes: Uint8Array) {
  let next = hash >>> 0
  // Consume 4 bytes per iteration through a word view; the little-endian byte
  // order and the FNV-1a sequence are unchanged, so the id is identical.
  let index = 0
  if (bytes.byteOffset % 4 === 0) {
    const full = bytes.length - (bytes.length % 4)
    const words = new Uint32Array(bytes.buffer, bytes.byteOffset, full / 4)
    for (const word of words) {
      next ^= word & 0xff; next = Math.imul(next, 0x01000193) >>> 0
      next ^= (word >>> 8) & 0xff; next = Math.imul(next, 0x01000193) >>> 0
      next ^= (word >>> 16) & 0xff; next = Math.imul(next, 0x01000193) >>> 0
      next ^= word >>> 24; next = Math.imul(next, 0x01000193) >>> 0
    }
    index = full
  }
  for (; index < bytes.length; index++) {
    next ^= bytes[index]
    next = Math.imul(next, 0x01000193) >>> 0
  }
  return next
}

/** Deterministic exact-buffer identity; transforms/materials are entity state. */
export function geometryAssetId(vertices: Float32Array, indices: Uint32Array): GeometryAssetId {
  let first = hashBytes(0x811c9dc5, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
  first = hashBytes(first, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
  let second = hashBytes(0x9e3779b9, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
  second = hashBytes(second, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
  return `asset:v1:${first.toString(16).padStart(8, '0')}${second.toString(16).padStart(8, '0')}:${vertices.byteLength}:${indices.byteLength}`
}

function sameView<T extends Float32Array | Uint32Array>(left: T, right: T) {
  if (left.constructor !== right.constructor || left.length !== right.length) return false
  for (let index = 0; index < left.length; index++) if (left[index] !== right[index]) return false
  return true
}

/** Zero-copy compatibility adapter from the legacy eager mesh publication. */
export function geometrySceneFromMeshes(meshes: readonly MeshData[]): GeometryScene {
  const assets: GeometryAsset[] = []
  const assetsById = new Map<GeometryAssetId, GeometryAsset>()
  const entities: SceneEntity[] = []
  for (let index = 0; index < meshes.length; index++) {
    const mesh = meshes[index]
    let id = mesh.geometryAssetId ?? geometryAssetId(mesh.vertices, mesh.indices)
    const existing = assetsById.get(id)
    if (existing && (!sameView(existing.vertices, mesh.vertices) || !sameView(existing.indices, mesh.indices))) {
      id = `${id}:collision:${index}`
    }
    if (!assetsById.has(id)) {
      const asset = Object.freeze({ version: 1 as const, id, vertices: mesh.vertices, indices: mesh.indices })
      assetsById.set(id, asset)
      assets.push(asset)
    }
    entities.push(Object.freeze({
      id: mesh.entityId ?? `entity:legacy/${index}`,
      geometryAssetId: id,
      ...(mesh.nativeGeometry?{nativeGeometry:mesh.nativeGeometry}:{}),
      color: [...mesh.color] as [number, number, number, number],
      transform: mesh.transform,
      inspection: Object.freeze({
        state: 'ready' as const,
        version: 1 as const,
        bvh: mesh.bvh,
        edgeIndices: mesh.edgeIndices,
        faceIds: mesh.faceIds,
        ...(mesh.faceIdsInferred === undefined ? {} : {faceIdsInferred:mesh.faceIdsInferred}),
        ...(mesh.faceIdsAuthoritative === undefined ? {} : {faceIdsAuthoritative:mesh.faceIdsAuthoritative}),
        provenance: mesh.provenance,
        topology: mesh.topology,
      }),
    }))
  }
  return Object.freeze({ version: 1, assets: Object.freeze(assets), entities: Object.freeze(entities) })
}

/** Materialize the legacy eager view for consumers not yet migrated. */
export function meshesFromGeometryScene(scene: GeometryScene): MeshData[] {
  const assets = new Map(scene.assets.map(asset => [asset.id, asset]))
  return scene.entities.map(entity => {
    const asset = assets.get(entity.geometryAssetId)
    const artifacts = entity.inspection
    if (!asset || artifacts?.state !== 'ready') {
      throw new Error(`Scene entity ${entity.id} is missing eager geometry or inspection artifacts`)
    }
    return {
      entityId: entity.id,
      geometryAssetId: asset.id,
      ...(entity.nativeGeometry?{nativeGeometry:entity.nativeGeometry}:{}),
      vertices: asset.vertices,
      indices: asset.indices,
      bvh: artifacts.bvh,
      edgeIndices: artifacts.edgeIndices,
      color: [...entity.color],
      transform: entity.transform,
      faceIds: artifacts.faceIds,
      ...(artifacts.faceIdsInferred === undefined ? {} : {faceIdsInferred:artifacts.faceIdsInferred}),
      ...(artifacts.faceIdsAuthoritative === undefined ? {} : {faceIdsAuthoritative:artifacts.faceIdsAuthoritative}),
      provenance: [...artifacts.provenance],
      topology: artifacts.topology,
    }
  })
}

/** Transfer each shared asset and entity-owned artifact buffer exactly once. */
export function geometrySceneTransferables(scene: GeometryScene): ArrayBuffer[] {
  assertGeometryScene(scene)
  const buffers = new Set<ArrayBuffer>()
  const add = (view: ArrayBufferView | undefined) => {
    if (!view || !(view.buffer instanceof ArrayBuffer)) return
    if (view.byteOffset !== 0 || view.byteLength !== view.buffer.byteLength) {
      throw new Error('Geometry scene transfer views must own their complete backing buffer')
    }
    buffers.add(view.buffer)
  }
  for (const asset of scene.assets) { add(asset.vertices); add(asset.indices) }
  for (const entity of scene.entities) {
    add(entity.transform)
    const artifacts = entity.inspection
    if (artifacts?.state !== 'ready') continue
    add(artifacts.edgeIndices)
    add(artifacts.faceIds)
    add(artifacts.bvh.bounds)
    add(artifacts.bvh.nodes)
    add(artifacts.bvh.triangles)
  }
  return [...buffers]
}

export function assertGeometryScene(scene: GeometryScene): void {
  if (scene.version !== 1) throw new Error('Unsupported geometry scene version')
  const assets = new Map<GeometryAssetId, GeometryAsset>()
  for (const asset of scene.assets) {
    if (assets.has(asset.id)) throw new Error(`Duplicate geometry asset ${asset.id}`)
    if (asset.version !== 1 || asset.vertices.length % 6 !== 0) throw new Error(`Invalid geometry asset ${asset.id}`)
    const vertexCount = asset.vertices.length / 6
    if (asset.indices.some(index => index >= vertexCount)) throw new Error(`Geometry asset ${asset.id} has an out-of-range index`)
    assets.set(asset.id, asset)
  }
  const entityIds = new Set<SceneEntityId>()
  for (const entity of scene.entities) {
    if(entity.nativeGeometry!==undefined&&!isNativeGeometryArtifact(entity.nativeGeometry))throw new Error('Invalid native geometry snapshot')
    if (entityIds.has(entity.id)) throw new Error(`Duplicate scene entity ${entity.id}`)
    if (!assets.has(entity.geometryAssetId)) throw new Error(`Scene entity ${entity.id} references a missing geometry asset`)
    if (entity.transform.length !== 16 || !entity.transform.every(Number.isFinite)
      || entity.color.length !== 4 || !entity.color.every(Number.isFinite)) {
      throw new Error(`Scene entity ${entity.id} has invalid presentation data`)
    }
    entityIds.add(entity.id)
  }
}
