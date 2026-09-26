import {lightenSolid,type LighteningOptions} from './solidLightening'
import {textureSurface,type SurfaceTextureOptions} from './surfaceTexture'
import type {DirectDocument,DirectBody,DirectSketch} from './directModeling'
import {normalizePolygonMesh,type PolygonMesh} from './geometry/polygon'
import type {Vec3} from './directSketchGeometry'
import {callGeometryRust} from './geometry/kernel'
export type CadAction='lighten'|'texture'|'union'|'difference'|'intersection'|'loft'|'sweep'|'mirror'|'pattern'|'align'|'distribute'|'draft'|'hole'|'thread'|'joint'|'resize'
export interface CadOptions {action:CadAction;ids:string[];sketches:DirectSketch[];axis:Vec3;origin:Vec3;amount:number;count:number;width:number;height:number;depth:number;pitch:number;secondary:number;mode:string;pathId:string;profileIds:string[];texture?:SurfaceTextureOptions;lightening?:LighteningOptions}
/** Kernel results decode DirectBody mesh fields as plain arrays; box once, here. */
const typedBody=(body:DirectBody):DirectBody=>{normalizePolygonMesh(body.mesh);return body}
export const bounds=(bodies:DirectBody[])=>callGeometryRust<{min:number[];max:number[]}>('body_bounds',{positions:bodies.map(b=>b.mesh.positions)})
export function pathPoints(path:number[][],fractions:number[]):number[][] {
 return callGeometryRust('cad_path_points',{path,fractions})
}
export function pathPoint(path:number[][],fraction:number):number[]{return pathPoints(path,[fraction])[0]}
export function cadOperation(input:DirectDocument,o:CadOptions):DirectDocument{
 const d=structuredClone(input),selected=o.ids.map(id=>d.bodies.find(b=>b.id===id)).filter((b):b is DirectBody=>!!b)
 if(![...o.origin,o.amount,o.width,o.height,o.depth,o.pitch,o.secondary].every(Number.isFinite))throw Error('Enter finite parameters.')
 const replace=(b:DirectBody)=>{d.bodies[d.bodies.findIndex(old=>old.id===b.id)]=b}
 if(o.action==='mirror'){
  if(!selected.length)throw Error('Select bodies.')
  const mirrored=callGeometryRust<DirectBody[]>('cad_mirror_bodies',{bodies:selected,origin:o.origin,axis:o.axis}).map(typedBody)
  for(const body of mirrored){if(o.mode==='copy')d.bodies.push({...body,id:crypto.randomUUID()});else replace(body)}
  return d
 }
 if(o.action==='align'||o.action==='distribute'){
  const bodies=callGeometryRust<DirectBody[]>('cad_arrange_bodies',{bodies:selected,axis:o.axis,action:o.action,mode:o.mode}).map(typedBody)
  bodies.forEach(replace);return d
 }
 if(o.action==='joint'){
  const bodies=callGeometryRust<DirectBody[]>('cad_joint_bodies',{bodies:selected,axis:o.axis,origin:o.origin,amount:o.amount,mode:o.mode}).map(typedBody)
  bodies.forEach(replace);return d
 }
 if(o.action==='pattern'){
  const groups=callGeometryRust<DirectBody[][]>('cad_pattern_bodies',{bodies:selected,axis:o.axis,amount:o.amount,count:o.count,path:o.sketches.find(s=>s.id===o.pathId)??null}).map(g=>g.map(typedBody))
  groups.forEach((group,i)=>group.forEach(body=>{if(i===0)replace(body);else d.bodies.push({...body,id:crypto.randomUUID()})}))
  return d
 }
 if(['union','difference','intersection'].includes(o.action)){
  const result=callGeometryRust<DirectBody[]>('cad_boolean_bodies',{bodies:selected,operation:o.action}).map(typedBody)
  const first=selected[0]?.id
  d.bodies=d.bodies.flatMap(body=>body.id===first?result:o.ids.includes(body.id)?[]:[body])
  return d
 }
 if(o.action==='hole'){
  if(!selected.length)throw Error('Select bodies.')
  replace(typedBody(callGeometryRust<DirectBody>('cad_hole_body',{body:selected[0],axis:o.axis,origin:o.origin,width:o.width,depth:o.depth,height:o.height,secondary:o.secondary,mode:o.mode})))
  return d
 }
 if(o.action==='draft'){
  callGeometryRust<DirectBody[]>('cad_draft_bodies',{bodies:selected,axis:o.axis,origin:o.origin,amount:o.amount}).map(typedBody).forEach(replace)
  return d
 }
 if(o.action==='loft'||o.action==='sweep'){
  const mesh=normalizePolygonMesh(callGeometryRust<PolygonMesh>('cad_build_sections',{action:o.action,sketches:o.sketches,profileIds:o.profileIds,pathId:o.pathId}))
  d.bodies.push({id:crypto.randomUUID(),name:o.action==='loft'?'Loft':'Sweep',mesh});return d
 }
 if(o.action==='thread'){
  if(!selected.length)throw Error('Select bodies.')
  replace(typedBody(callGeometryRust<DirectBody>('cad_thread_body',{body:selected[0],axis:o.axis,origin:o.origin,width:o.width,depth:o.depth,pitch:o.pitch,mode:o.mode})));return d
 }
 if(!selected.length)throw Error('Select bodies.')
 if(o.action==='lighten'){if(!o.lightening)throw Error('Configure lightening.');for(const b of selected)replace(lightenSolid(b,o.lightening));return d}
 if(o.action==='texture'){if(!o.texture)throw Error('Configure surface texture.');for(const b of selected)replace(textureSurface(b,o.texture));return d}
 if(o.action==='resize'){
  const bodies=callGeometryRust<DirectBody[]>('cad_resize_bodies',{bodies:selected,desired:[o.width,o.height,o.depth]}).map(typedBody)
  bodies.forEach(replace);return d
 }

 return d
}
