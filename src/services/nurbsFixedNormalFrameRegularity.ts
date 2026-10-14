import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'

export interface FixedNormalFrameRegularity {
 regularityCertified:boolean
 cells:number
 certifiedIntervals:number
 reason:string|null
 continuousBound:false
 surfaceRegularityCertified:false
 globalEmbeddingCertified:false
 method:'original-path-adaptive-fixed-normal-frame-regularity'
}

/** Rust covers the original path and twist with one shared work budget.
 * Frame regularity remains separate from retained error and body validity. */
export function inspectFixedNormalFrameRegularity(path:NurbsCurve,normal:[number,number,number],twist:NurbsCurve,maxCells=10000):FixedNormalFrameRegularity {
 return callNurbsRust('sweep_fixed_normal_frame_regularity',{path,normal,twist,maxCells})
}
