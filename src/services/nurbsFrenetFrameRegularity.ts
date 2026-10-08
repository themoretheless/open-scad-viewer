import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'

export interface FrenetFrameRegularity {
 regularityCertified:boolean
 cells:number
 certifiedIntervals:number
 reason:string|null
 continuousBound:false
 surfaceRegularityCertified:false
 globalEmbeddingCertified:false
 method:'original-path-adaptive-frenet-frame-regularity'
}

/** Whole original rational-path Frenet jet cover; no surface or body admission. */
export function inspectFrenetFrameRegularity(path:NurbsCurve,twist:NurbsCurve,maxCells=10000):FrenetFrameRegularity {
 return callNurbsRust('sweep_frenet_frame_regularity',{path,twist,maxCells})
}
