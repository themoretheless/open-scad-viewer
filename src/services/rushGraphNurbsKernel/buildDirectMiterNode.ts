import {createDirectMiterOwnedBody,createNativeDirectMiterGeometry} from '../geometry/brep'
import {readSweepCorrections} from './readSweepCorrections'
import type {NurbsCurve} from '../nurbsCurve'
import type {compileRushGraphNurbs} from '../rushGraphNurbsCompiler'
type Node=Extract<ReturnType<typeof compileRushGraphNurbs>['resolved_document']['nodes'][number],{op:'brep_miter_sweep'}>
export function buildDirectMiterNode(n:Node,needCurve:(id:string)=>NurbsCurve) {
 const corrections=readSweepCorrections(n,true)
 if(n.max_deviation===undefined&&n.circle_correction_tolerance!==undefined)throw new Error('Direct miter circle correction requires max_deviation')
 if(n.max_deviation!==undefined){
  const body=createDirectMiterOwnedBody({direct:true,loops:n.loops.map(wire=>wire.map(needCurve)),points:n.points,normal:n.normal,closed:n.closed,miterLimit:n.miter_limit,maxDeviation:n.max_deviation,...corrections})
  if(!body.boundaryCertificate.continuousBound||body.boundaryCertificate.withinBudget!==true)throw new Error('Direct miter complete boundary error exceeds max_deviation or is unproved')
  return {body,placement:{body,quantum:n.circle_correction_quantum??n.cap_correction_quantum??2**-40,maxWork:n.circle_correction_max_work??n.cap_correction_max_work??1000000}}
 }
 return {body:createNativeDirectMiterGeometry(n.loops.map(wire=>wire.map(needCurve)),n.points,n.normal,n.miter_limit,n.closed,corrections.capCorrection),placement:null}
}
