import {callGeometryRust} from './geometry/kernel'
import type {DirectDocument,Point2} from './directModeling'
import {type AnalyticCurve,type SketchPlane} from './directSketchGeometry'
/** Lower-left corner and positive local dimensions. No document mutation. */
export function numericRectangle(origin:Point2,size:Point2):Point2[]|null {
 if(!origin.every(Number.isFinite)||!size.every(Number.isFinite))return null // Binary transport rejects nonfinite numbers.
 return callGeometryRust<Point2[]|null>('cad_client_geometry',{operation:'numericRectangle',origin,size})
}
/** Material-left line/semicircle definitions for the existing retained-profile worker. */
export function numericSlotSources(a:Point2,b:Point2,width:number,plane:SketchPlane):{document:DirectDocument;ids:string[]}|null {
 if(!a.every(Number.isFinite)||!b.every(Number.isFinite)||!Number.isFinite(width))return null
 const geometry=callGeometryRust<{capA:AnalyticCurve;capB:AnalyticCurve;rightA:Point2;leftA:Point2;rightB:Point2;leftB:Point2}|null>('cad_client_geometry',{operation:'numericSlot',a,b,width})
 if(!geometry)return null
 const {capA,capB,rightA,leftA,rightB,leftB}=geometry
 const ids=['slot-line-0','slot-cap-0','slot-line-1','slot-cap-1']
 const base={closed:false,plane:{origin:[...plane.origin] as [number,number,number],u:[...plane.u] as [number,number,number],v:[...plane.v] as [number,number,number]}}
 return {ids,document:{version:1,bodies:[],sketches:[
  {...base,id:ids[0],name:'Slot side',points:[rightA,rightB]},
  {...base,id:ids[1],name:'Slot cap',points:[rightB,leftB],analytic:capB},
  {...base,id:ids[2],name:'Slot side',points:[leftB,leftA]},
  {...base,id:ids[3],name:'Slot cap',points:[leftA,rightA],analytic:capA},
 ]}}
}
