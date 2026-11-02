import {reduceNurbsCurveDegreeCertified} from '../../src/services/nurbsFoundation'
import type {NurbsCurve} from '../../src/services/nurbsCurve'
type Check=(value:boolean,label:string)=>void
const quadratic=(points:number[][],weights=[1,1,1]):NurbsCurve=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:points,weights,periodic:false})
const perturbed=()=>quadratic([[1,0],[1,0],[1+Number.EPSILON,0]])
export const degreeReductionZeroCases=[
 {name:'refuses rounded elevation as zero-error authority',run(check:Check){
  const source=perturbed(),r=reduceNurbsCurveDegreeCertified(source,1,0)
  check(r.certificate.accepted===false,'nonzero nominal quadratic rejected at zero budget')
  check(r.certificate.exactZeroRecognized===false,'rounded elevation is not exact identity')
  check(r.certificate.rolledBack===true,'zero-budget rollback reported')
  check((r.certificate.hausdorffErrorUpper??0)>=2**-54,'bound encloses independent exact midpoint deviation')
  check(r.curve.degree===2&&JSON.stringify(r.curve.controlPoints)===JSON.stringify(source.controlPoints)
   &&JSON.stringify(r.curve.knots)===JSON.stringify(source.knots)&&JSON.stringify(r.curve.weights)===JSON.stringify(source.weights),'original nominal definition retained')
 }},
 {name:'accepts finite budget with a nonzero continuous bound',run(check:Check){
  const r=reduceNurbsCurveDegreeCertified(perturbed(),1,1e-12)
  check(r.certificate.accepted===true,'finite budget admits bounded edit')
  check(r.certificate.exactZeroRecognized===false,'finite budget does not fabricate identity')
  check((r.certificate.hausdorffErrorUpper??0)>=2**-54&&(r.certificate.hausdorffErrorUpper??Infinity)<=1e-12,'nonzero bound stays within requested budget')
 }},
 {name:'preserves true polynomial and rational zero-error reductions',run(check:Check){
  for(const source of [quadratic([[0,0],[2,0],[4,0]]),quadratic([[0,0],[3,0],[4,0]],[1,2,3])]) {
   const r=reduceNurbsCurveDegreeCertified(source,1,0)
   check(r.certificate.accepted===true&&r.certificate.exactZeroRecognized===true,'exact original/candidate identity admitted')
   check(r.curve.degree===1&&r.certificate.hausdorffErrorUpper===0,'true reduction retains proved zero bound')
  }
 }},
]
