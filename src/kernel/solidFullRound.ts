/** Full Round as a Solid document edit. Rust constructs, proves and replays the exact body;
 * the document keeps the native history and a display mesh derived from its current revision.
 */
import { parseDirectDocument, type DirectBody, type DirectDocument } from '../app/directModeling'
import { stringifyMeshJson } from '../exchange/meshJson'
import { callGeometryRust } from './geometry/kernel'
import { normalizePolygonMesh } from './geometry/polygon'
import { prepareExactFullRoundDisplay } from './geometry/brepExactDisplay'
import {
  createExactFullRoundHistory,
  currentExactFullRoundArtifact,
  editExactFullRoundHistory,
  restoreExactFullRoundHistory,
  type ExactFullRoundHistoryDocument,
  type ExactFullRoundHistoryState,
} from './exactFullRoundHistory'
export {
  canRedoFullRound,
  canUndoFullRound,
  isDirectExactFullRound,
  type DirectExactFullRound,
} from '../app/directExactFullRound'

export type SolidFullRoundAction = 'create' | 'undo' | 'redo'
export interface SolidFullRoundOptions {
  id: string
  action: SolidFullRoundAction
  faces: [number, number, number]
  segments: number
}
function validFaces(faces: unknown): faces is [number, number, number] {
  return (
    Array.isArray(faces) &&
    faces.length === 3 &&
    faces.every(f => Number.isInteger(f) && f >= 0 && f <= 0xffffffff) &&
    new Set(faces).size === 3
  )
}

function withHistory(body: DirectBody, state: ExactFullRoundHistoryState, segments: number): DirectBody {
  const display = prepareExactFullRoundDisplay(currentExactFullRoundArtifact(state), segments)
  if (!display.report.closed || !display.displayIndices.length)
    throw Error('Full Round did not produce a closed display body.')
  const { brep: _exactReplacesBrep, ...rest } = body
  return {
    ...rest,
    mesh: normalizePolygonMesh({ positions: display.displayVertices, indices: display.displayIndices }),
    exactFullRound: { history: structuredClone(state.document) as ExactFullRoundHistoryDocument, segments },
  }
}

/** Worker-owned snapshot; body identity, name, group and material are preserved. */
export function applySolidFullRound(document: DirectDocument, p: SolidFullRoundOptions): DirectDocument {
  if (!Number.isInteger(p.segments) || p.segments < 1 || p.segments > 32)
    throw Error('Full Round display segments must be 1..32.')
  const d = parseDirectDocument(stringifyMeshJson(document)),
    index = d.bodies.findIndex(b => b.id === p.id),
    body = d.bodies[index]
  if (!body) throw Error('Select an existing body.')
  if (body.instance) throw Error('Full Round needs an independent body, not a linked instance.')
  let state: ExactFullRoundHistoryState
  if (p.action === 'create') {
    if (!body.brep) throw Error('Full Round needs a B-rep body.')
    if (!validFaces(p.faces)) throw Error('Full Round needs three distinct face indices.')
    const artifact = callGeometryRust<Readonly<Record<string, unknown>>>('cad_full_round_exact', {
      source: body.brep,
      faces: p.faces,
    })
    state = createExactFullRoundHistory(artifact)
  } else {
    if (!body.exactFullRound) throw Error('Select a Full Round body.')
    const restored = restoreExactFullRoundHistory(body.exactFullRound.history)
    state = editExactFullRoundHistory(restored, { action: p.action })
  }
  d.bodies[index] = withHistory(body, state, p.segments)
  return parseDirectDocument(stringifyMeshJson(d))
}

/** Document restore: stored histories are untrusted until Rust has replayed every revision,
 * and the display cache is rebuilt from the replayed current body.
 */
export function restoreSolidFullRoundBodies(document: DirectDocument): DirectDocument {
  if (!document.bodies.some(b => b.exactFullRound)) return document
  return {
    ...document,
    bodies: document.bodies.map(b =>
      b.exactFullRound
        ? withHistory(b, restoreExactFullRoundHistory(b.exactFullRound.history), b.exactFullRound.segments)
        : b,
    ),
  }
}
