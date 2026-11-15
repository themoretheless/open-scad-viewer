import { GEOMETRY_WORKER_PAYLOAD_LIMITS } from '../geometryWorkerProtocol'
import type { GeometryBuildSuccess, GeometryWorkerEvent, GeometryWorkerWireEvent } from '../geometryWorkerProtocol'
import { assertGeometryScene, meshesFromGeometryScene, type GeometryScene } from '../../core/scene'
import type { MeshData } from '../../core/mesh'
import { hasExactKeys, isDenseExactArray, isFloat32Payload, isRecord, isUint32Payload } from './isRecord'

export interface MaterializedWireScene {
  readonly meshes: MeshData[]
  /** Backing buffers of shared assets: they may appear in several materialized meshes. */
  readonly assetBuffers: ReadonlySet<ArrayBuffer>
}

const isIdentity = (value: unknown, prefix: string) =>
  typeof value === 'string' &&
  value.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters &&
  value.startsWith(prefix)

function hasWireAssets(assets: unknown[], assetBuffers: Set<ArrayBuffer>): boolean {
  for (const asset of assets) {
    if (
      !isRecord(asset) ||
      !hasExactKeys(asset, ['version', 'id', 'vertices', 'indices']) ||
      asset.version !== 1 ||
      !isIdentity(asset.id, 'asset:') ||
      !isFloat32Payload(asset.vertices) ||
      !isUint32Payload(asset.indices)
    )
      return false
    for (const view of [asset.vertices, asset.indices]) {
      const buffer = view.buffer as ArrayBuffer
      if (assetBuffers.has(buffer)) return false
      assetBuffers.add(buffer)
    }
  }
  return true
}

/** Worker results publish eager inspection artifacts: every entity carries a ready set. */
function isWireEntity(entity: unknown): boolean {
  if (
    !isRecord(entity) ||
    !hasExactKeys(entity, ['id', 'geometryAssetId', 'color', 'transform', 'inspection'], ['nativeGeometry', 'lods'])
  )
    return false
  const inspection = entity.inspection
  return (
    isIdentity(entity.id, 'entity:') &&
    isIdentity(entity.geometryAssetId, 'asset:') &&
    isDenseExactArray(entity.color, 4) &&
    isFloat32Payload(entity.transform) &&
    isRecord(inspection) &&
    hasExactKeys(
      inspection,
      ['state', 'version', 'bvh', 'edgeIndices', 'faceIds', 'provenance', 'topology'],
      ['faceIdsAuthoritative', 'faceIdsInferred'],
    ) &&
    inspection.state === 'ready' &&
    inspection.version === 1 &&
    isDenseExactArray(inspection.provenance, GEOMETRY_WORKER_PAYLOAD_LIMITS.triangles)
  )
}

const materialized = new WeakMap<object, MaterializedWireScene>()

/**
 * Validate the structure of a wire `GeometryScene` and materialize the eager
 * per-entity mesh view (zero-copy: instances share their asset's views). The
 * caller still runs the deep mesh validators and payload budget on the result.
 * Returns null for anything that is not an exact, internally consistent scene.
 */
export function materializeWireScene(value: unknown): MaterializedWireScene | null {
  if (!isRecord(value)) return null
  const cached = materialized.get(value)
  if (cached) return cached
  const assetBuffers = new Set<ArrayBuffer>()
  if (
    !hasExactKeys(value, ['version', 'assets', 'entities']) ||
    value.version !== 1 ||
    !isDenseExactArray(value.assets, GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes) ||
    !isDenseExactArray(value.entities, GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes) ||
    !hasWireAssets(value.assets, assetBuffers) ||
    !value.entities.every(isWireEntity)
  )
    return null
  try {
    const scene = value as unknown as GeometryScene
    assertGeometryScene(scene)
    const result = { meshes: meshesFromGeometryScene(scene), assetBuffers }
    materialized.set(value, result)
    return result
  } catch {
    return null
  }
}

/** Host view of a validated wire event: a success carries its materialized meshes. */
export function materializeGeometryWorkerEvent(event: GeometryWorkerWireEvent): GeometryWorkerEvent {
  if (event.status !== 'succeeded') return event
  const wire = materializeWireScene(event.scene)
  if (!wire) throw new Error('Geometry Worker success carries an invalid scene')
  const { scene: _scene, ...rest } = event
  const success: GeometryBuildSuccess = { ...rest, meshes: wire.meshes }
  return success
}
