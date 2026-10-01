import type {SolidInstanceBatchCache} from './solidInstanceBatchCache'
import {transformSketch} from './directSketchGeometry'
import {parseDirectDocument,type DirectDocument} from './directModeling'
import {transformSelection} from './directSolidTools'
import {createSolidInstance,transformSolidInstance,resolveSolidInstances} from './solidInstances'
import {createRuledSketchLoft} from './geometry/brep'
import {stringifyMeshJson} from './meshJson'
export type SolidSceneEditOperation='sketch-transform'|'transform'|'instance-create'|'instance-transform'|'instance-place'|'loft'
export interface SolidSceneEditOptions {operation:SolidSceneEditOperation;id:string;ids:string[];createdId:string;x:number;y:number;z:number;axis:'x'|'y'|'z';angle:number;scale:number}
export function isSolidSceneEdit(operation:unknown):operation is SolidSceneEditOperation{return ['sketch-transform','transform','instance-create','instance-transform','instance-place','loft'].includes(String(operation))}
export function applySolidSceneEdit(document:DirectDocument,p:SolidSceneEditOptions,instanceCache?:SolidInstanceBatchCache):DirectDocument {
 const d=structuredClone(document),axis=p.axis==='x'?[1,0,0] as const:p.axis==='y'?[0,1,0] as const:[0,0,1] as const
 switch(p.operation){
  case 'sketch-transform':{
   const i=d.sketches.findIndex(s=>s.id===p.id);if(i<0)throw Error('Select an existing sketch.')
   d.sketches[i]=transformSketch(d.sketches[i],[p.x,p.y],p.angle,p.scale);return d
  }
  case 'transform':return resolveSolidInstances(transformSelection(d,p.ids,[p.x,p.y,p.z],[...axis],p.angle,p.scale),instanceCache)
  case 'instance-transform':return transformSolidInstance(d,p.id,[p.x,p.y,p.z],[...axis],p.angle,p.scale)
  case 'instance-create':return createSolidInstance(d,p.id,p.createdId,[[1,0,0,p.x],[0,1,0,p.y],[0,0,1,p.z],[0,0,0,1]])
  case 'instance-place':{
   const body=d.bodies.find(b=>b.id===p.id);if(!body?.instance)throw Error('Select a linked instance.')
   body.instance.matrix[0][3]=p.x;body.instance.matrix[1][3]=p.y;body.instance.matrix[2][3]=p.z
   return resolveSolidInstances(d)
  }
  case 'loft':d.bodies.push({id:p.createdId,name:'Ruled loft',...createRuledSketchLoft(d.sketches,p.ids)});return parseDirectDocument(stringifyMeshJson(d))
  default:throw Error('Unsupported scene edit.')
 }
}
