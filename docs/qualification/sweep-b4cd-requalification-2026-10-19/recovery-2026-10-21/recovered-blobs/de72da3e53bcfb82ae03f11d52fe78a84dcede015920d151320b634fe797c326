import type {MeshData} from '../core/mesh'
import {bounds} from './cadWorkbench'
import {sceneBody} from './mainModeling'
import type {Vec3} from './directSketchGeometry'
/** Authored vertices, feature-edge midpoints and body centers, with a bounded UI budget. */
export function sceneSnapPoints(meshes:MeshData[],exclude:number[]=[]):Vec3[]{
 const result:Vec3[]=[]
 for(let i=0;i<meshes.length;i++){
  if(exclude.includes(i))continue
  const mesh=sceneBody(meshes[i],i).mesh,positions=mesh.positions,vertexCount=positions.length/3
  // Index the typed array directly; stop at the UI budget before allocating.
  for(let j=0;j<vertexCount&&result.length<5000;j++)result.push([positions[j*3],positions[j*3+1],positions[j*3+2]])
  if(vertexCount>0&&result.length<5000){const bb=bounds([{id:'',name:'',mesh}]);result.push(bb.min.map((v,k)=>(v+bb.max[k])/2) as Vec3)}
  if(result.length<5000){
   // BigInt keys: vertex ids reach Uint32 range, so a*(N)+b can exceed 2^53.
   const edges=new Map<bigint,{a:number;b:number;faces:number[]}>()
   const indices=mesh.indices,faceIds=meshes[i].faceIds
   for(let t=0;t<indices.length/3;t++){for(let j=0;j<3;j++){
    const a=indices[t*3+j],b=indices[t*3+(j+1)%3],lo=a<b?a:b,hi=a<b?b:a,key=(BigInt(lo)<<32n)|BigInt(hi)
    const e=edges.get(key)??{a,b,faces:[]};e.faces.push(faceIds[t]??t);edges.set(key,e)
   }}
   for(const e of edges.values()){if(result.length>=5000)break;if(e.faces.length!==2||e.faces[0]!==e.faces[1])result.push([(positions[e.a*3]+positions[e.b*3])/2,(positions[e.a*3+1]+positions[e.b*3+1])/2,(positions[e.a*3+2]+positions[e.b*3+2])/2])}
  }
  if(result.length>=5000)break
 }
 return result
}
