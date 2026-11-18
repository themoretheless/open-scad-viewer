/** JSON-boundary shape of a Full Round body. Native replay lives in `kernel/solidFullRound`. */
import type { ExactFullRoundHistoryDocument } from '../kernel/exactFullRoundHistory'
import type { DirectBody } from './directModeling'

/** Stored on a Solid body. The display mesh is a cache; the history is the authority. */
export interface DirectExactFullRound {
  history: ExactFullRoundHistoryDocument
  segments: number
}

const FORMAT = 'exact-full-round-history/1'
const MAX_REVISIONS = 64

/** Structural check at the JSON boundary; native replay happens in `restoreSolidFullRoundBodies`. */
export function isDirectExactFullRound(value: unknown): value is DirectExactFullRound {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const v = value as Record<string, unknown>,
    history = v.history as Record<string, unknown> | undefined
  return (
    Object.keys(v).length === 2 &&
    Number.isInteger(v.segments) &&
    (v.segments as number) >= 1 &&
    (v.segments as number) <= 32 &&
    !!history &&
    typeof history === 'object' &&
    history.format === FORMAT &&
    Array.isArray(history.revisions) &&
    history.revisions.length >= 1 &&
    history.revisions.length <= MAX_REVISIONS &&
    Number.isInteger(history.cursor) &&
    (history.cursor as number) >= 0 &&
    (history.cursor as number) < history.revisions.length
  )
}

export function canUndoFullRound(body: DirectBody | undefined): boolean {
  return !!body?.exactFullRound && body.exactFullRound.history.cursor > 0
}
export function canRedoFullRound(body: DirectBody | undefined): boolean {
  const h = body?.exactFullRound?.history
  return !!h && h.cursor < h.revisions.length - 1
}
