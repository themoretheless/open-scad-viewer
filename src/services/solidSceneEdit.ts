import type {SolidInstanceBatchCache} from './solidInstanceBatchCache'
import {transformSketch} from './directSketchGeometry'
import {parseDirectDocument,type DirectDocument} from './directModeling'
import {transformSelection} from './directSolidTools'
import {createSolidInstance,transformSolidInstance,resolveSolidInstances,detachSolidInstances} from './solidInstances'
import {createRuledSketchLoft} from './geometry/brep'
import {stringifyMeshJson} from './meshJson'
export type SolidSceneEditOperation='sketch-transform'|'transform'|'instance-create'|'instance-transform'|'instance-place'|'instance-detach'|'group-create'|'group-move'|'loft'
export interface SolidSceneEditOptions {operation:SolidSceneEditOperation;id:string;ids:string[];createdId:string;x:number;y:number;z:number;axis:'x'|'y'|'z';angle:number;scale:number;group?:string}
export function isSolidSceneEdit(operation:unknown):operation is SolidSceneEditOperation{return ['sketch-transform','transform','instance-create','instance-transform','instance-place','instance-detach','group-create','group-move','loft'].includes(String(operation))}
export function applySolidSceneEdit(document:DirectDocument,p:SolidSceneEditOptions,instanceCache?:SolidInstanceBatchCache):DirectDocument {
 if(p.operation==='instance-detach')return detachSolidInstances(document,p.ids,instanceCache)
 const axis=p.axis==='x'?[1,0,0] as const:p.axis==='y'?[0,1,0] as const:[0,0,1] as const
 if(p.operation==='transform')return resolveSolidInstances(transformSelection(document,p.ids,[p.x,p.y,p.z],[...axis],p.angle,p.scale),instanceCache)
 if(p.operation==='instance-transform')return transformSolidInstance(document,p.id,[p.x,p.y,p.z],[...axis],p.angle,p.scale,instanceCache)
 if(p.operation==='instance-create')return createSolidInstance(document,p.id,p.createdId,[[1,0,0,p.x],[0,1,0,p.y],[0,0,1,p.z],[0,0,0,1]],instanceCache)
 const d=structuredClone(document)
 switch(p.operation){
  case 'group-create':
  case 'group-move':{
   if(typeof p.group!=='string'||p.group.length>100||(p.operation==='group-create'&&!p.group.length))throw Error('Invalid object group.')
   if(p.operation==='group-create'){
    if(d.groups?.some(g=>g.name===p.group))throw Error('Group already exists.')
    d.groups=[...(d.groups??[]),{name:p.group,source:''}]
   }else{
    const ids=new Set(p.ids)
    for(const object of [...d.bodies,...d.sketches,...d.curves??[],...d.surfaces??[]])if(ids.has(object.id)){
     if(p.group)object.group=p.group;else delete object.group
    }
   }
   return d
  }
  case 'sketch-transform':{
   const i=d.sketches.findIndex(s=>s.id===p.id);if(i<0)throw Error('Select an existing sketch.')
   d.sketches[i]=transformSketch(d.sketches[i],[p.x,p.y],p.angle,p.scale);return d
  }
  case 'instance-place':{
   const body=d.bodies.find(b=>b.id===p.id);if(!body?.instance)throw Error('Select a linked instance.')
   body.instance.matrix[0][3]=p.x;body.instance.matrix[1][3]=p.y;body.instance.matrix[2][3]=p.z
   return resolveSolidInstances(d)
  }
  case 'loft':d.bodies.push({id:p.createdId,name:'Ruled loft',...createRuledSketchLoft(d.sketches,p.ids)});return parseDirectDocument(stringifyMeshJson(d))
  default:throw Error('Unsupported scene edit.')
 }
}
