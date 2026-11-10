import { deepFreeze } from '../core/objects'
/** Lossless exact-body snapshots. Hashes bind storage; Rust proves geometry. */
import { createNativeGeometryArtifact, isNativeGeometryArtifact, type NativeGeometryArtifact } from '../core/nativeGeometry'
import { callGeometryRust } from '../kernel/geometry/kernel'
import { packNativeGeometrySnapshot, readNativeGeometryData } from './nativeGeometrySnapshot'
import type { ExactRationalBrepGeometry } from '../kernel/brepNativeExecutor'

const ENCODING = 'exact-full-round-snapshot/1'
function restore(artifact: Readonly<Record<string, unknown>>): Readonly<Record<string, unknown>> {
  return callGeometryRust('cad_full_round_exact_restore', { artifact })
}
export function createExactFullRoundSnapshot(
  nodeId: string, artifact: Readonly<Record<string, unknown>>, document: unknown,
): NativeGeometryArtifact {
  const verified = restore(artifact)
  return createNativeGeometryArtifact(nodeId, 'rational-brep', { encoding: ENCODING, artifact: verified }, document, packNativeGeometrySnapshot)
}
export function restoreExactFullRoundSnapshot(snapshot: NativeGeometryArtifact): ExactRationalBrepGeometry {
  if (!isNativeGeometryArtifact(snapshot)) throw new Error('Invalid exact native geometry snapshot')
  if (snapshot.kind !== 'rational-brep') throw new Error('Expected an exact rational B-rep snapshot')
  const data = readNativeGeometryData(snapshot)
  if (!data || typeof data !== 'object' || Array.isArray(data)
    || Object.keys(data).length !== 2 || data.encoding !== ENCODING
    || !data.artifact || typeof data.artifact !== 'object' || Array.isArray(data.artifact)) {
    throw new Error('Invalid exact Full Round snapshot payload')
  }
  const artifact = restore(data.artifact)
  // Geometry comes only from the fresh native proof, not a caller's model copy.
  return deepFreeze({ kind: 'solid', representation: 'rational-brep',
    model: artifact.geometry as ExactRationalBrepGeometry['model'], artifact })
}
