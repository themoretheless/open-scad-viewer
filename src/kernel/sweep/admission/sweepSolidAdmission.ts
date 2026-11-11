import type {NativeGeometryArtifact} from '../../../core/nativeGeometry'
import type {NurbsBrep} from '../../geometry/brep'
import type {SweepVolumeAudit} from '../certificates/nurbsSweepEmbedding'
import {callGeometryRust} from '../../geometry/kernel'
/** Rust recomputes admission on the exact transferred model and provenance. */
export function inspectProgressiveSweepSolidAdmission(artifact:NativeGeometryArtifact,model:NurbsBrep):SweepVolumeAudit|null {
 return callGeometryRust('brep_sweep_solid_admission',{documentJson:artifact.documentJson,nodeId:artifact.nodeId,geometryJson:artifact.geometryJson,model})
}
