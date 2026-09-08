import type {MeshData} from '../core/mesh'
import type {PickHit} from './rendererContracts'
import {flattenExportMeshes} from './meshExportAdapter'
import {directBodiesScad, extrudeDirectSketch, type DirectDocument} from './directModeling'
import {solidTopology, pushPullFace, bevelSolidEdge, shellSolid, splitSolid, transformSelection, facePlane} from './directSolidTools'
import {booleanPolygonMeshes} from './polygonKernel'
import {importedStlToMeshData} from './stlImport'
export type MainOperation='push'|'fillet'|'chamfer'|'shell'|'split'|'move'|'rotate'|'scale'|'duplicate'|'delete'|'profile'
export interface MainParameters {amount:number;x:number;y:number;z:number;axis:'x'|'y'|'z';edge:number;shape:'rectangle'|'circle';width:number;height:number;cut:boolean}
export function primitiveSource(kind:string,size:number):string {
 if(!Number.isFinite(size)||size<.1||size>10000)throw Error('Size must be 0.1–10000 mm.')
 switch(kind){case 'box':return `cube([${size},${size},${size}]);`;case 'sphere':return `sphere(d=${size}, $fn=48);`;case 'cylinder':return `cylinder(h=${size},d=${size},$fn=48);`;case 'cone':return `cylinder(h=${size},d1=${size},d2=0,$fn=48);`;default:throw Error('Unknown primitive.')}
}
export function sceneBody(mesh:MeshData,index:number){const m=flattenExportMeshes([mesh]);return {id:String(index),name:`Body ${index+1}`,mesh:{positions:m.positions,indices:m.indices}}}
export function sceneFace(mesh:MeshData,hit:PickHit|null){const body=sceneBody(mesh,0),topology=solidTopology(body.mesh);return {body,topology,face:hit?topology.faces.findIndex(f=>f.triangles.includes(hit.triangleIndex)):-1}}
export function mainOperation(meshes:MeshData[],selected:number,hit:PickHit|null,op:MainOperation,p:MainParameters){
 if(!meshes[selected])throw Error('Select a body in the main scene.')
 if(![p.amount,p.x,p.y,p.z,p.width,p.height].every(Number.isFinite))throw Error('Enter finite dimensions.')
 const d:DirectDocument={version:1,sketches:[],bodies:meshes.map(sceneBody)},body=d.bodies[selected],topology=solidTopology(body.mesh)
 const face=hit?.meshIndex===selected?topology.faces.findIndex(f=>f.triangles.includes(hit.triangleIndex)):-1
 if(['push','shell','profile'].includes(op)&&face<0)throw Error('Click a face in the main viewport.')
 const axis= p.axis==='x'?[1,0,0] as const:p.axis==='y'?[0,1,0] as const:[0,0,1] as const
 switch(op){
 case 'push':d.bodies[selected]=pushPullFace(body,face,p.amount);break
 case 'fillet':case 'chamfer':d.bodies[selected]=bevelSolidEdge(body,p.edge,p.amount,op);break
 case 'shell':d.bodies[selected]=shellSolid(body,[face],p.amount);break
 case 'split':{const pair=splitSolid(body,[...axis],p.amount);pair[1].id='split';d.bodies.splice(selected,1,...pair);break}
 case 'delete':d.bodies.splice(selected,1);break
 case 'duplicate':{const next=transformSelection(d,[body.id],[p.x,p.y,p.z],[...axis],0,1).bodies[selected];next.id='copy';d.bodies.push(next);break}
 case 'move':case 'rotate':case 'scale':return transformSelection(d,[body.id],op==='move'?[p.x,p.y,p.z]:[0,0,0],[...axis],op==='rotate'?p.amount:0,op==='scale'?p.amount:1)
 case 'profile':{
  if(p.width<=0||p.height<=0||p.amount===0)throw Error('Profile dimensions must be positive and depth nonzero.')
  const plane=facePlane(body,topology.faces[face]);plane.origin=[...hit!.point]
  if(p.cut)plane.v=plane.v.map(v=>-v) as [number,number,number]
  const points:[number,number][]=p.shape==='circle'?Array.from({length:48},(_,i)=>[p.x+p.width/2*Math.cos(i*Math.PI/24),p.y+p.width/2*Math.sin(i*Math.PI/24)]):[[p.x,p.y],[p.x+p.width,p.y],[p.x+p.width,p.y+p.height],[p.x,p.y+p.height]]
  const profile=extrudeDirectSketch({id:'profile',name:'Profile',closed:true,points,plane},Math.abs(p.amount),'profile-body')
  d.bodies[selected]={...body,mesh:booleanPolygonMeshes(body.mesh,profile.mesh,p.cut?'difference':'union')};break
 }
 }
 return d
}
export const mainSource=(d:DirectDocument)=>directBodiesScad(d)
export function previewMeshes(d:DirectDocument):MeshData[]{return d.bodies.map(b=>importedStlToMeshData({triangleCount:b.mesh.indices.length/3,positions:new Float32Array(b.mesh.indices.flatMap(i=>b.mesh.positions.slice(i*3,i*3+3)))},[.35,.7,.95,1]))}
