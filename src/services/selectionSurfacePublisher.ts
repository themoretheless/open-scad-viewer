import type {MeshData} from '../core/mesh'
import {SurfaceGroupContentCache} from './surfaceGroupContentCache'

/** One cache per worker; transferred results never detach retained cache entries. */
export function createSelectionSurfacePublisher(
  infer: (vertices: Float32Array, indices: Uint32Array) => Uint32Array,
) {
  const cache = new SurfaceGroupContentCache()
  return (mesh: MeshData): MeshData => {
    if (mesh.faceIdsAuthoritative || mesh.faceIdsInferred || mesh.indices.length / 3 > 100000) return mesh
    const compute = () => infer(mesh.vertices, mesh.indices)
    const ids = mesh.geometryAssetId === undefined ? compute() : cache.getOrCompute(mesh.geometryAssetId, compute)
    return {...mesh, faceIds: ids.slice(), faceIdsInferred: true}
  }
}
