import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'

export interface AuthoredFrameRegularity {
 regularityCertified:boolean
 cells:number
 certifiedIntervals:number
 reason:string|null
 continuousBound:false
 method:'original-law-adaptive-authored-frame-regularity'
}

/** Rust proves nonzero/nonparallel authored directions over the whole law
 * traversal. This premise does not certify sweep error or material validity. */
export function inspectAuthoredFrameRegularity(longitudinal:NurbsCurve,transverse:NurbsCurve,maxCells=10000):AuthoredFrameRegularity {
 return callNurbsRust('sweep_authored_frame_regularity',{longitudinal,transverse,maxCells})
}
