import type { PolygonMesh } from './geometry/polygon'
import type { DirectBody } from './directModeling'
import type { Vec3, SketchPlane } from './directSketchGeometry'
import { callGeometryRust } from './geometry/kernel'
export interface SolidFace { triangles:number[]; normal:Vec3; offset:number; vertices:number[]; center:Vec3 }
export interface SolidEdge { a:number; b:number; faces:[number,number] }
export function solidTopology(mesh:PolygonMesh):{faces:SolidFace[];edges:SolidEdge[]} {
 return callGeometryRust('cad_mesh_topology',{mesh})
}
/** Numerical unique-support admission for legacy display selections. */
export function selectedBrepSupport(body:DirectBody,triangles:number[]):number {
 return callGeometryRust<number>('cad_select_brep_support',{body,triangles})
}
export function selectedBrepStraightEdge(body:DirectBody,vertices:[number,number]):number {
 return callGeometryRust<number>('cad_select_brep_edge',{body,vertices})
}
export function facePlane(body:DirectBody,face:SolidFace):SketchPlane {
 return callGeometryRust('cad_face_plane',{mesh:body.mesh,face})
}
export function pushPullFace(body:DirectBody,faceIndex:number,distance:number):DirectBody {
 return callGeometryRust('cad_planar_edit',{body,action:'push',faces:[faceIndex],amount:distance})
}
/** Native retained-body editing; fillet surfaces are currently planar facets. */
export function bevelBrepBody(body:DirectBody,edges:number[],size:number,kind:'chamfer'|'fillet',segments=16):DirectBody {
 if(body.brep){
  throw new Error(`Mesh ${kind} cannot be claimed as analytic-${kind} for B-rep bodies; refuse faceted fallback (openscad-viewer/brep-1 quarantine)`)
 }
 return callGeometryRust('cad_edge_edit',{body,edges,size,kind,segments})
}
export function bevelSolidEdge(body:DirectBody,edgeIndex:number,size:number,kind:'chamfer'|'fillet'):DirectBody {
 return bevelBrepBody(body,[edgeIndex],size,kind)
}
export function shellSolid(body:DirectBody,openingFaces:number[],thickness:number):DirectBody {
 if(body.brep){
  throw new Error('Mesh shell cannot be claimed as analytic-shell for B-rep bodies; refuse faceted fallback (openscad-viewer/brep-1 quarantine)')
 }
 return callGeometryRust('cad_planar_edit',{body,action:'shell',faces:openingFaces,amount:thickness})
}
export function splitSolid(body:DirectBody,normal:Vec3,offset:number):[DirectBody,DirectBody] {
 const [positive,negative]=callGeometryRust<[DirectBody,DirectBody]>('cad_split_body',{body,normal,offset})
 return [{...positive,name:(body.name+' · +').slice(0,100)},{...negative,id:body.id+'-split',name:(body.name+' · −').slice(0,100)}]
}
export function transformBodies(bodies:DirectBody[],delta:Vec3,axis:Vec3,angle:number,scale:number):DirectBody[] {
 return callGeometryRust<DirectBody[]>('cad_transform_bodies',{bodies,delta,axis,angle,scale})
}

/** Transform the entire selection around one world-space pivot, preserving analytic sketches. */
export function transformSelection(document: import('./directModeling').DirectDocument, ids:string[],delta:Vec3,axis:Vec3,angle:number,scale:number):import('./directModeling').DirectDocument {
 return callGeometryRust<import('./directModeling').DirectDocument>('cad_transform_selection',{document,ids,delta,axis,angle,scale})
}
