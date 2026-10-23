import type {DirectDocument} from './directModeling'
import {matchNurbsCurveG1} from './nurbsConstructors'

/** A refused native candidate never changes document geometry or identities. */
export function matchSolidCurve(document:DirectDocument,referenceId:string,editedId:string,referenceEnd:'start'|'end'='end',editedEnd:'start'|'end'='start',maxAngleDegrees=1e-6){
 if(referenceId===editedId)throw Error('Select two different NURBS curves.')
 const reference=document.curves?.find(c=>c.id===referenceId),edited=document.curves?.find(c=>c.id===editedId)
 if(!reference||!edited)throw Error('G1 matching requires two NURBS curves.')
 const result=matchNurbsCurveG1(reference.curve,edited.curve,referenceEnd,editedEnd,maxAngleDegrees)
 const next=structuredClone(document)
 if(result.report.accepted)next.curves!.find(c=>c.id===editedId)!.curve=result.curve
 return {document:next,candidate:result.curve,report:result.report}
}
