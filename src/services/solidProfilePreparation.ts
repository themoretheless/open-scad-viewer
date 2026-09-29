import type {DirectDocument,Point2} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
import {withRetainedProfile,retainedProfileDisplay} from './retainedSketchProfile'
import type {BrepProfile} from './geometry/brepProfile'
import {xyPlane} from './directSketchGeometry'
export interface ProfilePreparationReport {
 profile?:BrepProfile
 detail?:string
 accepted:boolean
 reason:'accepted'|'endpoint-topology'|'disconnected'|'invalid-contour'
 segmentDefect?:{kind:'intersection'|'overlap'|'degenerate'|'unproven';segments:{index:number;a:Point2;b:Point2}[]}
 points:Point2[]
 defects:{chain:number;end:'start'|'end';point:Point2;kind:'gap'|'ambiguous';candidates:number[]}[]
 connectors:{a:Point2;b:Point2}[]
}
/** Merge selected open polylines/arcs into the first selected identity; preserve input coordinates. */
export function prepareSolidProfile(document:DirectDocument,ids:string[],tolerance:number){
 if(!ids.length||new Set(ids).size!==ids.length)throw Error('Select distinct open sketches.')
 const sketches=ids.map(id=>document.sketches.find(s=>s.id===id))
 if(sketches.some(s=>!s||s.closed||s.retainedProfile||s.analytic?.kind==='circle'))throw Error('Profile preparation requires open polylines or arcs.')
 const first=sketches[0]!,plane=first.plane??xyPlane()
 if(sketches.some(s=>{const p=s!.plane??xyPlane();return ![p.origin,p.u,p.v].every((a,i)=>a.every((x,j)=>x===[plane.origin,plane.u,plane.v][i][j]))}))throw Error('Profile inputs must use the same sketch plane.')
 const report=sketches.some(s=>s!.analytic)
  ?callGeometryRust<ProfilePreparationReport>('cad_prepare_retained_profile',{sketches,tolerance})
  :callGeometryRust<ProfilePreparationReport>('cad_prepare_profile',{chains:sketches.map(s=>s!.points),tolerance})
 if(report.profile)report.points=retainedProfileDisplay(report.profile)[0]
 const next=structuredClone(document)
 if(report.accepted){
  next.sketches=next.sketches.filter(s=>!ids.slice(1).includes(s.id))
  const result=next.sketches.find(s=>s.id===first.id)!
  if(report.profile)Object.assign(result,withRetainedProfile(result,report.profile))
  else{result.points=report.points;result.closed=true}
  delete result.analytic;delete result.dimensions
 }
 return {document:next,report,plane,id:first.id}
}
