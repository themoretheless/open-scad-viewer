import type {MeshData} from '../core/mesh'
import type {PickHit} from './rendererContracts'
import {flattenGroupGeometry, sceneBody} from './meshFlatten'
import {directBodiesScad, extrudeDirectSketch, type DirectDocument} from './directModeling'
import {solidTopology, pushPullFace, bevelSolidEdge, shellSolid, splitSolid, transformSelection, facePlane} from './directSolidTools'
import {localMeshBevel} from './generalMeshTools'
import {extendedShell,extendedBevel} from './mainSolidExtensions'
import {unit3} from './directSketchGeometry'
import {booleanPolygonMeshes} from './geometry/polygon'
import {importedStlToMeshData} from './stlImport'
export type MainOperation='push'|'fillet'|'chamfer'|'shell'|'split'|'move'|'rotate'|'scale'|'duplicate'|'delete'|'profile'
export interface MainParameters {amount:number;x:number;y:number;z:number;axis:'x'|'y'|'z';edge:number;shape:'rectangle'|'circle';width:number;height:number;cut:boolean; normal?:[number,number,number]; openings?:number[]; selection?:number[];step?:number;adaptive?:boolean;snap?:boolean;edges?:number[];endRadius?:number}
export function primitiveSource(kind:string,size:number):string {
 if(!Number.isFinite(size)||size<.1||size>10000)throw Error('Size must be 0.1–10000 mm.')
 switch(kind){case 'box':return `cube([${size},${size},${size}]);`;case 'sphere':return `sphere(d=${size}, $fn=48);`;case 'cylinder':return `cylinder(h=${size},d=${size},$fn=48);`;case 'cone':return `cylinder(h=${size},d1=${size},d2=0,$fn=48);`;default:throw Error('Unknown primitive.')}
}
export { sceneBody }
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
 case 'fillet':case 'chamfer':{if((p.edges?.length??0)>1||p.endRadius!==undefined){let next=body;for(const edge of p.edges?.length?p.edges:[p.edge])next=localMeshBevel(body,edge,p.amount,op,p.endRadius??p.amount,next);d.bodies[selected]=next}else d.bodies[selected]=extendedBevel(body,p.edge,p.amount,op);break}
 case 'shell':d.bodies[selected]=extendedShell(body,p.openings?.length?p.openings:[face],p.amount,p.step,p.adaptive);break
 case 'split':{const pair=splitSolid(body,p.normal?unit3(p.normal):[...axis],p.amount);pair[1].id='split';d.bodies.splice(selected,1,...pair);break}
 case 'delete':d.bodies=d.bodies.filter((_,i)=>!(p.selection??[selected]).includes(i));break
 case 'duplicate':{const ids=(p.selection??[selected]).map(String),next=transformSelection(d,ids,[p.x,p.y,p.z],[...axis],0,1);d.bodies.push(...next.bodies.filter(b=>ids.includes(b.id)).map(b=>({...b,id:b.id+'-copy'})));break}
 case 'move':case 'rotate':case 'scale':return transformSelection(d,(p.selection??[selected]).map(String),op==='move'?[p.x,p.y,p.z]:[0,0,0],[...axis],op==='rotate'?p.amount:0,op==='scale'?p.amount:1)
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
export function previewMeshes(d:DirectDocument):MeshData[]{return d.bodies.map(b=>{
 const indices=b.mesh.indices,source=b.mesh.positions,positions=new Float32Array(indices.length*3)
 for(let k=0;k<indices.length;k++){const s=indices[k]*3,o=k*3;positions[o]=source[s];positions[o+1]=source[s+1];positions[o+2]=source[s+2]}
 return importedStlToMeshData({triangleCount:indices.length/3,positions},[.35,.7,.95,1])})}
