import {callNurbsRust} from '../../geometry/nurbs'

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
 return {...callNurbsRust<SweepBoundaryCertificate>('sweep_boundary_certificate',{wall:wall===null||!Number.isFinite(wall)?null:wall,caps:caps?.map(value=>Number.isFinite(value)?value:null)??null,closed,budget:Number.isFinite(budget)?budget:null}),budget}
}
