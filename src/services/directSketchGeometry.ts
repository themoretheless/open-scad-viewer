import {callGeometryRust} from './geometry/kernel'
import type { DirectSketch, Point2 } from './directModeling'
export type Vec3 = [number,number,number]
export interface SketchPlane { origin:Vec3; u:Vec3; v:Vec3 }
export interface AnalyticCurve { kind:'circle'|'arc'; center:Point2; radius:number; start:number; sweep:number }
export const dot3=(a:number[],b:number[])=>a.reduce((s,v,i)=>s+v*b[i],0)
export const cross3=(a:number[],b:number[]):Vec3=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
export const unit3=(a:number[]):Vec3=>{const n=Math.hypot(...a);if(n<1e-9)throw Error('Zero direction');return a.map(v=>v/n) as Vec3}
export const xyPlane=():SketchPlane=>({origin:[0,0,0],u:[1,0,0],v:[0,1,0]})
export function worldPoints(points:number[][],plane:SketchPlane=xyPlane()):Vec3[] {
 return callGeometryRust('cad_world_points',{points,plane})
}
export function worldPoint(p:number[],plane:SketchPlane=xyPlane()):Vec3 {return worldPoints([p],plane)[0]}
export function sampleCurve(c:AnalyticCurve):Point2[] {
 return callGeometryRust('cad_sample_curve',{curve:c})
}
export function bakeSketch(s:DirectSketch):DirectSketch {const next=structuredClone(s);delete next.analytic;return next}
export function transformSketch(s:DirectSketch,delta:Point2,angle:number,scale:number,pivot?:Point2):DirectSketch {
 return callGeometryRust('cad_transform_sketch',{sketch:s,delta,angle,scale,pivot:pivot??null})
}
export function validateSimpleSketch(points:Point2[],closed=true) {
 callGeometryRust('cad_validate_sketch',{points,closed})
}
export function offsetSketch(s:DirectSketch,distance:number):DirectSketch {
 return callGeometryRust('cad_offset_sketch',{sketch:s,distance})
}
/** Trim a clicked segment between intersections, retaining remaining chains. */
export function trimSketch(s:DirectSketch,edge:number,at:number,boundaries:DirectSketch[]):DirectSketch[] {
 return callGeometryRust('cad_trim_sketch',{sketch:s,edge,at,boundaries})
}
export function extendSketch(s:DirectSketch,end:'start'|'end',boundaries:DirectSketch[]):DirectSketch {
 return callGeometryRust('cad_extend_sketch',{sketch:s,end,boundaries})
}
