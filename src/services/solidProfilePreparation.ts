import {sampleSolidNurbsCurve} from './solidNurbs'
import {profileIntersectionDiagnostics} from './profileIntersectionDiagnostics'
import {inspectProfileIntersections,type ProfileIntersectionReport} from './geometry/profileIntersections'
import type {DirectDocument,DirectSketch,Point2} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
import {withRetainedProfile,retainedProfileDisplay} from './retainedSketchProfile'
import type {BrepProfile} from './geometry/brepProfile'
import {xyPlane,type SketchPlane} from './directSketchGeometry'
import {validateNurbsCurve,type NurbsCurve} from './nurbsCurve'
export interface ProfilePreparationReport {
 /** Original assembled XY definitions for read-only diagnostics of a refused contour. */
 diagnosticLoops?:NurbsCurve[][]
 intersections?:ProfileIntersectionReport
 diagnosticDisplay?:{curve:number;kind:'intersection'|'overlap'|'unproven';points:Point2[]}[]
 intersectionDiagnosticError?:string
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
/** Preserve rational definitions in a common plane and consume inputs atomically. */
export function prepareSolidProfile(document:DirectDocument,ids:string[],tolerance:number){
 if(!ids.length||new Set(ids).size!==ids.length)throw Error('Select distinct open sketches or NURBS curves.')
 if(ids.some(id=>!isProfilePreparationInput(document,id)))throw Error('Profile preparation requires open polylines, arcs or NURBS curves.')
 const items=ids.map(id=>({sketch:document.sketches.find(s=>s.id===id),curve:document.curves?.find(c=>c.id===id)}))
 for(const item of items)if(item.curve)validateNurbsCurve(item.curve.curve)
 const sourcePlane=items.find(i=>i.sketch)?.sketch?.plane??(items.some(i=>i.sketch)?xyPlane():null)
 const projection=items.some(i=>i.curve)?callGeometryRust<{plane:SketchPlane;curves:NurbsCurve[];projectionMaxDeviationMm:number}>('cad_client_geometry',{operation:'profileProjection',curves:items.filter(i=>i.curve).map(i=>i.curve!.curve),plane:sourcePlane}):null
 const plane=projection?.plane??sourcePlane!
 let projectedIndex=0
 if(items.some(({sketch:s})=>{if(!s)return false;const p=s.plane??xyPlane();return ![p.origin,p.u,p.v].every((a,i)=>a.every((x,j)=>x===[plane.origin,plane.u,plane.v][i][j]))}))throw Error('Profile inputs must use the same sketch plane.')
 const projectionMaxDeviationMm=projection?.projectionMaxDeviationMm??0
 const sketches:(DirectSketch&{curves?:NurbsCurve[]})[]=items.map(({sketch,curve})=>{
  if(sketch)return sketch
  const source=curve!.curve,n=source.controlPoints.length
  if(source.periodic||source.knots.slice(0,source.degree+1).some(k=>k!==source.knots[source.degree])||source.knots.slice(n).some(k=>k!==source.knots[n]))throw Error(`Profile input ${curve!.id} requires exactly clamped non-periodic endpoints.`)
  const projected=projection!.curves[projectedIndex++]
  return {id:curve!.id,name:curve!.name,group:curve!.group,closed:false,points:[],plane,curves:[projected]}
 })
 const first=sketches[0]
 const report=items.some(i=>i.curve)||sketches.some(s=>s.analytic)
  ?callGeometryRust<ProfilePreparationReport>('cad_prepare_retained_profile',{sketches,tolerance})
  :callGeometryRust<ProfilePreparationReport>('cad_prepare_profile',{chains:sketches.map(s=>s.points),tolerance})
 if(!report.accepted&&report.reason==='invalid-contour'&&report.diagnosticLoops){
  try{
   report.intersections=inspectProfileIntersections(report.diagnosticLoops,{maxPairs:128,maxBoxes:8192,toleranceMm:1e-7})
   const presentation=profileIntersectionDiagnostics(report.diagnosticLoops,report.intersections)
   const kind=presentation.overlaps.length?'overlap':presentation.points.length?'intersection':!presentation.complete?'unproven':undefined
   if(kind&&!report.segmentDefect)report.segmentDefect={kind,segments:[]}
   const highlighted=new Map<number,'intersection'|'overlap'|'unproven'>()
   for(const [kind,events] of [['unproven',[...presentation.unresolved,...presentation.endpointBands]],['intersection',presentation.points],['overlap',presentation.overlaps]] as const)
    for(const event of events)for(const ref of [event.first,event.second])highlighted.set(ref.curve,kind)
   report.diagnosticDisplay=[...highlighted].map(([curve,kind])=>({curve,kind,points:sampleSolidNurbsCurve(report.diagnosticLoops![0]![curve]!,48).map(p=>[p[0],p[1]] as Point2)}))
  }
  catch(error){report.intersectionDiagnosticError=error instanceof Error?error.message:String(error)}
 }
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
