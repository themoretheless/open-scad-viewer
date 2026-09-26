import type {MeshData} from '../core/mesh'
import {bounds} from './cadWorkbench'
import {sceneBody} from './mainModeling'
import type {Vec3} from './directSketchGeometry'
/** Authored vertices, feature-edge midpoints and body centers, with a bounded UI budget. */
export function sceneSnapPoints(meshes:MeshData[],exclude:number[]=[]):Vec3[]{
 const result:Vec3[]=[]
 for(let i=0;i<meshes.length;i++){
  if(exclude.includes(i))continue
  const mesh=sceneBody(meshes[i],i).mesh,p=Array.from({length:mesh.positions.length/3},(_,j)=>Array.from(mesh.positions.slice(j*3,j*3+3)) as Vec3)
  result.push(...p.slice(0,Math.max(0,5000-result.length)))
  if(p.length&&result.length<5000){const bb=bounds([{id:'',name:'',mesh}]);result.push(bb.min.map((v,k)=>(v+bb.max[k])/2) as Vec3)}
  const edges=new Map<string,{a:number;b:number;faces:number[]}>()
  for(let t=0;t<mesh.indices.length/3;t++){const ids=mesh.indices.slice(t*3,t*3+3);for(let j=0;j<3;j++){const a=ids[j],b=ids[(j+1)%3],key=[a,b].sort((a,b)=>a-b).join(',');const e=edges.get(key)??{a,b,faces:[]};e.faces.push(meshes[i].faceIds[t]??t);edges.set(key,e)}}
  for(const e of edges.values()){if(result.length>=5000)break;if(e.faces.length!==2||e.faces[0]!==e.faces[1])result.push(p[e.a].map((v,k)=>(v+p[e.b][k])/2) as Vec3)}
  if(result.length>=5000)break
 }
 return result
}
