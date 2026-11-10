import type {NurbsSurface} from './nurbsSurface'
import {callNurbsRust} from './geometry/nurbs'

export interface RetainedPatchRegularity {
 surfaceRegularityCertified:boolean
 cells:number
 unresolvedPatches:number[]
 continuousBound:false
 globalEmbeddingCertified:false
 solidCertified:false
 method:'actual-retained-patch-shared-jacobian-regularity'
}

/** Rust checks actual retained Jacobians with one shared work budget.
 * Regularity alone supplies no intersection, nesting or Solid admission. */
export function inspectRetainedPatchRegularity(patches:NurbsSurface[],maxCells=10000):RetainedPatchRegularity {
 return callNurbsRust('sweep_retained_patch_regularity',{patches,maxCells})
}
