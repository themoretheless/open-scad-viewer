import type {MeshData} from '../core/mesh'
import type {PickHit} from './rendererContracts'
import {transformPoint,type Vec3} from './math3d'
type FaceMesh=Pick<MeshData,'nativeGeometry'|'faceIdsAuthoritative'|'faceIds'|'vertices'|'indices'|'transform'|'provenance'>
/** Re-anchor the selected native face on a new display tessellation.
 * Never reuse the old triangle index/barycentrics. Changed geometry is rejected
 * until a real topology lineage map exists.
 */
export function remapNativeFaceSelection(previous:FaceMesh,next:FaceMesh,hit:PickHit,nextIndex:number):PickHit|null {
 const a=previous.nativeGeometry,b=next.nativeGeometry
 if(!a||!b||!previous.faceIdsAuthoritative||!next.faceIdsAuthoritative||hit.faceId===null
   ||!['surface','brep','subdivision','patches'].includes(a.kind)
   ||a.nodeId!==b.nodeId||a.kind!==b.kind||a.revision!==b.revision
   ||previous.transform.some((v,i)=>v!==next.transform[i]))return null
 const triangleIndex=next.faceIds.indexOf(hit.faceId)
 if(triangleIndex<0)return null
 const points=[0,1,2].map(c=>{const offset=next.indices[triangleIndex*3+c]*6;return transformPoint(next.transform,[next.vertices[offset],next.vertices[offset+1],next.vertices[offset+2]])})
 const u=points[1].map((v,i)=>v-points[0][i]),v=points[2].map((v,i)=>v-points[0][i])
 const n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]],length=Math.hypot(...n)
 if(!length)return null
 const run=next.provenance.find(r=>r.triangleStart<=triangleIndex&&triangleIndex<r.triangleEnd)
 return {meshIndex:nextIndex,triangleIndex,faceId:hit.faceId,point:points[0].map((_,i)=>(points[0][i]+points[1][i]+points[2][i])/3) as Vec3,normal:n.map(x=>x/length) as Vec3,barycentric:[1/3,1/3,1/3],source:run?.source??null,backside:run?.backside??false}
}
