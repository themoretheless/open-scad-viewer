/** Display transport only. Rust restores the exact body and owns all sampling. */
import { callGeometryRust } from './kernel'
import type { BrepMesh } from './brep'

export interface ExactFullRoundDisplay extends Pick<BrepMesh, 'report' | 'faceIds'> {
  readonly representation: 'display-mesh'
  readonly sourceRepresentation: 'rational-brep'
  readonly displayVertices: number[]
  readonly displayIndices: number[]
  readonly topologyFaceIds: string[]
  readonly surfaceArea: number
  /** Uniform sampling does not certify Hausdorff error or mass properties. */
  readonly geometricErrorCertified: false
}
export function prepareExactFullRoundDisplay(
  artifact: Readonly<Record<string, unknown>>, segments = 4, maxTriangles = 20_000,
): ExactFullRoundDisplay {
  return callGeometryRust('cad_full_round_exact_display', { artifact, segments, maxTriangles })
}
