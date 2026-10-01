import type {DirectDocument,DirectSketch,Point2} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
import {withRetainedProfile,retainedProfileDisplay} from './retainedSketchProfile'
import type {BrepProfile} from './geometry/brepProfile'
import {xyPlane,cross3,dot3,unit3,type SketchPlane,type Vec3} from './directSketchGeometry'
import {validateNurbsCurve,type NurbsCurve} from './nurbsCurve'
export interface ProfilePreparationReport {
 /** Ordered like the returned profile; connectors identify both input chains. */
 curveSources?:({chain:number;segment:number;reversed:boolean;connector:false}|{chain:number;end:'start'|'end';nextChain:number;reversed:boolean;connector:true})[]
 /** Measured floating-point reconstruction error; no exact coplanarity claim. */
 projectionMaxDeviationMm?:number
 profile?:BrepProfile
 detail?:string
 accepted:boolean
 reason:'accepted'|'endpoint-topology'|'disconnected'|'invalid-contour'
 segmentDefect?:{kind:'intersection'|'overlap'|'degenerate'|'unproven';segments:{index:number;a:Point2;b:Point2}[]}
 points:Point2[]
 defects:{chain:number;end:'start'|'end';point:Point2;kind:'gap'|'ambiguous';candidates:number[]}[]
 connectors:{a:Point2;b:Point2}[]
}
/** Preparation accepts open sketches and independently stored NURBS curves. */
export function isProfilePreparationInput(document:DirectDocument,id:string):boolean {
 const sketch=document.sketches.find(s=>s.id===id)
 return !!(sketch&&!sketch.closed&&!sketch.retainedProfile&&sketch.analytic?.kind!=='circle'||document.curves?.some(c=>c.id===id&&!c.curve.periodic))
}
const world=(p:number[]):Vec3=>[p[0],p[1],p[2]??0]
function inferPlane(curves:NurbsCurve[]):SketchPlane {
 const points=curves.flatMap(c=>c.controlPoints.map(world)),origin=points[0]
 if(points.every(p=>p[2]===origin[2]))return {...xyPlane(),origin:[0,0,origin[2]]}
 const vectors=points.map(p=>p.map((x,k)=>x-origin[k]) as Vec3)
 const longest=vectors.reduce((a,b)=>Math.hypot(...a)>Math.hypot(...b)?a:b)
 const u=unit3(longest)
 let normal=vectors.map(v=>cross3(u,v)).reduce((a,b)=>Math.hypot(...a)>Math.hypot(...b)?a:b)
 if(Math.hypot(...normal)<1e-9){
  const axis=[0,1,2].sort((a,b)=>Math.abs(u[a])-Math.abs(u[b]))[0]
  const reference:Vec3=[0,0,0];reference[axis]=1;normal=cross3(u,reference)
 }
 normal=unit3(normal)
 const axis=[0,1,2].sort((a,b)=>Math.abs(normal[b])-Math.abs(normal[a]))[0]
 if(normal[axis]<0)normal=normal.map(x=>-x) as Vec3
 return {origin,u,v:unit3(cross3(normal,u))}
}
/** Preserve rational definitions in a common plane and consume inputs atomically. */
export function prepareSolidProfile(document:DirectDocument,ids:string[],tolerance:number){
 if(!ids.length||new Set(ids).size!==ids.length)throw Error('Select distinct open sketches or NURBS curves.')
 if(ids.some(id=>!isProfilePreparationInput(document,id)))throw Error('Profile preparation requires open polylines, arcs or NURBS curves.')
 const items=ids.map(id=>({sketch:document.sketches.find(s=>s.id===id),curve:document.curves?.find(c=>c.id===id)}))
 for(const item of items)if(item.curve)validateNurbsCurve(item.curve.curve)
 const plane=items.find(i=>i.sketch)?.sketch?.plane??(items.some(i=>i.sketch)?xyPlane():inferPlane(items.map(i=>i.curve!.curve)))
 if(items.some(({sketch:s})=>{if(!s)return false;const p=s.plane??xyPlane();return ![p.origin,p.u,p.v].every((a,i)=>a.every((x,j)=>x===[plane.origin,plane.u,plane.v][i][j]))}))throw Error('Profile inputs must use the same sketch plane.')
 let projectionMaxDeviationMm=0
 const sketches:(DirectSketch&{curves?:NurbsCurve[]})[]=items.map(({sketch,curve})=>{
  if(sketch)return sketch
  const source=curve!.curve,n=source.controlPoints.length
  if(source.periodic||source.knots.slice(0,source.degree+1).some(k=>k!==source.knots[source.degree])||source.knots.slice(n).some(k=>k!==source.knots[n]))throw Error(`Profile input ${curve!.id} requires exactly clamped non-periodic endpoints.`)
  const projected:NurbsCurve={...source,controlPoints:source.controlPoints.map(point=>{
   const p=world(point),delta=p.map((x,k)=>x-plane.origin[k]),local:[number,number]=[dot3(delta,plane.u),dot3(delta,plane.v)]
   const reconstructed=plane.origin.map((x,k)=>x+local[0]*plane.u[k]+local[1]*plane.v[k])
   const deviation=Math.hypot(...p.map((x,k)=>x-reconstructed[k]))
   if(!Number.isFinite(deviation)||deviation>1e-7)throw Error(`Profile input ${curve!.id} must lie in the same sketch plane; control deviation exceeds 0.0000001 mm.`)
   projectionMaxDeviationMm=Math.max(projectionMaxDeviationMm,deviation)
   return local
  })}
  return {id:curve!.id,name:curve!.name,group:curve!.group,closed:false,points:[],plane,curves:[projected]}
 })
 const first=sketches[0]
 const report=items.some(i=>i.curve)||sketches.some(s=>s.analytic)
  ?callGeometryRust<ProfilePreparationReport>('cad_prepare_retained_profile',{sketches,tolerance})
  :callGeometryRust<ProfilePreparationReport>('cad_prepare_profile',{chains:sketches.map(s=>s.points),tolerance})
 if(items.some(i=>i.curve))report.projectionMaxDeviationMm=projectionMaxDeviationMm
 if(report.profile)report.points=retainedProfileDisplay(report.profile)[0]
 const next=structuredClone(document)
 if(report.accepted){
  next.sketches=next.sketches.filter(s=>!ids.slice(1).includes(s.id))
  next.curves=next.curves?.filter(c=>!ids.includes(c.id))
  let result=next.sketches.find(s=>s.id===first.id)
  if(!result){result={id:first.id,name:first.name,group:first.group,closed:false,points:[],plane};next.sketches.push(result)}
  if(report.profile)Object.assign(result,withRetainedProfile(result,report.profile))
  else{result.points=report.points;result.closed=true}
  delete result.analytic;delete result.dimensions
 }
 return {document:next,report,plane,id:first.id}
}
