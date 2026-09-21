/** Display representation selected before a mesh request is dispatched. */
export type DisplayRepresentation = 'bounding-box' | 'preview' | 'exact'

export interface DisplayPolicyInput {
  /** Estimated projected diameter of the object in screen pixels. */
  projectedPixels: number
  /** Desired geometric error in screen pixels. */
  targetErrorPixels: number
  /** Bytes already admitted to the display cache. */
  admittedBytes: number
  /** Hard display-cache budget. */
  budgetBytes: number
  /** Whether the exact representation is available for this object. */
  exactAvailable: boolean
  /** Estimated bytes for the next representation. */
  previewBytes: number
  exactBytes: number
}

const finiteNonNegative = (value: number) => Number.isFinite(value) && value >= 0

/**
 * Selects a display level without performing IO or geometry work.
 *
 * The policy is intentionally conservative at a memory boundary: a level is
 * admitted only when its complete estimated footprint fits the budget. This
 * keeps scheduling decisions deterministic and lets the caller retain the
 * last admitted representation when refinement is refused.
 */
export function selectDisplayRepresentation(input: DisplayPolicyInput): DisplayRepresentation {
  if (![input.projectedPixels, input.targetErrorPixels, input.admittedBytes,
    input.budgetBytes, input.previewBytes, input.exactBytes].every(finiteNonNegative)
    || input.budgetBytes < input.admittedBytes) return 'bounding-box'

  const remaining = input.budgetBytes - input.admittedBytes
  const needsPreview = input.projectedPixels > input.targetErrorPixels
  if (!needsPreview || input.previewBytes > remaining) return 'bounding-box'
  if (input.exactAvailable && input.exactBytes <= remaining) return 'exact'
  return 'preview'
}
