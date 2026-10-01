import type {DirectDocument,Point2} from './directModeling'
import {sampleCurve,type SketchPlane} from './directSketchGeometry'
const coordinateLimit=1e6
const coordinates=(p:Point2)=>p.every(x=>Number.isFinite(x)&&Math.abs(x)<=coordinateLimit)
/** Lower-left corner and positive local dimensions. No document mutation. */
export function numericRectangle(origin:Point2,size:Point2):Point2[]|null {
 if(!coordinates(origin)||size.some(x=>!Number.isFinite(x)||x<.01||x>coordinateLimit))return null
 const end:Point2=[origin[0]+size[0],origin[1]+size[1]]
 if(!coordinates(end))return null
 return [[...origin],[end[0],origin[1]],end,[origin[0],end[1]]]
}
/** Material-left line/semicircle definitions for the existing retained-profile worker. */
export function numericSlotSources(a:Point2,b:Point2,width:number,plane:SketchPlane):{document:DirectDocument;ids:string[]}|null {
 if(!coordinates(a)||!coordinates(b)||!Number.isFinite(width)||width<.02||width>coordinateLimit)return null
 const length=Math.hypot(b[0]-a[0],b[1]-a[1]),radius=width/2
 if(length<1e-6||[a,b].some(p=>p.some(x=>Math.abs(x)+radius>coordinateLimit)))return null
 const angle=Math.atan2(b[1]-a[1],b[0]-a[0])*180/Math.PI
 const capB={kind:'arc' as const,center:[...b] as Point2,radius,start:angle-90,sweep:180},capA={kind:'arc' as const,center:[...a] as Point2,radius,start:angle+90,sweep:180}
 // The sampler and retained arc builder share native endpoint arithmetic. Use
 // those endpoints for the lines to avoid artificial sub-ulp connector edges.
 const pointsB=sampleCurve(capB),pointsA=sampleCurve(capA)
 const rightA=pointsA.at(-1)!,leftA=pointsA[0],rightB=pointsB[0],leftB=pointsB.at(-1)!
 const ids=['slot-line-0','slot-cap-0','slot-line-1','slot-cap-1']
 const base={closed:false,plane:{origin:[...plane.origin] as [number,number,number],u:[...plane.u] as [number,number,number],v:[...plane.v] as [number,number,number]}}
 return {ids,document:{version:1,bodies:[],sketches:[
  {...base,id:ids[0],name:'Slot side',points:[rightA,rightB]},
  {...base,id:ids[1],name:'Slot cap',points:[rightB,leftB],analytic:capB},
  {...base,id:ids[2],name:'Slot side',points:[leftB,leftA]},
  {...base,id:ids[3],name:'Slot cap',points:[leftA,rightA],analytic:capA},
 ]}}
}
