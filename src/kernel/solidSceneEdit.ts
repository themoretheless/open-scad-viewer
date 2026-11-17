import type {SolidInstanceBatchCache} from './solidInstanceBatchCache'
import {DirectInputError} from '../i18n/kernelErrors'
import {transformSketch} from '../app/directSketchGeometry'
import {parseDirectDocument,type DirectDocument} from '../app/directModeling'
import {transformSelection} from '../app/directSolidTools'
import {createSolidInstance,transformSolidInstance,resolveSolidInstances,detachSolidInstances} from './solidInstances'
import {createSolidInstanceArray,solidInstanceArrayPlacements,type SolidInstanceArraySpec} from './solidInstanceArrays'
import {worldPoints} from '../app/directSketchGeometry'
import {sampleCurveArrayPath} from './solidInstanceArrayCurvePath'
import {createRuledSketchLoft} from './geometry/brep'
import {stringifyMeshJson} from '../exchange/meshJson'
export type SolidSceneEditOperation='duplicate'|'sketch-transform'|'transform'|'instance-create'|'instance-transform'|'instance-place'|'instance-detach'|'instance-array'|'group-create'|'group-move'|'loft'
export interface SolidSceneEditOptions {operation:SolidSceneEditOperation;id:string;ids:string[];createdId:string;x:number;y:number;z:number;axis:'x'|'y'|'z'|[number,number,number];angle:number;scale:number;group?:string;copyIds?:string[];array?:SolidSceneInstanceArray}
/** Linked-instance array of `id`: `step`/`extent` run along `axis` with `distance` as the spacing or the total span; `path` follows the sketch or NURBS curve `pathId` (a curve is sampled within `toleranceMm`, default 0.01 mm). Copies get ids `${createdId}:n`. */
export interface SolidSceneInstanceArray {kind:'step'|'extent'|'path';count:number;distance:number;pathId?:string;align?:boolean;toleranceMm?:number}
export function isSolidSceneEdit(operation:unknown):operation is SolidSceneEditOperation{return ['duplicate','sketch-transform','transform','instance-create','instance-transform','instance-place','instance-detach','instance-array','group-create','group-move','loft'].includes(String(operation))}
export function applySolidSceneEdit(document:DirectDocument,p:SolidSceneEditOptions,instanceCache?:SolidInstanceBatchCache):DirectDocument {
 if(p.operation==='duplicate'){
  const originals=[...document.bodies,...document.sketches],selected=originals.filter(object=>p.ids.includes(object.id))
  if(!selected.length||selected.length!==p.ids.length||new Set(p.ids).size!==p.ids.length||!p.copyIds||p.copyIds.length!==p.ids.length||new Set(p.copyIds).size!==p.copyIds.length||p.copyIds.some(id=>!id||[...originals,...document.curves??[],...document.surfaces??[],...document.sourceBodies??[]].some(object=>object.id===id)))throw Error('Invalid duplicate identities.')
  const copies=new Map(p.ids.map((id,index)=>[id,p.copyIds![index]]))
  const moved=transformSelection(document,p.ids,[p.x,p.y,p.z],[0,0,1],0,1),next=structuredClone(document)
  for(const sketch of moved.sketches)if(copies.has(sketch.id)){sketch.id=copies.get(sketch.id)!;sketch.name=(sketch.name+' · copy').slice(0,100);next.sketches.push(sketch)}
  for(const body of moved.bodies)if(copies.has(body.id)){body.id=copies.get(body.id)!;if(body.instance&&copies.has(body.instance.sourceId))body.instance.sourceId=copies.get(body.instance.sourceId)!;body.name=(body.name+' · copy').slice(0,100);next.bodies.push(body)}
  return resolveSolidInstances(next,instanceCache)
 }
 if(p.operation==='instance-detach')return detachSolidInstances(document,p.ids,instanceCache)
 // A frame axis (CPlane/Local) arrives as a vector; world axes keep their letter.
 const axis=Array.isArray(p.axis)?p.axis:p.axis==='x'?[1,0,0] as const:p.axis==='y'?[0,1,0] as const:[0,0,1] as const
 if(Array.isArray(p.axis)&&(p.axis.length!==3||!p.axis.every(Number.isFinite)||Math.hypot(...p.axis)<1e-9))throw Error('Invalid transform axis.')
 if(p.operation==='transform')return resolveSolidInstances(transformSelection(document,p.ids,[p.x,p.y,p.z],[...axis],p.angle,p.scale),instanceCache)
 if(p.operation==='instance-transform')return transformSolidInstance(document,p.id,[p.x,p.y,p.z],[...axis],p.angle,p.scale,instanceCache)
 if(p.operation==='instance-array')return createSolidInstanceArray(document,p.id,`${p.createdId}:`,solidInstanceArrayPlacements(instanceArraySpec(document,p,[...axis])),instanceCache)
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
   return resolveSolidInstances(d,instanceCache)
  }
  case 'loft':if(p.ids.length<2||p.ids.length>64)throw new DirectInputError('DIRECT_LOFT_SECTIONS','Select 2–64 closed polygon sketches in loft order.',undefined,p.ids);d.bodies.push({id:p.createdId,name:'Ruled loft',...createRuledSketchLoft(d.sketches,p.ids)});return parseDirectDocument(stringifyMeshJson(d))
  default:throw Error('Unsupported scene edit.')
 }
}
function instanceArraySpec(document:DirectDocument,p:SolidSceneEditOptions,direction:[number,number,number]):SolidInstanceArraySpec{
 const a=p.array
 if(!a)throw new DirectInputError('DIRECT_PATTERN_ARRAY','Array parameters are missing.','patternCount',[])
 if(a.kind!=='path')return a.kind==='step'?{kind:'step',direction,step:a.distance,count:a.count}:{kind:'extent',direction,extent:a.distance,count:a.count}
 const curve=document.curves?.find(c=>c.id===a.pathId)
 if(curve){const path=sampleCurveArrayPath(curve.curve,a.toleranceMm);return {kind:'path',points:path.points,closed:path.closed,count:a.count,align:!!a.align}}
 const sketch=document.sketches.find(s=>s.id===a.pathId)
 if(!sketch||sketch.points.length<2)throw new DirectInputError('DIRECT_PATTERN_ARRAY','Select a sketch line or a NURBS curve as the array path.','patternCount',[])
 const points=worldPoints(sketch.points,sketch.plane)
 return {kind:'path',points:sketch.closed?[...points,points[0]]:points,closed:!!sketch.closed,count:a.count,align:!!a.align}
}
