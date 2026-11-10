import type { NativeGeometryArtifact } from '../core/nativeGeometry'
import type { SemanticRepresentation } from '../core/semanticProgram'

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}
/** Correlate serialized snapshot shape with the admitted program carrier.
 * Native geometry proof remains inside the worker; this grants no certificate.
 */
export function diagnosticSnapshotGeometry(snapshot: NativeGeometryArtifact, representation: SemanticRepresentation): Record<string, unknown> | null {
  const data: unknown = JSON.parse(snapshot.geometryJson)
  if (!record(data)) return null
  if (representation === 'analytic-brep' && snapshot.kind === 'brep') {
    return record(data.geometry) ? data.geometry : null
  }
  if (representation !== 'rational-brep' || snapshot.kind !== 'rational-brep'
    || Object.keys(data).length !== 2 || data.encoding !== 'exact-full-round-snapshot/1'
    || !record(data.artifact) || data.artifact.format !== 'admitted-full-round/1'
    || !record(data.artifact.geometry) || data.artifact.geometry.format !== 'exact-rational-nurbs-brep/1') return null
  return data.artifact.geometry
}
