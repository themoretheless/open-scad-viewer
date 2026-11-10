/** Native feature-history edits published as replacement scene meshes.
 * Callers commit the returned mesh only after this entire operation succeeds.
 */
import type { MeshData } from '../core/mesh'
import { geometryAssetId } from '../core/scene'
import { createExactFullRoundSnapshot, restoreExactFullRoundSnapshot } from './exactFullRoundSnapshot'
import { createExactFullRoundHistory, restoreExactFullRoundHistory, editExactFullRoundHistory,
  currentExactFullRoundArtifact, type ExactHistoryBudget, type ExactFullRoundHistoryEdit } from './exactFullRoundHistory'
import { prepareExactFullRoundDisplay } from '../kernel/geometry/brepExactDisplay'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'

const DOCUMENT = 'exact-full-round-history-document/1'
/** The source program remains attribution. It is not attestation of the edited body. */
export function editExactFullRoundSceneMesh(
  previous: MeshData, edit: ExactFullRoundHistoryEdit, segments: number,
  budget?: ExactHistoryBudget,
): MeshData {
  const snapshot = previous.nativeGeometry
  if (!snapshot || snapshot.nodeId !== previous.entityId || !previous.faceIdsAuthoritative) {
    throw new Error('Exact history edit requires an owned native scene mesh')
  }
  // A single source attribution can be retained. Multiple triangle-dependent
  // runs require a real lineage remap, which this feature does not invent.
  if (previous.provenance.length !== 1 || previous.provenance[0].triangleStart !== 0
    || previous.provenance[0].triangleEnd !== previous.indices.length / 3) {
    throw new Error('Exact history edit requires one complete source attribution')
  }
  const restored = restoreExactFullRoundSnapshot(snapshot)
  const document = JSON.parse(snapshot.documentJson)
  const stored = document?.encoding === DOCUMENT
  if (stored && (Object.keys(document).length !== 3 || !('history' in document) || !('sourceDocument' in document))) {
    throw new Error('Invalid exact history scene document')
  }
  const history = stored ? restoreExactFullRoundHistory(document.history, budget)
    : createExactFullRoundHistory(restored.artifact, budget)
  // A valid history of a different body cannot authorize this snapshot.
  // Both values have been independently reconstructed by native proof.
  if (JSON.stringify(currentExactFullRoundArtifact(history)) !== JSON.stringify(restored.artifact)) {
    throw new Error('Exact history current body differs from scene snapshot')
  }
  const next = editExactFullRoundHistory(history, edit, budget)
  const artifact = currentExactFullRoundArtifact(next)
  const display = prepareExactFullRoundDisplay(artifact, segments)
  if (!display.report.closed || !display.displayIndices.length || display.displayIndices.length / 3 > 20000) {
    throw new Error('Exact history edit did not produce an admitted closed display')
  }
  const nativeGeometry = createExactFullRoundSnapshot(snapshot.nodeId, artifact, {
    encoding: DOCUMENT, history: next.document, sourceDocument: stored ? document.sourceDocument : document,
  })
  const vertices = new Float32Array(display.displayVertices), indices = new Uint32Array(display.displayIndices)
  const edges = extractSemanticEdges(vertices, indices)
  return {
    ...previous, nativeGeometry, vertices, indices,
    geometryAssetId: geometryAssetId(vertices, indices), bvh: buildMeshBvh(vertices, indices),
    edgeIndices: edges.indices, faceIds: new Uint32Array(display.faceIds), faceIdsAuthoritative: true,
    faceIdsInferred: false, transform: new Float32Array(previous.transform),
    color: [...previous.color],
    provenance: [{ ...previous.provenance[0], triangleStart: 0, triangleEnd: indices.length / 3 }],
    topology: { boundary: display.report.boundaryEdges, nonManifold: display.report.nonManifoldEdges,
      degenerate: display.report.degenerateTriangles, crease: edges.diagnostics.crease },
  }
}
