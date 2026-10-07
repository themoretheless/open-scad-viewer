import type {NurbsBrep} from './geometry/brep'
import {callNurbsRust} from './geometry/nurbs'
import type {SweepProjectiveSeamAudit} from './nurbsSweepAudit'
export interface MiterStationSmoothness {
 method:'retained-miter-station-joins'
 scope:'wall-station-seams'
 maxWork:number
 exactWork:number
 extractionComplete:boolean
 unclassifiedFaces:number[]
 unpairedEdges:number[]
 capEdges:number[]
 edgeIds:number[]
 g2:SweepProjectiveSeamAudit
 g1Audit:SweepProjectiveSeamAudit|null
 stationG1Certified:boolean
 stationG2Certified:boolean
}
/** Retained topology ownership, exact admission and shared budgets are native.
 * Abort checks bracket the synchronous call; they do not interrupt native work. */
export function inspectMiterStationSmoothness(model:NurbsBrep,capFaces:number[],maxWork=2000000,checkAbort?:()=>void):MiterStationSmoothness {
 checkAbort?.()
 const report=callNurbsRust<MiterStationSmoothness>('brep_miter_station_smoothness_audit',{model,capFaces,maxWork})
 checkAbort?.()
 return report
}
