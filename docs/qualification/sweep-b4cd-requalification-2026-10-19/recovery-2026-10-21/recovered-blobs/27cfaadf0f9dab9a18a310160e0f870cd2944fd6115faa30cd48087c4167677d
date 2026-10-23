import type {NurbsBrep} from './geometry/brep'
import {callGeometryRust} from './geometry/kernel'
import {sampleSolidNurbsCurve} from './solidNurbs'
export interface BoundaryUse {face:number;wire:number;coedge:number;edge:number;status:'within-tolerance'|'mismatch'|'unresolved';parameter:number|null;distanceIntervalMm:[number,number]|null}
export interface BoundaryAgreement {
 method:'interval-curve-surface-agreement';scope:'edge-surface-correspondence';solidGeometryStatus:'not-certified'
 complete:boolean;allWithinTolerance:boolean;uses:BoundaryUse[];mismatchCount:number;unresolvedCount:number;cells:number;maxCells:number;toleranceMm:number
 lines:{edge:number;points:number[][]}[]
}
export function boundaryExpectation(model:NurbsBrep,maxCells:number){return {maxCells,toleranceMm:model.toleranceMm,uses:model.faces.flatMap((f,face)=>[f.outer,...f.holes].flatMap(wire=>model.loops[wire].coedges.map((c,coedge)=>({face,wire,coedge,edge:c.edge}))))}}
export function validBoundaryAgreement(e:ReturnType<typeof boundaryExpectation>,value:unknown):value is BoundaryAgreement {
 const v=value as BoundaryAgreement
 if(!v||v.method!=='interval-curve-surface-agreement'||v.scope!=='edge-surface-correspondence'||v.solidGeometryStatus!=='not-certified'
  ||v.maxCells!==e.maxCells||v.toleranceMm!==e.toleranceMm||!Number.isInteger(v.cells)||v.cells<0||v.cells>e.maxCells
  ||!Array.isArray(v.uses)||v.uses.length!==e.uses.length||!Array.isArray(v.lines))return false
 let mismatch=0,unresolved=0
 for(let i=0;i<e.uses.length;i++){
  const u=v.uses[i],expected=e.uses[i]
  if(!u||!(['face','wire','coedge','edge'] as const).every(k=>u[k]===expected[k]))return false
  if(u.status==='mismatch'){
   mismatch++
   if(!Number.isFinite(u.parameter)||u.parameter===null||u.parameter<0||u.parameter>1||!Array.isArray(u.distanceIntervalMm)||u.distanceIntervalMm.length!==2)return false
   const [lo,hi]=u.distanceIntervalMm
   if(!Number.isFinite(lo)||!Number.isFinite(hi)||lo<=e.toleranceMm||hi<lo)return false
  }else if(u.status==='within-tolerance'||u.status==='unresolved'){
   if(u.status==='unresolved')unresolved++
   if(u.parameter!==null||u.distanceIntervalMm!==null)return false
  }else return false
 }
 if(v.mismatchCount!==mismatch||v.unresolvedCount!==unresolved||v.complete!==(unresolved===0)||v.allWithinTolerance!==(mismatch===0&&unresolved===0)||v.cells<e.uses.length-unresolved)return false
 const edges=new Set(v.uses.filter(u=>u.status==='mismatch').map(u=>u.edge))
 if(v.lines.length!==edges.size)return false
 for(const line of v.lines){
  if(!line||!edges.delete(line.edge)||!Array.isArray(line.points)||line.points.length<2||line.points.length>4096
   ||!line.points.every(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite)))return false
 }
 return edges.size===0
}
export function inspectBoundaryAgreement(model:NurbsBrep,maxCells:number):BoundaryAgreement {
 const result=callGeometryRust<Omit<BoundaryAgreement,'lines'>>('cad_boundary_agreement',{model,maxCells})
 const lines=[...new Set(result.uses.filter(u=>u.status==='mismatch').map(u=>u.edge))].map(edge=>({edge,points:sampleSolidNurbsCurve(model.edges[edge].curve,32)}))
 return {...result,lines}
}
