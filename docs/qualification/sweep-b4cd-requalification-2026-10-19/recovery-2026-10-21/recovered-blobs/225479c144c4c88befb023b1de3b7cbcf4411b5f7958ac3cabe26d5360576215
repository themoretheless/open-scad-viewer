import type {DirectDocument} from './directModeling'
import {matchNurbsSurfaceJets,type SurfaceJetBoundary} from './nurbsConstructors'

export interface SolidSurfaceMatchOptions {
 referenceBoundary:SurfaceJetBoundary
 editedBoundary:SurfaceJetBoundary
 order:1|2
 scale:number
 reverse:boolean
 maxError?:number
}
/** Native candidate and proof report. A failed native acceptance gate leaves the document unchanged. */
export function matchSolidSurface(document:DirectDocument,referenceId:string,editedId:string,options:SolidSurfaceMatchOptions){
 if(referenceId===editedId)throw Error('Select two different NURBS surfaces.')
 const reference=document.surfaces?.find(s=>s.id===referenceId)
 const edited=document.surfaces?.find(s=>s.id===editedId)
 if(!reference||!edited)throw Error('Surface matching requires two NURBS surfaces.')
 const result=matchNurbsSurfaceJets(reference.surface,edited.surface,options.referenceBoundary,options.editedBoundary,options.order,options.scale,options.reverse,options.maxError)
 const next=structuredClone(document)
 if(result.report.accepted)next.surfaces!.find(s=>s.id===editedId)!.surface=result.surface
 return {document:next,candidate:result.surface,report:result.report}
}

import {prepareNurbsSurfaceSeams} from './nurbsConstructors'
/** Both representations change together in one document transaction. */
export function prepareSolidSurfaceSeams(document:DirectDocument,referenceId:string,editedId:string,options:Pick<SolidSurfaceMatchOptions,'referenceBoundary'|'editedBoundary'|'reverse'> & {maxError:number;openPeriodic?:boolean}){
 if(referenceId===editedId)throw Error('Select two different NURBS surfaces.')
 const reference=document.surfaces?.find(s=>s.id===referenceId),edited=document.surfaces?.find(s=>s.id===editedId)
 if(!reference||!edited)throw Error('Seam preparation requires two NURBS surfaces.')
 const result=prepareNurbsSurfaceSeams(reference.surface,edited.surface,options.referenceBoundary,options.editedBoundary,options.reverse,options.maxError,options.openPeriodic)
 const next=structuredClone(document)
 if(result.report.accepted){
  next.surfaces!.find(s=>s.id===referenceId)!.surface=result.reference
  next.surfaces!.find(s=>s.id===editedId)!.surface=result.edited
 }
 return {document:next,report:result.report,basis:result.basis}
}
