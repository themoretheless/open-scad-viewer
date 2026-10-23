export interface DepthCandidate<T> {
  /** Stable identity for the active selection mode. */
  key: string
  /** Ray distance in world units. */
  distance: number
  value: T
}

export interface DepthCycleState {
  x: number
  y: number
  signature: string
  index: number
  timestamp: number
}

export interface DepthCycleResult<T> {
  candidate: DepthCandidate<T> | null
  state: DepthCycleState | null
}

export const MAX_NORMALIZED_DEPTH_CANDIDATES = 32

/**
 * Sort ray hits front-to-back, discard invalid/duplicate selectable targets,
 * and enforce a hard candidate budget before they reach interactive state.
 */
export function normalizeDepthCandidates<T>(
  candidates: readonly DepthCandidate<T>[],
  maximum = 32,
): DepthCandidate<T>[] {
  const limit = Math.max(0, Math.min(MAX_NORMALIZED_DEPTH_CANDIDATES, Math.trunc(maximum)))
  if (!limit) return []
  const result: Array<{ candidate: DepthCandidate<T>; order: number }> = []
  for (let order = 0; order < candidates.length; order += 1) {
    const candidate = candidates[order]
    if (!candidate.key.length || !Number.isFinite(candidate.distance) || candidate.distance < 0) continue

    const duplicate = result.findIndex(item => item.candidate.key === candidate.key)
    if (duplicate >= 0) {
      if (candidate.distance < result[duplicate].candidate.distance) {
        result[duplicate] = { candidate, order: result[duplicate].order }
      }
    } else {
      result.push({ candidate, order })
    }
    result.sort((a, b) => a.candidate.distance - b.candidate.distance || a.order - b.order)
    if (result.length > limit) result.pop()
  }
  return result.map(item => item.candidate)
}

/**
 * Repeated clicks at the same screen location cycle through the same bounded
 * front-to-back candidate list. Any camera/section/mode mutation should clear
 * the caller-owned previous state.
 */
export function chooseDepthCandidate<T>(
  candidates: readonly DepthCandidate<T>[],
  x: number,
  y: number,
  previous: DepthCycleState | null,
  timestamp: number,
  options: { radiusPx?: number; timeoutMs?: number } = {},
): DepthCycleResult<T> {
  if (!candidates.length || ![x, y, timestamp].every(Number.isFinite)) {
    return { candidate: null, state: null }
  }

  const radius = Math.max(0, options.radiusPx ?? 4)
  const timeout = Math.max(0, options.timeoutMs ?? 1_500)
  const signature = candidates.map(candidate => candidate.key).join('\u001f')
  const canContinue = previous !== null
    && previous.signature === signature
    && timestamp >= previous.timestamp
    && timestamp - previous.timestamp <= timeout
    && Math.hypot(x - previous.x, y - previous.y) <= radius
  const index = canContinue ? (previous.index + 1) % candidates.length : 0
  const state: DepthCycleState = {
    x: canContinue ? previous.x : x,
    y: canContinue ? previous.y : y,
    signature,
    index,
    timestamp,
  }
  return { candidate: candidates[index] ?? null, state }
}
