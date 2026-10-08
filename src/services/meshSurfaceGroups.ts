import type {MeshData} from '../core/mesh'
import {surfaceGroupsInKernel} from './geometry/meshAnalysis'
import {createSelectionSurfacePublisher} from './selectionSurfacePublisher'
/** Connected smooth patches inferred by the native topology kernel. */
export function inferSurfaceIds(vertices:Float32Array,indices:Uint32Array,angleDegrees=30):Uint32Array {
 if(vertices.byteLength > 16 * 1024 * 1024) {
  // Pack only referenced records to fit the bounded WASM transfer. Topology stays native.
  if(vertices.length%6 || indices.length%3) throw new Error('Invalid surface grouping input/budget')
  for(let i=0;i<vertices.length;i+=6) if(!Number.isFinite(vertices[i])||!Number.isFinite(vertices[i+1])||!Number.isFinite(vertices[i+2])) throw new Error('Nonfinite mesh position')
  const remap=new Map<number,number>(), packed:number[]=[], mapped=new Uint32Array(indices.length)
  for(let i=0;i<indices.length;i++) {
   const source=indices[i];if(source>=vertices.length/6)throw new Error('Invalid triangle index')
   let id=remap.get(source)
   if(id===undefined){id=remap.size;remap.set(source,id);for(let k=0;k<6;k++)packed.push(vertices[source*6+k])}
   mapped[i]=id
  }
  return surfaceGroupsInKernel(Float32Array.from(packed),mapped,6,angleDegrees)
 }
 return surfaceGroupsInKernel(vertices,indices,6,angleDegrees)
}
/** Preserve authored face IDs exactly; only legacy meshes use inferred patches. */
export function withSelectionSurfaces(mesh:MeshData):MeshData {
 return publishSelectionSurfaces(mesh)
}

const publishSelectionSurfaces = createSelectionSurfacePublisher(
 (vertices, indices) => surfaceGroupsInKernel(vertices, indices, 6),
 { cloneResult: false },
)
