import type {NativeGeometryArtifact} from '../../../core/nativeGeometry'
import {callGeometryRust} from '../../geometry/kernel'
export interface SweepViewportEvidence {
 nodeId:string
 solidGeometryCertified:boolean
 continuousBound:boolean
 profileRegularityCertified:boolean
 wallRegularityCertified:boolean
 profileG2Certified?:boolean
 profileG1Certified?:boolean
 profileSeamCount?:number
 stationG1Certified?:boolean
 stationG2Certified?:boolean
 stationSeamCount?:number
 stationContinuity?:'C0'|'G1'|'G2'
 capContinuity?:'C0'|'absent'
 boundaryErrorUpper:number|null
 boundaryErrorBudget:number|null
 boundaryErrorWithinBudget:boolean|null
}
/** Rust validates reported proof scopes and shared work before display. */
export function readSweepViewportEvidence(artifact:NativeGeometryArtifact|undefined):SweepViewportEvidence|null {
 try{return callGeometryRust('brep_sweep_viewport_evidence',{artifact:artifact??null})}catch{return null}
}
