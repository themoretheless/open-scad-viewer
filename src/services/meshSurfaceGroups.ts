import type {MeshData} from '../core/mesh'
import {SurfaceGroupContentCache} from './surfaceGroupContentCache'
const cache=new WeakMap<Float32Array,WeakMap<Uint32Array,Uint32Array>>()
// Republished meshes carry fresh typed arrays; the content identity survives.
const contentCache=new SurfaceGroupContentCache()
/** Connected smooth patches, not a reconstruction or certificate of CAD surfaces.
 * Sharp (>30 degree), boundary, degenerate and non-manifold edges are barriers.
 * Welding uses identical coordinates only; nearby independent sheets never merge.
 */
export function inferSurfaceIds(vertices:Float32Array,indices:Uint32Array,angleDegrees=30):Uint32Array {
 const count=indices.length/3
 if(!Number.isInteger(count)||vertices.length%6||count>100000||!Number.isFinite(angleDegrees)||angleDegrees<0||angleDegrees>60)throw new Error('Invalid surface grouping input/budget')
 const parents=Uint32Array.from({length:count},(_,i)=>i),normals=new Float64Array(count*3)
 const find=(i:number):number=>{while(parents[i]!==i){parents[i]=parents[parents[i]];i=parents[i]}return i}
 const canonical=new Uint32Array(vertices.length/6),points=new Map<string,number>()
 for(let i=0;i<canonical.length;i++){const x=vertices[i*6],y=vertices[i*6+1],z=vertices[i*6+2];if(!Number.isFinite(x)||!Number.isFinite(y)||!Number.isFinite(z))throw new Error('Nonfinite mesh position');const key=x+','+y+','+z;let id=points.get(key);if(id===undefined){id=points.size;points.set(key,id)}canonical[i]=id}
 const edges=new Map<number,{triangle:number;from:number;to:number;other:number;count:number}>()
 for(let t=0;t<count;t++) {
  const v=[indices[t*3]!,indices[t*3+1]!,indices[t*3+2]!];if(v.some(i=>i>=canonical.length))throw new Error('Invalid triangle index')
  const a=[0,1,2].map(j=>vertices[v[1]*6+j]-vertices[v[0]*6+j]),b=[0,1,2].map(j=>vertices[v[2]*6+j]-vertices[v[0]*6+j])
  const n=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]],length=Math.hypot(...n)
  if(length)normals.set(n.map(x=>x/length),t*3)
  for(let i=0;i<3;i++){const from=canonical[v[i]]!,to=canonical[v[(i+1)%3]]!,key=Math.min(from,to)*2097152+Math.max(from,to);const e=edges.get(key);if(e){e.count++;if(e.from===to&&e.to===from)e.other=t}else edges.set(key,{triangle:t,from,to,other:-1,count:1})}
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
