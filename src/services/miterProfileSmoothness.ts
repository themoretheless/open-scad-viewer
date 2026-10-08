import type {NurbsBrep} from './geometry/brep'
import {callNurbsRust} from './geometry/nurbs'
import type {MiterStationSmoothness} from './miterStationSmoothness'
import type {SweepProjectiveSeamAudit} from './nurbsSweepAudit'
export interface MiterProfileSmoothness {
 method:'retained-miter-profile-joins'
 maxWork:number
 scope:'wall-profile-seams'
 extractionComplete:boolean
 unclassifiedFaces:number[]
 unpairedEdges:number[]
 edgeIds:number[]
 profile:SweepProjectiveSeamAudit
 g1Audit:SweepProjectiveSeamAudit|null
 profileG1Certified:boolean
 g1Method:'implied-by-G2'|'exact-projective-audit'|'unproved'
 exactWork:number
 station:MiterStationSmoothness
 totalExactWork:number
 stationContinuity:'C0'|'G1'|'G2'
 capContinuity:'C0'|'absent'
 fullBoundarySmoothnessCertified:false
}
/** Retained topology ownership, exact admission and shared budgets are native.
 * Abort checks bracket the synchronous call; they do not interrupt native work. */
export function inspectMiterProfileSmoothness(model:NurbsBrep,capFaces:number[],maxWork=2000000,checkAbort?:()=>void):MiterProfileSmoothness {
 checkAbort?.()
 const report=callNurbsRust<MiterProfileSmoothness>('brep_miter_profile_smoothness_audit',{model,capFaces,maxWork})
 checkAbort?.()
 return report
}
