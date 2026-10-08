import {callGeometryRust} from '../../geometry/kernel'
import type {NurbsBrep} from '../../geometry/brep'
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
/** Native retained topology extraction and shared G2/G1 work accounting. */
export function inspectMiterStationSmoothness(model:NurbsBrep,capFaces:number[],maxWork=2000000,checkAbort?:()=>void):MiterStationSmoothness {
 checkAbort?.()
 const report=callGeometryRust<MiterStationSmoothness>('brep_miter_station_smoothness_audit',{model,capFaces,maxWork})
 checkAbort?.()
 return report
}
