import type {MeshData} from '../core/mesh'
import {sceneFace} from './mainModeling'
import {facePlane} from './directSolidTools'
import type {PickHit} from './rendererContracts'
export interface SketchSupport {entity:string;face:number}
export function resolveSketchSupport(meshes:MeshData[],support:SketchSupport){
 const i=meshes.findIndex(m=>m.entityId===support.entity);if(i<0)throw Error('Sketch support body is missing; pick a new support face.')
 const triangle=Array.from(meshes[i].faceIds).indexOf(support.face);if(triangle<0)throw Error('Sketch support face changed; pick a new support face.')
 const t=sceneFace(meshes[i],{meshIndex:i,triangleIndex:triangle} as PickHit);if(t.face<0)throw Error('Sketch support is not planar.');return facePlane(t.body,t.topology.faces[t.face])
}
