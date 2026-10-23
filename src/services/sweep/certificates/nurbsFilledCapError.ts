import {callNurbsRust} from '../../geometry/nurbs'
/** Conditional cap-set Hausdorff bound. Premises must belong to the same
 * original sweep and actual retained caps; this function is not admission.
 * Exact region correspondence + ideal ownership + nonsingular projection
 * transfer each paired boundary error to the full region. sqrt(2) epsilon is
 * rounded outward, including nonparallel planes.
 * Shell orientation, embedding and wall error remain separate obligations. */
export function filledMiterCapErrorUpper(premises:{
 idealCapDomainsCertified:boolean
 retainedCapRegionsExact:boolean
 projectionNormalDots:[[number,number],[number,number]]|null
 endpointContourErrorUpper:[number,number]|null
 correctionDisplacementUpper:number|null
 decompositionErrorUpper?:[number,number]|null
 parallelPlanesCertified?:[boolean,boolean]|null
}):[number,number]|null {
 try {
  return callNurbsRust<{capErrorUpper:[number,number]|null}>('sweep_filled_cap_error_upper',{
   ...premises,
   decompositionErrorUpper:premises.decompositionErrorUpper===undefined?[0,0]:premises.decompositionErrorUpper,
  }).capErrorUpper
 }catch{return null}
}

/** Union of already certified retained wall and filled cap sets. Taking the
 * maximum adds no rounding. A missing open cap proof cannot be replaced by
 * the wall proof. This is a boundary-set bound, not a volume/shell certificate. */
export function certifiedSweepBoundaryErrorUpper(wall:number|null,caps:[number,number]|null,closed:boolean):number|null {
 try {
  return callNurbsRust<{boundaryErrorUpper:number|null}>('sweep_boundary_error_upper',{wall,caps:closed?null:caps,closed}).boundaryErrorUpper
 }catch{return null}
}
