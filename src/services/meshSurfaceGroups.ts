import type {MeshData} from '../core/mesh'
import {surfaceGroupsInKernel} from './geometry/meshAnalysis'
/** Connected smooth patches, not a reconstruction or certificate of CAD surfaces.
 * Sharp (>30 degree), boundary, degenerate and non-manifold edges are barriers.
 * Welding uses identical coordinates only; nearby independent sheets never merge.
 */
export function selectionSurfaceIds(vertices:Float32Array,indices:Uint32Array,angleDegrees=30):Uint32Array {return surfaceGroupsInKernel(vertices,indices,6,angleDegrees)}
/** Preserve authored face IDs exactly; only legacy meshes use inferred patches. */
export function withSelectionSurfaces(mesh:MeshData):MeshData {
 if(mesh.faceIdsAuthoritative)return mesh
 if(mesh.faceIdsSurfaceGroups)return mesh
 const triangleCount=mesh.indices.length/3
 if(triangleCount>100000)return mesh
 return {...mesh,faceIds:selectionSurfaceIds(mesh.vertices,mesh.indices),faceIdsSurfaceGroups:true,faceIdsAuthoritative:false}
}
