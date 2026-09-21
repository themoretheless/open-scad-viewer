import type {MeshData} from '../core/mesh'
import {SurfaceGroupContentCache} from './surfaceGroupContentCache'

export interface SelectionSurfacePublisherOptions {
  /** Clone cached ids when the result crosses a transferable ownership boundary. */
  cloneResult?: boolean
}

/** One cache per worker; transferred results never detach retained cache entries. */
export function createSelectionSurfacePublisher(
  infer: (vertices: Float32Array, indices: Uint32Array) => Uint32Array,
  options: SelectionSurfacePublisherOptions = {},
) {
  const cache = new SurfaceGroupContentCache()
  const byVertices = new WeakMap<Float32Array, WeakMap<Uint32Array, Uint32Array>>()
  const cloneResult = options.cloneResult ?? true
  return (mesh: MeshData): MeshData => {
    if (mesh.faceIdsAuthoritative || mesh.faceIdsInferred || mesh.indices.length / 3 > 100000) return mesh
    const compute = () => infer(mesh.vertices, mesh.indices)
    let ids: Uint32Array
    if (mesh.geometryAssetId !== undefined) {
      ids = cache.getOrCompute(mesh.geometryAssetId, compute)
    } else {
      let byIndices = byVertices.get(mesh.vertices)
      if (!byIndices) {
        byIndices = new WeakMap()
        byVertices.set(mesh.vertices, byIndices)
      }
      ids = byIndices.get(mesh.indices) ?? compute()
      byIndices.set(mesh.indices, ids)
    }
    return {...mesh, faceIds: cloneResult ? ids.slice() : ids, faceIdsInferred: true}
  }
}
