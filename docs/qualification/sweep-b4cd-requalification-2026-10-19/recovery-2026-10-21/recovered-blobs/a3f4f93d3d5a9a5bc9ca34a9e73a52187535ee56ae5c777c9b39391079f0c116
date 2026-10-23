import {callNurbsRust} from './geometry/nurbs'

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
 // Preserve the caller's budget as request metadata; all proof decisions are native.
 // The binary codec cannot encode NaN/infinity. Null preserves a missing
 // numerical premise, which Rust classifies conservatively as unproved.
 const wire=(value:number|null)=>value!==null&&Number.isFinite(value)?value:null
 const report=callNurbsRust<SweepBoundaryCertificate>('sweep_boundary_certificate',{wall:wire(wall),caps:caps?.map(wire)??null,closed,budget:wire(budget)})
 return {...report,budget}
}
