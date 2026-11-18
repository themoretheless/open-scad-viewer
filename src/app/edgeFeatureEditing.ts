/**
 * Editor-side helpers for edge fillets and chamfers (roadmap C10): dragging the radius with the
 * mouse and remembering the edge chain of an applied feature so that undoing it restores the
 * selection instead of dropping it.
 */

/**
 * Radius after a horizontal drag of `deltaPx` screen pixels, where `pxPerMm` is the current view
 * scale. Rounded to `step` (none when 0) and kept at or above `min`.
 */
export function dragDimensionValue(start: number, deltaPx: number, pxPerMm: number, step = 0, min = 0.01): number {
  if (!Number.isFinite(start) || !Number.isFinite(deltaPx) || !(pxPerMm > 0)) return start
  let value = start + deltaPx / pxPerMm
  if (step > 0) value = Math.round(value / step) * step
  // Trim binary noise from the step rounding.
  value = Number(value.toFixed(6))
  return Math.max(min, value)
}

/** The edge chain an applied fillet or chamfer consumed, keyed by the body state it was applied to. */
export interface EdgeFeatureMemory {
  bodyId: string
  edges: number[]
  /** Serialized B-rep of the body before the feature. */
  before: string
}

interface BodyLike {
  id: string
  brep?: unknown
}

export function rememberEdgeFeature(body: BodyLike | undefined, edges: readonly number[]): EdgeFeatureMemory | null {
  if (!body?.brep) return null
  const unique = [...new Set(edges.filter(i => Number.isInteger(i) && i >= 0))].sort((a, b) => a - b)
  return unique.length ? { bodyId: body.id, edges: unique, before: JSON.stringify(body.brep) } : null
}

/**
 * Edges to reselect when the body is back in the state the feature was applied to (undo). `keep`
 * tells whether the memory still applies: it is dropped once the body disappears.
 */
export function recallEdgeFeature(
  memory: EdgeFeatureMemory | null,
  bodies: readonly BodyLike[],
): { edges: number[] | null; keep: boolean } {
  if (!memory) return { edges: null, keep: false }
  const body = bodies.find(b => b.id === memory.bodyId)
  if (!body?.brep) return { edges: null, keep: false }
  return JSON.stringify(body.brep) === memory.before
    ? { edges: [...memory.edges], keep: false }
    : { edges: null, keep: true }
}
