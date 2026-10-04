import {certifiedSweepBoundaryErrorUpper} from './nurbsFilledCapError'

/** Two-sided Hausdorff bound for the complete constructed boundary union.
 * Wall jets already include authored frames/guides/miter and stored rounding;
 * their post-construction bound also includes correction and decomposition.
 * Filled cap bounds require original domains, actual region correspondence and
 * nonsingular plane projection. This says nothing about volume or embedding.
 * Only the owning constructor may assemble these same-request components. */
export interface SweepBoundaryCertificate {
 method:'retained-sweep-boundary-union'
 scope:'boundary-set-hausdorff'
 continuousBound:boolean
 withinBudget:boolean|null
 errorUpper:number|null
 budget:number
 closed:boolean
 wallErrorUpper:number|null
 filledCapErrorUpper:[number,number]|null
 reason:'invalid-budget'|'wall-bound-unproved'|'filled-cap-bound-unproved'|'boundary-budget-exceeded'|null
}
export function composeSweepBoundaryCertificate(wall:number|null,caps:[number,number]|null,closed:boolean,budget:number):SweepBoundaryCertificate {
 const valid=(x:number|null)=>x!==null&&Number.isFinite(x)&&x>=0
 const budgetValid=Number.isFinite(budget)&&budget>0
 const errorUpper=budgetValid?certifiedSweepBoundaryErrorUpper(wall,caps,closed):null
 const withinBudget=errorUpper===null?null:errorUpper<=budget
 const reason=!budgetValid?'invalid-budget':!valid(wall)?'wall-bound-unproved':errorUpper===null?'filled-cap-bound-unproved':withinBudget===false?'boundary-budget-exceeded':null
 return {method:'retained-sweep-boundary-union',scope:'boundary-set-hausdorff',continuousBound:errorUpper!==null,withinBudget,errorUpper,budget,closed,wallErrorUpper:valid(wall)?wall:null,filledCapErrorUpper:!closed&&caps?.length===2&&caps.every(valid)?[...caps]:null,reason}
}
