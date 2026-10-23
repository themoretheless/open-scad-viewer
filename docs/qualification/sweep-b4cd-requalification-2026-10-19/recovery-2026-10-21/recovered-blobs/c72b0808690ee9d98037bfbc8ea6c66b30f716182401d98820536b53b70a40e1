import type {DirectDocument} from './directModeling'
import {reduceNurbsCurveDegreeCertified} from './nurbsFoundation'
/** Keep the kernel's refusal certificate available; the caller must not commit a rollback. */
export function reduceSolidCurve(document:DirectDocument,id:string,degree:number,maxError:number) {
 const source=document.curves?.find(c=>c.id===id)
 if(!source)throw Error('Select a NURBS curve.')
 if(source.bridge)throw Error('Detach the linked bridge before reducing its degree.')
 const result=reduceNurbsCurveDegreeCertified(source.curve,degree,maxError)
 const next=structuredClone(document)
 if(result.certificate.accepted&&!result.certificate.rolledBack)next.curves!.find(c=>c.id===id)!.curve=result.curve
 return {document:next,certificate:result.certificate}
}

import {reduceNurbsSurfaceDegreeCertified} from './nurbsFoundation'
export function reduceSolidSurface(document:DirectDocument,id:string,axis:'u'|'v',degree:number,maxError:number) {
 const source=document.surfaces?.find(s=>s.id===id)
 if(!source)throw Error('Select a NURBS surface.')
 const result=reduceNurbsSurfaceDegreeCertified(source.surface,axis,degree,maxError)
 const next=structuredClone(document)
 if(result.certificate.accepted&&!result.certificate.rolledBack)next.surfaces!.find(s=>s.id===id)!.surface=result.surface
 return {document:next,certificate:result.certificate}
}

import {rebuildNurbsCurveCertified} from './nurbsFoundation'
export function rebuildSolidCurve(document:DirectDocument,id:string,degree:number,controlCount:number,maxError:number) {
 const source=document.curves?.find(c=>c.id===id)
 if(!source)throw Error('Select a NURBS curve.')
 if(source.bridge)throw Error('Detach the linked bridge before rebuilding it.')
 const result=rebuildNurbsCurveCertified(source.curve,degree,controlCount,maxError)
 const next=structuredClone(document)
 if(result.certificate.accepted&&!result.certificate.rolledBack)next.curves!.find(c=>c.id===id)!.curve=result.curve
 return {document:next,certificate:result.certificate}
}

import {rebuildNurbsSurfaceCertified} from './nurbsFoundation'
export function rebuildSolidSurface(document:DirectDocument,id:string,axis:'u'|'v',degree:number,controlCount:number,maxError:number) {
 const source=document.surfaces?.find(s=>s.id===id)
 if(!source)throw Error('Select a NURBS surface.')
 const result=rebuildNurbsSurfaceCertified(source.surface,axis,degree,controlCount,maxError)
 const next=structuredClone(document)
 if(result.certificate.accepted&&!result.certificate.rolledBack)next.surfaces!.find(s=>s.id===id)!.surface=result.surface
 return {document:next,certificate:result.certificate}
}

export type SolidNurbsRefitOperation='nurbs-rebuild'|'nurbs-reduce'|'nurbs-surface-rebuild'|'nurbs-surface-reduce'
export interface SolidNurbsRefitOptions {operation:SolidNurbsRefitOperation;id:string;axis:'u'|'v';degree:number;controlCount:number;maxError:number}
export function isSolidNurbsRefit(operation:unknown):operation is SolidNurbsRefitOperation {
 return ['nurbs-rebuild','nurbs-reduce','nurbs-surface-rebuild','nurbs-surface-reduce'].includes(String(operation))
}
export function refitSolidNurbs(document:DirectDocument,p:SolidNurbsRefitOptions){
 switch(p.operation){
  case 'nurbs-rebuild':return rebuildSolidCurve(document,p.id,p.degree,p.controlCount,p.maxError)
  case 'nurbs-reduce':return reduceSolidCurve(document,p.id,p.degree,p.maxError)
  case 'nurbs-surface-rebuild':return rebuildSolidSurface(document,p.id,p.axis,p.degree,p.controlCount,p.maxError)
  case 'nurbs-surface-reduce':return reduceSolidSurface(document,p.id,p.axis,p.degree,p.maxError)
  default:throw Error('Unsupported NURBS refit operation.')
 }
}
