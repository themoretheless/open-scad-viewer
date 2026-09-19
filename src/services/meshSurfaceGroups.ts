import type {MeshData} from '../core/mesh'
import {SurfaceGroupContentCache} from './surfaceGroupContentCache'
const cache=new WeakMap<Float32Array,WeakMap<Uint32Array,Uint32Array>>()
// Republished meshes carry fresh typed arrays; the content identity survives.
const contentCache=new SurfaceGroupContentCache()
function canonicalVertexIds(vertices:Float32Array,indices:Uint32Array):Uint32Array {
 const canonical=new Uint32Array(vertices.length/6),points=new Map<string,number>()
 // Above this ratio unused positions can exhaust the edge-key radix. Reuse
 // canonical as a reference bitmap so weld IDs remain below 300000.
 const sparse=canonical.length>indices.length
 if(sparse)for(const index of indices){if(index>=canonical.length)throw new Error('Invalid triangle index');canonical[index]=1}
 for(let i=0;i<canonical.length;i++){const x=vertices[i*6],y=vertices[i*6+1],z=vertices[i*6+2];if(!Number.isFinite(x)||!Number.isFinite(y)||!Number.isFinite(z))throw new Error('Nonfinite mesh position');if(sparse&&canonical[i]===0)continue;const key=x+','+y+','+z;let id=points.get(key);if(id===undefined){id=points.size;points.set(key,id)}canonical[i]=id}
 return canonical
}
/** Connected smooth patches, not a reconstruction or certificate of CAD surfaces.
 * Sharp (>30 degree), boundary, degenerate and non-manifold edges are barriers.
 * Welding uses identical coordinates only; nearby independent sheets never merge.
 */
export function inferSurfaceIds(vertices:Float32Array,indices:Uint32Array,angleDegrees=30):Uint32Array {
 const count=indices.length/3
 if(!Number.isInteger(count)||vertices.length%6||count>100000||!Number.isFinite(angleDegrees)||angleDegrees<0||angleDegrees>60)throw new Error('Invalid surface grouping input/budget')
 const parents=new Uint32Array(count),normals=new Float64Array(count*3)
 const find=(i:number):number=>{while(parents[i]!==i){parents[i]=parents[parents[i]];i=parents[i]}return i}
 const canonical=canonicalVertexIds(vertices,indices)
 const edges=new Map<number,{triangle:number;from:number;to:number;other:number;count:number}>()
 for(let t=0;t<count;t++) {
  parents[t]=t
  const v0=indices[t*3]!,v1=indices[t*3+1]!,v2=indices[t*3+2]!
  if(v0>=canonical.length||v1>=canonical.length||v2>=canonical.length)throw new Error('Invalid triangle index')
  const p0=v0*6,p1=v1*6,p2=v2*6
  const ax=vertices[p1]-vertices[p0],ay=vertices[p1+1]-vertices[p0+1],az=vertices[p1+2]-vertices[p0+2]
  const bx=vertices[p2]-vertices[p0],by=vertices[p2+1]-vertices[p0+1],bz=vertices[p2+2]-vertices[p0+2]
  const nx=ay*bz-az*by,ny=az*bx-ax*bz,nz=ax*by-ay*bx,length=Math.hypot(nx,ny,nz)
  if(length){normals[t*3]=nx/length;normals[t*3+1]=ny/length;normals[t*3+2]=nz/length}
  const c0=canonical[v0],c1=canonical[v1],c2=canonical[v2]
  for(let i=0;i<3;i++){const from=i===0?c0:i===1?c1:c2,to=i===0?c1:i===1?c2:c0,key=Math.min(from,to)*2097152+Math.max(from,to);const e=edges.get(key);if(e){e.count++;if(e.from===to&&e.to===from)e.other=t}else edges.set(key,{triangle:t,from,to,other:-1,count:1})}
 }
 const threshold=Math.cos(angleDegrees*Math.PI/180)
 for(const e of edges.values())if(e.count===2&&e.other>=0){const a=e.triangle,b=e.other,dot=normals[a*3]*normals[b*3]+normals[a*3+1]*normals[b*3+1]+normals[a*3+2]*normals[b*3+2];if(dot>=threshold-1e-12){const x=find(a),y=find(b);parents[Math.max(x,y)]=Math.min(x,y)}}
 const ids=new Map<number,number>(),result=new Uint32Array(count)
 for(let i=0;i<count;i++){const root=find(i);let id=ids.get(root);if(id===undefined){id=ids.size;ids.set(root,id)}result[i]=id}
 return result
}
/** Preserve authored face IDs exactly; only legacy meshes use inferred patches. */
export function withSelectionSurfaces(mesh:MeshData):MeshData {
 if(mesh.faceIdsAuthoritative)return mesh
 if(mesh.indices.length/3>100000)return mesh
 // Content-keyed cache survives republication; buffer-identity WeakMap remains
 // the fallback for meshes without a content id.
 const contentId=mesh.geometryAssetId
 if(contentId!==undefined){
  const ids=contentCache.getOrCompute(contentId,()=>inferSurfaceIds(mesh.vertices,mesh.indices))
  return {...mesh,faceIds:ids,faceIdsAuthoritative:false}
 }
 let byIndices=cache.get(mesh.vertices);if(!byIndices){byIndices=new WeakMap();cache.set(mesh.vertices,byIndices)}
 let ids=byIndices.get(mesh.indices);if(!ids){ids=inferSurfaceIds(mesh.vertices,mesh.indices);byIndices.set(mesh.indices,ids)}
 return {...mesh,faceIds:ids,faceIdsAuthoritative:false}
}
