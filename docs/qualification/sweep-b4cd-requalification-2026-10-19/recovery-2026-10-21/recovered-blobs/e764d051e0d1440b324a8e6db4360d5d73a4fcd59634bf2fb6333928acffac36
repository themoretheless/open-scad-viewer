import {sampleCurve} from './directSketchGeometry'
import {requirePolylineSketch} from './retainedSketchProfile'
import type {DirectDocument} from './directModeling'
import {updateSolidNurbsControlPoint} from './solidNurbs'
import {bridgeNurbsCurves} from './directDimensions'
export type SolidPointEditOptions=
 | {kind:'cv';id:string;u:number;v:number;point:[number,number,number];weight?:number}
 | {kind:'sketch-vertex';id:string;index:number;point:[number,number]}
 | {kind:'analytic-handle';id:string;handle:'center'|'radius'|'start'|'end';point:[number,number]}
 | {kind:'vertices';id:string;indices:number[];delta:[number,number,number]}
/** Edit an isolated snapshot, retaining the dependent bridge update used by CV dragging. */
export function applySolidPointEdit(source:DirectDocument,p:SolidPointEditOptions):DirectDocument {
 if(p.kind==='analytic-handle'||p.kind==='sketch-vertex'){
  const d=structuredClone(source),sketch=d.sketches.find(s=>s.id===p.id)
  if(!sketch||!p.point.every(Number.isFinite))throw Error('Select an existing sketch and a finite point.')
  if(p.kind==='sketch-vertex'){
   requirePolylineSketch(sketch)
   if(!Number.isInteger(p.index)||p.index<0||p.index>=sketch.points.length)throw Error('Sketch vertex is out of range.')
   delete sketch.analytic;sketch.points[p.index]=[...p.point]
  }else{
   const a=sketch.analytic;if(!a)throw Error('Select an analytic circle or arc.')
   if(p.handle==='center')a.center=[...p.point]
   else if(p.handle==='radius')a.radius=Math.max(.01,Math.hypot(p.point[0]-a.center[0],p.point[1]-a.center[1]))
   else {const angle=Math.atan2(p.point[1]-a.center[1],p.point[0]-a.center[0])*180/Math.PI;if(p.handle==='start'){const end=a.start+a.sweep;a.start=angle;a.sweep=((end-angle)%360+360)%360||360}else a.sweep=((angle-a.start)%360+360)%360||360}
   sketch.points=sampleCurve(a);delete sketch.dimensions
  }
  return d
 }
 if(p.kind==='vertices'){
  const d=structuredClone(source),body=d.bodies.find(b=>b.id===p.id)
  if(!body||body.brep)throw Error('Select vertices of a mesh body.')
  if(!p.delta.every(Number.isFinite)||!p.indices.length||p.indices.some(i=>!Number.isInteger(i)||i<0||i*3+2>=body.mesh.positions.length))throw Error('Invalid vertex edit.')
  for(const i of new Set(p.indices))for(let k=0;k<3;k++)body.mesh.positions[i*3+k]+=p.delta[k]
  return d
 }
 const d=updateSolidNurbsControlPoint(source,p.id,p.u,p.v,p.point,p.weight),curves=d.curves??[]
 for(const bridge of curves.filter(item=>item.bridge)){
  const meta=bridge.bridge!,a=curves.find(item=>item.id===meta.sourceA),b=curves.find(item=>item.id===meta.sourceB)
  if(a&&b)bridge.curve=bridgeNurbsCurves(a.curve,b.curve,meta.endA,meta.endB,meta.tension)
 }
 return d
}
