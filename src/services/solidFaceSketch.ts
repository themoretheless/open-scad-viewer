import {bodyPoints,type DirectBody,type Point2} from './directModeling'
import {facePlane,solidTopology} from './directSolidTools'
import {polygonBoundaryLoops} from './geometry/polygon'
/** Prepare the supporting plane and every boundary before publishing a workplane. */
export function prepareSolidFaceSketch(body:DirectBody,index:number){
 if(!Number.isSafeInteger(index)||index<0)throw Error('Select a planar face.')
 const face=solidTopology(body.mesh).faces[index]
 if(!face)throw Error('Selected face is unavailable. Select the face again.')
 const plane=facePlane(body,face),points=bodyPoints(body)
 const outline=polygonBoundaryLoops({positions:body.mesh.positions,indices:Uint32Array.from(face.triangles.flatMap(t=>Array.from(body.mesh.indices.slice(t*3,t*3+3))))}).map(loop=>loop.map(i=>{
  const q=points[i].map((v,k)=>v-plane.origin[k])
  return [q.reduce((s,v,k)=>s+v*plane.u[k],0),q.reduce((s,v,k)=>s+v*plane.v[k],0)] as Point2
 }))
 if(!outline.length||outline.some(loop=>loop.length<3))throw Error('Selected face has no closed boundary. Select another planar face.')
 return {plane,outline,normal:face.normal}
}
