import {callGeometryRust} from '../../geometry/kernel'
import type {NurbsBrep} from '../../geometry/brep'
import type {SweepProjectiveSeamAudit} from './nurbsSweepAudit'
import type {MiterStationSmoothness} from './miterStationSmoothness'
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
/** Native retained topology extraction and shared G2/G1 work accounting. */
export function inspectMiterProfileSmoothness(model:NurbsBrep,capFaces:number[],maxWork=2000000,checkAbort?:()=>void):MiterProfileSmoothness {
 checkAbort?.()
 const report=callGeometryRust<MiterProfileSmoothness>('brep_miter_profile_smoothness_audit',{model,capFaces,maxWork})
 checkAbort?.()
 return report
}
